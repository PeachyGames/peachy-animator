use crate::weighted_tests::{assert_points, reference, rig};
use crate::*;
use peachy_document::*;

pub(super) fn mesh(doc: &Document) -> &Mesh {
    let Attachment::Mesh(mesh) = doc.attachment(&reference("weighted")).unwrap() else {
        unreachable!()
    };
    mesh
}

pub(super) fn mesh_mut(doc: &mut Document) -> &mut Mesh {
    let Attachment::Mesh(mesh) = doc.attachment_mut(&reference("weighted")).unwrap() else {
        unreachable!()
    };
    mesh
}

pub(super) fn slot_local_rig() -> Document {
    let mut doc = rig();
    mesh_mut(&mut doc).vertices = MeshVertices::SlotLocal {
        positions: vec![[0., 0.], [2., 0.], [0., 2.]],
    };
    doc
}

pub(super) fn setup_pose(doc: &Document) -> MeshBindPose {
    MeshBindPose {
        locals: doc.bones.iter().map(|bone| bone.setup).collect(),
    }
}

fn bind(bone: usize, pose: MeshBindPose) -> Command {
    Command::BindMeshToBone {
        attachment: reference("weighted"),
        bone,
        pose,
    }
}

fn unbind(pose: MeshBindPose) -> Command {
    Command::UnbindMesh {
        attachment: reference("weighted"),
        pose,
    }
}

fn vertices(doc: &Document, locals: Vec<Transform>) -> Vec<[f32; 2]> {
    scene_from_locals(doc, "default", locals).attachments[0]
        .vertices
        .clone()
}

fn assert_metadata(before: &Document, after: &Document) {
    let mut expected = before.clone();
    mesh_mut(&mut expected).vertices = mesh(after).vertices.clone();
    assert_eq!(after, &expected);
}

pub(super) fn pending_with_redo(doc: Document) -> Session {
    let mut session = Session::new(doc).unwrap();
    let mut pose = session.local_pose(Some(0), 0.137)[1];
    pose.x += 13.;
    session
        .edit_pose(
            1,
            EditTarget::Animate {
                animation: 0,
                time: 0.137,
            },
            pose,
            &[Property::X],
            false,
        )
        .unwrap();
    session
        .dispatch(Command::SetBoneLength {
            bone: 0,
            length: 11.,
        })
        .unwrap();
    session.undo();
    assert!(session.can_redo());
    session
}

#[test]
fn initial_bind_preserves_geometry_with_a_different_slot_bone_and_reflected_nonuniform_parents() {
    let original = slot_local_rig();
    let pose = setup_pose(&original);
    let expected = [[160., 220.], [160., 224.], [166., 220.]];
    assert_points(&vertices(&original, pose.locals.clone()), &expected);
    let bound = apply_command(&original, bind(1, pose.clone())).unwrap();
    assert_metadata(&original, &bound);
    assert_points(&vertices(&bound, pose.locals), &expected);
    let MeshVertices::Weighted { influences } = &mesh(&bound).vertices else {
        panic!("Expected weighted result")
    };
    assert_eq!(influences.len(), 3);
    for vertex in influences {
        assert_eq!(vertex.len(), 1);
        assert_eq!(vertex[0].bone, 1);
        assert_eq!(vertex[0].weight, 1.);
    }
    assert_points(
        &influences.iter().map(|v| v[0].position).collect::<Vec<_>>(),
        &[[102., -47.5], [102., -48.5], [106., -47.5]],
    );
    let moved = apply_command(
        &bound,
        Command::SetSetup {
            bone: 1,
            transform: Transform {
                x: bound.bones[1].setup.x + 1.,
                ..bound.bones[1].setup
            },
        },
    )
    .unwrap();
    assert_points(
        &vertices(&moved, setup_pose(&moved).locals),
        &[[160., 222.], [160., 226.], [166., 222.]],
    );
    assert_eq!(mesh(&moved), mesh(&bound));
    assert_eq!(original, slot_local_rig());
}

