//! Real workspace events distinguish requested seeds from whole affected tracks.
use super::test_support::{editor_context, editor_fixture, workspace_frame};
use super::*;
use peachy_engine::AnimationKey;

fn seed(bone: usize, property: Property, time: f32) -> AnimationKey {
    AnimationKey::Bone {
        bone,
        property,
        time,
    }
}

fn fixture() -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    let mut doc = editor.session.document().clone();
    doc.fps = 60;
    let track = |bone, property, times: &[f32], values: &[f32]| Track {
        bone,
        property,
        keys: times
            .iter()
            .zip(values)
            .map(|(&time, &value)| Keyframe {
                time,
                value,
                curve: Curve::Linear,
            })
            .collect(),
    };
    doc.animations[0] = Animation::new(
        "offset",
        vec![
            track(8, Property::Y, &[0., 0.2, 0.6, 1.], &[8., 32., -16., 8.]),
            track(
                7,
                Property::Rotation,
                &[0., 0.35, 0.8, 1.],
                &[15., -25., 45., 15.],
            ),
        ],
    );
    editor.session = Session::new(doc).unwrap();
    editor.session.set_skin(&editor.skin);
    editor.key_selection = Default::default();
    editor.selection = Default::default();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    (editor, ctx)
}

fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..3 {
        workspace_frame(editor, ctx, vec![]);
    }
}
fn point(editor: &Editor, name: &str) -> Pos2 {
    let [left, top, right, bottom] = *editor
        .capture_targets
        .get(name)
        .unwrap_or_else(|| panic!("Missing target {name}: {:?}", editor.capture_targets.keys()));
    assert!(right > left && bottom > top, "{name}");
    pos2((left + right) * 0.5, (top + bottom) * 0.5)
}
fn pointer(
    editor: &mut Editor,
    ctx: &egui::Context,
    at: Pos2,
    pressed: bool,
    modifiers: egui::Modifiers,
) {
    workspace_frame(
        editor,
        ctx,
        vec![
            egui::Event::PointerMoved(at),
            egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers,
            },
        ],
    );
}
fn click(editor: &mut Editor, ctx: &egui::Context, name: &str, modifiers: egui::Modifiers) {
    let at = point(editor, name);
    workspace_frame(editor, ctx, vec![egui::Event::PointerMoved(at)]);
    pointer(editor, ctx, at, true, modifiers);
    pointer(editor, ctx, at, false, modifiers);
}
fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}
fn enable(editor: &mut Editor, ctx: &egui::Context) {
    click(editor, ctx, "timelineOffset", egui::Modifiers::NONE);
    assert!(editor.cycle_offset.enabled);
}
fn select_single(editor: &mut Editor, ctx: &egui::Context) {
    click(editor, ctx, "timelineKey:bone:8:Y:1", egui::Modifiers::NONE);
    assert_eq!(editor.key_selection.keys(), &[seed(8, Property::Y, 0.2)]);
}
fn preview(
    editor: &mut Editor,
    ctx: &egui::Context,
    target: &str,
    frames: f32,
    modifiers: egui::Modifiers,
) -> (Pos2, Pos2) {
    let origin = point(editor, target);
    let [left, _, right, _] = editor.capture_targets["timelineTrack"];
    let end = origin + vec2((right - left) * frames / 60. / editor.display_extent(), 0.);
    workspace_frame(editor, ctx, vec![egui::Event::PointerMoved(origin)]);
    pointer(editor, ctx, origin, true, modifiers);
    workspace_frame(editor, ctx, vec![egui::Event::PointerMoved(end)]);
    (origin, end)
}
fn pending_and_redo(editor: &mut Editor) -> peachy_engine::PoseEdits {
    let mut transform = editor.session.document().bones[6].setup;
    transform.x = 199.;
    editor
        .session
        .edit_pose(
            6,
            EditTarget::Animate {
                animation: 0,
                time: editor.time,
            },
            transform,
            &[Property::X],
            false,
        )
        .unwrap();
    editor
        .session
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "redo witness".into(),
        })
        .unwrap();
    editor.session.undo();
    assert!(editor.session.can_redo());
    editor.session.pose_edits().clone()
}

