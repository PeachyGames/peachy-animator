use crate::*;
use peachy_document::*;
use std::collections::BTreeMap;

fn fixture() -> Document {
    let mut doc = Document::new("Original multi-bone authoring fixture".into(), 30).unwrap();
    doc.bones[0].setup.x = 2.;
    doc.bones[0].setup.y = -1.;
    for (name, parent, x, y) in [("child", 0, 4., 3.), ("sibling", 0, -2., 1.)] {
        doc.bones.push(Bone {
            name: name.into(),
            parent: Some(parent),
            length: 10.,
            setup: Transform {
                x,
                y,
                ..Transform::default()
            },
            inherit: Inherit::Normal,
        });
    }
    let adjacent = f32::from_bits(0.4_f32.to_bits() + 1);
    doc.animations.push(Animation::new(
        "motion",
        vec![
            Track {
                bone: 0,
                property: Property::X,
                keys: vec![
                    Keyframe {
                        time: 0.4,
                        value: 5.,
                        curve: Curve::BezierOffset([0.2, 3., 0.8, -2.]),
                    },
                    Keyframe {
                        time: adjacent,
                        value: 9.,
                        curve: Curve::Stepped,
                    },
                ],
            },
            Track {
                bone: 2,
                property: Property::Rotation,
                keys: vec![Keyframe {
                    time: 0.,
                    value: -17.,
                    curve: Curve::Stepped,
                }],
            },
        ],
    ));
    doc.validate().unwrap();
    doc
}

fn animate(time: f32) -> EditTarget {
    EditTarget::Animate { animation: 0, time }
}

fn edit(bone: usize, transform: Transform, properties: &[Property]) -> BonePoseEdit {
    BonePoseEdit {
        bone,
        transform,
        properties: properties.to_vec(),
    }
}

fn key(bone: usize, properties: &[Property]) -> BonePoseKey {
    BonePoseKey {
        bone,
        properties: properties.to_vec(),
    }
}

fn snapshot(session: &Session) -> serde_json::Value {
    serde_json::json!({
        "document": session.document(),
        "pending": session.pose_edits(),
        "dirty": session.is_dirty(),
        "undo": session.can_undo(),
        "redo": session.can_redo(),
        "generation": session.document_generation(),
        "skin": session.skin(),
        "effects": session.last_edit_effects(),
    })
}

fn with_redo() -> Session {
    let mut session = Session::new(fixture()).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo branch".into(),
        })
        .unwrap();
    session.undo();
    assert!(session.can_redo());
    session
}

#[test]
fn multi_bone_setup_parent_child_results_publish_once_and_undo_together() {
    let doc = fixture();
    let mut session = Session::new(doc.clone()).unwrap();
    let root = Transform {
        x: 10.,
        y: -1.,
        rotation: 90.,
        ..doc.bones[0].setup
    };
    let child = Transform {
        x: 7.,
        y: 0.,
        ..doc.bones[1].setup
    };
    session
        .edit_poses(
            EditTarget::Setup,
            &[
                edit(0, root, &[Property::Rotation]),
                edit(1, child, &[Property::X]),
            ],
            false,
        )
        .unwrap();
    // Setup deliberately follows the existing complete-transform SetPose contract.
    assert_eq!(session.document().bones[0].setup, root);
    assert_eq!(session.document().bones[1].setup, child);
    assert_eq!(session.document().bones[2], doc.bones[2]);
    let applied = session.evaluated_pose(EditTarget::Setup).unwrap();
    assert!((applied.world()[1].x - 10.).abs() < 1e-5);
    assert!((applied.world()[1].y - 6.).abs() < 1e-5);
    assert!(session.is_dirty());
    assert_eq!(session.document_generation(), 1);
    session.undo();
    assert_eq!(session.document(), &doc);
    assert!(!session.can_undo());
    assert!(!session.is_dirty());
    session.redo();
    assert_eq!(session.document().bones[0].setup, root);
    assert_eq!(session.document().bones[1].setup, child);
}

