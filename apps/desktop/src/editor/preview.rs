//! Independent keyed-pose playback. Preview never queries or edits Session's pending pose.
use super::icons::Icon;
use super::*;
use peachy_engine::{Scene, checked_evaluate_scene};

pub(super) struct PreviewView {
    animation: usize,
    skin: String,
    time: f32,
    playing: bool,
    direction: PlayDirection,
    speed: f32,
    looping: bool,
    zoom: f32,
    pan: Vec2,
    // Keep a world center separately from pixel pan. Subtract before scaling so
    // a very large finite origin cannot erase the small canvas coordinates.
    center: [f32; 2],
    fit: bool,
    show_bones: bool,
    error: Option<String>,
    controls: Vec<egui::Id>,
    epoch: u64,
}

impl Default for PreviewView {
    fn default() -> Self {
        Self {
            animation: 0,
            skin: String::new(),
            time: 0.,
            playing: false,
            direction: PlayDirection::Forward,
            speed: 1.,
            looping: true,
            zoom: 1.,
            pan: Vec2::ZERO,
            center: [0., 0.],
            fit: true,
            show_bones: false,
            error: None,
            controls: Vec::new(),
            epoch: 0,
        }
    }
}

impl PreviewView {
    pub(super) fn reset_document(&mut self) {
        self.animation = 0;
        self.skin.clear();
        self.time = 0.;
        self.playing = false;
        self.direction = PlayDirection::Forward;
        self.zoom = 1.;
        self.pan = Vec2::ZERO;
        self.center = [0., 0.];
        self.fit = true;
        self.error = None;
        self.controls.clear();
        self.epoch = self.epoch.wrapping_add(1);
    }

    fn duration(&self, doc: &Document) -> f32 {
        peachy_engine::last_key_time(doc, self.animation)
    }

    fn reconcile(&mut self, doc: &Document) {
        if self.animation >= doc.animations.len() {
            self.animation = doc.animations.len().saturating_sub(1);
            self.time = 0.;
            self.playing = false;
            self.fit = true;
        }
        if !doc.skins.contains_key(&self.skin) {
            self.skin = preferred_skin(doc);
            self.fit = true;
        }
        let duration = self.duration(doc);
        self.time = if self.time.is_finite() {
            self.time.clamp(0., duration)
        } else {
            self.playing = false;
            0.
        };
        if duration == 0. {
            self.playing = false;
        }
        self.speed = if self.speed.is_finite() {
            self.speed.clamp(0.01, 2.)
        } else {
            1.
        };
        if !self.zoom.is_finite()
            || self.zoom <= 0.
            || !self.pan.is_finite()
            || !self.center.into_iter().all(f32::is_finite)
        {
            self.zoom = 1.;
            self.pan = Vec2::ZERO;
            self.center = [0., 0.];
            self.fit = true;
        }
    }

    fn seek(&mut self, time: f64, duration: f32) {
        if time.is_finite() {
            self.time = time.clamp(0., f64::from(duration)) as f32;
            self.playing = false;
        }
    }

    fn toggle(&mut self, direction: PlayDirection, duration: f32) {
        if duration <= 0. {
            self.playing = false;
            self.time = 0.;
            return;
        }
        if self.playing && self.direction == direction {
            self.playing = false;
        } else {
            if direction == PlayDirection::Forward && self.time >= duration {
                self.time = 0.;
            } else if direction == PlayDirection::Backward && self.time <= 0. {
                self.time = duration;
            }
            self.direction = direction;
            self.playing = true;
        }
    }

    fn advance(&mut self, elapsed: f32, duration: f32) {
        let elapsed = if elapsed.is_finite() && elapsed >= 0. {
            elapsed.min(0.1) * self.speed
        } else {
            0.
        };
        let step = peachy_engine::advance_playback(
            self.time,
            elapsed,
            duration,
            self.looping,
            self.direction,
        );
        self.time = step.time;
        self.playing = step.playing;
    }

    fn screen(&self, rect: egui::Rect, point: [f32; 2]) -> Pos2 {
        pos2(
            (f64::from(rect.center().x)
                + f64::from(self.pan.x)
                + (f64::from(point[0]) - f64::from(self.center[0])) * f64::from(self.zoom))
                as f32,
            (f64::from(rect.center().y) + f64::from(self.pan.y)
                - (f64::from(point[1]) - f64::from(self.center[1])) * f64::from(self.zoom))
                as f32,
        )
    }

