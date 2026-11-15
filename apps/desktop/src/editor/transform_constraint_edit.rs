//! Owned constraint-field transactions; mapping conversion stays in the engine.
use super::*;
use peachy_engine::{
    TransformMappingRangeField, edited_transform_mapping_range, transform_mapping_range,
};

#[cfg(test)]
#[path = "transform_mapping_ui_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum ConstraintField {
    Mix(TransformProperty),
    Offset(TransformProperty),
    Range(TransformMappingRangeField),
}

impl ConstraintField {
    pub fn key(self) -> String {
        format!("{self:?}")
    }
    pub fn factor(self) -> f64 {
        if matches!(self, Self::Mix(_)) {
            100.
        } else {
            1.
        }
    }
}

pub(super) struct ConstraintEdit {
    index: usize,
    field: ConstraintField,
    baseline: TransformConstraint,
    accepted: TransformConstraint,
    control: egui::Id,
    rect: egui::Rect,
    animation: usize,
    time: f32,
    skin: String,
    generation: u64,
    numeric: bool,
    touched: bool,
    shown: f64,
    undefined: bool,
    finish: bool,
}

#[derive(Default)]
pub(super) struct TransformConstraintView {
    edit: Option<ConstraintEdit>,
    blocked: Option<egui::Id>,
    pub error: Option<String>,
}

