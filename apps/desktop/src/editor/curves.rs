//! Shared channel selection and the relative Curves surface.
use super::*;
use peachy_engine::{
    AnimationKey, CurveChannel, CurveData, CurvePreset, CurveSelection, curve_channels_for_targets,
    curve_targets,
};

pub(super) struct CurvesView {
    primary: Option<(AnimationKey, usize)>,
    selection: Vec<AnimationKey>,
    bounds: Option<[f64; 4]>,
    pub(super) error: Option<String>,
    pub(super) match_handles: bool,
    pub(super) separate: bool,
    pub(super) edit: Option<super::curve_controller::HandleEdit>,
    pub(super) draft: Option<super::curve_controller::HandleDraft>,
}

impl Default for CurvesView {
    fn default() -> Self {
        Self {
            primary: None,
            selection: Vec::new(),
            bounds: None,
            error: None,
            match_handles: true,
            separate: false,
            edit: None,
            draft: None,
        }
    }
}

#[cfg(test)]
#[path = "curves_tests.rs"]
mod tests;

fn relative_handles(channel: &CurveChannel) -> Option<[f64; 4]> {
    let [left, right] = channel.values;
    let span = right - left;
    if span == 0. {
        return None;
    }
    match channel.curve {
        CurveData::Standard(Curve::Bezier(h)) => Some(h.map(f64::from)),
        CurveData::Standard(Curve::BezierOffset(h)) if span != 0. => Some([
            f64::from(h[0]),
            f64::from(h[1]) / span,
            f64::from(h[2]),
            f64::from(h[3]) / span,
        ]),
        CurveData::Ik(IkCurve::Bezier { time, mix }) if span != 0. => Some([
            time[0],
            (f64::from(mix[0]) - left) / span,
            time[1],
            (f64::from(mix[1]) - left) / span,
        ]),
        _ => None,
    }
}

fn point(rect: egui::Rect, x: f64, y: f64) -> Pos2 {
    pos2(
        (f64::from(rect.left()) + x * f64::from(rect.width())) as f32,
        (f64::from(rect.bottom()) - y * f64::from(rect.height())) as f32,
    )
}

fn relative_point(rect: egui::Rect, bounds: [f64; 4], x: f64, y: f64) -> Pos2 {
    point(rect, x, (y - bounds[2]) / (bounds[3] - bounds[2]))
}

fn fitted_bounds(channels: &[&CurveChannel]) -> [f64; 4] {
    let (mut low, mut high) = (0f64, 1f64);
    for handles in channels
        .iter()
        .filter_map(|channel| relative_handles(channel))
    {
        for value in [handles[1], handles[3]]
            .into_iter()
            .filter(|value| value.is_finite())
        {
            low = low.min(value);
            high = high.max(value);
        }
    }
    let span = high - low;
    if !span.is_finite() || span <= 0. {
        return [0., 1., 0., 1.];
    }
    if low < 0. || high > 1. {
        let pad = span * 0.04;
        if (low - pad).is_finite()
            && (high + pad).is_finite()
            && (high - low + 2. * pad).is_finite()
        {
            low -= pad;
            high += pad;
        }
    }
    [0., 1., low, high]
}

fn relative_path(channel: &CurveChannel, area: egui::Rect, bounds: [f64; 4]) -> Vec<Pos2> {
    let point = |rect, x, y| relative_point(rect, bounds, x, y);
    if channel.values[0] == channel.values[1] {
        return Vec::new();
    }
    if matches!(
        channel.curve,
        CurveData::Standard(Curve::Linear) | CurveData::Ik(IkCurve::Linear)
    ) {
        return vec![point(area, 0., 0.), point(area, 1., 1.)];
    }
    if matches!(
        channel.curve,
        CurveData::Standard(Curve::Stepped) | CurveData::Ik(IkCurve::Stepped)
    ) {
        return vec![
            point(area, 0., 0.),
            point(area, 1., 0.),
            point(area, 1., 1.),
        ];
    }
    let Some(h) = relative_handles(channel) else {
        return Vec::new();
    };
    (0..=32)
        .map(|n| {
            let t = n as f64 / 32.;
            let s = 1. - t;
            point(
                area,
                3. * s * s * t * h[0] + 3. * s * t * t * h[2] + t * t * t,
                3. * s * s * t * h[1] + 3. * s * t * t * h[3] + t * t * t,
            )
        })
        .collect()
}

