//! Slot curves are unmultiplied RGBA values, sharing the timeline's typed keys.
use super::*;
use peachy_engine::{AnimationKey, KeyOwner};

const INTERVAL_LIMIT: usize = 256;
const SAMPLES: usize = 24;
const CHANNEL_NAMES: [&str; 4] = ["R", "G", "B", "A"];
const CHANNEL_COLORS: [Color32; 4] = [
    Color32::from_rgb(244, 74, 72),
    Color32::from_rgb(133, 229, 88),
    Color32::from_rgb(35, 171, 248),
    Color32::from_rgb(219, 121, 231),
];

#[derive(Clone, Copy, Debug, PartialEq)]
struct SlotContext {
    animation: usize,
    slot: usize,
    time: f32,
    channel: usize,
}

struct SlotDraft {
    context: SlotContext,
    source: Curve,
    source_color: [u8; 4],
    next_color: [u8; 4],
    text: [String; 4],
    changed: bool,
}

struct SlotCurveEdit {
    context: SlotContext,
    source: ColorKey,
    next: ColorKey,
    curves: [Curve; 4],
    handle: usize,
    rect: egui::Rect,
    bounds: [f64; 4],
    cursor: f32,
    skin: String,
    tool: Tool,
    axes: Axes,
    auto_key: bool,
    pose_generation: u64,
    pose: peachy_engine::PoseEdits,
    finish_pending: bool,
}

#[derive(Default)]
pub(super) struct SlotCurveView {
    edit: Option<SlotCurveEdit>,
    draft: Option<SlotDraft>,
    blocked: bool,
    error: Option<String>,
    owner: Option<(usize, usize)>,
    context: Option<SlotContext>,
    channel: usize,
    bounds: Option<[f64; 4]>,
    rendered_points: usize,
    rendered_intervals: usize,
}

/// Only visible intervals and the exact selected interval are copied for drawing.
/// The exact picker reads its visible rows directly from the document.
struct SlotPlot {
    intervals: Vec<(usize, ColorKey, ColorKey)>,
    markers: Vec<(usize, ColorKey)>,
    selected: Option<(ColorKey, Option<ColorKey>)>,
    first: Option<ColorKey>,
    last: Option<ColorKey>,
    visible_intervals: usize,
}

fn channel_value(key: &ColorKey, channel: usize) -> f32 {
    f32::from(key.color[channel]) / 255.
}

fn handles(curve: Curve) -> Option<[f32; 4]> {
    match curve {
        Curve::Bezier(handles) | Curve::BezierOffset(handles) => Some(handles),
        _ => None,
    }
}

fn value_handles(curve: Curve, left: f32, right: f32) -> Option<[f64; 4]> {
    let h = handles(curve)?;
    let left = f64::from(left);
    let span = if matches!(curve, Curve::BezierOffset(_)) {
        1.
    } else {
        f64::from(right) - left
    };
    Some([
        f64::from(h[0]),
        left + f64::from(h[1]) * span,
        f64::from(h[2]),
        left + f64::from(h[3]) * span,
    ])
}

fn curve_with_value_handles(
    source: Curve,
    values: [f64; 4],
    left: f32,
    right: f32,
) -> Option<Curve> {
    if !values.into_iter().all(f64::is_finite) {
        return None;
    }
    let span = f64::from(right) - f64::from(left);
    // Equal endpoints still need independent value tangents for overshoot.
    let offset = matches!(source, Curve::BezierOffset(_)) || span == 0.;
    let divisor = if offset { 1. } else { span };
    let h = [
        values[0] as f32,
        ((values[1] - f64::from(left)) / divisor) as f32,
        values[2] as f32,
        ((values[3] - f64::from(left)) / divisor) as f32,
    ];
    h.into_iter().all(f32::is_finite).then_some(if offset {
        Curve::BezierOffset(h)
    } else {
        Curve::Bezier(h)
    })
}

fn changed_channel(mut curves: [Curve; 4], channel: usize, curve: Curve) -> [Curve; 4] {
    if curve == Curve::Stepped {
        return [Curve::Stepped; 4];
    }
    if curves == [Curve::Stepped; 4] {
        curves = [Curve::Linear; 4];
    }
    curves[channel] = curve;
    curves
}

fn valid_geometry(rect: egui::Rect, bounds: [f64; 4]) -> bool {
    rect.is_finite()
        && rect.width() > 0.
        && rect.height() > 0.
        && bounds.into_iter().all(f64::is_finite)
        && bounds[1] > bounds[0]
        && bounds[3] > bounds[2]
}

fn point(rect: egui::Rect, bounds: [f64; 4], time: f64, value: f64) -> Pos2 {
    pos2(
        (f64::from(rect.left())
            + (time - bounds[0]) / (bounds[1] - bounds[0]) * f64::from(rect.width()))
            as f32,
        (f64::from(rect.bottom())
            - (value - bounds[2]) / (bounds[3] - bounds[2]) * f64::from(rect.height()))
            as f32,
    )
}

