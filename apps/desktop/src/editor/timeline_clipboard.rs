//! Clipboard presentation coordinates the shared key commands and editor-session state.
use super::dopesheet_selection::SelectionScope;
use super::icons::Icon;
use super::*;
use peachy_engine::{AnimationKeyClipboard, pasted_animation_key_group, pasted_animation_keys};

#[path = "timeline_group_clipboard.rs"]
mod group;
#[path = "timeline_leaf_clipboard.rs"]
mod leaf;
#[cfg(test)]
#[path = "timeline_leaf_clipboard_tests.rs"]
mod leaf_tests;
pub(super) use leaf::LeafSourceReceipt;
#[cfg(test)]
#[path = "timeline_group_clipboard_tests.rs"]
mod group_tests;
use group::CopiedKeys;
pub(super) use group::{GroupClipboard, PasteReceipt};

fn clipboard_text(clipboard: &AnimationKeyClipboard) -> String {
    serde_json::json!({
        "peachyAnimatorClipboard": "keys",
        "version": 2,
        "payload": clipboard,
    })
    .to_string()
}

fn matches_clipboard(text: &str, clipboard: &AnimationKeyClipboard) -> bool {
    text == clipboard_text(clipboard)
}

pub(super) fn clipboard_len(clipboard: &AnimationKeyClipboard) -> usize {
    match clipboard {
        AnimationKeyClipboard::Bone { keys } => keys.len(),
        AnimationKeyClipboard::Ik { keys } => keys.len(),
        AnimationKeyClipboard::Slider { keys } => keys.len(),
        AnimationKeyClipboard::Deform { keys, .. } => keys.len(),
        AnimationKeyClipboard::DrawOrder { keys, .. } => keys.len(),
        AnimationKeyClipboard::Slot {
            attachment_keys,
            color_keys,
        } => attachment_keys.len() + color_keys.len(),
    }
}

#[derive(Debug, PartialEq)]
enum TimelineAction {
    SelectAll,
    Delete,
    Copy,
    Cut,
    PasteText(String),
    PasteCurrent,
}

/// Per-editor key latch; no selection, document or clipboard data is stored here.
#[derive(Default)]
pub(super) struct TimelineInput {
    select_all_held: bool,
}

impl TimelineInput {
    pub(super) fn select_all_held(&self) -> bool {
        self.select_all_held
    }

    fn observe_key_state(&mut self, down: bool) {
        self.select_all_held = down;
    }
}

/// Native clipboard events and Select All share one ordered action stream.
/// Preserve every event when text or another control owns input.
fn take_timeline_actions(
    events: &mut Vec<egui::Event>,
    focused: bool,
    state: &mut TimelineInput,
) -> Vec<TimelineAction> {
    if !focused {
        return Vec::new();
    }
    let native_copy = events
        .iter()
        .any(|event| matches!(event, egui::Event::Copy));
    let native_cut = events.iter().any(|event| matches!(event, egui::Event::Cut));
    let native_paste = events
        .iter()
        .any(|event| matches!(event, egui::Event::Paste(_)));
    let mut actions = Vec::new();
    for event in std::mem::take(events) {
        let action = match event {
            egui::Event::Key {
                key: egui::Key::A,
                pressed,
                repeat,
                modifiers,
                ..
            } => {
                let held = state.select_all_held;
                state.select_all_held = pressed;
                if pressed && modifiers.matches_exact(egui::Modifiers::COMMAND) {
                    if held || repeat {
                        continue;
                    }
                    TimelineAction::SelectAll
                } else {
                    events.push(event);
                    continue;
                }
            }
            egui::Event::Copy => TimelineAction::Copy,
            egui::Event::Cut => TimelineAction::Cut,
            egui::Event::Paste(text) => TimelineAction::PasteText(text),
            egui::Event::Key {
                key: egui::Key::Delete | egui::Key::Backspace,
                pressed: true,
                modifiers,
                ..
            } if modifiers.matches_exact(egui::Modifiers::NONE) => TimelineAction::Delete,
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if modifiers.matches_exact(egui::Modifiers::COMMAND)
                && matches!(key, egui::Key::C | egui::Key::X | egui::Key::V) =>
            {
                match key {
                    egui::Key::C if !native_copy => TimelineAction::Copy,
                    egui::Key::X if !native_cut => TimelineAction::Cut,
                    egui::Key::V if !native_paste => TimelineAction::PasteCurrent,
                    // A host may emit both forms. The dedicated paste text remains
                    // authoritative, so a raw key cannot bypass envelope validation.
                    _ => continue,
                }
            }
            other => {
                events.push(other);
                continue;
            }
        };
        actions.push(action);
    }
    actions
}

impl Editor {
    /// Run after shortcut routing, including its text/modal early-return paths.
    /// Observing physical state does not consume input owned by another control.
    pub(super) fn observe_timeline_input(&mut self, ctx: &egui::Context) {
        let down = ctx.input(|input| input.focused && input.key_down(egui::Key::A));
        self.timeline_input.observe_key_state(down);
    }