    fn fit_points(&mut self, rect: egui::Rect, points: &[[f32; 2]]) {
        if points.is_empty() {
            self.center = [0., 0.];
            self.zoom = 1.;
        } else {
            let mut min = [f64::INFINITY; 2];
            let mut max = [f64::NEG_INFINITY; 2];
            for point in points {
                for axis in 0..2 {
                    min[axis] = min[axis].min(f64::from(point[axis]));
                    max[axis] = max[axis].max(f64::from(point[axis]));
                }
            }
            self.center = [0, 1].map(|axis| ((min[axis] + max[axis]) * 0.5) as f32);
            self.zoom = ((f64::from(rect.width()) - 20.).max(1.) / (max[0] - min[0]).max(1.))
                .min((f64::from(rect.height()) - 20.).max(1.) / (max[1] - min[1]).max(1.))
                .clamp(f64::from(f32::MIN_POSITIVE), 6.) as f32;
        }
        self.pan = Vec2::ZERO;
        self.fit = false;
    }
}

impl Editor {
    fn preview_canvas_id() -> egui::Id {
        egui::Id::new("independent-preview-canvas")
    }

    pub(super) fn advance_preview(&mut self, ctx: &egui::Context) {
        self.preview.reconcile(self.session.document());
        if !self.preview_visible()
            || !ctx.input(|input| input.focused)
            || self.error.is_some()
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
            || egui::Popup::is_any_open(ctx)
        {
            self.preview.playing = false;
            if let Some(id) = ctx.memory(|memory| memory.focused())
                && self.preview.controls.contains(&id)
            {
                ctx.memory_mut(|memory| memory.surrender_focus(id));
            }
            return;
        }
        if self.preview.playing {
            let duration = self.preview.duration(self.session.document());
            self.preview
                .advance(ctx.input(|input| input.stable_dt), duration);
            ctx.request_repaint();
        }
    }

