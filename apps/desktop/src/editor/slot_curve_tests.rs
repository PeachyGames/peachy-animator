use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
use super::*;

fn fixture() -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    let mut doc = editor.session.document().clone();
    doc.animations[0].color_tracks = vec![ColorTrack {
        slot: 0,
        keys: [0.2, 0.3, 0.300005, 1.]
            .into_iter()
            .enumerate()
            .map(|(index, time)| ColorKey {
                time,
                color: [
                    [220, 180, 60, 255],
                    [180, 120, 80, 210],
                    [40, 100, 210, 110],
                    [230, 210, 190, 255],
                ][index],
                curves: [
                    Curve::Bezier([0.25, 0.2, 0.75, 0.8]),
                    Curve::Linear,
                    Curve::BezierOffset([0.3, 0.1, 0.7, -0.1]),
                    Curve::Linear,
                ],
            })
            .collect(),
    }];
    doc.animations[0].attachment_tracks = vec![AttachmentTrack {
        slot: 0,
        keys: vec![AttachmentKey {
            time: 0.3,
            attachment: None,
        }],
    }];
    editor.session = Session::new(doc).unwrap();
    editor.select(TreeItem::Slot(0));
    editor.time = 0.3;
    editor.reconcile_selection();
    editor.key_selection.replace(vec![color_key(0.3)]);
    // These regressions exercise independent handles and unchanged neighbors.
    editor.curves.separate = true;
    editor.open_graph_view();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    (editor, ctx)
}