    fn select_all_dopesheet_keys(&mut self) {
        self.cancel_gesture();
        self.reconcile_dopesheet_selection();
        let scope = match self.key_selection.select_all_shown() {
            Ok(scope) => scope,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        let count = self.key_selection.len();
        self.status = match scope {
            SelectionScope::None => "No keys shown".into(),
            SelectionScope::Rows => format!("Selected {count} keys in selected rows"),
            SelectionScope::Shown => format!("Selected all {count} shown keys"),
        };
    }

    fn clipboard_context_available(&self) -> bool {
        !self.setup_mode
            && (self.dopesheet_visible() || self.graph_visible())
            && self
                .session
                .document()
                .animations
                .get(self.animation)
                .is_some()
            && self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
            && self.export_dialog.is_none()
            && !self.slot_color_dialog_open()
    }

    /// Read only committed keys. Pending pose edits never enter the clipboard.
    fn selected_clipboard(&mut self) -> Option<CopiedKeys> {
        if !self.clipboard_context_available() {
            return None;
        }
        self.cancel_gesture();
        self.reconcile_dopesheet_selection();
        if !self.clipboard_context_available() || self.key_selection.is_empty() {
            return None;
        }
        let keys = self.key_selection.keys();
        let single_owner = keys.iter().all(|key| key.owner() == keys[0].owner());
        let result = if single_owner {
            LeafSourceReceipt::copy(&self.session, self.animation, keys).map(CopiedKeys::Single)
        } else {
            GroupClipboard::copy(&self.session, self.animation, keys).map(CopiedKeys::Group)
        };
        match result {
            Ok(clipboard) => Some(clipboard),
            Err(error) => {
                self.error = Some(error);
                None
            }
        }
    }

    fn store_copied_keys(&mut self, clipboard: CopiedKeys) {
        self.paste_receipt = None;
        match clipboard {
            CopiedKeys::Single(copied) => {
                self.timeline_clipboard = Some(copied.clipboard);
                self.leaf_source = Some(copied.source);
                self.group_clipboard = None;
            }
            CopiedKeys::Group(keys) => {
                self.group_clipboard = Some(keys);
                self.timeline_clipboard = None;
                self.leaf_source = None;
            }
        }
    }
    pub(super) fn clear_timeline_clipboard(&mut self) {
        self.timeline_clipboard = None;
        self.leaf_source = None;
        self.group_clipboard = None;
        self.paste_receipt = None;
    }
    pub(super) fn timeline_clipboard_count(&self) -> usize {
        self.group_clipboard.as_ref().map_or_else(
            || self.timeline_clipboard.as_ref().map_or(0, clipboard_len),
            GroupClipboard::len,
        )
    }
    pub(super) fn timeline_clipboard_diagnostic(&self) -> serde_json::Value {
        let mut diagnostic = if let Some(group) = &self.group_clipboard {
            group.diagnostic()
        } else {
            self.timeline_clipboard.as_ref().map_or_else(|| serde_json::json!({"kind":"empty","keys":0,"entryCount":0}), |keys| {
                self.leaf_source.as_ref().map_or_else(||serde_json::json!({"kind":"single","version":2,"keys":clipboard_len(keys),"entryCount":1}),|source|source.diagnostic(keys))
            })
        };
        diagnostic["lastPaste"] = self
            .paste_receipt
            .as_ref()
            .map(PasteReceipt::diagnostic)
            .unwrap_or(serde_json::Value::Null);
        diagnostic
    }
    fn record_timeline_paste(&mut self, accepted: &[peachy_engine::AnimationKey]) {
        self.paste_receipt = Some(PasteReceipt::new(
            self.animation,
            self.time,
            accepted,
            self.key_selection.keys(),
            self.session.document_generation(),
        ));
    }
    fn matches_owned_clipboard(&self, text: &str) -> bool {
        self.group_clipboard.as_ref().map_or_else(
            || {
                self.timeline_clipboard
                    .as_ref()
                    .is_some_and(|clipboard| matches_clipboard(text, clipboard))
            },
            |clipboard| text == clipboard.text(),
        )
    }

    fn clipboard_status(&mut self, verb: &str, count: usize) {
        let unit = if count == 1 { "key" } else { "keys" };
        let notice = if self.session.pose_edits().is_empty() {
            ""
        } else {
            " · unkeyed pose retained"
        };
        self.status = format!("{verb} {count} {unit}{notice}");
    }

    pub(super) fn copy_dopesheet_keys(&mut self, ctx: &egui::Context) -> bool {
        let Some(clipboard) = self.selected_clipboard() else {
            return false;
        };
        let count = clipboard.len();
        ctx.copy_text(clipboard.text());
        self.store_copied_keys(clipboard);
        self.clipboard_status("Copied", count);
        true
    }

    pub(super) fn cut_dopesheet_keys(&mut self, ctx: &egui::Context) -> bool {
        let Some(clipboard) = self.selected_clipboard() else {
            return false;
        };
        let Some(command) = self.key_selection.deletion() else {
            return false;
        };
        self.playing = false;
        if !self.dispatch(command) {
            return false;
        }
        let count = clipboard.len();
        // A failed delete keeps the previous clipboard as well as document/history.
        ctx.copy_text(clipboard.text());
        self.store_copied_keys(clipboard);
        self.clipboard_status("Cut", count);
        true
    }

    pub(super) fn timeline_clipboard_shortcuts(&mut self, ctx: &egui::Context) {
        let focused = (self.dopesheet_has_focus(ctx)
            || self.deform_graph_has_focus(ctx)
            || self.slot_graph_has_focus(ctx)
            || self.scalar_graph_has_focus(ctx)
            || self.slider_graph_has_focus(ctx))
            && ctx.input(|input| input.focused)
            && self.clipboard_context_available();
        let actions = ctx.input_mut(|input| {
            take_timeline_actions(&mut input.events, focused, &mut self.timeline_input)
        });
        for action in actions {
            match action {
                TimelineAction::SelectAll => self.select_all_dopesheet_keys(),
                TimelineAction::Delete => self.delete_dopesheet_keys(),
                TimelineAction::Copy => {
                    self.copy_dopesheet_keys(ctx);
                }
                TimelineAction::Cut => {
                    self.cut_dopesheet_keys(ctx);
                }
                TimelineAction::PasteText(text) => {
                    if self.matches_owned_clipboard(&text) {
                        self.paste_dopesheet_keys();
                    } else {
                        if self.group_clipboard.take().is_some() {
                            self.paste_receipt = None;
                        }
                        self.status =
                            "Clipboard does not contain the keys copied in this project".into();
                    }
                }
                TimelineAction::PasteCurrent => {
                    self.paste_dopesheet_keys();
                }
            }
        }
    }

    pub(super) fn paste_dopesheet_keys(&mut self) -> bool {
        if !self.clipboard_context_available() {
            return false;
        }
        self.cancel_gesture();
        self.reconcile_dopesheet_selection();
        if !self.clipboard_context_available() {
            return false;
        }
        if let Some(group) = self.group_clipboard.clone() {
            return self.paste_key_group(group);
        }
        let Some(clipboard) = self.timeline_clipboard.clone() else {
            return false;
        };
        let target = match self.leaf_paste_target(&clipboard) {
            Ok(target) => target,
            Err(error) => {
                self.status = error;
                return false;
            }
        };
        let time = self.time;
        let accepted = match pasted_animation_keys(
            self.session.document(),
            self.animation,
            target.clone(),
            time,
            &clipboard,
        ) {
            Ok(keys) => keys,
            Err(error) => {
                self.status = error;
                return false;
            }
        };
        self.playing = false;
        if !self.dispatch(Command::PasteAnimationKeys {
            animation: self.animation,
            target,
            time,
            clipboard,
        }) {
            return false;
        }
        self.reconcile_dopesheet_selection();
        self.record_timeline_paste(&accepted);
        self.clipboard_status("Pasted", accepted.len());
        true
    }

    fn leaf_paste_target(
        &self,
        clipboard: &AnimationKeyClipboard,
    ) -> Result<peachy_engine::KeyOwner, String> {
        if let Some(TreeItem::Attachment(attachment)) = &self.selection
            && !self.attachment_editable(attachment)
        {
            return Err("The selected attachment is read-only in this skin.".into());
        }
        if matches!(clipboard, AnimationKeyClipboard::Bone { .. })
            && let Some(target) = self.bone_leaf_pose_target()?
        {
            return Ok(target);
        }
        // Explicit typed targets retain their shared kind/layout validation.
        if let Some(target) = self.timeline_owner() {
            if !self.timeline_owner_editable(&target) {
                return Err("The selected timeline owner is read-only in this skin.".into());
            }
            return Ok(target);
        }
        let target = self
            .leaf_source
            .as_ref()
            .ok_or("Select a compatible item or copy these keys in this project first.")?
            .target(clipboard)?;
        if !self.timeline_owner_editable(&target) {
            return Err("The original item for these keys is read-only in this skin.".into());
        }
        Ok(target)
    }

    fn bone_leaf_pose_target(&self) -> Result<Option<peachy_engine::KeyOwner>, String> {
        let doc = self.session.document();
        let slot = match self.selection.as_ref() {
            Some(TreeItem::Slot(slot) | TreeItem::DrawOrderSlot { slot }) => *slot,
            Some(TreeItem::Attachment(attachment))
                if matches!(doc.attachment(attachment), Some(Attachment::Region(_))) =>
            {
                doc.slots
                    .iter()
                    .position(|slot| slot.name == attachment.slot)
                    .ok_or("The selected region's slot no longer exists")?
            }
            _ => return Ok(None),
        };
        let bone = doc
            .slots
            .get(slot)
            .ok_or("The selected slot no longer exists")?
            .bone;
        if !doc.activity(&self.skin).bones[bone] {
            return Err("The selected item's parent bone is inactive in this skin.".into());
        }
        Ok(Some(peachy_engine::KeyOwner::Bone { bone }))
    }

    fn paste_key_group(&mut self, group: GroupClipboard) -> bool {
        let targets = match group.targets() {
            Ok(targets) => targets,
            Err(error) => {
                self.status = error;
                return false;
            }
        };
        if targets
            .iter()
            .any(|mapping| !self.timeline_owner_editable(&mapping.target))
        {
            self.status = "A copied key group's original owner is read-only in this skin.".into();
            return false;
        }
        let accepted = match pasted_animation_key_group(
            self.session.document(),
            self.animation,
            self.time,
            &group.clipboard,
            &targets,
        ) {
            Ok(keys) => keys,
            Err(error) => {
                self.status = error;
                return false;
            }
        };
        self.playing = false;
        if !self.dispatch(Command::PasteAnimationKeyGroup {
            animation: self.animation,
            time: self.time,
            clipboard: group.clipboard,
            targets,
        }) {
            return false;
        }
        self.reconcile_dopesheet_selection();
        self.record_timeline_paste(&accepted);
        self.clipboard_status("Pasted", accepted.len());
        true
    }

    pub(super) fn timeline_clipboard_toolbar(&mut self, ui: &mut egui::Ui) {
        let command = if cfg!(target_os = "macos") {
            "Cmd"
        } else {
            "Ctrl"
        };
        let context = self.clipboard_context_available();
        let selected = context && !self.key_selection.is_empty();
        let copy = icons::toolbar_button(
            ui,
            Icon::Copy,
            "Copy keys",
            &format!("{command}+C"),
            selected,
        );
        self.record_target("timelineCopy", copy.rect);
        if copy.clicked() && self.copy_dopesheet_keys(ui.ctx()) {
            ui.memory_mut(|memory| memory.request_focus(Self::dopesheet_id()));
        }
        let cut =
            icons::toolbar_button(ui, Icon::Cut, "Cut keys", &format!("{command}+X"), selected);
        self.record_target("timelineCut", cut.rect);
        if cut.clicked() && self.cut_dopesheet_keys(ui.ctx()) {
            ui.memory_mut(|memory| memory.request_focus(Self::dopesheet_id()));
        }
        let delete =
            icons::toolbar_button(ui, Icon::Delete, "Delete selected keys", "Delete", selected);
        self.record_target("timelineDelete", delete.rect);
        if delete.clicked() {
            self.delete_dopesheet_keys();
            ui.memory_mut(|memory| memory.request_focus(Self::dopesheet_id()));
        }
        let can_paste = context
            && if let Some(group) = &self.group_clipboard {
                group.len() > 0 && group.targets().is_ok()
            } else {
                (self.timeline_owner().is_some()
                    || self
                        .leaf_source
                        .as_ref()
                        .is_some_and(LeafSourceReceipt::has_owner))
                    && self
                        .timeline_clipboard
                        .as_ref()
                        .is_some_and(|keys| clipboard_len(keys) > 0)
            };
        let paste = icons::toolbar_button(
            ui,
            Icon::Paste,
            "Paste keys",
            &format!("{command}+V"),
            can_paste,
        );
        self.record_target("timelinePaste", paste.rect);
        if paste.clicked() && self.paste_dopesheet_keys() {
            ui.memory_mut(|memory| memory.request_focus(Self::dopesheet_id()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::dopesheet_selection::KeyId;
    use super::super::test_support::{
        editor_context, editor_fixture, select_rotation, workspace_frame,
    };
    use super::*;

    #[test]
    fn grouped_ik_copy_paste_rejects_bone_target_and_retains_f64_handles() {
        let mut editor = super::super::test_support::animated_ik_editor();
        let mut document = editor.session.document().clone();
        let curve = IkCurve::Bezier {
            time: [0.123456789123, 0.876543219876],
            mix: [-0.25, 1.5],
        };
        document.animations[0].ik_tracks[0].keys[0].curve = curve;
        editor.session = Session::new(document).unwrap();
        editor.reconcile_dopesheet_selection();
        editor.key_selection.replace(vec![KeyId::Ik {
            constraint: 1,
            time: 0.,
        }]);
        let ctx = editor_context();
        assert!(editor.copy_dopesheet_keys(&ctx));
        let copied = editor.timeline_clipboard.clone().unwrap();
        assert!(matches_clipboard(&clipboard_text(&copied), &copied));
        editor.select(TreeItem::Bone(1));
        let before = editor.session.document().clone();
        assert!(!editor.paste_dopesheet_keys());
        assert_eq!(editor.session.document(), &before);
        editor.select(TreeItem::Constraint(1));
        editor.set_time(2.);
        assert!(editor.paste_dopesheet_keys());
        let track = &editor.session.document().animations[0].ik_tracks[0];
        assert_eq!(track.keys.last().unwrap().curve, curve);
        assert_eq!(track.keys.last().unwrap().time, 2.);
        assert!(editor.key_selection.is_empty());
        assert_eq!(
            editor.timeline_clipboard_diagnostic()["lastPaste"]["acceptedKeys"],
            serde_json::json!([KeyId::Ik {
                constraint: 1,
                time: 2.
            }])
        );
        editor.undo();
        assert_eq!(editor.session.document(), &before);
        assert!(editor.key_selection.is_empty());
    }

    fn missing_attachment_paste_fixture(with_color: bool) -> (Editor, egui::Context, String) {
        let mut editor = editor_fixture();
        let mut document = editor.session.document().clone();
        document.animations[0].attachment_tracks = vec![AttachmentTrack {
            slot: 0,
            keys: vec![AttachmentKey {
                time: 0.75,
                attachment: document.slots[0].attachment.clone(),
            }],
        }];
        document.animations[0]
            .color_tracks
            .retain(|track| track.slot != 0);
        editor.session = Session::new(document).unwrap();
        editor.select(TreeItem::Slot(0));
        editor.time = 0.75;
        editor.reconcile_selection();
        let clipboard = AnimationKeyClipboard::Slot {
            attachment_keys: vec![AttachmentKey {
                time: 0.,
                attachment: Some("missing-other-slot-art".into()),
            }],
            color_keys: if with_color {
                vec![ColorKey {
                    time: 0.,
                    color: [23, 61, 121, 201],
                    curves: [Curve::Linear; 4],
                }]
            } else {
                vec![]
            },
        };
        let text = clipboard_text(&clipboard);
        editor.timeline_clipboard = Some(clipboard);
        let ctx = editor_context();
        for _ in 0..3 {
            workspace_frame(&mut editor, &ctx, vec![]);
        }
        ctx.memory_mut(|memory| memory.request_focus(Editor::dopesheet_id()));
        (editor, ctx, text)
    }

    #[test]
    fn native_slot_paste_skips_unknown_attachment_selection_and_delete_keeps_existing_key() {
        let (mut editor, ctx, text) = missing_attachment_paste_fixture(true);
        let original = editor.session.document().clone();
        workspace_frame(&mut editor, &ctx, vec![egui::Event::Paste(text)]);
        assert!(editor.key_selection.is_empty());
        assert_eq!(
            editor.timeline_clipboard_diagnostic()["lastPaste"]["acceptedKeys"],
            serde_json::json!([KeyId::SlotColor {
                slot: 0,
                time: 0.75
            }])
        );
        assert!(editor.status.starts_with("Pasted 1 key"));
        assert_eq!(
            editor.session.document().animations[0].attachment_tracks,
            original.animations[0].attachment_tracks
        );
        assert_eq!(
            editor.session.document().animations[0].color_tracks[0].keys[0].color,
            [23, 61, 121, 201]
        );
        let pasted = editor.session.document().clone();
        workspace_frame(&mut editor, &ctx, vec![delete_event(egui::Key::Delete)]);
        assert_eq!(editor.session.document(), &pasted);
        assert!(editor.key_selection.is_empty());
        editor.undo();
        assert_eq!(editor.session.document(), &original);
    }

    #[test]
    fn native_all_skipped_slot_paste_preserves_document_history_and_valid_prior_selection() {
        let (mut editor, ctx, text) = missing_attachment_paste_fixture(false);
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "redo sentinel".into(),
            })
            .unwrap();
        editor.session.undo();
        assert!(!editor.session.can_undo());
        assert!(editor.session.can_redo());
        editor.key_selection.replace(vec![KeyId::SlotAttachment {
            slot: 0,
            time: 0.75,
        }]);
        let original = editor.session.document().clone();
        let pending = editor.session.pose_edits().clone();
        workspace_frame(&mut editor, &ctx, vec![egui::Event::Paste(text)]);
        assert_eq!(
            editor.key_selection.keys(),
            &[KeyId::SlotAttachment {
                slot: 0,
                time: 0.75
            }]
        );
        assert!(editor.status.starts_with("Pasted 0 keys"));
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert!(!editor.session.can_undo());
        assert!(editor.session.can_redo());
        workspace_frame(&mut editor, &ctx, vec![delete_event(egui::Key::Delete)]);
        assert!(
            editor.session.document().animations[0]
                .attachment_tracks
                .is_empty()
        );
        assert!(editor.session.can_undo());
        assert!(!editor.session.can_redo());
        editor.undo();
        assert_eq!(editor.session.document(), &original);
    }

    fn clipboard() -> AnimationKeyClipboard {
        AnimationKeyClipboard::Bone {
            keys: vec![peachy_engine::ClipboardKey {
                property: Property::Rotation,
                key: Keyframe {
                    time: 0.,
                    value: 42.,
                    curve: Curve::Bezier([0.2, -0.3, 0.7, 1.2]),
                },
            }],
        }
    }

    #[test]
    fn clipboard_mirror_is_versioned_and_carries_only_owned_key_data() {
        let clipboard = clipboard();
        let text = clipboard_text(&clipboard);
        let envelope: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(envelope["peachyAnimatorClipboard"], "keys");
        assert_eq!(envelope["version"], 2);
        assert_eq!(
            envelope["payload"],
            serde_json::to_value(&clipboard).unwrap()
        );
        assert!(!text.contains("\"bone\":"));
        assert!(!text.contains("path"));
        assert!(matches_clipboard(&text, &clipboard));
    }

    #[test]
    fn external_stale_or_altered_clipboard_text_cannot_paste_session_keys() {
        let clipboard = clipboard();
        let mut changed = clipboard.clone();
        let AnimationKeyClipboard::Bone { keys } = &mut changed else {
            unreachable!()
        };
        keys[0].key.value = 43.;
        assert!(!matches_clipboard("external text", &clipboard));
        assert!(!matches_clipboard(&clipboard_text(&changed), &clipboard));
        let mut envelope: serde_json::Value =
            serde_json::from_str(&clipboard_text(&clipboard)).unwrap();
        envelope["version"] = 1.into();
        assert!(!matches_clipboard(&envelope.to_string(), &clipboard));
        envelope["version"] = 2.into();
        envelope["peachyAnimatorClipboard"] = "another application".into();
        assert!(!matches_clipboard(&envelope.to_string(), &clipboard));
    }

    #[test]
    fn native_clipboard_events_preserve_order_and_text_focus_keeps_all_input() {
        let events = vec![
            egui::Event::Text("editing".into()),
            egui::Event::Copy,
            egui::Event::Cut,
            egui::Event::Paste("payload".into()),
        ];
        let mut text_events = events.clone();
        assert!(
            take_timeline_actions(&mut text_events, false, &mut TimelineInput::default())
                .is_empty()
        );
        assert_eq!(text_events, events);
        let mut chart_events = events;
        assert_eq!(
            take_timeline_actions(&mut chart_events, true, &mut TimelineInput::default()),
            vec![
                TimelineAction::Copy,
                TimelineAction::Cut,
                TimelineAction::PasteText("payload".into())
            ]
        );
        assert_eq!(chart_events, vec![egui::Event::Text("editing".into())]);
    }

    #[test]
    fn raw_clipboard_key_fallback_does_not_consume_plain_translate_shortcut() {
        let key = |modifiers| egui::Event::Key {
            key: egui::Key::V,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        let plain = key(egui::Modifiers::NONE);
        let mut events = vec![plain.clone(), key(egui::Modifiers::COMMAND)];
        assert_eq!(
            take_timeline_actions(&mut events, true, &mut TimelineInput::default()),
            vec![TimelineAction::PasteCurrent]
        );
        assert_eq!(events, vec![plain]);
    }

    #[test]
    fn mixed_native_and_raw_events_cannot_duplicate_actions_or_bypass_external_text_rejection() {
        let raw = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        };
        let mut events = vec![
            raw(egui::Key::V),
            egui::Event::Paste("external text".into()),
            egui::Event::Copy,
            raw(egui::Key::C),
            raw(egui::Key::X),
            egui::Event::Cut,
        ];
        let actions = take_timeline_actions(&mut events, true, &mut TimelineInput::default());
        assert_eq!(
            actions,
            vec![
                TimelineAction::PasteText("external text".into()),
                TimelineAction::Copy,
                TimelineAction::Cut,
            ]
        );
        assert!(!matches_clipboard("external text", &clipboard()));
        assert!(events.is_empty());
    }

    fn a_event(pressed: bool, repeat: bool, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed,
            repeat,
            modifiers,
        }
    }

    #[test]
    fn select_all_latch_requires_release_and_keeps_clipboard_actions_in_order() {
        let down = || a_event(true, false, egui::Modifiers::COMMAND);
        let up = || a_event(false, false, egui::Modifiers::NONE);
        let mut state = TimelineInput::default();
        let mut events = vec![down(), egui::Event::Copy, down(), up()];
        assert_eq!(
            take_timeline_actions(&mut events, true, &mut state),
            vec![TimelineAction::SelectAll, TimelineAction::Copy]
        );
        assert!(!state.select_all_held());
        assert_eq!(events, vec![up()]);

        let mut events = vec![down(), up(), down(), egui::Event::Copy, up()];
        assert_eq!(
            take_timeline_actions(&mut events, true, &mut state),
            vec![
                TimelineAction::SelectAll,
                TimelineAction::SelectAll,
                TimelineAction::Copy
            ]
        );
        assert!(!state.select_all_held());

        let mut events = vec![down()];
        assert_eq!(
            take_timeline_actions(&mut events, true, &mut state),
            vec![TimelineAction::SelectAll]
        );
        let mut next_batch = vec![down(), a_event(true, true, egui::Modifiers::COMMAND)];
        assert!(take_timeline_actions(&mut next_batch, true, &mut state).is_empty());
        assert!(next_batch.is_empty());
        let mut released = vec![up(), down()];
        assert_eq!(
            take_timeline_actions(&mut released, true, &mut state),
            vec![TimelineAction::SelectAll]
        );
    }

    #[test]
    fn select_all_repeats_do_not_advance_and_other_controls_keep_their_a_events() {
        let mut state = TimelineInput::default();
        let mut repeated = vec![a_event(true, true, egui::Modifiers::COMMAND)];
        assert!(take_timeline_actions(&mut repeated, true, &mut state).is_empty());
        assert!(state.select_all_held());
        let untouched = vec![
            a_event(false, false, egui::Modifiers::NONE),
            a_event(true, false, egui::Modifiers::COMMAND),
            delete_event(egui::Key::Delete),
            delete_event(egui::Key::Backspace),
            egui::Event::Text("text editing".into()),
        ];
        let mut text_events = untouched.clone();
        assert!(take_timeline_actions(&mut text_events, false, &mut state).is_empty());
        assert_eq!(text_events, untouched);
        let plain = vec![
            a_event(true, false, egui::Modifiers::NONE),
            a_event(false, false, egui::Modifiers::NONE),
        ];
        let mut events = plain.clone();
        assert!(take_timeline_actions(&mut events, true, &mut state).is_empty());
        assert_eq!(events, plain);
    }

    fn editor_frame(
        editor: &mut Editor,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        text_focus: bool,
    ) {
        let mut output = ctx.run_ui(
            egui::RawInput {
                events,
                focused: true,
                ..Default::default()
            },
            |ui| {
                let ctx = ui.ctx().clone();
                let canvas = ui.interact(
                    ui.max_rect(),
                    Editor::dopesheet_id(),
                    Sense::click_and_drag(),
                );
                if text_focus {
                    let mut text = "editable".to_owned();
                    let text = ui
                        .add(egui::TextEdit::singleline(&mut text).id(egui::Id::new("test-field")));
                    text.request_focus();
                } else {
                    canvas.request_focus();
                }
                editor.reconcile_selection();
                editor.shortcuts(&ctx);
                editor.observe_timeline_input(&ctx);
                editor.reconcile_selection();
            },
        );
        // These input/controller checks intentionally have no GPU texture backend.
        output.textures_delta.clear();
    }

    fn a_cycle() -> Vec<egui::Event> {
        vec![
            a_event(true, false, egui::Modifiers::COMMAND),
            a_event(false, false, egui::Modifiers::NONE),
        ]
    }

    #[test]
    fn editor_select_all_and_copy_follow_event_order_without_touching_pose_time_or_history() {
        let mut editor = editor_fixture();
        let original = editor.session.document().clone();
        let mut pose = editor.session.local_pose(Some(0), editor.time)[8];
        pose.rotation += 12.;
        editor
            .session
            .edit_pose(
                8,
                EditTarget::Animate {
                    animation: 0,
                    time: editor.time,
                },
                pose,
                &[Property::Rotation],
                false,
            )
            .unwrap();
        let pending = editor.session.pose_edits().clone();
        let ctx = editor_context();
        let mut events = a_cycle();
        events.insert(1, egui::Event::Copy);
        editor_frame(&mut editor, &ctx, events.clone(), false);
        assert_eq!(editor.key_selection.len(), 4);
        assert_eq!(
            clipboard_len(editor.timeline_clipboard.as_ref().unwrap()),
            4
        );
        editor_frame(&mut editor, &ctx, events, false);
        assert_eq!(editor.key_selection.len(), 9);
        assert_eq!(
            clipboard_len(editor.timeline_clipboard.as_ref().unwrap()),
            9
        );
        select_rotation(&mut editor);
        let mut events = a_cycle();
        events.insert(0, egui::Event::Copy);
        editor_frame(&mut editor, &ctx, events, false);
        assert_eq!(editor.key_selection.len(), 4);
        assert_eq!(
            clipboard_len(editor.timeline_clipboard.as_ref().unwrap()),
            1
        );
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(editor.time, 0.42);
        assert!(!editor.playing);
        assert!(!editor.session.is_dirty());
        editor.session.undo();
        assert!(editor.session.pose_edits().is_empty());
        assert!(!editor.session.can_undo());
        assert!(editor.session.can_redo());
        editor_frame(&mut editor, &ctx, a_cycle(), false);
        assert!(editor.session.can_redo());
        assert!(!editor.session.can_undo());
    }

    #[test]
    fn editor_latch_observes_release_in_modal_and_text_fields_without_routing_selection() {
        let mut editor = editor_fixture();
        let original = editor.session.document().clone();
        let ctx = editor_context();
        editor_frame(
            &mut editor,
            &ctx,
            vec![a_event(true, false, egui::Modifiers::COMMAND)],
            false,
        );
        assert_eq!(editor.key_selection.len(), 4);
        assert!(editor.timeline_input.select_all_held());
        editor_frame(
            &mut editor,
            &ctx,
            vec![a_event(true, false, egui::Modifiers::COMMAND)],
            false,
        );
        assert_eq!(editor.key_selection.len(), 4);
        editor.begin_rename(TreeItem::Bone(8));
        editor_frame(
            &mut editor,
            &ctx,
            vec![a_event(false, false, egui::Modifiers::NONE)],
            false,
        );
        assert!(!editor.timeline_input.select_all_held());
        assert_eq!(editor.key_selection.len(), 4);
        editor.rename = None;
        editor_frame(&mut editor, &ctx, a_cycle(), false);
        assert_eq!(editor.key_selection.len(), 9);
        select_rotation(&mut editor);
        editor_frame(&mut editor, &ctx, Vec::new(), true);
        let mut text_events = a_cycle();
        text_events.push(delete_event(egui::Key::Backspace));
        editor_frame(&mut editor, &ctx, text_events, true);
        assert_eq!(editor.key_selection.len(), 1);
        assert!(!editor.timeline_input.select_all_held());
        editor_frame(&mut editor, &ctx, a_cycle(), false);
        assert_eq!(editor.key_selection.len(), 4);
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
        editor.timeline_input.select_all_held = true;
        let mut output = ctx.run_ui(
            egui::RawInput {
                focused: false,
                ..Default::default()
            },
            |ui| {
                let before = ui.ctx().input(|input| input.events.clone());
                editor.observe_timeline_input(ui.ctx());
                assert_eq!(ui.ctx().input(|input| input.events.clone()), before);
            },
        );
        output.textures_delta.clear();
        assert!(!editor.timeline_input.select_all_held());
    }

    #[test]
    fn editor_two_complete_a_chords_in_one_batch_progress_exactly_twice() {
        let mut editor = editor_fixture();
        let ctx = editor_context();
        let mut events = a_cycle();
        events.extend(a_cycle());
        editor_frame(&mut editor, &ctx, events, false);
        assert_eq!(editor.key_selection.len(), 9);
        assert!(!editor.timeline_input.select_all_held());
        assert!(!editor.playing);
        assert_eq!(editor.play_direction, PlayDirection::Forward);
        assert_eq!(editor.time, 0.42);
        assert!(!editor.session.can_undo());
        let plain = || {
            vec![
                a_event(true, false, egui::Modifiers::NONE),
                a_event(false, false, egui::Modifiers::NONE),
            ]
        };
        editor_frame(&mut editor, &ctx, plain(), false);
        assert!(editor.playing);
        assert_eq!(editor.play_direction, PlayDirection::Backward);
        editor_frame(&mut editor, &ctx, plain(), false);
        assert!(!editor.playing);
        assert_eq!(editor.key_selection.len(), 9);
    }

    #[test]
    fn editor_select_all_cancels_preview_and_expands_prepress_rows_without_losing_redo() {
        let mut editor = editor_fixture();
        let mut pose = editor.session.local_pose(Some(0), editor.time)[8];
        pose.rotation += 12.;
        editor
            .session
            .edit_pose(
                8,
                EditTarget::Animate {
                    animation: 0,
                    time: editor.time,
                },
                pose,
                &[Property::Rotation],
                false,
            )
            .unwrap();
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "redo branch".into(),
            })
            .unwrap();
        editor.session.undo();
        let original = editor.session.document().clone();
        let pending = editor.session.pose_edits().clone();
        assert!(editor.session.can_redo());
        let selected = KeyId::Bone {
            bone: 8,
            property: Property::X,
            time: 0.,
        };
        editor.key_selection.replace(vec![selected.clone()]);
        let geometry = super::super::key_drag::ChartGeometry {
            start: 0.,
            width: 240.,
            extent: 2.4,
        };
        editor.key_drag = super::super::key_drag::KeyDrag::new(
            0,
            KeyId::Bone {
                bone: 8,
                property: Property::Rotation,
                time: 0.3,
            },
            editor.key_selection.keys(),
            geometry,
            30.,
            egui::Modifiers::NONE,
        );
        editor.update_dopesheet_drag(60., false);
        assert!(editor.key_drag.as_ref().unwrap().active);
        assert_ne!(editor.session.document(), &original);
        assert!(editor.key_selection.contains(KeyId::Bone {
            bone: 8,
            property: Property::Rotation,
            time: 0.6
        }));
        let ctx = editor_context();
        editor_frame(&mut editor, &ctx, a_cycle(), false);
        assert!(editor.key_drag.is_none());
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert!(editor.session.can_redo());
        assert!(!editor.session.is_dirty());
        assert_eq!(editor.key_selection.len(), 2);
        assert!(editor.key_selection.keys().iter().all(|key| matches!(
            key,
            KeyId::Bone {
                property: Property::X,
                ..
            }
        )));
        assert_eq!(editor.time, 0.42);
    }

    fn delete_event(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn editor_delete_and_select_all_respect_the_original_keyboard_event_order() {
        let ctx = editor_context();
        let mut editor = editor_fixture();
        let original = editor.session.document().clone();
        let expected = peachy_engine::apply_command(
            &original,
            Command::DeleteKey {
                animation: 0,
                bone: 8,
                property: Property::Rotation,
                time: 0.3,
            },
        )
        .unwrap();
        let mut events = vec![delete_event(egui::Key::Delete)];
        events.extend(a_cycle());
        editor_frame(&mut editor, &ctx, events, false);
        assert_eq!(editor.session.document(), &expected);
        assert_eq!(editor.key_selection.len(), 8);
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());

        select_rotation(&mut editor);
        let mut events = a_cycle();
        events.push(delete_event(egui::Key::Backspace));
        editor_frame(&mut editor, &ctx, events, false);
        assert!(editor.key_selection.is_empty());
        assert!(
            !editor.session.document().animations[0]
                .tracks
                .iter()
                .any(|track| track.bone == 8 && track.property == Property::Rotation)
        );
        assert!(
            editor.session.document().animations[0]
                .tracks
                .iter()
                .any(|track| track.bone == 7 && track.property == Property::Rotation)
        );
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
    }
}
