use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
use super::*;

fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..4 {
        workspace_frame(editor, ctx, vec![]);
    }
}
fn rect(editor: &Editor, name: &str) -> egui::Rect {
    let [l, t, r, b] = editor.capture_targets[name];
    egui::Rect::from_min_max(pos2(l, t), pos2(r, b))
}
fn click(editor: &mut Editor, ctx: &egui::Context, name: &str) {
    let p = rect(editor, name).center();
    for pressed in [true, false] {
        workspace_frame(
            editor,
            ctx,
            vec![
                egui::Event::PointerMoved(p),
                egui::Event::PointerButton {
                    pos: p,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    settle(editor, ctx);
}

fn pointer(editor: &mut Editor, ctx: &egui::Context, p: Pos2, pressed: bool) {
    workspace_frame(
        editor,
        ctx,
        vec![
            egui::Event::PointerMoved(p),
            egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
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

fn curve_editor() -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    let mut document = editor.session.document().clone();
    for (property, values, handles) in [
        (Property::Rotation, [0., 30., 10.], [0.25, 0.2, 0.75, 0.8]),
        (Property::X, [0., 10., 40.], [0.2, 0.9, 0.8, 0.3]),
    ] {
        document.animations[0]
            .tracks
            .iter_mut()
            .find(|track| track.bone == 8 && track.property == property)
            .unwrap()
            .keys = [0., 0.5, 1.]
            .into_iter()
            .zip(values)
            .map(|(time, value)| Keyframe {
                time,
                value,
                curve: Curve::Bezier(handles),
            })
            .collect();
    }
    editor.session = Session::new(document).unwrap();
    editor.key_selection.replace(vec![AnimationKey::Bone {
        bone: 8,
        property: Property::Rotation,
        time: 0.,
    }]);
    editor.curves.separate = true;
    editor.open_curves_view();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    (editor, ctx)
}

fn rotation(editor: &Editor, index: usize) -> Keyframe {
    editor.session.document().animations[0]
        .tracks
        .iter()
        .find(|track| track.bone == 8 && track.property == Property::Rotation)
        .unwrap()
        .keys[index]
}

#[test]
fn curves_view_occupies_a_separate_vertical_pane_above_the_full_width_dopesheet() {
    let mut editor = editor_fixture();
    let ctx = editor_context();
    editor.open_curves_view();
    settle(&mut editor, &ctx);
    let curves = rect(&editor, "curvesView");
    let tree = rect(&editor, "tree");
    let bottom = rect(&editor, "animationViews");
    assert!(curves.right() <= tree.left());
    assert!(curves.bottom() <= bottom.top());
    assert!(bottom.right() > tree.right());
    assert!(!editor.capture_targets.contains_key("viewTab:Curves"));
    assert!(editor.dopesheet_visible());
    assert!(!editor.graph_visible());
    let before = editor.session.document().clone();
    editor.set_setup_mode(true);
    settle(&mut editor, &ctx);
    assert!(!editor.curves_visible());
    assert!(!editor.capture_targets.contains_key("curvesView"));
    editor.set_setup_mode(false);
    settle(&mut editor, &ctx);
    assert!(editor.curves_visible());
    assert_eq!(editor.session.document(), &before);
}

#[test]
fn dopesheet_exact_scalar_selection_drives_curves_and_scalar_graph_without_quantization() {
    let mut editor = editor_fixture();
    let ctx = editor_context();
    editor.open_curves_view();
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "timelineKey:Rotation:1");
    let selected = AnimationKey::Bone {
        bone: 8,
        property: Property::Rotation,
        time: 0.3,
    };
    assert_eq!(editor.key_selection.keys(), std::slice::from_ref(&selected));
    let channels = editor.selected_curve_channels().unwrap();
    assert_eq!(channels[0].target.key, selected);
    assert_eq!(channels[0].target.next_time, 0.300005);
    let before = editor.session.document().clone();
    editor.open_graph_view();
    settle(&mut editor, &ctx);
    assert_eq!(
        editor.graph_diagnostic()["selectedTime"],
        serde_json::json!(0.3f32)
    );
    assert_eq!(
        editor.graph_diagnostic()["property"],
        serde_json::json!(Property::Rotation)
    );
    click(&mut editor, &ctx, "graphKeyPicker");
    click(&mut editor, &ctx, "graphKeyChoice:2");
    assert_eq!(
        editor.key_selection.keys(),
        &[AnimationKey::Bone {
            bone: 8,
            property: Property::Rotation,
            time: 0.300005
        }]
    );
    assert_eq!(
        editor.selected_curve_channels().unwrap()[0]
            .target
            .key
            .time(),
        0.300005
    );
    assert_eq!(editor.session.document(), &before);
}

#[test]
fn curves_vertical_pane_keeps_a_usable_viewport_and_full_width_graph_at_minimum_window() {
    let mut editor = editor_fixture();
    let ctx = editor_context();
    editor.open_curves_view();
    editor.open_graph_view();
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
    let viewport = rect(&editor, "viewport");
    let curves = rect(&editor, "curvesView");
    let bottom = rect(&editor, "animationViews");
    assert!(
        viewport.width() > 400. && viewport.height() >= 100.,
        "{viewport:?}"
    );
    assert!(viewport.right() <= curves.left());
    assert!(curves.bottom() <= bottom.top());
    let graph = rect(&editor, "graphPlot");
    assert!(graph.width() > 1000. && graph.height() >= 70., "{graph:?}");
}

#[test]
fn curves_fit_outside_handles_freeze_bounds_during_drag_and_render_extreme_data_finitely() {
    let (mut editor, ctx) = curve_editor();
    let mut document = editor.session.document().clone();
    let track = document.animations[0]
        .tracks
        .iter_mut()
        .find(|track| track.bone == 8 && track.property == Property::Rotation)
        .unwrap();
    track.keys[0].curve = Curve::Bezier([0.25, -1. / 12., 0.75, 13. / 12.]);
    editor.session = Session::new(document).unwrap();
    settle(&mut editor, &ctx);
    let original = editor.session.document().clone();
    let fitted = editor.curves.bounds.unwrap();
    assert!(fitted[2] <= -1. / 12. && fitted[3] >= 13. / 12.);
    assert_eq!(&fitted[..2], &[0., 1.]);
    let plot = rect(&editor, "curvesPlot");
    for index in 0..2 {
        assert!(plot.contains(rect(&editor, &format!("curvesHandle:{index}")).center()));
    }
    let from = rect(&editor, "curvesHandle:0").center();
    pointer(&mut editor, &ctx, from, true);
    let to = from - vec2(10., plot.height() * 1.5);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(to)]);
    assert!(editor.shared_curve_edit_active());
    assert_ne!(editor.session.document(), &original);
    assert_eq!(editor.curves.bounds, Some(fitted));
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(to - vec2(0., 20.))],
    );
    assert!(editor.shared_curve_edit_active());
    assert_eq!(editor.curves.bounds, Some(fitted));
    pointer(&mut editor, &ctx, to - vec2(0., 20.), false);
    settle(&mut editor, &ctx);
    assert!(!editor.shared_curve_edit_active());
    assert_ne!(editor.curves.bounds, Some(fitted));
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    let mut document = original;
    document.animations[0]
        .tracks
        .iter_mut()
        .find(|track| track.bone == 8 && track.property == Property::Rotation)
        .unwrap()
        .keys[0]
        .curve = Curve::Bezier([f32::MAX, f32::MAX, -f32::MAX, f32::MAX]);
    editor.session = Session::new(document).unwrap();
    editor.key_selection.replace(vec![AnimationKey::Bone {
        bone: 8,
        property: Property::Rotation,
        time: 0.,
    }]);
    editor.open_curves_view();
    let original = editor.session.document().clone();
    fn finite_lines(shape: &egui::Shape) -> bool {
        match shape {
            egui::Shape::LineSegment { points, .. } => points.iter().all(|point| point.is_finite()),
            egui::Shape::Vec(shapes) => shapes.iter().all(finite_lines),
            egui::Shape::Path(path) => path.points.iter().all(|point| point.is_finite()),
            _ => true,
        }
    }
    for _ in 0..4 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
                focused: true,
                ..Default::default()
            },
            |ui| editor.workspace_ui(ui),
        );
        assert!(
            output
                .shapes
                .iter()
                .all(|clipped| finite_lines(&clipped.shape))
        );
        output.textures_delta.clear();
    }
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
}