#[test]
fn cycle_offset_ui_single_seed_two_previews_select_only_mapped_seed_and_commit_once() {
    let (mut editor, ctx) = fixture();
    select_single(&mut editor, &ctx);
    enable(&mut editor, &ctx);
    let pending = pending_and_redo(&mut editor);
    let before = editor.session.document().clone();
    let (origin, first) = preview(
        &mut editor,
        &ctx,
        "timelineKey:bone:8:Y:1",
        3.,
        egui::Modifiers::NONE,
    );
    assert!(editor.key_drag.as_ref().is_some_and(|drag| drag.active));
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(origin + (first - origin) * 2.)],
    );
    assert!(editor.error.is_none(), "{:?}", editor.error);
    let receipt = editor.cycle_offset.receipt.as_ref().unwrap();
    assert_eq!(receipt.mappings.len(), 1);
    let accepted = receipt.mappings[0].target.clone().unwrap();
    assert_eq!(editor.key_selection.keys(), std::slice::from_ref(&accepted));
    assert_eq!(editor.key_selection.primary(), Some(&accepted));
    assert_eq!((accepted.time() * 60.).round(), 18.);
    assert_eq!(receipt.keys.len(), 5);
    assert_eq!(receipt.boundary_keys.len(), 2);
    assert_eq!(
        editor.session.document().animations[0].tracks[1],
        before.animations[0].tracks[1]
    );
    assert_eq!(editor.session.pose_edits(), &pending);
    pointer(
        &mut editor,
        &ctx,
        origin + (first - origin) * 2.,
        false,
        egui::Modifiers::NONE,
    );
    let after = editor.session.document().clone();
    assert!(editor.key_drag.is_none());
    assert!(!editor.session.can_redo());
    editor.undo();
    assert_eq!(editor.session.document(), &before);
    assert_eq!(editor.session.pose_edits(), &pending);
    editor.redo();
    assert_eq!(editor.session.document(), &after);
    assert_eq!(editor.session.pose_edits(), &pending);
}

#[test]
fn cycle_offset_ui_escape_restores_mixed_seeds_pending_redo_and_blocks_held_reentry() {
    let (mut editor, ctx) = fixture();
    select_single(&mut editor, &ctx);
    click(
        &mut editor,
        &ctx,
        "timelineKey:bone:7:Rotation:1",
        egui::Modifiers::COMMAND,
    );
    enable(&mut editor, &ctx);
    let pending = pending_and_redo(&mut editor);
    let before = editor.session.document().clone();
    let selected = editor.key_selection.keys().to_vec();
    let primary = editor.key_selection.primary().cloned();
    let (_, held) = preview(
        &mut editor,
        &ctx,
        "timelineKey:bone:8:Y:1",
        12.,
        egui::Modifiers::NONE,
    );
    assert_ne!(editor.session.document(), &before);
    assert_eq!(editor.key_selection.len(), 2);
    assert_eq!(editor.cycle_offset.receipt.as_ref().unwrap().keys.len(), 9);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
    );
    assert_eq!(editor.session.document(), &before);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.key_selection.keys(), selected);
    assert_eq!(editor.key_selection.primary(), primary.as_ref());
    assert!(editor.session.can_redo());
    assert!(editor.cycle_offset.blocked);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::PointerMoved(held + vec2(25., 0.)),
            egui::Event::PointerButton {
                pos: held,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    assert!(editor.cycle_offset.blocked);
    assert!(editor.key_drag.is_none());
    assert_eq!(editor.session.document(), &before);
    pointer(&mut editor, &ctx, held, false, egui::Modifiers::NONE);
    assert!(!editor.cycle_offset.blocked);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.key_selection.keys(), selected);
}

#[test]
fn cycle_offset_ui_negative_wrap_is_not_clamped_like_ordinary_retiming() {
    let (mut editor, ctx) = fixture();
    select_single(&mut editor, &ctx);
    enable(&mut editor, &ctx);
    let before = editor.session.document().clone();
    let (_, held) = preview(
        &mut editor,
        &ctx,
        "timelineKey:bone:8:Y:1",
        -18.,
        egui::Modifiers::NONE,
    );
    assert!(editor.error.is_none(), "{:?}", editor.error);
    assert!(editor.key_drag.as_ref().unwrap().offset < -0.2);
    assert_eq!(editor.key_selection.len(), 1);
    assert_eq!((editor.key_selection.keys()[0].time() * 60.).round(), 54.);
    pointer(&mut editor, &ctx, held, false, egui::Modifiers::NONE);
    editor.undo();
    assert_eq!(editor.session.document(), &before);
}

