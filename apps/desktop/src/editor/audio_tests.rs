use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
use super::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct Output {
    position: f64,
    playing: bool,
    gain: f32,
    looping: bool,
    fail_play: bool,
    calls: Vec<&'static str>,
}
struct Mock(Rc<RefCell<Output>>);
impl AudioPlayer for Mock {
    fn position(&self) -> f64 {
        self.0.borrow().position
    }
    fn is_playing(&self) -> bool {
        self.0.borrow().playing
    }
    fn play(&mut self) -> Result<(), String> {
        let mut output = self.0.borrow_mut();
        output.calls.push("play");
        if output.fail_play {
            return Err("Mock audio device unavailable".into());
        }
        output.playing = true;
        Ok(())
    }
    fn pause(&mut self) -> Result<(), String> {
        let mut output = self.0.borrow_mut();
        output.calls.push("pause");
        output.playing = false;
        Ok(())
    }
    fn stop(&mut self) -> Result<(), String> {
        let mut output = self.0.borrow_mut();
        output.calls.push("stop");
        output.playing = false;
        output.position = 0.;
        Ok(())
    }
    fn seek(&mut self, seconds: f64) -> Result<(), String> {
        let mut output = self.0.borrow_mut();
        output.calls.push("seek");
        output.position = seconds;
        Ok(())
    }
    fn set_volume(&mut self, gain: f32) {
        let mut output = self.0.borrow_mut();
        output.calls.push("volume");
        output.gain = gain;
    }
    fn set_looping(&mut self, looping: bool) {
        let mut output = self.0.borrow_mut();
        output.calls.push("loop");
        output.looping = looping;
    }
}
impl Drop for Mock {
    fn drop(&mut self) {
        let mut output = self.0.borrow_mut();
        output.calls.push("drop");
        output.playing = false;
    }
}
fn clip(frames: u32) -> WavClip {
    let rate = 8_000u32;
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + frames * 2).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(rate.to_le_bytes());
    bytes.extend((rate * 2).to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((frames * 2).to_le_bytes());
    for frame in 0..frames {
        bytes.extend(
            if frame.is_multiple_of(2) {
                10_000i16
            } else {
                -10_000i16
            }
            .to_le_bytes(),
        );
    }
    files::decode_wav(bytes).unwrap()
}
fn main_state(editor: &Editor) -> serde_json::Value {
    serde_json::json!({"document":editor.session.document(),"pending":editor.session.pose_edits(),
        "selection":editor.selection.as_ref().map(TreeItem::key),"animation":editor.animation,"skin":editor.skin,
        "time":editor.time,"playing":editor.playing,"direction":editor.play_direction,"looping":editor.looping,
        "speed":editor.playback_speed,"tool":editor.tool as u8,"undo":editor.session.can_undo(),"redo":editor.session.can_redo(),"dirty":editor.session.is_dirty()})
}
fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..3 {
        workspace_frame(editor, ctx, vec![]);
    }
}
fn fixture() -> (Editor, egui::Context, Rc<RefCell<Output>>) {
    let mut editor = editor_fixture();
    let mut pose = editor.selected_transform().unwrap();
    pose.rotation += 13.;
    editor.edit_transform(pose, &[Property::Rotation]);
    let output = Rc::new(RefCell::new(Output::default()));
    editor.audio.install(
        "Original tone.wav".into(),
        clip(8_000),
        Some(Box::new(Mock(output.clone()))),
    );
    editor.open_audio_view();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    (editor, ctx, output)
}
fn point(editor: &Editor, name: &str) -> Pos2 {
    let [l, t, r, b] = editor.capture_targets[name];
    pos2((l + r) * 0.5, (t + b) * 0.5)
}
fn pointer(editor: &mut Editor, ctx: &egui::Context, point: Pos2, pressed: bool) {
    workspace_frame(
        editor,
        ctx,
        vec![
            egui::Event::PointerMoved(point),
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
}
fn click(editor: &mut Editor, ctx: &egui::Context, name: &str) {
    let p = point(editor, name);
    for down in [true, false] {
        pointer(editor, ctx, p, down);
    }
    settle(editor, ctx);
}
fn key(editor: &mut Editor, ctx: &egui::Context, key: egui::Key, modifiers: egui::Modifiers) {
    for pressed in [true, false] {
        workspace_frame(
            editor,
            ctx,
            vec![egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed,
                repeat: false,
                modifiers,
            }],
        );
    }
}

#[test]
fn audio_controls_use_the_mock_native_clock_and_leave_main_pose_and_history_unchanged() {
    let (mut editor, ctx, output) = fixture();
    let main = main_state(&editor);
    click(&mut editor, &ctx, "audioPlay");
    assert!(output.borrow().playing);
    output.borrow_mut().position = 0.35;
    settle(&mut editor, &ctx);
    assert_eq!(editor.audio.position(), 0.35);
    for _ in 0..12 {
        workspace_frame(&mut editor, &ctx, vec![]);
    }
    assert_eq!(
        editor.audio.position(),
        0.35,
        "egui dt must not simulate audio progress"
    );
    click(&mut editor, &ctx, "audioPlay");
    assert!(!output.borrow().playing);
    click(&mut editor, &ctx, "audioLoop");
    assert!(output.borrow().looping);
    let [left, top, _, bottom] = editor.capture_targets["audioVolume"];
    let p = pos2(left + 25., (top + bottom) * 0.5);
    for pressed in [true, false] {
        pointer(&mut editor, &ctx, p, pressed);
    }
    assert!(editor.audio.volume < 1.);
    assert_eq!(output.borrow().gain, editor.audio.volume);
    click(&mut editor, &ctx, "audioMute");
    assert_eq!(output.borrow().gain, 0.);
    click(&mut editor, &ctx, "audioMute");
    assert_eq!(output.borrow().gain, editor.audio.volume);
    let [left, top, right, _] = editor.capture_targets["audioWaveform"];
    let p = pos2(left + (right - left) * 0.25, top + 20.);
    for pressed in [true, false] {
        pointer(&mut editor, &ctx, p, pressed);
    }
    assert!((output.borrow().position - 0.25).abs() < 0.001);
    click(&mut editor, &ctx, "audioStop");
    assert_eq!(output.borrow().position, 0.);
    assert_eq!(main_state(&editor), main);
}

#[test]
fn audio_focus_owns_transport_and_escape_without_deselecting_or_nudging_the_main_editor() {
    let (mut editor, ctx, output) = fixture();
    let main = main_state(&editor);
    click(&mut editor, &ctx, "audioWaveform");
    for name in [
        egui::Key::C,
        egui::Key::V,
        egui::Key::X,
        egui::Key::Z,
        egui::Key::G,
        egui::Key::Delete,
        egui::Key::ArrowUp,
    ] {
        key(&mut editor, &ctx, name, egui::Modifiers::NONE);
    }
    let time = editor.audio.position();
    key(
        &mut editor,
        &ctx,
        egui::Key::ArrowRight,
        egui::Modifiers::NONE,
    );
    assert!((editor.audio.position() - time - 0.1).abs() < 1e-6);
    key(&mut editor, &ctx, egui::Key::R, egui::Modifiers::COMMAND);
    assert!(output.borrow().looping);
    key(&mut editor, &ctx, egui::Key::Space, egui::Modifiers::NONE);
    assert!(output.borrow().playing);
    key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(!output.borrow().playing);
    assert!(!editor.audio.owned_focus);
    assert_eq!(main_state(&editor), main);
    // A later main-control click clears ownership; its Escape must not pause audio.
    click(&mut editor, &ctx, "audioWaveform");
    key(&mut editor, &ctx, egui::Key::Space, egui::Modifiers::NONE);
    assert!(output.borrow().playing);
    click(&mut editor, &ctx, "rotateTool");
    assert!(!editor.audio.owned_focus);
    key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(output.borrow().playing);
}

#[test]
fn failed_replacement_retains_the_clip_and_successful_replacement_blocks_a_stale_held_seek() {
    let (mut editor, ctx, output) = fixture();
    let main = main_state(&editor);
    click(&mut editor, &ctx, "audioPlay");
    let bytes = editor.audio.loaded.as_ref().unwrap().clip.bytes.clone();
    let missing = std::env::temp_dir().join(format!(
        "peachy-missing-audio-{}-{}.wav",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    editor.audio.load(&missing);
    assert_eq!(editor.audio.loaded.as_ref().unwrap().clip.bytes, bytes);
    assert!(output.borrow().playing && editor.audio.error.is_some());
    editor.audio.error = None;
    let start = point(&editor, "audioWaveform");
    pointer(&mut editor, &ctx, start, true);
    assert!(editor.audio.scrub.is_some());
    let replacement = Rc::new(RefCell::new(Output::default()));
    editor.audio.install(
        "Replacement.wav".into(),
        clip(4_000),
        Some(Box::new(Mock(replacement.clone()))),
    );
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(start + vec2(25., 0.))],
    );
    assert_eq!(replacement.borrow().position, 0.);
    assert!(editor.audio.blocked && editor.audio.scrub.is_none());
    assert!(!output.borrow().playing && output.borrow().calls.contains(&"drop"));
    pointer(&mut editor, &ctx, start, false);
    settle(&mut editor, &ctx);
    assert!(!editor.audio.blocked);
    assert_eq!(editor.audio.duration(), 0.5);
    assert_eq!(main_state(&editor), main);
}

#[test]
fn hide_focus_modal_and_document_reset_pause_or_stop_only_owned_audio() {
    for action in ["close", "f9", "focus", "modal", "reset"] {
        let (mut editor, ctx, output) = fixture();
        let main = main_state(&editor);
        click(&mut editor, &ctx, "audioPlay");
        assert!(output.borrow().playing);
        match action {
            "close" => click(&mut editor, &ctx, "closeView:Audio"),
            "f9" => {
                click(&mut editor, &ctx, "audioWaveform");
                key(&mut editor, &ctx, egui::Key::Space, egui::Modifiers::NONE);
                key(&mut editor, &ctx, egui::Key::F9, egui::Modifiers::NONE);
            }
            "focus" => {
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        focused: false,
                        screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
                        ..Default::default()
                    },
                    |ui| editor.workspace_ui(ui),
                );
                out.textures_delta.clear();
            }
            "modal" => {
                editor.error = Some("test modal".into());
                workspace_frame(&mut editor, &ctx, vec![]);
            }
            "reset" => editor.audio.reset_document(),
            _ => unreachable!(),
        }
        assert!(!output.borrow().playing, "{action}");
        if action == "reset" {
            assert!(editor.audio.loaded.is_none());
            assert!(output.borrow().calls.contains(&"drop"));
        } else {
            assert!(editor.audio.loaded.is_some());
        }
        assert_eq!(main_state(&editor), main, "{action}");
    }
}