#[test]
fn curves_match_drag_changes_only_the_dragged_handle_and_commits_one_undo() {
    let (mut editor, ctx) = curve_editor();
    editor.key_selection.select(
        AnimationKey::Bone {
            bone: 8,
            property: Property::X,
            time: 0.,
        },
        true,
    );
    settle(&mut editor, &ctx);
    let original = editor.session.document().clone();
    let plot_width = rect(&editor, "curvesPlot").width();
    let from = rect(&editor, "curvesHandle:0").center();
    pointer(&mut editor, &ctx, from, true);
    let to = from + vec2(16., -31.);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(to)]);
    assert_eq!(
        rect(&editor, "curvesPlot").width(),
        plot_width,
        "Curves footer must not grow its dock during a handle drag"
    );
    assert!(editor.shared_curve_edit_active());
    pointer(&mut editor, &ctx, to + vec2(4., -3.), false);
    settle(&mut editor, &ctx);
    assert!(!editor.shared_curve_edit_active());
    assert_ne!(editor.session.document(), &original);
    for property in [Property::Rotation, Property::X] {
        let track = |document: &Document| {
            document.animations[0]
                .tracks
                .iter()
                .find(|track| track.bone == 8 && track.property == property)
                .unwrap()
                .clone()
        };
        let before = track(&original);
        let after = track(editor.session.document());
        assert_eq!(after.keys[1..], before.keys[1..]);
        assert_eq!(after.keys[0].time, before.keys[0].time);
        assert_eq!(after.keys[0].value, before.keys[0].value);
        let (Curve::Bezier(a), Curve::Bezier(b)) = (after.keys[0].curve, before.keys[0].curve)
        else {
            panic!("Bezier shape lost");
        };
        assert_ne!(&a[..2], &b[..2]);
        assert_eq!(&a[2..], &b[2..]);
    }
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
}