#[test]
fn multi_bone_pending_edits_and_selected_keying_preserve_unrelated_channels_and_history() {
    let doc = fixture();
    let mut session = Session::new(doc.clone()).unwrap();
    let mut locals = session
        .authored_pose(animate(0.4))
        .unwrap()
        .bone_locals()
        .to_vec();
    locals[2].y = 25.;
    session
        .edit_pose(2, animate(0.4), locals[2], &[Property::Y], false)
        .unwrap();
    let initial_pending = session.pose_edits().clone();
    locals[0].rotation = 31.;
    locals[0].x = 19.;
    locals[1].scale_x = -1.5;
    session
        .edit_poses(
            animate(0.4),
            &[
                edit(0, locals[0], &[Property::X, Property::Rotation]),
                edit(1, locals[1], &[Property::ScaleX]),
            ],
            false,
        )
        .unwrap();
    assert_eq!(session.document(), &doc);
    assert!(!session.is_dirty());
    let pending = session.pose_edits().clone();
    assert_eq!(
        session.authored_pose(animate(0.4)).unwrap().bone_locals(),
        locals
    );
    session.undo();
    assert_eq!(session.pose_edits(), &initial_pending);
    session.redo();
    assert_eq!(session.pose_edits(), &pending);

    session
        .key_poses(
            0,
            0.4,
            &[key(0, &[Property::X]), key(1, &[Property::ScaleX])],
        )
        .unwrap();
    let root_x = session.document().animations[0]
        .tracks
        .iter()
        .find(|t| t.bone == 0 && t.property == Property::X)
        .unwrap();
    let original_x = &doc.animations[0].tracks[0];
    assert_eq!(
        root_x.keys[0].time.to_bits(),
        original_x.keys[0].time.to_bits()
    );
    assert_eq!(root_x.keys[0].curve, original_x.keys[0].curve);
    assert_eq!(root_x.keys[1], original_x.keys[1]);
    assert_eq!(root_x.keys[0].value, 17.);
    assert_eq!(
        session
            .pose_edits()
            .key_state(session.document(), 0, 0.4, 0, &[Property::Rotation]),
        KeyState::Edited
    );
    assert_eq!(
        session
            .pose_edits()
            .key_state(session.document(), 0, 0.4, 2, &[Property::Y]),
        KeyState::Edited
    );
    assert!(
        !session.document().animations[0]
            .tracks
            .iter()
            .any(|t| t.bone == 1 && t.property == Property::ScaleY)
    );
    let keyed = session.document().clone();
    let keyed_pending = session.pose_edits().clone();
    session.undo();
    assert_eq!(session.document(), &doc);
    assert_eq!(session.pose_edits(), &pending);
    session.redo();
    assert_eq!(session.document(), &keyed);
    assert_eq!(session.pose_edits(), &keyed_pending);
}

#[test]
fn multi_bone_auto_key_expands_groups_from_one_pending_snapshot() {
    let doc = fixture();
    let mut session = Session::new(doc.clone()).unwrap();
    let mut locals = session
        .authored_pose(animate(0.4))
        .unwrap()
        .bone_locals()
        .to_vec();
    locals[0].y = 23.;
    locals[0].rotation = 41.;
    session
        .edit_pose(
            0,
            animate(0.4),
            locals[0],
            &[Property::Y, Property::Rotation],
            false,
        )
        .unwrap();
    locals[1].scale_y = 3.;
    session
        .edit_pose(1, animate(0.4), locals[1], &[Property::ScaleY], false)
        .unwrap();
    let prior = session.pose_edits().clone();
    locals[0].x = 16.;
    locals[1].scale_x = -2.;
    session
        .edit_poses(
            animate(0.4),
            &[
                edit(0, locals[0], &[Property::X]),
                edit(1, locals[1], &[Property::ScaleX]),
            ],
            true,
        )
        .unwrap();
    let authored = session.authored_pose(animate(0.4)).unwrap();
    assert_eq!(authored.bone_locals(), locals);
    for (bone, properties) in [
        (0, [Property::X, Property::Y]),
        (1, [Property::ScaleX, Property::ScaleY]),
    ] {
        for property in properties {
            let track = session.document().animations[0]
                .tracks
                .iter()
                .find(|t| t.bone == bone && t.property == property)
                .unwrap();
            assert!(
                track
                    .keys
                    .iter()
                    .any(|k| k.time.to_bits() == 0.4_f32.to_bits())
            );
            assert_eq!(
                session
                    .pose_edits()
                    .key_state(session.document(), 0, 0.4, bone, &[property]),
                KeyState::Keyed
            );
        }
    }
    assert_eq!(
        session
            .pose_edits()
            .key_state(session.document(), 0, 0.4, 0, &[Property::Rotation]),
        KeyState::Edited
    );
    let keyed = session.document().clone();
    session.undo();
    assert_eq!(session.document(), &doc);
    assert_eq!(session.pose_edits(), &prior);
    session.redo();
    assert_eq!(session.document(), &keyed);
}

