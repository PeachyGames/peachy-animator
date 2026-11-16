//! A fixed drag snapshot; screen input stays here and key transitions stay in the engine.
use super::dopesheet_selection::{KeyId, KeySelectionSnapshot, TimelineScope};
use super::*;
use peachy_engine::{AnimationKey, CycleOffsetRange, CycleOffsetReceipt, KeyOwner};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) enum KeyDragOperation {
    #[default]
    Retime,
    Offset(CycleOffsetRange),
}

/// Per-document view preferences and operation diagnostics, never project data.
#[derive(Default)]
pub(super) struct CycleOffsetView {
    pub enabled: bool,
    pub bounds: BTreeMap<usize, [Option<f32>; 2]>,
    pub blocked: bool,
    pub receipt: Option<CycleOffsetReceipt>,
    mask: Option<CycleOffsetMask>,
}
struct CycleOffsetMask {
    generation: u64,
    animation: usize,
    scope: TimelineScope,
    range: CycleOffsetRange,
    selectable: Result<std::sync::Arc<Vec<AnimationKey>>, String>,
}
impl CycleOffsetView {
    pub fn range(&self, animation: usize, end: f32) -> CycleOffsetRange {
        let bounds = self.bounds.get(&animation).copied().unwrap_or_default();
        CycleOffsetRange {
            start: bounds[0].unwrap_or(0.),
            end: bounds[1].unwrap_or(end),
        }
    }
    pub fn operation(&self, animation: usize, end: f32) -> KeyDragOperation {
        if self.enabled {
            KeyDragOperation::Offset(self.range(animation, end))
        } else {
            KeyDragOperation::Retime
        }
    }
}

pub(super) struct KeyDragContext {
    pub skin: String,
    pub fps: u32,
}

/// Keep event-time modifiers even when later keys changed the frame's modifier snapshot.
pub(super) fn primary_pointer_event(
    events: &[egui::Event],
    pressed: bool,
) -> Option<(Pos2, egui::Modifiers)> {
    events.iter().rev().find_map(|event| match event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: event_pressed,
            modifiers,
        } if *event_pressed == pressed => Some((*pos, *modifiers)),
        _ => None,
    })
}

#[derive(Clone, Copy)]
pub(super) struct ChartGeometry {
    pub start: f32,
    pub width: f32,
    pub extent: f32,
}

impl ChartGeometry {
    fn is_valid(self) -> bool {
        self.start.is_finite()
            && self.width.is_finite()
            && self.width > 0.
            && self.extent.is_finite()
            && self.extent > 0.
    }

    pub(super) fn x_at(self, time: f32) -> f32 {
        (f64::from(self.start) + f64::from(self.width) * (f64::from(time) / f64::from(self.extent)))
            as f32
    }

    pub(super) fn time_at(self, pointer_x: f32) -> Option<f32> {
        if !self.is_valid() || !pointer_x.is_finite() {
            return None;
        }
        let fraction =
            ((f64::from(pointer_x) - f64::from(self.start)) / f64::from(self.width)).clamp(0., 1.);
        Some((fraction * f64::from(self.extent)) as f32)
    }
}

pub(super) struct KeyDrag {
    pub animation: usize,
    pub owner: KeyOwner,
    pub presentation_scope: TimelineScope,
    pub presentation: TreeSelection,
    pub snapshot: Option<KeySelectionSnapshot>,
    pub primary_before: Option<KeyId>,
    pressed: KeyId,
    pub geometry: ChartGeometry,
    pub sources: Vec<AnimationKey>,
    selection_before: Vec<KeyId>,
    origin_x: f32,
    pressed_time: f32,
    pub copy: bool,
    pub toggle: bool,
    pub offset: f32,
    pub active: bool,
    pub operation: KeyDragOperation,
    pub context: Option<KeyDragContext>,
    pub receipt_before: Option<CycleOffsetReceipt>,
}