#[test]
fn small_nonzero_bind_scales_are_invertible_and_unrelated_zero_scales_do_not_reject() {
    for scale in [1e-4, 1e-12, 1e-20] {
        let mut original = slot_local_rig();
        original.bones[1].parent = None;
        original.bones[1].setup = Transform {
            x: -1.,
            y: 2.,
            rotation: 37.,
            scale_x: scale,
            scale_y: -2. * scale,
        };
        original.bones[3].setup.scale_x = 0.;
        original.bones[3].setup.scale_y = 0.;
        original.validate().unwrap();
        let pose = setup_pose(&original);
        let before = vertices(&original, pose.locals.clone());
        let bound = apply_command(&original, bind(1, pose.clone())).unwrap();
        assert_points(&vertices(&bound, pose.locals.clone()), &before);
        let unbound = apply_command(&bound, unbind(pose.clone())).unwrap();
        assert_points(&vertices(&unbound, pose.locals), &before);
        assert_metadata(&original, &unbound);
    }
}

#[test]
fn session_bind_and_unbind_capture_exact_pending_animate_pose_without_keying_or_discarding_it() {
    for time in [0.137, 2.37] {
        let original = slot_local_rig();
        let mut session = Session::new(original.clone()).unwrap();
        let target = EditTarget::Animate { animation: 0, time };
        let mut pose = session.local_pose(Some(0), time)[1];
        pose.x += 13.;
        session
            .edit_pose(1, target, pose, &[Property::X], false)
            .unwrap();
        let pending = session.pose_edits().clone();
        let locals = session.local_pose(Some(0), time);
        let before = vertices(session.document(), locals.clone());
        session
            .bind_mesh_to_bone(reference("weighted"), 1, target)
            .unwrap();
        let bound = session.document().clone();
        assert_metadata(&original, &bound);
        assert_points(&vertices(&bound, locals.clone()), &before);
        assert_eq!(session.pose_edits(), &pending);
        assert_eq!(session.document().animations, original.animations);
        assert!(session.is_dirty());
        session.undo();
        assert_eq!(session.document(), &original);
        assert_eq!(session.pose_edits(), &pending);
        assert!(!session.is_dirty());
        session.redo();
        assert_eq!(session.document(), &bound);
        session.mark_saved();
        session.unbind_mesh(reference("weighted"), target).unwrap();
        assert_points(&vertices(session.document(), locals), &before);
        assert_eq!(session.pose_edits(), &pending);
        assert!(session.is_dirty());
        assert!(!mesh(session.document()).is_weighted());
        session.undo();
        assert_eq!(session.document(), &bound);
        assert_eq!(session.pose_edits(), &pending);
        assert!(!session.is_dirty());
    }
}

#[test]
fn explicit_setup_binding_uses_setup_locals_while_retaining_pending_animation_pose() {
    let original = slot_local_rig();
    let mut session = pending_with_redo(original.clone());
    let pending = session.pose_edits().clone();
    let expected = vertices(&original, setup_pose(&original).locals);
    session
        .bind_mesh_to_bone(reference("weighted"), 1, EditTarget::Setup)
        .unwrap();
    assert_points(
        &vertices(session.document(), setup_pose(&original).locals),
        &expected,
    );
    assert_eq!(session.pose_edits(), &pending);
    assert!(!session.can_redo());
    assert_metadata(&original, session.document());
    session.undo();
    assert_eq!(session.document(), &original);
    assert_eq!(session.pose_edits(), &pending);
}