    pub(super) fn preview_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        if !self.preview_visible() || !ctx.input(|input| input.focused) {
            return false;
        }
        let focused = ctx.memory(|memory| memory.focused());
        if !focused.is_some_and(|id| self.preview.controls.contains(&id)) {
            return false;
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F9)) {
            self.preview.playing = false;
            self.toggle_views();
            return true;
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.preview.playing = false;
            if let Some(id) = focused {
                ctx.memory_mut(|memory| memory.surrender_focus(id));
            }
            return true;
        }
        // A Preview numeric field owns text editing, including its own Undo.
        // Global file/history actions remain available outside text entry.
        if ctx.text_edit_focused() {
            for key in [egui::Key::S, egui::Key::O, egui::Key::E] {
                if ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, key)) {
                    if let Some(id) = focused {
                        ctx.memory_mut(|memory| memory.surrender_focus(id));
                    }
                    match key {
                        egui::Key::S => self.save_project(),
                        egui::Key::O => self.open_project(ctx),
                        _ => self.export(),
                    }
                    return true;
                }
            }
            return true;
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::R)) {
            self.preview.looping = !self.preview.looping;
            return true;
        }
        if ctx.input(|input| input.modifiers.command || input.modifiers.ctrl || input.modifiers.alt)
        {
            return false;
        }
        let duration = self.preview.duration(self.session.document());
        for (key, direction) in [
            (egui::Key::A, PlayDirection::Backward),
            (egui::Key::D, PlayDirection::Forward),
        ] {
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key)) {
                self.preview.toggle(direction, duration);
            }
        }
        for (key, time) in [(egui::Key::Q, 0.), (egui::Key::E, f64::from(duration))] {
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key)) {
                self.preview.seek(time, duration);
            }
        }
        for (key, delta) in [(egui::Key::F, -1.), (egui::Key::R, 1.)] {
            if ctx.input(|input| input.key_pressed(key)) {
                let frames = if ctx.input(|input| input.modifiers.shift) {
                    10.
                } else {
                    1.
                };
                let time = f64::from(self.preview.time)
                    + delta * frames / f64::from(self.session.document().fps);
                self.preview.seek(time, duration);
            }
        }
        // Arrows remain with the focused numeric/slider widget. Canvas focus
        // blocks main bone/key nudges without inventing a second editing target.
        true
    }

    fn preview_control(&mut self, name: &str, response: &egui::Response) {
        self.record_target(name, response.rect);
        self.preview.controls.push(response.id);
        if response.clicked() {
            response.request_focus();
        }
    }

    fn preview_scene(&self) -> Result<Scene, String> {
        let doc = self.session.document();
        checked_evaluate_scene(
            doc,
            (!doc.animations.is_empty()).then_some(self.preview.animation),
            self.preview.time,
            &self.preview.skin,
            None,
        )
    }

    pub(super) fn preview_panel(&mut self, ui: &mut egui::Ui) {
        ui.push_id(("independent-preview", self.preview.epoch), |ui| {
            self.preview_contents(ui);
        });
    }

    fn preview_contents(&mut self, ui: &mut egui::Ui) {
        self.preview.reconcile(self.session.document());
        self.preview.controls.clear();
        self.record_target("previewPanelClip", ui.clip_rect());
        let animations: Vec<_> = self
            .session
            .document()
            .animations
            .iter()
            .map(|a| a.name.clone())
            .collect();
        let skins: Vec<_> = self.session.document().skins.keys().cloned().collect();
        let previous_animation = self.preview.animation;
        let previous_skin = self.preview.skin.clone();
        ui.horizontal(|ui| {
            ui.label("Animation");
            let combo = egui::ComboBox::from_id_salt("preview-animation")
                .width(ui.available_width())
                .wrap_mode(egui::TextWrapMode::Truncate)
                .selected_text(
                    animations
                        .get(self.preview.animation)
                        .map_or("No animations", String::as_str),
                )
                .show_ui(ui, |ui| {
                    for (index, name) in animations.iter().enumerate() {
                        let row = ui.selectable_value(&mut self.preview.animation, index, name);
                        self.record_target(&format!("previewAnimation:{index}"), row.rect);
                    }
                });
            self.preview_control("previewAnimation", &combo.response);
        });
        ui.horizontal(|ui| {
            ui.label("Skin");
            let combo = egui::ComboBox::from_id_salt("preview-skin")
                .width(ui.available_width())
                .wrap_mode(egui::TextWrapMode::Truncate)
                .selected_text(if self.preview.skin.is_empty() {
                    "No skins"
                } else {
                    &self.preview.skin
                })
                .show_ui(ui, |ui| {
                    for name in &skins {
                        let row = ui.selectable_value(&mut self.preview.skin, name.clone(), name);
                        self.record_target(&format!("previewSkin:{name}"), row.rect);
                    }
                });
            self.preview_control("previewSkin", &combo.response);
        });
        if self.preview.animation != previous_animation {
            self.preview.time = 0.;
            self.preview.playing = false;
            self.preview.fit = true;
        }
        if self.preview.skin != previous_skin {
            self.preview.fit = true;
        }
        let duration = self.preview.duration(self.session.document());
        ui.horizontal_wrapped(|ui| {
            for (name, icon, time) in [
                ("Start", Icon::FirstFrame, 0.),
                ("End", Icon::LastFrame, f64::from(duration)),
            ] {
                let response =
                    icons::toolbar_button(ui, icon, &format!("Preview {name}"), "", duration > 0.);
                self.preview_control(&format!("preview{name}"), &response);
                if response.clicked() {
                    self.preview.seek(time, duration);
                }
            }
            for (direction, name) in [
                (PlayDirection::Backward, "Backward"),
                (PlayDirection::Forward, "Forward"),
            ] {
                let response = ui
                    .add_enabled_ui(duration > 0., |ui| {
                        dopesheet::transport_button(
                            ui,
                            direction,
                            self.preview.playing && self.preview.direction == direction,
                        )
                    })
                    .inner
                    .on_hover_text(format!("Preview {name}; click again to stop"));
                self.preview_control(&format!("preview{name}"), &response);
                if response.clicked() {
                    self.preview.toggle(direction, duration);
                }
            }
            let looping = ui.checkbox(&mut self.preview.looping, "Loop");
            self.preview_control("previewLoop", &looping);
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Time");
            let mut time = f64::from(self.preview.time);
            let value = ui.add_enabled(
                duration > 0.,
                egui::DragValue::new(&mut time)
                    .speed(1. / f64::from(self.session.document().fps))
                    .range(0.0..=f64::from(duration))
                    .suffix(" s")
                    .custom_formatter(|value, _| compact_number(value))
                    .custom_parser(|text| {
                        text.trim()
                            .parse::<f64>()
                            .ok()
                            .filter(|value| value.is_finite())
                    })
                    .update_while_editing(false),
            );
            self.preview_control("previewTime", &value);
            if value.changed() {
                self.preview.seek(time, duration);
            }
            if value.gained_focus() || value.drag_started() {
                self.preview.playing = false;
            }
            ui.label(
                RichText::new(format!("/ {} s", compact_number(f64::from(duration))))
                    .small()
                    .color(MUTED),
            );
        });
        let mut progress = if duration > 0. {
            f64::from(self.preview.time) / f64::from(duration)
        } else {
            0.
        };
        let scrub = ui
            .add_enabled(
                duration > 0.,
                egui::Slider::new(&mut progress, 0.0..=1.0).show_value(false),
            )
            .on_hover_text("Seek this Preview without moving the editor playhead");
        self.preview_control("previewScrub", &scrub);
        if scrub.changed() {
            self.preview.seek(progress * f64::from(duration), duration);
        }
        if scrub.drag_started() || scrub.clicked() {
            self.preview.playing = false;
        }
        let mut percent = self.preview.speed * 100.;
        ui.scope(|ui| {
            ui.spacing_mut().slider_width = (ui.available_width() - 85.).max(40.);
            let speed = ui.add(
                egui::Slider::new(&mut percent, 1.0..=200.0)
                    .text("Speed")
                    .suffix("%")
                    .integer(),
            );
            self.preview_control("previewSpeed", &speed);
            if speed.changed() && percent.is_finite() {
                self.preview.speed = (percent / 100.).clamp(0.01, 2.);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let fit = ui.small_button("Fit");
            self.preview_control("previewFit", &fit);
            if fit.clicked() {
                self.preview.fit = true;
            }
            let bones = ui.checkbox(&mut self.preview.show_bones, "Bones");
            self.preview_control("previewBones", &bones);
            if bones.changed() {
                self.preview.fit = true;
            }
            ui.label(
                RichText::new(format!(
                    "{}%",
                    compact_number(f64::from(self.preview.zoom) * 100.)
                ))
                .small()
                .color(MUTED),
            );
        });
        ui.label(
            RichText::new("Single track. Mixing and multiple tracks are not available.")
                .small()
                .color(MUTED),
        );
        self.preview_canvas(ui);
        // Compound controls (notably Slider's numeric value) can focus an
        // internal child response rather than the ID returned by the widget.
        if let Some(id) = ui.memory(|memory| memory.focused())
            && let Some(response) = ui.ctx().read_response(id)
            && ui.min_rect().contains_rect(response.rect)
        {
            self.preview.controls.push(id);
        }
    }

    fn preview_canvas(&mut self, ui: &mut egui::Ui) {
        // The auxiliary dock wraps this panel in a vertical ScrollArea, whose
        // content height is unbounded. Size from its finite clip instead.
        let height = (ui.clip_rect().bottom() - ui.cursor().top()).max(100.);
        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), height));
        let response = ui.interact(rect, Self::preview_canvas_id(), Sense::click_and_drag());
        self.preview_control("previewViewport", &response);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0., Color32::from_gray(43));
        let scene = match self.preview_scene() {
            Ok(scene) => scene,
            Err(error) => {
                self.preview_error(&painter, rect, error);
                return;
            }
        };
        let mut points: Vec<_> = scene
            .attachments
            .iter()
            .flat_map(|draw| draw.vertices.iter().copied())
            .collect();
        if self.preview.show_bones {
            for (bone, world) in self.session.document().bones.iter().zip(&scene.world) {
                points.extend([world.point(0., 0.), world.point(bone.length, 0.)]);
            }
        }
        if !points
            .iter()
            .flatten()
            .all(|coordinate| coordinate.is_finite())
        {
            self.preview_error(
                &painter,
                rect,
                "Preview bone coordinates are outside the finite rendering range".into(),
            );
            return;
        }
        if self.preview.fit {
            self.preview.fit_points(rect, &points);
        }
        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll.is_finite() && scroll != 0. {
                let old_zoom = f64::from(self.preview.zoom);
                let zoom = (old_zoom * (f64::from(scroll) * 0.0015).exp())
                    .clamp(f64::from(f32::MIN_POSITIVE), 6.) as f32;
                let scale = f64::from(zoom) / old_zoom;
                let pan = vec2(
                    (f64::from(self.preview.pan.x) * scale) as f32,
                    (f64::from(self.preview.pan.y) * scale) as f32,
                );
                if pan.is_finite() {
                    self.preview.zoom = zoom;
                    self.preview.pan = pan;
                }
            }
        }
        if response.dragged_by(egui::PointerButton::Secondary)
            || response.dragged_by(egui::PointerButton::Middle)
        {
            let pan = self.preview.pan + response.drag_delta();
            if pan.is_finite() {
                self.preview.pan = pan;
            }
        }
        if !points
            .iter()
            .all(|&point| self.preview.screen(rect, point).is_finite())
        {
            self.preview_error(
                &painter,
                rect,
                "Preview cannot be displayed at this scale. Use Fit to reset the view.".into(),
            );
            return;
        }
        self.preview.error = None;
        for draw in &scene.attachments {
            if let Some(texture) = self.textures.get(&draw.image) {
                let mut mesh = egui::Mesh::with_texture(texture.id());
                for (&point, &uv) in draw.vertices.iter().zip(&draw.uvs) {
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: self.preview.screen(rect, point),
                        uv: pos2(uv[0], uv[1]),
                        color: Color32::WHITE,
                    });
                }
                mesh.indices = draw.triangles.clone();
                painter.add(egui::Shape::mesh(mesh));
            }
        }
        if self.preview.show_bones {
            for (bone, world) in self.session.document().bones.iter().zip(&scene.world) {
                painter.line_segment(
                    [
                        self.preview.screen(rect, world.point(0., 0.)),
                        self.preview.screen(rect, world.point(bone.length, 0.)),
                    ],
                    Stroke::new(1., MINT),
                );
            }
        }
        if scene.attachments.is_empty() && !self.preview.show_bones {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No visible attachments",
                FontId::proportional(11.),
                MUTED,
            );
        }
    }

    fn preview_error(&mut self, painter: &egui::Painter, rect: egui::Rect, error: String) {
        self.preview.playing = false;
        self.preview.error = Some(error.clone());
        // Draw the message through egui's bounded text layout, rather than an
        // unwrapped painter string that can escape a narrow dock.
        let galley = painter.layout(
            format!("Preview unavailable\n{error}"),
            FontId::proportional(11.),
            ACCENT,
            (rect.width() - 16.).max(1.),
        );
        painter.galley(rect.center() - galley.size() * 0.5, galley, ACCENT);
    }

    pub(super) fn preview_diagnostic(&self) -> serde_json::Value {
        let scene = self.preview_scene();
        serde_json::json!({
            "visible":self.preview_visible(), "animation":(!self.session.document().animations.is_empty()).then_some(self.preview.animation),
            "skin":self.preview.skin, "time":self.preview.time, "playing":self.preview.playing,
            "direction":self.preview.direction, "speed":self.preview.speed, "looping":self.preview.looping,
            "zoom":self.preview.zoom,"pan":[self.preview.pan.x,self.preview.pan.y], "center":self.preview.center,
            "showBones":self.preview.show_bones,"error":self.preview.error.as_ref().or_else(||scene.as_ref().err()),
            "attachments":scene.as_ref().ok().map(|scene|&scene.attachments),
            "appliedLocals":scene.as_ref().ok().map(|scene|&scene.applied_locals)
        })
    }
}

fn compact_number(value: f64) -> String {
    if value != 0. && (value.abs() >= 1e6 || value.abs() < 0.001) {
        format!("{value:.3e}")
    } else {
        format!("{value:.3}")
    }
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod tests;
