//! Whole-workspace input protects ordered selection and atomic authored-pose edits.
use super::test_support::{editor_context, editor_fixture, workspace_frame};
use super::*;

fn ready() -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    let mut doc = editor.session.document().clone();
    doc.bones[7].name = "group-alpha".into();
    doc.bones[8].name = "group-beta".into();
    for (bone, x, rotation) in [(7, 20., 0.), (8, 0., 30.)] {
        doc.bones[bone].parent = Some(0);
        doc.bones[bone].setup.x = x;
        doc.bones[bone].setup.y = 0.;
        doc.bones[bone].setup.rotation = rotation;
    }
    doc.animations[0].tracks.clear();
    editor.session = Session::new(doc).unwrap();
    editor.session.set_skin(&editor.skin);
    editor.key_selection = Default::default();
    editor.selection_history = Default::default();
    editor.time = 0.4;
    editor.axes = Axes::Parent;
    editor.tool = Tool::Move;
    editor.show_dopesheet = false;
    editor.show_timeline = false;
    editor.tree.query = "group-".into();
    editor.tree.collapsed.clear();
    editor.restore_selection(Some(TreeItem::Bone(7)));
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    (editor, ctx)
}

fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..4 {
        workspace_frame(editor, ctx, vec![]);
    }
}
fn point(editor: &Editor, target: &str) -> Pos2 {
    let [l, t, r, b] = editor.capture_targets[target];
    pos2((l + r) * 0.5, (t + b) * 0.5)
}
fn screen_world(editor: &Editor, world: [f32; 2]) -> Pos2 {
    let [l, t, r, b] = editor.capture_targets["viewport"];
    scene_view::screen_point(
        pos2((l + r) * 0.5, (t + b) * 0.5) + editor.pan,
        editor.zoom,
        world,
    )
}
fn button(point: Pos2, pressed: bool, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers,
    }
}
fn click(editor: &mut Editor, ctx: &egui::Context, target: &str, modifiers: egui::Modifiers) {
    let point = point(editor, target);
    for pressed in [true, false] {
        workspace_frame(
            editor,
            ctx,
            vec![
                egui::Event::ModifiersChanged(modifiers),
                egui::Event::PointerMoved(point),
                button(point, pressed, modifiers),
            ],
        );
    }
    settle(editor, ctx);
}
fn key(editor: &mut Editor, ctx: &egui::Context, key: egui::Key, modifiers: egui::Modifiers) {
    workspace_frame(
        editor,
        ctx,
        std::iter::once(egui::Event::ModifiersChanged(modifiers))
            .chain([true, false].map(|pressed| egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed,
                repeat: false,
                modifiers,
            }))
            .collect(),
    );
}
fn add_second(editor: &mut Editor, ctx: &egui::Context) {
    click(editor, ctx, "bone:8", egui::Modifiers::COMMAND);
    assert_eq!(editor.selected_bones(), vec![7, 8]);
    assert_eq!(editor.primary_bone(), Some(7));
}
fn type_value(editor: &mut Editor, ctx: &egui::Context, text: &str) {
    click(editor, ctx, "value:X", egui::Modifiers::NONE);
    assert!(ctx.text_edit_focused());
    key(editor, ctx, egui::Key::A, egui::Modifiers::COMMAND);
    workspace_frame(
        editor,
        ctx,
        vec![
            egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            egui::Event::Text(text.into()),
        ],
    );
    key(editor, ctx, egui::Key::Enter, egui::Modifiers::NONE);
    settle(editor, ctx);
}
fn pending_unrelated(editor: &mut Editor) {
    let mut pose = editor.session.local_pose(Some(0), editor.time)[0];
    pose.rotation = 13.5;
    editor
        .session
        .edit_pose(
            0,
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
            name: "redo marker".into(),
        })
        .unwrap();
    editor.undo();
    assert!(editor.session.can_redo());
}

