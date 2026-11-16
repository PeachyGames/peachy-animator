//! Original six-bone inputs and numeric observations from installed Spine 4.3.26.
//! Native controls used ordinary Translate with Bones/Images compensation off.
use crate::*;
use peachy_document::*;

fn fixture() -> Document {
    let mut doc = Document::new("Original group translation lab".into(), 60).unwrap();
    for (name, parent, x, y, rotation, length) in [
        ("source_parent", 0, 0., 120., 0., 0.),
        ("driver", 1, 20., 0., 0., 60.),
        ("follower_parent", 0, 0., -100., 0., 0.),
        ("follower", 3, 0., 0., 30., 120.),
        ("tip", 4, 120., 0., 0., 20.),
    ] {
        doc.bones.push(Bone {
            name: name.into(),
            parent: Some(parent),
            length,
            setup: Transform {
                x,
                y,
                rotation,
                ..Transform::default()
            },
            inherit: Inherit::Normal,
        });
    }
    doc.animations.push(Animation::new("host", vec![]));
    doc.validate().unwrap();
    doc
}

fn gesture(axes: Axes, axis: Option<usize>, end: [f32; 2]) -> TransformGesture {
    TransformGesture {
        tool: TransformTool::Translate,
        axes,
        axis,
        start: [0., 0.],
        end,
        snap_rotation: false,
    }
}

fn capture_frame(doc: &Document, bones: &[usize], primary: usize) -> BoneTransformFrame {
    let pose = checked_evaluate_pose(doc, None, 0.).unwrap();
    BoneTransformFrame::from_evaluated(doc, &pose, bones, primary).unwrap()
}

fn edit_for(edits: &[BonePoseEdit], bone: usize) -> &BonePoseEdit {
    edits.iter().find(|edit| edit.bone == bone).unwrap()
}

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.00003,
        "{actual} != {expected}"
    );
}

#[test]
fn bone_transform_group_native_unrelated_translation_uses_one_primary_delta() {
    let doc = fixture();
    let frame = capture_frame(&doc, &[2, 4], 2);
    assert_eq!(frame.members(), [2, 4]);
    assert_eq!(frame.edit_bones(), [2, 4]);
    assert_eq!(frame.primary(), 2);
    assert_eq!(frame.selection_primary(), 2);
    assert_eq!(frame.primary_inherit(), Inherit::Normal);
    // The retained native pointer move was nominally 20 world units; its JSON
    // records 19.99999 after screen-coordinate conversion and writer rounding.
    let delta = 19.99999;
    for axes in [Axes::Parent, Axes::World] {
        let edits = frame.drag(gesture(axes, Some(0), [delta, 0.])).unwrap();
        close(edit_for(&edits, 2).transform.x, 39.99999);
        close(edit_for(&edits, 4).transform.x, 19.99999);
        assert_eq!(edit_for(&edits, 4).transform.y, 0.);
        for edit in edits {
            assert_eq!(edit.properties, [Property::X, Property::Y]);
            assert_eq!(edit.transform.rotation, doc.bones[edit.bone].setup.rotation);
        }
    }
    let edits = frame
        .drag(gesture(Axes::Local, Some(0), [delta, 0.]))
        .unwrap();
    close(edit_for(&edits, 2).transform.x, 39.99999);
    close(edit_for(&edits, 4).transform.x, 17.3205);
    close(edit_for(&edits, 4).transform.y, 9.999994);
    // Each member receiving the same world gesture would incorrectly produce
    // follower (19.99999, 0), so this is a distinct native group contract.
    assert!(edit_for(&edits, 4).transform.y > 9.);
}

