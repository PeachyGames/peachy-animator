//! Mesh curves describe progress between shapes, independently of their vertex offsets.
use super::curve_gesture::CurveInputOwner;
use super::*;
use peachy_engine::{AnimationKey, KeyOwner};

#[derive(Clone, Copy)]
struct CurveKey {
    time: f32,
    curve: Curve,
}
struct CurveTrack {
    keys: Vec<CurveKey>,
}
impl From<&DeformTrack> for CurveTrack {
    fn from(track: &DeformTrack) -> Self {
        Self {
            keys: track
                .keys
                .iter()
                .map(|key| CurveKey {
                    time: key.time,
                    curve: key.curve,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct CurveContext {
    animation: usize,
    attachment: AttachmentRef,
    time: f32,
}

#[derive(Default)]
pub(super) struct DeformCurveView {
    blocked: bool,
    error: Option<String>,
    context: Option<CurveContext>,
    owner: Option<(usize, AttachmentRef)>,
    bounds: Option<[f64; 4]>,
    rendered_points: usize,
}

fn curve_handles(curve: Curve) -> Option<[f32; 4]> {
    match curve {
        Curve::Bezier(handles) | Curve::BezierOffset(handles) => Some(handles),
        _ => None,
    }
}

fn valid_curve_geometry(rect: egui::Rect, bounds: [f64; 4]) -> bool {
    rect.is_finite()
        && rect.width() > 0.
        && rect.height() > 0.
        && bounds.into_iter().all(f64::is_finite)
        && bounds[1] > bounds[0]
        && bounds[3] > bounds[2]
}

fn key_zoom_bounds(time: f32, next: Option<f32>, fps: u32) -> [f64; 4] {
    let start = f64::from(time);
    let grain = if time.next_up().is_finite() {
        f64::from(time.next_up()) - start
    } else {
        start - f64::from(time.next_down())
    };
    let span = next
        .map(|end| f64::from(end) - start)
        .filter(|span| span.is_finite() && *span > 0.)
        .unwrap_or_else(|| (1. / f64::from(fps)).max(grain));
    [
        (start - span * 0.1).max(0.),
        (start + span * 1.1).min(f64::from(f32::MAX)),
        -0.12,
        1.12,
    ]
}

fn curve_point(rect: egui::Rect, bounds: [f64; 4], point: [f64; 2]) -> Pos2 {
    pos2(
        (f64::from(rect.left())
            + (point[0] - bounds[0]) / (bounds[1] - bounds[0]) * f64::from(rect.width()))
            as f32,
        (f64::from(rect.bottom())
            - (point[1] - bounds[2]) / (bounds[3] - bounds[2]) * f64::from(rect.height()))
            as f32,
    )
}

impl Editor {
    fn deform_curve_input_available(&self, ctx: &egui::Context) -> bool {
        !self.setup_mode
            && ctx.input(|input| input.focused)
            && self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
            && self.export_dialog.is_none()
            && !egui::Popup::is_any_open(ctx)
    }

    fn deform_curve_context(&self, attachment: &AttachmentRef) -> Option<CurveContext> {
        if self.setup_mode {
            return None;
        }
        let KeyOwner::Deform { attachment } =
            peachy_engine::deform_key_owner(self.session.document(), attachment).ok()?
        else {
            return None;
        };
        if self.graph_owner()
            != Some(KeyOwner::Deform {
                attachment: attachment.clone(),
            })
        {
            return None;
        }
        let track = self
            .session
            .document()
            .animations
            .get(self.animation)?
            .deform_tracks
            .iter()
            .find(|track| track.attachment == attachment)?;
        let selected = self.primary_graph_key().and_then(|key| match key {
            AnimationKey::Deform {
                attachment: owner,
                time,
            } if owner == attachment && track.keys.iter().any(|key| key.time == time) => Some(time),
            _ => None,
        });
        let time = selected.or_else(|| {
            track
                .keys
                .iter()
                .find(|key| key.time == self.time)
                .map(|key| key.time)
        })?;
        let fallback = vec![AnimationKey::Deform {
            attachment: attachment.clone(),
            time,
        }];
        let selection = if self.key_selection.is_empty() {
            fallback.as_slice()
        } else {
            self.key_selection.keys()
        };
        let time = peachy_engine::curve_targets(self.session.document(), self.animation, selection)
            .ok()?
            .into_iter()
            .find_map(|target| match target.key {
                AnimationKey::Deform {
                    attachment: owner,
                    time,
                } if owner == attachment
                    && (time == selected.unwrap_or(self.time) || selection.len() == 1) =>
                {
                    Some(time)
                }
                _ => None,
            })?;
        Some(CurveContext {
            animation: self.animation,
            attachment,
            time,
        })
    }

    fn deform_curve_key(&self, context: &CurveContext) -> Option<&DeformKey> {
        self.session
            .document()
            .animations
            .get(context.animation)?
            .deform_tracks
            .iter()
            .find(|track| track.attachment == context.attachment)?
            .keys
            .iter()
            .find(|key| key.time == context.time)
    }

    pub(super) fn deform_graph_has_focus(&self, ctx: &egui::Context) -> bool {
        self.graph_visible()
            && self.deform_curve_input_available(ctx)
            && matches!(self.graph_owner(), Some(KeyOwner::Deform { .. }))
            && !ctx.text_edit_focused()
            && ctx.memory(|memory| memory.has_focus(Self::graph_id()))
    }

    pub(super) fn deform_curve_edit_active(&self) -> bool {
        self.shared_curve_owner() == Some(CurveInputOwner::DeformGraph)
    }

    pub(super) fn cancel_deform_curve_edit(&mut self) {
        if self.deform_curve_edit_active() {
            self.cancel_shared_curve_edit();
            self.graph.deform.blocked = true;
        }
    }

    pub(super) fn reconcile_deform_curve_input(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.pointer.primary_down()) {
            self.graph.deform.blocked = false;
        }
    }

    #[cfg(test)]
    pub(super) fn set_deform_key_curve(
        &mut self,
        attachment: AttachmentRef,
        time: f32,
        curve: Curve,
    ) {
        self.cancel_gesture();
        if self.setup_mode
            || self.error.is_some()
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
            || !curve.handles_valid()
        {
            return;
        }
        let Ok(owner @ KeyOwner::Deform { .. }) =
            peachy_engine::deform_key_owner(self.session.document(), &attachment)
        else {
            return;
        };
        if self.graph_owner() != Some(owner.clone()) {
            return;
        }
        let KeyOwner::Deform { attachment } = owner else {
            return;
        };
        let context = CurveContext {
            animation: self.animation,
            attachment: attachment.clone(),
            time,
        };
        if self
            .deform_curve_key(&context)
            .is_some_and(|key| key.curve != curve)
        {
            self.playing = false;
            self.dispatch(Command::SetDeformCurve {
                animation: self.animation,
                attachment,
                time,
                curve,
            });
        }
    }

    fn deform_curve_presets(
        &mut self,
        ui: &mut egui::Ui,
        context: &CurveContext,
        curve: Curve,
        prefix: &str,
    ) {
        let mut chosen = None;
        let enabled = self.deform_curve_input_available(ui.ctx());
        ui.horizontal(|ui| {
            for (label, value, selected) in [
                (
                    "Linear",
                    peachy_engine::CurvePreset::Linear,
                    curve == Curve::Linear,
                ),
                (
                    "Stepped",
                    peachy_engine::CurvePreset::Stepped,
                    curve == Curve::Stepped,
                ),
                (
                    "Bezier",
                    peachy_engine::CurvePreset::Bezier,
                    curve_handles(curve).is_some(),
                ),
            ] {
                let response = ui.add_enabled(enabled, egui::Button::selectable(selected, label));
                self.record_target(&format!("{prefix}:{label}"), response.rect);
                if response.clicked() {
                    chosen = Some(value);
                }
            }
        });
        if let Some(preset) = chosen {
            let selection = if self.key_selection.is_empty() {
                vec![peachy_engine::CurveSelection {
                    key: AnimationKey::Deform {
                        attachment: context.attachment.clone(),
                        time: context.time,
                    },
                    channel: 0,
                }]
            } else {
                self.curve_preset_selection()
            };
            self.apply_curve_preset_request(selection, preset);
        }
    }

    pub(super) fn deform_curve_properties(
        &mut self,
        ui: &mut egui::Ui,
        attachment: &AttachmentRef,
    ) {
        let Some(context) = self.deform_curve_context(attachment) else {
            return;
        };
        let Some(curve) = self.deform_curve_key(&context).map(|key| key.curve) else {
            return;
        };
        ui.label(RichText::new("Key interpolation").small().color(MUTED));
        self.deform_curve_presets(ui, &context, curve, "meshDeformCurve");
        let open = ui
            .small_button("Graph")
            .on_hover_text("Edit progress between this mesh pose and the next pose.");
        self.record_target("meshDeformGraph", open.rect);
        if open.clicked() {
            self.open_graph_view();
        }
    }

    fn deform_curve_numbers(&mut self, ui: &mut egui::Ui, context: &CurveContext, _curve: Curve) {
        let next = self.session.document().animations[context.animation]
            .deform_tracks
            .iter()
            .find(|track| track.attachment == context.attachment)
            .and_then(|track| {
                let index = track.keys.partition_point(|key| key.time <= context.time);
                track.keys.get(index).map(|key| key.time)
            });
        if let Some(next_time) = next
            && let Some(channel) = self.graph_curve_channel(
                AnimationKey::Deform {
                    attachment: context.attachment.clone(),
                    time: context.time,
                },
                next_time,
                0,
            )
        {
            self.graph_curve_numeric_controls(ui, CurveInputOwner::DeformGraph, &channel);
        }
    }

    pub(super) fn deform_graph_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let Some(KeyOwner::Deform { attachment }) = self.graph_owner() else {
            return false;
        };
        if self.setup_mode {
            return false;
        }
        let owner = (self.animation, attachment.clone());
        if self.graph.deform.owner.as_ref() != Some(&owner) {
            self.graph.deform.owner = Some(owner);
            self.graph.deform.bounds = None;
            self.curves.draft = None;
            self.graph.deform.context = None;
        }
        let Some(track) = self
            .session
            .document()
            .animations
            .get(self.animation)
            .and_then(|clip| {
                clip.deform_tracks
                    .iter()
                    .find(|track| track.attachment == attachment)
            })
            .map(CurveTrack::from)
        else {
            let enabled = self.deform_curve_input_available(ui.ctx());
            ui.add_enabled_ui(enabled, |ui| self.animation_controls(ui));
            ui.label("This mesh has no deform keys.");
            return true;
        };
        let current = self.deform_curve_context(&attachment);
        let selected = current.as_ref().map(|context| context.time);
        let mut choose = None;
        let mut targets = Vec::new();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Deform").strong());
            let picker = egui::ComboBox::from_id_salt("graph-deform-key")
                .selected_text(selected.map_or_else(
                    || "Choose exact key".to_owned(),
                    |time| {
                        format!(
                            "Key at {} frames",
                            time * self.session.document().fps as f32
                        )
                    },
                ))
                .show_ui(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(180.).show_rows(
                        ui,
                        22.,
                        track.keys.len(),
                        |ui, rows| {
                            for index in rows {
                                let key = &track.keys[index];
                                let response = ui.selectable_label(
                                    selected == Some(key.time),
                                    format!(
                                        "{} frames · {} s",
                                        key.time * self.session.document().fps as f32,
                                        key.time
                                    ),
                                );
                                targets.push((format!("graphDeformKey:{index}"), response.rect));
                                if response.clicked() {
                                    choose = Some(key.time);
                                }
                            }
                        },
                    );
                });
            targets.push(("graphDeformKeyPicker".to_owned(), picker.response.rect));
            let fit = ui.small_button("Fit");
            targets.push(("graphFit".to_owned(), fit.rect));
            if fit.clicked() {
                self.cancel_gesture();
                self.graph.deform.bounds = None;
            }
            let zoom = ui.add_enabled(selected.is_some(), egui::Button::new("Zoom Keys").small());
            targets.push(("graphZoomKeys".to_owned(), zoom.rect));
            if zoom.clicked()
                && let Some(time) = selected
            {
                self.cancel_gesture();
                let next = track
                    .keys
                    .iter()
                    .find(|key| key.time > time)
                    .map(|key| key.time);
                self.graph.deform.bounds =
                    Some(key_zoom_bounds(time, next, self.session.document().fps));
            }
            if let Some(context) = &current
                && let Some(curve) = self.deform_curve_key(context).map(|key| key.curve)
            {
                self.deform_curve_presets(ui, context, curve, "graphDeformCurve");
                if curve_handles(curve).is_some() {
                    let handles = egui::CollapsingHeader::new("Handles")
                        .id_salt("deform-handle-controls")
                        .default_open(false)
                        .show(ui, |ui| self.deform_curve_numbers(ui, context, curve));
                    targets.push((
                        "graphDeformHandles".to_owned(),
                        handles.header_response.rect,
                    ));
                }
            }
        });
        for (name, rect) in targets {
            self.record_target(&name, rect);
        }
        if let Some(time) = choose {
            self.cancel_gesture();
            self.set_time(time);
            self.key_selection.replace(vec![AnimationKey::Deform {
                attachment: attachment.clone(),
                time,
            }]);
        }
        let enabled = self.deform_curve_input_available(ui.ctx());
        ui.add_enabled_ui(enabled, |ui| self.animation_controls(ui));
        if self.animation != self.graph.deform.owner.as_ref().unwrap().0 {
            return true;
        }
        let Some(track) = self.session.document().animations[self.animation]
            .deform_tracks
            .iter()
            .find(|track| track.attachment == attachment)
            .map(CurveTrack::from)
        else {
            self.graph.deform.context = None;
            self.curves.draft = None;
            self.graph.deform.bounds = None;
            ui.label("This mesh has no deform keys.");
            return true;
        };
        let context = self.deform_curve_context(&attachment);
        let selected = context.as_ref().map(|context| context.time);
        self.graph.deform.context = context.clone();
        if self.graph.deform.bounds.is_none() {
            let end =
                f64::from(self.animation_end()).max(1. / f64::from(self.session.document().fps));
            let mut bounds = [
                -end * 0.04,
                (end * 1.1).min(f64::from(f32::MAX)),
                -0.12,
                1.12,
            ];
            for pair in track.keys.windows(2) {
                if let Some(handles) = curve_handles(pair[0].curve) {
                    let duration = f64::from(pair[1].time) - f64::from(pair[0].time);
                    for index in 0..2 {
                        let time =
                            f64::from(pair[0].time) + f64::from(handles[index * 2]) * duration;
                        bounds[0] = bounds[0].min(time);
                        bounds[1] = bounds[1].max(time);
                        bounds[2] = bounds[2].min(f64::from(handles[index * 2 + 1]) - 0.1);
                        bounds[3] = bounds[3].max(f64::from(handles[index * 2 + 1]) + 0.1);
                    }
                }
            }
            self.graph.deform.bounds = Some(bounds);
        }
        let bounds = self.graph.deform.bounds.unwrap();
        // The clip rect includes the panel's frame margin. Use its content
        // bounds so filling the canvas cannot grow the panel on every frame.
        let footer = if self.graph.deform.error.is_some()
            || self.curves.error.is_some()
            || track.keys.len() > 257
        {
            ui.text_style_height(&egui::TextStyle::Small) + ui.spacing().item_spacing.y
        } else {
            0.
        };
        let height = (ui.available_height() - footer).max(110.);
        let (_, canvas_rect) = ui.allocate_space(vec2(ui.available_width(), height));
        self.record_target("graphDeformCanvas", canvas_rect);
        self.record_target("graphCanvas", canvas_rect);
        self.record_target("graphClip", ui.clip_rect());
        let rect = egui::Rect::from_min_max(
            canvas_rect.min + vec2(244., 24.),
            canvas_rect.max - vec2(45., 10.),
        );
        if !valid_curve_geometry(rect, bounds) {
            self.cancel_deform_curve_edit();
            self.graph.deform.error =
                Some("Graph interval cannot be displayed as finite coordinates.".into());
            ui.painter_at(canvas_rect).text(
                canvas_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Graph interval cannot be displayed as finite coordinates.",
                FontId::proportional(11.),
                ACCENT,
            );
            return true;
        }
        self.record_target("graphPlot", rect);
        let response = ui.interact(canvas_rect, Self::graph_id(), Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Deform progress graph")
        });
        let painter = ui.painter_at(canvas_rect);
        painter.rect_filled(canvas_rect, 4., Color32::from_rgb(21, 26, 33));
        let name = self
            .session
            .document()
            .attachment(&attachment)
            .and_then(|value| value.name().or_else(|| value.path()))
            .unwrap_or(&attachment.placeholder)
            .to_owned();
        let bone = self
            .session
            .document()
            .slots
            .iter()
            .find(|slot| slot.name == attachment.slot)
            .and_then(|slot| self.session.document().bones.get(slot.bone))
            .map_or(attachment.slot.clone(), |bone| bone.name.clone());
        let label_rect = egui::Rect::from_min_max(
            canvas_rect.min,
            pos2(rect.left() - 5., canvas_rect.bottom()),
        );
        painter.rect_filled(label_rect, 0., Color32::from_gray(53));
        painter.text(
            pos2(label_rect.left() + 12., label_rect.top() + 36.),
            egui::Align2::LEFT_CENTER,
            bone,
            FontId::proportional(12.),
            MINT,
        );
        let row = egui::Rect::from_min_max(
            pos2(label_rect.left() + 5., label_rect.top() + 48.),
            pos2(label_rect.right() - 3., label_rect.top() + 70.),
        );
        self.record_target("graphDeformRow", row);
        ui.painter_at(row).text(
            pos2(row.left() + 16., row.center().y),
            egui::Align2::LEFT_CENTER,
            format!("Deform: {name}"),
            FontId::proportional(12.),
            Color32::from_rgb(45, 190, 225),
        );
        let plot_painter = ui.painter_at(rect);
        for value in [0., 0.25, 0.5, 0.75, 1.] {
            let y = curve_point(rect, bounds, [0., value]).y;
            if !rect.y_range().contains(y) {
                continue;
            }
            plot_painter.line_segment(
                [pos2(rect.left(), y), pos2(rect.right(), y)],
                Stroke::new(1., Color32::from_rgb(43, 49, 58)),
            );
            painter.text(
                pos2(rect.right() + 6., y),
                egui::Align2::LEFT_CENTER,
                format!("{:.0}", value * 100.),
                FontId::monospace(10.),
                MUTED,
            );
        }
        let fps = f64::from(self.session.document().fps);
        let desired = ((bounds[1] - bounds[0]) * fps
            / (f64::from(rect.width()) / 65.).clamp(1., 64.))
        .max(1.);
        let decade = 10f64.powf(desired.log10().floor());
        let step = [1., 2., 5., 10.]
            .into_iter()
            .map(|multiple| multiple * decade)
            .find(|step| *step >= desired)
            .unwrap();
        let first = (bounds[0] * fps / step).ceil() * step;
        for index in 0..128 {
            let frame = first + index as f64 * step;
            let time = frame / fps;
            if time > bounds[1] {
                break;
            }
            let x = curve_point(rect, bounds, [time, 0.]).x;
            if !x.is_finite() {
                continue;
            }
            plot_painter.line_segment(
                [pos2(x, rect.top()), pos2(x, rect.bottom())],
                Stroke::new(1., Color32::from_rgb(43, 49, 58)),
            );
            painter.text(
                pos2(x, canvas_rect.top() + 11.),
                egui::Align2::CENTER_CENTER,
                format!("{frame:.0}"),
                FontId::monospace(10.),
                MUTED,
            );
        }
        let color = Color32::from_rgb(45, 190, 225);
        let visible: Vec<_> = track
            .keys
            .windows(2)
            .enumerate()
            .filter(|(_, pair)| {
                f64::from(pair[1].time) >= bounds[0] && f64::from(pair[0].time) <= bounds[1]
            })
            .map(|(index, _)| index)
            .collect();
        let mut indices: Vec<_> = if visible.len() <= 256 {
            visible.clone()
        } else {
            (0..256)
                .map(|index| visible[index * (visible.len() - 1) / 255])
                .collect()
        };
        if let Some(context) = &context
            && let Some(index) = track.keys.iter().position(|key| key.time == context.time)
            && index + 1 < track.keys.len()
            && !indices.contains(&index)
        {
            indices.push(index);
        }
        let mut rendered = 0;
        for &index in &indices {
            let from = &track.keys[index];
            let to = &track.keys[index + 1];
            let points: Vec<_> = (0..=40)
                .map(|part| {
                    let progress = part as f32 / 40.;
                    let time = f64::from(from.time)
                        + (f64::from(to.time) - f64::from(from.time)) * f64::from(progress);
                    curve_point(
                        rect,
                        bounds,
                        [
                            time,
                            f64::from(peachy_engine::curve_progress(from.curve, progress)),
                        ],
                    )
                })
                .collect();
            rendered += points.len();
            if points.iter().all(|point| point.is_finite()) {
                if from.curve == Curve::Stepped {
                    for (index, pair) in points.windows(2).enumerate() {
                        if index % 2 == 0 {
                            plot_painter.line_segment([pair[0], pair[1]], Stroke::new(1.5, color));
                        }
                    }
                } else {
                    plot_painter.add(egui::Shape::line(points, Stroke::new(1.7, color)));
                }
            }
            let top = curve_point(rect, bounds, [f64::from(to.time), 1.]);
            let bottom = curve_point(rect, bounds, [f64::from(to.time), 0.]);
            for part in 0..12 {
                if part % 2 == 0 {
                    plot_painter.line_segment(
                        [
                            top.lerp(bottom, part as f32 / 12.),
                            top.lerp(bottom, (part + 1) as f32 / 12.),
                        ],
                        Stroke::new(1., color),
                    );
                }
            }
        }
        self.graph.deform.rendered_points = rendered;
        let mut key_positions = Vec::new();
        let mut marker_indices: BTreeSet<_> = indices
            .iter()
            .flat_map(|index| [*index, *index + 1])
            .collect();
        if let Some(context) = &context
            && let Some(index) = track.keys.iter().position(|key| key.time == context.time)
        {
            marker_indices.insert(index);
        }
        if !track.keys.is_empty() {
            marker_indices.insert(0);
            marker_indices.insert(track.keys.len() - 1);
        }
        for index in marker_indices {
            let key = &track.keys[index];
            if f64::from(key.time) < bounds[0] || f64::from(key.time) > bounds[1] {
                continue;
            }
            for progress in if index == 0 { vec![0.] } else { vec![0., 1.] } {
                let point = curve_point(rect, bounds, [f64::from(key.time), progress]);
                if !point.is_finite() || !rect.contains(point) {
                    continue;
                }
                let marker = egui::Rect::from_center_size(point, vec2(6., 6.));
                if selected == Some(key.time) {
                    plot_painter.rect_filled(marker, 0., color);
                } else {
                    plot_painter.rect_stroke(
                        marker,
                        0.,
                        Stroke::new(1., color),
                        egui::StrokeKind::Middle,
                    );
                }
                key_positions.push((key.time, point));
                if progress == 0. {
                    self.record_target(
                        &format!("graphDeformKeyPoint:{index}"),
                        egui::Rect::from_center_size(point, vec2(10., 10.)),
                    );
                }
            }
        }
        if let Some(context) = &context
            && let Some(index) = track.keys.iter().position(|key| key.time == context.time)
            && let Some(next) = track.keys.get(index + 1)
            && let Some(channel) = self.graph_curve_channel(
                AnimationKey::Deform {
                    attachment: context.attachment.clone(),
                    time: context.time,
                },
                next.time,
                0,
            )
        {
            self.curve_handles_on_surface(
                ui,
                &channel,
                curve_controller::CurveSurface {
                    owner: CurveInputOwner::DeformGraph,
                    rect,
                    bounds,
                    normalized: false,
                    color,
                },
            );
        }
        let playhead = curve_point(rect, bounds, [f64::from(self.time), 0.]).x;
        if playhead.is_finite() {
            plot_painter.line_segment(
                [pos2(playhead, rect.top()), pos2(playhead, rect.bottom())],
                Stroke::new(1., ACCENT),
            );
        }
        let press = ui.input(|input| super::key_drag::primary_pointer_event(&input.events, true));
        if !self.shared_curve_edit_active()
            && !self.graph.deform.blocked
            && !self.curve_gesture_blocked(ui.ctx())
            && self.deform_curve_input_available(ui.ctx())
            && ui.is_enabled()
            && let Some((pointer, modifiers)) = press
            && response.contains_pointer()
            && ui.clip_rect().contains(pointer)
        {
            response.request_focus();
            if rect.contains(pointer)
                && let Some((time, _)) = key_positions
                    .iter()
                    .filter(|(_, point)| point.distance(pointer) <= 9.)
                    .min_by(|(_, a), (_, b)| a.distance(pointer).total_cmp(&b.distance(pointer)))
            {
                self.cancel_gesture();
                self.set_time(*time);
                self.key_selection.select(
                    AnimationKey::Deform {
                        attachment: attachment.clone(),
                        time: *time,
                    },
                    modifiers.command || modifiers.ctrl,
                );
            }
        }
        if let Some(error) = self
            .graph
            .deform
            .error
            .as_ref()
            .or(self.curves.error.as_ref())
        {
            ui.colored_label(ACCENT, error);
        }
        if visible.len() > 256 {
            ui.label("Showing a bounded set of intervals. Choose an exact key, then Zoom Keys.");
        }
        true
    }

    pub(super) fn deform_curve_diagnostic(&self) -> serde_json::Value {
        serde_json::json!({"context": self.graph.deform.context.as_ref().map(|context|
            serde_json::json!({"animation":context.animation,"attachment":context.attachment,"time":context.time})),
            "bounds":self.graph.deform.bounds,"active":self.deform_curve_edit_active(),"blocked":self.graph.deform.blocked,
            "error":self.graph.deform.error.as_ref().or(self.curves.error.as_ref()),"renderedPoints":self.graph.deform.rendered_points})
    }
}

#[cfg(test)]
#[path = "deform_timeline_ui_tests.rs"]
mod deform_timeline_ui_tests;