#[test]
fn multi_bone_tree_pointer_modifiers_keep_first_primary_history_and_number_groups() {
    let (mut editor, ctx) = ready();
    let original = editor.session.document().clone();
    pending_unrelated(&mut editor);
    let pending = editor.session.pose_edits().clone();
    click(&mut editor, &ctx, "rotateTool", egui::Modifiers::NONE);
    let origin = point(&editor, "gizmoOrigin");
    let second = point(&editor, "bone:8");
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::ModifiersChanged(egui::Modifiers::COMMAND),
            egui::Event::PointerMoved(second),
            button(second, true, egui::Modifiers::COMMAND),
        ],
    );
    // A fast physical Command release must not change the press's additive intent.
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            button(second, false, egui::Modifiers::NONE),
        ],
    );
    settle(&mut editor, &ctx);
    assert_eq!(editor.selected_bones(), vec![7, 8]);
    assert_eq!(editor.primary_bone(), Some(7));
    assert_eq!(point(&editor, "gizmoOrigin"), origin);
    let group = editor.selection.clone();
    key(&mut editor, &ctx, egui::Key::Num3, egui::Modifiers::COMMAND);
    click(&mut editor, &ctx, "bone:8", egui::Modifiers::NONE);
    assert_eq!(editor.selection.single(), Some(&TreeItem::Bone(8)));
    key(&mut editor, &ctx, egui::Key::PageUp, egui::Modifiers::NONE);
    assert_eq!(editor.selection, group);
    key(
        &mut editor,
        &ctx,
        egui::Key::PageDown,
        egui::Modifiers::NONE,
    );
    assert_eq!(editor.selection.single(), Some(&TreeItem::Bone(8)));
    key(&mut editor, &ctx, egui::Key::Num3, egui::Modifiers::NONE);
    assert_eq!(editor.selection, group);
    key(&mut editor, &ctx, egui::Key::PageUp, egui::Modifiers::NONE);
    assert_eq!(editor.selection.single(), Some(&TreeItem::Bone(8)));
    key(
        &mut editor,
        &ctx,
        egui::Key::PageDown,
        egui::Modifiers::NONE,
    );
    assert_eq!(editor.selection, group);
    key(&mut editor, &ctx, egui::Key::PageUp, egui::Modifiers::NONE);
    click(&mut editor, &ctx, "bone:7", egui::Modifiers::NONE);
    key(
        &mut editor,
        &ctx,
        egui::Key::PageDown,
        egui::Modifiers::NONE,
    );
    assert_eq!(editor.selection.single(), Some(&TreeItem::Bone(7)));
    key(&mut editor, &ctx, egui::Key::Num3, egui::Modifiers::NONE);
    click(&mut editor, &ctx, "bone:8", egui::Modifiers::COMMAND);
    assert_eq!(editor.selection.single(), Some(&TreeItem::Bone(7)));
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(editor.session.can_redo());
}

#[test]
fn multi_bone_coalesced_command_click_uses_press_modifiers_in_tree_and_viewport() {
    for viewport in [false, true] {
        let (mut editor, ctx) = ready();
        let original = editor.session.document().clone();
        let point = if viewport {
            let scene = editor.scene_for_view().unwrap();
            let [l, t, r, b] = editor.capture_targets["viewport"];
            let origin = pos2((l + r) * 0.5, (t + b) * 0.5) + editor.pan;
            let world = scene.world[8].point(editor.session.document().bones[8].length * 0.8, 0.);
            scene_view::screen_point(origin, editor.zoom, world)
        } else {
            point(&editor, "bone:8")
        };
        workspace_frame(
            &mut editor,
            &ctx,
            vec![
                egui::Event::ModifiersChanged(egui::Modifiers::COMMAND),
                egui::Event::PointerMoved(point),
                button(point, true, egui::Modifiers::COMMAND),
                egui::Event::ModifiersChanged(egui::Modifiers::NONE),
                button(point, false, egui::Modifiers::NONE),
            ],
        );
        settle(&mut editor, &ctx);
        assert_eq!(editor.selected_bones(), vec![7, 8], "viewport={viewport}");
        assert_eq!(editor.primary_bone(), Some(7));
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
    }
}

