use crate::binding_tests::{mesh, mesh_mut, pending_with_redo, setup_pose, slot_local_rig};
use crate::weighted_tests::{assert_points, reference, rig};
use crate::*;
use peachy_document::*;

fn vertex(doc: &Document, index: usize) -> &[BoneInfluence] {
    let MeshVertices::Weighted { influences } = &mesh(doc).vertices else {
        unreachable!()
    };
    &influences[index]
}

fn vertex_mut(doc: &mut Document, index: usize) -> &mut Vec<BoneInfluence> {
    let MeshVertices::Weighted { influences } = &mut mesh_mut(doc).vertices else {
        unreachable!()
    };
    &mut influences[index]
}

fn bits(vertex: &[BoneInfluence]) -> Vec<(usize, [u32; 2], u32)> {
    vertex
        .iter()
        .map(|entry| {
            (
                entry.bone,
                entry.position.map(f32::to_bits),
                entry.weight.to_bits(),
            )
        })
        .collect()
}

fn add(bone: usize, pose: MeshBindPose) -> Command {
    Command::AddMeshBoneBinding {
        attachment: reference("weighted"),
        bone,
        pose,
    }
}

fn direct(vertex: usize, bone: usize, weight: f32, pose: MeshBindPose) -> Command {
    Command::SetMeshBoneWeight {
        attachment: reference("weighted"),
        vertex,
        bone,
        weight,
        pose,
    }
}

fn positions(doc: &Document, pose: &MeshBindPose) -> Vec<[f32; 2]> {
    scene_from_locals(doc, "default", pose.locals.clone())
        .unwrap()
        .attachments[0]
        .vertices
        .clone()
}

fn three_bones() -> Document {
    let mut doc = rig();
    *vertex_mut(&mut doc, 0) = vec![
        BoneInfluence {
            bone: 0,
            position: [0., 0.],
            weight: 0.2,
        },
        BoneInfluence {
            bone: 1,
            position: [0., 0.],
            weight: 0.3,
        },
        BoneInfluence {
            bone: 3,
            position: [0., 0.],
            weight: 0.5,
        },
    ];
    doc
}

fn assert_only_vertex_changed(before: &Document, after: &Document, index: usize) {
    let mut expected = before.clone();
    *vertex_mut(&mut expected, index) = vertex(after, index).to_vec();
    assert_eq!(after, &expected);
    for index in (0..mesh(before).vertex_count()).filter(|&other| other != index) {
        assert_eq!(bits(vertex(before, index)), bits(vertex(after, index)));
    }
}

#[test]
fn scratch_binding_add_zero_and_direct_thirty_percent_work_without_keys_or_rebinding() {
    let original = slot_local_rig();
    let mut session = Session::new(original.clone()).unwrap();
    session
        .bind_mesh_to_bone(reference("weighted"), 1, EditTarget::Setup)
        .unwrap();
    let bound = session.document().clone();
    let pose = setup_pose(&bound);
    let before = positions(&bound, &pose);
    session
        .add_mesh_bone_binding(reference("weighted"), 3, EditTarget::Setup)
        .unwrap();
    let added = session.document().clone();
    assert_points(&positions(&added, &pose), &before);
    for index in 0..mesh(&added).vertex_count() {
        assert_eq!(vertex(&added, index).len(), 2);
        assert_eq!(
            bits(&vertex(&added, index)[..1]),
            bits(vertex(&bound, index))
        );
        assert_eq!(vertex(&added, index)[1].bone, 3);
        assert_eq!(vertex(&added, index)[1].weight.to_bits(), 0f32.to_bits());
    }
    session
        .set_mesh_bone_weight(reference("weighted"), 1, 3, 0.3, EditTarget::Setup)
        .unwrap();
    let edited = session.document().clone();
    assert_eq!(
        vertex(&edited, 1)
            .iter()
            .map(|entry| entry.weight)
            .collect::<Vec<_>>(),
        [0.7, 0.3]
    );
    assert_only_vertex_changed(&added, &edited, 1);
    assert_eq!(
        vertex(&edited, 1)
            .iter()
            .map(|entry| (entry.bone, entry.position))
            .collect::<Vec<_>>(),
        vertex(&added, 1)
            .iter()
            .map(|entry| (entry.bone, entry.position))
            .collect::<Vec<_>>()
    );
    assert_points(&positions(&edited, &pose), &before);
    let mut moved = pose;
    moved.locals[3].x += 10.;
    let mut expected = before;
    expected[1][0] += 3.;
    assert_points(&positions(&edited, &moved), &expected);
    assert_eq!(edited.animations, original.animations);
    session.undo();
    assert_eq!(session.document(), &added);
    session.undo();
    assert_eq!(session.document(), &bound);
    session.undo();
    assert_eq!(session.document(), &original);
    assert!(!session.is_dirty());
}