impl KeyDrag {
    pub fn new(
        animation: usize,
        pressed: KeyId,
        selected: &[KeyId],
        geometry: ChartGeometry,
        origin_x: f32,
        modifiers: egui::Modifiers,
    ) -> Option<Self> {
        if !geometry.is_valid() || !origin_x.is_finite() {
            return None;
        }
        let keys = if selected.contains(&pressed) {
            selected.to_vec()
        } else {
            vec![pressed.clone()]
        };
        Some(Self {
            animation,
            owner: pressed.owner(),
            presentation_scope: TimelineScope::Owners(vec![pressed.owner()]),
            presentation: Default::default(),
            snapshot: None,
            primary_before: selected.first().cloned(),
            pressed: pressed.clone(),
            geometry,
            sources: keys,
            selection_before: selected.to_vec(),
            origin_x,
            pressed_time: pressed.time(),
            copy: (modifiers.command || modifiers.ctrl) && modifiers.shift,
            toggle: modifiers.command || modifiers.ctrl,
            offset: 0.,
            active: false,
            operation: KeyDragOperation::Retime,
            context: None,
            receipt_before: None,
        })
    }
    pub fn should_preview(
        &self,
        pointer_x: f32,
        threshold: f32,
        dragging: bool,
        released: bool,
        clicked: bool,
    ) -> bool {
        if self.active {
            dragging || released
        } else {
            (released || (dragging && !clicked)) && (pointer_x - self.origin_x).abs() > threshold
        }
    }
    pub fn offset_at(&self, pointer_x: f32, fps: u32, unsnapped: bool) -> Option<f32> {
        let mut offset = (f64::from(pointer_x) - f64::from(self.origin_x))
            / f64::from(self.geometry.width)
            * f64::from(self.geometry.extent);
        if !offset.is_finite() || fps == 0 {
            return None;
        }
        if !unsnapped {
            offset = ((f64::from(self.pressed_time) + offset) * f64::from(fps)).round()
                / f64::from(fps)
                - f64::from(self.pressed_time);
        }
        let offset = offset as f32;
        if !offset.is_finite() {
            return None;
        }
        match self.operation {
            KeyDragOperation::Offset(_) => Some(offset),
            KeyDragOperation::Retime => {
                let earliest = self.sources.iter().map(|key| key.time()).reduce(f32::min)?;
                Some(offset.max(-earliest))
            }
        }
    }
    #[cfg(test)]
    fn command(&self, delta: f32) -> Command {
        Command::MoveAnimationKeys {
            animation: self.animation,
            keys: self.sources.clone(),
            delta,
            copy: self.copy,
        }
    }
    #[cfg(test)]
    fn selection_at(&self, offset: f32) -> Vec<KeyId> {
        self.sources
            .iter()
            .map(|key| match key {
                AnimationKey::Slider {
                    constraint,
                    property,
                    time,
                } => AnimationKey::Slider {
                    constraint: *constraint,
                    property: *property,
                    time: time + offset,
                },
                AnimationKey::Bone {
                    bone,
                    property,
                    time,
                } => AnimationKey::Bone {
                    bone: *bone,
                    property: *property,
                    time: time + offset,
                },
                AnimationKey::Ik { constraint, time } => AnimationKey::Ik {
                    constraint: *constraint,
                    time: time + offset,
                },
                AnimationKey::Deform { attachment, time } => AnimationKey::Deform {
                    attachment: attachment.clone(),
                    time: time + offset,
                },
                AnimationKey::SlotAttachment { slot, time } => AnimationKey::SlotAttachment {
                    slot: *slot,
                    time: time + offset,
                },
                AnimationKey::SlotColor { slot, time } => AnimationKey::SlotColor {
                    slot: *slot,
                    time: time + offset,
                },
                AnimationKey::DrawOrder { time } => AnimationKey::DrawOrder {
                    time: time + offset,
                },
            })
            .collect()
    }
    pub fn diagnostic(&self) -> serde_json::Value {
        serde_json::json!({
            "active":self.active,"animation":self.animation,"owner":self.owner,
            "sources":self.sources,"offset":self.offset,"copy":self.copy,"presentation":self.presentation.diagnostic(),
            "presentationScope":match &self.presentation_scope {TimelineScope::All=>serde_json::json!({"type":"all"}),TimelineScope::Owners(owners)=>serde_json::json!({"type":"owners","owners":owners})},
            "toggle":self.toggle,
            "pressedTime":self.pressed_time,"originX":self.origin_x,
            "operation":match self.operation {
                KeyDragOperation::Retime=>serde_json::json!({"type":"retime"}),
                KeyDragOperation::Offset(range)=>serde_json::json!({"type":"offset","range":{"start":range.start,"end":range.end}})
            },
            "skin":self.context.as_ref().map(|context|&context.skin),
            "fps":self.context.as_ref().map(|context|context.fps),
            "chart":{"start":self.geometry.start,"width":self.geometry.width,"extent":self.geometry.extent}
        })
    }
}