#[test]
fn bone_transform_group_selected_ancestors_filter_edits_without_reordering_selection() {
    let doc = fixture();
    for (members, expected_edits) in [
        (vec![5, 2, 4, 1], vec![4, 1]),
        (vec![1, 4, 2, 5], vec![1, 4]),
    ] {
        let frame = capture_frame(&doc, &members, 2);
        assert_eq!(frame.members(), members);
        assert_eq!(frame.selection_primary(), 2);
        assert_eq!(frame.primary(), 1);
        close(frame.primary_world().x, 0.);
        close(frame.primary_world().y, 120.);
        assert_eq!(frame.edit_bones(), expected_edits);
        for axes in [Axes::Parent, Axes::Local, Axes::World] {
            let edits = frame.drag(gesture(axes, Some(0), [20., 0.])).unwrap();
            assert_eq!(
                edits.iter().map(|edit| edit.bone).collect::<Vec<_>>(),
                expected_edits
            );
            assert!(!edits.iter().any(|edit| edit.bone == 2 || edit.bone == 5));
            let mut session = Session::new(doc.clone()).unwrap();
            session
                .edit_poses(EditTarget::Setup, &edits, false)
                .unwrap();
            // Native parent+child exports retain the child's exact local pose.
            assert_eq!(session.document().bones[2].setup, doc.bones[2].setup);
            assert_eq!(session.document().bones[5].setup, doc.bones[5].setup);
            session.undo();
            assert_eq!(session.document(), &doc);
        }
    }
    // A selected ancestor need not be the direct parent, and the highest one
    // controls the anchor even when it was selected after the logical primary.
    let nested = capture_frame(&doc, &[5, 4, 0], 5);
    assert_eq!(nested.primary(), 0);
    assert_eq!(nested.edit_bones(), [0]);
}

#[test]
fn bone_transform_group_primary_axis_projection_precedes_member_local_frames() {
    let doc = fixture();
    let frame = capture_frame(&doc, &[2, 4], 2);
    let edits = frame
        .drag(gesture(Axes::Local, Some(0), [20., 15.]))
        .unwrap();
    close(edit_for(&edits, 4).transform.x, 17.320508);
    close(edit_for(&edits, 4).transform.y, 10.);
    let edits = frame
        .drag(gesture(Axes::Local, Some(1), [20., 10.]))
        .unwrap();
    close(edit_for(&edits, 4).transform.x, -5.);
    close(edit_for(&edits, 4).transform.y, 8.660254);
    // A primary with a different local rotation extracts a different common
    // tool delta; membership order itself is not used as an implicit primary.
    let other = capture_frame(&doc, &[2, 4], 4);
    let edits = other.drag(gesture(Axes::Local, None, [20., 0.])).unwrap();
    close(edit_for(&edits, 4).transform.x, 20.);
    close(edit_for(&edits, 4).transform.y, 0.);
    close(edit_for(&edits, 2).transform.x, 37.320508);
    close(edit_for(&edits, 2).transform.y, -10.);
}

#[test]
fn bone_transform_group_world_conversion_uses_each_frozen_parent_and_authored_result() {
    let mut doc = fixture();
    doc.bones[1].setup.scale_x = 2.;
    doc.bones[3].setup.rotation = 90.;
    let pose = checked_evaluate_pose(&doc, None, 0.).unwrap();
    let frame = BoneTransformFrame::from_evaluated(&doc, &pose, &[2, 4], 2).unwrap();
    let edits = frame.drag(gesture(Axes::World, None, [20., 10.])).unwrap();
    for edit in edits {
        let parent = pose.world()[doc.bones[edit.bone].parent.unwrap()];
        let before = parent.point(doc.bones[edit.bone].setup.x, doc.bones[edit.bone].setup.y);
        let after = parent.point(edit.transform.x, edit.transform.y);
        close(after[0] - before[0], 20.);
        close(after[1] - before[1], 10.);
    }
    // The same captured pose can have applied values different from authored
    // values. Translating must add to authored data, not bake an applied rotation.
    let mut constrained = pose;
    constrained.applied_locals[4].rotation = 60.;
    let frame = BoneTransformFrame::from_evaluated(&doc, &constrained, &[2, 4], 2).unwrap();
    let edits = frame.drag(gesture(Axes::Local, None, [20., 0.])).unwrap();
    assert_eq!(edit_for(&edits, 4).transform.rotation, 30.);
    close(edit_for(&edits, 4).transform.x, 5.);
    close(edit_for(&edits, 4).transform.y, 8.660254);
}