#[test]
fn add_zero_preserves_old_nonunit_repeated_signed_zero_payloads_in_setup_and_pending_poses() {
    for time in [0., 0.137, 2.37] {
        let mut original = rig();
        for index in 0..mesh(&original).vertex_count() {
            vertex_mut(&mut original, index)[0].weight = 0.2;
            vertex_mut(&mut original, index).push(BoneInfluence {
                bone: 1,
                position: [-0., 5.],
                weight: 0.3,
            });
            vertex_mut(&mut original, index).push(BoneInfluence {
                bone: 0,
                position: [3., -0.],
                weight: -0.,
            });
        }
        let mut session = pending_with_redo(original.clone());
        let pending = session.pose_edits().clone();
        let pose = MeshBindPose {
            locals: session.local_pose(Some(0), time),
        };
        let before = positions(&original, &pose);
        session
            .add_mesh_bone_binding(
                reference("weighted"),
                3,
                EditTarget::Animate { animation: 0, time },
            )
            .unwrap();
        let added = session.document().clone();
        for index in 0..mesh(&added).vertex_count() {
            assert_eq!(
                bits(&vertex(&added, index)[..3]),
                bits(vertex(&original, index))
            );
            assert_eq!(vertex(&added, index)[3].weight, 0.);
        }
        assert_points(&positions(&added, &pose), &before);
        assert_eq!(session.pose_edits(), &pending);
        assert_eq!(added.bones, original.bones);
        assert_eq!(added.animations, original.animations);
        session.undo();
        assert_eq!(session.document(), &original);
        assert_eq!(session.pose_edits(), &pending);
        session.redo();
        assert_eq!(session.document(), &added);
    }
}

#[test]
fn direct_assigns_known_vectors_and_preserves_recipient_ratios_without_shape_compensation() {
    let original = three_bones();
    let pose = setup_pose(&original);
    let edited = apply_command(&original, direct(0, 3, 0.6, pose.clone())).unwrap();
    let weights: Vec<_> = vertex(&edited, 0)
        .iter()
        .map(|entry| entry.weight)
        .collect();
    for (&actual, expected) in weights.iter().zip([0.16, 0.24, 0.6]) {
        assert!((actual - expected).abs() < 1e-7);
    }
    assert_eq!(weights[2].to_bits(), 0.6f32.to_bits());
    assert!((weights[0] / weights[1] - 2. / 3.).abs() < 1e-7);
    assert!((weights.iter().map(|&weight| f64::from(weight)).sum::<f64>() - 1.).abs() < 1e-7);
    assert_only_vertex_changed(&original, &edited, 0);
    for (old, new) in vertex(&original, 0).iter().zip(vertex(&edited, 0)) {
        assert_eq!(old.bone, new.bone);
        assert_eq!(
            old.position.map(f32::to_bits),
            new.position.map(f32::to_bits)
        );
    }
    assert_points(&positions(&original, &pose)[..1], &[[0.1, 15.]]);
    assert_points(&positions(&edited, &pose)[..1], &[[-1.52, 12.8]]);
    for (weight, expected) in [(0., [0.4, 0.6, 0.]), (1., [0., 0., 1.])] {
        let edited = apply_command(&original, direct(0, 3, weight, pose.clone())).unwrap();
        assert_eq!(vertex(&edited, 0).len(), 3);
        for (entry, expected) in vertex(&edited, 0).iter().zip(expected) {
            assert!((entry.weight - expected).abs() < 1e-7);
        }
    }
}

