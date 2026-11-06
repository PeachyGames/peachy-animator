use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
use super::*;

fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..4 {
        workspace_frame(editor, ctx, vec![]);
    }
}
fn point(editor: &Editor, name: &str) -> Pos2 {
    let [left, top, right, bottom] = editor.capture_targets[name];
    pos2((left + right) * 0.5, (top + bottom) * 0.5)
}
fn pointer(
    editor: &mut Editor,
    ctx: &egui::Context,
    pos: Pos2,
    pressed: bool,
    button: egui::PointerButton,
    modifiers: egui::Modifiers,
) {
    workspace_frame(
        editor,
        ctx,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers,
            },
        ],
    );
}
fn click(editor: &mut Editor, ctx: &egui::Context, name: &str) {
    let pos = point(editor, name);
    for pressed in [true, false] {
        pointer(
            editor,
            ctx,
            pos,
            pressed,
            egui::PointerButton::Primary,
            egui::Modifiers::NONE,
        );
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
    settle(editor, ctx);
}
fn text(editor: &mut Editor, ctx: &egui::Context, value: &str) {
    click(editor, ctx, "compactFrameValue");
    assert!(ctx.text_edit_focused());
    workspace_frame(editor, ctx, vec![egui::Event::Text(value.into())]);
}
fn ruler_point(editor: &Editor, fraction: f32) -> Pos2 {
    let [left, top, right, _] = editor.capture_targets["compactRuler"];
    pos2(left + 8. + (right - left - 16.) * fraction, top + 10.)
}
fn fixture() -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    editor.show_timeline = true;
    editor.show_dopesheet = false;
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    (editor, ctx)
}

#[test]
fn compact_view_opens_independently_closes_restores_and_retains_mode_visibility() {
    let mut editor = editor_fixture();
    let ctx = editor_context();
    editor.show_dopesheet = false;
    let original = editor.session.document().clone();
    let selection = editor.selection.clone();
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "viewsMenu");
    click(&mut editor, &ctx, "viewTimeline");
    assert!(editor.show_timeline && !editor.show_dopesheet);
    assert!(editor.capture_targets.contains_key("compactRuler"));
    click(&mut editor, &ctx, "compactStart");
    assert_eq!(editor.time, 0.);
    click(&mut editor, &ctx, "compactNextFrame");
    assert_eq!(editor.time, 1. / original.fps as f32);
    click(&mut editor, &ctx, "compactPreviousFrame");
    assert_eq!(editor.time, 0.);
    click(&mut editor, &ctx, "compactNextKey");
    assert_eq!(editor.time, 0.2);
    click(&mut editor, &ctx, "compactPreviousKey");
    assert_eq!(editor.time, 0.);
    click(&mut editor, &ctx, "compactEnd");
    assert_eq!(editor.time, editor.animation_end());
    click(&mut editor, &ctx, "compactForward");
    assert!(editor.playing);
    click(&mut editor, &ctx, "compactBackward");
    assert!(editor.playing && editor.play_direction == PlayDirection::Backward);
    click(&mut editor, &ctx, "compactBackward");
    assert!(!editor.playing);
    click(&mut editor, &ctx, "compactLoop");
    assert!(!editor.looping);
    key(&mut editor, &ctx, egui::Key::F9, egui::Modifiers::NONE);
    assert!(!editor.show_timeline);
    key(&mut editor, &ctx, egui::Key::F9, egui::Modifiers::NONE);
    assert!(editor.show_timeline);
    editor.set_setup_mode(true);
    settle(&mut editor, &ctx);
    assert!(editor.show_timeline && !editor.capture_targets.contains_key("compactRuler"));
    editor.set_setup_mode(false);
    settle(&mut editor, &ctx);
    assert!(editor.capture_targets.contains_key("compactRuler"));
    click(&mut editor, &ctx, "closeView:Timeline");
    assert!(!editor.show_timeline);
    click(&mut editor, &ctx, "viewsMenu");
    click(&mut editor, &ctx, "viewTimeline");
    assert!(editor.show_timeline);
    click(&mut editor, &ctx, "viewsMenu");
    click(&mut editor, &ctx, "viewRestore");
    assert!(!editor.show_timeline && editor.show_dopesheet);
    assert_eq!(editor.selection, selection);
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
}