#[test]
fn multi_bone_setup_common_rotation_display_preserves_wrapping_and_ik_raw_turns() {
    fn has_text(shape: &egui::Shape, rect: egui::Rect, text: &str) -> bool {
        match shape {
            egui::Shape::Text(shape) => rect.contains(shape.pos) && shape.galley.text() == text,
            egui::Shape::Vec(shapes) => shapes.iter().any(|shape| has_text(shape, rect, text)),
            _ => false,
        }
    }
    for (rotation, ik, display) in [
        (-10., false, "350.00"),
        (370., false, "10.00"),
        (370., true, "370.00"),
    ] {
        let (mut editor, ctx) = ready();
        let mut doc = editor.session.document().clone();
        for bone in [7, 8] {
            doc.bones[bone].setup.rotation = rotation;
        }
        if ik {
            doc.constraints = [7, 8]
                .map(|bone| {
                    Constraint::Ik(IkConstraint {
                        name: format!("raw turns {bone}"),
                        bones: vec![bone],
                        target: 0,
                        setup: IkSetup {
                            mix: 0.,
                            bend_positive: true,
                        },
                        stretch: false,
                    })
                })
                .to_vec();
        }
        editor.session = Session::new(doc).unwrap();
        editor.session.set_skin(&editor.skin);
        editor.key_selection = Default::default();
        editor.set_setup_mode(true);
        settle(&mut editor, &ctx);
        add_second(&mut editor, &ctx);
        let original = editor.session.document().clone();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
                focused: true,
                ..Default::default()
            },
            |ui| editor.workspace_ui(ui),
        );
        let [l, t, r, b] = editor.capture_targets["value:Rotation"];
        let rect = egui::Rect::from_min_max(pos2(l, t), pos2(r, b));
        assert!(
            output
                .shapes
                .iter()
                .any(|shape| has_text(&shape.shape, rect, display)),
            "rotation={rotation}, IK={ik}"
        );
        output.textures_delta.clear();
        click(&mut editor, &ctx, "value:Rotation", egui::Modifiers::NONE);
        key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
        settle(&mut editor, &ctx);
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
    }
}

#[test]
fn multi_bone_parent_numeric_absolute_and_full_set_key_are_independent_atomic_undos() {
    let (mut editor, ctx) = ready();
    pending_unrelated(&mut editor);
    add_second(&mut editor, &ctx);
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let before = editor.session.local_pose(Some(0), editor.time);
    type_value(&mut editor, &ctx, "10");
    let accepted = editor.session.pose_edits().clone();
    let locals = editor.session.local_pose(Some(0), editor.time);
    for bone in [7, 8] {
        let mut expected = before[bone];
        expected.x = 10.;
        assert_eq!(locals[bone], expected);
    }
    assert_eq!(locals[0], before[0]);
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_redo());
    editor.undo();
    assert_eq!(editor.session.pose_edits(), &pending);
    editor.redo();
    assert_eq!(editor.session.pose_edits(), &accepted);
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "key:X", egui::Modifiers::NONE);
    let keyed = editor.session.document().clone();
    for bone in [7, 8] {
        for property in [Property::X, Property::Y] {
            let track = keyed.animations[0]
                .tracks
                .iter()
                .find(|track| track.bone == bone && track.property == property)
                .unwrap();
            assert_eq!(track.keys.len(), 1);
            assert_eq!(
                track.keys[0].value,
                property.read(locals[bone]) - property.read(original.bones[bone].setup)
            );
        }
    }
    assert_eq!(editor.session.local_pose(Some(0), editor.time), locals);
    assert!(editor.session.pose_edits().has_bone(0, 0));
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &accepted);
    editor.redo();
    assert_eq!(editor.session.document(), &keyed);
}