#[test]
fn cycle_offset_ui_copy_modifier_refuses_without_mutating_selection_pending_or_redo() {
    let (mut editor, ctx) = fixture();
    select_single(&mut editor, &ctx);
    enable(&mut editor, &ctx);
    let pending = pending_and_redo(&mut editor);
    let before = editor.session.document().clone();
    let selected = editor.key_selection.keys().to_vec();
    let (_, held) = preview(
        &mut editor,
        &ctx,
        "timelineKey:bone:8:Y:1",
        6.,
        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
    );
    assert!(editor.key_drag.is_none());
    assert!(editor.cycle_offset.blocked);
    assert!(editor.status.contains("Copying keys"));
    assert_eq!(editor.session.document(), &before);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.key_selection.keys(), selected);
    assert!(editor.session.can_redo());
    pointer(&mut editor, &ctx, held, false, egui::Modifiers::NONE);
}

#[test]
fn cycle_offset_ui_changed_frozen_context_or_focus_cancels_without_publishing() {
    for change in 0..6 {
        let (mut editor, ctx) = fixture();
        select_single(&mut editor, &ctx);
        enable(&mut editor, &ctx);
        let pending = pending_and_redo(&mut editor);
        let before = editor.session.document().clone();
        let (_, held) = preview(
            &mut editor,
            &ctx,
            "timelineKey:bone:8:Y:1",
            6.,
            egui::Modifiers::NONE,
        );
        assert_ne!(editor.session.document(), &before);
        match change {
            0 => editor.setup_mode = true,
            1 => editor.animation = 1,
            2 => editor.skin = "default".into(),
            3 => editor.cycle_offset.enabled = false,
            4 => {
                editor.cycle_offset.bounds.insert(0, [None, Some(0.8)]);
            }
            _ => {}
        }
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
                focused: change != 5,
                ..Default::default()
            },
            |ui| editor.workspace_ui(ui),
        );
        output.textures_delta.clear();
        assert!(editor.key_drag.is_none(), "context {change}");
        assert_eq!(editor.session.document(), &before, "context {change}");
        assert_eq!(editor.session.pose_edits(), &pending, "context {change}");
        assert!(editor.session.can_redo(), "context {change}");
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(held + vec2(30., 0.))],
        );
        assert_eq!(editor.session.document(), &before, "context {change}");
        pointer(&mut editor, &ctx, held, false, egui::Modifiers::NONE);
    }
}

#[test]
fn cycle_offset_ui_save_cancels_preview_keeps_pending_and_blocks_continued_hold() {
    let (mut editor, ctx) = fixture();
    select_single(&mut editor, &ctx);
    enable(&mut editor, &ctx);
    let pending = pending_and_redo(&mut editor);
    let before = editor.session.document().clone();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "peachy-cycle-offset-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    editor.project_path = Some(directory.join("project.peachy"));
    editor.asset_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
    let (_, held) = preview(
        &mut editor,
        &ctx,
        "timelineKey:bone:8:Y:1",
        6.,
        egui::Modifiers::NONE,
    );
    assert_ne!(editor.session.document(), &before);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::S, egui::Modifiers::COMMAND)],
    );
    assert!(editor.error.is_none(), "{:?}", editor.error);
    assert_eq!(editor.session.document(), &before);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(editor.cycle_offset.blocked);
    assert_eq!(
        files::load(editor.project_path.as_ref().unwrap()).unwrap(),
        before
    );
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(held + vec2(30., 0.))],
    );
    assert_eq!(editor.session.document(), &before);
    pointer(&mut editor, &ctx, held, false, egui::Modifiers::NONE);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn cycle_offset_ui_minimum_toolbar_exposes_blank_automatic_bounds_without_data_changes() {
    let (mut editor, ctx) = fixture();
    let before = editor.session.document().clone();
    let selected = editor.key_selection.keys().to_vec();
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
    let [left, top, right, bottom] = editor.capture_targets["dopesheetClip"];
    let clip = egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom));
    let mut previous = None;
    for target in [
        "timelineOffset",
        "timelineLoopStart",
        "timelineLoopEnd",
        "closeView:Dopesheet",
    ] {
        let [left, top, right, bottom] = editor.capture_targets[target];
        let rect = egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom));
        assert!(clip.contains_rect(rect), "{target}: {rect:?}, {clip:?}");
        assert!(
            rect.width()
                >= if target.starts_with("timelineLoop") {
                    40.
                } else {
                    20.
                },
            "{target}: {rect:?}"
        );
        if let Some(previous) = previous {
            assert!(left >= previous, "overlap: {target}");
        }
        previous = Some(right);
    }
    assert_eq!(
        editor.cycle_offset_diagnostic()["bounds"],
        serde_json::json!({"start":null,"end":null})
    );
    assert_eq!(
        editor.cycle_offset_diagnostic()["effectiveRange"],
        serde_json::json!({"start":0.,"end":1.})
    );
    assert_eq!(editor.session.document(), &before);
    assert_eq!(editor.key_selection.keys(), selected);
    assert!(!editor.session.can_undo());
}