#[test]
fn curves_gray_handle_becomes_primary_and_match_off_edits_only_that_curve() {
    let (mut editor, ctx) = curve_editor();
    editor.key_selection.select(
        AnimationKey::Bone {
            bone: 8,
            property: Property::X,
            time: 0.,
        },
        true,
    );
    editor.curves.match_handles = false;
    editor.open_graph_view();
    settle(&mut editor, &ctx);
    let original = editor.session.document().clone();
    let area = rect(&editor, "curvesPlot").shrink2(vec2(12., 15.));
    let from = point(area, 0.2, 0.9);
    pointer(&mut editor, &ctx, from, true);
    assert_eq!(
        editor.curves.primary,
        Some((
            AnimationKey::Bone {
                bone: 8,
                property: Property::X,
                time: 0.
            },
            0
        ))
    );
    let to = from + vec2(18., 20.);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(to)]);
    pointer(&mut editor, &ctx, to, false);
    settle(&mut editor, &ctx);
    let old_rotation = original.animations[0]
        .tracks
        .iter()
        .find(|track| track.bone == 8 && track.property == Property::Rotation)
        .unwrap();
    let new_rotation = editor.session.document().animations[0]
        .tracks
        .iter()
        .find(|track| track.bone == 8 && track.property == Property::Rotation)
        .unwrap();
    assert_eq!(old_rotation, new_rotation);
    assert_ne!(editor.session.document(), &original);
    editor.open_graph_view();
    settle(&mut editor, &ctx);
    assert_eq!(
        editor.graph_diagnostic()["property"],
        serde_json::json!(Property::X)
    );
    assert_eq!(editor.key_selection.len(), 2);
}