#[test]
fn direct_normalizes_only_explicit_vertex_and_retains_signed_zeros_and_unselected_duplicates() {
    let mut original = rig();
    *vertex_mut(&mut original, 0) = vec![
        BoneInfluence {
            bone: 1,
            position: [0., -0.],
            weight: 0.2,
        },
        BoneInfluence {
            bone: 0,
            position: [2., 3.],
            weight: 0.3,
        },
        BoneInfluence {
            bone: 2,
            position: [-0., 4.],
            weight: -0.,
        },
    ];
    vertex_mut(&mut original, 2).push(BoneInfluence {
        bone: 1,
        position: [3., 4.],
        weight: 0.2,
    });
    let before = original.clone();
    let edited = apply_command(&original, direct(0, 1, 0.2, setup_pose(&original))).unwrap();
    assert_eq!(vertex(&edited, 0)[0].weight, 0.2);
    assert_eq!(vertex(&edited, 0)[1].weight, 0.8);
    assert_eq!(
        bits(&vertex(&edited, 0)[2..]),
        bits(&vertex(&original, 0)[2..])
    );
    assert_only_vertex_changed(&original, &edited, 0);
    assert_eq!(original, before);
    assert_eq!(vertex(&original, 0)[1].weight, 0.3);
}

#[test]
fn subnormal_positive_recipients_and_explicit_underflow_keep_entries_and_noop_keeps_redo() {
    let tiny = f32::from_bits(1);
    let mut original = rig();
    *vertex_mut(&mut original, 0) = vec![
        BoneInfluence {
            bone: 0,
            position: [0., 0.],
            weight: tiny,
        },
        BoneInfluence {
            bone: 1,
            position: [1., 0.],
            weight: tiny,
        },
    ];
    let edited = apply_command(&original, direct(0, 0, 0.3, setup_pose(&original))).unwrap();
    assert_eq!(vertex(&edited, 0)[0].weight, 0.3);
    assert_eq!(vertex(&edited, 0)[1].weight, 0.7);
    *vertex_mut(&mut original, 0) = vec![
        BoneInfluence {
            bone: 0,
            position: [0., 0.],
            weight: tiny,
        },
        BoneInfluence {
            bone: 1,
            position: [1., 0.],
            weight: 1.,
        },
        BoneInfluence {
            bone: 3,
            position: [0., 1.],
            weight: 0.,
        },
    ];
    let requested = f32::from_bits(1f32.to_bits() - 1);
    let edited = apply_command(&original, direct(0, 3, requested, setup_pose(&original))).unwrap();
    assert_eq!(vertex(&edited, 0).len(), 3);
    assert_eq!(vertex(&edited, 0)[0].weight, 0.);
    assert!(vertex(&edited, 0)[1].weight > 0.);
    assert_eq!(vertex(&edited, 0)[2].weight.to_bits(), requested.to_bits());
    let original = apply_command(&rig(), add(3, setup_pose(&rig()))).unwrap();
    let mut session = pending_with_redo(original.clone());
    let pending = session.pose_edits().clone();
    session
        .set_mesh_bone_weight(reference("weighted"), 0, 1, 1., EditTarget::Setup)
        .unwrap();
    assert_eq!(session.document(), &original);
    assert_eq!(session.pose_edits(), &pending);
    assert!(!session.is_dirty());
    assert!(session.can_redo());
    assert!(
        session
            .set_mesh_bone_weight(reference("weighted"), 0, 1, 0.3, EditTarget::Setup)
            .is_err()
    );
    assert_eq!(session.document(), &original);
    assert!(session.can_redo());
}