#[test]
fn frame_entry_accepts_subframes_and_past_end_without_creating_keys_or_selecting_them() {
    let (mut editor, ctx) = fixture();
    let original = editor.session.document().clone();
    let selected = editor.key_selection.keys().to_vec();
    text(&mut editor, &ctx, "15.5");
    assert_eq!(editor.time, 0.42);
    key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(editor.time, (15.5 / f64::from(original.fps)) as f32);
    text(&mut editor, &ctx, "90.25");
    key(&mut editor, &ctx, egui::Key::Tab, egui::Modifiers::NONE);
    assert_eq!(
        editor.time,
        (90.25 / f64::from(original.fps)) as f32,
        "text={}, entry={:?}, error={:?}, textFocus={}, focused={:?}",
        editor.timeline_view.frame_text,
        editor
            .timeline_view
            .frame_entry
            .as_ref()
            .map(|e| (e.animation, e.edited)),
        editor.timeline_view.error,
        ctx.text_edit_focused(),
        ctx.memory(|m| m.focused())
    );
    assert!(editor.timeline_view.range.start > 0.);
    assert!(
        editor.timeline_view.range.start + editor.timeline_view.range.span
            >= f64::from(editor.time)
    );
    assert_eq!(editor.animation_end(), 0.8);
    assert_eq!(editor.key_selection.keys(), selected);
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
}

#[test]
fn rounded_noop_invalid_text_escape_and_view_changes_preserve_pose_bits_and_redo() {
    let (mut editor, ctx) = fixture();
    editor
        .session
        .dispatch(Command::RenameBone {
            bone: 0,
            name: "redo".into(),
        })
        .unwrap();
    editor.session.undo();
    let mut pose = editor.selected_transform().unwrap();
    pose.rotation += 13.;
    editor.edit_transform(pose, &[Property::Rotation]);
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let time = editor.time;
    let history = (editor.session.can_undo(), editor.session.can_redo());
    click(&mut editor, &ctx, "compactFrameValue");
    key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(editor.time.to_bits(), time.to_bits());
    text(&mut editor, &ctx, "NaN");
    key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    assert!(editor.timeline_view.error.is_some());
    key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    assert!(editor.timeline_view.error.is_some());
    key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(editor.timeline_view.frame_entry.is_none() && editor.timeline_view.error.is_none());
    text(&mut editor, &ctx, "99");
    key(&mut editor, &ctx, egui::Key::F9, egui::Modifiers::NONE);
    assert!(!editor.show_timeline);
    key(&mut editor, &ctx, egui::Key::F9, egui::Modifiers::NONE);
    assert!(editor.show_timeline);
    text(&mut editor, &ctx, "90");
    click(&mut editor, &ctx, "closeView:Timeline");
    assert_eq!(editor.time.to_bits(), time.to_bits());
    click(&mut editor, &ctx, "viewsMenu");
    click(&mut editor, &ctx, "viewTimeline");
    text(&mut editor, &ctx, "90");
    click(&mut editor, &ctx, "compactZoomKeys");
    assert_eq!(editor.time.to_bits(), time.to_bits());
    assert_eq!(editor.time.to_bits(), time.to_bits());
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.session.document(), &original);
    assert_eq!(
        (editor.session.can_undo(), editor.session.can_redo()),
        history
    );
}