#[test]
fn multi_bone_invalid_members_and_targets_preserve_redo_document_and_pending() {
    let mut session = with_redo();
    let root = session.document().bones[0].setup;
    let child = session.document().bones[1].setup;
    let good = edit(0, Transform { x: 8., ..root }, &[Property::X]);
    let mut cases = vec![
        vec![],
        vec![good.clone(), edit(0, child, &[Property::Y])],
        vec![good.clone(), edit(1, child, &[])],
        vec![good.clone(), edit(1, child, &[Property::X, Property::X])],
        vec![good.clone(), edit(usize::MAX, child, &[Property::X])],
    ];
    for property in Property::ALL {
        let mut nonfinite = child;
        property.write(&mut nonfinite, f32::INFINITY);
        cases.push(vec![good.clone(), edit(1, nonfinite, &[Property::X])]);
    }
    let before = snapshot(&session);
    for target in [EditTarget::Setup, animate(0.4)] {
        for edits in &cases {
            for auto_key in [false, true] {
                assert!(session.edit_poses(target, edits, auto_key).is_err());
                assert_eq!(snapshot(&session), before);
            }
        }
    }
    for target in [
        animate(-0.1),
        animate(f32::NAN),
        EditTarget::Animate {
            animation: 99,
            time: 0.,
        },
    ] {
        assert!(
            session
                .edit_poses(target, std::slice::from_ref(&good), false)
                .is_err()
        );
        assert_eq!(snapshot(&session), before);
    }
    for keys in [
        vec![],
        vec![key(0, &[Property::X]), key(0, &[Property::Y])],
        vec![key(0, &[Property::X]), key(1, &[])],
        vec![key(1, &[Property::Y, Property::Y])],
        vec![key(usize::MAX, &[Property::X])],
    ] {
        assert!(session.key_poses(0, 0.4, &keys).is_err());
        assert_eq!(snapshot(&session), before);
    }
    assert!(
        session
            .key_poses(0, f32::INFINITY, &[key(0, &[Property::X])])
            .is_err()
    );
    assert_eq!(snapshot(&session), before);
    session.redo();
    assert_eq!(session.document().animations[0].name, "redo branch");
}

#[test]
fn multi_bone_fixed_base_preview_cancels_and_failed_last_auto_key_keeps_valid_preview() {
    let mut session = with_redo();
    let original = session.document().clone();
    let base = session
        .authored_pose(animate(0.4))
        .unwrap()
        .bone_locals()
        .to_vec();
    let first = [
        edit(0, Transform { x: 22., ..base[0] }, &[Property::X]),
        edit(
            1,
            Transform {
                rotation: 27.,
                ..base[1]
            },
            &[Property::Rotation],
        ),
    ];
    assert!(
        session
            .preview_edit_poses(animate(0.4), &first, false)
            .is_err()
    );
    session.begin_transaction();
    session
        .preview_edit_poses(animate(0.4), &first, true)
        .unwrap();
    // A second explicit member list replaces the first preview, rather than
    // retaining its keyed X/Rotation channels as an accumulated delta.
    let second = [
        edit(0, Transform { y: 33., ..base[0] }, &[Property::Y]),
        edit(
            1,
            Transform {
                scale_x: 2.,
                ..base[1]
            },
            &[Property::ScaleX],
        ),
    ];
    session
        .preview_edit_poses(animate(0.4), &second, true)
        .unwrap();
    assert_eq!(
        session.document().animations[0].tracks[0],
        original.animations[0].tracks[0]
    );
    assert!(
        !session.document().animations[0]
            .tracks
            .iter()
            .any(|t| t.bone == 1 && t.property == Property::Rotation)
    );
    let valid = snapshot(&session);
    let mut bad = second.to_vec();
    bad[1].transform.y = f32::NAN;
    assert!(
        session
            .preview_edit_poses(animate(0.4), &bad, true)
            .is_err()
    );
    assert_eq!(snapshot(&session), valid);
    session.cancel_transaction();
    assert_eq!(session.document(), &original);
    assert!(session.pose_edits().is_empty());
    assert!(session.can_redo());
    assert!(!session.is_dirty());
    session.redo();
    assert_eq!(session.document().animations[0].name, "redo branch");

    // A failure from a later member's transform group also publishes nothing.
    let mut zero_setup = fixture();
    zero_setup.bones[1].setup.scale_y = 0.;
    let mut session = Session::new(zero_setup).unwrap();
    session.begin_transaction();
    let mut locals = session
        .authored_pose(animate(0.4))
        .unwrap()
        .bone_locals()
        .to_vec();
    locals[0].x = 17.;
    session
        .preview_edit_poses(animate(0.4), &[edit(0, locals[0], &[Property::X])], true)
        .unwrap();
    let before = snapshot(&session);
    locals[1].scale_x = 2.;
    assert!(
        session
            .preview_edit_poses(
                animate(0.4),
                &[
                    edit(0, locals[0], &[Property::X]),
                    edit(1, locals[1], &[Property::ScaleX])
                ],
                true
            )
            .is_err()
    );
    assert_eq!(snapshot(&session), before);
    session.cancel_transaction();
    assert!(!session.can_undo());
}

