//! A compact shared playhead. Timeline keys are markers, never edit targets.
use super::icons::Icon;
use super::key_drag::primary_pointer_event;
use super::timeline_model::{TimelineMap, TimelineRange, key_times, ticks};
use super::*;

const PLAYHEAD: Color32 = Color32::from_rgb(91, 199, 224);

#[derive(Clone, Copy)]
struct Scrub {
    map: TimelineMap,
    animation: usize,
    looping: bool,
    wrap: bool,
}

struct FrameEntry {
    id: egui::Id,
    animation: usize,
    edited: bool,
}

pub(super) struct TimelineView {
    range: TimelineRange,
    fit: bool,
    epoch: u64,
    scrub: Option<Scrub>,
    blocked: bool,
    frame_text: String,
    frame_entry: Option<FrameEntry>,
    aborted: Option<egui::Id>,
    error: Option<String>,
}

impl Default for TimelineView {
    fn default() -> Self {
        Self {
            range: TimelineRange::default(),
            fit: true,
            epoch: 0,
            scrub: None,
            blocked: false,
            frame_text: String::new(),
            frame_entry: None,
            aborted: None,
            error: None,
        }
    }
}

impl TimelineView {
    pub fn reset_range(&mut self) {
        self.fit = true;
        self.epoch = self.epoch.wrapping_add(1);
    }
    pub fn reset_document(&mut self) {
        self.reset_range();
        self.range = TimelineRange::default();
        self.frame_text.clear();
        self.error = None;
    }
    fn reveal(&mut self, time: f32) {
        let native = time;
        let time = f64::from(native);
        if time < self.range.start || time > self.range.start + self.range.span {
            let next = native.next_up();
            let spacing = if next.is_finite() {
                f64::from(next) - time
            } else {
                time - f64::from(native.next_down())
            };
            self.range.span = self.range.span.max(spacing * 4.).min(f64::from(f32::MAX));
            self.range.start = (time - self.range.span * 0.8)
                .clamp(0., (f64::from(f32::MAX) - self.range.span).max(0.));
        }
    }
}

impl Editor {
    pub(super) fn timeline_id() -> egui::Id {
        egui::Id::new("compact-timeline-ruler")
    }

    pub(super) fn timeline_has_focus(&self, ctx: &egui::Context) -> bool {
        self.show_timeline
            && !self.setup_mode
            && ctx.input(|input| input.focused)
            && self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
            && !egui::Popup::is_any_open(ctx)
            && !ctx.text_edit_focused()
            && ctx.memory(|memory| memory.has_focus(Self::timeline_id()))
    }

    pub(super) fn cancel_timeline_interaction(&mut self) {
        if self.timeline_view.scrub.take().is_some() {
            self.timeline_view.blocked = true;
        }
        if let Some(entry) = self.timeline_view.frame_entry.take() {
            self.timeline_view.aborted = Some(entry.id);
        }
        self.timeline_view.error = None;
    }