#[test]
fn multi_bone_mixed_same_as_primary_is_a_real_edit_but_untouched_focus_is_not() {
    let (mut editor, ctx) = ready();
    pending_unrelated(&mut editor);
    add_second(&mut editor, &ctx);
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    click(&mut editor, &ctx, "value:X", egui::Modifiers::NONE);
    assert!(ctx.text_edit_focused());
    key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    settle(&mut editor, &ctx);
    assert!(editor.drag.is_none());
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(editor.session.can_redo());
    type_value(&mut editor, &ctx, "20");
    let locals = editor.session.local_pose(Some(0), editor.time);
    assert_eq!(locals[7].x, 20.);
    assert_eq!(locals[8].x, 20.);
    editor.undo();
    assert_eq!(editor.session.pose_edits(), &pending);
}

#[test]
fn multi_bone_parent_numeric_assigns_selected_descendant_as_well_as_ancestor() {
    let (mut editor, ctx) = ready();
    let mut doc = editor.session.document().clone();
    doc.bones[8].parent = Some(7);
    doc.bones[8].setup.x = 120.;
    editor.session = Session::new(doc).unwrap();
    editor.session.set_skin(&editor.skin);
    editor.key_selection = Default::default();
    settle(&mut editor, &ctx);
    add_second(&mut editor, &ctx);
    let original = editor.session.document().clone();
    let before = editor.session.local_pose(Some(0), editor.time);
    type_value(&mut editor, &ctx, "10");
    let authored = editor.session.local_pose(Some(0), editor.time);
    for bone in [7, 8] {
        let mut expected = before[bone];
        expected.x = 10.;
        assert_eq!(authored[bone], expected);
    }
    assert_eq!(editor.session.document(), &original);
    editor.undo();
    assert_eq!(editor.session.local_pose(Some(0), editor.time), before);
    assert!(editor.session.pose_edits().is_empty());
}

#[test]
fn multi_bone_numeric_context_change_cancels_draft_before_stale_enter() {
    let (mut editor, ctx) = ready();
    add_second(&mut editor, &ctx);
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    click(&mut editor, &ctx, "value:X", egui::Modifiers::NONE);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("99".into())]);
    assert!(editor.drag.is_some());
    click(&mut editor, &ctx, "bone:8", egui::Modifiers::NONE);
    key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    assert!(editor.drag.is_none());
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.selection.single(), Some(&TreeItem::Bone(8)));
}

#[test]
fn multi_bone_viewport_translation_uses_primary_axis_and_each_member_frame() {
    for (axes, beta) in [
        (Axes::Parent, [20., 0.]),
        (Axes::World, [20., 0.]),
        (Axes::Local, [17.320_509, 10.]),
    ] {
        let (mut editor, ctx) = ready();
        editor.set_axes(axes);
        add_second(&mut editor, &ctx);
        let original = editor.session.document().clone();
        let before = editor.session.local_pose(Some(0), editor.time);
        let selection = editor.selection.clone();
        let start = point(&editor, "gizmo:X");
        workspace_frame(
            &mut editor,
            &ctx,
            vec![
                egui::Event::PointerMoved(start),
                button(start, true, egui::Modifiers::NONE),
            ],
        );
        let end = start + vec2(20., 0.);
        workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
        assert!(editor.viewport_transform_active());
        let authored = editor.session.local_pose(Some(0), editor.time);
        assert!((authored[7].x - before[7].x - 20.).abs() < 0.0001);
        assert!((authored[8].x - before[8].x - beta[0]).abs() < 0.0001);
        assert!((authored[8].y - before[8].y - beta[1]).abs() < 0.0001);
        assert_eq!(authored[7].rotation, before[7].rotation);
        assert_eq!(authored[8].rotation, before[8].rotation);
        workspace_frame(
            &mut editor,
            &ctx,
            vec![button(end, false, egui::Modifiers::NONE)],
        );
        assert!(!editor.viewport_transform_active());
        assert_eq!(editor.selection, selection);
        assert_eq!(editor.session.document(), &original);
        editor.undo();
        assert_eq!(editor.session.local_pose(Some(0), editor.time), before);
        assert!(!editor.session.can_undo());
        editor.redo();
        assert_eq!(editor.session.local_pose(Some(0), editor.time), authored);
    }
}

