//! Animation actions coordinate dialogs and selection; clip edits belong to the engine.
use super::rename::{DialogAction, NameDialog, show_name_dialog};
use super::*;

pub(super) struct DuplicateAnimationState {
    animation: usize,
    dialog: NameDialog,
}

impl Editor {
    pub(super) fn animation_properties(&mut self, ui: &mut egui::Ui, animation: usize) {
        let a = &self.session.document().animations[animation];
        ui.label(format!(
            "{:.0} frames · {} tracks",
            animation::frame_at(a.duration(), self.session.document().fps),
            a.tracks.len() + a.ik_tracks.len()
        ));
        ui.label(if self.animation == animation {
            "Active animation"
        } else {
            "Selected for inspection; another animation is active."
        });
        let activate = ui.button("Activate animation");
        self.record_target("animationActivate", activate.rect);
        if activate.clicked() {
            self.activate_animation(animation);
        }
    }

    pub(super) fn begin_duplicate_animation(&mut self, animation: usize) {
        if animation >= self.session.document().animations.len() {
            return;
        }
        self.cancel_gesture();
        self.playing = false;
        self.duplicate_animation = Some(DuplicateAnimationState {
            animation,
            dialog: NameDialog::new("Duplicate animation", String::new()),
        });
    }

    pub(super) fn duplicate_animation_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut state) = self.duplicate_animation.take() else {
            return;
        };
        if state.animation >= self.session.document().animations.len() {
            return;
        }
        let output = show_name_dialog(ctx, egui::Id::new("duplicate-animation"), &mut state.dialog);
        self.record_target("duplicateName", output.name);
        self.record_target("duplicateConfirm", output.confirm);
        self.record_target("duplicateCancel", output.cancel);
        match output.action {
            DialogAction::Cancel => return,
            DialogAction::Confirm => {
                let copy = self.session.document().animations.len();
                let command = Command::DuplicateAnimation {
                    animation: state.animation,
                    name: state.dialog.name.clone(),
                };
                match self.session.dispatch(command) {
                    Ok(()) => {
                        self.reconcile_selection();
                        self.select(TreeItem::Animation(copy));
                        self.status =
                            "Animation duplicated; select its visibility dot to activate".into();
                        return;
                    }
                    Err(error) => state.dialog.reject(error),
                }
            }
            DialogAction::None => {}
        }
        self.duplicate_animation = Some(state);
    }
}
