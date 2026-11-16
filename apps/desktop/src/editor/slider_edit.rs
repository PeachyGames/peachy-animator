//! Manual Slider inputs freeze their context; Session owns pending poses and history.
use super::*;
use peachy_engine::{
    SliderMappingRangeField, SliderPose, SliderPoseEdit, edited_slider_mapping_range,
    slider_mapping_range,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum SliderField {
    Pose(SliderProperty),
    Range(SliderMappingRangeField),
}

impl SliderField {
    fn factor(self, fps: u32) -> f64 {
        match self {
            Self::Pose(SliderProperty::Time)
            | Self::Range(
                SliderMappingRangeField::TargetMin | SliderMappingRangeField::TargetMax,
            ) => f64::from(fps),
            Self::Pose(SliderProperty::Mix) => 100.,
            Self::Range(_) => 1.,
        }
    }
}

struct SliderEdit {
    constraint: usize,
    field: SliderField,
    accepted: SliderConstraint,
    pose: SliderPose,
    setup: bool,
    animation: usize,
    time: f32,
    auto_key: bool,
    skin: String,
    generation: u64,
    control: egui::Id,
    rect: egui::Rect,
    numeric: bool,
    touched: bool,
    valid_text: bool,
    shown: f64,
    undefined: bool,
    duration: f32,
    finish: bool,
}

struct SliderNumber {
    field: SliderField,
    value: Option<f64>,
    target: String,
    width: f32,
    enabled: bool,
}

#[derive(Default)]
pub(super) struct SliderView {
    edit: Option<SliderEdit>,
    blocked: Option<egui::Id>,
    pub error: Option<String>,
    pub(super) picker: Option<super::slider_source::SliderSourcePicker>,
}

impl Editor {
    pub(super) fn slider_edit_active(&self) -> bool {
        self.slider_view.edit.is_some() || self.slider_view.picker.is_some()
    }

    fn slider_input_available(&self, ctx: &egui::Context) -> bool {
        self.show_tree && self.weights_input_available(ctx) && self.slider_view.picker.is_none()
    }

    pub(super) fn slider_property_available(&self, index: usize, property: SliderProperty) -> bool {
        self.slider_constraint(index).is_some_and(|value| {
            property == SliderProperty::Mix
                || (value.bone.is_none()
                    && self.session.document().animations[value.animation].duration() > 0.)
        })
    }

    pub(super) fn cancel_slider_edit(&mut self) {
        self.slider_view.picker = None;
        if let Some(edit) = self.slider_view.edit.take() {
            self.session.cancel_transaction();
            self.slider_view.blocked = Some(edit.control);
        }
        self.slider_view.error = None;
    }

    fn finish_slider_edit(&mut self) {
        if self
            .slider_view
            .edit
            .as_ref()
            .is_some_and(|edit| !edit.valid_text)
            || self.slider_view.error.is_some()
        {
            self.cancel_slider_edit();
        } else if self.slider_view.edit.take().is_some() {
            self.session.commit_transaction();
        }
    }

    pub(super) fn reconcile_slider_context(&mut self) {
        self.reconcile_slider_source_picker();
        if self.slider_view.edit.as_ref().is_some_and(|edit| {
            !self.show_tree
                || self.selection.single() != Some(&TreeItem::Constraint(edit.constraint))
                || self.setup_mode != edit.setup
                || self.animation != edit.animation
                || self.time.to_bits() != edit.time.to_bits()
                || self.auto_key != edit.auto_key
                || self.skin != edit.skin
                || self.pose_view_generation != edit.generation
                || self.slider_constraint(edit.constraint) != Some(&edit.accepted)
                || !self.slider_field_available(edit.constraint, edit.field)
                || self.session.document().animations[edit.accepted.animation]
                    .duration()
                    .to_bits()
                    != edit.duration.to_bits()
                || self
                    .session
                    .slider_pose(edit.constraint, self.weights_edit_target())
                    != Ok(edit.pose)
        }) {
            self.cancel_slider_edit();
        }
    }

    pub(super) fn reconcile_slider_input(&mut self, ctx: &egui::Context) {
        if ctx.input(|input| input.pointer.button_pressed(egui::PointerButton::Primary)) {
            self.slider_view.blocked = None;
        }
        self.reconcile_slider_context();
        if !self.show_tree || !self.weights_input_available(ctx) {
            self.cancel_slider_edit();
        }
        if let Some(control) = self.slider_view.blocked {
            ctx.memory_mut(|memory| memory.surrender_focus(control));
        }
    }

    pub(super) fn settle_slider_input(&mut self, ctx: &egui::Context) {
        // Do not clear a cancellation made by this frame's primary press.
        self.reconcile_slider_context();
        if !self.show_tree || !self.weights_input_available(ctx) {
            self.cancel_slider_edit();
        }
        if !ctx.input(|input| input.pointer.primary_down())
            && self
                .slider_view
                .edit
                .as_ref()
                .is_some_and(|edit| edit.finish)
        {
            self.finish_slider_edit();
        }
        if ctx.input(|input| {
            !input.pointer.primary_down()
                && !input.pointer.button_released(egui::PointerButton::Primary)
        }) {
            self.slider_view.blocked = None;
        }
    }

    fn begin_slider_number(
        &mut self,
        index: usize,
        field: SliderField,
        response: &egui::Response,
        numeric: bool,
    ) -> Result<(), String> {
        self.finish_slider_edit();
        self.cancel_gesture();
        if !self.slider_input_available(&response.ctx)
            || self.selection.single() != Some(&TreeItem::Constraint(index))
            || !self.slider_field_available(index, field)
        {
            return Err("Select an editable Slider property".into());
        }
        let accepted = self
            .slider_constraint(index)
            .cloned()
            .ok_or("Unknown Slider")?;
        let pose = self
            .session
            .slider_pose(index, self.weights_edit_target())?;
        let initial = match field {
            SliderField::Pose(property) => Some(pose.value(property)),
            SliderField::Range(field) => {
                slider_mapping_range(self.session.document(), index)?.value(field)
            }
        };
        let duration = self.session.document().animations[accepted.animation].duration();
        self.session.commit_transaction();
        self.session.begin_transaction();
        self.playing = false;
        self.slider_view.edit = Some(SliderEdit {
            constraint: index,
            field,
            accepted,
            pose,
            setup: self.setup_mode,
            animation: self.animation,
            time: self.time,
            auto_key: self.auto_key,
            skin: self.skin.clone(),
            generation: self.pose_view_generation,
            control: response.id,
            rect: response.rect,
            numeric,
            touched: false,
            valid_text: true,
            shown: initial.unwrap_or(0.) * field.factor(self.session.document().fps),
            undefined: initial.is_none(),
            duration,
            finish: false,
        });
        Ok(())
    }

    fn slider_field_available(&self, index: usize, field: SliderField) -> bool {
        match field {
            SliderField::Pose(property) => self.slider_property_available(index, property),
            SliderField::Range(field) => {
                self.setup_mode
                    && slider_mapping_range(self.session.document(), index).is_ok_and(|range| {
                        range.source_max.is_some()
                            || (field == SliderMappingRangeField::SourceMax
                                && range.target_min != range.target_max)
                    })
            }
        }
    }

    fn preview_slider_number(&mut self, shown: f64) -> Result<(), String> {
        let edit = self
            .slider_view
            .edit
            .as_ref()
            .ok_or("No active Slider edit")?;
        let raw = shown / edit.field.factor(self.session.document().fps);
        let value = raw as f32;
        if !raw.is_finite()
            || !value.is_finite()
            || (edit.field == SliderField::Pose(SliderProperty::Time) && raw < 0.)
        {
            return Err("Enter a finite Frame at least zero, or a finite Mix percentage".into());
        }
        let target = if edit.setup {
            EditTarget::Setup
        } else {
            EditTarget::Animate {
                animation: edit.animation,
                time: edit.time,
            }
        };
        let index = edit.constraint;
        match edit.field {
            SliderField::Pose(property) => {
                let change = match property {
                    SliderProperty::Time => SliderPoseEdit::Time { value },
                    SliderProperty::Mix => SliderPoseEdit::Mix { value },
                };
                self.session
                    .preview_edit_slider_pose(index, target, &[change], edit.auto_key)?;
            }
            SliderField::Range(field) => {
                let candidate =
                    edited_slider_mapping_range(&edit.accepted, edit.duration, field, raw)?;
                self.session
                    .preview_transaction(Command::SetSliderConstraint {
                        constraint: index,
                        value: candidate,
                    })?;
            }
        }
        let accepted = self
            .slider_constraint(index)
            .cloned()
            .ok_or("Unknown Slider")?;
        let pose = self.session.slider_pose(index, target)?;
        let edit = self.slider_view.edit.as_mut().unwrap();
        edit.accepted = accepted;
        edit.pose = pose;
        edit.shown = shown;
        edit.undefined = false;
        Ok(())
    }

    pub(super) fn slider_edit_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        if self.slider_source_shortcuts(ctx) {
            return true;
        }
        let Some(edit) = self.slider_view.edit.as_ref() else {
            return false;
        };
        if !self.slider_input_available(ctx) {
            self.cancel_slider_edit();
            return false;
        }
        let control = edit.control;
        let owns = ctx.memory(|memory| memory.has_focus(control));
        if owns && edit.numeric && transform_gesture::numeric_intent(ctx) {
            self.slider_view.edit.as_mut().unwrap().touched = true;
        }
        if owns && ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::S))
        {
            self.cancel_slider_edit();
            ctx.memory_mut(|memory| memory.surrender_focus(control));
            self.save_project();
            return true;
        }
        if owns && ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::O))
        {
            self.cancel_slider_edit();
            ctx.memory_mut(|memory| memory.surrender_focus(control));
            self.open_project(ctx);
            return true;
        }
        if owns && ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::E))
        {
            self.cancel_slider_edit();
            ctx.memory_mut(|memory| memory.surrender_focus(control));
            self.export();
            return true;
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F9)) {
            self.toggle_views();
            return true;
        }
        if (!ctx.text_edit_focused() || owns)
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.cancel_slider_edit();
            ctx.memory_mut(|memory| memory.surrender_focus(control));
            return true;
        }
        false
    }

    fn slider_number_response(
        &mut self,
        index: usize,
        field: SliderField,
        response: &egui::Response,
        shown: f64,
        numeric: bool,
        parsed: Option<bool>,
    ) {
        if self.slider_view.blocked.is_some() || !self.slider_input_available(&response.ctx) {
            return;
        }
        let starts = (numeric && response.gained_focus())
            || response.drag_started()
            || (response.changed()
                && (!numeric || transform_gesture::numeric_intent(&response.ctx)));
        if starts
            && self
                .slider_view
                .edit
                .as_ref()
                .is_none_or(|edit| edit.control != response.id)
            && let Err(error) = self.begin_slider_number(
                index,
                field,
                response,
                numeric && response.has_focus() && !response.dragged(),
            )
        {
            self.slider_view.error = Some(error);
            return;
        }
        let Some(edit) = self
            .slider_view
            .edit
            .as_mut()
            .filter(|edit| edit.control == response.id)
        else {
            return;
        };
        if !edit.numeric && edit.rect != response.rect {
            self.cancel_slider_edit();
            return;
        }
        if response.has_focus() || response.is_pointer_button_down_on() {
            edit.finish = false;
        }
        if edit.numeric && transform_gesture::numeric_intent(&response.ctx) {
            edit.touched = true;
        }
        if let Some(valid) = parsed {
            edit.valid_text = valid;
        }
        let entered_undefined =
            edit.undefined && edit.numeric && edit.touched && parsed == Some(true);
        if (response.changed() || entered_undefined)
            && edit.valid_text
            && (!edit.numeric || edit.touched)
            && (entered_undefined || edit.shown.to_bits() != shown.to_bits())
            && (!edit.undefined || entered_undefined)
        {
            self.slider_view.error = self.preview_slider_number(shown).err();
        }
        if response.lost_focus()
            || (!(numeric && response.has_focus())
                && !response.ctx.input(|input| input.pointer.primary_down()))
        {
            self.slider_view.edit.as_mut().unwrap().finish = true;
        }
    }

    pub(super) fn slider_number(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        property: SliderProperty,
        value: f64,
        enabled: bool,
    ) {
        self.slider_field_number(
            ui,
            index,
            SliderNumber {
                field: SliderField::Pose(property),
                value: Some(value),
                target: format!("slider{property:?}Value"),
                width: 64.,
                enabled,
            },
        );
    }

    pub(super) fn slider_range_number(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        field: SliderMappingRangeField,
        value: Option<f64>,
    ) {
        self.slider_field_number(
            ui,
            index,
            SliderNumber {
                field: SliderField::Range(field),
                value,
                target: format!("sliderRange{field:?}"),
                width: 58.,
                enabled: self.slider_field_available(index, SliderField::Range(field)),
            },
        );
    }

    fn slider_field_number(&mut self, ui: &mut egui::Ui, index: usize, number: SliderNumber) {
        let SliderNumber {
            field,
            value,
            target,
            width,
            enabled,
        } = number;
        let factor = field.factor(self.session.document().fps);
        let edit = self
            .slider_view
            .edit
            .as_ref()
            .filter(|edit| edit.constraint == index && edit.field == field);
        let undefined = edit.map_or(value.is_none(), |edit| edit.undefined);
        let mut shown = self
            .slider_view
            .edit
            .as_ref()
            .filter(|edit| edit.constraint == index && edit.field == field)
            .map_or(value.unwrap_or(0.) * factor, |edit| edit.shown);
        let id = egui::Id::new((
            "slider-number",
            index,
            format!("{field:?}"),
            self.setup_mode,
            self.animation,
            self.time.to_bits(),
            &self.skin,
            self.pose_view_generation,
        ));
        let (rect, _) = ui.allocate_exact_size(vec2(width, 22.), Sense::hover());
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id(id)
                .max_rect(rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.set_clip_rect(rect.intersect(ui.clip_rect()));
        child.spacing_mut().interact_size = vec2(width, 22.);
        let focused = child.memory(|memory| memory.has_focus(child.next_auto_id()));
        let parsed = std::cell::Cell::new(None);
        let response = child.add_enabled(
            enabled && self.slider_input_available(ui.ctx()),
            egui::DragValue::new(&mut shown)
                .speed(0.2)
                .range(
                    if field == SliderField::Pose(SliderProperty::Time) {
                        0.
                    } else {
                        f64::MIN
                    }..=f64::MAX,
                )
                .clamp_existing_to_range(false)
                .update_while_editing(false)
                .custom_formatter(move |value, _| {
                    if undefined {
                        String::new()
                    } else if focused {
                        value.to_string()
                    } else {
                        format!("{value:.2}")
                            .trim_end_matches('0')
                            .trim_end_matches('.')
                            .to_string()
                    }
                })
                .custom_parser(|text| {
                    let value = text
                        .trim()
                        .trim_end_matches('%')
                        .trim()
                        .parse::<f64>()
                        .ok()
                        .filter(|value| {
                            value.is_finite()
                                && ((*value / factor) as f32).is_finite()
                                && (field != SliderField::Pose(SliderProperty::Time)
                                    || *value >= 0.)
                        });
                    parsed.set(Some(value.is_some()));
                    value
                })
                .suffix(if field == SliderField::Pose(SliderProperty::Mix) {
                    "%"
                } else {
                    ""
                }),
        );
        self.slider_target(ui, &target, response.rect.intersect(rect));
        self.slider_number_response(index, field, &response, shown, true, parsed.get());
    }

    pub(super) fn slider_rail(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        property: SliderProperty,
        value: f64,
        end: f64,
        enabled: bool,
    ) {
        let field = SliderField::Pose(property);
        let mut shown = self
            .slider_view
            .edit
            .as_ref()
            .filter(|edit| edit.constraint == index && edit.field == field)
            .map_or(value * field.factor(self.session.document().fps), |edit| {
                edit.shown
            });
        let id = egui::Id::new(("slider-rail", index, property, self.pose_view_generation));
        let response = ui
            .scope_builder(egui::UiBuilder::new().id(id), |ui| {
                ui.spacing_mut().slider_width = (ui.available_width() - 30.).clamp(32., 120.);
                ui.add_enabled(
                    enabled && self.slider_input_available(ui.ctx()),
                    egui::Slider::new(&mut shown, 0.0..=end)
                        .show_value(false)
                        .clamping(egui::SliderClamping::Edits),
                )
            })
            .inner;
        self.slider_target(ui, &format!("slider{property:?}Slider"), response.rect);
        self.slider_number_response(index, field, &response, shown, false, None);
    }

    pub(super) fn slider_key_ready(&mut self) -> Result<(), String> {
        let Some(edit) = self.slider_view.edit.as_mut() else {
            return Ok(());
        };
        edit.finish = false;
        if !edit.valid_text {
            return Err("Enter a finite Slider value before keying".into());
        }
        self.slider_view.error.clone().map_or(Ok(()), Err)
    }

    pub(super) fn seal_slider_for_key(&mut self) {
        if let Some(edit) = self.slider_view.edit.take() {
            self.slider_view.blocked = Some(edit.control);
        }
    }

    pub(super) fn hold_slider_key_error(&mut self, error: &str) {
        if let Some(edit) = self.slider_view.edit.as_mut() {
            edit.finish = false;
            self.slider_view.error = Some(error.into());
        }
    }

    pub(super) fn slider_edit_diagnostic(&self) -> serde_json::Value {
        let selected = match self.selection.single().cloned() {
            Some(TreeItem::Constraint(index)) if self.slider_constraint(index).is_some() => {
                Some(index)
            }
            _ => None,
        };
        serde_json::json!({"selected":selected,"value":selected.and_then(|index|self.slider_constraint(index)),
            "authored":selected.and_then(|index|self.session.slider_pose(index,self.weights_edit_target()).ok()),
            "active":self.slider_view.edit.as_ref().map(|edit|serde_json::json!({
                "constraint":edit.constraint,"property":match edit.field {SliderField::Pose(property)=>Some(property),_=>None},"field":format!("{:?}",edit.field),"setup":edit.setup,
                "animation":edit.animation,"time":edit.time,"autoKey":edit.auto_key,
                "numeric":edit.numeric,"touched":edit.touched,"validText":edit.valid_text,"shown":edit.shown})),
            "sourcePicker":self.slider_view.picker.as_ref().map(|picker|picker.constraint),
            "sourceFocus":self.session.slider_source_focus(),"sourcePreferences":self.session.slider_source_preferences(),
            "blocked":self.slider_view.blocked.is_some(),"error":self.slider_view.error})
    }

    pub(super) fn slider_text_invalid(&self) -> bool {
        self.slider_view
            .edit
            .as_ref()
            .is_some_and(|edit| !edit.valid_text)
    }
}