fn path_distance(pointer: Pos2, points: &[Pos2]) -> f32 {
    points
        .windows(2)
        .filter(|pair| pair.iter().all(|point| point.is_finite()))
        .map(|pair| {
            let edge = pair[1] - pair[0];
            let length = edge.length_sq();
            let t = if length > 0. {
                ((pointer - pair[0]).dot(edge) / length).clamp(0., 1.)
            } else {
                0.
            };
            pointer.distance(pair[0] + edge * t)
        })
        .filter(|distance| distance.is_finite())
        .fold(f32::INFINITY, f32::min)
}

fn channel_color(channel: &CurveChannel) -> Color32 {
    match &channel.target.key {
        AnimationKey::Bone { property, .. } => match *property {
            Property::Rotation => Color32::from_rgb(109, 210, 126),
            Property::X | Property::Y => Color32::from_rgb(105, 168, 225),
            Property::ScaleX | Property::ScaleY => Color32::from_rgb(211, 116, 116),
            Property::ShearX | Property::ShearY => Color32::from_rgb(190, 145, 210),
        },
        AnimationKey::SlotColor { .. } => [
            Color32::from_rgb(244, 74, 72),
            Color32::from_rgb(133, 229, 88),
            Color32::from_rgb(35, 171, 248),
            Color32::from_rgb(219, 121, 231),
        ][channel.channel],
        AnimationKey::Deform { .. } => Color32::from_rgb(45, 190, 225),
        _ => MINT,
    }
}

impl Editor {
    pub(super) fn preferred_scalar_curve_selection(&self) -> Option<AnimationKey> {
        if self.curves.selection.as_slice() != self.key_selection.keys() {
            return None;
        }
        let (
            AnimationKey::Bone {
                bone,
                property,
                time,
            },
            _,
        ) = self.curves.primary.as_ref()?
        else {
            return None;
        };
        self.key_selection.keys().iter().find(|key| matches!(key,
            AnimationKey::Bone {bone: selected_bone, property:selected_property,time:selected_time}
            if selected_bone==bone && selected_property==property && (selected_time==time || self.key_selection.len()==1)
        )).cloned()
    }

