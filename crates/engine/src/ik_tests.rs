use crate::*;
use peachy_document::*;
use std::collections::BTreeMap;

fn rig() -> Document {
    Document {
        format: FORMAT_VERSION,
        name: "Original normal IK fixture".into(),
        fps: 30,
        bones: vec![
            Bone {
                name: "root".into(),
                parent: None,
                length: 0.,
                setup: Transform::default(),
                inherit: peachy_document::Inherit::Normal,
            },
            Bone {
                name: "upper".into(),
                parent: Some(0),
                length: 10.,
                setup: Transform::default(),
                inherit: peachy_document::Inherit::Normal,
            },
            Bone {
                name: "lower".into(),
                parent: Some(1),
                length: 10.,
                setup: Transform {
                    x: 10.,
                    ..Transform::default()
                },
                inherit: peachy_document::Inherit::Normal,
            },
            Bone {
                name: "target".into(),
                parent: Some(0),
                length: 0.,
                setup: Transform {
                    x: 10.,
                    y: 10.,
                    ..Transform::default()
                },
                inherit: peachy_document::Inherit::Normal,
            },
        ],
        constraints: vec![],
        slots: vec![],
        skins: BTreeMap::new(),
        animations: vec![Animation {
            ik_tracks: vec![],
            name: "reach".into(),
            tracks: vec![],
            physics: vec![],
            physics_resets: vec![],
            inherit_tracks: vec![],
            deform_tracks: vec![],
        }],
        skin_scope: Default::default(),
    }
}

fn constraint() -> IkConstraint {
    IkConstraint {
        name: "limb".into(),
        bones: vec![1, 2],
        target: 3,
        setup: IkSetup {
            mix: 1.,
            bend_positive: true,
        },
    }
}

fn one(mix: f32) -> Document {
    let mut document = rig();
    document.bones[3].setup.x = 0.;
    let mut ik = constraint();
    ik.bones = vec![1];
    ik.setup.mix = mix;
    document.constraints.push(Constraint::Ik(ik));
    document
}

fn two(mix: f32, positive: bool) -> Document {
    let mut document = rig();
    let mut ik = constraint();
    ik.setup.mix = mix;
    ik.setup.bend_positive = positive;
    document.constraints.push(Constraint::Ik(ik));
    document
}

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= 0.001, "{actual} != {expected}");
}
fn point(actual: [f32; 2], expected: [f32; 2]) {
    near(actual[0], expected[0]);
    near(actual[1], expected[1]);
}

#[test]
fn normal_one_and_both_two_bone_bends_match_original_poses_without_baking() {
    for mix in [0., 0.25, 0.5, 0.75, 1.] {
        let document = one(mix);
        let before = document.clone();
        let pose = checked_evaluate_pose(&document, None, 0.).unwrap();
        near(pose.applied_locals()[1].rotation, 90. * mix);
        assert_eq!(pose.authored_locals()[1].rotation, 0.);
        assert_eq!(document, before);
    }
    for positive in [true, false] {
        let document = two(1., positive);
        let before = document.clone();
        let pose = checked_evaluate_pose(&document, None, 0.).unwrap();
        near(
            pose.applied_locals()[1].rotation,
            if positive { 0. } else { 90. },
        );
        near(
            pose.applied_locals()[2].rotation,
            if positive { 90. } else { -90. },
        );
        point(pose.world()[2].point(10., 0.), [10., 10.]);
        assert_eq!(pose.authored_locals(), local_pose(&document, None, 0.));
        assert_eq!(document, before);
    }
    let mut zero = one(1.);
    zero.bones[1].length = 0.;
    let pose = checked_evaluate_pose(&zero, None, 0.).unwrap();
    near(pose.applied_locals()[1].rotation, 90.);
    point(pose.world()[1].point(0., 0.), [0., 0.]);
}