#[test]
fn curves_escape_restores_pending_pose_and_redo_and_blocks_held_reentry() {
    let (mut editor, ctx) = curve_editor();
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
        .dispatch(Command::SetKeyCurve {
            animation: 0,
            bone: 8,
            property: Property::X,
            time: 0.5,
            curve: Curve::Stepped,
        })
        .unwrap();
    editor.session.undo();
    assert!(editor.session.can_redo());
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    settle(&mut editor, &ctx);
    let from = rect(&editor, "curvesHandle:0").center();
    pointer(&mut editor, &ctx, from, true);
    let to = from + vec2(19., -14.);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(to)]);
    assert_ne!(editor.session.document(), &original);
    workspace_frame(
        &mut editor,
        &ctx,
        chord(egui::Key::Escape, egui::Modifiers::NONE),
    );
    assert!(!editor.shared_curve_edit_active());
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(editor.session.can_redo());
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(to + vec2(10., 0.))],
    );
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.shared_curve_edit_active());
    pointer(&mut editor, &ctx, to, false);
}

#[test]
fn scalar_graph_tangent_and_numeric_controls_preserve_exact_key_values_and_times() {
    let (mut editor, ctx) = curve_editor();
    editor.open_graph_view();
    settle(&mut editor, &ctx);
    let original = editor.session.document().clone();
    let from = rect(&editor, "graphScalarHandle:0").center();
    pointer(&mut editor, &ctx, from, true);
    let to = from + vec2(24., -15.);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(to)]);
    assert!(editor.shared_curve_edit_active());
    pointer(&mut editor, &ctx, to, false);
    settle(&mut editor, &ctx);
    let dragged = rotation(&editor, 0);
    assert_eq!(dragged.time, 0.);
    assert_eq!(dragged.value, 0.);
    assert_ne!(editor.session.document(), &original);
    click(&mut editor, &ctx, "graphScalarHandles");
    click(&mut editor, &ctx, "graphScalarValue:1");
    assert!(ctx.text_edit_focused(), "{:?}", editor.curves_diagnostic());
    workspace_frame(
        &mut editor,
        &ctx,
        chord(egui::Key::A, egui::Modifiers::COMMAND),
    );
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("19.25".into())]);
    assert_eq!(
        editor.curve_handle_diagnostic()["draft"]["text"][1],
        serde_json::json!("19.25")
    );
    workspace_frame(
        &mut editor,
        &ctx,
        chord(egui::Key::Tab, egui::Modifiers::NONE),
    );
    click(&mut editor, &ctx, "graphFit");
    let Curve::Bezier(handles) = rotation(&editor, 0).curve else {
        panic!("Bezier shape lost");
    };
    assert!(
        (handles[1] * 30. - 19.25).abs() < 0.00001,
        "handles={handles:?}, status={}, error={:?}",
        editor.status,
        editor.curves.error
    );
    assert_eq!(rotation(&editor, 0).time.to_bits(), dragged.time.to_bits());
    assert_eq!(
        rotation(&editor, 0).value.to_bits(),
        dragged.value.to_bits()
    );
    editor.undo();
    assert_eq!(rotation(&editor, 0), dragged);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn curves_single_terminal_uses_previous_interval_but_multiple_terminals_do_not_edit() {
    let (mut editor, ctx) = curve_editor();
    editor.key_selection.replace(vec![AnimationKey::Bone {
        bone: 8,
        property: Property::Rotation,
        time: 1.,
    }]);
    settle(&mut editor, &ctx);
    assert_eq!(
        editor.selected_curve_channels().unwrap()[0]
            .target
            .key
            .time(),
        0.5
    );
    let untouched_last = rotation(&editor, 2);
    click(&mut editor, &ctx, "curvesInterpolation:Bezier");
    assert_eq!(rotation(&editor, 2), untouched_last);
    let Curve::Bezier(handles) = rotation(&editor, 1).curve else {
        panic!("Bezier reset lost");
    };
    assert!((handles[0] - 1. / 3.).abs() < 0.000001);
    assert_eq!(handles[1], 0.);
    assert!((handles[2] - 2. / 3.).abs() < 0.000001);
    assert_eq!(handles[3], 1.);
    editor.key_selection.select(
        AnimationKey::Bone {
            bone: 8,
            property: Property::X,
            time: 1.,
        },
        true,
    );
    settle(&mut editor, &ctx);
    assert!(editor.selected_curve_channels().unwrap().is_empty());
    let original = editor.session.document().clone();
    click(&mut editor, &ctx, "curvesInterpolation:Bezier");
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn curves_ik_drag_preserves_untouched_f64_time_handle_and_discrete_bend() {
    let mut editor = super::super::test_support::animated_ik_editor();
    let mut document = editor.session.document().clone();
    let precise_time = 0.8765432109876543f64;
    document.animations[0].ik_tracks[0].keys[0].curve = IkCurve::Bezier {
        time: [0.1234567890123456, precise_time],
        mix: [0.3, 0.7],
    };
    editor.session = Session::new(document).unwrap();
    editor.key_selection.replace(vec![AnimationKey::Ik {
        constraint: 1,
        time: 0.,
    }]);
    editor.curves.separate = true;
    editor.open_curves_view();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    let original = editor.session.document().clone();
    let from = rect(&editor, "curvesHandle:0").center();
    pointer(&mut editor, &ctx, from, true);
    let to = from + vec2(16., -17.);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(to)]);
    pointer(&mut editor, &ctx, to, false);
    settle(&mut editor, &ctx);
    let before = &original.animations[0].ik_tracks[0].keys;
    let after = &editor.session.document().animations[0].ik_tracks[0].keys;
    assert_eq!(after[1], before[1]);
    assert_eq!(after[0].time.to_bits(), before[0].time.to_bits());
    assert_eq!(after[0].mix.to_bits(), before[0].mix.to_bits());
    assert_eq!(after[0].bend_positive, before[0].bend_positive);
    let IkCurve::Bezier { time, mix } = after[0].curve else {
        panic!("IK Bezier shape lost");
    };
    assert_eq!(time[1].to_bits(), precise_time.to_bits());
    assert_eq!(mix[1].to_bits(), 0.7f32.to_bits());
    assert_ne!(after[0].curve, before[0].curve);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn scalar_graph_invalid_numeric_input_remains_visible_without_document_or_history_changes() {
    let (mut editor, ctx) = curve_editor();
    editor.open_graph_view();
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "graphScalarHandles");
    let original = editor.session.document().clone();
    click(&mut editor, &ctx, "graphScalarValue:1");
    workspace_frame(
        &mut editor,
        &ctx,
        chord(egui::Key::A, egui::Modifiers::COMMAND),
    );
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("NaN".into())]);
    workspace_frame(
        &mut editor,
        &ctx,
        chord(egui::Key::Tab, egui::Modifiers::NONE),
    );
    settle(&mut editor, &ctx);
    assert!(
        editor
            .curves
            .error
            .as_ref()
            .is_some_and(|error| error.contains("finite"))
    );
    assert!(editor.capture_targets.contains_key("graphScalarError"));
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
    assert!(!editor.session.can_redo());
}

#[test]
fn curves_flat_offset_transition_is_blank_and_read_only_while_preserving_raw_excursions() {
    let (mut editor, ctx) = curve_editor();
    let mut document = editor.session.document().clone();
    let track = document.animations[0]
        .tracks
        .iter_mut()
        .find(|track| track.bone == 8 && track.property == Property::Rotation)
        .unwrap();
    track.keys[1].value = track.keys[0].value;
    track.keys[0].curve = Curve::BezierOffset([0.25, 7.5, 0.75, -5.]);
    editor.session = Session::new(document).unwrap();
    settle(&mut editor, &ctx);
    let original = editor.session.document().clone();
    assert!(!editor.capture_targets.contains_key("curvesHandle:0"));
    assert!(!editor.capture_targets.contains_key("curvesChannel:0"));
    let center = rect(&editor, "curvesPlot").center();
    pointer(&mut editor, &ctx, center, true);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(center + vec2(20., 20.))],
    );
    pointer(&mut editor, &ctx, center + vec2(20., 20.), false);
    click(&mut editor, &ctx, "curvesPresetControl:0");
    assert!(!editor.shared_curve_edit_active());
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
}