    pub(super) fn curves_id() -> egui::Id {
        egui::Id::new("curves-handle-canvas")
    }
    pub(super) fn curve_preset_selection(&self) -> Vec<CurveSelection> {
        self.key_selection
            .keys()
            .iter()
            .flat_map(|key| {
                let count = if matches!(key, AnimationKey::SlotColor { .. }) {
                    4
                } else {
                    1
                };
                (0..count)
                    .map(|channel| CurveSelection {
                        key: key.clone(),
                        channel,
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    pub(super) fn apply_selected_curve_preset(&mut self, preset: CurvePreset) {
        let selection = self.curve_preset_selection();
        self.apply_curve_preset_request(selection, preset);
    }

    pub(super) fn apply_curve_preset_request(
        &mut self,
        selection: Vec<CurveSelection>,
        preset: CurvePreset,
    ) {
        self.cancel_gesture();
        if self.setup_mode || selection.is_empty() {
            return;
        }
        self.playing = false;
        self.dispatch(Command::ApplyCurvePreset {
            animation: self.animation,
            selection,
            preset,
            separate: self.curves.separate,
        });
    }

    pub(super) fn curve_preset_controls(&mut self, ui: &mut egui::Ui, prefix: &str) {
        let channels = self.selected_curve_channels().unwrap_or_default();
        let source = channels
            .iter()
            .find(|channel| {
                self.curves.primary.as_ref() == Some(&(channel.target.key.clone(), channel.channel))
            })
            .or(channels.first());
        let enabled = !channels.is_empty() && self.slot_authoring_input_available(ui.ctx());
        for (glyph, preset) in [
            (
                curve_widgets::InterpolationGlyph::Stepped,
                CurvePreset::Stepped,
            ),
            (
                curve_widgets::InterpolationGlyph::Linear,
                CurvePreset::Linear,
            ),
            (
                curve_widgets::InterpolationGlyph::Bezier,
                CurvePreset::Bezier,
            ),
        ] {
            let selected = source.is_some_and(|channel| {
                matches!(
                    (glyph, channel.curve),
                    (
                        curve_widgets::InterpolationGlyph::Linear,
                        CurveData::Standard(Curve::Linear) | CurveData::Ik(IkCurve::Linear),
                    ) | (
                        curve_widgets::InterpolationGlyph::Stepped,
                        CurveData::Standard(Curve::Stepped) | CurveData::Ik(IkCurve::Stepped),
                    ) | (
                        curve_widgets::InterpolationGlyph::Bezier,
                        CurveData::Standard(Curve::Bezier(_) | Curve::BezierOffset(_))
                            | CurveData::Ik(IkCurve::Bezier { .. }),
                    )
                )
            });
            let response = curve_widgets::interpolation_button(ui, glyph, selected, enabled);
            self.record_target(&format!("{prefix}:{glyph:?}"), response.rect);
            if response.clicked() {
                self.apply_selected_curve_preset(preset);
            }
        }
    }
    pub(super) fn selected_curve_channels(&self) -> Result<Vec<CurveChannel>, String> {
        let targets = curve_targets(
            self.session.document(),
            self.animation,
            self.key_selection.keys(),
        )?;
        let mut shown: Vec<_> = targets.iter().take(256).cloned().collect();
        if let Some((key, _)) = self.curves.primary.as_ref()
            && let Some(primary) = targets.iter().find(|target| target.key == *key)
            && !shown.contains(primary)
        {
            shown.push(primary.clone());
        }
        if shown.is_empty() {
            return Ok(Vec::new());
        }
        curve_channels_for_targets(self.session.document(), self.animation, &shown)
    }

    pub(super) fn curves_panel(&mut self, ui: &mut egui::Ui) {
        self.record_target("curvesView", ui.clip_rect());
        ui.horizontal(|ui| {
            icons::heading(ui, icons::Icon::Curves, "Curves");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let close =
                    icons::toolbar_button(ui, icons::Icon::Delete, "Close Curves view", "", true);
                self.record_target("closeView:Curves", close.rect);
                if close.clicked() {
                    self.cancel_gesture();
                    self.view_tabs.open.remove(&views::DockView::Curves);
                }
            });
        });
        ui.separator();
        let channels = match self.selected_curve_channels() {
            Ok(channels) => channels,
            Err(error) => {
                self.curves.error = Some(error);
                Vec::new()
            }
        };
        if self.curves.selection.as_slice() != self.key_selection.keys()
            || !channels.iter().any(|channel| {
                self.curves.primary.as_ref() == Some(&(channel.target.key.clone(), channel.channel))
            })
        {
            self.curves.primary = channels
                .first()
                .map(|channel| (channel.target.key.clone(), channel.channel));
            self.curves.selection = self.key_selection.keys().to_vec();
            self.curves.error = None;
        }
        ui.horizontal(|ui| {
            self.curve_preset_controls(ui, "curvesInterpolation");
            let matched = ui
                .selectable_label(self.curves.match_handles, "Match")
                .on_hover_text("Apply handle changes to selected keys.");
            self.record_target("curvesMatch", matched.rect);
            if matched.clicked() {
                self.cancel_gesture();
                self.curves.match_handles = !self.curves.match_handles;
            }
            let separate = ui
                .selectable_label(self.curves.separate, "Separate")
                .on_hover_text("Allow a cusp by adjusting handles independently through a key.");
            self.record_target("curvesSeparate", separate.rect);
            if separate.clicked() {
                self.cancel_gesture();
                self.curves.separate = !self.curves.separate;
            }
        });
        let height = ui.available_height().max(70.);
        let (_, canvas) = ui.allocate_space(vec2(ui.available_width(), height));
        let response = ui.interact(canvas, Self::curves_id(), Sense::click_and_drag());
        if response.clicked() {
            response.request_focus();
        }
        self.record_target("curvesCanvas", canvas);
        let plot = egui::Rect::from_min_max(canvas.min, canvas.max - vec2(96., 0.));
        let presets = egui::Rect::from_min_max(pos2(plot.right() + 5., canvas.top()), canvas.max);
        self.record_target("curvesPlot", plot);
        self.record_target("curvesPresets", presets);
        let painter = ui.painter_at(canvas);
        painter.rect_filled(plot, 3., Color32::from_rgb(28, 29, 31));
        painter.rect_filled(presets, 3., Color32::from_gray(48));
        let area = plot.shrink2(vec2(12., 15.));
        let mut shown: Vec<_> = channels.iter().take(256).collect();
        if let Some(primary) = channels.iter().find(|channel| {
            self.curves.primary.as_ref() == Some(&(channel.target.key.clone(), channel.channel))
        }) && !shown.contains(&primary)
        {
            shown.push(primary);
        }
        let bounds = self
            .active_curve_bounds(curve_gesture::CurveInputOwner::Curves)
            .unwrap_or_else(|| fitted_bounds(&shown));
        self.curves.bounds = Some(bounds);
        let point = |rect, x, y| relative_point(rect, bounds, x, y);
        ui.scope_builder(
            egui::UiBuilder::new().max_rect(egui::Rect::from_min_max(
                pos2(presets.left() + 3., presets.bottom() - 27.),
                presets.max,
            )),
            |ui| {
                ui.spacing_mut().item_spacing.x = 3.;
                ui.horizontal(|ui| {
                    for (index, (icon, name)) in [
                        (icons::Icon::Duplicate, "Add curve preset"),
                        (icons::Icon::RestoreWorkspace, "Update curve preset"),
                        (icons::Icon::Delete, "Remove curve preset"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let response = icons::toolbar_button(ui, icon, name, "", false)
                            .on_disabled_hover_text(format!(
                                "{name}\nPreset management is not available."
                            ));
                        self.record_target(&format!("curvesPresetControl:{index}"), response.rect);
                    }
                });
            },
        );
        if channels
            .iter()
            .any(|channel| channel.values[0] != channel.values[1])
        {
            for n in 0..=10 {
                let f = n as f64 / 10.;
                painter.line_segment(
                    [point(area, f, bounds[2]), point(area, f, bounds[3])],
                    Stroke::new(1., Color32::from_gray(if n % 5 == 0 { 68 } else { 40 })),
                );
                painter.line_segment(
                    [
                        point(area, 0., bounds[2] + f * (bounds[3] - bounds[2])),
                        point(area, 1., bounds[2] + f * (bounds[3] - bounds[2])),
                    ],
                    Stroke::new(1., Color32::from_gray(if n % 5 == 0 { 68 } else { 40 })),
                );
            }
            for n in 0..12 {
                if n % 2 == 0 {
                    painter.line_segment(
                        [
                            point(area, n as f64 / 12., n as f64 / 12.),
                            point(area, (n + 1) as f64 / 12., (n + 1) as f64 / 12.),
                        ],
                        Stroke::new(1., MUTED),
                    );
                }
            }
        }
        let press = ui.input(|input| key_drag::primary_pointer_event(&input.events, true));
        if !self.shared_curve_edit_active()
            && !self.curve_gesture_blocked(ui.ctx())
            && self.slot_authoring_input_available(ui.ctx())
            && ui.is_enabled()
            && let Some((pointer, _)) = press
            && plot.contains(pointer)
            && ui.clip_rect().contains(pointer)
            && ui.rect_contains_pointer(plot)
        {
            let current_handle_hit = shown.iter().any(|channel| {
                self.curves.primary.as_ref() == Some(&(channel.target.key.clone(), channel.channel))
                    && relative_handles(channel).is_some_and(|h| {
                        [point(area, h[0], h[1]), point(area, h[2], h[3])]
                            .into_iter()
                            .any(|p| p.is_finite() && area.contains(p) && p.distance(pointer) <= 9.)
                    })
            });
            if !current_handle_hit {
                let closest = shown
                    .iter()
                    .map(|channel| {
                        let mut distance =
                            path_distance(pointer, &relative_path(channel, area, bounds));
                        if let Some(h) = relative_handles(channel) {
                            for p in [point(area, h[0], h[1]), point(area, h[2], h[3])] {
                                if p.is_finite() && area.contains(p) {
                                    distance = distance.min(p.distance(pointer));
                                }
                            }
                        }
                        (*channel, distance)
                    })
                    .filter(|(_, distance)| *distance <= 8.)
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                if let Some((channel, _)) = closest {
                    self.curves.primary = Some((channel.target.key.clone(), channel.channel));
                    response.request_focus();
                }
            }
        }
        // Keep stable target identities while painting the emphasized path last.
        let mut drawing: Vec<_> = shown.into_iter().enumerate().collect();
        drawing.sort_by_key(|(_, channel)| {
            self.curves.primary.as_ref() == Some(&(channel.target.key.clone(), channel.channel))
        });
        for (index, channel) in drawing {
            let primary = self.curves.primary.as_ref()
                == Some(&(channel.target.key.clone(), channel.channel));
            let color = if primary {
                channel_color(channel)
            } else {
                MUTED
            };
            let points = relative_path(channel, area, bounds);
            let clipped = ui.painter_at(plot);
            if !points.is_empty() && points.iter().all(|p| p.is_finite()) {
                let midpoint = points[points.len() / 2];
                if plot.contains(midpoint) && ui.clip_rect().contains(midpoint) {
                    self.record_target(
                        &format!("curvesChannel:{index}"),
                        egui::Rect::from_center_size(midpoint, vec2(10., 10.)),
                    );
                }
                clipped.add(egui::Shape::line(
                    points,
                    Stroke::new(if primary { 1.7 } else { 1. }, color),
                ));
            }
            let Some(h) = relative_handles(channel) else {
                continue;
            };
            let controls = [
                point(area, 0., 0.),
                point(area, h[0], h[1]),
                point(area, h[2], h[3]),
                point(area, 1., 1.),
            ];
            for i in 0..2 {
                let anchor = controls[if i == 0 { 0 } else { 3 }];
                if anchor.is_finite() && controls[i + 1].is_finite() {
                    clipped.line_segment([anchor, controls[i + 1]], Stroke::new(1., color));
                }
                if controls[i + 1].is_finite() && plot.contains(controls[i + 1]) {
                    clipped.circle_filled(controls[i + 1], 3., color);
                }
            }
        }
        if let Some(primary) = channels.iter().find(|channel| {
            self.curves.primary.as_ref() == Some(&(channel.target.key.clone(), channel.channel))
        }) {
            self.curve_handles_on_surface(
                ui,
                primary,
                curve_controller::CurveSurface {
                    owner: curve_gesture::CurveInputOwner::Curves,
                    rect: area,
                    bounds,
                    normalized: true,
                    color: channel_color(primary),
                },
            );
        }
        if let Some(error) = self.curves.error.as_ref() {
            painter.text(
                canvas.center(),
                egui::Align2::CENTER_CENTER,
                error,
                FontId::proportional(11.),
                ACCENT,
            );
        }
    }

    pub(super) fn curves_diagnostic(&self) -> serde_json::Value {
        serde_json::json!({"visible":self.curves_visible(),"primary":self.curves.primary,"bounds":self.curves.bounds,"error":self.curves.error,"match":self.curves.match_handles,"separate":self.curves.separate,"presetManagementEnabled":false,"handles":self.curve_handle_diagnostic()})
    }
}