#[test]
fn original_multi_turn_and_positive_180_ties_are_not_repeated_shortest_modulo() {
    // Independent original public-API observations, rounded to native f32.
    // These multi-turn cases deliberately falsify a conventional modulo solver.
    let cases = [
        (-1080., 0.25, -877.5),
        (-1080., 0.5, -675.),
        (-1080., 0.75, -472.5),
        (-720., 0.5, -495.),
        (-359., 0.5, -314.5),
        (-181., 0.5, -225.5),
        (181., 0.5, 135.5),
        (359., 0.5, 404.5),
        (720., 0.25, 652.5),
        (720., 0.5, 585.),
        (720., 0.75, 517.5),
        (1080., 0.5, 765.),
    ];
    for (authored, mix, expected) in cases {
        let mut document = one(mix);
        document.bones[1].setup.rotation = authored;
        let pose = checked_evaluate_pose(&document, None, 0.).unwrap();
        near(pose.applied_locals()[1].rotation, expected);
        assert_eq!(pose.authored_locals()[1].rotation, authored);
        let mut child = two(mix, true);
        child.bones[2].setup.rotation = authored;
        let pose = checked_evaluate_pose(&child, None, 0.).unwrap();
        near(pose.applied_locals()[2].rotation, expected);
    }
    for (authored, expected) in [
        (-1080., -720.),
        (-720., -540.),
        (-359., -359.5),
        (-181., -270.5),
        (181., 270.5),
        (359., 359.5),
        (720., 540.),
        (1080., 720.),
        (-180., -90.),
        (180., 270.),
    ] {
        let mut document = two(0.5, true);
        document.bones[1].setup.rotation = authored;
        let pose = checked_evaluate_pose(&document, None, 0.).unwrap();
        near(pose.applied_locals()[1].rotation, expected);
        near(pose.applied_locals()[2].rotation, 45.);
    }
    for (authored, expected) in [(-180., -90.), (180., 270.)] {
        let mut document = one(0.5);
        document.bones[1].setup.rotation = authored;
        document.bones[3].setup = Transform {
            x: 10.,
            ..Transform::default()
        };
        near(
            checked_evaluate_pose(&document, None, 0.)
                .unwrap()
                .applied_locals()[1]
                .rotation,
            expected,
        );
    }
}

#[test]
fn first_link_is_the_child_offset_and_unequal_inner_outer_reach_is_limited_geometrically() {
    let mut document = two(1., true);
    document.bones[1].length = 99.;
    document.bones[2].setup.x = 7.;
    for (target, tip, parent_rotation, child_rotation) in [
        ([7., 10.], [7., 10.], 0., 90.),
        ([30., 0.], [17., 0.], 0., 0.),
        ([1., 0.], [3., 0.], -180., 180.),
    ] {
        document.bones[3].setup.x = target[0];
        document.bones[3].setup.y = target[1];
        let pose = checked_evaluate_pose(&document, None, 0.).unwrap();
        point(pose.world()[2].point(10., 0.), tip);
        near(pose.applied_locals()[1].rotation, parent_rotation);
        near(pose.applied_locals()[2].rotation, child_rotation);
        assert_eq!(pose.authored_locals()[2].x, 7.);
    }
}

#[test]
fn animated_targets_and_fractional_pending_poses_use_authored_data_then_one_solve() {
    let mut document = two(1., true);
    document.animations[0].tracks.push(Track {
        bone: 3,
        property: Property::X,
        keys: vec![
            Keyframe {
                time: 0.25,
                value: 0.,
                curve: Curve::Linear,
            },
            Keyframe {
                time: 0.75,
                value: -10.,
                curve: Curve::Stepped,
            },
        ],
    });
    let before = document.clone();
    for time in [0., 0.137, 0.25, 0.5, 0.75, 2.] {
        let pose = checked_evaluate_pose(&document, Some(0), time).unwrap();
        point(
            pose.world()[2].point(10., 0.),
            [pose.world()[3].x, pose.world()[3].y],
        );
    }
    assert_eq!(document, before);
    let mut session = Session::new(one(0.5)).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.137,
    };
    let mut upper = session.local_pose(Some(0), 0.137)[1];
    upper.rotation = 20.;
    session
        .edit_pose(1, target, upper, &[Property::Rotation], false)
        .unwrap();
    let mut goal = session.local_pose(Some(0), 0.137)[3];
    goal.y = 12.5;
    session
        .edit_pose(3, target, goal, &[Property::Y], false)
        .unwrap();
    let pending = session.pose_edits().clone();
    let saved = session.document().clone();
    let pose = session.evaluated_pose(target).unwrap();
    near(pose.applied_locals()[1].rotation, 55.);
    assert_eq!(pose.authored_locals()[1].rotation, 20.);
    let snapshot = session.mesh_bind_pose(target).unwrap();
    assert_eq!(snapshot.locals, pose.applied_locals());
    let fk = world_pose(session.document(), &snapshot.locals);
    point(fk[2].point(10., 0.), pose.world()[2].point(10., 0.));
    let twice = checked_setup_pose_from_locals(session.document(), snapshot.locals).unwrap();
    near(twice.applied_locals()[1].rotation, 72.5);
    assert_eq!(session.pose_edits(), &pending);
    assert_eq!(session.document(), &saved);
    assert!(!session.is_dirty());
}