impl Editor {
    /// Every memory-only transition invalidates this shell cache explicitly.
    pub(super) fn invalidate_cycle_offset_mask(&mut self) {
        self.cycle_offset.mask = None;
    }

    pub(super) fn cycle_offset_selectable(
        &mut self,
    ) -> Result<std::sync::Arc<Vec<AnimationKey>>, String> {
        let animation = self.animation;
        let range = self.cycle_offset.range(animation, self.animation_end());
        let generation = self.session.document_generation();
        let scope = self.key_selection.scope().clone();
        let cached = self.cycle_offset.mask.as_ref().is_some_and(|mask| {
            mask.generation == generation
                && mask.animation == animation
                && mask.scope == scope
                && mask.range == range
        });
        if !cached {
            let keys = self
                .key_selection
                .shown()
                .keys
                .iter()
                .filter(|key| matches!(key, AnimationKey::Bone { .. }))
                .cloned()
                .collect::<Vec<_>>();
            let selectable = self
                .session
                .cycle_offset_selectable_keys(animation, &keys, range)
                .map(std::sync::Arc::new)
                .map_err(|error| Self::cycle_offset_error_message(&error));
            self.cycle_offset.mask = Some(CycleOffsetMask {
                generation,
                animation,
                scope,
                range,
                selectable,
            });
        }
        self.cycle_offset.mask.as_ref().unwrap().selectable.clone()
    }

    fn cycle_offset_error_message(error: &str) -> String {
        if error.contains("Stale Cycle Offset") {
            "These curves changed after Offset began. Turn Offset off and on to use the current curves.".into()
        } else {
            format!("Offset unavailable: {error}")
        }
    }

    pub(super) fn reconcile_cycle_offset_selection(&mut self) {
        if !self.cycle_offset.enabled || self.setup_mode {
            return;
        }
        match self.cycle_offset_selectable() {
            Ok(selectable) => self.key_selection.retain_keys(|key| {
                !matches!(key, AnimationKey::Bone { .. }) || selectable.contains(key)
            }),
            Err(error) => self.status = error,
        }
    }

    pub(super) fn toggle_cycle_offset(&mut self) {
        self.cancel_gesture();
        // Undo/Redo can restore origins while this view mode is off. Clear on
        // both transitions, after the owned transaction has been canceled.
        match self.session.reset_cycle_offset_memory() {
            Ok(()) => {
                self.cycle_offset.enabled = !self.cycle_offset.enabled;
                self.cycle_offset.receipt = None;
                self.invalidate_cycle_offset_mask();
                self.status = if self.cycle_offset.enabled {
                    "Offset: selected keys seed whole closed Bone property tracks.".into()
                } else {
                    "Offset off".into()
                };
                self.reconcile_cycle_offset_selection();
            }
            Err(error) => self.status = Self::cycle_offset_error_message(&error),
        }
    }