#[test]
fn add_uses_only_new_bone_inverse_and_reuses_small_scale_precision_and_zero_finiteness_guards() {
    let mut original = rig();
    original.bones[1].setup.scale_x = 0.;
    original.bones[2].setup.scale_y = 0.;
    original.bones[3].setup.scale_x = 1e-20;
    original.bones[3].setup.scale_y = -1e-20;
    let pose = setup_pose(&original);
    let before = positions(&original, &pose);
    let added = apply_command(&original, add(3, pose.clone())).unwrap();
    assert_points(&positions(&added, &pose), &before);
    for transform in [
        Transform {
            scale_x: 0.,
            ..original.bones[3].setup
        },
        Transform {
            x: 1e8,
            ..Default::default()
        },
    ] {
        let mut pose = setup_pose(&rig());
        pose.locals[3] = transform;
        assert!(apply_command(&rig(), add(3, pose)).is_err());
    }
    let mut original = rig();
    original.bones[0].setup = Transform {
        scale_x: 1e38,
        ..Default::default()
    };
    original.bones[1].parent = None;
    original.bones[1].setup = Transform::default();
    original.bones[2].parent = None;
    original.bones[2].setup = Transform::default();
    original.bones[3].parent = Some(0);
    original.bones[3].setup = Transform {
        rotation: 45.,
        ..Default::default()
    };
    for (index, position) in [[0., 0.], [20., 0.], [0., 20.]].into_iter().enumerate() {
        vertex_mut(&mut original, index)[0].position = position;
    }
    assert!(
        positions(&original, &setup_pose(&original))
            .iter()
            .flatten()
            .all(|value| value.is_finite())
    );
    let error = apply_command(&original, add(3, setup_pose(&original))).unwrap_err();
    assert!(error.contains("nonfinite world vertices"));
}

#[test]
fn direct_requires_no_inverse_but_rejects_nonfinite_candidate_world_geometry() {
    let mut original = three_bones();
    let mut pose = setup_pose(&original);
    pose.locals[0].scale_x = 0.;
    pose.locals[0].scale_y = 0.;
    pose.locals[3].scale_x = 0.;
    pose.locals[3].scale_y = 0.;
    let edited = apply_command(&original, direct(0, 3, 0.6, pose)).unwrap();
    assert_only_vertex_changed(&original, &edited, 0);
    vertex_mut(&mut original, 0)[0].position = [f32::MAX, f32::MAX];
    let error = apply_command(&original, direct(0, 3, 0.6, setup_pose(&original))).unwrap_err();
    assert!(error.contains("nonfinite world vertices"));
}