#[test]
fn independent_constraints_keep_list_order_and_dependencies_are_explicit_errors() {
    let mut document = two(1., true);
    document.bones.extend([
        Bone {
            name: "second upper".into(),
            parent: Some(0),
            length: 10.,
            setup: Transform::default(),
            inherit: peachy_document::Inherit::Normal,
        },
        Bone {
            name: "second lower".into(),
            parent: Some(4),
            length: 10.,
            setup: Transform {
                x: 10.,
                ..Transform::default()
            },
            inherit: peachy_document::Inherit::Normal,
        },
        Bone {
            name: "second target".into(),
            parent: Some(0),
            length: 0.,
            setup: Transform {
                x: 10.,
                y: -10.,
                ..Transform::default()
            },
            inherit: peachy_document::Inherit::Normal,
        },
    ]);
    let mut other = constraint();
    other.name = "second".into();
    other.bones = vec![4, 5];
    other.target = 6;
    document.constraints.push(Constraint::Ik(other));
    document.validate().unwrap();
    let forward = checked_evaluate_pose(&document, None, 0.).unwrap();
    point(forward.world()[2].point(10., 0.), [10., 10.]);
    point(forward.world()[5].point(10., 0.), [10., -10.]);
    document.constraints.reverse();
    let reverse = checked_evaluate_pose(&document, None, 0.).unwrap();
    assert_eq!(forward.applied_locals(), reverse.applied_locals());
    let Constraint::Ik(second) = &mut document.constraints[0] else {
        unreachable!("fixture constraint is IK")
    };
    second.target = 2;
    assert!(
        checked_evaluate_pose(&document, None, 0.)
            .unwrap_err()
            .contains("inside another chain")
    );
    let Constraint::Ik(second) = &mut document.constraints[0] else {
        unreachable!("fixture constraint is IK")
    };
    second.target = 6;
    second.bones = vec![1];
    assert!(
        checked_evaluate_pose(&document, None, 0.)
            .unwrap_err()
            .contains("shared constrained bones")
    );
}

/// A one-bone aim nested below a two-bone chain (a toe below a leg) solves after
/// the chain in list order, against the chain's solved frame. Listing it first
/// solves it against the unsolved frame instead; both orders were matched with
/// the external runtime (docs/VALIDATION.md).
#[test]
fn nested_chains_solve_sequentially_in_constraint_order() {
    let mut document = two(1., true);
    document.bones.extend([
        Bone {
            name: "toe".into(),
            parent: Some(2),
            length: 4.,
            setup: Transform {
                x: 10.,
                ..Transform::default()
            },
            inherit: peachy_document::Inherit::Normal,
        },
        Bone {
            name: "toe target".into(),
            parent: Some(0),
            length: 0.,
            setup: Transform {
                x: 20.,
                y: 10.,
                ..Transform::default()
            },
            inherit: peachy_document::Inherit::Normal,
        },
    ]);
    let mut toe = constraint();
    toe.name = "toe".into();
    toe.bones = vec![4];
    toe.target = 5;
    document.constraints.push(Constraint::Ik(toe));
    document.validate().unwrap();
    let forward = checked_evaluate_pose(&document, None, 0.).unwrap();
    // The leg reaches its target, then the toe aims from the solved ankle.
    point(forward.world()[2].point(10., 0.), [10., 10.]);
    let ankle = forward.world()[4].point(0., 0.);
    point(ankle, [10., 10.]);
    let tip = forward.world()[4].point(4., 0.);
    point(tip, [14., 10.]);
    document.constraints.reverse();
    let reverse = checked_evaluate_pose(&document, None, 0.).unwrap();
    assert_ne!(forward.applied_locals(), reverse.applied_locals());
    point(reverse.world()[2].point(10., 0.), [10., 10.]);
}

#[test]
fn two_bone_child_offsets_off_the_parent_axis_reach_the_target() {
    for (x, y) in [(10., 3.), (6., -8.), (-4., 9.)] {
        for positive in [true, false] {
            let mut document = two(1., positive);
            document.bones[2].setup.x = x;
            document.bones[2].setup.y = y;
            document.bones[3].setup.x = 8.;
            document.bones[3].setup.y = 9.;
            document.validate().unwrap();
            let pose = checked_evaluate_pose(&document, None, 0.).unwrap();
            point(pose.world()[2].point(10., 0.), [8., 9.]);
            // The authored child offset is untouched; only rotations are applied.
            assert_eq!(pose.applied_locals()[2].x, x);
            assert_eq!(pose.applied_locals()[2].y, y);
        }
    }
}

