use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
use super::*;

fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..4 {
        workspace_frame(editor, ctx, vec![]);
    }
}

fn point(editor: &Editor, target: &str) -> Pos2 {
    let [left, top, right, bottom] = editor.capture_targets[target];
    pos2((left + right) * 0.5, (top + bottom) * 0.5)
}

fn pointer(pos: Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}

fn chord(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
    [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers,
        })
        .collect()
}

fn click(editor: &mut Editor, ctx: &egui::Context, target: &str) {
    let pos = point(editor, target);
    for pressed in [true, false] {
        workspace_frame(editor, ctx, pointer(pos, pressed));
    }
    settle(editor, ctx);
}

fn graph_editor() -> Editor {
    let mut editor = editor_fixture();
    let mut document = editor.session.document().clone();
    let track = document.animations[0]
        .tracks
        .iter_mut()
        .find(|track| track.bone == 8 && track.property == Property::Rotation)
        .unwrap();
    track.keys = vec![
        Keyframe {
            time: 0.,
            value: -20.,
            curve: Curve::Linear,
        },
        Keyframe {
            time: 0.5,
            value: 10.,
            curve: Curve::Bezier([0.25, -0.2, 0.75, 1.2]),
        },
        Keyframe {
            time: 1.,
            value: 40.,
            curve: Curve::Stepped,
        },
    ];
    editor.session = Session::new(document).unwrap();
    editor.show_tree = false;
    editor.show_dopesheet = false;
    editor.show_timeline = false;
    editor.open_graph_view();
    editor
}

fn rotation_key(editor: &Editor, time: f32) -> Keyframe {
    editor.session.document().animations[0]
        .tracks
        .iter()
        .find(|track| track.bone == 8 && track.property == Property::Rotation)
        .unwrap()
        .keys
        .iter()
        .find(|key| key.time == time)
        .copied()
        .unwrap()
}

fn start_drag(editor: &mut Editor, ctx: &egui::Context, time: f32, pixels: f32) -> (Pos2, Pos2) {
    settle(editor, ctx);
    let from = point(editor, &format!("graphKey:{time}"));
    let to = from - vec2(0., pixels);
    workspace_frame(editor, ctx, pointer(from, true));
    workspace_frame(editor, ctx, vec![egui::Event::PointerMoved(to)]);
    (from, to)
}

#[test]
fn value_drag_preserves_exact_times_curves_pending_pose_and_one_step_history() {
    let mut editor = graph_editor();
    let ctx = editor_context();
    let mut pose = editor.selected_transform().unwrap();
    pose.rotation += 9.;
    editor.edit_transform(pose, &[Property::Rotation]);
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let time = editor.time;
    let selected = editor.selection.clone();
    let (_, first) = start_drag(&mut editor, &ctx, 0.5, 20.);
    assert!(editor.graph_edit_active());
    let edit = editor.graph.edit.as_ref().unwrap();
    let expected = (f64::from(edit.key.value)
        + 35. / f64::from(edit.rect.height()) * (edit.bounds.value_max - edit.bounds.value_min))
        as f32;
    let final_pointer = first - vec2(0., 15.);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(final_pointer)],
    );
    assert_eq!(rotation_key(&editor, 0.5).value, expected);
    workspace_frame(&mut editor, &ctx, pointer(final_pointer, false));
    settle(&mut editor, &ctx);
    assert!(!editor.graph_edit_active());
    let edited = editor.session.document().clone();
    let original_keys = &original.animations[0].tracks[0].keys;
    let new_keys = &edited.animations[0].tracks[0].keys;
    assert_eq!(original_keys.len(), new_keys.len());
    for (old, new) in original_keys.iter().zip(new_keys) {
        assert_eq!(old.time.to_bits(), new.time.to_bits());
        assert_eq!(old.curve, new.curve);
        if old.time != 0.5 {
            assert_eq!(old.value.to_bits(), new.value.to_bits());
        }
    }
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.time, time);
    assert_eq!(editor.selection, selected);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    editor.redo();
    assert_eq!(editor.session.document(), &edited);
    assert_eq!(editor.session.pose_edits(), &pending);
}