#[test]
fn invalid_weights_repeats_sparse_rebinding_and_batches_are_atomic_with_pending_pose_and_redo() {
    let original = three_bones();
    let mut session = pending_with_redo(original.clone());
    let pending = session.pose_edits().clone();
    let undo_available = session.can_undo();
    let pose = setup_pose(&original);
    let mut invalid_pose = pose.clone();
    invalid_pose.locals[0].x = f32::NAN;
    let mut new_bone_pose = pose.clone();
    new_bone_pose.locals.push(Transform::default());
    let mut commands = vec![
        add(1, pose.clone()),
        add(3, pose.clone()),
        add(usize::MAX, pose.clone()),
        add(2, MeshBindPose { locals: vec![] }),
        add(2, invalid_pose.clone()),
        direct(usize::MAX, 3, 0.6, pose.clone()),
        direct(0, usize::MAX, 0.6, pose.clone()),
        direct(1, 3, 0.6, pose.clone()),
        direct(0, 3, 0.6, invalid_pose),
        Command::Batch {
            commands: vec![
                direct(0, 3, 0.6, pose.clone()),
                direct(1, 3, 0.6, pose.clone()),
            ],
        },
        Command::Batch {
            commands: vec![
                Command::AddBone {
                    bone: Bone {
                        name: "bad hierarchy".into(),
                        parent: Some(usize::MAX),
                        length: 1.,
                        setup: Transform::default(),
                        inherit: peachy_document::Inherit::Normal,
                    },
                },
                add(2, new_bone_pose),
            ],
        },
    ];
    commands.extend(
        [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.1, 1.1]
            .map(|weight| direct(0, 3, weight, pose.clone())),
    );
    for command in commands {
        assert!(session.dispatch(command).is_err());
        assert_eq!(session.document(), &original);
        assert_eq!(session.pose_edits(), &pending);
        assert_eq!(session.can_undo(), undo_available);
        assert!(session.can_redo());
        assert!(!session.is_dirty());
    }
    let mut repeated = original.clone();
    vertex_mut(&mut repeated, 0).push(BoneInfluence {
        bone: 0,
        position: [9., 7.],
        weight: -0.,
    });
    assert!(
        apply_command(&repeated, direct(0, 3, 0.6, setup_pose(&repeated)))
            .unwrap_err()
            .contains("ambiguous")
    );
    // Presence on only one vertex, even at zero weight, is already a global binding.
    let mut sparse = rig();
    vertex_mut(&mut sparse, 2).push(BoneInfluence {
        bone: 3,
        position: [0., 0.],
        weight: 0.,
    });
    assert!(apply_command(&sparse, add(3, setup_pose(&sparse))).is_err());
    assert!(apply_command(&sparse, direct(0, 3, 0.3, setup_pose(&sparse))).is_err());
    for target in [
        EditTarget::Animate {
            animation: usize::MAX,
            time: 0.,
        },
        EditTarget::Animate {
            animation: 0,
            time: -1.,
        },
        EditTarget::Animate {
            animation: 0,
            time: f32::NAN,
        },
    ] {
        assert!(
            session
                .add_mesh_bone_binding(reference("weighted"), 2, target)
                .is_err()
        );
        assert!(
            session
                .set_mesh_bone_weight(reference("weighted"), 0, 3, 0.6, target)
                .is_err()
        );
        assert_eq!(session.document(), &original);
        assert_eq!(session.pose_edits(), &pending);
        assert!(session.can_redo());
    }
}

#[test]
fn fixed_base_add_previews_restore_raw_payload_pending_pose_saved_state_and_redo() {
    let original = rig();
    let mut session = pending_with_redo(original.clone());
    let pending = session.pose_edits().clone();
    let pose = MeshBindPose {
        locals: session.local_pose(Some(0), 0.137),
    };
    session.begin_transaction();
    session.preview_transaction(add(3, pose.clone())).unwrap();
    assert_eq!(vertex(session.document(), 0)[1].bone, 3);
    session.preview_transaction(add(2, pose.clone())).unwrap();
    assert_eq!(vertex(session.document(), 0).len(), 2);
    assert_eq!(vertex(session.document(), 0)[1].bone, 2);
    let valid = session.document().clone();
    assert!(session.preview_transaction(add(1, pose.clone())).is_err());
    assert_eq!(session.document(), &valid);
    session.cancel_transaction();
    assert_eq!(session.document(), &original);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
    assert!(!session.is_dirty());
    session.begin_transaction();
    session.preview_transaction(add(3, pose)).unwrap();
    session.commit_transaction();
    let added = session.document().clone();
    assert!(!session.can_redo());
    session.undo();
    assert_eq!(session.document(), &original);
    assert_eq!(session.pose_edits(), &pending);
    session.redo();
    assert_eq!(session.document(), &added);
}

