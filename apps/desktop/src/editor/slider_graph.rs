//! Time/Mix share the scalar Graph's exact keys and owned value transaction.
//! Presentation converts host/referenced seconds to frames and Mix to percent.
use super::*;
use peachy_engine::{AnimationKey, KeyOwner};

#[cfg(test)]
#[path = "slider_graph_tests.rs"]
mod tests;

fn color(property: SliderProperty) -> Color32 {
    match property {
        SliderProperty::Time => Color32::from_rgb(90, 180, 205),
        SliderProperty::Mix => Color32::from_rgb(195, 120, 205),
    }
}

impl Editor {
    fn selected_slider_graph_key(&self, constraint: usize) -> Option<(SliderProperty, f32)> {
        if let Some(AnimationKey::Slider {
            constraint: selected,
            property,
            time,
        }) = self.preferred_scalar_curve_selection()
            && selected == constraint
        {
            return Some((property, time));
        }
        self.key_selection.keys().iter().find_map(|key| match key {
            AnimationKey::Slider {
                constraint: selected,
                property,
                time,
            } if *selected == constraint => Some((*property, *time)),
            _ => None,
        })
    }

    pub(super) fn value_graph_context(&self) -> Option<GraphContext> {
        if let Some(KeyOwner::Slider { constraint }) = self.timeline_owner() {
            (!self.setup_mode).then_some(GraphContext::Slider {
                animation: self.animation,
                constraint,
                property: self.graph.slider_property,
            })
        } else {
            self.graph_context()
        }
    }

    pub(in crate::editor) fn slider_graph_has_focus(&self, ctx: &egui::Context) -> bool {
        self.graph_visible()
            && matches!(self.timeline_owner(), Some(KeyOwner::Slider { .. }))
            && !ctx.text_edit_focused()
            && self.slot_authoring_input_available(ctx)
            && ctx.memory(|memory| memory.has_focus(Self::graph_id()))
    }

    fn choose_slider_graph_property(&mut self, property: SliderProperty) {
        if property != self.graph.slider_property {
            self.cancel_gesture();
            self.key_selection.clear();
            self.graph.slider_property = property;
            self.graph.selected = None;
        }
    }