#[test]
fn cycle_offset_ui_loop_bounds_accept_explicit_default_and_clear_back_to_automatic() {
    let (mut editor, ctx) = fixture();
    let pending = pending_and_redo(&mut editor);
    let before = editor.session.document().clone();
    click(&mut editor, &ctx, "timelineLoopEnd", egui::Modifiers::NONE);
    workspace_frame(&mut editor, &ctx, vec![]);
    assert!(
        ctx.text_edit_focused(),
        "Blank automatic End must be editable"
    );
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            key(egui::Key::A, egui::Modifiers::COMMAND),
            egui::Event::Text("60".into()),
            key(egui::Key::Enter, egui::Modifiers::NONE),
        ],
    );
    assert_eq!(editor.cycle_offset.bounds.get(&0).unwrap()[1], Some(1.));
    click(&mut editor, &ctx, "timelineLoopEnd", egui::Modifiers::NONE);
    workspace_frame(&mut editor, &ctx, vec![]);
    assert!(ctx.text_edit_focused());
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            key(egui::Key::A, egui::Modifiers::COMMAND),
            key(egui::Key::Backspace, egui::Modifiers::NONE),
            key(egui::Key::Enter, egui::Modifiers::NONE),
        ],
    );
    assert_eq!(editor.cycle_offset.bounds.get(&0).unwrap()[1], None);
    assert_eq!(editor.session.document(), &before);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(editor.session.can_redo());
}

fn release_key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers,
    }
}

fn first_offset(editor: &mut Editor, ctx: &egui::Context) {
    select_single(editor, ctx);
    enable(editor, ctx);
    let (_, held) = preview(
        editor,
        ctx,
        "timelineKey:bone:8:Y:1",
        6.,
        egui::Modifiers::NONE,
    );
    assert!(editor.error.is_none(), "{:?}", editor.error);
    pointer(editor, ctx, held, false, egui::Modifiers::NONE);
    settle(editor, ctx);
    assert_eq!(
        editor.session.document().animations[0].tracks[0].keys.len(),
        5
    );
}

