//! Session-only WAV inspection/audition. Audio is never a document edit or key.
use super::audio_output::{self, AudioPlayer};
use super::*;
use peachy_io::WavClip;

struct LoadedAudio {
    name: String,
    clip: WavClip,
    player: Option<Box<dyn AudioPlayer>>,
    position: f64,
    playing: bool,
}

#[derive(Clone, Copy)]
struct Scrub {
    left: f32,
    width: f32,
    start: f64,
    span: f64,
}

pub(super) struct AudioView {
    loaded: Option<LoadedAudio>,
    volume: f32,
    muted: bool,
    looping: bool,
    start: f64,
    span: f64,
    error: Option<String>,
    controls: Vec<egui::Id>,
    owned_focus: bool,
    epoch: u64,
    scrub: Option<Scrub>,
    blocked: bool,
}

impl Default for AudioView {
    fn default() -> Self {
        Self {
            loaded: None,
            volume: 1.,
            muted: false,
            looping: false,
            start: 0.,
            span: 1.,
            error: None,
            controls: Vec::new(),
            owned_focus: false,
            epoch: 0,
            scrub: None,
            blocked: false,
        }
    }
}

impl AudioView {
    pub(super) fn reset_document(&mut self) {
        self.stop();
        self.loaded = None;
        self.start = 0.;
        self.span = 1.;
        self.error = None;
        self.controls.clear();
        self.owned_focus = false;
        self.cancel_input();
        self.epoch = self.epoch.wrapping_add(1);
    }
    fn duration(&self) -> f64 {
        self.loaded
            .as_ref()
            .map_or(0., |loaded| loaded.clip.duration())
    }
    fn position(&self) -> f64 {
        self.loaded.as_ref().map_or(0., |loaded| loaded.position)
    }
    fn playing(&self) -> bool {
        self.loaded.as_ref().is_some_and(|loaded| loaded.playing)
    }
    fn can_play(&self) -> bool {
        self.loaded
            .as_ref()
            .is_some_and(|loaded| loaded.player.is_some())
    }
    fn cancel_input(&mut self) {
        if self.scrub.take().is_some() {
            self.blocked = true;
            self.epoch = self.epoch.wrapping_add(1);
        }
    }
    fn pause(&mut self) {
        self.cancel_input();
        if let Some(loaded) = &mut self.loaded {
            if let Some(player) = &mut loaded.player {
                if let Err(error) = player.pause() {
                    player.set_volume(0.);
                    let _ = player.stop();
                    self.error = Some(error);
                }
            } else {
                loaded.playing = false;
            }
        }
        self.poll();
    }
    fn stop(&mut self) {
        self.cancel_input();
        if let Some(loaded) = &mut self.loaded {
            if let Some(player) = &mut loaded.player {
                match player.stop() {
                    Ok(()) => self.error = None,
                    Err(error) => {
                        player.set_volume(0.);
                        self.error = Some(error);
                    }
                }
            } else {
                loaded.position = 0.;
                loaded.playing = false;
            }
        }
        self.poll();
    }
    fn toggle_play(&mut self) {
        self.cancel_input();
        if self.playing() {
            self.pause();
            return;
        }
        self.apply_preferences();
        if let Some(loaded) = &mut self.loaded
            && let Some(player) = &mut loaded.player
        {
            if loaded.position >= loaded.clip.duration()
                && let Err(error) = player.seek(0.)
            {
                self.error = Some(error);
                return;
            }
            match player.play() {
                Ok(()) => self.error = None,
                Err(error) => self.error = Some(error),
            }
        }
        self.poll();
    }
    fn seek(&mut self, time: f64) {
        if !time.is_finite() {
            return;
        }
        if let Some(loaded) = &mut self.loaded {
            let time = time.clamp(0., loaded.clip.duration());
            if let Some(player) = &mut loaded.player {
                match player.seek(time) {
                    Ok(()) => self.error = None,
                    Err(error) => self.error = Some(error),
                }
            } else {
                loaded.position = time;
            }
        }
        self.poll();
    }
    fn apply_preferences(&mut self) {
        if let Some(loaded) = &mut self.loaded
            && let Some(player) = &mut loaded.player
        {
            player.set_volume(if self.muted { 0. } else { self.volume });
            player.set_looping(self.looping);
        }
    }
    fn install(&mut self, name: String, clip: WavClip, player: Option<Box<dyn AudioPlayer>>) {
        self.stop();
        self.loaded = Some(LoadedAudio {
            name,
            clip,
            player,
            position: 0.,
            playing: false,
        });
        self.error = None;
        self.start = 0.;
        self.span = self.duration();
        self.cancel_input();
        self.blocked = true;
        self.epoch = self.epoch.wrapping_add(1);
        self.controls.clear();
        self.owned_focus = false;
        self.apply_preferences();
    }
    fn load(&mut self, path: &std::path::Path) {
        // Stage both parsers before replacing a usable clip or stopping its output.
        let result = files::load_wav(path).and_then(|clip| {
            let player = audio_output::open(&clip)?;
            Ok((clip, player))
        });
        match result {
            Ok((clip, player)) => self.install(
                path.file_name()
                    .map_or_else(|| "WAV".into(), |name| name.to_string_lossy().into_owned()),
                clip,
                player,
            ),
            Err(error) => self.error = Some(error),
        }
    }
    fn fit(&mut self) {
        self.start = 0.;
        self.span = self.duration().max(f64::MIN_POSITIVE);
    }
    fn poll(&mut self) {
        if let Some(loaded) = &mut self.loaded
            && let Some(player) = &mut loaded.player
        {
            let position = player.position();
            if !position.is_finite() || position < 0. {
                player.set_volume(0.);
                let _ = player.stop();
                loaded.playing = false;
                self.error = Some("The audio output reported an invalid position".into());
            } else {
                loaded.position = position.min(loaded.clip.duration());
                loaded.playing = player.is_playing();
            }
        }
    }
}

