//! Captured inheritance is part of a posed FK snapshot, not a second solve.
use crate::binding_tests::{mesh, mesh_mut, pending_with_redo, setup_pose, slot_local_rig};
use crate::weighted_tests::{reference, rig};
use crate::*;
use peachy_document::*;

fn target() -> EditTarget {
    EditTarget::Animate {
        animation: 0,
        time: 0.75,
    }
}
fn keyed(doc: &mut Document, mode: Inherit) {
    doc.animations[0].tracks.clear();
    doc.animations[0].inherit_tracks = [1, 2]
        .map(|bone| InheritTrack {
            bone,
            keys: vec![InheritKey {
                time: 0.5,
                inherit: mode,
            }],
        })
        .to_vec();
}
fn geometry(session: &Session) -> Vec<[f32; 2]> {
    checked_scene_from_authored(
        session.document(),
        "default",
        &session.authored_pose(target()).unwrap(),
    )
    .unwrap()
    .attachments[0]
        .vertices
        .clone()
}
fn close(actual: &[[f32; 2]], expected: &[[f32; 2]]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        for (&actual, &expected) in actual.iter().zip(expected) {
            assert!(
                (f64::from(actual) - f64::from(expected)).abs() <= 0.001,
                "{actual} vs {expected}"
            );
        }
    }
}

#[test]
fn bind_pose_inherit_keyed_modes_preserve_actual_bind_add_remove_geometry_and_pending_pose() {
    for mode in Inherit::ALL {
        let mut doc = slot_local_rig();
        keyed(&mut doc, mode);
        let mut session = Session::new(doc.clone()).unwrap();
        let mut pending_transform = doc.bones[1].setup;
        pending_transform.x += 7.125;
        session
            .edit_pose(1, target(), pending_transform, &[Property::X], false)
            .unwrap();
        let pending = session.pose_edits().clone();
        let original_geometry = geometry(&session);
        let authored = session.authored_pose(target()).unwrap();
        let evaluated = session.evaluated_pose(target()).unwrap();
        let captured = session.mesh_bind_pose(target()).unwrap();
        assert_eq!(captured.inherits.as_deref(), Some(authored.inherits()));
        assert_eq!(captured.locals, evaluated.applied_locals());
        let reconstructed = world_pose_with(
            &doc,
            &captured.locals,
            captured.inherits.as_deref().unwrap(),
        );
        for (actual, expected) in reconstructed.iter().zip(evaluated.world()) {
            assert_eq!(
                [actual.a, actual.b, actual.c, actual.d, actual.x, actual.y],
                [
                    expected.a, expected.b, expected.c, expected.d, expected.x, expected.y
                ]
            );
        }
        if mode != Inherit::Normal {
            let legacy = world_pose(&doc, &captured.locals);
            assert!(
                legacy.iter().zip(&reconstructed).any(|(a, b)| [
                    a.a - b.a,
                    a.b - b.b,
                    a.c - b.c,
                    a.d - b.d
                ]
                .iter()
                .any(|difference| difference.abs() > 0.01)),
                "The original fixture must distinguish keyed mode from Setup FK"
            );
        }
        session
            .bind_mesh_to_bone(reference("weighted"), 1, target())
            .unwrap();
        close(&geometry(&session), &original_geometry);
        assert_eq!(session.pose_edits(), &pending);
        let bound = session.document().clone();
        session
            .add_mesh_bone_binding(reference("weighted"), 2, target())
            .unwrap();
        close(&geometry(&session), &original_geometry);
        let MeshVertices::Weighted { influences: old } = &mesh(&bound).vertices else {
            panic!()
        };
        let MeshVertices::Weighted { influences: new } = &mesh(session.document()).vertices else {
            panic!()
        };
        for (old, new) in old.iter().zip(new) {
            assert_eq!(&new[..old.len()], old);
            assert_eq!(new.last().unwrap().bone, 2);
            assert_eq!(new.last().unwrap().weight.to_bits(), 0_f32.to_bits());
        }
        assert_eq!(session.pose_edits(), &pending);
        session
            .unbind_mesh(reference("weighted"), target())
            .unwrap();
        close(&geometry(&session), &original_geometry);
        assert_eq!(session.pose_edits(), &pending);
        assert_eq!(session.document().animations, doc.animations);
        session.undo();
        assert!(mesh(session.document()).is_weighted());
        assert_eq!(session.pose_edits(), &pending);
        session.redo();
        close(&geometry(&session), &original_geometry);
    }
}

fn large_vertex(doc: &mut Document) {
    let MeshVertices::Weighted { influences } = &mut mesh_mut(doc).vertices else {
        panic!()
    };
    influences[0] = vec![
        BoneInfluence {
            bone: 1,
            position: [0., 1.5e38],
            weight: 0.2,
        },
        BoneInfluence {
            bone: 3,
            position: [0., 0.],
            weight: 0.8,
        },
    ];
}
fn direct(pose: MeshBindPose) -> Command {
    Command::SetMeshBoneWeight {
        attachment: reference("weighted"),
        vertex: 0,
        bone: 3,
        weight: 0.6,
        pose,
    }
}