fn zoom_bounds(time: f32, next: Option<f32>, fps: u32, values: [f64; 2]) -> [f64; 4] {
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
        values[0],
        values[1],
    ]
}

fn shown_intervals(
    track: &ColorTrack,
    bounds: [f64; 4],
    selected: Option<f32>,
) -> (Vec<usize>, usize) {
    let end = track.keys.len().saturating_sub(1);
    let first = track
        .keys
        .partition_point(|key| f64::from(key.time) < bounds[0])
        .saturating_sub(1)
        .min(end);
    let last = track
        .keys
        .partition_point(|key| f64::from(key.time) <= bounds[1])
        .min(end);
    let count = last.saturating_sub(first);
    let shown = count.min(INTERVAL_LIMIT);
    let mut indices: Vec<_> = (0..shown)
        .map(|index| {
            first
                + if shown <= 1 {
                    0
                } else {
                    index * (count - 1) / (shown - 1)
                }
        })
        .collect();
    if let Some(time) = selected
        && let Ok(index) = track.keys.binary_search_by(|key| key.time.total_cmp(&time))
        && index < end
        && !indices.contains(&index)
    {
        indices.push(index);
    }
    (indices, count)
}

impl Editor {
    fn slot_color_track(&self, animation: usize, slot: usize) -> Option<&ColorTrack> {
        self.session
            .document()
            .animations
            .get(animation)?
            .color_tracks
            .iter()
            .find(|track| track.slot == slot)
    }

    fn slot_curve_key(&self, context: SlotContext) -> Option<&ColorKey> {
        let track = self.slot_color_track(context.animation, context.slot)?;
        let index = track
            .keys
            .binary_search_by(|key| key.time.total_cmp(&context.time))
            .ok()?;
        track.keys.get(index)
    }

    fn slot_curve_context(&self, slot: usize) -> Option<SlotContext> {
        if self.setup_mode || self.timeline_owner() != Some(KeyOwner::Slot { slot }) {
            return None;
        }
        let track = self.slot_color_track(self.animation, slot)?;
        let mut selected = self
            .key_selection
            .keys()
            .iter()
            .filter_map(|key| match key {
                AnimationKey::SlotColor { slot: owner, time } if *owner == slot => Some(*time),
                _ => None,
            });
        let selected_time = selected.next().filter(|_| selected.next().is_none());
        let time = selected_time.or(Some(self.time))?;
        track
            .keys
            .binary_search_by(|key| key.time.total_cmp(&time))
            .ok()?;
        Some(SlotContext {
            animation: self.animation,
            slot,
            time,
            channel: self.graph.slot.channel,
        })
    }

    pub(super) fn slot_graph_has_focus(&self, ctx: &egui::Context) -> bool {
        self.graph_visible()
            && !self.setup_mode
            && self.slot_authoring_input_available(ctx)
            && matches!(self.timeline_owner(), Some(KeyOwner::Slot { .. }))
            && !ctx.text_edit_focused()
            && ctx.memory(|memory| memory.has_focus(Self::graph_id()))
    }

    pub(super) fn slot_curve_edit_active(&self) -> bool {
        self.graph.slot.edit.is_some()
    }

    pub(super) fn cancel_slot_curve_edit(&mut self) {
        if self.graph.slot.edit.take().is_some() {
            self.session.cancel_transaction();
            self.graph.slot.blocked = true;
        }
        self.graph.slot.draft = None;
    }