fn color_key(time: f32) -> AnimationKey {
    AnimationKey::SlotColor { slot: 0, time }
}
fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..3 {
        workspace_frame(editor, ctx, vec![]);
    }
}
fn target(editor: &Editor, name: &str) -> Pos2 {
    let [left, top, right, bottom] = editor.capture_targets[name];
    pos2((left + right) * 0.5, (top + bottom) * 0.5)
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
    let point = target(editor, name);
    pointer(editor, ctx, point, true);
    pointer(editor, ctx, point, false);
    settle(editor, ctx);
}
fn key_event(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn slot_graph_value_handles_preserve_descending_and_equal_endpoint_curves() {
    let source = Curve::Bezier([0.2, -0.1, 0.8, 1.2]);
    let values = value_handles(source, 0.8, 0.2).unwrap();
    assert!((values[1] - 0.86).abs() < 0.000001);
    assert!((values[3] - 0.08).abs() < 0.000001);
    assert_eq!(
        curve_with_value_handles(source, values, 0.8, 0.2),
        Some(source)
    );
    let curve = curve_with_value_handles(
        Curve::Bezier([0.2, 0., 0.8, 1.]),
        [0.2, 0.8, 0.8, 0.8],
        0.5,
        0.5,
    )
    .unwrap();
    assert!(matches!(curve, Curve::BezierOffset(_)));
    assert!(peachy_engine::curve_value(curve, 0.5, 0.5, 0.5) > 0.5);
    assert!(curve_with_value_handles(source, [0., f64::INFINITY, 1., 0.], 0., 1.).is_none());
    assert!(curve_with_value_handles(source, [0., f64::MAX, 1., 0.], 0., 1.).is_none());
}

#[test]
fn slot_graph_exact_presets_preserve_pending_values_neighbors_and_grouped_step() {
    let (mut editor, _) = fixture();
    let mut pose = editor.session.local_pose(Some(0), editor.time)[8];
    pose.rotation += 7.;
    editor
        .session
        .edit_pose(
            8,
            EditTarget::Animate {
                animation: 0,
                time: editor.time,
            },
            pose,
            &[Property::Rotation],
            false,
        )
        .unwrap();
    let pending = editor.session.pose_edits().clone();
    let original = editor.session.document().clone();
    editor.set_slot_key_curve(0, 0.3, 0, Curve::Linear);
    let after = editor.session.document().clone();
    assert_eq!(
        after.animations[0].color_tracks[0].keys[1].curves[0],
        Curve::Linear
    );
    let mut expected = original.clone();
    expected.animations[0].color_tracks[0].keys[1].curves[0] = Curve::Linear;
    assert_eq!(after, expected);
    assert_eq!(editor.session.pose_edits(), &pending);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    editor.set_slot_key_curve(0, 0.3, 2, Curve::Stepped);
    assert_eq!(
        editor.session.document().animations[0].color_tracks[0].keys[1].curves,
        [Curve::Stepped; 4]
    );
    let bezier = Curve::Bezier([0.2, 0.1, 0.8, 0.9]);
    editor.set_slot_key_curve(0, 0.3, 2, bezier);
    assert_eq!(
        editor.session.document().animations[0].color_tracks[0].keys[1].curves,
        [Curve::Linear, Curve::Linear, bezier, Curve::Linear]
    );
    assert_eq!(
        editor.session.document().animations[0].color_tracks[0].keys[2],
        original.animations[0].color_tracks[0].keys[2]
    );
}

#[test]
fn slot_graph_plots_rgba_values_in_frame_time_and_selects_channel_exactly() {
    let (mut editor, ctx) = fixture();
    assert!(editor.capture_targets.contains_key("graphSlotRow"));
    assert!(!editor.capture_targets.contains_key("graphDeformRow"));
    let first = target(&editor, "graphSlotKeyPoint:0:0");
    let middle = target(&editor, "graphSlotKeyPoint:1:0");
    let last = target(&editor, "graphSlotKeyPoint:3:0");
    assert!(((middle.x - first.x) / (last.x - first.x) - 0.125).abs() < 0.001);
    assert!(
        target(&editor, "graphSlotKeyPoint:0:0").y < target(&editor, "graphSlotKeyPoint:0:2").y
    );
    assert_eq!(editor.graph.slot.rendered_intervals, 3);
    assert_eq!(editor.graph.slot.rendered_points, 3 * 4 * (SAMPLES + 1));
    let selected = target(&editor, "graphSlotKeyPoint:3:2");
    pointer(&mut editor, &ctx, selected, true);
    pointer(&mut editor, &ctx, selected, false);
    settle(&mut editor, &ctx);
    assert_eq!(editor.time, 1.);
    assert_eq!(editor.graph.slot.channel, 2);
    assert_eq!(editor.key_selection.keys(), &[color_key(1.)]);
}

#[test]
fn slot_graph_zoom_fractional_handle_drag_is_exact_frozen_and_one_undo() {
    let (mut editor, ctx) = fixture();
    click(&mut editor, &ctx, "graphZoomKeys");
    let start_key = target(&editor, "graphSlotKeyPoint:1:0");
    let next_key = target(&editor, "graphSlotKeyPoint:2:0");
    assert!(next_key.x - start_key.x > 100.);
    let original = editor.session.document().clone();
    let start = target(&editor, "graphSlotHandle:0");
    pointer(&mut editor, &ctx, start, true);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(start + vec2(18., -12.))],
    );
    assert!(editor.slot_curve_edit_active());
    pointer(&mut editor, &ctx, start + vec2(32., -24.), false);
    settle(&mut editor, &ctx);
    assert!(!editor.slot_curve_edit_active());
    let changed = editor.session.document().clone();
    assert_ne!(changed, original);
    let mut expected = original.clone();
    expected.animations[0].color_tracks[0].keys[1].curves[0] =
        changed.animations[0].color_tracks[0].keys[1].curves[0];
    assert_eq!(changed, expected);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
    editor.redo();
    assert_eq!(editor.session.document(), &changed);
}

#[test]
fn slot_graph_escape_and_owner_change_cancel_held_preview_without_rearming() {
    let (mut editor, ctx) = fixture();
    click(&mut editor, &ctx, "graphZoomKeys");
    let original = editor.session.document().clone();
    let start = target(&editor, "graphSlotHandle:0");
    pointer(&mut editor, &ctx, start, true);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(start + vec2(20., -15.))],
    );
    assert_ne!(editor.session.document(), &original);
    workspace_frame(&mut editor, &ctx, vec![key_event(egui::Key::Escape)]);
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.slot_curve_edit_active());
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(start + vec2(40., -30.))],
    );
    assert_eq!(editor.session.document(), &original);
    pointer(&mut editor, &ctx, start + vec2(40., -30.), false);
    settle(&mut editor, &ctx);
    let start = target(&editor, "graphSlotHandle:0");
    pointer(&mut editor, &ctx, start, true);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(start + vec2(20., -15.))],
    );
    editor.select(TreeItem::Bone(8));
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.slot_curve_edit_active());
}