    pub(super) fn reconcile_timeline_input(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.pointer.primary_down()) {
            self.timeline_view.blocked = false;
        }
        if self.setup_mode
            || !self.show_timeline
            || self.error.is_some()
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
            || egui::Popup::is_any_open(ctx)
            || !ctx.input(|input| input.focused)
        {
            self.cancel_timeline_interaction();
            ctx.memory_mut(|memory| memory.surrender_focus(Self::timeline_id()));
        }
        if self
            .timeline_view
            .scrub
            .is_some_and(|scrub| scrub.animation != self.animation || scrub.looping != self.looping)
            || self
                .timeline_view
                .frame_entry
                .as_ref()
                .is_some_and(|entry| entry.animation != self.animation)
        {
            self.cancel_timeline_interaction();
        }
        if let Some(id) = self.timeline_view.aborted.take() {
            ctx.memory_mut(|memory| memory.surrender_focus(id));
        }
    }

    pub(super) fn timeline_view_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        if self.error.is_some()
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
            || egui::Popup::is_any_open(ctx)
            || !ctx.input(|input| input.focused)
        {
            self.cancel_timeline_interaction();
            return false;
        }
        let owns_text = self
            .timeline_view
            .frame_entry
            .as_ref()
            .is_some_and(|entry| ctx.memory(|memory| memory.has_focus(entry.id)));
        if owns_text && !egui::Popup::is_any_open(ctx) {
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.cancel_timeline_interaction();
                ctx.memory_mut(|memory| memory.request_focus(Self::timeline_id()));
                return true;
            }
            for key in [egui::Key::S, egui::Key::O, egui::Key::E] {
                if ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, key)) {
                    self.cancel_timeline_interaction();
                    if key == egui::Key::S {
                        self.save_project();
                    } else if key == egui::Key::O {
                        self.open_project(ctx);
                    } else {
                        self.export();
                    }
                    return true;
                }
            }
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F9)) {
                self.toggle_views();
                return true;
            }
        }
        if self.timeline_view.scrub.is_some()
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.cancel_timeline_interaction();
            return true;
        }
        false
    }

    fn timeline_seek(&mut self, time: f32, reveal: bool) {
        self.playing = false;
        self.set_time(time);
        if reveal {
            self.timeline_view.reveal(self.time);
        }
    }

    fn timeline_frame_entry(&mut self, ui: &mut egui::Ui) {
        let was_editing = self.timeline_view.frame_entry.is_some();
        let tab = ui.input(|input| input.key_pressed(egui::Key::Tab));
        let id = ui.make_persistent_id(("frame", self.animation, self.timeline_view.epoch));
        let aborted = self.timeline_view.aborted.take();
        if let Some(aborted) = aborted {
            ui.memory_mut(|memory| memory.surrender_focus(aborted));
        }
        if self.timeline_view.frame_entry.is_none() {
            self.timeline_view.frame_text =
                frame_label(f64::from(self.time) * f64::from(self.session.document().fps));
        }
        ui.label("Frame");
        let response = ui.add(
            egui::TextEdit::singleline(&mut self.timeline_view.frame_text)
                .id(id)
                .desired_width(76.),
        ).on_hover_text("Enter a frame, including decimals. Enter/Tab applies; Escape or clicking away cancels.");
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::TextEdit,
                ui.is_enabled(),
                "Timeline frame",
            )
        });
        self.record_target("compactFrameValue", response.rect);
        if (response.gained_focus() || response.has_focus())
            && aborted != Some(id)
            && self.timeline_view.frame_entry.is_none()
        {
            self.cancel_gesture();
            self.playing = false;
            self.timeline_view.frame_entry = Some(FrameEntry {
                id,
                animation: self.animation,
                edited: response.changed(),
            });
            if let Some(mut state) = egui::text_edit::TextEditState::load(ui.ctx(), id) {
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(self.timeline_view.frame_text.chars().count()),
                    )));
                state.store(ui.ctx(), id);
            }
        }
        if response.changed() {
            if let Some(entry) = &mut self.timeline_view.frame_entry {
                entry.edited = true;
            }
            self.timeline_view.error = None;
        }
        // A rapid re-entry may be classified as a text double click on release.
        // Keep the untouched numeric value selected in full, including its fraction.
        if response.clicked()
            && self
                .timeline_view
                .frame_entry
                .as_ref()
                .is_some_and(|entry| !entry.edited)
            && let Some(mut state) = egui::text_edit::TextEditState::load(ui.ctx(), id)
        {
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(self.timeline_view.frame_text.chars().count()),
                )));
            state.store(ui.ctx(), id);
        }
        let enter = (response.has_focus() || response.lost_focus())
            && ui.input(|input| input.key_pressed(egui::Key::Enter));
        if (enter || (was_editing && tab))
            && let Some(entry) = self.timeline_view.frame_entry.take()
        {
            if !self.show_timeline || self.setup_mode || entry.animation != self.animation {
                return;
            }
            if entry.edited {
                let time = parse_frame(&self.timeline_view.frame_text, self.session.document().fps);
                if let Some(time) = time {
                    self.timeline_seek(time, true);
                } else {
                    self.timeline_view.error = Some("Enter a finite, nonnegative frame.".into());
                    self.timeline_view.frame_entry = Some(entry);
                    response.request_focus();
                    return;
                }
            }
            response.surrender_focus();
            if enter {
                ui.memory_mut(|memory| memory.request_focus(Self::timeline_id()));
            }
        } else if (response.lost_focus() || !response.has_focus())
            && self.timeline_view.frame_entry.is_some()
        {
            self.timeline_view.frame_entry = None;
            self.timeline_view.error = None;
        }
    }

    pub(super) fn compact_timeline(&mut self, ui: &mut egui::Ui) {
        self.reconcile_timeline_input(ui.ctx());
        self.record_target("compactTimeline", ui.max_rect());
        self.record_target("compactTimelineClip", ui.clip_rect());
        if self.timeline_view.fit {
            self.timeline_view.range =
                TimelineRange::fit(self.animation_end(), self.session.document().fps);
            self.timeline_view.fit = false;
        }
        ui.horizontal(|ui| {
            icons::heading(ui, Icon::Timeline, "Timeline");
            for (direction, name) in [
                (PlayDirection::Backward, "Backward"),
                (PlayDirection::Forward, "Forward"),
            ] {
                let response = dopesheet::transport_button(
                    ui,
                    direction,
                    self.playing && self.play_direction == direction,
                );
                self.record_target(&format!("compact{name}"), response.rect);
                if response
                    .on_hover_text(if direction == PlayDirection::Forward {
                        "Play forward/stop · D"
                    } else {
                        "Play backward/stop · A"
                    })
                    .clicked()
                {
                    self.toggle_playback(direction);
                    ui.memory_mut(|memory| memory.request_focus(Self::timeline_id()));
                }
            }
            for (icon, name, hint) in [
                (Icon::FirstFrame, "Start", "First frame · Q"),
                (Icon::PreviousFrame, "PreviousFrame", "Previous frame · F"),
                (Icon::NextFrame, "NextFrame", "Next frame · R"),
                (Icon::PreviousKey, "PreviousKey", "Previous key · S"),
                (Icon::NextKey, "NextKey", "Next key · W"),
                (Icon::LastFrame, "End", "Last key · E"),
            ] {
                let response = icons::toolbar_button(ui, icon, hint, "", true);
                self.record_target(&format!("compact{name}"), response.rect);
                if response.clicked() {
                    match name {
                        "Start" => self.timeline_seek(0., true),
                        "End" => self.timeline_seek(self.animation_end(), true),
                        "PreviousFrame" => {
                            self.step_frames(-1.);
                            self.timeline_view.reveal(self.time);
                        }
                        "NextFrame" => {
                            self.step_frames(1.);
                            self.timeline_view.reveal(self.time);
                        }
                        "PreviousKey" => {
                            self.jump_key(PlayDirection::Backward);
                            self.timeline_view.reveal(self.time);
                        }
                        _ => {
                            self.jump_key(PlayDirection::Forward);
                            self.timeline_view.reveal(self.time);
                        }
                    }
                    ui.memory_mut(|memory| memory.request_focus(Self::timeline_id()));
                }
            }
            self.timeline_frame_entry(ui);
            let looping = ui.checkbox(&mut self.looping, "Loop");
            self.record_target("compactLoop", looping.rect);
            if looping.changed() {
                self.cancel_gesture();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let close =
                    icons::toolbar_button(ui, Icon::Delete, "Close Timeline view", "", true);
                self.record_target("closeView:Timeline", close.rect);
                if close.clicked() {
                    self.cancel_gesture();
                    self.show_timeline = false;
                }
            });
        });
        if !self.show_timeline {
            return;
        }
        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), 44.));
        let response = ui.interact(rect, Self::timeline_id(), Sense::click_and_drag());
        self.record_target("compactRuler", rect);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Timeline ruler")
        });
        let mut map = TimelineMap {
            range: self.timeline_view.range,
            left: rect.left() + 8.,
            width: (rect.width() - 16.).max(1.),
        };
        if response.hovered() && self.timeline_view.scrub.is_none() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll != 0. {
                let anchor = ui
                    .input(|input| input.pointer.hover_pos())
                    .map_or(0.5, |pointer| f64::from((pointer.x - map.left) / map.width));
                self.timeline_view.range = self.timeline_view.range.zoom(
                    f64::from((-scroll * 0.005).exp()),
                    anchor,
                    self.session.document().fps,
                );
                map.range = self.timeline_view.range;
            }
        }
        if self.timeline_view.scrub.is_none() && response.dragged_by(egui::PointerButton::Secondary)
        {
            self.timeline_view.range = self
                .timeline_view
                .range
                .pan(response.drag_delta().x, map.width);
            map.range = self.timeline_view.range;
        }
        let (press, release) = ui.input(|input| {
            (
                primary_pointer_event(&input.events, true),
                primary_pointer_event(&input.events, false),
            )
        });
        let enabled = self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
            && !egui::Popup::is_any_open(ui.ctx())
            && ui.input(|input| input.focused);
        if enabled
            && !self.timeline_view.blocked
            && let Some((position, modifiers)) = press
            && rect.contains(position)
            && response.is_pointer_button_down_on()
        {
            self.cancel_gesture();
            let start = map
                .time_at(position.x, self.session.document().fps, !modifiers.shift)
                .unwrap_or(0.);
            self.timeline_view.scrub = Some(Scrub {
                map,
                animation: self.animation,
                looping: self.looping,
                wrap: self.looping && self.animation_end() > 0. && start < self.animation_end(),
            });
            response.request_focus();
        }
        if self
            .timeline_view
            .scrub
            .is_some_and(|scrub| scrub.animation != self.animation || scrub.looping != self.looping)
        {
            self.cancel_timeline_interaction();
        }
        if enabled && let Some(scrub) = self.timeline_view.scrub.take() {
            let pointer = release
                .map(|(position, _)| position)
                .or_else(|| ui.input(|input| input.pointer.interact_pos()));
            let shift = release.or(press).map_or_else(
                || ui.input(|input| input.modifiers.shift),
                |(_, modifiers)| modifiers.shift,
            );
            if let Some(time) = pointer.and_then(|pointer| {
                scrub
                    .map
                    .time_at(pointer.x, self.session.document().fps, !shift)
            }) {
                let end = self.animation_end();
                let time = if scrub.wrap && time > end {
                    time.rem_euclid(end)
                } else {
                    time
                };
                self.timeline_seek(time, false);
            }
            if ui.input(|input| input.pointer.primary_down()) && release.is_none() {
                self.timeline_view.scrub = Some(scrub);
            }
        }
        if response.clicked_elsewhere() {
            response.surrender_focus();
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 3., Color32::from_rgb(25, 29, 34));
        for (time, label) in ticks(map.range, map.width, self.session.document().fps) {
            if let Some(x) = map.x_at(time) {
                painter.line_segment(
                    [pos2(x, rect.top() + 20.), pos2(x, rect.bottom())],
                    Stroke::new(1., Color32::from_gray(65)),
                );
                painter.text(
                    pos2(x, rect.top() + 9.),
                    egui::Align2::CENTER_CENTER,
                    label,
                    FontId::monospace(11.),
                    MUTED,
                );
            }
        }
        for (index, time) in key_times(self.session.document(), self.animation)
            .into_iter()
            .enumerate()
        {
            if let Some(x) = map.x_at(time) {
                let point = pos2(x, rect.bottom() - 7.);
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        point + vec2(0., -3.),
                        point + vec2(3., 0.),
                        point + vec2(0., 3.),
                        point + vec2(-3., 0.),
                    ],
                    ACCENT,
                    Stroke::NONE,
                ));
                self.record_target(
                    &format!("compactKey:{index}"),
                    egui::Rect::from_center_size(point, vec2(6., 6.)),
                );
            }
        }
        if let Some(x) = map.x_at(self.animation_end()) {
            painter.line_segment(
                [pos2(x, rect.top() + 18.), pos2(x, rect.bottom())],
                Stroke::new(1.5, ACCENT),
            );
        }
        if let Some(x) = map.x_at(self.time) {
            painter.line_segment(
                [pos2(x, rect.top() + 16.), pos2(x, rect.bottom())],
                Stroke::new(1.5, PLAYHEAD),
            );
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(x - 4., rect.top() + 17.),
                    pos2(x + 4., rect.top() + 17.),
                    pos2(x, rect.top() + 23.),
                ],
                PLAYHEAD,
                Stroke::NONE,
            ));
            self.record_target(
                "compactPlayhead",
                egui::Rect::from_center_size(pos2(x, rect.center().y), vec2(2., rect.height())),
            );
        }
        response.on_hover_text(
            "Click/drag to scrub · Shift: between frames · right drag: pan · wheel: zoom",
        );
        ui.horizontal(|ui| {
            let base = TimelineRange::fit(self.animation_end(), self.session.document().fps);
            let mut zoom = base.span / self.timeline_view.range.span;
            ui.spacing_mut().slider_width = 120.;
            let slider = ui.add(
                egui::Slider::new(&mut zoom, 0.1..=16.)
                    .logarithmic(true)
                    .show_value(false)
                    .clamping(egui::SliderClamping::Never)
                    .text("Zoom"),
            );
            self.record_target("compactZoom", slider.rect);
            if slider.changed() && zoom.is_finite() && zoom > 0. {
                self.cancel_gesture();
                self.timeline_view.range = self.timeline_view.range.zoom(
                    base.span / zoom / self.timeline_view.range.span,
                    0.5,
                    self.session.document().fps,
                );
            }
            let fit = ui.small_button("Zoom Keys");
            self.record_target("compactZoomKeys", fit.rect);
            if fit.clicked() {
                self.cancel_gesture();
                self.timeline_view.range = base;
                ui.memory_mut(|memory| memory.request_focus(Self::timeline_id()));
            }
            ui.label(
                RichText::new(format!("{} fps", self.session.document().fps))
                    .small()
                    .color(MUTED),
            );
            if let Some(error) = &self.timeline_view.error {
                ui.colored_label(ACCENT, error);
            } else if let Some(animation) = self.session.document().animations.get(self.animation) {
                ui.add(egui::Label::new(RichText::new(&animation.name).color(MUTED)).truncate());
            }
        });
    }

    pub(super) fn timeline_view_diagnostic(&self) -> serde_json::Value {
        serde_json::json!({"visible":self.show_timeline && !self.setup_mode,"storedVisible":self.show_timeline,"start":self.timeline_view.range.start,"span":self.timeline_view.range.span,"scrubbing":self.timeline_view.scrub.is_some(),"blocked":self.timeline_view.blocked,"editingFrame":self.timeline_view.frame_entry.is_some(),"error":self.timeline_view.error,"keyTimes":key_times(self.session.document(),self.animation)})
    }
}

fn frame_label(frame: f64) -> String {
    if frame.abs() >= 1_000_000. {
        format!("{frame:.6e}")
    } else {
        format!("{frame:.3}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn parse_frame(text: &str, fps: u32) -> Option<f32> {
    let frame = text.trim().parse::<f64>().ok()?;
    let time = frame / f64::from(fps);
    (fps > 0 && frame.is_finite() && frame >= 0. && time <= f64::from(f32::MAX))
        .then_some(time as f32)
        .filter(|time| time.is_finite())
}

#[cfg(test)]
#[path = "timeline_view_tests.rs"]
mod tests;