#[test]
fn removing_all_bindings_preserves_raw_nonunit_repeated_influence_geometry_and_restores_it_on_undo()
{
    let mut original = rig();
    let MeshVertices::Weighted { influences } = &mut mesh_mut(&mut original).vertices else {
        unreachable!()
    };
    for vertex in influences {
        vertex[0].weight = 0.4;
        let mut repeated = vertex[0];
        repeated.position[0] += 1.5;
        repeated.weight = 0.3333;
        vertex.push(repeated);
        vertex.push(BoneInfluence {
            bone: 0,
            position: [3., 2.],
            weight: 0.5,
        });
        vertex.push(BoneInfluence {
            bone: 2,
            position: [-12., 99.],
            weight: 0.,
        });
    }
    original.validate().unwrap();
    let pose = setup_pose(&original);
    let before = vertices(&original, pose.locals.clone());
    let mut session = Session::new(original.clone()).unwrap();
    session
        .unbind_mesh(reference("weighted"), EditTarget::Setup)
        .unwrap();
    let unbound = session.document().clone();
    assert_metadata(&original, &unbound);
    assert!(!mesh(&unbound).is_weighted());
    assert_points(&vertices(&unbound, pose.locals), &before);
    session.undo();
    assert_eq!(session.document(), &original);
    assert!(!session.is_dirty());
    session.redo();
    assert_eq!(session.document(), &unbound);
}

#[test]
fn singular_rejection_applies_only_to_the_inverse_the_operation_actually_needs() {
    let mut cannot_bind = slot_local_rig();
    cannot_bind.bones[1].setup.scale_x = 0.;
    assert!(apply_command(&cannot_bind, bind(1, setup_pose(&cannot_bind))).is_err());

    // A collapsed source slot can still be bound through an invertible target bone.
    let mut collapsed_source = slot_local_rig();
    collapsed_source.bones[2].setup.scale_x = 0.;
    let pose = setup_pose(&collapsed_source);
    let before = vertices(&collapsed_source, pose.locals.clone());
    let bound = apply_command(&collapsed_source, bind(1, pose.clone())).unwrap();
    assert_points(&vertices(&bound, pose.locals), &before);

    let mut cannot_unbind = rig();
    cannot_unbind.bones[2].setup.scale_y = 0.;
    assert!(apply_command(&cannot_unbind, unbind(setup_pose(&cannot_unbind))).is_err());

    // Unbind evaluates driver transforms forward, so a collapsed driver is allowed.
    let mut collapsed_driver = rig();
    collapsed_driver.bones[1].setup.scale_x = 0.;
    collapsed_driver.bones[1].setup.scale_y = 0.;
    let pose = setup_pose(&collapsed_driver);
    let before = vertices(&collapsed_driver, pose.locals.clone());
    let unbound = apply_command(&collapsed_driver, unbind(pose.clone())).unwrap();
    assert_points(&vertices(&unbound, pose.locals), &before);
}

#[test]
fn finite_inverse_coordinates_that_overflow_when_rendered_are_rejected_atomically() {
    let mut original = slot_local_rig();
    original.bones[0].setup = Transform {
        scale_x: 1e38,
        ..Default::default()
    };
    original.bones[1].setup = Transform {
        rotation: 45.,
        ..Default::default()
    };
    original.bones[2].parent = None;
    original.bones[2].setup = Transform::default();
    mesh_mut(&mut original).vertices = MeshVertices::SlotLocal {
        positions: vec![[0., 0.], [20., 0.], [0., 20.]],
    };
    original.validate().unwrap();
    let mut session = Session::new(original.clone()).unwrap();
    session
        .dispatch(Command::SetBoneLength {
            bone: 0,
            length: 11.,
        })
        .unwrap();
    session.undo();
    assert!(
        session
            .bind_mesh_to_bone(reference("weighted"), 1, EditTarget::Setup)
            .is_err()
    );
    assert_eq!(session.document(), &original);
    assert!(!session.is_dirty());
    assert!(!session.can_undo());
    assert!(session.can_redo());

    let mut weighted = rig();
    weighted.bones[0].setup = Transform {
        scale_x: 1e38,
        ..Default::default()
    };
    weighted.bones[2].setup = Transform {
        rotation: 45.,
        ..Default::default()
    };
    mesh_mut(&mut weighted).vertices = MeshVertices::Weighted {
        influences: [[0., 0.], [20., 0.], [0., 20.]]
            .map(|position| {
                vec![BoneInfluence {
                    bone: 3,
                    position,
                    weight: 1.,
                }]
            })
            .to_vec(),
    };
    weighted.validate().unwrap();
    let mut session = Session::new(weighted.clone()).unwrap();
    assert!(
        session
            .unbind_mesh(reference("weighted"), EditTarget::Setup)
            .is_err()
    );
    assert_eq!(session.document(), &weighted);
    assert!(!session.is_dirty());
    assert!(!session.can_undo());
}