#[test]
fn exact_picker_and_zoom_allow_independent_edit_of_close_native_keys() {
    let mut editor = graph_editor();
    let ctx = editor_context();
    let mut document = editor.session.document().clone();
    document.animations[0].tracks[0].keys = vec![
        Keyframe {
            time: 0.,
            value: -10.,
            curve: Curve::Linear,
        },
        Keyframe {
            time: 0.3,
            value: 0.,
            curve: Curve::Stepped,
        },
        Keyframe {
            time: 0.300005,
            value: 2.,
            curve: Curve::Bezier([0.2, 0.1, 0.8, 0.9]),
        },
        Keyframe {
            time: 1.,
            value: 10.,
            curve: Curve::Linear,
        },
    ];
    editor.session = Session::new(document).unwrap();
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "graphKeyPicker");
    click(&mut editor, &ctx, "graphKeyChoice:2");
    assert_eq!(editor.graph.selected, Some(0.300005));
    click(&mut editor, &ctx, "graphZoomKeys");
    let first = point(&editor, "graphKey:0.3");
    let second = point(&editor, "graphKey:0.300005");
    assert!((first.x - second.x).abs() > 20.);
    let original = editor.session.document().clone();
    let (_, to) = start_drag(&mut editor, &ctx, 0.300005, 15.);
    workspace_frame(&mut editor, &ctx, pointer(to, false));
    settle(&mut editor, &ctx);
    assert_eq!(
        rotation_key(&editor, 0.3),
        original.animations[0].tracks[0].keys[1]
    );
    let changed = rotation_key(&editor, 0.300005);
    assert!(changed.value > 2.);
    assert_eq!(changed.time, 0.300005);
    assert_eq!(
        changed.curve,
        original.animations[0].tracks[0].keys[2].curve
    );
    assert_eq!(
        editor.session.document().animations[0].tracks[0].keys.len(),
        4
    );
}

#[test]
fn escape_and_document_undo_cancel_a_held_preview_without_reentry_or_redo_loss() {
    let mut editor = graph_editor();
    let ctx = editor_context();
    editor
        .session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo survives".into(),
        })
        .unwrap();
    editor.undo();
    let original = editor.session.document().clone();
    assert!(editor.session.can_redo());
    let (_, to) = start_drag(&mut editor, &ctx, 0.5, 20.);
    assert_ne!(editor.session.document(), &original);
    workspace_frame(
        &mut editor,
        &ctx,
        chord(egui::Key::Escape, egui::Modifiers::NONE),
    );
    assert!(!editor.graph_edit_active());
    assert_eq!(editor.session.document(), &original);
    assert!(editor.graph.blocked);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(to - vec2(0., 30.))],
    );
    assert_eq!(editor.session.document(), &original);
    workspace_frame(&mut editor, &ctx, pointer(to, false));
    settle(&mut editor, &ctx);
    assert!(editor.session.can_redo());
    let (_, to) = start_drag(&mut editor, &ctx, 0.5, 20.);
    workspace_frame(
        &mut editor,
        &ctx,
        chord(egui::Key::Z, egui::Modifiers::COMMAND),
    );
    assert!(!editor.graph_edit_active());
    assert_eq!(editor.session.document(), &original);
    workspace_frame(&mut editor, &ctx, pointer(to, false));
    settle(&mut editor, &ctx);
    assert!(editor.session.can_redo());
}

#[test]
fn graph_focus_never_deletes_pastes_keys_or_nudges_the_main_bone() {
    let mut editor = graph_editor();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    let original = editor.session.document().clone();
    let pose = editor.session.pose_edits().clone();
    ctx.memory_mut(|memory| memory.request_focus(Editor::graph_id()));
    for (key, modifiers) in [
        (egui::Key::Delete, egui::Modifiers::NONE),
        (egui::Key::Backspace, egui::Modifiers::NONE),
        (egui::Key::ArrowUp, egui::Modifiers::NONE),
        (egui::Key::K, egui::Modifiers::NONE),
        (egui::Key::L, egui::Modifiers::NONE),
        (egui::Key::D, egui::Modifiers::NONE),
        (egui::Key::V, egui::Modifiers::COMMAND),
    ] {
        workspace_frame(&mut editor, &ctx, chord(key, modifiers));
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pose);
        assert!(!editor.playing);
    }
}

