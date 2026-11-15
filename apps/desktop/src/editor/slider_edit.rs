//! Manual Slider inputs freeze their context; Session owns pending poses and history.
use super::*;
use peachy_engine::{SliderPose, SliderPoseEdit};

struct SliderEdit {
    constraint: usize,
    property: SliderProperty,
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
    finish: bool,
}

#[derive(Default)]
pub(super) struct SliderView {
    edit: Option<SliderEdit>,
    blocked: Option<egui::Id>,
    pub error: Option<String>,
}

impl Editor {
    pub(super) fn slider_edit_active(&self) -> bool {
        self.slider_view.edit.is_some()
    }

    fn slider_input_available(&self, ctx: &egui::Context) -> bool {
        self.show_tree && self.weights_input_available(ctx)
    }

    pub(super) fn slider_property_available(&self, index: usize, property: SliderProperty) -> bool {
        self.slider_constraint(index).is_some_and(|value| {
            property == SliderProperty::Mix
                || (value.bone.is_none()
                    && self.session.document().animations[value.animation].duration() > 0.)
        })
    }

    pub(super) fn cancel_slider_edit(&mut self) {
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
        if self.slider_view.edit.as_ref().is_some_and(|edit| {
            !self.show_tree
                || self.selection != Some(TreeItem::Constraint(edit.constraint))
                || self.setup_mode != edit.setup
                || self.animation != edit.animation
                || self.time.to_bits() != edit.time.to_bits()
                || self.auto_key != edit.auto_key
                || self.skin != edit.skin
                || self.pose_view_generation != edit.generation
                || self.slider_constraint(edit.constraint) != Some(&edit.accepted)
                || !self.slider_property_available(edit.constraint, edit.property)
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
        if !self.slider_input_available(ctx) {
            self.cancel_slider_edit();
        }
        if let Some(control) = self.slider_view.blocked {
            ctx.memory_mut(|memory| memory.surrender_focus(control));
        }
    }

    pub(super) fn settle_slider_input(&mut self, ctx: &egui::Context) {
        // Do not clear a cancellation made by this frame's primary press.
        self.reconcile_slider_context();
        if !self.slider_input_available(ctx) {
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
        property: SliderProperty,
        response: &egui::Response,
        numeric: bool,
    ) -> Result<(), String> {
        self.finish_slider_edit();
        self.cancel_gesture();
        if !self.slider_input_available(&response.ctx)
            || self.selection != Some(TreeItem::Constraint(index))
            || !self.slider_property_available(index, property)
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
        self.session.commit_transaction();
        self.session.begin_transaction();
        self.playing = false;
        self.slider_view.edit = Some(SliderEdit {
            constraint: index,
            property,
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
            shown: pose.value(property) * self.slider_factor(property),
            finish: false,
        });
        Ok(())
    }

    fn slider_factor(&self, property: SliderProperty) -> f64 {
        if property == SliderProperty::Time {
            f64::from(self.session.document().fps)
        } else {
            100.
        }
    }

    fn preview_slider_number(&mut self, shown: f64) -> Result<(), String> {
        let edit = self
            .slider_view
            .edit
            .as_ref()
            .ok_or("No active Slider edit")?;
        let raw = shown / self.slider_factor(edit.property);
        let value = raw as f32;
        if !raw.is_finite()
            || !value.is_finite()
            || (edit.property == SliderProperty::Time && raw < 0.)
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
        let change = match edit.property {
            SliderProperty::Time => SliderPoseEdit::Time { value },
            SliderProperty::Mix => SliderPoseEdit::Mix { value },
        };
        let index = edit.constraint;
        self.session
            .preview_edit_slider_pose(index, target, &[change], edit.auto_key)?;
        let accepted = self
            .slider_constraint(index)
            .cloned()
            .ok_or("Unknown Slider")?;
        let pose = self.session.slider_pose(index, target)?;
        let edit = self.slider_view.edit.as_mut().unwrap();
        edit.accepted = accepted;
        edit.pose = pose;
        edit.shown = shown;
        Ok(())
    }

    pub(super) fn slider_edit_shortcuts(&mut self, ctx: &egui::Context) -> bool {
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
        property: SliderProperty,
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
                property,
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
        if response.changed()
            && edit.valid_text
            && (!edit.numeric || edit.touched)
            && edit.shown.to_bits() != shown.to_bits()
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
        let mut shown = self
            .slider_view
            .edit
            .as_ref()
            .filter(|edit| edit.constraint == index && edit.property == property)
            .map_or(value * self.slider_factor(property), |edit| edit.shown);
        let id = egui::Id::new((
            "slider-number",
            index,
            property,
            self.setup_mode,
            self.animation,
            self.time.to_bits(),
            &self.skin,
            self.pose_view_generation,
        ));
        let (rect, _) = ui.allocate_exact_size(vec2(64., 22.), Sense::hover());
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id(id)
                .max_rect(rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.set_clip_rect(rect.intersect(ui.clip_rect()));
        child.spacing_mut().interact_size = vec2(64., 22.);
        let focused = child.memory(|memory| memory.has_focus(child.next_auto_id()));
        let parsed = std::cell::Cell::new(None);
        let factor = self.slider_factor(property);
        let response = child.add_enabled(
            enabled && self.slider_input_available(ui.ctx()),
            egui::DragValue::new(&mut shown)
                .speed(0.2)
                .range(
                    if property == SliderProperty::Time {
                        0.
                    } else {
                        f64::MIN
                    }..=f64::MAX,
                )
                .clamp_existing_to_range(false)
                .update_while_editing(false)
                .custom_formatter(move |value, _| {
                    if focused {
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
                                && (property != SliderProperty::Time || *value >= 0.)
                        });
                    parsed.set(Some(value.is_some()));
                    value
                })
                .suffix(if property == SliderProperty::Mix {
                    "%"
                } else {
                    ""
                }),
        );
        self.slider_target(ui, &format!("slider{property:?}Value"), response.rect);
        self.slider_number_response(index, property, &response, shown, true, parsed.get());
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
        let mut shown = self
            .slider_view
            .edit
            .as_ref()
            .filter(|edit| edit.constraint == index && edit.property == property)
            .map_or(value * self.slider_factor(property), |edit| edit.shown);
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
        self.slider_number_response(index, property, &response, shown, false, None);
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
        let selected = match self.selection {
            Some(TreeItem::Constraint(index)) if self.slider_constraint(index).is_some() => {
                Some(index)
            }
            _ => None,
        };
        serde_json::json!({"selected":selected,"value":selected.and_then(|index|self.slider_constraint(index)),
            "authored":selected.and_then(|index|self.session.slider_pose(index,self.weights_edit_target()).ok()),
            "active":self.slider_view.edit.as_ref().map(|edit|serde_json::json!({
                "constraint":edit.constraint,"property":edit.property,"setup":edit.setup,
                "animation":edit.animation,"time":edit.time,"autoKey":edit.auto_key,
                "numeric":edit.numeric,"touched":edit.touched,"validText":edit.valid_text,"shown":edit.shown})),
            "blocked":self.slider_view.blocked.is_some(),"error":self.slider_view.error})
    }

    pub(super) fn slider_text_invalid(&self) -> bool {
        self.slider_view
            .edit
            .as_ref()
            .is_some_and(|edit| !edit.valid_text)
    }
}