#[test]
fn unsupported_or_degenerate_queries_preserve_raw_native_data_pending_pose_and_history() {
    let baseline = two(1., true);
    for (bone, setup) in [
        (
            0,
            Transform {
                scale_x: 0.,
                ..Transform::default()
            },
        ),
        (
            1,
            Transform {
                scale_x: -1.,
                ..Transform::default()
            },
        ),
        (
            1,
            Transform {
                scale_x: 2.,
                scale_y: 2.,
                ..Transform::default()
            },
        ),
        (
            1,
            Transform {
                shear_x: 20.,
                ..Transform::default()
            },
        ),
        (2, Transform::default()),
    ] {
        let mut document = baseline.clone();
        document.bones[bone].setup = setup;
        document.validate().unwrap();
        let before = document.clone();
        assert!(checked_evaluate_pose(&document, None, 0.).is_err());
        assert_eq!(
            serde_json::from_str::<Document>(&serde_json::to_string(&document).unwrap()).unwrap(),
            before
        );
    }
    let mut document = baseline.clone();
    document.bones[2].length = 0.;
    assert!(checked_evaluate_pose(&document, None, 0.).is_err());
    let mut document = baseline.clone();
    document.bones[3].setup.x = 0.;
    document.bones[3].setup.y = 0.;
    assert!(
        checked_evaluate_pose(&document, None, 0.)
            .unwrap_err()
            .contains("noncoincident")
    );
    let mut unrelated = baseline;
    unrelated.bones.push(Bone {
        name: "unrelated".into(),
        parent: Some(0),
        length: 0.,
        setup: Transform {
            scale_x: 0.,
            scale_y: -2.,
            shear_y: 45.,
            ..Transform::default()
        },
        inherit: peachy_document::Inherit::Normal,
    });
    assert!(checked_evaluate_pose(&unrelated, None, 0.).is_ok());
    let mut session = Session::new(unrelated).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.137,
    };
    let mut child = session.local_pose(Some(0), 0.137)[2];
    child.x = 11.;
    session
        .edit_pose(2, target, child, &[Property::X], false)
        .unwrap();
    session
        .dispatch(Command::RenameBone {
            bone: 3,
            name: "renamed".into(),
        })
        .unwrap();
    session.undo();
    let document = session.document().clone();
    let pending = session.pose_edits().clone();
    assert!(session.can_redo());
    assert!(
        session
            .evaluated_pose(target)
            .unwrap_err()
            .contains("constant nonzero setup offset")
    );
    assert_eq!(session.document(), &document);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
    assert!(!session.is_dirty());
}

#[test]
fn coordinate_precision_guard_rejects_loss_and_accepts_recoverable_large_frames() {
    let mut document = one(1.);
    document.bones[0].setup.x = 1e8;
    document.bones[1].setup.rotation = 90.;
    document.bones[3].setup = Transform {
        x: 32.,
        ..Transform::default()
    };
    let before = document.clone();
    assert!(
        checked_evaluate_pose(&document, None, 0.)
            .unwrap_err()
            .contains("coordinate precision")
    );
    assert_eq!(document, before);
    document.bones[1].length = 16.;
    document.bones[2].setup.x = 16.;
    document.bones[2].length = 16.;
    let pose = checked_evaluate_pose(&document, None, 0.).unwrap();
    point(pose.world()[1].point(16., 0.), [100000016., 0.]);
    let mut document = one(1.);
    document.bones[1].length = 1e8;
    document.bones[3].setup.x = 10.;
    assert!(
        checked_evaluate_pose(&document, None, 0.)
            .unwrap_err()
            .contains("coordinate precision")
    );
}

#[test]
fn relevant_target_precision_cannot_hide_a_wrong_orientation_behind_zero_length() {
    for length in [0., 16.] {
        let mut document = one(1.);
        document.bones.remove(2);
        let Constraint::Ik(ik) = &mut document.constraints[0] else {
            unreachable!("fixture constraint is IK")
        };
        ik.target = 2;
        document.bones[0].setup.x = 1e8;
        document.bones[1].length = length;
        document.bones[2].setup.x = 10.;
        document.bones[2].setup.y = 10.;
        document.validate().unwrap();
        let result = checked_evaluate_pose(&document, None, 0.);
        println!(
            "original large-origin length={length}: {:?}",
            result
                .as_ref()
                .map(|pose| pose.applied_locals()[1].rotation)
        );
        match result {
            Ok(pose) => near(pose.applied_locals()[1].rotation, 45.),
            Err(error) => assert!(error.contains("coordinate precision"), "{error}"),
        }
    }
}