    fn restore_dopesheet_drag_selection(&mut self, drag: KeyDrag) {
        if let Some(snapshot) = drag.snapshot {
            let animation = self.active_animation();
            let scope = self.timeline_scope();
            if let Err(error) =
                self.key_selection
                    .restore_snapshot(&self.session, animation, scope, snapshot)
            {
                self.key_selection.clear();
                self.status = error;
            }
        } else {
            // Pure legacy drag fixtures construct the snapshot without a view.
            self.key_selection.replace(drag.selection_before);
            self.key_selection.set_primary(drag.primary_before);
        }
        self.reconcile_dopesheet_selection();
    }
    pub(super) fn cancel_dopesheet_drag(&mut self) {
        if let Some(drag) = self.key_drag.take() {
            if drag.active {
                self.session.cancel_transaction();
            }
            self.cycle_offset.blocked = true;
            self.cycle_offset.receipt = drag.receipt_before.clone();
            self.invalidate_cycle_offset_mask();
            self.restore_dopesheet_drag_selection(drag);
        }
    }
    pub(super) fn reconcile_dopesheet_input(&mut self, ctx: &egui::Context) {
        if ctx.input(|input| {
            !input.pointer.primary_down()
                && !input.pointer.button_released(egui::PointerButton::Primary)
        }) {
            self.cycle_offset.blocked = false;
        }
        if self.key_drag.is_some()
            && (!ctx.input(|input| input.focused)
                || self.error.is_some()
                || self.rename.is_some()
                || self.duplicate_animation.is_some()
                || self.export_dialog.is_some()
                || self.slot_color_dialog_open()
                || egui::Popup::is_any_open(ctx)
                || ctx.text_edit_focused())
        {
            self.cancel_dopesheet_drag();
        }
    }
    pub(super) fn update_dopesheet_drag(&mut self, pointer_x: f32, unsnapped: bool) {
        let Some(mut drag) = self.key_drag.take() else {
            return;
        };
        if matches!(drag.operation, KeyDragOperation::Offset(_)) && drag.copy {
            self.cycle_offset.blocked = true;
            self.restore_dopesheet_drag_selection(drag);
            self.status = "Copying keys while using Offset is not supported.".into();
            return;
        }
        if !drag.active {
            // Finish a previous numeric-control edit before capturing this gesture's base.
            self.session.commit_transaction();
            self.session.begin_transaction();
            drag.active = true;
            self.key_selection.replace(drag.sources.clone());
            self.key_selection.set_primary(Some(drag.pressed.clone()));
        }
        self.playing = false;
        let offset = drag.offset_at(pointer_x, self.session.document().fps, unsnapped);
        let result = offset
            .ok_or_else(|| "Invalid key drag position".to_string())
            .and_then(|offset| {
                let (selected, primary) = match drag.operation {
                    KeyDragOperation::Retime => {
                        let selected = self.session.preview_move_animation_keys(
                            drag.animation,
                            &drag.sources,
                            offset,
                            drag.copy,
                        )?;
                        let primary = drag
                            .sources
                            .iter()
                            .position(|key| key == &drag.pressed)
                            .and_then(|index| selected.get(index))
                            .cloned();
                        (selected, primary)
                    }
                    KeyDragOperation::Offset(range) => {
                        let receipt = self.session.preview_offset_animation_keys(
                            drag.animation,
                            &drag.sources,
                            range,
                            offset,
                        )?;
                        let selected = receipt
                            .mappings
                            .iter()
                            .filter_map(|mapping| mapping.target.clone())
                            .collect();
                        let primary = receipt
                            .mappings
                            .iter()
                            .find(|mapping| mapping.source == drag.pressed)
                            .and_then(|mapping| mapping.target.clone());
                        self.cycle_offset.receipt = Some(receipt);
                        self.invalidate_cycle_offset_mask();
                        (selected, primary)
                    }
                };
                Ok((offset, selected, primary))
            });
        match result {
            Ok((offset, selected, primary)) => {
                drag.offset = offset;
                self.key_selection.replace(selected);
                self.key_selection.set_primary(primary);
                self.key_drag = Some(drag);
            }
            Err(error) => {
                self.session.cancel_transaction();
                self.cycle_offset.blocked = true;
                self.cycle_offset.receipt = drag.receipt_before.clone();
                self.invalidate_cycle_offset_mask();
                self.restore_dopesheet_drag_selection(drag);
                if error.contains("Stale Cycle Offset") {
                    self.status = Self::cycle_offset_error_message(&error);
                } else {
                    self.error = Some(error);
                }
            }
        }
    }
    pub(super) fn finish_dopesheet_drag(&mut self) {
        if let Some(drag) = self.key_drag.take()
            && drag.active
        {
            self.session.commit_transaction();
            self.status = if drag.offset == 0. {
                "Key timing unchanged".into()
            } else {
                format!(
                    "{} {} keys",
                    match drag.operation {
                        KeyDragOperation::Offset(_) => "Offset",
                        KeyDragOperation::Retime if drag.copy => "Duplicated",
                        KeyDragOperation::Retime => "Moved",
                    },
                    drag.sources.len()
                )
            };
        }
    }

