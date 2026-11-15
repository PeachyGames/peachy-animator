//! Observed manual Slider properties; pose and key conversion stay in the engine.
use super::*;

#[cfg(test)]
#[path = "slider_ui_tests.rs"]
mod tests;

const EMPTY_FRAME: &str = "Frame editing requires a referenced animation with a positive duration.";

impl Editor {
    pub(super) fn slider_constraint(&self, index: usize) -> Option<&SliderConstraint> {
        self.session.document().constraints.get(index)?.as_slider()
    }

    pub(super) fn slider_target(&mut self, ui: &egui::Ui, name: &str, rect: egui::Rect) {
        let visible = rect.intersect(ui.clip_rect());
        if visible.is_positive() && visible.is_finite() {
            self.record_target(name, visible);
        } else {
            self.capture_targets.remove(name);
        }
    }

    pub(super) fn slider_properties(&mut self, ui: &mut egui::Ui, index: usize) {
        self.reconcile_slider_context();
        let Some(value) = self.slider_constraint(index).cloned() else {
            return;
        };
        ui.horizontal(|ui| {
            icons::cue(ui, icons::Icon::SliderConstraint);
            ui.strong("Slider");
        });
        let fps = f64::from(self.session.document().fps);
        let animation = &self.session.document().animations[value.animation];
        let name = animation.name.clone();
        let duration = animation.duration();
        let end_frame = (f64::from(duration) * fps).max(1.);
        let pose = match self.session.slider_pose(index, self.weights_edit_target()) {
            Ok(pose) => pose,
            Err(error) => {
                ui.colored_label(ACCENT, error);
                return;
            }
        };
        egui::Grid::new(("slider-properties",index)).num_columns(2).show(ui, |ui| {
            ui.label("Animation");
            let response = ui.add_enabled(false,egui::Button::new(name)).on_disabled_hover_text("Changing the referenced animation is not available yet.");
            self.slider_target(ui,"sliderAnimation",response.rect);
            ui.end_row();
            ui.label("Options");
            ui.horizontal(|ui| {
                self.slider_option(ui,index,"Loop",value.looped);
                self.slider_option(ui,index,"Additive",value.additive);
                if value.bone.is_some() {
                    self.slider_option(ui,index,"Local",value.local);
                }
            });
            ui.end_row();
            self.slider_source_bone_row(ui,index,&value);
            if let Some(property)=value.property.filter(|_|value.bone.is_some()) {
                self.slider_source_range_row(ui,index,property);
            }
            for property in SliderProperty::ALL {
                if property==SliderProperty::Time && value.bone.is_some() { continue; }
                ui.label(if property==SliderProperty::Time {"Frame"}else{"Mix"});
                ui.horizontal(|ui| {
                    let available=self.slider_property_available(index,property);
                    self.slider_number(ui,index,property,pose.value(property),available);
                    let end=if property==SliderProperty::Time {end_frame}else{100.};
                    self.slider_rail(ui,index,property,pose.value(property),end,available);
                    if !self.setup_mode {
                        match self.session.pose_edits().slider_key_state(self.session.document(),self.animation,self.time,index,property) {
                            Ok(state)=>{
                                let response=ui.push_id(("slider-key",index,property),|ui|ui.add_enabled_ui(available && self.weights_input_available(ui.ctx()),|ui|super::animation::pose_key_button(ui,state)).inner).inner
                                    .on_hover_text("Key this Slider property. Right-click removes its key.");
                                self.slider_target(ui,&format!("slider{property:?}Key"),response.rect);
                                if response.clicked() {
                                    self.request_pose_key(pose_key::PoseKeyAction::Slider{constraint:index,properties:vec![property]});
                                } else if response.secondary_clicked() && state==KeyState::Keyed {
                                    self.delete_slider_key(index,property);
                                }
                            },
                            Err(error)=>{ui.colored_label(ACCENT,"Key timing error").on_hover_text(error);}
                        }
                    }
                });
                ui.end_row();
            }
        });
        if duration <= 0. {
            let status = ui.label(RichText::new(EMPTY_FRAME).small().color(MUTED));
            self.slider_target(ui, "sliderCapability", status.rect);
        }
        if let Some(error) = self.slider_view.error.as_ref() {
            ui.label(RichText::new(error).small().color(ACCENT));
        } else if self.slider_text_invalid() {
            ui.label(
                RichText::new("Enter a finite Slider value. Manual Frame must be at least zero.")
                    .small()
                    .color(ACCENT),
            );
        }
    }

    fn delete_slider_key(&mut self, index: usize, property: SliderProperty) {
        match peachy_engine::slider_key_time(
            self.session.document(),
            self.animation,
            self.time,
            index,
            property,
        ) {
            Ok(Some(time)) => {
                self.cancel_gesture();
                self.dispatch(Command::DeleteAnimationKeys {
                    animation: self.animation,
                    keys: vec![peachy_engine::AnimationKey::Slider {
                        constraint: index,
                        property,
                        time,
                    }],
                });
            }
            Ok(None) => {}
            Err(error) => self.status = error,
        }
    }
}