#[test]
fn relevant_wide_target_math_accepts_exact_large_and_tiny_directions_without_global_bans() {
    let mut document = one(1.);
    document.bones.remove(2);
    let Constraint::Ik(ik) = &mut document.constraints[0] else {
        unreachable!("fixture constraint is IK")
    };
    ik.target = 2;
    document.bones[0].setup.x = 1e8;
    document.bones[1].setup.rotation = 90.;
    document.bones[1].length = 16.;
    document.bones[2].setup = Transform {
        x: 32.,
        ..Transform::default()
    };
    let pose = checked_evaluate_pose(&document, None, 0.).unwrap();
    near(pose.applied_locals()[1].rotation, 0.);
    point(pose.world()[1].point(16., 0.), [100000016., 0.]);
    document.bones[0].setup.x = 1.;
    document.bones[1].length = 0.;
    document.bones[1].setup.rotation = 0.;
    document.bones[2].setup.x = 2e-8;
    document.bones[2].setup.y = 2e-8;
    let pose = checked_evaluate_pose(&document, None, 0.).unwrap();
    near(pose.applied_locals()[1].rotation, 45.);
    // Existing f32 FK loses some precision on this unrelated branch. It is not
    // an input to the active IK and must not become a blanket large-pose ban.
    document.bones.extend([
        Bone {
            name: "unrelated root".into(),
            parent: None,
            length: 0.,
            setup: Transform {
                x: 1e8,
                ..Transform::default()
            },
            inherit: peachy_document::Inherit::Normal,
        },
        Bone {
            name: "unrelated child".into(),
            parent: Some(3),
            length: 0.,
            setup: Transform {
                x: 10.,
                y: 10.,
                ..Transform::default()
            },
            inherit: peachy_document::Inherit::Normal,
        },
    ]);
    document.validate().unwrap();
    near(
        checked_evaluate_pose(&document, None, 0.)
            .unwrap()
            .applied_locals()[1]
            .rotation,
        45.,
    );
}

#[test]
fn partial_mix_weight_binding_add_and_unbind_use_applied_snapshot_once_with_pending_history() {
    let mut document = one(0.5);
    document.slots.push(Slot {
        color: [255; 4],
        name: "surface".into(),
        bone: 0,
        attachment: Some("triangle".into()),
    });
    document.skins = BTreeMap::from([(
        "default".into(),
        BTreeMap::from([(
            "surface".into(),
            BTreeMap::from([(
                "triangle".into(),
                Attachment::Mesh(Mesh {
                    name: None,
                    path: "original-ik-triangle".into(),
                    vertices: MeshVertices::SlotLocal {
                        positions: vec![[0., 0.], [2., 0.], [0., 1.]],
                    },
                    uvs: vec![[0., 0.], [1., 0.], [0., 1.]],
                    triangles: vec![0, 1, 2],
                    hull: 3,
                    edges: vec![],
                    width: 0.,
                    height: 0.,
                }),
            )]),
        )]),
    )]);
    let attachment = AttachmentRef {
        skin: "default".into(),
        slot: "surface".into(),
        placeholder: "triangle".into(),
    };
    let mut session = Session::new(document).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.137,
    };
    let mut upper = session.local_pose(Some(0), 0.137)[1];
    upper.rotation = 20.;
    session
        .edit_pose(1, target, upper, &[Property::Rotation], false)
        .unwrap();
    let mut goal = session.local_pose(Some(0), 0.137)[3];
    goal.y = 12.5;
    session
        .edit_pose(3, target, goal, &[Property::Y], false)
        .unwrap();
    let pending = session.pose_edits().clone();
    let baseline = session.document().clone();
    let scene = |session: &Session| {
        checked_setup_scene_from_locals(
            session.document(),
            "default",
            session.local_pose(Some(0), 0.137),
        )
        .unwrap()
    };
    let original = scene(&session);
    let snapshot = session.mesh_bind_pose(target).unwrap();
    near(snapshot.locals[1].rotation, 55.);
    session
        .bind_mesh_to_bone(attachment.clone(), 1, target)
        .unwrap();
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.is_dirty());
    let bound = scene(&session);
    for (&actual, &expected) in bound.attachments[0]
        .vertices
        .iter()
        .zip(&original.attachments[0].vertices)
    {
        point(actual, expected);
    }
    point(bound.bounds.unwrap().min, original.bounds.unwrap().min);
    point(bound.bounds.unwrap().max, original.bounds.unwrap().max);
    assert_eq!(
        pick_attachment(&bound.attachments, [0.5, 0.25]).map(|draw| &draw.attachment),
        Some(&attachment)
    );
    session.undo();
    assert_eq!(session.document(), &baseline);
    assert_eq!(session.pose_edits(), &pending);
    assert!(!session.is_dirty());
    session.redo();
    session
        .add_mesh_bone_binding(attachment.clone(), 3, target)
        .unwrap();
    let bound_payload = session.document().clone();
    let decoded: Document =
        serde_json::from_str(&serde_json::to_string(&bound_payload).unwrap()).unwrap();
    decoded.validate().unwrap();
    assert_eq!(decoded, bound_payload);
    session.unbind_mesh(attachment.clone(), target).unwrap();
    for (&actual, &expected) in scene(&session).attachments[0]
        .vertices
        .iter()
        .zip(&original.attachments[0].vertices)
    {
        point(actual, expected);
    }
    session.undo();
    assert_eq!(session.document(), &bound_payload);
    assert_eq!(session.pose_edits(), &pending);
    session.mark_saved();
    assert!(!session.is_dirty());
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.document().animations[0].tracks.is_empty());
}