#[test]
fn finite_inverse_precision_loss_cannot_shorten_a_ten_unit_edge_during_bind_or_unbind() {
    for weighted in [false, true] {
        let mut original = if weighted { rig() } else { slot_local_rig() };
        original.bones[0].setup = Transform::default();
        original.bones[1].parent = None;
        original.bones[1].setup = Transform {
            x: if weighted { 0. } else { 1e8 },
            ..Default::default()
        };
        original.bones[2].parent = None;
        original.bones[2].setup = Transform {
            x: if weighted { 1e8 } else { 0. },
            ..Default::default()
        };
        let points = vec![[0., 0.], [10., 0.], [0., 10.]];
        mesh_mut(&mut original).vertices = if weighted {
            MeshVertices::Weighted {
                influences: points
                    .iter()
                    .map(|&position| {
                        vec![BoneInfluence {
                            bone: 1,
                            position,
                            weight: 1.,
                        }]
                    })
                    .collect(),
            }
        } else {
            MeshVertices::SlotLocal {
                positions: points.clone(),
            }
        };
        let local_pose = setup_pose(&original);
        assert_eq!(vertices(&original, local_pose.locals.clone()), points);
        // The inverse's finite x coordinate rounds to -99_999_992, which
        // reconstructs world x=8 instead of the original x=10.
        assert_eq!((10f64 - f64::from(1e8f32)) as f32 + 1e8f32, 8.);
        let mut session = pending_with_redo(original.clone());
        let pending = session.pose_edits().clone();
        let undo_available = session.can_undo();
        let result = if weighted {
            session.unbind_mesh(reference("weighted"), EditTarget::Setup)
        } else {
            session.bind_mesh_to_bone(reference("weighted"), 1, EditTarget::Setup)
        };
        assert!(result.unwrap_err().contains("coordinate precision"));
        assert_eq!(session.document(), &original);
        assert_eq!(session.pose_edits(), &pending);
        assert!(!session.is_dirty());
        assert_eq!(session.can_undo(), undo_available);
        assert!(session.can_redo());
    }
}

#[test]
fn exactly_recoverable_large_bind_poses_are_accepted_without_a_position_cutoff() {
    for weighted in [false, true] {
        let mut original = if weighted { rig() } else { slot_local_rig() };
        original.bones[0].setup = Transform::default();
        original.bones[1].parent = None;
        original.bones[1].setup = Transform {
            x: if weighted { 0. } else { 1e8 },
            ..Default::default()
        };
        original.bones[2].parent = None;
        original.bones[2].setup = Transform {
            x: if weighted { 1e8 } else { 0. },
            ..Default::default()
        };
        let points = vec![[0., 0.], [16., 0.], [0., 16.]];
        mesh_mut(&mut original).vertices = if weighted {
            MeshVertices::Weighted {
                influences: points
                    .iter()
                    .map(|&position| {
                        vec![BoneInfluence {
                            bone: 1,
                            position,
                            weight: 1.,
                        }]
                    })
                    .collect(),
            }
        } else {
            MeshVertices::SlotLocal {
                positions: points.clone(),
            }
        };
        let pose = setup_pose(&original);
        let result = apply_command(
            &original,
            if weighted {
                unbind(pose.clone())
            } else {
                bind(1, pose.clone())
            },
        )
        .unwrap();
        assert_eq!(vertices(&result, pose.locals), points);
        assert_metadata(&original, &result);
    }
}