#[test]
fn bind_pose_inherit_direct_guard_uses_actual_keyed_world_in_both_finite_directions() {
    let mut doc = rig();
    keyed(&mut doc, Inherit::OnlyTranslation);
    large_vertex(&mut doc);
    let mut session = Session::new(doc.clone()).unwrap();
    let captured = session.mesh_bind_pose(target()).unwrap();
    let legacy = MeshBindPose {
        locals: captured.locals.clone(),
        inherits: None,
    };
    assert!(
        apply_command(&doc, direct(legacy))
            .unwrap_err()
            .contains("nonfinite world vertices")
    );
    session
        .set_mesh_bone_weight(reference("weighted"), 0, 3, 0.6, target())
        .unwrap();
    assert!(
        geometry(&session)
            .iter()
            .flatten()
            .all(|value| value.is_finite())
    );
    let MeshVertices::Weighted { influences } = &mesh(session.document()).vertices else {
        panic!()
    };
    assert_eq!(influences[0][1].weight.to_bits(), 0.6_f32.to_bits());
    assert_eq!(influences[0][0].position, [0., 1.5e38]);

    // Reversing Setup/keyed modes makes only the legacy guard appear finite.
    let mut opposite = rig();
    opposite.bones[1].inherit = Inherit::OnlyTranslation;
    opposite.animations[0].tracks.clear();
    opposite.animations[0].inherit_tracks = vec![InheritTrack {
        bone: 1,
        keys: vec![InheritKey {
            time: 0.5,
            inherit: Inherit::Normal,
        }],
    }];
    large_vertex(&mut opposite);
    let mut rejected = pending_with_redo(opposite.clone());
    let pending = rejected.pose_edits().clone();
    let captured = rejected.mesh_bind_pose(target()).unwrap();
    let legacy = MeshBindPose {
        locals: captured.locals.clone(),
        inherits: None,
    };
    assert!(apply_command(&opposite, direct(legacy)).is_ok());
    assert!(
        rejected
            .set_mesh_bone_weight(reference("weighted"), 0, 3, 0.6, target())
            .unwrap_err()
            .contains("nonfinite world vertices")
    );
    assert_eq!(rejected.document(), &opposite);
    assert_eq!(rejected.pose_edits(), &pending);
    assert!(rejected.can_redo());
}

#[test]
fn bind_pose_inherit_legacy_wire_and_explicit_setup_produce_identical_binding_results() {
    let doc = slot_local_rig();
    let legacy = setup_pose(&doc);
    let wire = serde_json::to_value(&legacy).unwrap();
    assert!(wire.get("inherits").is_none());
    let restored: MeshBindPose = serde_json::from_value(wire).unwrap();
    assert_eq!(restored, legacy);
    let explicit = MeshBindPose {
        locals: legacy.locals.clone(),
        inherits: Some(doc.bones.iter().map(|bone| bone.inherit).collect()),
    };
    let bind = |pose| Command::BindMeshToBone {
        attachment: reference("weighted"),
        bone: 1,
        pose,
    };
    assert_eq!(
        apply_command(&doc, bind(legacy)).unwrap(),
        apply_command(&doc, bind(explicit.clone())).unwrap()
    );
    let encoded = serde_json::to_value(&explicit).unwrap();
    assert_eq!(
        encoded["inherits"].as_array().unwrap().len(),
        doc.bones.len()
    );
    assert_eq!(
        serde_json::from_value::<MeshBindPose>(encoded).unwrap(),
        explicit
    );
}

#[test]
fn bind_pose_inherit_malformed_modes_and_checked_domain_fail_atomically() {
    let original = rig();
    let mut session = pending_with_redo(original.clone());
    let pending = session.pose_edits().clone();
    let pose = setup_pose(&original);
    for modes in [
        vec![],
        vec![Inherit::Normal; original.bones.len() - 1],
        vec![Inherit::Normal; original.bones.len() + 1],
    ] {
        let malformed = MeshBindPose {
            locals: pose.locals.clone(),
            inherits: Some(modes),
        };
        assert!(
            session
                .dispatch(Command::Batch {
                    commands: vec![
                        Command::RenameAnimation {
                            animation: 0,
                            name: "must rollback".into()
                        },
                        Command::AddMeshBoneBinding {
                            attachment: reference("weighted"),
                            bone: 3,
                            pose: malformed
                        },
                    ]
                })
                .unwrap_err()
                .contains("inheritance mode per bone")
        );
        assert_eq!(session.document(), &original);
        assert_eq!(session.pose_edits(), &pending);
        assert!(session.can_redo());
    }
    let base = serde_json::to_value(&pose).unwrap();
    for invalid in [
        serde_json::Value::Null,
        serde_json::json!(["unexpected"]),
        serde_json::json!("normal"),
        serde_json::json!([0]),
    ] {
        let mut wire = base.clone();
        wire["inherits"] = invalid;
        assert!(serde_json::from_value::<MeshBindPose>(wire).is_err());
    }
    let mut unknown = base;
    unknown["inherit"] = serde_json::json!([]);
    assert!(serde_json::from_value::<MeshBindPose>(unknown).is_err());
    let mut singular = pose;
    singular.locals[0].scale_x = 0.;
    singular.locals[0].scale_y = 0.;
    let mut modes = vec![Inherit::Normal; original.bones.len()];
    modes[1] = Inherit::NoScale;
    singular.inherits = Some(modes);
    assert!(
        session
            .dispatch(direct(singular))
            .unwrap_err()
            .contains("nondegenerate parent")
    );
    assert_eq!(session.document(), &original);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
}