#[test]
fn required_singular_frames_and_between_key_coincidence_are_queries_not_destructive_edits() {
    let mut singular = one(1.);
    singular.bones[0].setup.rotation = 1e20;
    singular.validate().unwrap();
    let original = singular.clone();
    assert!(
        checked_evaluate_pose(&singular, None, 0.)
            .unwrap_err()
            .contains("invertible")
    );
    assert_eq!(singular, original);
    let mut document = one(1.);
    document.bones[3].setup.y = 0.;
    document.animations[0].tracks.push(Track {
        bone: 3,
        property: Property::X,
        keys: vec![
            Keyframe {
                time: 0.,
                value: -1.,
                curve: Curve::Linear,
            },
            Keyframe {
                time: 1.,
                value: 1.,
                curve: Curve::Linear,
            },
        ],
    });
    let mut session = Session::new(document).unwrap();
    session
        .dispatch(Command::RenameBone {
            bone: 3,
            name: "temporary".into(),
        })
        .unwrap();
    session.undo();
    let original = session.document().clone();
    let pending = session.pose_edits().clone();
    assert!(
        session
            .evaluated_pose(EditTarget::Animate {
                animation: 0,
                time: 0.25
            })
            .is_ok()
    );
    assert!(
        session
            .evaluated_pose(EditTarget::Animate {
                animation: 0,
                time: 0.5
            })
            .unwrap_err()
            .contains("noncoincident")
    );
    assert!(
        session
            .evaluated_pose(EditTarget::Animate {
                animation: 0,
                time: 0.75
            })
            .is_ok()
    );
    assert_eq!(session.document(), &original);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
    assert!(!session.is_dirty());
}

#[test]
fn local_move_projects_the_drawn_applied_axis_and_preserves_raw_authored_values() {
    let document = one(0.5);
    let pose = checked_evaluate_pose(&document, None, 0.).unwrap();
    let parent = pose.world()[0];
    let world = pose.world()[1];
    let authored = pose.authored_locals()[1];
    let applied = pose.applied_locals()[1];
    near(authored.rotation, 0.);
    near(applied.rotation, 45.);
    let basis = transform_axes_from_world(
        applied,
        parent,
        world,
        Axes::Local,
        TransformTool::Translate,
    )[0];
    let result = drag_transform_in_frame(
        authored,
        applied,
        parent,
        world,
        TransformGesture {
            tool: TransformTool::Translate,
            axes: Axes::Local,
            axis: Some(0),
            start: [world.x, world.y],
            end: [world.x + basis[0] * 10., world.y + basis[1] * 10.],
            snap_rotation: false,
        },
    )
    .unwrap();
    near(result.x, 10. * std::f32::consts::FRAC_1_SQRT_2);
    near(result.y, 10. * std::f32::consts::FRAC_1_SQRT_2);
    assert_eq!(result.rotation, 0.);
    assert_eq!(result.scale_x, 1.);
    assert_eq!(result.shear_x, 0.);
    // The shared pose boundary receives authored edits, not the solved45 value.
    let mut session = Session::new(document).unwrap();
    session
        .edit_pose(
            1,
            EditTarget::Setup,
            result,
            &[Property::X, Property::Y],
            false,
        )
        .unwrap();
    assert_eq!(session.document().bones[1].setup.rotation, 0.);
    point(
        session.evaluated_pose(EditTarget::Setup).unwrap().world()[1].point(0., 0.),
        [result.x, result.y],
    );
}

#[test]
fn applied_pivot_and_scale_frame_produce_deltas_on_the_distinct_authored_baseline() {
    let authored = Transform {
        x: 3.,
        y: -4.,
        rotation: 720.,
        scale_x: 2.,
        scale_y: 3.,
        shear_x: 5.,
        shear_y: 7.,
    };
    let applied = Transform {
        x: 100.,
        y: 200.,
        rotation: 45.,
        ..Transform::default()
    };
    let parent = Affine::from_transform(Transform::default());
    let world = parent.then(Affine::from_transform(applied));
    let gesture = |tool, axis, start, end| TransformGesture {
        tool,
        axes: Axes::Local,
        axis,
        start,
        end,
        snap_rotation: false,
    };
    let start = [world.x + 10., world.y];
    let end = [world.x, world.y + 10.];
    let rotated = drag_transform_in_frame(
        authored,
        applied,
        parent,
        world,
        gesture(TransformTool::Rotate, None, start, end),
    )
    .unwrap();
    near(rotated.rotation, 810.);
    assert_eq!(
        (rotated.x, rotated.y, rotated.scale_x, rotated.shear_y),
        (3., -4., 2., 7.)
    );
    let sheared = drag_transform_in_frame(
        authored,
        applied,
        parent,
        world,
        gesture(TransformTool::Shear, Some(1), start, end),
    )
    .unwrap();
    near(sheared.shear_y, 97.);
    assert_eq!(
        (sheared.rotation, sheared.shear_x, sheared.x),
        (720., 5., 3.)
    );
    let start = world.point(1., 0.);
    let end = world.point(2., 0.);
    let scaled = drag_transform_in_frame(
        authored,
        applied,
        parent,
        world,
        gesture(TransformTool::Scale, Some(0), start, end),
    )
    .unwrap();
    near(scaled.scale_x, 4.);
    assert_eq!(
        (scaled.scale_y, scaled.rotation, scaled.shear_x, scaled.x),
        (3., 720., 5., 3.)
    );
    let mut invalid_world = world;
    invalid_world.x = f32::NAN;
    assert!(
        drag_transform_in_frame(
            authored,
            applied,
            parent,
            invalid_world,
            gesture(TransformTool::Rotate, None, start, end)
        )
        .is_err()
    );
    assert_eq!(authored.rotation, 720.);
}