impl Editor {
    /// Stop owned audition before a synchronous native dialog blocks UI polling.
    /// Retain the loaded snapshot and preferences even when the dialog is cancelled.
    pub(super) fn pause_audio_for_modal(&mut self) {
        self.audio.pause();
        self.audio.epoch = self.audio.epoch.wrapping_add(1);
        self.audio.controls.clear();
        self.audio.owned_focus = false;
    }

    pub(super) fn load_audio_path(&mut self, path: &std::path::Path) {
        self.audio.load(path);
    }

    pub(super) fn advance_audio(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.pointer.primary_down()) {
            self.audio.blocked = false;
        }
        if !self.audio_visible()
            || !ctx.input(|input| input.focused)
            || self.error.is_some()
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
            || egui::Popup::is_any_open(ctx)
        {
            self.audio.cancel_input();
            self.audio.pause();
            if let Some(id) = ctx.memory(|memory| memory.focused())
                && self.audio.controls.contains(&id)
            {
                ctx.memory_mut(|memory| memory.surrender_focus(id));
                self.audio.epoch = self.audio.epoch.wrapping_add(1);
            }
            return;
        }
        self.audio.poll();
        if self.audio.playing() {
            ctx.request_repaint();
        }
    }

    pub(super) fn observe_audio_focus(&mut self, ctx: &egui::Context) {
        self.audio.owned_focus = self.audio_visible()
            && ctx.input(|input| input.focused)
            && ctx
                .memory(|memory| memory.focused())
                .is_some_and(|id| self.audio.controls.contains(&id));
    }

    pub(super) fn audio_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        if !self.audio_visible() || !ctx.input(|input| input.focused) {
            return false;
        }
        let focused = ctx.memory(|memory| memory.focused());
        let owns = focused.is_some_and(|id| self.audio.controls.contains(&id));
        let escape = ctx.input(|input| input.key_pressed(egui::Key::Escape));
        if !owns && !(focused.is_none() && escape && self.audio.owned_focus) {
            return false;
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.audio.cancel_input();
            self.audio.pause();
            self.audio.epoch = self.audio.epoch.wrapping_add(1);
            if let Some(id) = focused {
                ctx.memory_mut(|memory| memory.surrender_focus(id));
            }
            self.audio.owned_focus = false;
            return true;
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F9)) {
            self.audio.cancel_input();
            self.audio.pause();
            self.toggle_views();
            return true;
        }
        if ctx.text_edit_focused() {
            for key in [egui::Key::S, egui::Key::O, egui::Key::E] {
                if ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, key)) {
                    self.audio.epoch = self.audio.epoch.wrapping_add(1);
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
            self.audio.looping = !self.audio.looping;
            self.audio.apply_preferences();
            return true;
        }
        if ctx.input(|input| input.modifiers.command || input.modifiers.ctrl || input.modifiers.alt)
        {
            return false;
        }
        for key in [egui::Key::Space, egui::Key::D] {
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key)) {
                self.audio.toggle_play();
            }
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::A)) {
            self.audio.pause();
        }
        for (key, time) in [(egui::Key::Q, 0.), (egui::Key::E, self.audio.duration())] {
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key)) {
                self.audio.cancel_input();
                self.audio.seek(time);
            }
        }
        // Slider arrows remain with that widget; only the waveform owns seek arrows.
        if focused == Some(self.audio_canvas_id()) {
            let modifiers = ctx.input(|input| input.modifiers);
            for (key, delta) in [(egui::Key::ArrowLeft, -0.1), (egui::Key::ArrowRight, 0.1)] {
                if ctx.input_mut(|input| input.consume_key(modifiers, key)) {
                    let step = if modifiers.shift { 10. } else { 1. };
                    self.audio.cancel_input();
                    self.audio.seek(self.audio.position() + delta * step);
                }
            }
            // Canvas arrows must not also navigate focus into the Volume slider.
            for key in [egui::Key::ArrowUp, egui::Key::ArrowDown] {
                ctx.input_mut(|input| input.consume_key(modifiers, key));
            }
        }
        true
    }

    fn audio_canvas_id(&self) -> egui::Id {
        egui::Id::new(("session-audio-waveform", self.audio.epoch))
    }
    fn audio_control(&mut self, name: &str, response: &egui::Response) {
        self.record_target(name, response.rect);
        self.audio.controls.push(response.id);
        if response.clicked() {
            response.request_focus();
        }
    }

    pub(super) fn audio_panel(&mut self, ui: &mut egui::Ui) {
        ui.push_id(("session-audio", self.audio.epoch), |ui| {
            self.audio_contents(ui)
        });
    }
    fn audio_contents(&mut self, ui: &mut egui::Ui) {
        self.audio.controls.clear();
        self.record_target("audioPanelClip", ui.clip_rect());
        ui.horizontal(|ui| {
            let open=ui.button("Load WAV").on_hover_text("PCM8/16/24/32 or float32, mono/stereo, 8–192 kHz; maximum64 MiB. Session-only; source files are unchanged.");
            self.audio_control("audioLoad",&open);
            if open.clicked() {
                self.audio.pause();
                if let Some(path)=rfd::FileDialog::new().add_filter("WAV audio",&["wav"]).pick_file() {self.load_audio_path(&path);}
            }
            let name=self.audio.loaded.as_ref().map_or("No audio loaded",|loaded|loaded.name.as_str());
            ui.add(egui::Label::new(name).truncate());
        });
        ui.horizontal(|ui| {
            let play = ui
                .add_enabled(
                    self.audio.can_play(),
                    egui::Button::new(if self.audio.playing() {
                        "Pause"
                    } else {
                        "Play"
                    })
                    .small(),
                )
                .on_hover_text(
                    "Audition this WAV · Space/D toggles, A pauses while Audio has focus",
                );
            self.audio_control("audioPlay", &play);
            if play.clicked() {
                self.audio.toggle_play();
            }
            let stop = ui.add_enabled(
                self.audio.loaded.is_some(),
                egui::Button::new("Stop").small(),
            );
            self.audio_control("audioStop", &stop);
            if stop.clicked() {
                self.audio.stop();
            }
            let looping = ui.checkbox(&mut self.audio.looping, "Loop");
            self.audio_control("audioLoop", &looping);
            if looping.changed() {
                self.audio.apply_preferences();
            }
            let fit = ui.add_enabled(
                self.audio.loaded.is_some(),
                egui::Button::new("Fit").small(),
            );
            self.audio_control("audioFit", &fit);
            if fit.clicked() {
                self.audio.fit();
            }
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().slider_width = (ui.available_width() - 175.).max(30.);
            let mut percent = self.audio.volume * 100.;
            let volume = ui.add(
                egui::Slider::new(&mut percent, 0.0..=100.0)
                    .text("Volume")
                    .suffix("%")
                    .integer(),
            );
            self.audio_control("audioVolume", &volume);
            if volume.changed() && percent.is_finite() {
                self.audio.volume = (percent / 100.).clamp(0., 1.);
                self.audio.apply_preferences();
            }
            let muted = ui.checkbox(&mut self.audio.muted, "Mute");
            self.audio_control("audioMute", &muted);
            if muted.changed() {
                self.audio.apply_preferences();
            }
        });
        let metadata = self.audio.loaded.as_ref().map_or_else(
            || "Load a WAV to inspect its waveform".into(),
            |loaded| {
                format!(
                    "{:.3} / {:.3} s · {} kHz · {}",
                    loaded.position,
                    loaded.clip.duration(),
                    loaded.clip.sample_rate as f32 / 1000.,
                    if loaded.clip.channels == 1 {
                        "mono"
                    } else {
                        "stereo"
                    }
                )
            },
        );
        ui.add(egui::Label::new(RichText::new(metadata).small().color(MUTED)).truncate());
        ui.label(
            RichText::new("Session-only WAV · no event timelines")
                .small()
                .color(MUTED),
        );
        if self
            .audio
            .loaded
            .as_ref()
            .is_some_and(|loaded| loaded.player.is_none())
        {
            ui.label(
                RichText::new("Playback is available on macOS in this build.")
                    .small()
                    .color(MUTED),
            );
        }
        if let Some(error) = &self.audio.error {
            ui.label(RichText::new(error).small().color(ACCENT));
        }
        self.audio_waveform(ui);
        if let Some(id) = ui.memory(|memory| memory.focused())
            && let Some(response) = ui.ctx().read_response(id)
            && ui.min_rect().contains_rect(response.rect)
        {
            self.audio.controls.push(id);
        }
    }

    fn audio_waveform(&mut self, ui: &mut egui::Ui) {
        let height = (ui.clip_rect().bottom() - ui.cursor().top()).max(80.);
        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), height));
        let response = ui
            .interact(rect, self.audio_canvas_id(), Sense::click_and_drag())
            .on_hover_text(
                "Summary waveform · click/drag to seek · wheel zoom · right/middle drag pan",
            );
        self.audio_control("audioWaveform", &response);
        // Register canvas-owned arrows before egui's next begin-pass navigation.
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                response.id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..Default::default()
                },
            )
        });
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0., Color32::from_gray(39));
        let Some(loaded) = &self.audio.loaded else {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No waveform",
                FontId::proportional(11.),
                MUTED,
            );
            return;
        };
        let duration = loaded.clip.duration();
        let min_span = 1. / f64::from(loaded.clip.sample_rate);
        self.audio.span = self.audio.span.clamp(min_span, duration);
        self.audio.start = self
            .audio
            .start
            .clamp(0., (duration - self.audio.span).max(0.));
        if response.hovered() && self.audio.scrub.is_none() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll.is_finite() && scroll != 0. {
                let fraction = ui
                    .input(|input| input.pointer.hover_pos())
                    .map_or(0.5, |point| {
                        ((point.x - rect.left()) / rect.width()).clamp(0., 1.)
                    }) as f64;
                let anchor = self.audio.start + self.audio.span * fraction;
                self.audio.span = (self.audio.span * (-f64::from(scroll) * 0.002).exp())
                    .clamp(min_span, duration);
                self.audio.start = (anchor - self.audio.span * fraction)
                    .clamp(0., (duration - self.audio.span).max(0.));
            }
        }
        if response.dragged_by(egui::PointerButton::Secondary)
            || response.dragged_by(egui::PointerButton::Middle)
        {
            self.audio.start = (self.audio.start
                - f64::from(response.drag_delta().x / rect.width()) * self.audio.span)
                .clamp(0., (duration - self.audio.span).max(0.));
        }
        let amplitude = loaded.clip.peaks.iter().fold(1.0_f32, |scale, peak| {
            scale.max(peak.min.abs()).max(peak.max.abs())
        });
        let middle = rect.center().y;
        painter.line_segment(
            [pos2(rect.left(), middle), pos2(rect.right(), middle)],
            Stroke::new(1., Color32::from_gray(66)),
        );
        for (index, peak) in loaded.clip.peaks.iter().enumerate() {
            let first = index as f64 * duration / loaded.clip.peaks.len() as f64;
            let last = (index + 1) as f64 * duration / loaded.clip.peaks.len() as f64;
            if last < self.audio.start || first > self.audio.start + self.audio.span {
                continue;
            }
            let fraction =
                |time: f64| ((time - self.audio.start) / self.audio.span).clamp(0., 1.) as f32;
            let left = rect.left() + rect.width() * fraction(first);
            let right = rect.left() + rect.width() * fraction(last);
            let y = |value: f32| middle - (value / amplitude) * (rect.height() - 26.) * 0.45;
            let top = y(peak.max);
            let bottom = y(peak.min);
            painter.rect_filled(
                egui::Rect::from_min_max(
                    pos2(left, top),
                    pos2(
                        (right.max(left + 1.)).min(rect.right()),
                        bottom.max(top + 1.),
                    ),
                ),
                0.,
                MINT,
            );
        }
        for index in 0..=4 {
            let fraction = index as f32 / 4.;
            painter.text(
                pos2(rect.left() + rect.width() * fraction, rect.bottom() - 3.),
                if index == 0 {
                    egui::Align2::LEFT_BOTTOM
                } else if index == 4 {
                    egui::Align2::RIGHT_BOTTOM
                } else {
                    egui::Align2::CENTER_BOTTOM
                },
                format!(
                    "{:.2}s",
                    self.audio.start + self.audio.span * f64::from(fraction)
                ),
                FontId::monospace(10.),
                MUTED,
            );
        }
        let cursor = (loaded.position - self.audio.start) / self.audio.span;
        if (0.0..=1.0).contains(&cursor) {
            let x = rect.left() + rect.width() * cursor as f32;
            painter.line_segment(
                [pos2(x, rect.top()), pos2(x, rect.bottom() - 16.)],
                Stroke::new(1., Color32::from_rgb(91, 199, 224)),
            );
            self.capture_targets.insert(
                "audioCursor".into(),
                [x - 1., rect.top(), x + 1., rect.bottom() - 16.],
            );
        }
        if self.audio.blocked {
            return;
        }
        if ui.input(|input| input.pointer.primary_pressed())
            && let Some(pointer) = ui.input(|input| input.pointer.press_origin())
            && rect.contains(pointer)
            && ui.clip_rect().contains(pointer)
        {
            self.audio.pause();
            self.audio.scrub = Some(Scrub {
                left: rect.left(),
                width: rect.width(),
                start: self.audio.start,
                span: self.audio.span,
            });
            response.request_focus();
        }
        if let Some(scrub) = self.audio.scrub
            && let Some(pointer) = ui.input(|input| input.pointer.interact_pos())
        {
            let fraction = f64::from(((pointer.x - scrub.left) / scrub.width).clamp(0., 1.));
            self.audio.seek(scrub.start + scrub.span * fraction);
            if !ui.input(|input| input.pointer.primary_down()) {
                self.audio.scrub = None;
            }
        }
    }

    pub(super) fn audio_diagnostic(&self) -> serde_json::Value {
        serde_json::json!({"visible":self.audio_visible(),"loaded":self.audio.loaded.as_ref().map(|loaded|serde_json::json!({"name":loaded.name,"sampleRate":loaded.clip.sample_rate,"channels":loaded.clip.channels,"frames":loaded.clip.frames,"duration":loaded.clip.duration(),"waveformBins":loaded.clip.peaks.len(),"playbackAvailable":loaded.player.is_some()})),
            "position":self.audio.position(),"playing":self.audio.playing(),"volume":self.audio.volume,"muted":self.audio.muted,"looping":self.audio.looping,
            "start":self.audio.start,"span":self.audio.span,"scrubbing":self.audio.scrub.is_some(),"blocked":self.audio.blocked,"error":self.audio.error})
    }
}