#[test]
fn scrubbing_snaps_or_preserves_shift_subframes_and_undo_recovers_pending_pose() {
    let (mut editor, ctx) = fixture();
    let original = editor.session.document().clone();
    let selected = editor.key_selection.keys().to_vec();
    let mut pose = editor.selected_transform().unwrap();
    pose.rotation += 19.;
    editor.edit_transform(pose, &[Property::Rotation]);
    let pos = ruler_point(&editor, 0.5);
    pointer(
        &mut editor,
        &ctx,
        pos,
        true,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert!(editor.timeline_view.scrub.is_some());
    assert!(editor.session.pose_edits().is_empty());
    pointer(
        &mut editor,
        &ctx,
        pos,
        false,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert_eq!(editor.time, 13. / 30.);
    assert_eq!(editor.key_selection.keys(), selected);
    key(&mut editor, &ctx, egui::Key::Z, egui::Modifiers::COMMAND);
    assert_eq!(editor.selected_transform().unwrap().rotation, pose.rotation);
    let shift = egui::Modifiers::SHIFT;
    let pos = ruler_point(&editor, 0.51);
    let expected =
        (editor.timeline_view.range.start + editor.timeline_view.range.span * 0.51) as f32;
    pointer(
        &mut editor,
        &ctx,
        pos,
        true,
        egui::PointerButton::Primary,
        shift,
    );
    assert!((editor.time - expected).abs() < 0.000001);
    pointer(
        &mut editor,
        &ctx,
        pos,
        false,
        egui::PointerButton::Primary,
        shift,
    );
    assert!((editor.time - expected).abs() < 0.000001);
    assert_eq!(editor.session.document(), &original);
    assert!(editor.key_selection.keys().is_empty()); // Document Undo clears selection by existing policy.
}

#[test]
fn scrub_map_freezes_and_escape_history_and_hide_cannot_restart_a_held_pointer() {
    let (mut editor, ctx) = fixture();
    editor.looping = false;
    let pos = ruler_point(&editor, 0.4);
    pointer(
        &mut editor,
        &ctx,
        pos,
        true,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    let map = editor.timeline_view.scrub.unwrap().map;
    let moved = ruler_point(&editor, 0.8);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(moved)]);
    assert_eq!(editor.timeline_view.scrub.unwrap().map.range, map.range);
    key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(editor.timeline_view.scrub.is_none() && editor.timeline_view.blocked);
    let time = editor.time;
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(pos)]);
    assert_eq!(editor.time, time);
    pointer(
        &mut editor,
        &ctx,
        pos,
        false,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert!(!editor.timeline_view.blocked);
    let mut pose = editor.selected_transform().unwrap();
    pose.rotation += 11.;
    editor.edit_transform(pose, &[Property::Rotation]);
    pointer(
        &mut editor,
        &ctx,
        pos,
        true,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert!(editor.session.pose_edits().is_empty());
    editor.undo();
    assert_eq!(editor.selected_transform().unwrap().rotation, pose.rotation);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(moved)]);
    assert_eq!(editor.selected_transform().unwrap().rotation, pose.rotation);
    editor.toggle_views();
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(pos)]);
    pointer(
        &mut editor,
        &ctx,
        pos,
        false,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    editor.toggle_views();
    settle(&mut editor, &ctx);
    assert_eq!(editor.selected_transform().unwrap().rotation, pose.rotation);
}

#[test]
fn loop_scrub_wraps_only_when_started_before_the_end_and_marks_all_tracks_read_only() {
    let (mut editor, ctx) = fixture();
    editor.show_dopesheet = true;
    settle(&mut editor, &ctx);
    let original = editor.session.document().clone();
    let selected = editor.key_selection.keys().to_vec();
    let start = ruler_point(&editor, 0.5);
    let past = ruler_point(&editor, 0.99);
    pointer(
        &mut editor,
        &ctx,
        start,
        true,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(past)]);
    assert!(editor.time < 0.1);
    pointer(
        &mut editor,
        &ctx,
        past,
        false,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    pointer(
        &mut editor,
        &ctx,
        past,
        true,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    pointer(
        &mut editor,
        &ctx,
        past,
        false,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert!(editor.time > editor.animation_end());
    assert_eq!(
        key_times(editor.session.document(), 0),
        vec![0., 0.2, 0.3, 0.300005, 0.4, 0.5, 0.6, 0.8]
    );
    key(&mut editor, &ctx, egui::Key::Delete, egui::Modifiers::NONE);
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.key_selection.keys(), selected);
    key(&mut editor, &ctx, egui::Key::A, egui::Modifiers::NONE);
    assert!(editor.playing && editor.play_direction == PlayDirection::Backward);
    key(&mut editor, &ctx, egui::Key::A, egui::Modifiers::NONE);
    assert!(!editor.playing);
}

#[test]
fn pan_zoom_fit_and_animation_switch_do_not_serialize_view_data_or_commit_stale_frame_text() {
    let (mut editor, ctx) = fixture();
    let original = editor.session.document().clone();
    let time = editor.time;
    let mut pose = editor.selected_transform().unwrap();
    pose.rotation += 7.;
    editor.edit_transform(pose, &[Property::Rotation]);
    let pending = editor.session.pose_edits().clone();
    let initial = editor.timeline_view.range;
    let pos = ruler_point(&editor, 0.5);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::MouseWheel {
                phase: egui::TouchPhase::Move,
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0., 120.),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    settle(&mut editor, &ctx);
    assert!(editor.timeline_view.range.span < initial.span);
    pointer(
        &mut editor,
        &ctx,
        pos,
        true,
        egui::PointerButton::Secondary,
        egui::Modifiers::NONE,
    );
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(pos - vec2(80., 0.))],
    );
    pointer(
        &mut editor,
        &ctx,
        pos - vec2(80., 0.),
        false,
        egui::PointerButton::Secondary,
        egui::Modifiers::NONE,
    );
    assert!(editor.timeline_view.range.start > 0.);
    click(&mut editor, &ctx, "compactZoomKeys");
    assert_eq!(editor.timeline_view.range, initial);
    assert_eq!(editor.time, time);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.session.document(), &original);
    text(&mut editor, &ctx, "90");
    editor.activate_animation(1);
    settle(&mut editor, &ctx);
    assert_eq!(editor.time, 0.);
    assert!(editor.timeline_view.frame_entry.is_none());
    assert_eq!(
        editor.timeline_view.range,
        TimelineRange::fit(editor.animation_end(), original.fps)
    );
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn real_save_click_cancels_uncommitted_frame_text_without_seeking_or_saving_pending_pose() {
    let (mut editor, ctx) = fixture();
    let original = editor.session.document().clone();
    let time = editor.time;
    let mut pose = editor.selected_transform().unwrap();
    pose.rotation += 9.;
    editor.edit_transform(pose, &[Property::Rotation]);
    let pending = editor.session.pose_edits().clone();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "peachy-timeline-save-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("rig.peachy");
    editor.project_path = Some(path.clone());
    editor.asset_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
    text(&mut editor, &ctx, "90");
    click(&mut editor, &ctx, "toolbarSave");
    assert!(editor.error.is_none(), "{:?}", editor.error);
    assert_eq!(editor.time, time);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(files::load(&path).unwrap(), original);
    assert!(editor.timeline_view.frame_entry.is_none());
    text(&mut editor, &ctx, "120");
    key(&mut editor, &ctx, egui::Key::S, egui::Modifiers::COMMAND);
    assert_eq!(editor.time, time);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(files::load(&path).unwrap(), original);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn modal_guards_cancel_text_and_scrub_without_executing_view_shortcuts_or_restarting() {
    let (mut editor, ctx) = fixture();
    let time = editor.time;
    let original = editor.session.document().clone();
    text(&mut editor, &ctx, "90");
    editor.error = Some("Guarded error".into());
    key(&mut editor, &ctx, egui::Key::F9, egui::Modifiers::NONE);
    assert!(editor.show_timeline && editor.timeline_view.frame_entry.is_none());
    assert_eq!(editor.time, time);
    assert_eq!(editor.session.document(), &original);
    editor.error = None;
    settle(&mut editor, &ctx);
    let pos = ruler_point(&editor, 0.4);
    pointer(
        &mut editor,
        &ctx,
        pos,
        true,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert!(editor.timeline_view.scrub.is_some());
    let time = editor.time;
    editor.error = Some("Guarded error".into());
    let moved = ruler_point(&editor, 0.8);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(moved)]);
    assert!(editor.timeline_view.scrub.is_none() && editor.timeline_view.blocked);
    editor.error = None;
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(moved)]);
    assert_eq!(editor.time, time);
    pointer(
        &mut editor,
        &ctx,
        moved,
        false,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert!(!editor.timeline_view.blocked);
}