#[test]
fn audio_errors_stay_local_and_invalid_output_positions_do_not_become_null_diagnostics() {
    let (mut editor, ctx, output) = fixture();
    let main = main_state(&editor);
    output.borrow_mut().fail_play = true;
    click(&mut editor, &ctx, "audioPlay");
    assert!(editor.audio.error.is_some() && editor.error.is_none());
    assert!(!output.borrow().playing);
    output.borrow_mut().fail_play = false;
    output.borrow_mut().position = 0.25;
    settle(&mut editor, &ctx);
    output.borrow_mut().position = f64::NAN;
    workspace_frame(&mut editor, &ctx, vec![]);
    assert_eq!(editor.audio.position(), 0.25);
    assert!(
        editor.audio_diagnostic()["position"]
            .as_f64()
            .unwrap()
            .is_finite()
    );
    assert!(editor.audio.error.is_some() && editor.error.is_none());
    assert_eq!(main_state(&editor), main);
}

#[test]
fn waveform_zoom_pan_fit_and_minimum_dock_controls_are_bounded_and_session_only() {
    let (mut editor, ctx, _) = fixture();
    let main = main_state(&editor);
    for _ in 0..3 {
        let mut out = ctx.run_ui(
            egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1120., 740.))),
                ..Default::default()
            },
            |ui| editor.workspace_ui(ui),
        );
        out.textures_delta.clear();
    }
    let [left, top, right, bottom] = editor.capture_targets["audioPanelClip"];
    for name in [
        "audioLoad",
        "audioPlay",
        "audioStop",
        "audioLoop",
        "audioVolume",
        "audioMute",
        "audioFit",
        "audioWaveform",
    ] {
        let [l, t, r, b] = editor.capture_targets[name];
        assert!([l, t, r, b].into_iter().all(f32::is_finite), "{name}");
        assert!(
            l >= left && r <= right + 1. && t >= top && b <= bottom + 1.,
            "{name} outside clip: {:?} {:?}",
            [l, t, r, b],
            [left, top, right, bottom]
        );
    }
    settle(&mut editor, &ctx);
    let p = point(&editor, "audioWaveform");
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::PointerMoved(p),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0., 200.),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    assert!(editor.audio.span < editor.audio.duration());
    let button = |position, pressed| egui::Event::PointerButton {
        pos: position,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(p), button(p, true)],
    );
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(p - vec2(20., 0.))],
    );
    workspace_frame(&mut editor, &ctx, vec![button(p - vec2(20., 0.), false)]);
    assert!(editor.audio.start > 0.);
    click(&mut editor, &ctx, "audioFit");
    assert_eq!(editor.audio.start, 0.);
    assert_eq!(editor.audio.span, editor.audio.duration());
    assert_eq!(main_state(&editor), main);
}

#[test]
fn waveform_inspection_without_a_native_backend_has_no_simulated_playback() {
    let mut editor = editor_fixture();
    let main = main_state(&editor);
    editor
        .audio
        .install("Original waveform.wav".into(), clip(8_000), None);
    editor.open_audio_view();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    assert!(!editor.audio.can_play());
    click(&mut editor, &ctx, "audioPlay");
    assert!(!editor.audio.playing());
    let [left, top, right, _] = editor.capture_targets["audioWaveform"];
    let p = pos2(left + (right - left) * 0.75, top + 20.);
    for pressed in [true, false] {
        pointer(&mut editor, &ctx, p, pressed);
    }
    assert!((editor.audio.position() - 0.75).abs() < 0.001);
    settle(&mut editor, &ctx);
    assert!((editor.audio.position() - 0.75).abs() < 0.001);
    assert_eq!(
        editor.audio_diagnostic()["loaded"]["playbackAvailable"],
        false
    );
    assert_eq!(main_state(&editor), main);
}