#[cfg(test)]
#[path = "audio_tests.rs"]
mod tests;

#[cfg(test)]
#[test]
fn blocking_modal_pause_retains_the_clip_preferences_and_main_history_even_on_pause_failure() {
    use std::{cell::RefCell, rc::Rc};
    struct State {
        playing: bool,
        position: f64,
        gain: f32,
        fail_pause: bool,
    }
    struct Player(Rc<RefCell<State>>);
    impl AudioPlayer for Player {
        fn position(&self) -> f64 {
            self.0.borrow().position
        }
        fn is_playing(&self) -> bool {
            self.0.borrow().playing
        }
        fn play(&mut self) -> Result<(), String> {
            self.0.borrow_mut().playing = true;
            Ok(())
        }
        fn pause(&mut self) -> Result<(), String> {
            let mut state = self.0.borrow_mut();
            if state.fail_pause {
                return Err("Owned output pause failed".into());
            }
            state.playing = false;
            Ok(())
        }
        fn stop(&mut self) -> Result<(), String> {
            let mut state = self.0.borrow_mut();
            state.playing = false;
            state.position = 0.;
            Ok(())
        }
        fn seek(&mut self, seconds: f64) -> Result<(), String> {
            self.0.borrow_mut().position = seconds;
            Ok(())
        }
        fn set_volume(&mut self, gain: f32) {
            self.0.borrow_mut().gain = gain;
        }
        fn set_looping(&mut self, _looping: bool) {}
    }
    let mut bytes = b"RIFF".to_vec();
    bytes.extend(16_036u32.to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(8_000u32.to_le_bytes());
    bytes.extend(16_000u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(16_000u32.to_le_bytes());
    bytes.resize(16_044, 0);
    for fail_pause in [false, true] {
        let mut editor = super::test_support::editor_fixture();
        let mut pose = editor.selected_transform().unwrap();
        pose.x += 3.;
        editor.edit_transform(pose, &[Property::X]);
        let document = editor.session.document().clone();
        let pending = editor.session.pose_edits().clone();
        let undo = editor.session.can_undo();
        let redo = editor.session.can_redo();
        let output = Rc::new(RefCell::new(State {
            playing: true,
            position: 0.25,
            gain: 1.,
            fail_pause,
        }));
        editor.audio.install(
            "Original silent WAV".into(),
            files::decode_wav(bytes.clone()).unwrap(),
            Some(Box::new(Player(output.clone()))),
        );
        editor.audio.volume = 0.37;
        editor.audio.looping = true;
        editor.audio.apply_preferences();
        editor.audio.poll();
        editor.audio.start = 0.1;
        editor.audio.span = 0.5;
        editor.audio.blocked = false;
        editor.audio.scrub = Some(Scrub {
            left: 0.,
            width: 100.,
            start: 0.1,
            span: 0.5,
        });
        editor.audio.controls.push(egui::Id::new("owned-volume"));
        editor.audio.owned_focus = true;
        let epoch = editor.audio.epoch;
        editor.pause_audio_for_modal();
        editor.audio.poll(); // Returning from a cancelled dialog never auto-resumes.
        assert!(!output.borrow().playing && !editor.audio.playing());
        assert_eq!(editor.audio.loaded.as_ref().unwrap().clip.bytes, bytes);
        assert_eq!(
            editor.audio.loaded.as_ref().unwrap().name,
            "Original silent WAV"
        );
        assert_eq!(editor.audio.volume, 0.37);
        assert!(!editor.audio.muted && editor.audio.looping);
        assert_eq!((editor.audio.start, editor.audio.span), (0.1, 0.5));
        assert!(
            editor.audio.scrub.is_none()
                && editor.audio.blocked
                && editor.audio.controls.is_empty()
        );
        assert!(!editor.audio.owned_focus && editor.audio.epoch > epoch);
        assert_eq!(editor.session.document(), &document);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(
            (editor.session.can_undo(), editor.session.can_redo()),
            (undo, redo)
        );
        assert!(editor.error.is_none());
        if fail_pause {
            assert!(editor.audio.error.is_some());
            assert_eq!(output.borrow().gain, 0.);
        } else {
            assert!(editor.audio.error.is_none());
            assert_eq!(editor.audio.position(), 0.25);
        }
    }
}