#[test]
fn invalid_bind_commands_and_batches_preserve_document_pose_saved_state_and_redo() {
    let original = slot_local_rig();
    let mut session = pending_with_redo(original.clone());
    let pending = session.pose_edits().clone();
    let original_undo = session.can_undo();
    let pose = setup_pose(&original);
    let mut bad_length = pose.clone();
    bad_length.locals.pop();
    let mut nonfinite = pose.clone();
    nonfinite.locals[1].x = f32::NAN;
    let mut singular = pose.clone();
    singular.locals[1].scale_x = 0.;
    let mut overflow = pose.clone();
    overflow.locals[1].scale_x = f32::from_bits(1);
    overflow.locals[1].scale_y = f32::from_bits(1);
    let mut nonfinite_world = pose.clone();
    nonfinite_world.locals[2].scale_x = f32::MAX;
    let missing = AttachmentRef {
        placeholder: "missing".into(),
        ..reference("weighted")
    };
    for command in [
        bind(usize::MAX, pose.clone()),
        bind(1, bad_length),
        bind(1, nonfinite),
        bind(1, singular),
        bind(1, overflow),
        bind(1, nonfinite_world),
        Command::BindMeshToBone {
            attachment: missing,
            bone: 1,
            pose: pose.clone(),
        },
        Command::Batch {
            commands: vec![
                Command::SetBoneLength {
                    bone: 0,
                    length: 19.,
                },
                bind(usize::MAX, pose.clone()),
            ],
        },
        Command::Batch {
            commands: vec![bind(1, pose.clone()), bind(1, pose.clone())],
        },
    ] {
        assert!(session.dispatch(command).is_err());
        assert_eq!(session.document(), &original);
        assert_eq!(session.pose_edits(), &pending);
        assert!(!session.is_dirty());
        assert_eq!(session.can_undo(), original_undo);
        assert!(session.can_redo());
    }
    session.redo();
    assert_eq!(session.document().bones[0].length, 11.);
    assert_eq!(session.pose_edits(), &pending);
}

#[test]
fn already_weighted_binding_is_rejected_and_slot_local_unbind_validates_before_noop() {
    let weighted = rig();
    assert!(apply_command(&weighted, bind(1, setup_pose(&weighted))).is_err());
    let mut original = slot_local_rig();
    original.bones[2].setup.scale_x = 0.;
    let mut session = pending_with_redo(original.clone());
    let pending = session.pose_edits().clone();
    let undo_available = session.can_undo();
    session
        .unbind_mesh(reference("weighted"), EditTarget::Setup)
        .unwrap();
    assert_eq!(session.document(), &original);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
    assert_eq!(session.can_undo(), undo_available);
    let mut invalid = setup_pose(&original);
    invalid.locals[0].scale_y = f32::INFINITY;
    for command in [
        unbind(MeshBindPose { locals: vec![] }),
        unbind(invalid),
        Command::UnbindMesh {
            attachment: reference("unknown slot"),
            pose: setup_pose(&original),
        },
        Command::UnbindMesh {
            attachment: AttachmentRef {
                skin: "missing".into(),
                ..reference("weighted")
            },
            pose: setup_pose(&original),
        },
    ] {
        assert!(session.dispatch(command).is_err());
        assert_eq!(session.document(), &original);
        assert_eq!(session.pose_edits(), &pending);
        assert!(session.can_redo());
    }
    for target in [
        EditTarget::Animate {
            animation: usize::MAX,
            time: 0.,
        },
        EditTarget::Animate {
            animation: 0,
            time: f32::NAN,
        },
        EditTarget::Animate {
            animation: 0,
            time: -1.,
        },
        EditTarget::Animate {
            animation: 0,
            time: f32::INFINITY,
        },
    ] {
        assert!(session.unbind_mesh(reference("weighted"), target).is_err());
        assert!(
            session
                .bind_mesh_to_bone(reference("weighted"), 1, target)
                .is_err()
        );
        assert_eq!(session.document(), &original);
        assert_eq!(session.pose_edits(), &pending);
        assert!(session.can_redo());
    }
}