#[test]
fn empty_and_extreme_cursors_and_both_animation_views_fit_the_minimum_workspace() {
    let (mut editor, ctx) = fixture();
    let mut document = editor.session.document().clone();
    document.animations[0].tracks.clear();
    editor.session = Session::new(document).unwrap();
    editor.timeline_view.reset_document();
    editor.show_dopesheet = true;
    for _ in 0..4 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1120., 740.))),
                focused: true,
                ..Default::default()
            },
            |ui| editor.workspace_ui(ui),
        );
        output.textures_delta.clear();
    }
    let [left, top, right, bottom] = editor.capture_targets["compactTimelineClip"];
    let clip = egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom));
    for name in [
        "compactFrameValue",
        "compactForward",
        "compactEnd",
        "compactRuler",
        "compactZoomKeys",
        "closeView:Timeline",
    ] {
        let [left, top, right, bottom] = editor.capture_targets[name];
        assert!(
            clip.contains_rect(egui::Rect::from_min_max(
                pos2(left, top),
                pos2(right, bottom)
            )),
            "{name} is clipped"
        );
    }
    assert_eq!(editor.animation_end(), 0.);
    assert!(
        editor
            .capture_targets
            .values()
            .flatten()
            .all(|value| value.is_finite())
    );
    settle(&mut editor, &ctx);
    let max_frame = f64::from(f32::MAX) * f64::from(editor.session.document().fps);
    text(&mut editor, &ctx, &max_frame.to_string());
    key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(editor.time, f32::MAX);
    assert!(editor.capture_targets.contains_key("compactPlayhead"));
    let map = TimelineMap {
        range: editor.timeline_view.range,
        left: 0.,
        width: 1000.,
    };
    assert!(map.time_at(0., 30, false).unwrap() < f32::MAX);
    assert!(
        editor
            .capture_targets
            .values()
            .flatten()
            .all(|value| value.is_finite())
    );
    assert!(editor.timeline_view_diagnostic().to_string().len() < 4000);
}