#[test]
fn bone_transform_group_invalid_frames_and_late_results_fail_without_partial_edits() {
    let doc = fixture();
    let pose = checked_evaluate_pose(&doc, None, 0.).unwrap();
    for (members, primary) in [
        (vec![], 2),
        (vec![2, 2], 2),
        (vec![2, 99], 2),
        (vec![2, 4], 0),
    ] {
        assert!(BoneTransformFrame::from_evaluated(&doc, &pose, &members, primary).is_err());
    }
    let mut malformed = doc.clone();
    malformed.bones[4].parent = Some(99);
    assert!(BoneTransformFrame::from_evaluated(&malformed, &pose, &[2, 4], 2).is_err());
    malformed.bones[4].parent = Some(5);
    assert!(BoneTransformFrame::from_evaluated(&malformed, &pose, &[2, 4], 2).is_err());
    for value in [f32::NAN, f32::INFINITY] {
        let mut invalid = pose.clone();
        invalid.world[4].x = value;
        assert!(BoneTransformFrame::from_evaluated(&doc, &invalid, &[2, 4], 2).is_err());
        let mut invalid = pose.clone();
        invalid.applied_locals[4].shear_y = value;
        assert!(BoneTransformFrame::from_evaluated(&doc, &invalid, &[2, 4], 2).is_err());
    }
    let mut inactive = pose.clone();
    inactive.active_bones[4] = false;
    assert!(BoneTransformFrame::from_evaluated(&doc, &inactive, &[2, 4], 2).is_err());
    let mut singular = pose.clone();
    singular.world[3].a = 0.;
    singular.world[3].c = 0.;
    let singular_frame = BoneTransformFrame::from_evaluated(&doc, &singular, &[2, 4], 2).unwrap();
    assert!(singular_frame.translate(Axes::World, [1., 0.]).is_err());
    assert!(singular_frame.translate(Axes::Parent, [1., 0.]).is_ok());
    assert!(singular_frame.translate(Axes::Local, [1., 0.]).is_ok());
    let mut singular_suppressed_child = pose.clone();
    singular_suppressed_child.world[4].a = 0.;
    singular_suppressed_child.world[4].c = 0.;
    let suppressed =
        BoneTransformFrame::from_evaluated(&doc, &singular_suppressed_child, &[4, 5], 4).unwrap();
    assert_eq!(
        suppressed
            .translate(Axes::World, [1., 0.])
            .unwrap()
            .iter()
            .map(|edit| edit.bone)
            .collect::<Vec<_>>(),
        [4]
    );
    let mut wrong_layout = pose.clone();
    wrong_layout.inherits.pop();
    assert!(BoneTransformFrame::from_evaluated(&doc, &wrong_layout, &[2, 4], 2).is_err());
    let frame = BoneTransformFrame::from_evaluated(&doc, &pose, &[2, 4], 2).unwrap();
    assert!(
        frame
            .drag(gesture(Axes::Parent, Some(2), [1., 1.]))
            .is_err()
    );
    assert!(
        frame
            .drag(gesture(Axes::Parent, None, [f32::NAN, 1.]))
            .is_err()
    );
    for tool in [
        TransformTool::Rotate,
        TransformTool::Scale,
        TransformTool::Shear,
    ] {
        let mut unsupported = gesture(Axes::Parent, Some(0), [1., 1.]);
        unsupported.tool = tool;
        assert!(frame.drag(unsupported).is_err());
    }
    // A representable first candidate must not escape when a later bone's
    // authored coordinate overflows. The immutable frame remains reusable.
    let mut extreme = pose;
    extreme.authored_locals[4].x = f32::MAX;
    let frame = BoneTransformFrame::from_evaluated(&doc, &extreme, &[2, 4], 2).unwrap();
    assert!(
        frame
            .drag(gesture(Axes::Parent, None, [f32::MAX, 0.]))
            .is_err()
    );
    assert!(frame.drag(gesture(Axes::Parent, None, [0., 0.])).is_ok());
}