#[test]
fn fixed_base_bind_previews_and_cancel_restore_original_payload_pose_and_history() {
    let original = slot_local_rig();
    let mut session = pending_with_redo(original.clone());
    let pending = session.pose_edits().clone();
    let pose = MeshBindPose {
        locals: session.local_pose(Some(0), 0.137),
    };
    session.begin_transaction();
    session.preview_transaction(bind(1, pose.clone())).unwrap();
    assert!(mesh(session.document()).is_weighted());
    // A second Bind succeeds against the original slot-local transaction base.
    session.preview_transaction(bind(0, pose.clone())).unwrap();
    let valid_preview = session.document().clone();
    let MeshVertices::Weighted { influences } = &mesh(&valid_preview).vertices else {
        unreachable!()
    };
    assert!(influences.iter().all(|v| v[0].bone == 0));
    assert!(
        session
            .preview_transaction(bind(usize::MAX, pose.clone()))
            .is_err()
    );
    assert_eq!(session.document(), &valid_preview);
    assert_eq!(session.pose_edits(), &pending);
    session.cancel_transaction();
    assert_eq!(session.document(), &original);
    assert_eq!(session.pose_edits(), &pending);
    assert!(!session.is_dirty());
    assert!(session.can_redo());
    session.begin_transaction();
    session.preview_transaction(bind(1, pose.clone())).unwrap();
    session.commit_transaction();
    let bound = session.document().clone();
    assert!(!session.can_redo());
    session.undo();
    assert_eq!(session.document(), &original);
    assert_eq!(session.pose_edits(), &pending);
    session.redo();
    session.mark_saved();
    session.begin_transaction();
    session.preview_transaction(unbind(pose)).unwrap();
    assert!(!mesh(session.document()).is_weighted());
    session.cancel_transaction();
    assert_eq!(session.document(), &bound);
    assert_eq!(session.pose_edits(), &pending);
    assert!(!session.is_dirty());
}

#[test]
fn exact_attachment_identity_and_strict_serialization_preserve_other_skins_and_region_data() {
    let mut original = slot_local_rig();
    original
        .skins
        .insert("other".into(), original.skins["default"].clone());
    original.skins.insert("active".into(), Default::default());
    original
        .skins
        .get_mut("default")
        .unwrap()
        .get_mut("weighted")
        .unwrap()
        .insert(
            "region".into(),
            Attachment::Region(Region {
                name: None,
                path: "original-region".into(),
                width: 10.,
                height: 20.,
                transform: Transform::default(),
            }),
        );
    let region = AttachmentRef {
        placeholder: "region".into(),
        ..reference("weighted")
    };
    let pose = setup_pose(&original);
    for command in [
        Command::BindMeshToBone {
            attachment: region.clone(),
            bone: 1,
            pose: pose.clone(),
        },
        Command::UnbindMesh {
            attachment: region,
            pose: pose.clone(),
        },
        Command::BindMeshToBone {
            attachment: AttachmentRef {
                skin: "active".into(),
                ..reference("weighted")
            },
            bone: 1,
            pose: pose.clone(),
        },
    ] {
        assert!(apply_command(&original, command).is_err());
    }
    let (resolved, _) = original.resolve_attachment("active", 0, "shape").unwrap();
    assert_eq!(resolved, reference("weighted"));
    let bound = apply_command(&original, bind(1, pose.clone())).unwrap();
    assert_metadata(&original, &bound);
    for (source, command, tag) in [
        (&original, bind(1, pose.clone()), "bindMeshToBone"),
        (&bound, unbind(pose), "unbindMesh"),
    ] {
        let encoded = serde_json::to_value(&command).unwrap();
        assert_eq!(encoded["command"], tag);
        let decoded: Command = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(
            apply_command(source, command).unwrap(),
            apply_command(source, decoded).unwrap()
        );
        for field in ["command", "pose", "local"] {
            let mut invalid = encoded.clone();
            match field {
                "command" => invalid["extra"] = true.into(),
                "pose" => invalid["pose"]["extra"] = true.into(),
                _ => invalid["pose"]["locals"][0]["extra"] = true.into(),
            }
            assert!(serde_json::from_value::<Command>(invalid).is_err());
        }
    }
    let native = serde_json::to_value(&bound).unwrap();
    assert_eq!(native["format"], 4);
    assert!(
        native["skins"]["default"]["weighted"]["shape"]
            .get("pose")
            .is_none()
    );
}