#[test]
fn constraint_previews_restore_fixed_base_pending_saved_state_redo_and_commit_one_undo() {
    let mut session = Session::new(one(0.)).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.137,
    };
    let mut upper = session.local_pose(Some(0), 0.137)[1];
    upper.rotation = 15.;
    session
        .edit_pose(1, target, upper, &[Property::Rotation], false)
        .unwrap();
    session
        .dispatch(Command::RenameBone {
            bone: 3,
            name: "temporary".into(),
        })
        .unwrap();
    session.undo();
    let baseline = session.document().clone();
    let pending = session.pose_edits().clone();
    assert!(session.can_redo());
    session.begin_transaction();
    let mut edited = constraint();
    edited.bones = vec![1];
    edited.setup.mix = 0.5;
    session
        .preview_transaction(Command::SetIkConstraint {
            constraint: 0,
            value: edited.clone(),
        })
        .unwrap();
    near(
        session.evaluated_pose(target).unwrap().applied_locals()[1].rotation,
        52.5,
    );
    edited.setup.mix = 0.25;
    session
        .preview_transaction(Command::SetIkConstraint {
            constraint: 0,
            value: edited.clone(),
        })
        .unwrap();
    near(
        session.evaluated_pose(target).unwrap().applied_locals()[1].rotation,
        33.75,
    );
    let valid = session.document().clone();
    let mut invalid = edited.clone();
    invalid.target = 99;
    assert!(
        session
            .preview_transaction(Command::SetIkConstraint {
                constraint: 0,
                value: invalid
            })
            .is_err()
    );
    assert_eq!(session.document(), &valid);
    session.cancel_transaction();
    assert_eq!(session.document(), &baseline);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
    assert!(!session.is_dirty());
    session.begin_transaction();
    session
        .preview_transaction(Command::SetIkConstraint {
            constraint: 0,
            value: edited,
        })
        .unwrap();
    session.commit_transaction();
    assert!(session.is_dirty());
    session.undo();
    assert_eq!(session.document(), &baseline);
    assert_eq!(session.pose_edits(), &pending);
    assert!(!session.is_dirty());
}

#[test]
fn evaluated_draw_axes_use_the_supplied_world_without_recomposing_authored_rotation() {
    let local = Transform::default();
    let parent = Affine::from_transform(local);
    let world = Affine::from_transform(Transform {
        rotation: 90.,
        ..local
    });
    for tool in [TransformTool::Scale, TransformTool::Shear] {
        let axes = transform_axes_from_world(local, parent, world, Axes::World, tool);
        assert!(axes[0][0].abs() < 0.0001 && (axes[0][1] - 1.).abs() < 0.0001);
        assert!((axes[1][0] + 1.).abs() < 0.0001 && axes[1][1].abs() < 0.0001);
    }
    assert_eq!(
        transform_axes_from_world(local, parent, world, Axes::World, TransformTool::Translate),
        [[1., 0.], [0., 1.]]
    );
}

#[test]
fn native6_requires_strict_constraint_and_setup_data() {
    let mut document = rig();
    document.constraints.push(Constraint::Ik(constraint()));
    let value = serde_json::to_value(&document).unwrap();
    let decoded: Document = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(decoded, document);
    decoded.validate().unwrap();
    for path in [
        vec!["constraints"],
        vec!["constraints", "setup"],
        vec!["constraints", "mix"],
        vec!["constraints", "bend_positive"],
    ] {
        let mut invalid = value.clone();
        if path.len() == 1 {
            invalid.as_object_mut().unwrap().remove("constraints");
        } else if path[1] == "setup" {
            invalid["constraints"][0]
                .as_object_mut()
                .unwrap()
                .remove("setup");
        } else {
            invalid["constraints"][0]["setup"]
                .as_object_mut()
                .unwrap()
                .remove(path[1]);
        }
        assert!(serde_json::from_value::<Document>(invalid).is_err());
    }
    for field in ["order", "enabled", "softness"] {
        let mut invalid = value.clone();
        invalid["constraints"][0][field] = 0.into();
        assert!(serde_json::from_value::<Document>(invalid).is_err());
    }
    let mut invalid = value;
    invalid["constraints"][0]["setup"]["stretch"] = false.into();
    assert!(serde_json::from_value::<Document>(invalid).is_err());
}