#[test]
fn direct_previews_rescale_from_base_and_commit_one_undo_with_pending_animate_pose_intact() {
    let original = three_bones();
    let mut session = pending_with_redo(original.clone());
    let pending = session.pose_edits().clone();
    let pose = MeshBindPose {
        locals: session.local_pose(Some(0), 0.137),
    };
    let expected = apply_command(&original, direct(0, 3, 0.6, pose.clone())).unwrap();
    session.begin_transaction();
    for weight in [0.6, 0.2, 0.9, 0.6] {
        session
            .preview_transaction(direct(0, 3, weight, pose.clone()))
            .unwrap();
    }
    assert_eq!(session.document(), &expected);
    assert_eq!(
        bits(vertex(session.document(), 0)),
        bits(vertex(&expected, 0))
    );
    assert!(
        session
            .preview_transaction(direct(1, 3, 0.3, pose.clone()))
            .is_err()
    );
    assert_eq!(session.document(), &expected);
    session.cancel_transaction();
    assert_eq!(session.document(), &original);
    assert_eq!(session.pose_edits(), &pending);
    assert!(!session.is_dirty());
    assert!(session.can_redo());
    session.begin_transaction();
    session
        .set_mesh_bone_weight(
            reference("weighted"),
            0,
            3,
            0.6,
            EditTarget::Animate {
                animation: 0,
                time: 0.137,
            },
        )
        .unwrap();
    session.commit_transaction();
    assert_eq!(session.document(), &expected);
    assert_eq!(session.pose_edits(), &pending);
    session.mark_saved();
    session.undo();
    assert_eq!(session.document(), &original);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.is_dirty());
    session.redo();
    assert_eq!(session.document(), &expected);
    assert!(!session.is_dirty());
}

#[test]
fn strict_command_and_native_roundtrips_retain_zero_binding_api_identity_and_other_skins() {
    let mut original = rig();
    original
        .skins
        .insert("alternate".into(), original.skins["default"].clone());
    let pose = setup_pose(&original);
    let added = apply_command(&original, add(3, pose.clone())).unwrap();
    assert_eq!(added.skins["alternate"], original.skins["alternate"]);
    for (source, command, tag) in [
        (&original, add(3, pose.clone()), "addMeshBoneBinding"),
        (&added, direct(0, 3, 0.3, pose), "setMeshBoneWeight"),
    ] {
        let encoded = serde_json::to_value(&command).unwrap();
        assert_eq!(encoded["command"], tag);
        let decoded: Command = serde_json::from_value(encoded.clone()).unwrap();
        let edited = apply_command(source, command).unwrap();
        assert_eq!(apply_command(source, decoded).unwrap(), edited);
        let native = serde_json::to_value(&edited).unwrap();
        assert_eq!(native["format"], FORMAT_VERSION);
        assert_eq!(serde_json::from_value::<Document>(native).unwrap(), edited);
        for field in ["command", "pose", "transform"] {
            let mut invalid = encoded.clone();
            match field {
                "command" => invalid["normalize"] = false.into(),
                "pose" => invalid["pose"]["extra"] = true.into(),
                _ => invalid["pose"]["locals"][0]["extra"] = true.into(),
            }
            assert!(serde_json::from_value::<Command>(invalid).is_err());
        }
    }
    let local = slot_local_rig();
    assert!(apply_command(&local, add(3, setup_pose(&local))).is_err());
    assert!(apply_command(&local, direct(0, 1, 1., setup_pose(&local))).is_err());
    let missing = AttachmentRef {
        skin: "missing".into(),
        ..reference("weighted")
    };
    assert!(
        apply_command(
            &original,
            Command::AddMeshBoneBinding {
                attachment: missing.clone(),
                bone: 3,
                pose: setup_pose(&original)
            }
        )
        .is_err()
    );
    assert!(
        apply_command(
            &original,
            Command::SetMeshBoneWeight {
                attachment: missing,
                vertex: 0,
                bone: 1,
                weight: 1.,
                pose: setup_pose(&original)
            }
        )
        .is_err()
    );
}