#[test]
fn slot_graph_numeric_actual_percent_entry_preserves_unselected_channel_curves() {
    let (mut editor, ctx) = fixture();
    click(&mut editor, &ctx, "graphSlotHandles");
    for _ in 0..20 {
        workspace_frame(&mut editor, &ctx, vec![]);
    }
    let original = editor.session.document().clone();
    let value = target(&editor, "graphSlotValue:1");
    pointer(&mut editor, &ctx, value, true);
    pointer(&mut editor, &ctx, value, false);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::Key {
                key: egui::Key::A,
                physical_key: Some(egui::Key::A),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            },
            egui::Event::Key {
                key: egui::Key::A,
                physical_key: Some(egui::Key::A),
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            },
            egui::Event::Paste("25".into()),
        ],
    );
    workspace_frame(&mut editor, &ctx, vec![key_event(egui::Key::Enter)]);
    settle(&mut editor, &ctx);
    let after = editor.session.document().clone();
    let key = &after.animations[0].color_tracks[0].keys[1];
    let next = &after.animations[0].color_tracks[0].keys[2];
    let values =
        value_handles(key.curves[0], channel_value(key, 0), channel_value(next, 0)).unwrap();
    assert!((values[1] - 0.25).abs() < 0.000001);
    let mut expected = original.clone();
    expected.animations[0].color_tracks[0].keys[1].curves[0] = key.curves[0];
    assert_eq!(after, expected);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn slot_graph_numeric_escape_keeps_slot_key_channel_pending_pose_and_redo() {
    for text in ["25", "NaN"] {
        let (mut editor, ctx) = fixture();
        let mut pose = editor.session.local_pose(Some(0), editor.time)[8];
        pose.rotation += 7.;
        editor
            .session
            .edit_pose(
                8,
                EditTarget::Animate {
                    animation: 0,
                    time: editor.time,
                },
                pose,
                &[Property::Rotation],
                false,
            )
            .unwrap();
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "retained redo".into(),
            })
            .unwrap();
        let redo = editor.session.document().clone();
        editor.session.undo();
        let original = editor.session.document().clone();
        let pending = editor.session.pose_edits().clone();
        let selection = editor.selection.clone();
        let keys = editor.key_selection.keys().to_vec();
        let channel = editor.graph.slot.channel;
        click(&mut editor, &ctx, "graphSlotHandles");
        for _ in 0..20 {
            workspace_frame(&mut editor, &ctx, vec![]);
        }
        let field = target(&editor, "graphSlotValue:1");
        pointer(&mut editor, &ctx, field, true);
        pointer(&mut editor, &ctx, field, false);
        workspace_frame(
            &mut editor,
            &ctx,
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: Some(egui::Key::A),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                },
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: Some(egui::Key::A),
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                },
                egui::Event::Paste(text.into()),
            ],
        );
        assert!(ctx.text_edit_focused());
        assert_eq!(editor.curve_handle_diagnostic()["draft"]["changed"], true);
        workspace_frame(&mut editor, &ctx, vec![key_event(egui::Key::Escape)]);
        settle(&mut editor, &ctx);
        assert!(!ctx.text_edit_focused());
        assert_eq!(editor.selection, selection, "Escape changed slot selection");
        assert_eq!(editor.key_selection.keys(), keys);
        assert_eq!(editor.graph.slot.channel, channel);
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert!(editor.session.can_redo());
        // A subsequent focus/blur must not publish the discarded text.
        click(&mut editor, &ctx, "graphSlotRow");
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        editor.redo();
        assert_eq!(editor.session.document(), &redo);
        assert_eq!(editor.session.pose_edits(), &pending);
    }
}

