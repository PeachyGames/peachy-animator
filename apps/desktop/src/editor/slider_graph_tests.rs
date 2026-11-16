use super::*;
use crate::editor::test_support::{editor_context, editor_fixture, workspace_frame};

fn ready() -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    let mut doc = editor.session.document().clone();
    doc.fps = 60;
    doc.constraints = vec![Constraint::Slider(SliderConstraint::new("manual graph", 0))];
    let mut host = Animation::new("slider host", vec![]);
    host.slider_tracks = SliderProperty::ALL
        .into_iter()
        .map(|property| SliderTrack {
            constraint: 0,
            property,
            keys: if property == SliderProperty::Time {
                vec![
                    Keyframe {
                        time: 0.,
                        value: 0.25,
                        curve: Curve::Bezier([0.25, 0.1, 0.75, 0.9]),
                    },
                    Keyframe {
                        time: 0.5,
                        value: 0.75,
                        curve: Curve::Linear,
                    },
                    Keyframe {
                        time: 0.500005,
                        value: 1.25,
                        curve: Curve::Bezier([0.2, 0.3, 0.8, 0.7]),
                    },
                    Keyframe {
                        time: 1.,
                        value: 1.5,
                        curve: Curve::Stepped,
                    },
                ]
            } else {
                vec![
                    Keyframe {
                        time: 0.,
                        value: 0.25,
                        curve: Curve::Bezier([0.25, 0.1, 0.75, 0.9]),
                    },
                    Keyframe {
                        time: 0.5,
                        value: 0.75,
                        curve: Curve::Stepped,
                    },
                    Keyframe {
                        time: 1.,
                        value: 1.,
                        curve: Curve::Linear,
                    },
                ]
            },
        })
        .collect();
    doc.animations.push(host);
    editor.animation = doc.animations.len() - 1;
    editor.session = Session::new(doc).unwrap();
    editor.setup_mode = false;
    editor.auto_key = false;
    editor.select(TreeItem::Constraint(0));
    editor.reconcile_dopesheet_selection();
    editor.key_selection.replace(vec![AnimationKey::Slider {
        constraint: 0,
        property: SliderProperty::Time,
        time: 0.,
    }]);
    editor.curves.separate = true;
    editor.show_tree = false;
    editor.show_dopesheet = false;
    editor.show_timeline = false;
    editor.open_graph_view();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    (editor, ctx)
}
fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..6 {
        workspace_frame(editor, ctx, vec![]);
    }
}
fn rect(editor: &Editor, name: &str) -> egui::Rect {
    let [l, t, r, b] = editor.capture_targets[name];
    egui::Rect::from_min_max(pos2(l, t), pos2(r, b))
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
    let point = rect(editor, name).center();
    pointer(editor, ctx, point, true);
    pointer(editor, ctx, point, false);
    settle(editor, ctx);
}
fn chord(editor: &mut Editor, ctx: &egui::Context, key: egui::Key, modifiers: egui::Modifiers) {
    workspace_frame(
        editor,
        ctx,
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed,
                repeat: false,
                modifiers,
            })
            .collect(),
    );
}
fn track(editor: &Editor, property: SliderProperty) -> &SliderTrack {
    editor.session.document().animations[editor.animation]
        .slider_tracks
        .iter()
        .find(|track| track.constraint == 0 && track.property == property)
        .unwrap()
}
fn curve_channel(editor: &Editor, property: SliderProperty) -> peachy_engine::CurveChannel {
    peachy_engine::curve_channels(
        editor.session.document(),
        editor.animation,
        &peachy_engine::CurveTarget {
            key: AnimationKey::Slider {
                constraint: 0,
                property,
                time: 0.,
            },
            next_time: 0.5,
        },
    )
    .unwrap()
    .remove(0)
}

#[test]
fn slider_graph_overlays_native_units_and_keeps_time_mix_selection_explicit() {
    let (mut editor, ctx) = ready();
    assert_eq!(editor.selected_bone(), None);
    let bounds = editor.graph.bounds.unwrap();
    let plot = rect(&editor, "graphPlot");
    let time = rect(&editor, "graphSliderKey:Time:0").center();
    let mix = rect(&editor, "graphSliderKey:Mix:0").center();
    let read_y = |point: Pos2| {
        bounds.value_min
            + f64::from(plot.bottom() - point.y) / f64::from(plot.height())
                * (bounds.value_max - bounds.value_min)
    };
    assert!((read_y(time) - 15.).abs() < 0.001);
    assert!((read_y(mix) - 25.).abs() < 0.001);
    click(&mut editor, &ctx, "graphSliderKey:Mix:0");
    assert_eq!(editor.graph.slider_property, SliderProperty::Mix);
    assert_eq!(
        editor.key_selection.keys(),
        &[AnimationKey::Slider {
            constraint: 0,
            property: SliderProperty::Mix,
            time: 0.
        }]
    );
    assert!(!editor.capture_targets.contains_key("graphPropertyPicker"));
    assert!(matches!(
        editor.graph.context,
        Some(GraphContext::Slider {
            constraint: 0,
            property: SliderProperty::Mix,
            ..
        })
    ));
    assert!(editor.slider_graph_has_focus(&ctx));
}

