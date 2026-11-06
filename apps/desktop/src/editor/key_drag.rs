//! A fixed drag snapshot; screen input stays here and key transitions stay in the engine.
use super::dopesheet_selection::KeyId;
use super::*;
use peachy_engine::TimelineKey;

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
    pub bone: usize,
    pub geometry: ChartGeometry,
    pub sources: Vec<TimelineKey>,
    selection_before: Vec<KeyId>,
    origin_x: f32,
    pressed_time: f32,
    pub copy: bool,
    pub toggle: bool,
    pub offset: f32,
    pub active: bool,
}

impl KeyDrag {
    pub fn new(
        animation: usize,
        bone: usize,
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
            vec![pressed]
        };
        Some(Self {
            animation,
            bone,
            geometry,
            sources: keys
                .iter()
                .map(|key| TimelineKey {
                    bone,
                    property: key.property,
                    time: key.time,
                })
                .collect(),
            selection_before: selected.to_vec(),
            origin_x,
            pressed_time: pressed.time,
            copy: (modifiers.command || modifiers.ctrl) && modifiers.shift,
            toggle: modifiers.command || modifiers.ctrl,
            offset: 0.,
            active: false,
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
            dragging && !released && !clicked && (pointer_x - self.origin_x).abs() > threshold
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
        let earliest = self.sources.iter().map(|key| key.time).reduce(f32::min)?;
        Some(offset.max(-earliest))
    }
    fn command(&self, delta: f32) -> Command {
        Command::MoveKeys {
            animation: self.animation,
            keys: self.sources.clone(),
            delta,
            copy: self.copy,
        }
    }
    fn selection_at(&self, offset: f32) -> Vec<KeyId> {
        self.sources
            .iter()
            .map(|key| KeyId {
                property: key.property,
                time: key.time + offset,
            })
            .collect()
    }
    pub fn diagnostic(&self) -> serde_json::Value {
        serde_json::json!({
            "active":self.active,"animation":self.animation,"bone":self.bone,
            "sources":self.sources,"offset":self.offset,"copy":self.copy,
            "toggle":self.toggle,
            "pressedTime":self.pressed_time,"originX":self.origin_x,
            "chart":{"start":self.geometry.start,"width":self.geometry.width,"extent":self.geometry.extent}
        })
    }
}

impl Editor {
    pub(super) fn cancel_dopesheet_drag(&mut self) {
        if let Some(drag) = self.key_drag.take() {
            if drag.active {
                self.session.cancel_transaction();
            }
            self.key_selection.replace(drag.selection_before);
            self.reconcile_dopesheet_selection();
        }
    }
    pub(super) fn update_dopesheet_drag(&mut self, pointer_x: f32, unsnapped: bool) {
        let Some(mut drag) = self.key_drag.take() else {
            return;
        };
        if !drag.active {
            // Finish a previous numeric-control edit before capturing this gesture's base.
            self.session.commit_transaction();
            self.session.begin_transaction();
            drag.active = true;
            self.key_selection.replace(drag.selection_at(0.));
        }
        self.playing = false;
        let offset = drag.offset_at(pointer_x, self.session.document().fps, unsnapped);
        let result = offset
            .ok_or_else(|| "Invalid key drag position".to_string())
            .and_then(|offset| {
                self.session.preview_transaction(drag.command(offset))?;
                Ok(offset)
            });
        match result {
            Ok(offset) => {
                drag.offset = offset;
                self.key_selection.replace(drag.selection_at(offset));
                self.key_drag = Some(drag);
            }
            Err(error) => {
                self.session.cancel_transaction();
                self.key_selection.replace(drag.selection_before);
                self.reconcile_dopesheet_selection();
                self.error = Some(error);
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
                    if drag.copy { "Duplicated" } else { "Moved" },
                    drag.sources.len()
                )
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(time: f32) -> KeyId {
        KeyId {
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
            8,
            key(0.12345),
            &[key(0.12345), key(0.6)],
            geometry(),
            140.,
            egui::Modifiers::NONE,
        )
        .unwrap();
        let offset = drag.offset_at(152., 30, false).unwrap();
        assert!((drag.selection_at(offset)[0].time - 7. / 30.).abs() < 1e-6);
        assert!((drag.offset_at(152., 30, true).unwrap() - 0.12).abs() < 1e-6);
        assert_eq!(drag.offset_at(-100., 30, false), Some(-0.12345));
        let selected = drag.selection_at(-0.12345);
        assert_eq!(selected[0].time, 0.);
        assert!((selected[1].time - 0.47655).abs() < 1e-6);
        assert_eq!(drag.geometry.extent, 2.4);
        assert!(drag.offset_at(f32::NAN, 30, false).is_none());
    }
    #[test]
    fn pressing_an_unselected_key_moves_only_it_and_keeps_the_prior_selection_for_cancel() {
        let drag = KeyDrag::new(
            0,
            8,
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
        assert_eq!(drag.sources[0].bone, 8);
        assert!(!drag.active);
    }
    #[test]
    fn modified_click_release_selects_without_retiming_or_using_later_frame_modifiers() {
        use super::super::dopesheet_selection::KeySelection;
        let mut document: Document = serde_json::from_str(include_str!(
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
        selected.reconcile(&document, Some(0), Some(8));
        selected.select(key(0.), false);
        let mut drag = KeyDrag::new(
            0,
            8,
            pressed,
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
        selected.select(pressed, drag.toggle);
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