#[test]
fn multi_bone_viewport_ancestor_filter_escape_block_and_arrow_nudge_keep_logical_selection() {
    let (mut editor, ctx) = ready();
    let mut doc = editor.session.document().clone();
    doc.bones[8].parent = Some(7);
    doc.bones[8].setup.x = 120.;
    editor.session = Session::new(doc).unwrap();
    editor.session.set_skin(&editor.skin);
    editor.key_selection = Default::default();
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "bone:8", egui::Modifiers::NONE);
    click(&mut editor, &ctx, "bone:7", egui::Modifiers::COMMAND);
    assert_eq!(editor.selected_bones(), vec![8, 7]);
    assert_eq!(editor.primary_bone(), Some(8));
    pending_unrelated(&mut editor);
    settle(&mut editor, &ctx);
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let before = editor.session.local_pose(Some(0), editor.time);
    let selection = editor.selection.clone();
    let scene = editor.scene_for_view().unwrap();
    let [l, t, r, b] = editor.capture_targets["viewport"];
    let origin = pos2((l + r) * 0.5, (t + b) * 0.5) + editor.pan;
    assert_eq!(
        point(&editor, "gizmoOrigin"),
        scene_view::screen_point(origin, editor.zoom, scene.world[7].point(0., 0.))
    );
    let expected_origin = point(&editor, "gizmoOrigin");
    click(&mut editor, &ctx, "rotateTool", egui::Modifiers::NONE);
    assert_eq!(point(&editor, "gizmoOrigin"), expected_origin);
    assert_eq!(editor.primary_bone(), Some(8));
    click(&mut editor, &ctx, "translateTool", egui::Modifiers::NONE);
    let start = point(&editor, "gizmo:X");
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::PointerMoved(start),
            button(start, true, egui::Modifiers::NONE),
        ],
    );
    let end = start + vec2(25., 0.);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
    assert!(editor.viewport_transform_active());
    assert_eq!(
        editor.transform_gesture_diagnostic()["memberBones"],
        serde_json::json!([8, 7])
    );
    assert_eq!(
        editor.transform_gesture_diagnostic()["editBones"],
        serde_json::json!([7])
    );
    assert_eq!(
        editor.session.local_pose(Some(0), editor.time)[8],
        before[8]
    );
    assert!(editor.session.pose_edits().has_bone(0, 7));
    assert!(!editor.session.pose_edits().has_bone(0, 8));
    key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(editor.drag.is_none() && editor.transform_blocked);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(editor.session.can_redo());
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(end + vec2(25., 0.))],
    );
    assert!(editor.drag.is_none());
    assert_eq!(editor.session.pose_edits(), &pending);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![button(end, false, egui::Modifiers::NONE)],
    );
    assert!(!editor.transform_blocked);
    key(
        &mut editor,
        &ctx,
        egui::Key::ArrowRight,
        egui::Modifiers::NONE,
    );
    let authored = editor.session.local_pose(Some(0), editor.time);
    assert_eq!(authored[7].x, before[7].x + 1.);
    assert_eq!(authored[8], before[8]);
    assert_eq!(editor.selection, selection);
    assert_eq!(editor.session.document(), &original);
    editor.undo();
    assert_eq!(editor.session.pose_edits(), &pending);
}

