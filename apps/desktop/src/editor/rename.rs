//! Dialog presentation shared by name-based authoring operations.
use super::*;

pub(super) struct NameDialog {
    title: String,
    pub name: String,
    pub error: Option<String>,
    first_frame: bool,
}

impl NameDialog {
    pub fn new(title: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            name: name.into(),
            error: None,
            first_frame: true,
        }
    }

    pub fn reject(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
        self.first_frame = true;
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum DialogAction {
    None,
    Confirm,
    Cancel,
}

pub(super) struct DialogOutput {
    pub action: DialogAction,
    pub name: egui::Rect,
    pub confirm: egui::Rect,
    pub cancel: egui::Rect,
}

/// This helper renders controls only. The caller owns the target and shared command.
pub(super) fn show_name_dialog(
    ctx: &egui::Context,
    id: egui::Id,
    state: &mut NameDialog,
) -> DialogOutput {
    let response = egui::Modal::new(id).show(ctx, |ui| {
        ui.set_min_width(300.);
        ui.heading(&state.title);
        ui.separator();
        let name = ui
            .horizontal(|ui| {
                ui.label("Name");
                egui::TextEdit::singleline(&mut state.name)
                    .desired_width(260.)
                    .show(ui)
            })
            .inner;
        let mut name = name;
        if state.first_frame {
            name.response.request_focus();
            name.state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(state.name.chars().count()),
                )));
            name.state.store(ctx, name.response.id);
            state.first_frame = false;
        }
        if let Some(error) = &state.error {
            ui.colored_label(Color32::from_rgb(230, 140, 130), error);
        }
        let (confirm, cancel) = ui
            .horizontal(|ui| (ui.button("OK"), ui.button("Cancel")))
            .inner;
        DialogOutput {
            action: if cancel.clicked() {
                DialogAction::Cancel
            } else if confirm.clicked() || ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                DialogAction::Confirm
            } else {
                DialogAction::None
            },
            name: name.response.rect,
            confirm: confirm.rect,
            cancel: cancel.rect,
        }
    });
    let close = response.should_close();
    let mut output = response.inner;
    if close {
        output.action = DialogAction::Cancel;
    }
    output
}

pub(super) struct RenameState {
    item: TreeItem,
    dialog: NameDialog,
}

impl Editor {
    pub(super) fn begin_rename(&mut self, item: TreeItem) {
        let kind = match item {
            TreeItem::Bone(_) => "bone",
            TreeItem::Slot(_) => "slot",
            TreeItem::Animation(_) => "animation",
            _ => return,
        };
        if item.exists(self.session.document()) {
            self.cancel_gesture();
            self.playing = false;
            self.rename = Some(RenameState {
                dialog: NameDialog::new(
                    format!("Rename {kind}"),
                    item.label(self.session.document()),
                ),
                item,
            });
        }
    }

    pub(super) fn rename_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut state) = self.rename.take() else {
            return;
        };
        if !state.item.exists(self.session.document()) {
            return;
        }
        let output = show_name_dialog(ctx, egui::Id::new("rename-item"), &mut state.dialog);
        self.record_target("renameName", output.name);
        self.record_target("renameConfirm", output.confirm);
        self.record_target("renameCancel", output.cancel);
        match output.action {
            DialogAction::Cancel => return,
            DialogAction::Confirm => {
                let name = state.dialog.name.clone();
                let command = match state.item {
                    TreeItem::Bone(bone) => Command::RenameBone { bone, name },
                    TreeItem::Slot(slot) => Command::RenameSlot { slot, name },
                    TreeItem::Animation(animation) => Command::RenameAnimation { animation, name },
                    _ => return,
                };
                match self.session.dispatch(command) {
                    Ok(()) => {
                        self.reconcile_selection();
                        return;
                    }
                    Err(error) => state.dialog.reject(error),
                }
            }
            DialogAction::None => {}
        }
        self.rename = Some(state);
    }
}