impl Editor {
    pub(super) fn constraint_edit_active(&self) -> bool {
        self.transform_constraint_view.edit.is_some()
    }
    fn constraint_input_available(&self, ctx: &egui::Context) -> bool {
        self.setup_mode && self.show_tree && self.weights_input_available(ctx)
    }
    pub(super) fn cancel_constraint_edit(&mut self) {
        if let Some(edit) = self.transform_constraint_view.edit.take() {
            self.session.cancel_transaction();
            self.transform_constraint_view.blocked = Some(edit.control);
        }
        self.transform_constraint_view.error = None;
    }
    fn finish_constraint_edit(&mut self) {
        if self.transform_constraint_view.edit.take().is_some() {
            self.session.commit_transaction();
        }
    }
    pub(super) fn reconcile_constraint_context(&mut self) {
        if self
            .transform_constraint_view
            .edit
            .as_ref()
            .is_some_and(|edit| {
                !self.setup_mode
                    || !self.show_tree
                    || self.selection != Some(TreeItem::Constraint(edit.index))
                    || self.animation != edit.animation
                    || self.time != edit.time
                    || self.skin != edit.skin
                    || self.pose_view_generation != edit.generation
                    || self.transform_constraint(edit.index) != Some(&edit.accepted)
            })
        {
            self.cancel_constraint_edit();
        }
    }
    pub(super) fn reconcile_constraint_input(&mut self, ctx: &egui::Context) {
        if ctx.input(|input| input.pointer.button_pressed(egui::PointerButton::Primary)) {
            self.transform_constraint_view.blocked = None;
        }
        self.reconcile_constraint_context();
        if !self.constraint_input_available(ctx) {
            self.cancel_constraint_edit();
        }
        if let Some(control) = self.transform_constraint_view.blocked {
            ctx.memory_mut(|memory| memory.surrender_focus(control));
        }
    }
    pub(super) fn settle_constraint_input(&mut self, ctx: &egui::Context) {
        self.reconcile_constraint_input(ctx);
        if !ctx.input(|input| input.pointer.any_down())
            && self
                .transform_constraint_view
                .edit
                .as_ref()
                .is_some_and(|edit| edit.finish)
        {
            self.finish_constraint_edit();
        }
        if ctx.input(|input| {
            input.pointer.button_pressed(egui::PointerButton::Primary)
                || (!input.pointer.primary_down()
                    && !input.pointer.button_released(egui::PointerButton::Primary))
        }) {
            self.transform_constraint_view.blocked = None;
        }
    }
    fn begin_constraint_number(
        &mut self,
        index: usize,
        field: ConstraintField,
        response: &egui::Response,
        numeric: bool,
    ) -> Result<(), String> {
        self.finish_constraint_edit();
        self.cancel_gesture();
        if !self.constraint_input_available(&response.ctx)
            || self.selection != Some(TreeItem::Constraint(index))
        {
            return Err("Select a Transform constraint in Setup".into());
        }
        let baseline = self
            .transform_constraint(index)
            .cloned()
            .ok_or("Select a Transform constraint")?;
        let initial = match field {
            ConstraintField::Mix(property) => Some(f64::from(baseline.mix.get(property)) * 100.),
            ConstraintField::Offset(property) => Some(f64::from(baseline.offsets.get(property))),
            ConstraintField::Range(field) => {
                let range = transform_mapping_range(
                    self.session.document(),
                    index,
                    TransformProperty::X,
                    TransformProperty::Rotate,
                )?;
                let value = range.value(field);
                if value.is_none()
                    && !(field == TransformMappingRangeField::SourceMax
                        && range.target_min != range.target_max)
                {
                    return Err("This source range has no finite maximum".into());
                }
                value
            }
        };
        self.session.commit_transaction();
        self.playing = false;
        self.session.begin_transaction();
        self.transform_constraint_view.edit = Some(ConstraintEdit {
            index,
            field,
            accepted: baseline.clone(),
            baseline,
            control: response.id,
            rect: response.rect,
            animation: self.animation,
            time: self.time,
            skin: self.skin.clone(),
            generation: self.pose_view_generation,
            numeric,
            touched: false,
            // The undefined draft stays blank. This storage placeholder is never
            // published without a genuine finite text entry.
            shown: initial.unwrap_or(0.),
            undefined: initial.is_none(),
            finish: false,
        });
        Ok(())
    }
    fn preview_constraint_number(&mut self, shown: f64) -> Result<(), String> {
        let edit = self
            .transform_constraint_view
            .edit
            .as_ref()
            .ok_or("No active constraint edit")?;
        let raw = shown / edit.field.factor();
        if !raw.is_finite() {
            return Err("Enter a finite number".into());
        }
        let candidate = match edit.field {
            ConstraintField::Range(field) => edited_transform_mapping_range(
                &edit.accepted,
                TransformProperty::X,
                TransformProperty::Rotate,
                field,
                raw,
            )?,
            ConstraintField::Mix(property) | ConstraintField::Offset(property) => {
                let value = raw as f32;
                if !value.is_finite() {
                    return Err("Value is outside the finite constraint range".into());
                }
                let mut candidate = edit.baseline.clone();
                if matches!(edit.field, ConstraintField::Mix(_)) {
                    candidate.mix.set(property, value);
                } else {
                    candidate.offsets.set(property, value);
                }
                candidate
            }
        };
        let index = edit.index;
        self.session
            .preview_transaction(Command::SetTransformConstraint {
                constraint: index,
                value: candidate.clone(),
            })?;
        let edit = self.transform_constraint_view.edit.as_mut().unwrap();
        edit.accepted = candidate;
        edit.shown = shown;
        edit.undefined = false;
        Ok(())
    }
    fn number_touched(ctx: &egui::Context) -> bool {
        ctx.input(|input|input.events.iter().any(|event|matches!(event,
            egui::Event::Text(_)|egui::Event::Paste(_)|egui::Event::Cut
            |egui::Event::Ime(egui::ImeEvent::Commit(_)|egui::ImeEvent::DeleteSurrounding{..})
            |egui::Event::Key{key:egui::Key::Backspace|egui::Key::Delete|egui::Key::ArrowUp|egui::Key::ArrowDown,pressed:true,..}
        ) || matches!(event,egui::Event::Key{key:egui::Key::Z|egui::Key::Y|egui::Key::X,pressed:true,modifiers,..} if modifiers.command)))
    }
    pub(super) fn constraint_edit_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        let Some(edit) = self.transform_constraint_view.edit.as_ref() else {
            return false;
        };
        if !self.constraint_input_available(ctx) {
            self.cancel_constraint_edit();
            return false;
        }
        let id = edit.control;
        let owns = ctx.memory(|memory| memory.has_focus(id));
        if owns && edit.numeric && Self::number_touched(ctx) {
            self.transform_constraint_view
                .edit
                .as_mut()
                .unwrap()
                .touched = true;
        }
        if owns && ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::S))
        {
            self.cancel_constraint_edit();
            ctx.memory_mut(|memory| memory.surrender_focus(id));
            self.save_project();
            return true;
        }
        if (!ctx.text_edit_focused() || owns)
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.cancel_constraint_edit();
            ctx.memory_mut(|memory| memory.surrender_focus(id));
            return true;
        }
        false
    }
    fn constraint_number_response(
        &mut self,
        index: usize,
        field: ConstraintField,
        response: &egui::Response,
        shown: f64,
        numeric: bool,
        parsed: Option<bool>,
    ) {
        if self.transform_constraint_view.blocked.is_some()
            || !self.constraint_input_available(&response.ctx)
        {
            return;
        }
        let starts = (numeric && response.gained_focus())
            || response.drag_started()
            || (response.changed() && (!numeric || Self::number_touched(&response.ctx)));
        if starts
            && self
                .transform_constraint_view
                .edit
                .as_ref()
                .is_none_or(|edit| edit.control != response.id)
            && let Err(error) = self.begin_constraint_number(
                index,
                field,
                response,
                numeric && response.has_focus() && !response.dragged(),
            )
        {
            self.transform_constraint_view.error = Some(error);
            return;
        }
        let Some(edit) = self
            .transform_constraint_view
            .edit
            .as_mut()
            .filter(|edit| edit.control == response.id)
        else {
            return;
        };
        if !edit.numeric && edit.rect != response.rect {
            self.cancel_constraint_edit();
            return;
        }
        if edit.numeric && Self::number_touched(&response.ctx) {
            edit.touched = true;
        }
        let explicit = !edit.numeric || edit.touched;
        let changed = edit.shown.to_bits() != shown.to_bits();
        let entered_undefined =
            edit.undefined && edit.numeric && edit.touched && parsed == Some(true);
        if explicit
            && (response.changed() || entered_undefined)
            && (changed || entered_undefined)
            && (!edit.undefined || entered_undefined)
        {
            self.transform_constraint_view.error = self.preview_constraint_number(shown).err();
        } else if explicit && parsed == Some(false) {
            self.transform_constraint_view.error = Some("Enter a finite number".into());
        }
        if (response.lost_focus()
            || (!(numeric && response.has_focus())
                && !response.ctx.input(|input| input.pointer.any_down())))
            && let Some(edit) = self.transform_constraint_view.edit.as_mut()
        {
            edit.finish = true;
        }
    }
    pub(super) fn constraint_number(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        field: ConstraintField,
        value: Option<f64>,
        target: &str,
        width: f32,
    ) {
        let factor = field.factor();
        let initial = value.unwrap_or(0.) * factor;
        let edit = self
            .transform_constraint_view
            .edit
            .as_ref()
            .filter(|edit| edit.index == index && edit.field == field);
        let mut shown = edit.map_or(initial, |edit| edit.shown);
        let undefined = edit.map_or(value.is_none(), |edit| edit.undefined);
        let (rect, _) = ui.allocate_exact_size(vec2(width, 22.), Sense::hover());
        let id = egui::Id::new((
            "constraint-number",
            index,
            field.key(),
            self.setup_mode,
            self.animation,
            self.time.to_bits(),
            &self.skin,
            self.pose_view_generation,
        ));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id(id)
                .max_rect(rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.set_clip_rect(rect.intersect(ui.clip_rect()));
        child.spacing_mut().interact_size = vec2(width, 22.);
        let widget_id = child.next_auto_id();
        let focused = child.memory(|memory| memory.has_focus(widget_id));
        let parsed = std::cell::Cell::new(None);
        let recoverable_source_max = value.is_none()
            && field == ConstraintField::Range(TransformMappingRangeField::SourceMax)
            && transform_mapping_range(
                self.session.document(),
                index,
                TransformProperty::X,
                TransformProperty::Rotate,
            )
            .is_ok_and(|range| range.source_max.is_none() && range.target_min != range.target_max);
        let enabled = (value.is_some() || recoverable_source_max)
            && self.constraint_input_available(ui.ctx())
            && self
                .transform_constraint(index)
                .is_some_and(|value| value.supported_subset().is_ok());
        let response = child.add_enabled(
            enabled,
            egui::DragValue::new(&mut shown)
                .range(f64::MIN..=f64::MAX)
                .clamp_existing_to_range(false)
                .update_while_editing(false)
                .speed(0.25)
                .custom_formatter(move |value, _| {
                    if undefined || value.is_nan() {
                        String::new()
                    } else if focused {
                        value.to_string()
                    } else if value != 0. && (value.abs() >= 1e7 || value.abs() < 1e-4) {
                        format!("{value:.2e}")
                    } else {
                        format!("{value:.2}")
                            .trim_end_matches('0')
                            .trim_end_matches('.')
                            .to_string()
                    }
                })
                .custom_parser(|text| {
                    let value = text.parse::<f64>().ok().filter(|value| value.is_finite());
                    parsed.set(Some(value.is_some()));
                    value
                })
                .suffix(if matches!(field, ConstraintField::Mix(_)) {
                    "%"
                } else {
                    ""
                }),
        );
        let visible = response.rect.intersect(rect).intersect(ui.clip_rect());
        if visible.is_positive() && visible.is_finite() {
            self.record_target(target, visible);
        } else {
            self.capture_targets.remove(target);
        }
        self.constraint_number_response(index, field, &response, shown, true, parsed.get());
    }
    pub(super) fn constraint_mix_slider(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        property: TransformProperty,
        value: f32,
    ) {
        let field = ConstraintField::Mix(property);
        let mut shown = self
            .transform_constraint_view
            .edit
            .as_ref()
            .filter(|edit| edit.index == index && edit.field == field)
            .map_or(f64::from(value) * 100., |edit| edit.shown);
        let id = egui::Id::new((
            "constraint-mix-slider",
            index,
            property,
            self.pose_view_generation,
        ));
        let response = ui
            .scope_builder(egui::UiBuilder::new().id(id), |ui| {
                ui.spacing_mut().slider_width = ui.available_width().max(32.);
                ui.add_enabled(
                    self.constraint_input_available(ui.ctx())
                        && self
                            .transform_constraint(index)
                            .is_some_and(|value| value.supported_subset().is_ok()),
                    egui::Slider::new(&mut shown, 0.0..=100.)
                        .show_value(false)
                        .clamping(egui::SliderClamping::Edits),
                )
            })
            .inner;
        let visible = response.rect.intersect(ui.clip_rect());
        if visible.is_positive() && visible.is_finite() {
            self.record_target(
                &format!("transformMixSlider{}", property.spine_name()),
                visible,
            );
        }
        self.constraint_number_response(index, field, &response, shown, false, None);
    }
    pub(super) fn constraint_offset_slider(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        property: TransformProperty,
        value: f32,
    ) {
        let field = ConstraintField::Offset(property);
        let mut shown = self
            .transform_constraint_view
            .edit
            .as_ref()
            .filter(|edit| edit.index == index && edit.field == field)
            .map_or(f64::from(value), |edit| edit.shown);
        let enabled = self.constraint_input_available(ui.ctx())
            && self
                .transform_constraint(index)
                .is_some_and(|value| value.supported_subset().is_ok());
        let id = egui::Id::new((
            "constraint-offset-slider",
            index,
            property,
            self.pose_view_generation,
        ));
        let response = ui
            .scope_builder(egui::UiBuilder::new().id(id), |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    let (rect, mut response) =
                        ui.allocate_exact_size(vec2(115., 22.), Sense::click_and_drag());
                    let rail = rect.shrink2(vec2(4., 5.));
                    if enabled
                        && let Some(pointer) = response.interact_pointer_pos()
                        && pointer.is_finite()
                        && ((response.is_pointer_button_down_on()
                            && ui.input(|input| input.pointer.primary_down()))
                            || response.dragged_by(egui::PointerButton::Primary)
                            || response.drag_stopped_by(egui::PointerButton::Primary)
                            || response.clicked_by(egui::PointerButton::Primary))
                    {
                        let next = -50.
                            + 100. * (f64::from(pointer.x) - f64::from(rail.left()))
                                / f64::from(rail.width());
                        if next.is_finite() && next.to_bits() != shown.to_bits() {
                            shown = next;
                            response.mark_changed();
                        }
                        if ui.input(|input| {
                            input.pointer.button_pressed(egui::PointerButton::Primary)
                        }) {
                            response.request_focus();
                        }
                    }
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rail, 3., ui.visuals().extreme_bg_color);
                    painter.rect_stroke(
                        rail,
                        3.,
                        ui.visuals().widgets.inactive.bg_stroke,
                        egui::StrokeKind::Inside,
                    );
                    painter.line_segment(
                        [
                            pos2(rail.center().x, rail.top()),
                            pos2(rail.center().x, rail.bottom()),
                        ],
                        Stroke::new(1., MUTED),
                    );
                    let x = f64::from(rail.left()) + (shown + 50.) / 100. * f64::from(rail.width());
                    if x.is_finite() && x >= f64::from(rail.left()) && x <= f64::from(rail.right())
                    {
                        let thumb = egui::Rect::from_center_size(
                            pos2(x as f32, rail.center().y),
                            vec2(5., 18.),
                        );
                        painter.rect_filled(thumb, 2., if enabled { MINT } else { MUTED });
                    }
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Slider,
                            enabled,
                            format!("Translate {} offset", property.spine_name()),
                        )
                    });
                    response
                })
                .inner
            })
            .inner;
        let visible = response.rect.intersect(ui.clip_rect());
        if visible.is_positive() && visible.is_finite() {
            self.record_target(
                &format!("transformOffsetSlider{}", property.spine_name()),
                visible,
            );
        }
        self.constraint_number_response(index, field, &response, shown, false, None);
    }
    pub(super) fn set_constraint_clamp(&mut self, index: usize, clamp: bool) {
        self.finish_constraint_edit();
        self.cancel_gesture();
        if self.setup_mode
            && let Some(mut value) = self.transform_constraint(index).cloned()
        {
            value.clamp = clamp;
            let _ = self.dispatch(Command::SetTransformConstraint {
                constraint: index,
                value,
            });
        }
    }
    pub(super) fn constraint_edit_diagnostic(&self) -> serde_json::Value {
        let selected = match self.selection {
            Some(TreeItem::Constraint(index)) if self.transform_constraint(index).is_some() => {
                Some(index)
            }
            _ => None,
        };
        let range = selected.and_then(|index| {
            transform_mapping_range(
                self.session.document(),
                index,
                TransformProperty::X,
                TransformProperty::Rotate,
            )
            .ok()
        });
        serde_json::json!({"selected":selected,"value":selected.and_then(|index|self.transform_constraint(index)),"range":range,"active":self.transform_constraint_view.edit.as_ref().map(|edit|serde_json::json!({
            "constraint":edit.index,"field":edit.field.key(),"value":(!edit.undefined).then_some(edit.shown),"undefined":edit.undefined,"numeric":edit.numeric,
            "animation":edit.animation,"time":edit.time,"skin":edit.skin,"finishPending":edit.finish})),
            "blocked":self.transform_constraint_view.blocked.is_some(),"error":self.transform_constraint_view.error})
    }
}
