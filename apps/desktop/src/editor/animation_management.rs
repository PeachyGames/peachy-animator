//! Animation actions coordinate dialogs and selection; clip edits belong to the engine.
use super::rename::{DialogAction, NameDialog, show_name_dialog};
use super::*;

#[cfg(test)]
#[path = "slider_creation_tests.rs"]
mod slider_creation_tests;

pub(super) struct DuplicateAnimationState {
    animation: usize,
    dialog: NameDialog,
}

impl Editor {
    fn slider_creation_animation(&self) -> Option<usize> {
        match self.selection {
            Some(TreeItem::Animation(animation))
                if self.setup_mode && animation < self.session.document().animations.len() =>
            {
                Some(animation)
            }
            _ => None,
        }
    }

    fn slider_creation_input_available(&self, ctx: &egui::Context) -> bool {
        // The New menu itself is a popup. Other modal contexts still block it.
        self.show_tree
            && ctx.input(|input| input.focused)
            && self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
            && self.export_dialog.is_none()
            && !self.slot_color_dialog_open()
    }

    pub(super) fn tree_creation_footer(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let menu = icons::menu_button(ui, icons::Icon::New, "New…", |ui| {
                let animation = self.slider_creation_animation();
                let enabled = animation.is_some() && self.slider_creation_input_available(ui.ctx());
                let slider = ui
                    .add_enabled_ui(enabled, |ui| {
                        icons::button(ui, icons::Icon::SliderConstraint, "Slider", None)
                    })
                    .inner
                    .on_disabled_hover_text(
                        "Select an animation in the Setup Tree to create a Slider.",
                    );
                self.slider_target(ui, "newSlider", slider.rect);
                if slider.clicked()
                    && let Some(animation) = animation
                {
                    ui.close();
                    self.create_slider_from_animation(animation, ui.ctx());
                }
            });
            self.slider_target(ui, "treeNew", menu.rect);
        });
    }

    fn create_slider_from_animation(&mut self, animation: usize, ctx: &egui::Context) {
        if self.slider_creation_animation() != Some(animation)
            || !self.slider_creation_input_available(ctx)
        {
            return;
        }
        self.cancel_gesture();
        self.playing = false;
        if self.dispatch(Command::CreateSliderConstraint { animation }) {
            self.select(TreeItem::Constraint(0));
            self.status = "Slider created from the selected animation".into();
        }
    }

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