#[test]
fn slot_graph_canvas_routes_color_only_and_mixed_clipboards_without_scalar_alias() {
    let (mut editor, ctx) = fixture();
    let original = editor.session.document().clone();
    click(&mut editor, &ctx, "graphSlotRow");
    assert!(editor.slot_graph_has_focus(&ctx));
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Copy]);
    let clipboard = editor.timeline_clipboard.clone().unwrap();
    let peachy_engine::AnimationKeyClipboard::Slot {
        ref attachment_keys,
        ref color_keys,
    } = clipboard
    else {
        panic!("Expected slot clipboard");
    };
    assert!(attachment_keys.is_empty());
    assert_eq!(color_keys.len(), 1);
    let text =
        serde_json::json!({"peachyAnimatorClipboard":"keys","version":2,"payload":clipboard})
            .to_string();
    editor.set_time(1.5);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Paste(text)]);
    assert_eq!(editor.key_selection.keys(), &[color_key(0.3)]);
    assert_eq!(
        editor.session.document().animations[0].tracks,
        original.animations[0].tracks
    );
    assert_eq!(
        editor.session.document().animations[0].attachment_tracks,
        original.animations[0].attachment_tracks
    );
    let pasted = editor.session.document().clone();
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Cut]);
    let color_keys = &editor.session.document().animations[0].color_tracks[0].keys;
    assert!(!color_keys.iter().any(|key| key.time == 0.3));
    assert!(color_keys.iter().any(|key| key.time == 1.5));
    assert_eq!(
        editor.session.document().animations[0].tracks,
        original.animations[0].tracks
    );
    assert_eq!(
        editor.session.document().animations[0].attachment_tracks,
        original.animations[0].attachment_tracks
    );
    editor.undo();
    assert_eq!(editor.session.document(), &pasted);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    editor.key_selection.replace(vec![
        color_key(0.3),
        AnimationKey::SlotAttachment { slot: 0, time: 0.3 },
    ]);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Copy]);
    let Some(peachy_engine::AnimationKeyClipboard::Slot {
        attachment_keys,
        color_keys,
    }) = editor.timeline_clipboard.as_ref()
    else {
        panic!("Expected mixed slot keys");
    };
    assert_eq!(attachment_keys.len(), 1);
    assert_eq!(color_keys.len(), 1);
}

#[test]
fn slot_graph_dense_plot_is_bounded_and_always_includes_exact_selection() {
    let (mut editor, ctx) = fixture();
    let mut doc = editor.session.document().clone();
    doc.animations[0].color_tracks[0].keys = (0..10_000)
        .map(|index| ColorKey {
            time: index as f32 * 0.01,
            color: [index as u8, 120, 170, 255],
            curves: [Curve::Linear; 4],
        })
        .collect();
    editor.session = Session::new(doc).unwrap();
    let time = editor.session.document().animations[0].color_tracks[0].keys[3456].time;
    editor.time = time;
    editor.key_selection.replace(vec![color_key(time)]);
    editor.graph.slot.bounds = None;
    settle(&mut editor, &ctx);
    assert!(editor.graph.slot.rendered_intervals <= INTERVAL_LIMIT + 1);
    assert!(editor.graph.slot.rendered_points <= (INTERVAL_LIMIT + 1) * 4 * (SAMPLES + 1));
    assert!(
        editor
            .capture_targets
            .contains_key("graphSlotKeyPoint:3456:0")
    );
    click(&mut editor, &ctx, "graphZoomKeys");
    assert!(editor.graph.slot.rendered_intervals <= 3);
    assert_eq!(
        editor.session.document().animations[0].color_tracks[0]
            .keys
            .len(),
        10_000
    );
}

#[test]
fn slot_graph_large_final_key_zoom_stays_finite_and_offplot_handles_are_not_targets() {
    let bounds = zoom_bounds(1e30, None, 30, [-0.08, 1.08]);
    assert!(valid_geometry(
        egui::Rect::from_min_size(Pos2::ZERO, vec2(800., 100.)),
        bounds
    ));
    let (mut editor, ctx) = fixture();
    click(&mut editor, &ctx, "graphZoomKeys");
    let original = editor.session.document().clone();
    editor.graph.slot.bounds = Some([0.3, 0.300005, -10., -9.]);
    settle(&mut editor, &ctx);
    assert!(!editor.capture_targets.contains_key("graphSlotHandle:0"));
    assert!(!editor.capture_targets.contains_key("graphSlotHandle:1"));
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn slot_graph_delete_final_color_track_handles_empty_track_same_frame() {
    let (mut editor, ctx) = fixture();
    let mut doc = editor.session.document().clone();
    doc.animations[0].color_tracks[0]
        .keys
        .retain(|key| key.time == 0.3);
    editor.session = Session::new(doc).unwrap();
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "graphSlotRow");
    workspace_frame(&mut editor, &ctx, vec![key_event(egui::Key::Delete)]);
    settle(&mut editor, &ctx);
    assert!(
        editor.session.document().animations[0]
            .color_tracks
            .is_empty()
    );
    assert_eq!(
        editor.session.document().animations[0]
            .attachment_tracks
            .len(),
        1
    );
    assert_eq!(editor.graph.slot.rendered_points, 0);
    assert!(!editor.slot_curve_edit_active());
}
