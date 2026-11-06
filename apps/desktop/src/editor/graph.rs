//! A bounded scalar value graph. Session owns every key change and its history.
use super::*;

const KEY_DRAW_LIMIT: usize = 256;
const CURVE_GRID: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GraphContext {
    animation: usize,
    bone: usize,
    property: Property,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct GraphBounds {
    time_min: f64,
    time_max: f64,
    value_min: f64,
    value_max: f64,
}

impl GraphBounds {
    fn valid(self) -> bool {
        [self.time_min, self.time_max, self.value_min, self.value_max]
            .into_iter()
            .all(f64::is_finite)
            && self.time_min >= 0.
            && self.time_max <= f64::from(f32::MAX)
            && self.time_max > self.time_min
            && self.value_max > self.value_min
    }
    fn point(self, rect: egui::Rect, time: f32, value: f32) -> Pos2 {
        pos2(
            (f64::from(rect.left())
                + (f64::from(time) - self.time_min) / (self.time_max - self.time_min)
                    * f64::from(rect.width())) as f32,
            (f64::from(rect.bottom())
                - (f64::from(value) - self.value_min) / (self.value_max - self.value_min)
                    * f64::from(rect.height())) as f32,
        )
    }
}

struct GraphEdit {
    context: GraphContext,
    source: Track,
    key: Keyframe,
    value: f32,
    rect: egui::Rect,
    bounds: GraphBounds,
    pointer: Pos2,
    cursor: f32,
    skin: String,
    tool: Tool,
    axes: Axes,
    auto_key: bool,
    pose_generation: u64,
    pose: peachy_engine::PoseEdits,
    moved: bool,
    finish_pending: bool,
}

pub(super) struct GraphView {
    property: Property,
    context: Option<GraphContext>,
    selected: Option<f32>,
    bounds: Option<GraphBounds>,
    fit: bool,
    edit: Option<GraphEdit>,
    blocked: bool,
    owned_focus: bool,
    error: Option<String>,
    rendered_keys: usize,
    rendered_points: usize,
}

impl Default for GraphView {
    fn default() -> Self {
        Self {
            property: Property::Rotation,
            context: None,
            selected: None,
            bounds: None,
            fit: true,
            edit: None,
            blocked: false,
            owned_focus: false,
            error: None,
            rendered_keys: 0,
            rendered_points: 0,
        }
    }
}

impl GraphView {
    pub(super) fn reset_document(&mut self) {
        // The shell cancels the owning transaction before Open.
        *self = Self::default();
    }
}

struct GraphPlot {
    curve: Vec<Pos2>,
    keys: Vec<(Keyframe, Pos2)>,
}

/// Evaluate a single existing interval through the engine. Binary search plus a
/// two-key Track avoids rescanning a dense full track for every display point.
fn graph_sample(track: &Track, time: f32) -> f32 {
    let next = track.keys.partition_point(|key| key.time <= time);
    if next == 0 {
        track.property.neutral()
    } else if next == track.keys.len() {
        track.keys[next - 1].value
    } else {
        peachy_engine::sample(
            &Track {
                bone: track.bone,
                property: track.property,
                keys: vec![track.keys[next - 1], track.keys[next]],
            },
            time,
        )
    }
}

fn shown_key_indices(track: &Track, bounds: GraphBounds, selected: Option<f32>) -> Vec<usize> {
    let first = track
        .keys
        .partition_point(|key| f64::from(key.time) < bounds.time_min);
    let last = track
        .keys
        .partition_point(|key| f64::from(key.time) <= bounds.time_max);
    let count = last.saturating_sub(first);
    let shown = count.min(KEY_DRAW_LIMIT);
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
        && !indices.contains(&index)
        && (bounds.time_min..=bounds.time_max).contains(&f64::from(time))
    {
        indices.push(index);
        indices.sort_unstable();
    }
    indices
}

fn graph_times(track: &Track, bounds: GraphBounds, selected: Option<f32>) -> Vec<f32> {
    let mut times: Vec<_> = (0..=CURVE_GRID)
        .map(|index| {
            (bounds.time_min
                + (bounds.time_max - bounds.time_min) * index as f64 / CURVE_GRID as f64)
                as f32
        })
        .collect();
    let indices = shown_key_indices(track, bounds, selected);
    for &index in &indices {
        let key = track.keys[index];
        times.push(key.time);
        if key.time > 0. {
            let before = key.time.next_down();
            if f64::from(before) >= bounds.time_min {
                times.push(before);
            }
        }
        if index + 1 < track.keys.len() && indices.len() <= 128 {
            let next = track.keys[index + 1].time;
            let divisions = if indices.len() <= 32 { 8 } else { 2 };
            for part in 1..divisions {
                let time = (f64::from(key.time)
                    + (f64::from(next) - f64::from(key.time)) * f64::from(part)
                        / f64::from(divisions)) as f32;
                if (bounds.time_min..=bounds.time_max).contains(&f64::from(time)) {
                    times.push(time);
                }
            }
        }
    }
    times.sort_by(f32::total_cmp);
    times.dedup(); // Exact native time only; close distinct keys stay distinct.
    times
}

fn graph_plot(
    track: &Track,
    bounds: GraphBounds,
    rect: egui::Rect,
    selected: Option<f32>,
) -> Result<GraphPlot, String> {
    if !bounds.valid() || !rect.is_finite() || rect.width() <= 0. || rect.height() <= 0. {
        return Err("Graph coordinates are outside the supported range. Use Fit.".into());
    }
    let mut curve = Vec::new();
    for time in graph_times(track, bounds, selected) {
        let value = graph_sample(track, time);
        let point = bounds.point(rect, time, value);
        if !value.is_finite() || !point.is_finite() {
            return Err("This interval cannot be evaluated as finite graph coordinates.".into());
        }
        curve.push(point);
    }
    let keys = shown_key_indices(track, bounds, selected)
        .into_iter()
        .map(|index| {
            let key = track.keys[index];
            (key, bounds.point(rect, key.time, key.value))
        })
        .collect();
    Ok(GraphPlot { curve, keys })
}

fn graph_number(value: f64) -> String {
    if value != 0. && !(0.001..10000.).contains(&value.abs()) {
        format!("{value:.2e}")
    } else {
        format!("{value:.2}")
    }
}

impl Editor {
    pub(super) fn graph_id() -> egui::Id {
        egui::Id::new("graph-value-canvas")
    }

    fn graph_context(&self) -> Option<GraphContext> {
        let bone = self.selected_bone()?;
        (!self.setup_mode
            && bone < self.session.document().bones.len()
            && self.animation < self.session.document().animations.len())
        .then_some(GraphContext {
            animation: self.animation,
            bone,
            property: self.graph.property,
        })
    }

    fn graph_track(&self, context: GraphContext) -> Option<&Track> {
        self.session
            .document()
            .animations
            .get(context.animation)?
            .tracks
            .iter()
            .find(|track| track.bone == context.bone && track.property == context.property)
    }

    fn graph_source_matches(&self, edit: &GraphEdit) -> bool {
        self.graph_track(edit.context).is_some_and(|track| {
            track.keys.len() == edit.source.keys.len()
                && track
                    .keys
                    .iter()
                    .zip(&edit.source.keys)
                    .all(|(current, base)| {
                        current.time == base.time
                            && current.curve == base.curve
                            && current.value.to_bits()
                                == if base.time == edit.key.time {
                                    edit.value.to_bits()
                                } else {
                                    base.value.to_bits()
                                }
                    })
        })
    }

    pub(super) fn graph_edit_active(&self) -> bool {
        self.graph.edit.is_some()
    }

    pub(super) fn cancel_graph_edit(&mut self) {
        if self.graph.edit.take().is_some() {
            self.session.cancel_transaction();
            self.graph.blocked = true;
        }
    }

    pub(super) fn reconcile_graph_input(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.pointer.primary_down()) {
            self.graph.blocked = false;
        }
        if !self.graph_visible() || !ctx.input(|input| input.focused) {
            self.graph.owned_focus = false;
            ctx.memory_mut(|memory| memory.surrender_focus(Self::graph_id()));
        }
        if self.graph.edit.as_ref().is_some_and(|edit| {
            !self.graph_visible()
                || !ctx.input(|input| input.focused)
                || self.error.is_some()
                || self.rename.is_some()
                || self.duplicate_animation.is_some()
                || egui::Popup::is_any_open(ctx)
                || self.graph_context() != Some(edit.context)
                || self.time.to_bits() != edit.cursor.to_bits()
                || self.skin != edit.skin
                || self.tool != edit.tool
                || self.axes != edit.axes
                || self.auto_key != edit.auto_key
                || self.pose_view_generation != edit.pose_generation
                || self.session.pose_edits() != &edit.pose
                || (ctx.text_edit_focused()
                    && !ctx.memory(|memory| memory.has_focus(Self::graph_id())))
                || !self.graph_source_matches(edit)
        }) {
            self.cancel_graph_edit();
        }
    }

    pub(super) fn settle_graph_input(&mut self, ctx: &egui::Context) {
        self.reconcile_graph_input(ctx);
        if !ctx.input(|input| input.pointer.primary_down())
            && self
                .graph
                .edit
                .as_ref()
                .is_some_and(|edit| edit.finish_pending)
        {
            self.graph.edit = None;
            self.session.commit_transaction();
        }
    }

    /// Called after global file/history/tool routing. Graph canvas focus never
    /// turns Delete, clipboard chords, arrows or transport letters into rig edits.
    pub(super) fn graph_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        if !self.graph_visible()
            || !ctx.input(|input| input.focused)
            || ctx.text_edit_focused()
            || egui::Popup::is_any_open(ctx)
            || !(ctx.memory(|memory| memory.has_focus(Self::graph_id()))
                || (self.graph.owned_focus
                    && ctx.input(|input| input.key_pressed(egui::Key::Escape)))
                || self.graph_edit_active())
        {
            return false;
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            if self.graph.edit.is_some() {
                self.cancel_graph_edit();
            } else {
                self.graph.selected = None;
            }
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F9)) {
            self.toggle_views();
        }
        true
    }

    pub(super) fn observe_graph_focus(&mut self, ctx: &egui::Context) {
        self.graph.owned_focus = self.graph_visible()
            && ctx.input(|input| input.focused)
            && ctx.memory(|memory| memory.has_focus(Self::graph_id()));
    }

    fn begin_graph_edit(
        &mut self,
        context: GraphContext,
        key: Keyframe,
        pointer: Pos2,
        rect: egui::Rect,
        bounds: GraphBounds,
    ) {
        self.cancel_gesture();
        let Some(source) = self.graph_track(context).cloned() else {
            return;
        };
        if !source.keys.contains(&key) {
            return;
        }
        self.session.commit_transaction();
        self.session.begin_transaction();
        self.playing = false;
        self.graph.error = None;
        self.graph.edit = Some(GraphEdit {
            context,
            source,
            key,
            value: key.value,
            rect,
            bounds,
            pointer,
            cursor: self.time,
            skin: self.skin.clone(),
            tool: self.tool,
            axes: self.axes,
            auto_key: self.auto_key,
            pose_generation: self.pose_view_generation,
            pose: self.session.pose_edits().clone(),
            moved: false,
            finish_pending: false,
        });
    }

    fn update_graph_edit(&mut self, pointer: Pos2) {
        let Some(edit) = self.graph.edit.as_mut() else {
            return;
        };
        let delta = f64::from(edit.pointer.y) - f64::from(pointer.y);
        edit.moved |= delta.abs() >= 2.;
        if !edit.moved {
            return;
        }
        let value = if delta == 0. {
            edit.key.value
        } else {
            (f64::from(edit.key.value)
                + delta / f64::from(edit.rect.height())
                    * (edit.bounds.value_max - edit.bounds.value_min)) as f32
        };
        if !value.is_finite() {
            self.cancel_graph_edit();
            self.graph.error =
                Some("Graph value is outside the finite native range; edit canceled.".into());
            return;
        }
        if value.to_bits() == edit.value.to_bits() {
            return;
        }
        let context = edit.context;
        let key = Keyframe { value, ..edit.key };
        match self.session.preview_transaction(Command::SetKey {
            animation: context.animation,
            bone: context.bone,
            property: context.property,
            key,
        }) {
            Ok(()) => {
                self.graph.edit.as_mut().unwrap().value = value;
            }
            Err(error) => {
                self.cancel_graph_edit();
                self.graph.error = Some(error);
            }
        }
    }

    fn fit_graph(&mut self, context: GraphContext, zoom_key: bool) {
        let Some(track) = self.graph_track(context) else {
            return;
        };
        let frame = 1. / f64::from(self.session.document().fps);
        let mut time_min = 0.;
        let mut time_max = f64::from(self.animation_end()).max(frame);
        if zoom_key && let Some(time) = self.graph.selected {
            let index = track.keys.partition_point(|key| key.time < time);
            if track.keys.get(index).is_some_and(|key| key.time == time) {
                let left = index
                    .checked_sub(1)
                    .map(|i| f64::from(time) - f64::from(track.keys[i].time));
                let right = track
                    .keys
                    .get(index + 1)
                    .map(|key| f64::from(key.time) - f64::from(time));
                let native_grain = if time.next_up().is_finite() {
                    f64::from(time.next_up()) - f64::from(time)
                } else {
                    f64::from(time) - f64::from(time.next_down())
                };
                let radius = left
                    .into_iter()
                    .chain(right)
                    .filter(|distance| *distance > 0.)
                    .min_by(f64::total_cmp)
                    .unwrap_or(frame);
                let radius = radius.max(native_grain);
                time_min = (f64::from(time) - radius).max(0.);
                time_max = (f64::from(time) + radius).min(f64::from(f32::MAX));
            }
        }
        if time_max <= time_min {
            time_min = (time_min - frame).max(0.);
            time_max = (time_max + frame).min(f64::from(f32::MAX));
        }
        let mut bounds = GraphBounds {
            time_min,
            time_max,
            value_min: -1.,
            value_max: 1.,
        };
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for time in graph_times(track, bounds, self.graph.selected) {
            let value = f64::from(graph_sample(track, time));
            if !value.is_finite() {
                self.graph.error =
                    Some("This interval cannot be evaluated as finite graph values.".into());
                return;
            }
            min = min.min(value);
            max = max.max(value);
        }
        let padding = ((max - min) * 0.1)
            .max(if min == max { min.abs() * 0.1 } else { 0. })
            .max(0.5);
        bounds.value_min = min - padding;
        bounds.value_max = max + padding;
        if bounds.valid() {
            self.graph.bounds = Some(bounds);
            self.graph.error = None;
        } else {
            self.graph.error = Some("Graph coordinates are outside the supported range.".into());
        }
        self.graph.fit = false;
    }

    pub(super) fn graph_panel(&mut self, ui: &mut egui::Ui) {
        self.reconcile_graph_input(ui.ctx());
        self.graph.rendered_keys = 0;
        self.graph.rendered_points = 0;
        let previous = self.graph.property;
        let picker = egui::ComboBox::from_id_salt("graph-property")
            .selected_text(previous.label())
            .show_ui(ui, |ui| {
                for property in Property::ALL {
                    let response =
                        ui.selectable_value(&mut self.graph.property, property, property.label());
                    self.record_target(&format!("graphProperty:{property:?}"), response.rect);
                }
            });
        self.record_target("graphPropertyPicker", picker.response.rect);
        if previous != self.graph.property {
            self.cancel_gesture();
        }
        let context = self.graph_context();
        if context != self.graph.context {
            self.cancel_graph_edit();
            self.graph.context = context;
            self.graph.selected = None;
            self.graph.bounds = None;
            self.graph.fit = true;
            self.graph.error = None;
        }
        let Some(context) = context else {
            ui.label("Select a bone in Animate to view its keyed values.");
            return;
        };
        let Some(track) = self.graph_track(context) else {
            ui.label("This bone has no keys for this property.");
            return;
        };
        if track.keys.is_empty() {
            ui.label("This property has no keys.");
            return;
        }
        if self
            .graph
            .selected
            .is_some_and(|time| !track.keys.iter().any(|key| key.time == time))
        {
            self.graph.selected = None;
        }
        let units = match context.property {
            Property::ScaleX | Property::ScaleY => "Native values · multipliers of Setup scale",
            Property::Rotation | Property::ShearX | Property::ShearY => {
                "Native values · degrees added to Setup"
            }
            Property::X | Property::Y => "Native values · units added to Setup",
        };
        ui.label(RichText::new(units).small().color(MUTED));
        let mut choose = None;
        let mut choices = Vec::new();
        let selection = self.graph.selected;
        let selected_text = selection.map_or_else(
            || "Choose exact key".into(),
            |time| format!("Key at {time} s"),
        );
        let key_picker = egui::ComboBox::from_id_salt("graph-exact-key")
            .width(ui.available_width())
            .wrap_mode(egui::TextWrapMode::Truncate)
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                let track = self.graph_track(context).unwrap();
                egui::ScrollArea::vertical().max_height(180.).show_rows(
                    ui,
                    22.,
                    track.keys.len(),
                    |ui, rows| {
                        for index in rows {
                            let key = track.keys[index];
                            let row = ui.add(
                                egui::Button::selectable(
                                    selection == Some(key.time),
                                    format!(
                                        "{}: t={} s · value={}",
                                        index + 1,
                                        key.time,
                                        key.value
                                    ),
                                )
                                .truncate(),
                            );
                            choices.push((index, row.rect));
                            if row
                                .on_hover_text(format!(
                                    "Native time {} s · native value {}",
                                    key.time, key.value
                                ))
                                .clicked()
                            {
                                choose = Some(key.time);
                            }
                        }
                    },
                );
            });
        self.record_target("graphKeyPicker", key_picker.response.rect);
        for (index, rect) in choices {
            self.record_target(&format!("graphKeyChoice:{index}"), rect);
        }
        if let Some(time) = choose {
            self.cancel_gesture();
            self.graph.selected = Some(time);
        }
        ui.horizontal(|ui| {
            let fit = ui.small_button("Fit");
            self.record_target("graphFit", fit.rect);
            if fit.clicked() {
                self.cancel_gesture();
                self.graph.fit = true;
            }
            let zoom = ui.add_enabled(
                self.graph.selected.is_some(),
                egui::Button::new("Zoom Keys").small(),
            );
            self.record_target("graphZoomKeys", zoom.rect);
            if zoom.clicked() {
                self.cancel_gesture();
                self.fit_graph(context, true);
            }
        });
        if self.graph.fit || self.graph.bounds.is_none() {
            self.fit_graph(context, false);
        }
        let Some(bounds) = self.graph.bounds else {
            if let Some(error) = &self.graph.error {
                ui.colored_label(ACCENT, error);
            }
            return;
        };
        let height = (ui.clip_rect().bottom() - ui.cursor().top()).clamp(100., 360.);
        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), height));
        self.record_target("graphCanvas", rect);
        self.record_target("graphClip", ui.clip_rect());
        let plot_rect =
            egui::Rect::from_min_max(rect.min + vec2(6., 23.), rect.max - vec2(62., 8.));
        self.record_target("graphPlot", plot_rect);
        let count = self
            .graph_track(context)
            .map_or(0, |track| track.keys.len());
        let mut hint =
            "Frames → · native values ↑ · drag a key vertically; timing is fixed.".to_string();
        if count > KEY_DRAW_LIMIT {
            hint.push_str(&format!(
                "\nShowing at most {} of {count} keys. Choose exact key, then Zoom Keys.",
                KEY_DRAW_LIMIT + 1
            ));
        }
        if let Some(time) = self.graph.selected
            && let Some(key) = self
                .graph_track(context)
                .and_then(|track| track.keys.iter().find(|key| key.time == time))
        {
            hint.push_str(&format!("\nSelected: {} s · value {}", key.time, key.value));
        }
        let canvas = ui
            .interact(rect, Self::graph_id(), Sense::click_and_drag())
            .on_hover_text(hint);
        canvas.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Graph key values")
        });
        if canvas.clicked_elsewhere() {
            canvas.surrender_focus();
        }
        if self
            .graph
            .edit
            .as_ref()
            .is_some_and(|edit| edit.rect != plot_rect)
        {
            self.cancel_graph_edit();
        }
        let preliminary = self
            .graph_track(context)
            .and_then(|track| graph_plot(track, bounds, plot_rect, self.graph.selected).ok());
        if let Some(plot) = &preliminary {
            let press = ui.input(|input| key_drag::primary_pointer_event(&input.events, true));
            if !self.graph.blocked
                && ui.input(|input| input.focused)
                && self.error.is_none()
                && self.rename.is_none()
                && self.duplicate_animation.is_none()
                && !egui::Popup::is_any_open(ui.ctx())
                && let Some((pointer, modifiers)) = press
                && !modifiers.command
                && !modifiers.ctrl
                && !modifiers.alt
                && rect.contains(pointer)
                && ui.clip_rect().contains(pointer)
                && (canvas.contains_pointer() || canvas.is_pointer_button_down_on())
            {
                let pressed = plot
                    .keys
                    .iter()
                    .filter(|(_, point)| point.distance(pointer) <= 8.)
                    .min_by(|(_, a), (_, b)| a.distance(pointer).total_cmp(&b.distance(pointer)))
                    .map(|(key, _)| *key);
                self.graph.selected = pressed.map(|key| key.time);
                canvas.request_focus();
                if let Some(key) = pressed {
                    self.begin_graph_edit(context, key, pointer, plot_rect, bounds);
                }
            }
        }
        if self.graph.edit.is_some() {
            let pointer = ui.input(|input| input.pointer.interact_pos());
            if let Some(pointer) = pointer {
                self.update_graph_edit(pointer);
            }
            if !ui.input(|input| input.pointer.primary_down())
                && let Some(edit) = self.graph.edit.as_mut()
            {
                edit.finish_pending = true;
            }
            ui.ctx().request_repaint();
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 3., Color32::from_rgb(21, 26, 33));
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
                pos2(x, rect.top() + 10.),
                egui::Align2::CENTER_CENTER,
                graph_number(
                    (bounds.time_min + (bounds.time_max - bounds.time_min) * fraction)
                        * f64::from(self.session.document().fps),
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
        let current = self
            .graph_track(context)
            .and_then(|track| graph_plot(track, bounds, plot_rect, self.graph.selected).ok());
        if let Some(plot) = current {
            self.graph.rendered_keys = plot.keys.len();
            self.graph.rendered_points = plot.curve.len();
            let clipped = painter.with_clip_rect(plot_rect);
            clipped.add(egui::Shape::line(plot.curve, Stroke::new(1.5, MINT)));
            for (index, (key, point)) in plot.keys.iter().enumerate() {
                let selected = self.graph.selected == Some(key.time);
                let target = egui::Rect::from_center_size(*point, vec2(10., 10.));
                self.record_target(&format!("graphKey:{}", key.time), target);
                self.record_target(&format!("graphDrawKey:{index}"), target);
                clipped.rect_filled(
                    egui::Rect::from_center_size(
                        *point,
                        Vec2::splat(if selected { 7. } else { 5. }),
                    ),
                    0.,
                    if selected { ACCENT } else { MINT },
                );
            }
            if (bounds.time_min..=bounds.time_max).contains(&f64::from(self.time)) {
                let x = bounds.point(plot_rect, self.time, 0.).x;
                clipped.line_segment(
                    [pos2(x, plot_rect.top()), pos2(x, plot_rect.bottom())],
                    Stroke::new(1., ACCENT),
                );
            }
        } else {
            self.cancel_graph_edit();
            self.graph.error =
                Some("Graph interval cannot be displayed as finite coordinates.".into());
        }
        if let Some(error) = &self.graph.error {
            let galley = painter.layout(
                error.clone(),
                FontId::proportional(11.),
                ACCENT,
                (rect.width() - 16.).max(1.),
            );
            painter.galley(rect.center() - galley.size() * 0.5, galley, ACCENT);
        }
    }

    pub(super) fn graph_diagnostic(&self) -> serde_json::Value {
        serde_json::json!({
            "visible":self.graph_visible(), "property":self.graph.property,
            "context":self.graph.context.map(|context|serde_json::json!({"animation":context.animation,"bone":context.bone,"property":context.property})),
            "selectedTime":self.graph.selected,"blocked":self.graph.blocked,"error":self.graph.error,
            "bounds":self.graph.bounds.map(|bounds|[bounds.time_min,bounds.time_max,bounds.value_min,bounds.value_max]),
            "renderedKeys":self.graph.rendered_keys,"renderedPoints":self.graph.rendered_points,
            "edit":self.graph.edit.as_ref().map(|edit|serde_json::json!({"time":edit.key.time,"sourceValue":edit.key.value,"value":edit.value,"curve":edit.key.curve,"moved":edit.moved,"finishPending":edit.finish_pending}))
        })
    }
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