#[test]
fn multi_bone_preview_commit_is_one_pending_history_step_and_exact_noop_preserves_redo() {
    let mut session = with_redo();
    let base = session
        .authored_pose(animate(0.4))
        .unwrap()
        .bone_locals()
        .to_vec();
    let unchanged = [
        edit(0, base[0], &[Property::X]),
        edit(1, base[1], &[Property::Rotation]),
    ];
    let before = snapshot(&session);
    session.edit_poses(animate(0.4), &unchanged, false).unwrap();
    assert_eq!(snapshot(&session), before);
    session.begin_transaction();
    for rotation in [12., 24., 36.] {
        session
            .preview_edit_poses(
                animate(0.4),
                &[
                    edit(
                        0,
                        Transform {
                            rotation,
                            ..base[0]
                        },
                        &[Property::Rotation],
                    ),
                    edit(
                        1,
                        Transform {
                            x: rotation,
                            ..base[1]
                        },
                        &[Property::X],
                    ),
                ],
                false,
            )
            .unwrap();
    }
    session.commit_transaction();
    assert!(!session.is_dirty());
    assert!(!session.can_redo());
    let pending = session.pose_edits().clone();
    session.undo();
    assert!(session.pose_edits().is_empty());
    assert!(!session.can_undo());
    session.redo();
    assert_eq!(session.pose_edits(), &pending);
}

