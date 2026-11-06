//! Clipboard presentation coordinates the shared key commands and editor-session state.
use super::dopesheet_selection::KeyId;
use super::icons::Icon;
use super::*;
use peachy_engine::{KeyClipboard, TimelineKey, copy_keys};

fn clipboard_text(clipboard: &KeyClipboard) -> String {
    serde_json::json!({
        "peachyAnimatorClipboard": "keys",
        "version": 1,
        "payload": clipboard,
    })
    .to_string()
}

fn matches_clipboard(text: &str, clipboard: &KeyClipboard) -> bool {
    text == clipboard_text(clipboard)
}

#[derive(Debug, PartialEq)]
enum ClipboardAction {
    Copy,
    Cut,
    PasteText(String),
    PasteCurrent,
}

/// Native hosts translate clipboard shortcuts into dedicated events, not Key presses.
/// Preserve every event when text or another control owns input.
fn take_clipboard_actions(events: &mut Vec<egui::Event>, focused: bool) -> Vec<ClipboardAction> {
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
            egui::Event::Copy => ClipboardAction::Copy,
            egui::Event::Cut => ClipboardAction::Cut,
            egui::Event::Paste(text) => ClipboardAction::PasteText(text),
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if modifiers.matches_exact(egui::Modifiers::COMMAND)
                && matches!(key, egui::Key::C | egui::Key::X | egui::Key::V) =>
            {
                match key {
                    egui::Key::C if !native_copy => ClipboardAction::Copy,
                    egui::Key::X if !native_cut => ClipboardAction::Cut,
                    egui::Key::V if !native_paste => ClipboardAction::PasteCurrent,
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
    fn clipboard_context_available(&self) -> bool {
        !self.setup_mode
            && self.show_dopesheet
            && self.selected_bone().is_some()
            && self
                .session
                .document()
                .animations
                .get(self.animation)
                .is_some()
            && self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
    }

    /// Read only committed keys. Pending pose edits never enter the clipboard.
    fn selected_clipboard(&mut self) -> Option<KeyClipboard> {
        self.cancel_gesture();
        self.reconcile_dopesheet_selection();
        if !self.clipboard_context_available() || self.key_selection.is_empty() {
            return None;
        }
        let bone = self.selected_bone()?;
        let keys: Vec<_> = self
            .key_selection
            .keys()
            .iter()
            .map(|key| TimelineKey {
                bone,
                property: key.property,
                time: key.time,
            })
            .collect();
        match copy_keys(self.session.document(), self.animation, &keys) {
            Ok(clipboard) => Some(clipboard),
            Err(error) => {
                self.error = Some(error);
                None
            }
        }
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
        let count = clipboard.keys.len();
        ctx.copy_text(clipboard_text(&clipboard));
        self.timeline_clipboard = Some(clipboard);
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
        let count = clipboard.keys.len();
        // A failed delete keeps the previous clipboard as well as document/history.
        ctx.copy_text(clipboard_text(&clipboard));
        self.timeline_clipboard = Some(clipboard);
        self.clipboard_status("Cut", count);
        true
    }

    pub(super) fn timeline_clipboard_shortcuts(&mut self, ctx: &egui::Context) {
        let focused = self.dopesheet_has_focus(ctx);
        let actions = ctx.input_mut(|input| take_clipboard_actions(&mut input.events, focused));
        for action in actions {
            match action {
                ClipboardAction::Copy => {
                    self.copy_dopesheet_keys(ctx);
                }
                ClipboardAction::Cut => {
                    self.cut_dopesheet_keys(ctx);
                }
                ClipboardAction::PasteText(text) => {
                    if self
                        .timeline_clipboard
                        .as_ref()
                        .is_some_and(|clipboard| matches_clipboard(&text, clipboard))
                    {
                        self.paste_dopesheet_keys();
                    } else {
                        self.status =
                            "Clipboard does not contain the keys copied in this project".into();
                    }
                }
                ClipboardAction::PasteCurrent => {
                    self.paste_dopesheet_keys();
                }
            }
        }
    }

    pub(super) fn paste_dopesheet_keys(&mut self) -> bool {
        self.cancel_gesture();
        self.reconcile_dopesheet_selection();
        if !self.clipboard_context_available() {
            return false;
        }
        let (Some(bone), Some(clipboard)) = (self.selected_bone(), self.timeline_clipboard.clone())
        else {
            return false;
        };
        let time = self.time;
        let selected: Vec<_> = clipboard
            .keys
            .iter()
            .map(|key| KeyId {
                property: key.property,
                time: key.key.time + time,
            })
            .collect();
        self.playing = false;
        if !self.dispatch(Command::PasteKeys {
            animation: self.animation,
            bone,
            time,
            clipboard,
        }) {
            return false;
        }
        let count = selected.len();
        self.key_selection.replace(selected);
        self.reconcile_dopesheet_selection();
        self.clipboard_status("Pasted", count);
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
            && self
                .timeline_clipboard
                .as_ref()
                .is_some_and(|c| !c.keys.is_empty());
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
    use super::*;

    fn clipboard() -> KeyClipboard {
        KeyClipboard {
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
        assert_eq!(envelope["version"], 1);
        assert_eq!(
            envelope["payload"],
            serde_json::to_value(&clipboard).unwrap()
        );
        assert!(!text.contains("bone"));
        assert!(!text.contains("path"));
        assert!(matches_clipboard(&text, &clipboard));
    }

    #[test]
    fn external_stale_or_altered_clipboard_text_cannot_paste_session_keys() {
        let clipboard = clipboard();
        let mut changed = clipboard.clone();
        changed.keys[0].key.value = 43.;
        assert!(!matches_clipboard("external text", &clipboard));
        assert!(!matches_clipboard(&clipboard_text(&changed), &clipboard));
        let mut envelope: serde_json::Value =
            serde_json::from_str(&clipboard_text(&clipboard)).unwrap();
        envelope["version"] = 2.into();
        assert!(!matches_clipboard(&envelope.to_string(), &clipboard));
        envelope["version"] = 1.into();
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
        assert!(take_clipboard_actions(&mut text_events, false).is_empty());
        assert_eq!(text_events, events);
        let mut chart_events = events;
        assert_eq!(
            take_clipboard_actions(&mut chart_events, true),
            vec![
                ClipboardAction::Copy,
                ClipboardAction::Cut,
                ClipboardAction::PasteText("payload".into())
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
            take_clipboard_actions(&mut events, true),
            vec![ClipboardAction::PasteCurrent]
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
        let actions = take_clipboard_actions(&mut events, true);
        assert_eq!(
            actions,
            vec![
                ClipboardAction::PasteText("external text".into()),
                ClipboardAction::Copy,
                ClipboardAction::Cut,
            ]
        );
        assert!(!matches_clipboard("external text", &clipboard()));
        assert!(events.is_empty());
    }
}