    pub(super) fn cycle_offset_diagnostic(&self) -> serde_json::Value {
        let range = self
            .cycle_offset
            .range(self.animation, self.animation_end());
        let bounds = self
            .cycle_offset
            .bounds
            .get(&self.animation)
            .copied()
            .unwrap_or_default();
        serde_json::json!({
            "enabled":self.cycle_offset.enabled,"blocked":self.cycle_offset.blocked,
            "bounds":{"start":bounds[0],"end":bounds[1]},
            "effectiveRange":{"start":range.start,"end":range.end},
            "hasMemory":!self.session.cycle_offset_memory().is_empty(),
            "selectableKeys":self.cycle_offset.mask.as_ref().and_then(|mask|mask.selectable.as_ref().ok()).map(|keys|keys.as_ref()),
            "selectionError":self.cycle_offset.mask.as_ref().and_then(|mask|mask.selectable.as_ref().err()),
            "lastReceipt":self.cycle_offset.receipt.as_ref().map(|receipt|serde_json::json!({
                "keys":receipt.keys,"boundaryKeys":receipt.boundary_keys,
                "mappings":receipt.mappings.iter().map(|mapping|serde_json::json!({"source":mapping.source,"target":mapping.target})).collect::<Vec<_>>(),
                "phases":receipt.phases.iter().map(|phase|serde_json::json!({"source":phase.source,"range":{"start":phase.range.start,"end":phase.range.end},"phase":phase.phase})).collect::<Vec<_>>()
            }))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ik_drag_freezes_group_identity_and_cancel_restores_selection_and_redo() {
        let mut editor = super::super::test_support::animated_ik_editor();
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "redo".into(),
            })
            .unwrap();
        editor.undo();
        editor.reconcile_dopesheet_selection();
        let selected = KeyId::Ik {
            constraint: 1,
            time: 1.,
        };
        editor.key_selection.replace(vec![selected.clone()]);
        let original = editor.session.document().clone();
        editor.key_drag = KeyDrag::new(
            0,
            selected.clone(),
            std::slice::from_ref(&selected),
            ChartGeometry {
                start: 0.,
                width: 200.,
                extent: 2.,
            },
            100.,
            egui::Modifiers::NONE,
        );
        editor.update_dopesheet_drag(50., true);
        assert_eq!(
            editor.key_selection.keys(),
            &[KeyId::Ik {
                constraint: 1,
                time: 0.5
            }]
        );
        let keys = &editor.session.document().animations[0].ik_tracks[0].keys;
        assert_eq!(keys[1].time, 0.5);
        assert_eq!(keys[1].curve, IkCurve::Stepped);
        assert_eq!(editor.time, 0.25);
        editor.cancel_dopesheet_drag();
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.key_selection.keys(), &[selected]);
        assert!(editor.session.can_redo());
        assert!(!editor.session.can_undo());
    }