#[test]
fn multi_bone_explicit_skin_authored_query_is_read_only_and_keeps_all_pending_overlays() {
    let mut doc = fixture();
    doc.skin_scope.required_bones.insert(1);
    doc.skins.insert("awake".into(), BTreeMap::new());
    doc.skin_scope.skins.insert(
        "awake".into(),
        SkinMembers {
            bones: vec![1],
            constraints: vec![],
        },
    );
    doc.slots.push(Slot {
        name: "marker".into(),
        bone: 1,
        attachment: None,
        color: WHITE,
        blend: BlendMode::Normal,
    });
    doc.animations[0].color_tracks.push(ColorTrack {
        slot: 0,
        keys: vec![ColorKey {
            time: 0.,
            color: [10, 20, 30, 40],
            curves: [Curve::Linear; 4],
        }],
    });
    doc.validate().unwrap();
    let mut session = Session::new(doc.clone()).unwrap();
    let locals = session
        .authored_pose(animate(0.4))
        .unwrap()
        .bone_locals()
        .to_vec();
    session
        .edit_poses(
            animate(0.4),
            &[
                edit(
                    0,
                    Transform {
                        rotation: 48.,
                        ..locals[0]
                    },
                    &[Property::Rotation],
                ),
                edit(
                    1,
                    Transform {
                        x: 19.,
                        ..locals[1]
                    },
                    &[Property::X],
                ),
            ],
            false,
        )
        .unwrap();
    let before = snapshot(&session);
    let asleep = session
        .authored_pose_with_skin(animate(0.4), "default")
        .unwrap();
    let awake = session
        .authored_pose_with_skin(animate(0.4), "awake")
        .unwrap();
    assert!(!asleep.activity().bones[1] && awake.activity().bones[1]);
    assert_eq!(asleep.slot_colors()[0], [1.; 4]);
    assert_eq!(
        awake.slot_colors()[0],
        [10_u8, 20, 30, 40].map(|x| f32::from(x) / 255.)
    );
    for pose in [&asleep, &awake] {
        assert_eq!(pose.bone_locals()[0].rotation, 48.);
        assert_eq!(pose.bone_locals()[1].x, 19.);
    }
    assert_eq!(session.authored_pose(animate(0.4)).unwrap(), asleep);
    assert_eq!(snapshot(&session), before);

    session.set_skin("awake");
    session
        .edit_slot_pose(
            0,
            animate(0.4),
            &[SlotPoseEdit::Color {
                color: [0.15, 0.25, 0.35, 0.45],
            }],
            false,
        )
        .unwrap();
    let before = snapshot(&session);
    let pending_inactive = session
        .authored_pose_with_skin(animate(0.4), "default")
        .unwrap();
    assert!(!pending_inactive.activity().bones[1]);
    assert_eq!(pending_inactive.slot_colors()[0], [0.15, 0.25, 0.35, 0.45]);
    assert_eq!(snapshot(&session), before);
    assert_eq!(session.document(), &doc);
    assert_eq!(
        session
            .authored_pose_with_skin(EditTarget::Setup, "awake")
            .unwrap()
            .bone_locals(),
        doc.bones.iter().map(|b| b.setup).collect::<Vec<_>>()
    );
    assert!(
        session
            .authored_pose_with_skin(animate(-1.), "awake")
            .is_err()
    );
    assert_eq!(snapshot(&session), before);
}

#[test]
fn multi_bone_explicit_skin_edit_keys_without_mutating_view_context_or_other_pending() {
    let mut doc = fixture();
    doc.skins.insert("awake".into(), BTreeMap::new());
    doc.skin_scope.required_bones.insert(1);
    doc.skin_scope.skins.insert(
        "awake".into(),
        SkinMembers {
            bones: vec![1],
            constraints: vec![],
        },
    );
    let mut session = Session::new(doc.clone()).unwrap();
    let target = animate(0.4);
    let mut locals = session
        .authored_pose_with_skin(target, "awake")
        .unwrap()
        .bone_locals()
        .to_vec();
    locals[2].rotation = 37.;
    session
        .edit_pose(2, target, locals[2], &[Property::Rotation], false)
        .unwrap();
    let pending = session.pose_edits().clone();
    locals[0].rotation = 18.;
    locals[1].y = 14.;
    let edits = [
        edit(0, locals[0], &[Property::Rotation]),
        edit(1, locals[1], &[Property::Y]),
    ];
    assert_eq!(session.skin(), "default");
    session
        .edit_poses_with_skin(target, &edits, true, "awake")
        .unwrap();
    assert_eq!(session.skin(), "default");
    let awake = session.authored_pose_with_skin(target, "awake").unwrap();
    let sleeping = session.authored_pose(target).unwrap();
    assert!(awake.activity().bones[1]);
    assert!(!sleeping.activity().bones[1]);
    assert_eq!(awake.bone_locals()[0].rotation, 18.);
    assert_eq!(awake.bone_locals()[1].y, 14.);
    assert_eq!(awake.bone_locals()[2].rotation, 37.);
    assert_eq!(session.pose_edits(), &pending);
    let keyed = session.document().clone();
    let before_failure = snapshot(&session);
    let mut invalid = edits.to_vec();
    invalid[1].transform.shear_y = f32::NAN;
    assert!(
        session
            .edit_poses_with_skin(target, &invalid, true, "awake")
            .is_err()
    );
    assert_eq!(snapshot(&session), before_failure);
    session.undo();
    assert_eq!(session.document(), &doc);
    assert_eq!(session.pose_edits(), &pending);
    assert_eq!(session.skin(), "default");
    session.redo();
    assert_eq!(session.document(), &keyed);
    assert_eq!(session.skin(), "default");
}