#[test]
fn typed_context_changes_and_view_hide_cancel_before_any_retargeting() {
    let mut editor = graph_editor();
    let ctx = editor_context();
    let original = editor.session.document().clone();
    let (_, to) = start_drag(&mut editor, &ctx, 0.5, 20.);
    editor.select(TreeItem::Slot(0));
    settle(&mut editor, &ctx);
    assert!(!editor.graph_edit_active());
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.capture_targets.contains_key("graphCanvas"));
    workspace_frame(&mut editor, &ctx, pointer(to, false));
    editor.select(TreeItem::Bone(8));
    settle(&mut editor, &ctx);
    let (_, to) = start_drag(&mut editor, &ctx, 0.5, 20.);
    editor.toggle_views();
    settle(&mut editor, &ctx);
    assert!(!editor.graph_edit_active());
    assert_eq!(editor.session.document(), &original);
    workspace_frame(&mut editor, &ctx, pointer(to, false));
    editor.toggle_views();
    settle(&mut editor, &ctx);
    assert!(editor.graph_visible());
    let (_, to) = start_drag(&mut editor, &ctx, 0.5, 20.);
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
            focused: false,
            ..Default::default()
        },
        |ui| editor.workspace_ui(ui),
    );
    output.textures_delta.clear();
    assert!(!editor.graph_edit_active());
    assert_eq!(editor.session.document(), &original);
    workspace_frame(&mut editor, &ctx, pointer(to, false));
}

#[test]
fn engine_sampling_and_bounded_dense_rendering_keep_all_document_keys() {
    let mut editor = graph_editor();
    let ctx = editor_context();
    let mut document = editor.session.document().clone();
    document.animations[0].tracks[0].keys = (0..10000)
        .map(|index| Keyframe {
            time: index as f32 / 1000.,
            value: (index % 17) as f32,
            curve: Curve::Linear,
        })
        .collect();
    editor.session = Session::new(document).unwrap();
    let original = editor.session.document().clone();
    settle(&mut editor, &ctx);
    let track = &original.animations[0].tracks[0];
    for time in [0., 0.0005, 0.5, 1., 9.9985, 12.] {
        assert_eq!(
            graph_sample(track, time),
            peachy_engine::sample(track, time)
        );
    }
    assert!(editor.graph.rendered_keys <= KEY_DRAW_LIMIT + 1);
    assert!(editor.graph.rendered_points <= 1024);
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
    let mut document = original;
    document.animations[0].tracks[0].keys = vec![
        Keyframe {
            time: 0.,
            value: -f32::MAX,
            curve: Curve::Linear,
        },
        Keyframe {
            time: f32::MAX,
            value: f32::MAX,
            curve: Curve::Linear,
        },
    ];
    editor.session = Session::new(document).unwrap();
    editor.graph.reset_document();
    settle(&mut editor, &ctx);
    assert!(editor.graph.error.is_none());
    assert!(editor.graph.bounds.unwrap().valid());
    editor.graph.selected = Some(f32::MAX);
    editor.fit_graph(editor.graph_context().unwrap(), true);
    settle(&mut editor, &ctx);
    assert!(editor.graph.error.is_none());
    assert!(editor.graph.bounds.unwrap().valid());
    assert!(
        editor
            .capture_targets
            .values()
            .flatten()
            .all(|value| value.is_finite())
    );
}

#[test]
fn every_supported_property_is_discoverable_and_missing_tracks_are_read_only() {
    let mut editor = graph_editor();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    let original = editor.session.document().clone();
    click(&mut editor, &ctx, "graphPropertyPicker");
    for property in Property::ALL {
        assert!(
            editor
                .capture_targets
                .contains_key(&format!("graphProperty:{property:?}"))
        );
    }
    click(&mut editor, &ctx, "graphProperty:ShearY");
    assert_eq!(editor.graph.property, Property::ShearY);
    assert!(!editor.capture_targets.contains_key("graphCanvas"));
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
}
