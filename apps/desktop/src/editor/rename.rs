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
            TreeItem::Constraint(index) if self.physics_constraint(index).is_some() => {
                "physics constraint"
            }
            TreeItem::Constraint(index) if self.transform_constraint(index).is_some() => {
                "transform constraint"
            }
            TreeItem::Constraint(_) => "IK constraint",
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
                    TreeItem::Constraint(constraint) => {
                        match self.session.document().constraints.get(constraint).cloned() {
                            Some(Constraint::Ik(mut value)) => {
                                value.name = name;
                                Command::SetIkConstraint { constraint, value }
                            }
                            Some(Constraint::Physics(mut value)) => {
                                value.name = name;
                                Command::SetPhysicsConstraint { constraint, value }
                            }
                            Some(Constraint::Transform(mut value)) => {
                                value.name = name;
                                Command::SetTransformConstraint { constraint, value }
                            }
                            Some(Constraint::Path(mut value)) => {
                                value.name = name;
                                Command::SetPathConstraint { constraint, value }
                            }
                            None => return,
                        }
                    }
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

#[cfg(test)]
mod tests {
    use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
    use super::*;

    fn fixture() -> Editor {
        let mut editor = editor_fixture();
        let mut document = editor.session.document().clone();
        document.constraints = vec![
            Constraint::Ik(IkConstraint {
                name: "arm IK".into(),
                bones: vec![7, 8],
                target: 0,
                setup: IkSetup {
                    mix: 0.,
                    bend_positive: true,
                },
            }),
            Constraint::Ik(IkConstraint {
                name: "other IK".into(),
                bones: vec![2],
                target: 0,
                setup: IkSetup {
                    mix: 0.,
                    bend_positive: false,
                },
            }),
        ];
        editor.session = Session::new(document).unwrap();
        editor.select(TreeItem::Constraint(0));
        editor
    }
    fn key(key: egui::Key) -> Vec<egui::Event> {
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            })
            .collect()
    }
    fn click(editor: &mut Editor, ctx: &egui::Context, target: &str) {
        let [left, top, right, bottom] = editor.capture_targets[target];
        let pos = pos2((left + right) / 2., (top + bottom) / 2.);
        for pressed in [true, false] {
            workspace_frame(
                editor,
                ctx,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }
    #[test]
    fn constraint_rename_reuses_header_dialog_and_preserves_data_pending_and_typed_history() {
        let mut editor = fixture();
        editor.setup_mode = true;
        let mut local = editor.session.local_pose(Some(0), editor.time)[2];
        local.rotation += 17.;
        editor
            .session
            .edit_pose(
                2,
                EditTarget::Animate {
                    animation: 0,
                    time: editor.time,
                },
                local,
                &[Property::Rotation],
                false,
            )
            .unwrap();
        let pending = editor.session.pose_edits().clone();
        let original = editor.session.document().clone();
        let time = editor.time;
        let ctx = editor_context();
        workspace_frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "ikMixValue");
        workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("50".into())]);
        assert!(editor.ik_edit.is_some());
        assert_ne!(editor.session.document(), &original);
        click(&mut editor, &ctx, "ikRename");
        assert!(editor.ik_edit.is_none());
        assert!(editor.rename.is_some());
        assert_eq!(editor.session.document(), &original);
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::Text("edited IK".into())],
        );
        workspace_frame(&mut editor, &ctx, key(egui::Key::Enter));
        assert!(editor.rename.is_none());
        let mut renamed = original.clone();
        let Constraint::Ik(ik) = &mut renamed.constraints[0] else {
            unreachable!("fixture constraint is IK")
        };
        ik.name = "edited IK".into();
        assert_eq!(editor.session.document(), &renamed);
        assert_eq!(editor.selection, Some(TreeItem::Constraint(0)));
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(editor.time, time);
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(editor.selection, Some(TreeItem::Constraint(0)));
        editor.redo();
        assert_eq!(editor.session.document(), &renamed);
        assert_eq!(editor.session.pose_edits(), &pending);
        workspace_frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "ikTargetChoose");
        assert!(editor.ik_target.is_some());
        workspace_frame(&mut editor, &ctx, key(egui::Key::F2));
        assert!(editor.ik_target.is_none());
        assert!(editor.rename.is_some());
        assert_eq!(editor.rename.as_ref().unwrap().dialog.name, "edited IK");
        workspace_frame(&mut editor, &ctx, key(egui::Key::Escape));
        assert!(editor.rename.is_none());
        assert_eq!(editor.session.document(), &renamed);
    }
    #[test]
    fn constraint_rename_duplicate_and_empty_failures_preserve_redo_and_a_corrected_name_is_one_step()
     {
        let mut editor = fixture();
        let original = editor.session.document().clone();
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "redo sentinel".into(),
            })
            .unwrap();
        editor.undo();
        let ctx = editor_context();
        workspace_frame(&mut editor, &ctx, vec![]);
        workspace_frame(&mut editor, &ctx, key(egui::Key::F2));
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::Text("other IK".into())],
        );
        workspace_frame(&mut editor, &ctx, key(egui::Key::Enter));
        assert!(editor.rename.as_ref().unwrap().dialog.error.is_some());
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
        workspace_frame(&mut editor, &ctx, vec![]);
        workspace_frame(&mut editor, &ctx, key(egui::Key::Backspace));
        workspace_frame(&mut editor, &ctx, key(egui::Key::Enter));
        assert_eq!(editor.rename.as_ref().unwrap().dialog.name, "");
        assert!(editor.rename.as_ref().unwrap().dialog.error.is_some());
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
        workspace_frame(&mut editor, &ctx, vec![]);
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::Text("corrected IK".into())],
        );
        workspace_frame(&mut editor, &ctx, key(egui::Key::Enter));
        assert!(editor.rename.is_none());
        let mut renamed = original.clone();
        let Constraint::Ik(ik) = &mut renamed.constraints[0] else {
            unreachable!("fixture constraint is IK")
        };
        ik.name = "corrected IK".into();
        assert_eq!(editor.session.document(), &renamed);
        assert_eq!(editor.selection, Some(TreeItem::Constraint(0)));
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
        editor.redo();
        assert_eq!(editor.session.document(), &renamed);
        assert_eq!(editor.selection, Some(TreeItem::Constraint(0)));
    }
}
