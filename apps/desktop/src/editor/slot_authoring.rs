//! Contextual slot commands share the engine's sampled and pending authored pose.
use super::*;
use peachy_engine::{SlotPose, SlotPoseEdit, SlotProperty};

impl Editor {
    pub(super) fn selected_slot(&self) -> Option<usize> {
        match self.selection.as_ref()? {
            TreeItem::Slot(slot)
            | TreeItem::DrawOrderSlot { slot }
            | TreeItem::Placeholder { slot, .. }
                if *slot < self.session.document().slots.len() =>
            {
                Some(*slot)
            }
            TreeItem::Attachment(key) if self.attachment_editable(key) => self
                .session
                .document()
                .slots
                .iter()
                .position(|slot| slot.name == key.slot),
            _ => None,
        }
    }

    pub(super) fn slot_authoring_input_available(&self, ctx: &egui::Context) -> bool {
        ctx.input(|input| input.focused)
            && self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
            && self.export_dialog.is_none()
            && !egui::Popup::is_any_open(ctx)
            && !self.slot_color_dialog_open()
    }

    pub(super) fn current_slot_pose(&self, slot: usize) -> Result<SlotPose, String> {
        self.session.slot_pose(slot, self.weights_edit_target())
    }

    pub(super) fn slot_property_state(
        &self,
        slot: usize,
        property: SlotProperty,
    ) -> Result<KeyState, String> {
        self.session.pose_edits().slot_key_state(
            self.session.document(),
            self.animation,
            self.time,
            slot,
            property,
        )
    }

    pub(super) fn key_slot_property(&mut self, slot: usize, property: SlotProperty) {
        self.request_pose_key(pose_key::PoseKeyAction::Slot {
            slot,
            properties: vec![property],
        });
    }

    pub(super) fn delete_slot_property_key(&mut self, slot: usize, property: SlotProperty) {
        if self.setup_mode || self.slot_property_state(slot, property) != Ok(KeyState::Keyed) {
            return;
        }
        match peachy_engine::slot_key_time(
            self.session.document(),
            self.animation,
            self.time,
            slot,
            property,
        ) {
            Ok(Some(time)) => {
                self.cancel_gesture();
                let key = match property {
                    SlotProperty::Attachment => {
                        peachy_engine::AnimationKey::SlotAttachment { slot, time }
                    }
                    SlotProperty::Color => peachy_engine::AnimationKey::SlotColor { slot, time },
                };
                self.dispatch(Command::DeleteAnimationKeys {
                    animation: self.animation,
                    keys: vec![key],
                });
            }
            Ok(None) => {}
            Err(error) => self.status = error,
        }
    }

    pub(super) fn slot_property_key_controls(
        &mut self,
        ui: &mut egui::Ui,
        slot: usize,
        property: SlotProperty,
        prefix: &str,
    ) {
        if self.setup_mode {
            return;
        }
        match self.slot_property_state(slot, property) {
            Ok(state) => {
                let enabled = self.slot_authoring_input_available(ui.ctx());
                let key = ui
                    .add_enabled_ui(enabled, |ui| super::animation::pose_key_button(ui, state))
                    .inner;
                self.record_target(&format!("{prefix}Key"), key.rect);
                let key = key
                    .on_hover_text("Key the current slot property. Right-click removes its key.");
                if key.clicked() {
                    self.key_slot_property(slot, property);
                } else if key.secondary_clicked() {
                    self.delete_slot_property_key(slot, property);
                }
                let delete = ui.add_enabled(
                    enabled && state == KeyState::Keyed,
                    egui::Button::new("Delete key").small(),
                );
                self.record_target(&format!("{prefix}Delete"), delete.rect);
                if delete.clicked() {
                    self.delete_slot_property_key(slot, property);
                }
            }
            Err(error) => {
                ui.colored_label(ACCENT, "Key timing error")
                    .on_hover_text(error);
            }
        }
    }

    pub(super) fn edit_slot_attachment(&mut self, slot: usize, attachment: Option<String>) {
        if self
            .current_slot_pose(slot)
            .is_ok_and(|pose| pose.attachment == attachment)
        {
            return;
        }
        self.cancel_gesture();
        self.playing = false;
        match self.session.edit_slot_pose(
            slot,
            self.weights_edit_target(),
            &[SlotPoseEdit::Attachment { attachment }],
            self.auto_key,
        ) {
            Ok(()) => {
                self.status = if self.setup_mode {
                    "Setup attachment updated"
                } else if self.auto_key {
                    "Keyed slot attachment"
                } else {
                    "Unkeyed slot attachment · K to key"
                }
                .into()
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub(super) fn shown_slot_attachment(&self, slot: usize) -> Option<AttachmentRef> {
        let pose = self.current_slot_pose(slot).ok()?;
        let name = pose.attachment.as_deref()?;
        self.session
            .document()
            .resolve_attachment(&self.skin, slot, name)
            .map(|(key, _)| key)
    }
}

#[cfg(test)]
#[path = "slot_authoring_tests.rs"]
mod tests;