    pub(super) fn reconcile_slot_curve_input(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.pointer.primary_down()) {
            self.graph.slot.blocked = false;
        }
        if self.graph.slot.edit.as_ref().is_some_and(|edit| {
            !self.graph_visible()
                || self.setup_mode
                || !self.slot_authoring_input_available(ctx)
                || self.timeline_owner()
                    != Some(KeyOwner::Slot {
                        slot: edit.context.slot,
                    })
                || self.animation != edit.context.animation
                || self.graph.slot.channel != edit.context.channel
                || self.time.to_bits() != edit.cursor.to_bits()
                || self.skin != edit.skin
                || self.tool != edit.tool
                || self.axes != edit.axes
                || self.auto_key != edit.auto_key
                || self.pose_view_generation != edit.pose_generation
                || self.session.pose_edits() != &edit.pose
                || self
                    .slot_curve_key(edit.context)
                    .is_none_or(|key| key.color != edit.source.color || key.curves != edit.curves)
                || self
                    .slot_color_track(edit.context.animation, edit.context.slot)
                    .is_none_or(|track| {
                        let index = track
                            .keys
                            .partition_point(|key| key.time <= edit.context.time);
                        track.keys.get(index) != Some(&edit.next)
                    })
                || ctx.text_edit_focused()
        }) {
            self.cancel_slot_curve_edit();
        }
    }

    pub(super) fn settle_slot_curve_input(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.pointer.primary_down())
            && self
                .graph
                .slot
                .edit
                .as_ref()
                .is_some_and(|edit| edit.finish_pending)
        {
            self.graph.slot.edit = None;
            self.session.commit_transaction();
        }
    }

    pub(super) fn set_slot_key_curve(
        &mut self,
        slot: usize,
        time: f32,
        channel: usize,
        curve: Curve,
    ) {
        self.cancel_gesture();
        if self.setup_mode
            || channel >= 4
            || !curve.handles_valid()
            || self.error.is_some()
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
            || self.timeline_owner() != Some(KeyOwner::Slot { slot })
        {
            return;
        }
        let context = SlotContext {
            animation: self.animation,
            slot,
            time,
            channel,
        };
        let Some(source) = self.slot_curve_key(context) else {
            return;
        };
        let curves = changed_channel(source.curves, channel, curve);
        if curves != source.curves {
            self.playing = false;
            self.dispatch(Command::SetSlotColorCurves {
                animation: self.animation,
                slot,
                time,
                curves,
            });
        }
    }

    fn slot_curve_presets(&mut self, ui: &mut egui::Ui, context: SlotContext, curve: Curve) {
        let enabled = self.slot_authoring_input_available(ui.ctx());
        let mut choose = None;
        for (name, value, selected) in [
            ("Linear", Curve::Linear, curve == Curve::Linear),
            ("Stepped", Curve::Stepped, curve == Curve::Stepped),
            (
                "Bezier",
                Curve::Bezier([0.42, 0., 0.58, 1.]),
                handles(curve).is_some(),
            ),
        ] {
            let response = ui.add_enabled(enabled, egui::Button::selectable(selected, name));
            self.record_target(&format!("graphSlotCurve:{name}"), response.rect);
            let response = response.on_hover_text(if name == "Stepped" {
                "Stepped applies to every RGBA channel."
            } else if curve == Curve::Stepped {
                "Leave grouped Stepped: other channels become Linear."
            } else {
                "Change only the selected channel's interpolation."
            });
            if response.clicked() && !selected {
                choose = Some(value);
            }
        }
        if let Some(curve) = choose {
            self.set_slot_key_curve(context.slot, context.time, context.channel, curve);
        }
    }

    fn slot_curve_numbers(
        &mut self,
        ui: &mut egui::Ui,
        context: SlotContext,
        key: &ColorKey,
        next: &ColorKey,
    ) {
        let curve = key.curves[context.channel];
        let Some(values) = value_handles(
            curve,
            channel_value(key, context.channel),
            channel_value(next, context.channel),
        ) else {
            return;
        };
        let enabled = self.slot_authoring_input_available(ui.ctx());
        if !enabled {
            self.graph.slot.draft = None;
        }
        if self.graph.slot.draft.as_ref().is_none_or(|draft| {
            draft.context != context
                || draft.source != curve
                || draft.source_color != key.color
                || draft.next_color != next.color
        }) {
            self.graph.slot.draft = Some(SlotDraft {
                context,
                source: curve,
                source_color: key.color,
                next_color: next.color,
                text: std::array::from_fn(|index| {
                    (values[index] * if index % 2 == 0 { 1. } else { 100. }).to_string()
                }),
                changed: false,
            });
        }
        let mut apply = false;
        let mut cancel = false;
        let mut targets = Vec::new();
        if let Some(draft) = self.graph.slot.draft.as_mut() {
            egui::Grid::new("slot-curve-handles")
                .num_columns(2)
                .show(ui, |ui| {
                    for (index, label) in ["Time 1", "Value 1 (%)", "Time 2", "Value 2 (%)"]
                        .iter()
                        .enumerate()
                    {
                        ui.label(*label);
                        let response = ui.add_enabled(
                            enabled,
                            egui::TextEdit::singleline(&mut draft.text[index])
                                .id_salt(("slot-curve-number", index))
                                .event_filter(egui::EventFilter {
                                    horizontal_arrows: true,
                                    vertical_arrows: true,
                                    escape: true,
                                    ..Default::default()
                                })
                                .desired_width(90.),
                        );
                        targets.push((format!("graphSlotValue:{index}"), response.rect));
                        draft.changed |= response.changed();
                        // Retain text focus until this handler runs: egui's
                        // default Escape navigation would reach global deselect.
                        if response.has_focus()
                            && ui.input_mut(|input| {
                                input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                            })
                        {
                            cancel = true;
                            response.surrender_focus();
                        }
                        apply |= draft.changed && response.lost_focus();
                        ui.end_row();
                    }
                });
        }
        for (name, rect) in targets {
            self.record_target(&name, rect);
        }
        if cancel || !self.slot_authoring_input_available(ui.ctx()) {
            self.graph.slot.draft = None;
            return;
        }
        if apply {
            let parsed: Option<Vec<f64>> = self.graph.slot.draft.as_ref().and_then(|draft| {
                draft
                    .text
                    .iter()
                    .map(|text| text.parse::<f64>().ok().filter(|value| value.is_finite()))
                    .collect()
            });
            let curve = parsed.and_then(|values| {
                curve_with_value_handles(
                    curve,
                    [values[0], values[1] / 100., values[2], values[3] / 100.],
                    channel_value(key, context.channel),
                    channel_value(next, context.channel),
                )
            });
            if let Some(curve) = curve {
                self.graph.slot.error = None;
                self.set_slot_key_curve(context.slot, context.time, context.channel, curve);
            } else {
                self.graph.slot.error = Some("Bezier handles must be finite numbers.".into());
            }
        }
    }

    fn slot_fit_bounds(&self, slot: usize) -> [f64; 4] {
        let end = f64::from(self.animation_end()).max(1. / f64::from(self.session.document().fps));
        let mut bounds = [
            -end * 0.04,
            (end * 1.1).min(f64::from(f32::MAX)),
            -0.08,
            1.08,
        ];
        if let Some(track) = self.slot_color_track(self.animation, slot) {
            for pair in track.keys.windows(2) {
                let duration = f64::from(pair[1].time) - f64::from(pair[0].time);
                for channel in 0..4 {
                    if let Some(h) = value_handles(
                        pair[0].curves[channel],
                        channel_value(&pair[0], channel),
                        channel_value(&pair[1], channel),
                    ) {
                        for index in 0..2 {
                            let time = f64::from(pair[0].time) + h[index * 2] * duration;
                            bounds[0] = bounds[0].min(time);
                            bounds[1] = bounds[1].max(time);
                            bounds[2] = bounds[2].min(h[index * 2 + 1] - 0.08);
                            bounds[3] = bounds[3].max(h[index * 2 + 1] + 0.08);
                        }
                    }
                }
            }
        }
        bounds
    }

    fn slot_plot(
        &self,
        slot: usize,
        bounds: [f64; 4],
        context: Option<SlotContext>,
    ) -> Option<SlotPlot> {
        let track = self.slot_color_track(self.animation, slot)?;
        let selected_time = context.map(|context| context.time);
        let (indices, visible_intervals) = shown_intervals(track, bounds, selected_time);
        let mut marker_indices: BTreeSet<usize> = indices
            .iter()
            .flat_map(|index| [*index, *index + 1])
            .collect();
        let selected = selected_time
            .and_then(|time| {
                track
                    .keys
                    .binary_search_by(|key| key.time.total_cmp(&time))
                    .ok()
            })
            .map(|index| {
                marker_indices.insert(index);
                (
                    track.keys[index].clone(),
                    track.keys.get(index + 1).cloned(),
                )
            });
        if !track.keys.is_empty() {
            marker_indices.insert(0);
            marker_indices.insert(track.keys.len() - 1);
        }
        Some(SlotPlot {
            intervals: indices
                .into_iter()
                .map(|index| {
                    (
                        index,
                        track.keys[index].clone(),
                        track.keys[index + 1].clone(),
                    )
                })
                .collect(),
            markers: marker_indices
                .into_iter()
                .map(|index| (index, track.keys[index].clone()))
                .collect(),
            selected,
            first: track.keys.first().cloned(),
            last: track.keys.last().cloned(),
            visible_intervals,
        })
    }

    pub(super) fn slot_graph_panel(&mut self, ui: &mut egui::Ui) -> bool {
        let Some(KeyOwner::Slot { slot }) = self.timeline_owner() else {
            return false;
        };
        if self.setup_mode {
            return false;
        }
        self.reconcile_dopesheet_selection();
        let owner = (self.animation, slot);
        if self.graph.slot.owner != Some(owner) {
            self.graph.slot.owner = Some(owner);
            self.graph.slot.context = None;
            self.graph.slot.bounds = None;
            self.graph.slot.draft = None;
            self.graph.slot.error = None;
        }
        self.graph.slot.rendered_points = 0;
        self.graph.slot.rendered_intervals = 0;
        let current = self.slot_curve_context(slot);
        let selected_time = current.map(|context| context.time);
        let mut choose = None;
        let mut channel = self.graph.slot.channel;
        let mut targets = Vec::new();
        let enabled = self.slot_authoring_input_available(ui.ctx());
        // The picker's own popup must remain interactive. Modal/focus guards
        // still disable navigation, which may discard an unkeyed pose.
        let picker_enabled = ui.ctx().input(|input| input.focused)
            && self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
            && !self.slot_color_dialog_open();
        ui.horizontal(|ui| {
            ui.label(RichText::new("RGBA").strong());
            let picker = ui
                .add_enabled_ui(picker_enabled, |ui| {
                    egui::ComboBox::from_id_salt("graph-slot-key")
                        .selected_text(selected_time.map_or_else(
                            || "Choose exact key".into(),
                            |time| {
                                format!(
                                    "Key at {} frames",
                                    time * self.session.document().fps as f32
                                )
                            },
                        ))
                        .show_ui(ui, |ui| {
                            if let Some(track) = self.slot_color_track(self.animation, slot) {
                                egui::ScrollArea::vertical().max_height(180.).show_rows(
                                    ui,
                                    22.,
                                    track.keys.len(),
                                    |ui, rows| {
                                        for index in rows {
                                            let key = &track.keys[index];
                                            let response = ui.selectable_label(
                                                selected_time == Some(key.time),
                                                format!(
                                                    "{} frames · {} s",
                                                    key.time * self.session.document().fps as f32,
                                                    key.time
                                                ),
                                            );
                                            targets.push((
                                                format!("graphSlotKey:{index}"),
                                                response.rect,
                                            ));
                                            if response.clicked() {
                                                choose = Some(key.time);
                                            }
                                        }
                                    },
                                );
                            }
                        })
                })
                .inner;
            targets.push(("graphSlotKeyPicker".into(), picker.response.rect));
            for index in 0..4 {
                let response = ui.add_enabled(
                    enabled,
                    egui::Button::selectable(
                        channel == index,
                        RichText::new(CHANNEL_NAMES[index]).color(CHANNEL_COLORS[index]),
                    ),
                );
                targets.push((
                    format!("graphSlotChannel:{}", CHANNEL_NAMES[index]),
                    response.rect,
                ));
                if response.clicked() {
                    channel = index;
                }
            }
            let fit = ui.add_enabled(enabled, egui::Button::new("Fit").small());
            targets.push(("graphFit".into(), fit.rect));
            if fit.clicked() {
                self.cancel_gesture();
                self.graph.slot.bounds = None;
            }
            let zoom = ui.add_enabled(
                enabled && selected_time.is_some(),
                egui::Button::new("Zoom Keys").small(),
            );
            targets.push(("graphZoomKeys".into(), zoom.rect));
            if zoom.clicked()
                && let Some(time) = selected_time
            {
                self.cancel_gesture();
                let next = self
                    .slot_color_track(self.animation, slot)
                    .and_then(|track| track.keys.iter().find(|key| key.time > time))
                    .map(|key| key.time);
                let fit = self.slot_fit_bounds(slot);
                self.graph.slot.bounds = Some(zoom_bounds(
                    time,
                    next,
                    self.session.document().fps,
                    [fit[2], fit[3]],
                ));
            }
        });
        for (name, rect) in targets {
            self.record_target(&name, rect);
        }
        if channel != self.graph.slot.channel {
            self.cancel_gesture();
            self.graph.slot.channel = channel;
            self.graph.slot.draft = None;
        }
        if let Some(time) = choose {
            self.cancel_gesture();
            self.set_time(time);
            self.key_selection
                .replace(vec![AnimationKey::SlotColor { slot, time }]);
        }
        if let Some(context) = self.slot_curve_context(slot)
            && let Some((key, next)) =
                self.slot_color_track(self.animation, slot)
                    .and_then(|track| {
                        let index = track
                            .keys
                            .binary_search_by(|key| key.time.total_cmp(&context.time))
                            .ok()?;
                        Some((
                            track.keys[index].clone(),
                            track.keys.get(index + 1).cloned(),
                        ))
                    })
        {
            ui.horizontal(|ui| {
                self.slot_curve_presets(ui, context, key.curves[context.channel]);
                if key.curves == [Curve::Stepped; 4] {
                    ui.small("Stepped: all RGBA. Leaving it makes other channels Linear.");
                }
                if handles(key.curves[context.channel]).is_some()
                    && let Some(next) = &next
                {
                    let header = egui::CollapsingHeader::new("Handles")
                        .id_salt("slot-handle-controls")
                        .default_open(false)
                        .show(ui, |ui| self.slot_curve_numbers(ui, context, &key, next));
                    self.record_target("graphSlotHandles", header.header_response.rect);
                }
            });
        }
        let enabled = self.slot_authoring_input_available(ui.ctx());
        ui.add_enabled_ui(enabled, |ui| self.animation_controls(ui));
        if self.animation != owner.0 || self.timeline_owner() != Some(KeyOwner::Slot { slot }) {
            return true;
        }
        let context = self.slot_curve_context(slot);
        self.graph.slot.context = context;
        if self.graph.slot.bounds.is_none() {
            self.graph.slot.bounds = Some(self.slot_fit_bounds(slot));
        }
        let bounds = self
            .graph
            .slot
            .edit
            .as_ref()
            .map_or(self.graph.slot.bounds.unwrap(), |edit| edit.bounds);
        let Some(plot) = self.slot_plot(slot, bounds, context) else {
            self.graph.slot.context = None;
            self.graph.slot.draft = None;
            ui.label("This slot has no color keys. Key Color to create its RGBA curves.");
            return true;
        };
        let footer_lines = usize::from(self.graph.slot.error.is_some())
            + usize::from(plot.visible_intervals > INTERVAL_LIMIT);
        let footer = footer_lines as f32
            * (ui.text_style_height(&egui::TextStyle::Small) + ui.spacing().item_spacing.y);
        let (_, canvas) = ui.allocate_space(vec2(
            ui.available_width(),
            (ui.available_height() - footer).max(90.),
        ));
        self.record_target("graphSlotCanvas", canvas);
        self.record_target("graphCanvas", canvas);
        self.record_target("graphClip", ui.clip_rect());
        let rect =
            egui::Rect::from_min_max(canvas.min + vec2(244., 24.), canvas.max - vec2(45., 10.));
        if !valid_geometry(rect, bounds) {
            self.cancel_slot_curve_edit();
            self.graph.slot.error =
                Some("Graph interval cannot be displayed as finite coordinates.".into());
            ui.label("Graph interval cannot be displayed as finite coordinates.");
            return true;
        }
        self.record_target("graphPlot", rect);
        let response = ui.interact(canvas, Self::graph_id(), Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Slot RGBA value graph")
        });
        if self
            .graph
            .slot
            .edit
            .as_ref()
            .is_some_and(|edit| edit.rect != rect)
        {
            self.cancel_slot_curve_edit();
        }
        let painter = ui.painter_at(canvas);
        painter.rect_filled(canvas, 4., Color32::from_rgb(21, 26, 33));
        let labels = egui::Rect::from_min_max(canvas.min, pos2(rect.left() - 5., canvas.bottom()));
        painter.rect_filled(labels, 0., Color32::from_gray(53));
        let slot_name = self.session.document().slots[slot].name.clone();
        let bone_name = self.session.document().bones[self.session.document().slots[slot].bone]
            .name
            .clone();
        painter.text(
            pos2(labels.left() + 12., labels.top() + 36.),
            egui::Align2::LEFT_CENTER,
            bone_name,
            FontId::proportional(12.),
            MINT,
        );
        let row = egui::Rect::from_min_max(
            pos2(labels.left() + 5., labels.top() + 48.),
            pos2(labels.right() - 3., labels.top() + 70.),
        );
        self.record_target("graphSlotRow", row);
        ui.painter_at(row).text(
            pos2(row.left() + 16., row.center().y),
            egui::Align2::LEFT_CENTER,
            format!("RGBA: {slot_name}"),
            FontId::proportional(12.),
            CHANNEL_COLORS[self.graph.slot.channel],
        );
        let plot_painter = ui.painter_at(rect);
        for value in [0., 0.25, 0.5, 0.75, 1.] {
            let y = point(rect, bounds, 0., value).y;
            if rect.y_range().contains(y) {
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
        let first_frame = (bounds[0] * fps / step).ceil() * step;
        for index in 0..128 {
            let frame = first_frame + index as f64 * step;
            let time = frame / fps;
            if time > bounds[1] {
                break;
            }
            let x = point(rect, bounds, time, 0.).x;
            if !x.is_finite() {
                continue;
            }
            plot_painter.line_segment(
                [pos2(x, rect.top()), pos2(x, rect.bottom())],
                Stroke::new(1., Color32::from_rgb(43, 49, 58)),
            );
            painter.text(
                pos2(x, canvas.top() + 11.),
                egui::Align2::CENTER_CENTER,
                format!("{frame:.0}"),
                FontId::monospace(10.),
                MUTED,
            );
        }
        let setup = self.session.document().slots[slot].color;
        for channel in 0..4 {
            let color = CHANNEL_COLORS[channel];
            if let Some(first) = &plot.first
                && bounds[0] < f64::from(first.time)
            {
                let value = f64::from(setup[channel]) / 255.;
                plot_painter.line_segment(
                    [
                        point(rect, bounds, bounds[0], value),
                        point(rect, bounds, f64::from(first.time).min(bounds[1]), value),
                    ],
                    Stroke::new(1.2, color),
                );
            }
            if let Some(last) = &plot.last
                && bounds[1] > f64::from(last.time)
            {
                let value = f64::from(channel_value(last, channel));
                plot_painter.line_segment(
                    [
                        point(rect, bounds, f64::from(last.time).max(bounds[0]), value),
                        point(rect, bounds, bounds[1], value),
                    ],
                    Stroke::new(1.2, color),
                );
            }
            for (_, from, to) in &plot.intervals {
                let left = channel_value(from, channel);
                let right = channel_value(to, channel);
                let points: Vec<_> = (0..=SAMPLES)
                    .map(|part| {
                        let progress = part as f32 / SAMPLES as f32;
                        let time = f64::from(from.time)
                            + (f64::from(to.time) - f64::from(from.time)) * f64::from(progress);
                        point(
                            rect,
                            bounds,
                            time,
                            f64::from(peachy_engine::curve_value(
                                from.curves[channel],
                                progress,
                                left,
                                right,
                            )),
                        )
                    })
                    .collect();
                self.graph.slot.rendered_points += points.len();
                if points.iter().all(|point| point.is_finite()) {
                    if from.curves[channel] == Curve::Stepped {
                        for (part, segment) in points.windows(2).enumerate() {
                            if part % 2 == 0 {
                                plot_painter.line_segment(
                                    [segment[0], segment[1]],
                                    Stroke::new(1.4, color),
                                );
                            }
                        }
                        plot_painter.line_segment(
                            [
                                point(rect, bounds, f64::from(to.time), f64::from(left)),
                                point(rect, bounds, f64::from(to.time), f64::from(right)),
                            ],
                            Stroke::new(1., color),
                        );
                    } else {
                        plot_painter.add(egui::Shape::line(
                            points,
                            Stroke::new(
                                if channel == self.graph.slot.channel {
                                    1.8
                                } else {
                                    1.3
                                },
                                color,
                            ),
                        ));
                    }
                }
            }
        }
        self.graph.slot.rendered_intervals = plot.intervals.len();
        let hit_rect = rect.intersect(ui.clip_rect());
        let mut keys = Vec::new();
        for (index, key) in &plot.markers {
            for (channel, &color) in CHANNEL_COLORS.iter().enumerate() {
                let position = point(
                    rect,
                    bounds,
                    f64::from(key.time),
                    f64::from(channel_value(key, channel)),
                );
                if !position.is_finite() || !hit_rect.contains(position) {
                    continue;
                }
                let marker = egui::Rect::from_center_size(position, vec2(6., 6.));
                if context
                    .is_some_and(|context| context.time == key.time && context.channel == channel)
                {
                    plot_painter.rect_filled(marker, 0., color);
                } else {
                    plot_painter.rect_stroke(
                        marker,
                        0.,
                        Stroke::new(1., color),
                        egui::StrokeKind::Middle,
                    );
                }
                self.record_target(
                    &format!("graphSlotKeyPoint:{index}:{channel}"),
                    egui::Rect::from_center_size(position, vec2(10., 10.)),
                );
                keys.push((key.time, channel, position));
            }
        }
        let mut handle_points = Vec::new();
        if let (Some(context), Some((key, Some(next)))) = (context, &plot.selected)
            && let Some(h) = value_handles(
                key.curves[context.channel],
                channel_value(key, context.channel),
                channel_value(next, context.channel),
            )
        {
            let anchors = [
                (key.time, channel_value(key, context.channel)),
                (next.time, channel_value(next, context.channel)),
            ];
            for index in 0..2 {
                let time = f64::from(key.time)
                    + h[index * 2] * (f64::from(next.time) - f64::from(key.time));
                let position = point(rect, bounds, time, h[index * 2 + 1]);
                if !position.is_finite() || !hit_rect.contains(position) {
                    continue;
                }
                let color = CHANNEL_COLORS[context.channel];
                let anchor = point(
                    rect,
                    bounds,
                    f64::from(anchors[index].0),
                    f64::from(anchors[index].1),
                );
                if anchor.is_finite() {
                    plot_painter.line_segment([anchor, position], Stroke::new(1., color));
                }
                plot_painter.circle_filled(position, 4., color);
                self.record_target(
                    &format!("graphSlotHandle:{index}"),
                    egui::Rect::from_center_size(position, vec2(12., 12.)),
                );
                handle_points.push((index, position));
            }
        }
        let playhead = point(rect, bounds, f64::from(self.time), 0.).x;
        if playhead.is_finite() {
            plot_painter.line_segment(
                [pos2(playhead, rect.top()), pos2(playhead, rect.bottom())],
                Stroke::new(1., ACCENT),
            );
        }
        let press = ui.input(|input| super::key_drag::primary_pointer_event(&input.events, true));
        if self.graph.slot.edit.is_none()
            && !self.graph.slot.blocked
            && self.slot_authoring_input_available(ui.ctx())
            && ui.is_enabled()
            && let Some((pointer, modifiers)) = press
            && response.contains_pointer()
            && ui.clip_rect().contains(pointer)
        {
            response.request_focus();
            if hit_rect.contains(pointer)
                && let Some((handle, _)) = handle_points
                    .iter()
                    .find(|(_, point)| point.distance(pointer) <= 9.)
                && let (Some(context), Some((source, Some(next)))) = (context, &plot.selected)
            {
                self.cancel_gesture();
                self.session.commit_transaction();
                self.session.begin_transaction();
                self.playing = false;
                self.graph.slot.edit = Some(SlotCurveEdit {
                    context,
                    source: source.clone(),
                    next: next.clone(),
                    curves: source.curves,
                    handle: *handle,
                    rect,
                    bounds,
                    cursor: self.time,
                    skin: self.skin.clone(),
                    tool: self.tool,
                    axes: self.axes,
                    auto_key: self.auto_key,
                    pose_generation: self.pose_view_generation,
                    pose: self.session.pose_edits().clone(),
                    finish_pending: false,
                });
            } else if hit_rect.contains(pointer)
                && let Some((time, channel, _)) = keys
                    .iter()
                    .filter(|(_, _, point)| point.distance(pointer) <= 9.)
                    .min_by(|(_, a_channel, a), (_, b_channel, b)| {
                        a.distance(pointer)
                            .total_cmp(&b.distance(pointer))
                            .then_with(|| {
                                (*b_channel == self.graph.slot.channel)
                                    .cmp(&(*a_channel == self.graph.slot.channel))
                            })
                    })
            {
                self.cancel_gesture();
                self.set_time(*time);
                self.graph.slot.channel = *channel;
                self.key_selection.select(
                    AnimationKey::SlotColor { slot, time: *time },
                    modifiers.command || modifiers.ctrl,
                );
            }
        }
        let release =
            ui.input(|input| super::key_drag::primary_pointer_event(&input.events, false));
        if self.graph.slot.edit.is_some() && (response.dragged() || release.is_some()) {
            if let Some(pointer) = release
                .map(|(point, _)| point)
                .or_else(|| response.interact_pointer_pos())
            {
                self.update_slot_curve_handle(pointer);
            }
            if release.is_some()
                && let Some(edit) = self.graph.slot.edit.as_mut()
            {
                edit.finish_pending = true;
            }
        }
        if let Some(error) = &self.graph.slot.error {
            ui.colored_label(ACCENT, error);
        }
        if plot.visible_intervals > INTERVAL_LIMIT {
            ui.small("Showing a bounded set of intervals. Choose an exact key, then Zoom Keys.");
        }
        true
    }

    fn update_slot_curve_handle(&mut self, pointer: Pos2) {
        let Some(mut edit) = self.graph.slot.edit.take() else {
            return;
        };
        if !valid_geometry(edit.rect, edit.bounds) || !pointer.is_finite() {
            self.session.cancel_transaction();
            self.graph.slot.blocked = true;
            return;
        }
        let channel = edit.context.channel;
        let Some(mut values) = value_handles(
            edit.source.curves[channel],
            channel_value(&edit.source, channel),
            channel_value(&edit.next, channel),
        ) else {
            self.session.cancel_transaction();
            return;
        };
        let time = edit.bounds[0]
            + (f64::from(pointer.x) - f64::from(edit.rect.left())) / f64::from(edit.rect.width())
                * (edit.bounds[1] - edit.bounds[0]);
        let value = edit.bounds[2]
            + (f64::from(edit.rect.bottom()) - f64::from(pointer.y))
                / f64::from(edit.rect.height())
                * (edit.bounds[3] - edit.bounds[2]);
        values[edit.handle * 2] = (time - f64::from(edit.context.time))
            / (f64::from(edit.next.time) - f64::from(edit.context.time));
        values[edit.handle * 2 + 1] = value;
        let Some(curve) = curve_with_value_handles(
            edit.source.curves[channel],
            values,
            channel_value(&edit.source, channel),
            channel_value(&edit.next, channel),
        ) else {
            self.session.cancel_transaction();
            self.graph.slot.blocked = true;
            self.graph.slot.error = Some("Bezier handles must be finite numbers.".into());
            return;
        };
        edit.curves = changed_channel(edit.source.curves, channel, curve);
        match self
            .session
            .preview_transaction(Command::SetSlotColorCurves {
                animation: edit.context.animation,
                slot: edit.context.slot,
                time: edit.context.time,
                curves: edit.curves,
            }) {
            Ok(()) => self.graph.slot.edit = Some(edit),
            Err(error) => {
                self.session.cancel_transaction();
                self.graph.slot.blocked = true;
                self.graph.slot.error = Some(error);
            }
        }
    }

    pub(super) fn slot_curve_diagnostic(&self) -> serde_json::Value {
        serde_json::json!({
            "context": self.graph.slot.context.map(|context| serde_json::json!({"animation":context.animation,"slot":context.slot,"time":context.time,"channel":CHANNEL_NAMES[context.channel]})),
            "bounds":self.graph.slot.bounds,"active":self.graph.slot.edit.is_some(),"blocked":self.graph.slot.blocked,
            "error":self.graph.slot.error,"renderedPoints":self.graph.slot.rendered_points,"renderedIntervals":self.graph.slot.rendered_intervals
        })
    }
}

#[cfg(test)]
#[path = "slot_curve_tests.rs"]
mod tests;