    fn key(time: f32) -> KeyId {
        KeyId::Bone {
            bone: 8,
            property: Property::Rotation,
            time,
        }
    }
    fn geometry() -> ChartGeometry {
        ChartGeometry {
            start: 100.,
            width: 240.,
            extent: 2.4,
        }
    }
    #[test]
    fn fixed_mapping_snaps_relative_offsets_and_clamps_the_whole_group_at_zero() {
        let drag = KeyDrag::new(
            0,
            key(0.12345),
            &[key(0.12345), key(0.6)],
            geometry(),
            140.,
            egui::Modifiers::NONE,
        )
        .unwrap();
        let offset = drag.offset_at(152., 30, false).unwrap();
        assert!((drag.selection_at(offset)[0].time() - 7. / 30.).abs() < 1e-6);
        assert!((drag.offset_at(152., 30, true).unwrap() - 0.12).abs() < 1e-6);
        assert_eq!(drag.offset_at(-100., 30, false), Some(-0.12345));
        let selected = drag.selection_at(-0.12345);
        assert_eq!(selected[0].time(), 0.);
        assert!((selected[1].time() - 0.47655).abs() < 1e-6);
        assert_eq!(drag.geometry.extent, 2.4);
        assert!(drag.offset_at(f32::NAN, 30, false).is_none());
    }
    #[test]
    fn pressing_an_unselected_key_moves_only_it_and_keeps_the_prior_selection_for_cancel() {
        let drag = KeyDrag::new(
            0,
            key(0.9),
            &[key(0.3), key(0.6)],
            geometry(),
            140.,
            egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        )
        .unwrap();
        assert_eq!(drag.selection_at(0.), vec![key(0.9)]);
        assert_eq!(drag.selection_before, vec![key(0.3), key(0.6)]);
        assert!(drag.copy);
        assert_eq!(drag.sources[0].owner(), KeyOwner::Bone { bone: 8 });
        assert!(!drag.active);
    }
    #[test]
    fn modified_click_release_selects_without_retiming_or_using_later_frame_modifiers() {
        use super::super::dopesheet_selection::KeySelection;
        let mut document: Document = peachy_io::decode_project(include_str!(
            "../../../../examples/peach-bot/peach-bot.peachy"
        ))
        .unwrap();
        document.animations[0].tracks[0].keys[1].time = 0.12345;
        document.validate().unwrap();
        let pressed = key(0.12345);
        let raw = egui::RawInput {
            events: vec![
                egui::Event::PointerButton {
                    pos: Pos2::ZERO,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::COMMAND,
                },
                egui::Event::PointerButton {
                    pos: Pos2::ZERO,
                    button: egui::PointerButton::Secondary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::PointerButton {
                    pos: Pos2::ZERO,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::COMMAND,
                },
                egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            ],
            ..Default::default()
        };
        let (position, modifiers) = primary_pointer_event(&raw.events, true).unwrap();
        let mut selected = KeySelection::default();
        selected.reconcile(&document, Some(0), Some(KeyOwner::Bone { bone: 8 }));
        selected.select(key(0.), false);
        let mut drag = KeyDrag::new(
            0,
            pressed.clone(),
            selected.keys(),
            geometry(),
            position.x,
            modifiers,
        )
        .unwrap();
        let mut session = Session::new(document.clone()).unwrap();
        // A release may also have a drag-stopped response after an earlier cancelled drag.
        if drag.should_preview(position.x, 6., false, true, true) {
            session.begin_transaction();
            let offset = drag.offset_at(position.x, document.fps, false).unwrap();
            session.preview_transaction(drag.command(offset)).unwrap();
            selected.replace(drag.selection_at(offset));
        }
        selected.select(pressed.clone(), drag.toggle);
        assert_eq!(selected.keys(), &[key(0.), pressed]);
        assert_eq!(session.document(), &document);
        assert!(!session.can_undo());
        assert!(!drag.should_preview(position.x, 6., false, true, false));
        assert!(!drag.should_preview(position.x, 6., true, false, false));
        assert!(drag.should_preview(position.x + 7., 6., true, false, false));
        drag.active = true;
        assert!(drag.should_preview(position.x, 6., false, true, false));
    }
}