#[test]
fn multi_bone_viewport_press_targets_survive_coalesced_and_staged_moves_to_other_bone() {
    for staged in [false, true] {
        for target in ["bone", "empty", "gizmo"] {
            let (mut editor, ctx) = ready();
            let scene = editor.scene_for_view().unwrap();
            let end = screen_world(&editor, scene.world[8].point(22.4, 0.));
            let start = match target {
                "bone" => screen_world(&editor, scene.world[7].point(4., 0.)),
                "gizmo" => point(&editor, "gizmo:X"),
                _ => {
                    let [l, _, _, b] = editor.capture_targets["viewportInput"];
                    pos2(l + 30., b - 45.)
                }
            };
            let original = editor.session.document().clone();
            let before = editor.session.local_pose(Some(0), editor.time);
            let selection = editor.selection.clone();
            let mut events = vec![
                egui::Event::PointerMoved(start),
                button(start, true, egui::Modifiers::NONE),
            ];
            if !staged {
                events.push(egui::Event::PointerMoved(end));
            }
            workspace_frame(&mut editor, &ctx, events);
            let expected = match target {
                "bone" => TreeItem::Bone(7).diagnostic(),
                "gizmo" => serde_json::json!({"type":"gizmo","axis":0}),
                _ => serde_json::json!({"type":"empty"}),
            };
            assert_eq!(editor.viewport_press_diagnostic()["target"], expected);
            assert!(!editor.viewport_transform_active());
            assert_eq!(editor.session.document(), &original);
            assert!(editor.session.pose_edits().is_empty());
            if staged {
                workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
            } else {
                // egui deliberately defers click-versus-drag on the press frame,
                // including when a move already crossed the threshold. No new
                // pointer event is needed to arm the retained drag next frame.
                workspace_frame(&mut editor, &ctx, vec![]);
            }
            assert!(
                editor.viewport_transform_active(),
                "{target}, staged={staged}"
            );
            assert_eq!(editor.selection, selection, "{target}, staged={staged}");
            let authored = editor.session.local_pose(Some(0), editor.time);
            assert_ne!(authored[7], before[7]);
            assert_eq!(authored[8], before[8]);
            assert_eq!(
                editor.transform_gesture_diagnostic()["input"]["axis"],
                if target == "gizmo" {
                    serde_json::json!(0)
                } else {
                    serde_json::Value::Null
                }
            );
            if target == "gizmo" {
                assert_eq!(authored[7].y, before[7].y);
            }
            key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
            workspace_frame(
                &mut editor,
                &ctx,
                vec![button(end, false, egui::Modifiers::NONE)],
            );
            assert_eq!(editor.session.document(), &original);
            assert!(editor.session.pose_edits().is_empty());
            assert!(!editor.session.can_undo());
        }
    }
}

#[test]
fn multi_bone_viewport_coalesced_release_then_hover_uses_original_click_and_modifiers() {
    for target in ["bone", "empty", "gizmo", "add"] {
        let (mut editor, ctx) = ready();
        let scene = editor.scene_for_view().unwrap();
        let other = screen_world(&editor, scene.world[8].point(22.4, 0.));
        let start = match target {
            "bone" => screen_world(&editor, scene.world[7].point(4., 0.)),
            "gizmo" => point(&editor, "gizmo:X"),
            "add" => other,
            _ => {
                let [l, _, _, b] = editor.capture_targets["viewportInput"];
                pos2(l + 30., b - 45.)
            }
        };
        let modifiers = if target == "add" {
            egui::Modifiers::COMMAND
        } else {
            egui::Modifiers::NONE
        };
        let original = editor.session.document().clone();
        let hover = if target == "add" {
            screen_world(&editor, scene.world[7].point(4., 0.))
        } else {
            other
        };
        workspace_frame(
            &mut editor,
            &ctx,
            vec![
                egui::Event::ModifiersChanged(modifiers),
                egui::Event::PointerMoved(start),
                button(start, true, modifiers),
                egui::Event::ModifiersChanged(egui::Modifiers::NONE),
                button(start, false, egui::Modifiers::NONE),
                egui::Event::PointerMoved(hover),
            ],
        );
        assert_eq!(
            editor.selected_bones(),
            match target {
                "empty" => vec![],
                "add" => vec![7, 8],
                _ => vec![7],
            },
            "{target}"
        );
        assert!(editor.viewport_press_diagnostic().is_null());
        assert!(editor.drag.is_none());
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.pose_edits().is_empty());
    }
}