#[test]
fn cycle_offset_ui_generated_bounds_have_no_hit_and_repeated_select_all_keeps_real_anchors() {
    let (mut editor, ctx) = fixture();
    first_offset(&mut editor, &ctx);
    let before = editor.session.document().clone();
    let history = (editor.session.can_undo(), editor.session.can_redo());
    let memory = editor.session.cycle_offset_memory().clone();
    assert!(
        !editor
            .capture_targets
            .contains_key("timelineKey:bone:8:Y:0")
    );
    assert!(
        !editor
            .capture_targets
            .contains_key("timelineKey:bone:8:Y:4")
    );
    assert!(
        editor
            .capture_targets
            .contains_key("timelineKey:bone:8:Y:2")
    );
    let [left, _, right, _] = editor.capture_targets["timelineTrack"];
    let y = point(&editor, "timelineKey:bone:8:Y:2").y;
    let generated = pos2(left, y);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(generated)],
    );
    pointer(&mut editor, &ctx, generated, true, egui::Modifiers::NONE);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(
            generated + vec2((right - left) * 0.1, 0.),
        )],
    );
    assert!(editor.key_drag.is_none());
    assert_eq!(editor.session.document(), &before);
    pointer(&mut editor, &ctx, generated, false, egui::Modifiers::NONE);
    assert_eq!(editor.session.cycle_offset_memory(), &memory);
    assert_eq!(
        (editor.session.can_undo(), editor.session.can_redo()),
        history
    );
    click(
        &mut editor,
        &ctx,
        "timelineKey:bone:8:Y:2",
        egui::Modifiers::NONE,
    );
    workspace_frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::A, egui::Modifiers::COMMAND)],
    );
    assert_eq!(editor.key_selection.len(), 3);
    assert!(
        editor
            .key_selection
            .keys()
            .iter()
            .all(|key| matches!(key, AnimationKey::Bone { bone: 8, .. }))
    );
    workspace_frame(
        &mut editor,
        &ctx,
        vec![release_key(egui::Key::A, egui::Modifiers::COMMAND)],
    );
    workspace_frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::A, egui::Modifiers::COMMAND)],
    );
    assert_eq!(editor.key_selection.len(), 7);
    assert_eq!(
        editor
            .key_selection
            .keys()
            .iter()
            .filter(|key| matches!(key, AnimationKey::Bone { bone: 8, .. }))
            .count(),
        3
    );
    assert_eq!(
        editor
            .key_selection
            .keys()
            .iter()
            .filter(|key| matches!(key, AnimationKey::Bone { bone: 7, .. }))
            .count(),
        4
    );
    assert!(!editor.key_selection.contains(seed(8, Property::Y, 0.)));
    assert!(!editor.key_selection.contains(seed(8, Property::Y, 1.)));
    assert_eq!(editor.session.document(), &before);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![release_key(egui::Key::A, egui::Modifiers::COMMAND)],
    );
    click(&mut editor, &ctx, "timelineOffset", egui::Modifiers::NONE);
    settle(&mut editor, &ctx);
    assert!(
        editor
            .capture_targets
            .contains_key("timelineKey:bone:8:Y:0")
    );
    assert!(
        editor
            .capture_targets
            .contains_key("timelineKey:bone:8:Y:4")
    );
}

#[test]
fn cycle_offset_ui_off_on_clears_history_restored_memory_and_recaptures_the_carried_seam() {
    let (mut editor, ctx) = fixture();
    first_offset(&mut editor, &ctx);
    let shifted = editor.session.document().clone();
    let (_, held) = preview(
        &mut editor,
        &ctx,
        "timelineKey:bone:8:Y:2",
        6.,
        egui::Modifiers::NONE,
    );
    assert!(editor.error.is_none(), "{:?}", editor.error);
    pointer(&mut editor, &ctx, held, false, egui::Modifiers::NONE);
    let twice_shifted = editor.session.document().clone();
    assert_ne!(twice_shifted, shifted);
    let history = (
        editor.session.can_undo(),
        editor.session.can_redo(),
        editor.session.is_dirty(),
    );
    click(&mut editor, &ctx, "timelineOffset", egui::Modifiers::NONE);
    assert!(!editor.cycle_offset.enabled);
    assert!(editor.session.cycle_offset_memory().is_empty());
    assert_eq!(editor.session.document(), &twice_shifted);
    assert_eq!(
        (
            editor.session.can_undo(),
            editor.session.can_redo(),
            editor.session.is_dirty()
        ),
        history
    );
    editor.undo();
    assert_eq!(editor.session.document(), &shifted);
    assert!(!editor.cycle_offset.enabled);
    assert!(!editor.session.cycle_offset_memory().is_empty());
    // Redo restores the second commit with the empty memory captured by Off.
    editor.redo();
    assert_eq!(editor.session.document(), &twice_shifted);
    assert!(editor.session.cycle_offset_memory().is_empty());
    editor.undo();
    assert!(!editor.cycle_offset.enabled);
    assert!(!editor.session.cycle_offset_memory().is_empty());
    assert_eq!(editor.session.document(), &shifted);
    let pending = pending_and_redo(&mut editor);
    let history = (
        editor.session.can_undo(),
        editor.session.can_redo(),
        editor.session.is_dirty(),
    );
    click(&mut editor, &ctx, "timelineOffset", egui::Modifiers::NONE);
    assert!(editor.cycle_offset.enabled);
    assert!(editor.session.cycle_offset_memory().is_empty());
    assert_eq!(editor.session.document(), &shifted);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(
        (
            editor.session.can_undo(),
            editor.session.can_redo(),
            editor.session.is_dirty()
        ),
        history
    );
    click(
        &mut editor,
        &ctx,
        "timelineKey:bone:8:Y:2",
        egui::Modifiers::COMMAND,
    );
    let (_, held) = preview(
        &mut editor,
        &ctx,
        "timelineKey:bone:8:Y:2",
        6.,
        egui::Modifiers::NONE,
    );
    assert!(editor.error.is_none(), "{:?}", editor.error);
    pointer(&mut editor, &ctx, held, false, egui::Modifiers::NONE);
    assert_eq!(
        editor.session.document().animations[0].tracks[0].keys.len(),
        6
    );
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.key_selection.len(), 1);
    let after = editor.session.document().clone();
    editor.undo();
    assert_eq!(editor.session.document(), &shifted);
    assert_eq!(editor.session.pose_edits(), &pending);
    editor.redo();
    assert_eq!(editor.session.document(), &after);
}