#[test]
fn slider_graph_value_drag_preserves_fractional_times_curves_and_pending_one_undo() {
    let (mut editor, ctx) = ready();
    editor
        .session
        .edit_slider_pose(
            0,
            editor.weights_edit_target(),
            &[peachy_engine::SliderPoseEdit::Mix { value: -0.25 }],
            false,
        )
        .unwrap();
    settle(&mut editor, &ctx);
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let source = rect(&editor, "graphSliderKey:Time:0.500005").center();
    let target = source - vec2(0., 18.);
    pointer(&mut editor, &ctx, source, true);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(target)]);
    assert!(editor.graph.edit.is_some());
    assert_ne!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    pointer(&mut editor, &ctx, target, false);
    assert!(!editor.graph_edit_active());
    let before = &original.animations[editor.animation].slider_tracks[0];
    let after = track(&editor, SliderProperty::Time);
    assert_eq!(
        after
            .keys
            .iter()
            .map(|key| key.time.to_bits())
            .collect::<Vec<_>>(),
        before
            .keys
            .iter()
            .map(|key| key.time.to_bits())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        after.keys.iter().map(|key| key.curve).collect::<Vec<_>>(),
        before.keys.iter().map(|key| key.curve).collect::<Vec<_>>()
    );
    assert_eq!(after.keys[1], before.keys[1]);
    assert_ne!(
        after.keys[2].value.to_bits(),
        before.keys[2].value.to_bits()
    );
    assert_eq!(
        track(&editor, SliderProperty::Mix),
        &original.animations[editor.animation].slider_tracks[1]
    );
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
}

#[test]
fn slider_graph_shared_handle_escape_restores_document_redo_and_blocks_held_reentry() {
    let (mut editor, ctx) = ready();
    editor
        .session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo marker".into(),
        })
        .unwrap();
    editor.session.undo();
    let original = editor.session.document().clone();
    let from = rect(&editor, "graphSliderHandle:0").center();
    let to = from + vec2(12., -16.);
    pointer(&mut editor, &ctx, from, true);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(to)]);
    assert_eq!(
        editor.shared_curve_owner(),
        Some(curve_gesture::CurveInputOwner::SliderGraph)
    );
    assert_ne!(editor.session.document(), &original);
    chord(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(!editor.graph_edit_active());
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.can_redo());
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(from)]);
    assert!(!editor.graph_edit_active());
    assert_eq!(editor.session.document(), &original);
    pointer(&mut editor, &ctx, from, false);
}

#[test]
fn slider_graph_numeric_handles_convert_host_frames_and_referenced_frames_without_touching_mix() {
    let (mut editor, ctx) = ready();
    let original = editor.session.document().clone();
    click(&mut editor, &ctx, "graphSliderHandles");
    let field = "graphSliderValue:1";
    click(&mut editor, &ctx, field);
    assert!(ctx.text_edit_focused());
    chord(&mut editor, &ctx, egui::Key::A, egui::Modifiers::COMMAND);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("45".into())]);
    chord(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    settle(&mut editor, &ctx);
    let handles =
        curve_controller::value_handles(&curve_channel(&editor, SliderProperty::Time)).unwrap();
    assert!((handles[1] - 0.75).abs() < 0.000001);
    assert_eq!(
        track(&editor, SliderProperty::Mix),
        &original.animations[editor.animation].slider_tracks[1]
    );
    let y_edited = editor.session.document().clone();
    click(&mut editor, &ctx, "graphSliderValue:0");
    chord(&mut editor, &ctx, egui::Key::A, egui::Modifiers::COMMAND);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("12".into())]);
    chord(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    settle(&mut editor, &ctx);
    let handles =
        curve_controller::value_handles(&curve_channel(&editor, SliderProperty::Time)).unwrap();
    assert!((handles[0] - 0.4).abs() < 0.000001);
    assert!((handles[1] - 0.75).abs() < 0.000001);
    editor.undo();
    assert_eq!(editor.session.document(), &y_edited);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn slider_graph_canvas_clipboard_delete_retains_other_property_and_has_no_bone_fallback() {
    let (mut editor, ctx) = ready();
    let original = editor.session.document().clone();
    let original_mix = track(&editor, SliderProperty::Mix).clone();
    click(&mut editor, &ctx, "graphSliderKey:Time:0.5");
    chord(&mut editor, &ctx, egui::Key::C, egui::Modifiers::COMMAND);
    assert!(
        matches!(editor.timeline_clipboard,Some(peachy_engine::AnimationKeyClipboard::Slider{ref keys})if keys.len()==1&&keys[0].property==SliderProperty::Time)
    );
    editor.set_time(0.75);
    settle(&mut editor, &ctx);
    chord(&mut editor, &ctx, egui::Key::V, egui::Modifiers::COMMAND);
    settle(&mut editor, &ctx);
    assert!(
        track(&editor, SliderProperty::Time)
            .keys
            .iter()
            .any(|key| key.time == 0.75)
    );
    assert_eq!(
        editor.key_selection.keys(),
        &[AnimationKey::Slider {
            constraint: 0,
            property: SliderProperty::Time,
            time: 0.5
        }]
    );
    let pasted = editor.session.document().clone();
    chord(&mut editor, &ctx, egui::Key::Delete, egui::Modifiers::NONE);
    settle(&mut editor, &ctx);
    assert!(
        !track(&editor, SliderProperty::Time)
            .keys
            .iter()
            .any(|key| key.time == 0.5)
    );
    assert!(
        track(&editor, SliderProperty::Time)
            .keys
            .iter()
            .any(|key| key.time == 0.75)
    );
    assert_eq!(track(&editor, SliderProperty::Mix), &original_mix);
    editor.undo();
    assert_eq!(editor.session.document(), &pasted);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.selected_bone(), None);
}
