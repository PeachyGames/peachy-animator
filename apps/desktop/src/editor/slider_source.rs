//! Explicit source picking and native mapping controls; transitions stay shared.
use super::*;
use peachy_engine::{SliderMappingRangeField, slider_mapping_range};

#[cfg(test)]
#[path = "slider_source_tests.rs"]
mod tests;

pub(super) struct SliderSourcePicker {
    pub constraint: usize,
    source: SliderConstraint,
    animation: usize,
    time: f32,
    skin: String,
    generation: u64,
    auto_key: bool,
}

impl Editor {
    fn slider_configuration_available(&self, ctx: &egui::Context) -> bool {
        self.setup_mode
            && self.show_tree
            && ctx.input(|input| input.focused)
            && self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
            && self.export_dialog.is_none()
            && !self.slot_color_dialog_open()
    }

    pub(super) fn slider_source_picker_active(&self) -> bool {
        self.slider_view.picker.is_some()
    }

    pub(super) fn reconcile_slider_source_focus(&mut self) {
        let focus = match self.selection {
            Some(TreeItem::Constraint(index)) if self.slider_constraint(index).is_some() => {
                Some(index)
            }
            _ => None,
        };
        if let Err(error) = self.session.focus_slider_source(focus) {
            self.slider_view.error = Some(error);
        }
    }

    pub(super) fn carry_slider_source_preferences(
        &mut self,
        incoming: &mut Session,
    ) -> Result<(), String> {
        // Called after successful decoding, with owned edits already canceled.
        // Include the accepted focused range without changing the old view.
        incoming
            .set_slider_source_preferences(self.session.slider_source_preferences_for_transfer())
    }

    fn begin_slider_source_picker(&mut self, constraint: usize, ctx: &egui::Context) {
        self.cancel_gesture();
        self.weights.bind = None;
        if !self.slider_configuration_available(ctx)
            || self.selection != Some(TreeItem::Constraint(constraint))
        {
            return;
        }
        let Some(source) = self.slider_constraint(constraint).cloned() else {
            return;
        };
        self.playing = false;
        self.slider_view.picker = Some(SliderSourcePicker {
            constraint,
            source,
            animation: self.animation,
            time: self.time,
            skin: self.skin.clone(),
            generation: self.pose_view_generation,
            auto_key: self.auto_key,
        });
        self.status =
            "Source: choose an existing bone in the Tree or viewport; Escape cancels".into();
    }

    pub(super) fn reconcile_slider_source_picker(&mut self) {
        if self.slider_view.picker.as_ref().is_some_and(|picker| {
            !self.show_tree
                || !self.setup_mode
                || self.selection != Some(TreeItem::Constraint(picker.constraint))
                || self.animation != picker.animation
                || self.time.to_bits() != picker.time.to_bits()
                || self.skin != picker.skin
                || self.pose_view_generation != picker.generation
                || self.auto_key != picker.auto_key
                || self.slider_constraint(picker.constraint) != Some(&picker.source)
        }) {
            self.slider_view.picker = None;
        }
    }

    pub(super) fn slider_source_selection(&mut self, item: &TreeItem) -> bool {
        let Some(picker) = self.slider_view.picker.take() else {
            return false;
        };
        let TreeItem::Bone(bone) = item else {
            self.slider_view.picker = Some(picker);
            self.status = "Choose an existing source bone; Escape cancels".into();
            return true;
        };
        if !self.setup_mode
            || self.selection != Some(TreeItem::Constraint(picker.constraint))
            || self.animation != picker.animation
            || self.time.to_bits() != picker.time.to_bits()
            || self.skin != picker.skin
            || self.pose_view_generation != picker.generation
            || self.auto_key != picker.auto_key
            || self.slider_constraint(picker.constraint) != Some(&picker.source)
        {
            self.status = "Source choice canceled because its context changed".into();
            return true;
        }
        if self.dispatch(Command::SetSliderSource {
            constraint: picker.constraint,
            bone: Some(*bone),
        }) {
            self.status = "Slider source updated".into();
        }
        true
    }