#[test]
fn cycle_offset_ui_stale_mask_refuses_without_side_effects_and_off_on_recovers() {
    let (mut editor, ctx) = fixture();
    first_offset(&mut editor, &ctx);
    let target = point(&editor, "timelineKey:bone:8:Y:2");
    let mut changed = editor.session.document().animations[0].tracks[0].keys[2];
    changed.value += 1.;
    assert!(editor.dispatch(Command::SetKey {
        animation: 0,
        bone: 8,
        property: Property::Y,
        key: changed
    }));
    let pending = pending_and_redo(&mut editor);
    let before = editor.session.document().clone();
    let selected = editor.key_selection.keys().to_vec();
    settle(&mut editor, &ctx);
    assert!(
        editor.status.contains("Turn Offset off and on"),
        "{}",
        editor.status
    );
    assert!(editor.error.is_none());
    assert!(
        !editor
            .capture_targets
            .contains_key("timelineKey:bone:8:Y:2")
    );
    pointer(&mut editor, &ctx, target, true, egui::Modifiers::NONE);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(target + vec2(40., 0.))],
    );
    pointer(&mut editor, &ctx, target, false, egui::Modifiers::NONE);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::A, egui::Modifiers::COMMAND)],
    );
    assert_eq!(editor.session.document(), &before);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.key_selection.keys(), selected);
    assert!(editor.session.can_redo());
    assert!(editor.status.contains("Turn Offset off and on"));
    workspace_frame(
        &mut editor,
        &ctx,
        vec![release_key(egui::Key::A, egui::Modifiers::COMMAND)],
    );
    click(&mut editor, &ctx, "timelineOffset", egui::Modifiers::NONE);
    click(&mut editor, &ctx, "timelineOffset", egui::Modifiers::NONE);
    settle(&mut editor, &ctx);
    assert!(
        editor
            .capture_targets
            .contains_key("timelineKey:bone:8:Y:2")
    );
    assert!(editor.session.cycle_offset_memory().is_empty());
    assert_eq!(editor.session.document(), &before);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(editor.session.can_redo());
    assert!(!editor.status.contains("Turn Offset off and on"));
}

#[test]
fn cycle_offset_ui_dopesheet_surface_keeps_controls_when_graph_is_open_and_active() {
    let (mut editor, ctx) = fixture();
    editor.view_tabs.open.insert(views::DockView::Graph);
    editor.view_tabs.animation_view = views::AnimationView::Graph;
    settle(&mut editor, &ctx);
    assert!(editor.graph_visible());
    assert!(!editor.capture_targets.contains_key("timelineOffset"));
    // Exercise the actual Dopesheet surface with Graph still active: controls
    // belong to the rendered surface, independent of the shared view flag.
    for _ in 0..3 {
        editor.capture_targets.clear();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1120., 740.))),
                focused: true,
                ..Default::default()
            },
            |ui| editor.timeline(ui),
        );
        output.textures_delta.clear();
    }
    for name in [
        "timelineOffset",
        "timelineLoopStart",
        "timelineLoopEnd",
        "closeView:Dopesheet",
    ] {
        let [left, top, right, bottom] = editor.capture_targets[name];
        assert!(
            left >= 0. && top >= 0. && right <= 1120. && bottom <= 740.,
            "{name}: {:?}",
            editor.capture_targets[name]
        );
    }
    editor.view_tabs.animation_view = views::AnimationView::Dopesheet;
    settle(&mut editor, &ctx);
    enable(&mut editor, &ctx);
    assert!(editor.view_tabs.open.contains(&views::DockView::Graph));
    assert!(editor.dopesheet_visible());
}