#[test]
fn multi_bone_viewport_pending_press_expires_on_mode_clip_skin_or_membership_change() {
    for context in ["mode", "clip", "skin", "members", "history"] {
        let (mut editor, ctx) = ready();
        let mut doc = editor.session.document().clone();
        doc.animations.push(Animation::new("alternate", vec![]));
        editor.session = Session::new(doc).unwrap();
        editor.session.set_skin(&editor.skin);
        editor.key_selection = Default::default();
        if context == "history" {
            pending_unrelated(&mut editor);
        }
        settle(&mut editor, &ctx);
        let scene = editor.scene_for_view().unwrap();
        let start = screen_world(&editor, scene.world[7].point(4., 0.));
        let end = screen_world(&editor, scene.world[8].point(22.4, 0.));
        workspace_frame(
            &mut editor,
            &ctx,
            vec![
                egui::Event::PointerMoved(start),
                button(start, true, egui::Modifiers::NONE),
            ],
        );
        assert!(!editor.viewport_press_diagnostic().is_null());
        match context {
            "mode" => editor.setup_mode = true,
            "clip" => editor.animation = 1,
            "skin" => editor.skin = "mint".into(),
            "history" => editor.undo(),
            _ => editor.selection = TreeSelection::singleton(TreeItem::Bone(8)),
        }
        let original = editor.session.document().clone();
        let selection = editor.selection.clone();
        workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
        assert!(editor.viewport_press_diagnostic().is_null());
        assert!(editor.drag.is_none(), "{context}");
        assert_eq!(editor.selection, selection);
        workspace_frame(
            &mut editor,
            &ctx,
            vec![button(end, false, egui::Modifiers::NONE)],
        );
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.pose_edits().is_empty());
        if context == "history" {
            assert!(editor.session.can_redo());
        }
    }
}

#[test]
fn multi_bone_viewport_attachment_click_is_not_replaced_by_later_bone_hover() {
    let (mut editor, ctx) = ready();
    editor.set_setup_mode(true);
    editor.set_tool(Tool::Rotate);
    editor.show_bones = false;
    settle(&mut editor, &ctx);
    let scene = editor.scene_for_view().unwrap();
    let (attachment, start) = scene
        .attachments
        .iter()
        .rev()
        .find_map(|draw| {
            draw.triangles
                .as_chunks::<3>()
                .0
                .iter()
                .find_map(|triangle| {
                    let world = [0, 1].map(|axis| {
                        triangle
                            .iter()
                            .map(|vertex| draw.vertices[*vertex as usize][axis] / 3.)
                            .sum::<f32>()
                    });
                    let point = screen_world(&editor, world);
                    let [l, t, r, b] = editor.capture_targets["viewportInput"];
                    let inside = egui::Rect::from_min_max(pos2(l, t), pos2(r, b)).contains(point);
                    (inside
                        && peachy_engine::pick_attachment(&scene.attachments, world)
                            .is_some_and(|hit| hit.attachment == draw.attachment))
                    .then(|| (draw.attachment.clone(), point))
                })
        })
        .expect("Original region geometry supplies an attachment hit");
    let end = screen_world(&editor, scene.world[8].point(22.4, 0.));
    let original = editor.session.document().clone();
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::PointerMoved(start),
            button(start, true, egui::Modifiers::NONE),
            button(start, false, egui::Modifiers::NONE),
            egui::Event::PointerMoved(end),
        ],
    );
    assert_eq!(
        editor.selection.single(),
        Some(&TreeItem::Attachment(attachment))
    );
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
}