    pub(super) fn slider_source_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        if !self.slider_configuration_available(ctx) || egui::Popup::is_any_open(ctx) {
            self.slider_view.picker = None;
            return false;
        }
        if self.slider_view.picker.is_some()
            && !ctx.text_edit_focused()
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.slider_view.picker = None;
            self.status = "Source choice canceled".into();
            return true;
        }
        false
    }

    pub(super) fn slider_option(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        label: &str,
        value: bool,
    ) {
        let mut checked = value;
        let enabled = self.slider_configuration_available(ui.ctx())
            && !self.slider_source_picker_active()
            && !egui::Popup::is_any_open(ui.ctx());
        let response = ui
            .add_enabled(enabled, egui::Checkbox::new(&mut checked, label))
            .on_disabled_hover_text("Slider configuration is edited in Setup.");
        self.slider_target(ui, &format!("slider{label}"), response.rect);
        if response.changed() {
            self.cancel_gesture();
            if let Some(mut candidate) = self.slider_constraint(index).cloned() {
                match label {
                    "Loop" => candidate.looped = checked,
                    "Additive" => candidate.additive = checked,
                    "Local" => candidate.local = checked,
                    _ => return,
                }
                self.dispatch(Command::SetSliderConstraint {
                    constraint: index,
                    value: candidate,
                });
            }
        }
    }

    pub(super) fn slider_source_bone_row(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        value: &SliderConstraint,
    ) {
        ui.label("Bone");
        ui.horizontal(|ui| {
            if let Some(bone) = value.bone {
                let name = self.session.document().bones[bone].name.clone();
                let link = ui.link(name);
                self.slider_target(ui, "sliderBone", link.rect);
                if link.clicked() {
                    self.select(TreeItem::Bone(bone));
                }
            } else {
                let label = ui.label("<none>");
                self.slider_target(ui, "sliderBone", label.rect);
            }
            let enabled = self.slider_configuration_available(ui.ctx())
                && !egui::Popup::is_any_open(ui.ctx());
            let choose = icons::toolbar_button(
                ui,
                icons::Icon::ChooseBone,
                "Choose source bone",
                "",
                enabled,
            );
            self.slider_target(ui, "sliderBoneChoose", choose.rect);
            if choose.clicked() {
                self.begin_slider_source_picker(index, ui.ctx());
            }
            if value.bone.is_some() {
                let clear = icons::toolbar_button(
                    ui,
                    icons::Icon::Delete,
                    "Clear source bone",
                    "",
                    enabled,
                );
                self.slider_target(ui, "sliderBoneClear", clear.rect);
                if clear.clicked() {
                    self.cancel_gesture();
                    self.dispatch(Command::SetSliderSource {
                        constraint: index,
                        bone: None,
                    });
                }
                match self
                    .session
                    .slider_evaluation(index, self.weights_edit_target())
                {
                    Ok(evaluation) => {
                        if let Some(time) = evaluation.sample_time {
                            let frame = time * f64::from(self.session.document().fps);
                            let label = ui.label(format!(
                                "Frame: {}",
                                format!("{frame:.2}")
                                    .trim_end_matches('0')
                                    .trim_end_matches('.')
                            ));
                            self.slider_target(ui, "sliderDerivedFrame", label.rect);
                        }
                    }
                    Err(error) => {
                        ui.label(RichText::new("Frame unavailable").small().color(ACCENT))
                            .on_hover_text(error);
                    }
                }
            }
        });
        ui.end_row();
    }

    pub(super) fn slider_source_range_row(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        property: TransformProperty,
    ) {
        ui.label("Property");
        ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
        let scroll = egui::ScrollArea::horizontal()
            .id_salt(("slider-source-range-scroll", index))
            .min_scrolled_width(0.)
            .auto_shrink([false, true])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                ui.set_width(392.);
                ui.horizontal(|ui| {
                    let enabled = self.slider_configuration_available(ui.ctx())
                        && !self.slider_source_picker_active();
                    let picker = ui
                        .add_enabled_ui(enabled, |ui| {
                            egui::ComboBox::from_id_salt((
                                "slider-source-property",
                                index,
                                self.pose_view_generation,
                            ))
                            .width(92.)
                            .selected_text(property.label())
                            .show_ui(ui, |ui| {
                                for candidate in TransformProperty::ALL {
                                    let choice = ui
                                        .selectable_label(candidate == property, candidate.label());
                                    self.slider_target(
                                        ui,
                                        &format!("sliderPropertyChoice:{}", candidate.spine_name()),
                                        choice.rect,
                                    );
                                    if choice.clicked() && candidate != property {
                                        self.cancel_gesture();
                                        match self
                                            .session
                                            .slider_source_property_command(index, candidate)
                                        {
                                            Ok(command) => {
                                                self.dispatch(command);
                                            }
                                            Err(error) => self.slider_view.error = Some(error),
                                        }
                                    }
                                }
                            })
                        })
                        .inner;
                    self.slider_target(ui, "sliderSourceProperty", picker.response.rect);
                    match slider_mapping_range(self.session.document(), index) {
                        Ok(range) => {
                            for field in [
                                SliderMappingRangeField::SourceMin,
                                SliderMappingRangeField::SourceMax,
                            ] {
                                self.slider_range_number(ui, index, field, range.value(field));
                            }
                            let (arrow, _) = ui.allocate_exact_size(vec2(16., 22.), Sense::hover());
                            let a = arrow.center() - vec2(6., 0.);
                            let b = arrow.center() + vec2(6., 0.);
                            ui.painter().line_segment([a, b], Stroke::new(1.5, MINT));
                            for offset in [-1., 1.] {
                                ui.painter().line_segment(
                                    [b - vec2(4., 3. * offset), b],
                                    Stroke::new(1.5, MINT),
                                );
                            }
                            for field in [
                                SliderMappingRangeField::TargetMin,
                                SliderMappingRangeField::TargetMax,
                            ] {
                                self.slider_range_number(ui, index, field, range.value(field));
                            }
                        }
                        Err(error) => {
                            ui.colored_label(ACCENT, error);
                        }
                    }
                });
            });
        self.slider_target(ui, "sliderRangeScroll", scroll.inner_rect);
        ui.end_row();
    }
}