fn coalesced_move_release(offset: bool) {
    let (mut editor, ctx) = fixture();
    select_single(&mut editor, &ctx);
    if offset {
        enable(&mut editor, &ctx);
    }
    let pending = pending_and_redo(&mut editor);
    let before = editor.session.document().clone();
    let origin = point(&editor, "timelineKey:bone:8:Y:1");
    let [left, _, right, _] = editor.capture_targets["timelineTrack"];
    let end = origin + vec2((right - left) * 0.1 / editor.display_extent(), 0.);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(origin)]);
    pointer(&mut editor, &ctx, origin, true, egui::Modifiers::NONE);
    assert!(editor.key_drag.as_ref().is_some_and(|drag| !drag.active));
    assert_eq!(editor.session.document(), &before);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::PointerMoved(end),
            egui::Event::PointerButton {
                pos: end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
            // A subsequent hover must not replace the owned release coordinate.
            egui::Event::PointerMoved(end + vec2(25., 0.)),
        ],
    );
    assert!(editor.error.is_none(), "{:?}", editor.error);
    assert!(
        editor.key_drag.is_none(),
        "Coalesced release left a dangling owner"
    );
    assert_ne!(editor.session.document(), &before);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.time, 0.2);
    assert_eq!(editor.key_selection.len(), 1);
    let selected = editor.key_selection.keys()[0].clone();
    assert_eq!((selected.time() * 60.).round(), 18.);
    let track = &editor.session.document().animations[0].tracks[0];
    assert_eq!(track.keys.len(), if offset { 5 } else { 4 });
    assert!(
        track
            .keys
            .iter()
            .any(|key| key.time == selected.time() && key.value == 32.)
    );
    assert_eq!(
        editor.session.document().animations[0].tracks[1],
        before.animations[0].tracks[1]
    );
    assert!(!editor.session.can_redo());
    let after = editor.session.document().clone();
    editor.undo();
    assert_eq!(editor.session.document(), &before);
    assert_eq!(editor.session.pose_edits(), &pending);
    editor.redo();
    assert_eq!(editor.session.document(), &after);
}

#[test]
fn cycle_offset_ui_offset_move_and_release_in_one_frame_publish_and_finish_once() {
    coalesced_move_release(true);
}

#[test]
fn cycle_offset_ui_ordinary_retime_move_and_release_in_one_frame_publish_and_finish_once() {
    coalesced_move_release(false);
}

#[test]
fn cycle_offset_ui_small_coalesced_modified_release_keeps_click_semantics_in_both_modes() {
    for offset in [false, true] {
        for modifiers in [
            egui::Modifiers::COMMAND,
            egui::Modifiers::CTRL,
            egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
        ] {
            let (mut editor, ctx) = fixture();
            select_single(&mut editor, &ctx);
            if offset {
                enable(&mut editor, &ctx);
            }
            let pending = pending_and_redo(&mut editor);
            let before = editor.session.document().clone();
            let history = (editor.session.can_undo(), editor.session.can_redo());
            let origin = point(&editor, "timelineKey:bone:7:Rotation:1");
            workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(origin)]);
            pointer(&mut editor, &ctx, origin, true, modifiers);
            workspace_frame(
                &mut editor,
                &ctx,
                vec![
                    egui::Event::PointerMoved(origin + vec2(1., 0.)),
                    egui::Event::ModifiersChanged(egui::Modifiers::NONE),
                    egui::Event::PointerButton {
                        pos: origin + vec2(1., 0.),
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            assert!(editor.key_drag.is_none());
            assert!(editor.error.is_none(), "{:?}", editor.error);
            assert_eq!(
                editor.key_selection.keys(),
                &[seed(8, Property::Y, 0.2), seed(7, Property::Rotation, 0.35)]
            );
            assert_eq!(editor.time, 0.2);
            assert_eq!(editor.session.document(), &before);
            assert_eq!(editor.session.pose_edits(), &pending);
            assert_eq!(
                (editor.session.can_undo(), editor.session.can_redo()),
                history
            );
            assert!(editor.cycle_offset.receipt.is_none());
        }
    }
}