#[test]
fn constraint_commands_validate_structure_atomically_and_preserve_order() {
    let document = rig();
    let first = apply_command(
        &document,
        Command::AddIkConstraint {
            constraint: constraint(),
        },
    )
    .unwrap();
    let mut other = constraint();
    other.name = "second".into();
    other.bones = vec![1];
    let second = apply_command(
        &first,
        Command::AddIkConstraint {
            constraint: other.clone(),
        },
    )
    .unwrap();
    assert_eq!(
        second.constraints,
        vec![Constraint::Ik(constraint()), Constraint::Ik(other)]
    );
    for bones in [vec![], vec![1, 1], vec![2, 1], vec![0, 1, 2], vec![99]] {
        let mut invalid = constraint();
        invalid.bones = bones;
        assert!(
            apply_command(
                &first,
                Command::SetIkConstraint {
                    constraint: 0,
                    value: invalid
                }
            )
            .is_err()
        );
    }
    for target in [1, 2, 99] {
        let mut invalid = constraint();
        invalid.target = target;
        assert!(
            apply_command(
                &first,
                Command::SetIkConstraint {
                    constraint: 0,
                    value: invalid
                }
            )
            .is_err()
        );
    }
    for mix in [f32::NAN, f32::INFINITY] {
        let mut invalid = constraint();
        invalid.setup.mix = mix;
        assert!(
            apply_command(
                &first,
                Command::SetIkConstraint {
                    constraint: 0,
                    value: invalid
                }
            )
            .is_err()
        );
    }
    assert!(
        apply_command(
            &first,
            Command::AddIkConstraint {
                constraint: constraint()
            }
        )
        .is_err()
    );
    assert!(
        apply_command(
            &first,
            Command::SetIkConstraint {
                constraint: 99,
                value: constraint()
            }
        )
        .is_err()
    );
    assert_eq!(first.constraints, vec![Constraint::Ik(constraint())]);
    let command = Command::SetIkConstraint {
        constraint: 0,
        value: constraint(),
    };
    let mut payload = serde_json::to_value(command).unwrap();
    payload["value"]["setup"]["uniform"] = true.into();
    assert!(serde_json::from_value::<Command>(payload).is_err());
}

#[test]
fn checked_fk_and_disabled_ik_keep_authored_bits_and_reject_invalid_queries() {
    let mut document = rig();
    document.bones[1].setup.scale_x = -0.0;
    document.bones[1].setup.shear_y = 35.;
    let mut disabled = constraint();
    disabled.setup.mix = 0.;
    document.constraints.push(Constraint::Ik(disabled));
    document.validate().unwrap();
    let before = document.clone();
    let authored = local_pose(&document, Some(0), 0.137);
    let expected = world_pose(&document, &authored);
    let evaluated = checked_evaluate_pose(&document, Some(0), 0.137).unwrap();
    assert_eq!(evaluated.authored_locals(), authored);
    assert_eq!(evaluated.applied_locals(), authored);
    for (actual, expected) in evaluated.world().iter().zip(expected) {
        assert_eq!(
            [actual.a, actual.b, actual.c, actual.d, actual.x, actual.y].map(f32::to_bits),
            [
                expected.a, expected.b, expected.c, expected.d, expected.x, expected.y
            ]
            .map(f32::to_bits)
        );
    }
    assert!(checked_evaluate_pose(&document, Some(99), 0.).is_err());
    assert!(checked_evaluate_pose(&document, None, f32::NAN).is_err());
    assert!(checked_evaluate_pose(&document, None, -0.1).is_err());
    assert!(checked_setup_pose_from_locals(&document, vec![]).is_err());
    assert_eq!(document, before);
    let mut session = Session::new(document).unwrap();
    let mut target = session.local_pose(Some(0), 0.137)[3];
    target.x = 12.5;
    session
        .edit_pose(
            3,
            EditTarget::Animate {
                animation: 0,
                time: 0.137,
            },
            target,
            &[Property::X],
            false,
        )
        .unwrap();
    let pending = session.pose_edits().clone();
    let posed = session
        .evaluated_pose(EditTarget::Animate {
            animation: 0,
            time: 0.137,
        })
        .unwrap();
    assert_eq!(posed.authored_locals()[3].x, 12.5);
    assert_eq!(
        session
            .mesh_bind_pose(EditTarget::Animate {
                animation: 0,
                time: 0.137
            })
            .unwrap()
            .locals,
        posed.applied_locals()
    );
    assert_eq!(session.pose_edits(), &pending);
    assert!(!session.is_dirty());
}