    pub(super) fn slider_graph_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let Some(KeyOwner::Slider { constraint }) = self.timeline_owner() else {
            return false;
        };
        self.animation_controls(ui);
        if let Some((property, time)) = self.selected_slider_graph_key(constraint) {
            self.graph.slider_property = property;
            self.graph.selected = Some(time);
        }
        let context = GraphContext::Slider {
            animation: self.animation,
            constraint,
            property: self.graph.slider_property,
        };
        if self.graph.context != Some(context) {
            if self.shared_curve_owner() != Some(curve_gesture::CurveInputOwner::Curves) {
                self.cancel_graph_edit();
            }
            self.graph.context = Some(context);
            self.graph.selected = self
                .selected_slider_graph_key(constraint)
                .map(|(_, time)| time);
            self.graph.bounds = None;
            self.graph.fit = true;
            self.graph.error = None;
        }
        let enabled = self.slot_authoring_input_available(ui.ctx());
        let picker_enabled = ui.ctx().input(|input| input.focused)
            && self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
            && self.export_dialog.is_none()
            && !self.slot_color_dialog_open();
        ui.horizontal(|ui| {
            ui.strong("Slider");
            for property in SliderProperty::ALL {
                let response = ui.add_enabled(
                    enabled,
                    egui::Button::selectable(
                        property == self.graph.slider_property,
                        if property == SliderProperty::Time {
                            "Frame"
                        } else {
                            "Mix"
                        },
                    ),
                );
                self.record_target(&format!("graphSliderProperty:{property:?}"), response.rect);
                if response.clicked() {
                    self.choose_slider_graph_property(property);
                }
            }
            let fit = ui.add_enabled(enabled, egui::Button::new("Fit").small());
            self.record_target("graphFit", fit.rect);
            if fit.clicked() {
                self.cancel_gesture();
                self.graph.fit = true;
            }
            let zoom = ui.add_enabled(
                enabled && self.graph.selected.is_some(),
                egui::Button::new("Zoom Keys").small(),
            );
            self.record_target("graphZoomKeys", zoom.rect);
            if zoom.clicked() {
                self.cancel_gesture();
                self.fit_graph(context, true);
            }
        });
        // Changing the property is a view action; render the new typed context
        // immediately so no following response can target the previous channel.
        let context = GraphContext::Slider {
            animation: self.animation,
            constraint,
            property: self.graph.slider_property,
        };
        self.graph.context = Some(context);
        let selection = self.graph.selected;
        let fps = self.session.document().fps as f32;
        let mut chosen = None;
        let mut targets = Vec::new();
        let picker = ui
            .add_enabled_ui(picker_enabled, |ui| {
                egui::ComboBox::from_id_salt("graph-slider-key")
                    .width(300.)
                    .selected_text(selection.map_or_else(
                        || "Choose exact key".into(),
                        |time| format!("Key at {} frames", time * fps),
                    ))
                    .show_ui(ui, |ui| {
                        if let Some(keys) = self.graph_keys(context) {
                            egui::ScrollArea::vertical().max_height(180.).show_rows(
                                ui,
                                22.,
                                keys.len(),
                                |ui, rows| {
                                    for index in rows {
                                        let key = keys[index];
                                        let response = ui.selectable_label(
                                            selection == Some(key.time),
                                            format!(
                                                "{} frames · {} s · {}",
                                                key.time * fps,
                                                key.time,
                                                graph_number(
                                                    f64::from(key.value)
                                                        * self.graph_factor(context)
                                                )
                                            ),
                                        );
                                        targets.push((
                                            format!("graphSliderKeyChoice:{index}"),
                                            response.rect,
                                        ));
                                        if response
                                            .on_hover_text(format!(
                                                "Exact host time {} s · stored value {}",
                                                key.time, key.value
                                            ))
                                            .clicked()
                                        {
                                            chosen = Some(key.time);
                                        }
                                    }
                                },
                            );
                        }
                    })
            })
            .inner;
        self.record_target("graphSliderKeyPicker", picker.response.rect);
        for (name, rect) in targets {
            self.record_target(&name, rect);
        }
        if let Some(time) = chosen {
            self.cancel_gesture();
            self.select_scalar_graph_key(context, time, false);
        }
        ui.horizontal(|ui| self.curve_preset_controls(ui, "graphSliderCurve"));
        let channel = self.selected_curve_channels().ok().and_then(|channels| {
            channels.into_iter().find(|channel| {
                matches!(channel.target.key,AnimationKey::Slider{constraint:selected,property,..}
                if selected==constraint&&property==self.graph.slider_property)
            })
        });
        let handles = egui::CollapsingHeader::new("Handles")
            .id_salt("slider-curve-handles")
            .default_open(false)
            .show(ui, |ui| {
                if let Some(channel) = channel.as_ref()
                    && curve_controller::value_handles(channel).is_some()
                {
                    self.graph_curve_numeric_controls(
                        ui,
                        curve_gesture::CurveInputOwner::SliderGraph,
                        channel,
                    );
                } else {
                    curve_widgets::handle_fields(
                        ui,
                        "slider-curve-placeholder",
                        &mut Default::default(),
                        false,
                        false,
                        ui.available_width() > 650.,
                        ["Host frame 1", "Value 1", "Host frame 2", "Value 2"],
                    );
                }
            });
        self.record_target("graphSliderHandles", handles.header_response.rect);
        if let Some(error) = self.curves.error.as_ref() {
            ui.colored_label(ACCENT, error);
        }
        if !SliderProperty::ALL.into_iter().any(|property| {
            self.graph_keys(context.slider_channel(property))
                .is_some_and(|keys| !keys.is_empty())
        }) {
            ui.label("This Slider has no Frame or Mix keys. Key a property to create its curve.");
            return true;
        }
        if self.graph.fit || self.graph.bounds.is_none() {
            self.fit_graph(context, false);
        }
        let Some(bounds) = self.graph.bounds else {
            ui.colored_label(ACCENT, "Graph values cannot be displayed. Use Fit.");
            return true;
        };
        let (_, canvas) =
            ui.allocate_space(vec2(ui.available_width(), ui.available_height().max(100.)));
        let plot_rect =
            egui::Rect::from_min_max(canvas.min + vec2(244., 24.), canvas.max - vec2(62., 8.));
        self.record_target("graphCanvas", canvas);
        self.record_target("graphSliderCanvas", canvas);
        self.record_target("graphClip", ui.clip_rect());
        self.record_target("graphPlot", plot_rect);
        if !bounds.valid()
            || !plot_rect.is_finite()
            || plot_rect.width() <= 0.
            || plot_rect.height() <= 0.
        {
            return true;
        }
        let painter = ui.painter_at(canvas);
        painter.rect_filled(canvas, 3., Color32::from_rgb(21, 26, 33));
        let rail =
            egui::Rect::from_min_max(canvas.min, pos2(plot_rect.left() - 5., canvas.bottom()));
        painter.rect_filled(rail, 0., Color32::from_gray(53));
        painter.text(
            pos2(rail.left() + 12., rail.top() + 35.),
            egui::Align2::LEFT_CENTER,
            "Sliders",
            FontId::proportional(12.),
            MINT,
        );
        let name = self
            .slider_constraint(constraint)
            .map_or("", |value| value.name.as_str())
            .to_owned();
        for (index, property) in SliderProperty::ALL.into_iter().enumerate() {
            let row = egui::Rect::from_min_max(
                pos2(rail.left() + 5., rail.top() + 48. + index as f32 * 24.),
                pos2(rail.right() - 3., rail.top() + 70. + index as f32 * 24.),
            );
            let response = ui.interact(
                row,
                egui::Id::new(("graph-slider-row", constraint, property)),
                Sense::click(),
            );
            self.slider_target(ui, &format!("graphSliderRow:{property:?}"), row);
            painter.text(
                pos2(row.left() + 16., row.center().y),
                egui::Align2::LEFT_CENTER,
                format!(
                    "Slider {}: {name}",
                    if property == SliderProperty::Time {
                        "frame"
                    } else {
                        "mix"
                    }
                ),
                FontId::proportional(12.),
                color(property),
            );
            if enabled && response.clicked() {
                self.choose_slider_graph_property(property);
            }
        }
        if let Some(channel) = channel.as_ref() {
            let factor = self.graph_factor(context);
            self.curve_handles_on_surface(
                ui,
                channel,
                curve_controller::CurveSurface {
                    owner: curve_gesture::CurveInputOwner::SliderGraph,
                    rect: plot_rect,
                    bounds: [
                        bounds.time_min,
                        bounds.time_max,
                        bounds.value_min / factor,
                        bounds.value_max / factor,
                    ],
                    normalized: false,
                    color: color(self.graph.slider_property),
                },
            );
        }
        let response = ui.interact(plot_rect, Self::graph_id(), Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Other,
                true,
                "Slider Frame and Mix value graph",
            )
        });
        if response.clicked_elsewhere() {
            response.surrender_focus();
        }
        if self
            .graph
            .edit
            .as_ref()
            .is_some_and(|edit| edit.rect != plot_rect)
        {
            self.cancel_graph_edit();
        }
        let mut plots = Vec::new();
        for property in SliderProperty::ALL {
            let context = context.slider_channel(property);
            if let Some(keys) = self.graph_keys(context)
                && !keys.is_empty()
            {
                match graph_plot_values(
                    keys,
                    self.graph_neutral(context),
                    self.graph_factor(context),
                    bounds,
                    plot_rect,
                    self.graph.selected,
                ) {
                    Ok(plot) => plots.push((context, plot)),
                    Err(error) => self.graph.error = Some(error),
                }
            }
        }
        if !self.graph.blocked
            && !self.shared_curve_edit_active()
            && enabled
            && let Some((point, modifiers)) =
                ui.input(|input| key_drag::primary_pointer_event(&input.events, true))
            && !modifiers.alt
            && plot_rect.contains(point)
            && ui.clip_rect().contains(point)
            && (response.contains_pointer() || response.is_pointer_button_down_on())
        {
            let pressed = plots
                .iter()
                .flat_map(|(context, plot)| {
                    plot.keys
                        .iter()
                        .map(move |(key, position)| (*context, *key, *position))
                })
                .filter(|(_, _, position)| position.distance(point) <= 8.)
                .min_by(|a, b| a.2.distance(point).total_cmp(&b.2.distance(point)))
                .map(|(context, key, _)| (context, key));
            response.request_focus();
            if let Some((selected, key)) = pressed {
                let GraphContext::Slider { property, .. } = selected else {
                    unreachable!()
                };
                self.graph.slider_property = property;
                self.graph.context = Some(selected);
                self.select_scalar_graph_key(
                    selected,
                    key.time,
                    modifiers.command || modifiers.ctrl,
                );
                if !modifiers.command && !modifiers.ctrl {
                    self.begin_graph_edit(selected, key, point, plot_rect, bounds);
                }
            } else {
                self.key_selection.clear();
                self.graph.selected = None;
            }
        }
        if self.graph.edit.is_some() {
            if let Some(point) = ui.input(|input| input.pointer.interact_pos()) {
                self.update_graph_edit(point);
            }
            if !ui.input(|input| input.pointer.primary_down())
                && let Some(edit) = self.graph.edit.as_mut()
            {
                edit.finish_pending = true;
            }
            ui.ctx().request_repaint();
        }
        for tick in 0..=3 {
            let fraction = f64::from(tick) / 3.;
            let x = plot_rect.left() + plot_rect.width() * fraction as f32;
            let y = plot_rect.bottom() - plot_rect.height() * fraction as f32;
            painter.line_segment(
                [pos2(x, plot_rect.top()), pos2(x, plot_rect.bottom())],
                Stroke::new(1., Color32::from_gray(45)),
            );
            painter.line_segment(
                [pos2(plot_rect.left(), y), pos2(plot_rect.right(), y)],
                Stroke::new(1., Color32::from_gray(45)),
            );
            painter.text(
                pos2(x, canvas.top() + 11.),
                egui::Align2::CENTER_CENTER,
                graph_number(
                    (bounds.time_min + (bounds.time_max - bounds.time_min) * fraction)
                        * f64::from(fps),
                ),
                FontId::monospace(9.),
                MUTED,
            );
            painter.text(
                pos2(plot_rect.right() + 4., y),
                egui::Align2::LEFT_CENTER,
                graph_number(bounds.value_min + (bounds.value_max - bounds.value_min) * fraction),
                FontId::monospace(9.),
                MUTED,
            );
        }
        let clipped = painter.with_clip_rect(plot_rect);
        self.graph.rendered_keys = 0;
        self.graph.rendered_points = 0;
        for property in SliderProperty::ALL {
            let context = context.slider_channel(property);
            let Some(keys) = self.graph_keys(context) else {
                continue;
            };
            let Ok(plot) = graph_plot_values(
                keys,
                self.graph_neutral(context),
                self.graph_factor(context),
                bounds,
                plot_rect,
                self.graph.selected,
            ) else {
                continue;
            };
            self.graph.rendered_keys += plot.keys.len();
            self.graph.rendered_points += plot.curve.len();
            clipped.add(egui::Shape::line(
                plot.curve,
                Stroke::new(1.5, color(property)),
            ));
            for (index, (key, point)) in plot.keys.into_iter().enumerate() {
                if !point.is_finite()
                    || !plot_rect.contains(point)
                    || !ui.clip_rect().contains(point)
                {
                    continue;
                }
                let target = egui::Rect::from_center_size(point, vec2(10., 10.))
                    .intersect(plot_rect)
                    .intersect(ui.clip_rect());
                self.record_target(&format!("graphSliderKey:{property:?}:{}", key.time), target);
                self.record_target(&format!("graphSliderDrawKey:{property:?}:{index}"), target);
                let selected = self.key_selection.keys().contains(&context.key(key.time));
                clipped.rect_filled(
                    egui::Rect::from_center_size(
                        point,
                        Vec2::splat(if selected { 7. } else { 5. }),
                    ),
                    0.,
                    if selected { ACCENT } else { color(property) },
                );
            }
        }
        if (bounds.time_min..=bounds.time_max).contains(&f64::from(self.time)) {
            let x = bounds.point_value(plot_rect, self.time, 0.).x;
            clipped.line_segment(
                [pos2(x, plot_rect.top()), pos2(x, plot_rect.bottom())],
                Stroke::new(1., ACCENT),
            );
        }
        if let Some(error) = self.graph.error.as_ref() {
            ui.colored_label(ACCENT, error);
        }
        true
    }
}