#[test]
fn bone_transform_group_semantic_delta_only_inverts_required_edit_roots() {
    let mut doc = fixture();
    doc.bones[4].setup.scale_x = 0.;
    let pose = checked_evaluate_pose(&doc, None, 0.).unwrap();
    // The selected tip's parent is singular, but that tip is suppressed by the
    // selected follower. Moving the follower needs only its nonsingular parent.
    let frame = BoneTransformFrame::from_evaluated(&doc, &pose, &[5, 4], 5).unwrap();
    assert_eq!(frame.primary(), 4);
    assert_eq!(frame.edit_bones(), [4]);
    for axes in [Axes::World, Axes::Parent, Axes::Local] {
        let edits = frame.translate(axes, [10., 0.]).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].bone, 4);
        assert_eq!(edits[0].transform.scale_x.to_bits(), 0.0_f32.to_bits());
        let mut session = Session::new(doc.clone()).unwrap();
        session
            .edit_poses(EditTarget::Setup, &edits, false)
            .unwrap();
        assert_eq!(session.document().bones[5], doc.bones[5]);
        session.undo();
        assert_eq!(session.document(), &doc);
        assert!(frame.drag(gesture(axes, None, [10., 0.])).is_ok());
    }
    // A required edit root's singular parent is different: World must refuse
    // atomically, while an explicit Parent/Local delta needs no inverse at all.
    doc.bones[3].setup.scale_x = 0.;
    let mut session = Session::new(doc.clone()).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    let pose = session.evaluated_pose(EditTarget::Setup).unwrap();
    let frame = BoneTransformFrame::from_evaluated(&doc, &pose, &[2, 4], 4).unwrap();
    let rejected = frame
        .translate(Axes::World, [10., 0.])
        .and_then(|edits| session.edit_poses(EditTarget::Setup, &edits, false));
    assert!(rejected.is_err());
    assert_eq!(session.document(), &doc);
    assert!(!session.is_dirty());
    assert!(session.can_redo());
    for axes in [Axes::Parent, Axes::Local] {
        assert!(frame.translate(axes, [10., 0.]).is_ok());
        assert!(frame.drag(gesture(axes, None, [10., 0.])).is_err());
    }
    assert!(frame.translate(Axes::Parent, [f32::INFINITY, 0.]).is_err());
    session.redo();
    assert_eq!(session.document().animations[0].name, "redo");
}

#[test]
fn bone_transform_group_snapshot_previews_reuse_one_solve_and_cancel_pending_exactly() {
    let mut session = Session::new(fixture()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.25,
    };
    let mut root = session.authored_pose(target).unwrap().bone_locals()[0];
    root.y = 12.;
    session
        .edit_pose(0, target, root, &[Property::Y], false)
        .unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    let pending = session.pose_edits().clone();
    let document = session.document().clone();
    let solves_before = crate::ik::checked_pose_calls();
    let pose = session.evaluated_pose(target).unwrap();
    assert_eq!(crate::ik::checked_pose_calls(), solves_before + 1);
    let frame = BoneTransformFrame::from_evaluated(session.document(), &pose, &[2, 4], 2).unwrap();
    session.begin_transaction();
    for delta in [5., 12., 20.] {
        let edits = frame
            .drag(gesture(Axes::Parent, None, [delta, 0.]))
            .unwrap();
        session.preview_edit_poses(target, &edits, false).unwrap();
    }
    assert_eq!(crate::ik::checked_pose_calls(), solves_before + 1);
    let current = session.authored_pose(target).unwrap();
    close(
        current.bone_locals()[2].x,
        pose.authored_locals()[2].x + 20.,
    );
    close(
        current.bone_locals()[4].x,
        pose.authored_locals()[4].x + 20.,
    );
    assert_eq!(current.bone_locals()[0].y, 12.);
    assert_eq!(session.document(), &document);
    session.cancel_transaction();
    assert_eq!(session.pose_edits(), &pending);
    assert_eq!(session.document(), &document);
    assert!(session.can_redo());
    session.redo();
    assert_eq!(session.document().animations[0].name, "redo");
}

#[test]
fn bone_transform_group_single_member_delegates_existing_tool_arithmetic() {
    let doc = fixture();
    let frame = capture_frame(&doc, &[4], 4);
    for axes in [Axes::Local, Axes::Parent, Axes::World] {
        for (tool, axis) in [
            (TransformTool::Translate, Some(0)),
            (TransformTool::Rotate, None),
            (TransformTool::Scale, Some(1)),
            (TransformTool::Shear, Some(0)),
        ] {
            let world = frame.primary_world();
            let gesture = TransformGesture {
                tool,
                axes,
                axis,
                start: [world.x + 12., world.y + 5.],
                end: [world.x + 18., world.y + 9.],
                snap_rotation: false,
            };
            let expected = drag_transform_in_frame(
                frame.primary_authored(),
                frame.primary_applied(),
                frame.primary_parent(),
                world,
                gesture,
            )
            .unwrap();
            let edits = frame.drag(gesture).unwrap();
            assert_eq!(edits.len(), 1);
            assert_eq!(edits[0].bone, 4);
            assert_eq!(
                Property::ALL.map(|property| property.read(edits[0].transform).to_bits()),
                Property::ALL.map(|property| property.read(expected).to_bits())
            );
        }
    }
}
