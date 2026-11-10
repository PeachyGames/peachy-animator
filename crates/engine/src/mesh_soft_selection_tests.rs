use crate::*;
use peachy_document::*;
use std::collections::BTreeMap;

fn identity() -> Affine {
    Affine {
        a: 1.,
        b: 0.,
        c: 0.,
        d: 1.,
        x: 0.,
        y: 0.,
    }
}

fn mesh(positions: &[[f32; 2]]) -> Mesh {
    let mesh = Mesh {
        name: Some("Original soft-selection mesh".into()),
        path: "original-soft-selection".into(),
        vertices: MeshVertices::SlotLocal {
            positions: positions.to_vec(),
        },
        uvs: vec![[0.5, 0.5]; positions.len()],
        triangles: (2..positions.len() as u32)
            .flat_map(|vertex| [0, 1, vertex])
            .collect(),
        hull: 3,
        width: 10.,
        height: 10.,
        edges: vec![0, 2, 2, 4, 4, 0],
        color: peachy_document::WHITE,
        link: None,
    };
    mesh.validate().unwrap();
    mesh
}

fn reference() -> AttachmentRef {
    AttachmentRef {
        skin: "default".into(),
        slot: "surface".into(),
        placeholder: "mesh".into(),
    }
}

fn rig(mesh: Mesh) -> Document {
    let doc = Document {
        format: FORMAT_VERSION,
        name: "Original soft-selection rig".into(),
        fps: 30,
        bones: vec![Bone {
            name: "root".into(),
            parent: None,
            length: 5.,
            setup: Transform::default(),
            inherit: peachy_document::Inherit::Normal,
        }],
        constraints: vec![],
        slots: vec![Slot {
            color: [255; 4],
            name: "surface".into(),
            bone: 0,
            attachment: Some("mesh".into()),
        }],
        skins: BTreeMap::from([(
            "default".into(),
            BTreeMap::from([(
                "surface".into(),
                BTreeMap::from([("mesh".into(), Attachment::Mesh(mesh))]),
            )]),
        )]),
        animations: vec![Animation {
            name: "move".into(),
            tracks: vec![],
            physics: vec![],
            physics_resets: vec![],
            ik_tracks: vec![],
            inherit_tracks: vec![],
            deform_tracks: vec![],
            attachment_tracks: vec![],
            color_tracks: vec![],
            events: vec![],
            draw_order: vec![],
            path_tracks: vec![],
        }],
        skin_scope: Default::default(),
        events: vec![],
        reference_scale: 100.0,
    };
    doc.validate().unwrap();
    doc
}

fn stored_mesh(doc: &Document) -> &Mesh {
    let Attachment::Mesh(mesh) = doc.attachment(&reference()).unwrap() else {
        panic!("Expected mesh")
    };
    mesh
}

fn positions(doc: &Document) -> &[[f32; 2]] {
    stored_mesh(doc).slot_local_positions().unwrap()
}

fn settings(size: f32, feather: f32) -> MeshSoftSelection {
    MeshSoftSelection {
        size,
        feather,
        hull_vertices: true,
    }
}

fn commands(command: &Command) -> &[Command] {
    let Command::Batch { commands } = command else {
        panic!("Translation must remain an atomic Batch")
    };
    commands
}

fn pending_with_redo(doc: Document) -> Session {
    let mut session = Session::new(doc).unwrap();
    let mut pose = session.local_pose(Some(0), 0.137)[0];
    pose.rotation += 13.;
    session
        .edit_pose(
            0,
            EditTarget::Animate {
                animation: 0,
                time: 0.137,
            },
            pose,
            &[Property::Rotation],
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
fn size_feather_hard_edge_and_coincident_zero_size_have_explicit_weights() {
    assert_eq!(MeshSoftSelection::default(), settings(0., 0.5));
    let shape = mesh(&[[0., 0.], [5., 0.], [7.5, 0.], [10., 0.], [10.000001, 0.]]);
    let feathered = MeshTranslation::new(&shape, identity(), 0, settings(10., 0.5)).unwrap();
    assert_eq!(feathered.weights(), [1., 1., 0.5, 0., 0.]);
    let hard = MeshTranslation::new(&shape, identity(), 0, settings(10., 0.)).unwrap();
    assert_eq!(hard.weights(), [1., 1., 1., 1., 0.]);
    let full = MeshTranslation::new(&shape, identity(), 0, settings(10., 1.)).unwrap();
    assert_eq!(full.weights(), [1., 0.5, 0.25, 0., 0.]);
    let coincident = mesh(&[[0., -0.], [0., 0.], [1., 0.]]);
    let zero =
        MeshTranslation::new(&coincident, identity(), 0, MeshSoftSelection::default()).unwrap();
    assert_eq!(zero.weights(), [1., 0., 0.]);
    let positive = MeshTranslation::new(&coincident, identity(), 0, settings(1., 1.)).unwrap();
    assert_eq!(positive.weights(), [1., 1., 0.]);
}

#[test]
fn hull_exclusion_keeps_the_selected_hull_and_all_interior_eligibility() {
    let shape = mesh(&[[0., 0.], [10., 0.], [0., 10.], [2., 2.], [3., 2.]]);
    let controls = MeshSoftSelection {
        hull_vertices: false,
        ..settings(100., 0.)
    };
    let interior = MeshTranslation::new(&shape, identity(), 3, controls).unwrap();
    assert_eq!(interior.weights(), [0., 0., 0., 1., 1.]);
    let hull = MeshTranslation::new(&shape, identity(), 1, controls).unwrap();
    assert_eq!(hull.weights(), [0., 1., 0., 1., 1.]);
    let zero = MeshTranslation::new(
        &shape,
        identity(),
        1,
        MeshSoftSelection {
            size: 0.,
            ..controls
        },
    )
    .unwrap();
    assert_eq!(zero.weights(), [0., 1., 0., 0., 0.]);
}

#[test]
fn world_distance_uses_scale_shear_reflection_and_is_independent_of_large_origins() {
    let shape = mesh(&[[0., 0.], [1., 0.], [0., 1.], [1., 1.]]);
    let frame = Affine {
        a: -2.,
        b: 1.,
        c: 0.,
        d: 3.,
        x: 100.,
        y: -50.,
    };
    let translation = MeshTranslation::new(&shape, frame, 0, settings(4., 1.)).unwrap();
    let diagonal = (1. - 10f64.sqrt() / 4.) as f32;
    assert_eq!(translation.weights(), [1., 0.5, diagonal, diagonal]);
    let line = mesh(&[[0., 0.], [1., 0.], [2., 0.]]);
    let far = Affine {
        x: f32::MAX,
        y: -f32::MAX,
        ..identity()
    };
    assert_eq!(
        MeshTranslation::new(&line, far, 0, settings(4., 1.))
            .unwrap()
            .weights(),
        MeshTranslation::new(&line, identity(), 0, settings(4., 1.))
            .unwrap()
            .weights(),
    );
    let wide = mesh(&[[-f32::MAX / 2., 0.], [f32::MAX / 2., 0.], [0., 0.]]);
    assert_eq!(
        MeshTranslation::new(&wide, identity(), 0, settings(f32::MAX, 1.))
            .unwrap()
            .weights(),
        [1., 0., 0.5]
    );
}

#[test]
fn world_delta_uses_frozen_linear_inverse_and_preserves_weighted_direction() {
    let shape = mesh(&[[0., 0.], [1., 0.], [0., 1.], [1., 1.]]);
    let frame = Affine {
        a: -2.,
        b: 1.,
        c: 0.,
        d: 3.,
        x: 100.,
        y: -50.,
    };
    let translation = MeshTranslation::new(&shape, frame, 0, settings(4., 1.)).unwrap();
    let original = rig(shape.clone());
    let changed = apply_command(
        &original,
        translation.world_command(reference(), [2., 3.]).unwrap(),
    )
    .unwrap();
    assert_eq!(positions(&changed)[0], [-0.5, 1.]);
    assert_eq!(positions(&changed)[1], [0.75, 0.5]);
    for (index, (&before, &after)) in shape
        .slot_local_positions()
        .unwrap()
        .iter()
        .zip(positions(&changed))
        .enumerate()
    {
        let before = frame.point(before[0], before[1]);
        let after = frame.point(after[0], after[1]);
        for axis in 0..2 {
            let actual = after[axis] - before[axis];
            let expected = [2., 3.][axis] * translation.weights()[index];
            assert!(
                (actual - expected).abs() < 0.0001,
                "{index}/{axis}: {actual} != {expected}"
            );
        }
    }
    let cancelled_inverse = mesh(&[[-f32::MAX, 0.], [-f32::MAX, 1.], [-f32::MAX, 2.]]);
    let half_scale = Affine {
        a: 0.5,
        ..identity()
    };
    let wide = MeshTranslation::new(
        &cancelled_inverse,
        half_scale,
        0,
        MeshSoftSelection::default(),
    )
    .unwrap();
    let changed = apply_command(
        &rig(cancelled_inverse),
        wide.world_command(reference(), [f32::MAX, 0.]).unwrap(),
    )
    .unwrap();
    assert_eq!(positions(&changed)[0], [f32::MAX, 0.]);
}

#[test]
fn frozen_batches_skip_unaffected_vertices_and_preserve_topology_uvs_and_raw_bits() {
    let shape = mesh(&[[0., -0.], [10., 0.], [0., 10.], [2., 2.], [3., 2.]]);
    let original = rig(shape.clone());
    let translation = MeshTranslation::new(
        &shape,
        identity(),
        3,
        MeshSoftSelection {
            hull_vertices: false,
            ..settings(100., 0.)
        },
    )
    .unwrap();
    let baseline = translation.command(reference(), [-0., 0.]).unwrap();
    assert_eq!(commands(&baseline).len(), 2);
    assert_eq!(apply_command(&original, baseline).unwrap(), original);
    let first = translation.command(reference(), [1., 2.]).unwrap();
    let repeated = translation.command(reference(), [1., 2.]).unwrap();
    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        serde_json::to_value(&repeated).unwrap()
    );
    assert_eq!(commands(&first).len(), 2);
    let changed = apply_command(&original, first).unwrap();
    let second = apply_command(
        &changed,
        translation.command(reference(), [4., 6.]).unwrap(),
    )
    .unwrap();
    let mut expected = original.clone();
    let Attachment::Mesh(expected_mesh) = expected.attachment_mut(&reference()).unwrap() else {
        unreachable!()
    };
    expected_mesh.slot_local_positions_mut().unwrap()[3] = [6., 8.];
    expected_mesh.slot_local_positions_mut().unwrap()[4] = [7., 8.];
    assert_eq!(second, expected);
    assert_eq!(
        positions(&second)[0].map(f32::to_bits),
        [0f32.to_bits(), (-0f32).to_bits()]
    );
    let local_axis =
        MeshTranslation::new(&shape, identity(), 0, MeshSoftSelection::default()).unwrap();
    let moved = apply_command(
        &original,
        local_axis.command(reference(), [1., 0.]).unwrap(),
    )
    .unwrap();
    assert_eq!(positions(&moved)[0][1].to_bits(), (-0f32).to_bits());
    assert_eq!(shape, stored_mesh(&original).clone());
}

#[test]
fn settings_geometry_projection_and_overflow_fail_before_a_command_can_escape() {
    let shape = mesh(&[[0., 0.], [1., 0.], [0., 1.]]);
    let original = shape.clone();
    for size in [-1., f32::NAN, f32::INFINITY] {
        assert!(MeshTranslation::new(&shape, identity(), 0, settings(size, 0.5)).is_err());
    }
    for feather in [-0.01, 1.01, f32::NAN, f32::INFINITY] {
        assert!(MeshTranslation::new(&shape, identity(), 0, settings(10., feather)).is_err());
    }
    assert!(MeshTranslation::new(&shape, identity(), 3, MeshSoftSelection::default()).is_err());
    for bad in [f32::NAN, f32::INFINITY] {
        assert!(
            MeshTranslation::new(
                &shape,
                Affine {
                    a: bad,
                    ..identity()
                },
                0,
                MeshSoftSelection::default()
            )
            .is_err()
        );
        assert!(
            MeshTranslation::new(
                &shape,
                Affine {
                    x: bad,
                    ..identity()
                },
                0,
                MeshSoftSelection::default()
            )
            .is_err()
        );
    }
    let mut invalid = shape.clone();
    invalid.uvs.pop();
    assert!(MeshTranslation::new(&invalid, identity(), 0, MeshSoftSelection::default()).is_err());
    let mut invalid = shape.clone();
    invalid.hull = 4;
    assert!(MeshTranslation::new(&invalid, identity(), 0, MeshSoftSelection::default()).is_err());
    let mut invalid = shape.clone();
    invalid.triangles[2] = 99;
    assert!(MeshTranslation::new(&invalid, identity(), 0, MeshSoftSelection::default()).is_err());
    let mut invalid = shape.clone();
    invalid.slot_local_positions_mut().unwrap()[2][0] = f32::NAN;
    assert!(
        MeshTranslation::new(
            &invalid,
            identity(),
            0,
            MeshSoftSelection {
                hull_vertices: false,
                ..settings(0., 0.5)
            }
        )
        .is_err()
    );
    let huge = mesh(&[[f32::MAX, 0.], [0., 0.], [0., 1.]]);
    assert!(
        MeshTranslation::new(
            &huge,
            Affine {
                a: 2.,
                ..identity()
            },
            1,
            MeshSoftSelection::default()
        )
        .is_err()
    );
    let overflow =
        MeshTranslation::new(&huge, identity(), 0, MeshSoftSelection::default()).unwrap();
    assert!(overflow.command(reference(), [f32::MAX, 0.]).is_err());
    let transform_overflow = MeshTranslation::new(
        &mesh(&[[0., 0.], [0., 1.], [0., 0.5]]),
        Affine {
            a: f32::MAX,
            ..identity()
        },
        0,
        MeshSoftSelection::default(),
    )
    .unwrap();
    assert!(transform_overflow.command(reference(), [2., 0.]).is_err());
    let translation =
        MeshTranslation::new(&shape, identity(), 0, MeshSoftSelection::default()).unwrap();
    for bad in [f32::NAN, f32::INFINITY] {
        assert!(translation.command(reference(), [bad, 0.]).is_err());
        assert!(translation.world_command(reference(), [0., bad]).is_err());
    }
    assert_eq!(shape, original);
}

#[test]
fn weighted_meshes_and_singular_world_gestures_are_explicit_atomic_failures() {
    let shape = mesh(&[[0., 0.], [1., 0.], [0., 1.]]);
    let mut weighted = shape.clone();
    weighted.vertices = MeshVertices::Weighted {
        influences: shape
            .slot_local_positions()
            .unwrap()
            .iter()
            .map(|&position| {
                vec![
                    BoneInfluence {
                        bone: 0,
                        position,
                        weight: 1.,
                    },
                    BoneInfluence {
                        bone: 0,
                        position: [-0., 0.],
                        weight: 0.,
                    },
                ]
            })
            .collect(),
    };
    let weighted_original = weighted.clone();
    assert!(MeshTranslation::new(&weighted, identity(), 0, settings(10., 0.5)).is_err());
    assert_eq!(weighted, weighted_original);
    let singular = Affine {
        a: 1.,
        b: 1.,
        c: 1.,
        d: 1.,
        x: 0.,
        y: 0.,
    };
    let singular_translation =
        MeshTranslation::new(&shape, singular, 0, settings(10., 0.5)).unwrap();
    assert!(
        singular_translation
            .world_command(reference(), [0., 0.])
            .is_err()
    );
    assert!(
        singular_translation
            .world_command(reference(), [1., 2.])
            .is_err()
    );
    assert!(!commands(&singular_translation.command(reference(), [1., 2.]).unwrap()).is_empty());
    let translation = MeshTranslation::new(&shape, identity(), 0, settings(10., 0.5)).unwrap();
    let doc = rig(weighted);
    let mut session = Session::new(doc.clone()).unwrap();
    assert!(
        session
            .dispatch(translation.command(reference(), [1., 2.]).unwrap())
            .is_err()
    );
    assert_eq!(session.document(), &doc);
    assert!(!session.can_undo() && !session.is_dirty());
}

#[test]
fn session_previews_remain_frozen_atomic_and_one_undo_step_with_pending_pose() {
    let shape = mesh(&[[0., 0.], [5., 0.], [7.5, 0.], [10., 0.]]);
    let original = rig(shape.clone());
    let translation = MeshTranslation::new(&shape, identity(), 0, settings(10., 0.5)).unwrap();
    let mut session = pending_with_redo(original.clone());
    let pending = session.pose_edits().clone();
    session
        .dispatch(translation.command(reference(), [0., -0.]).unwrap())
        .unwrap();
    assert!(session.can_redo());
    let mut missing = reference();
    missing.skin = "missing".into();
    assert!(
        session
            .dispatch(translation.command(missing, [1., 0.]).unwrap())
            .is_err()
    );
    assert_eq!(session.document(), &original);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
    session.begin_transaction();
    session
        .preview_transaction(translation.command(reference(), [2., 0.]).unwrap())
        .unwrap();
    let last_valid = session.document().clone();
    let mut invalid_batch = match translation.command(reference(), [3., 0.]).unwrap() {
        Command::Batch { commands } => commands,
        _ => unreachable!(),
    };
    invalid_batch.push(Command::SetMeshVertex {
        attachment: reference(),
        vertex: usize::MAX,
        position: [1., 2.],
    });
    assert!(
        session
            .preview_transaction(Command::Batch {
                commands: invalid_batch
            })
            .is_err()
    );
    assert_eq!(session.document(), &last_valid);
    assert_eq!(session.pose_edits(), &pending);
    session.cancel_transaction();
    assert_eq!(session.document(), &original);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
    session.begin_transaction();
    for delta in [2., 4., 6.] {
        session
            .preview_transaction(translation.command(reference(), [delta, 0.]).unwrap())
            .unwrap();
        assert_eq!(session.pose_edits(), &pending);
    }
    assert_eq!(
        positions(session.document()),
        [[6., 0.], [11., 0.], [10.5, 0.], [10., 0.]]
    );
    session.commit_transaction();
    let edited = session.document().clone();
    assert!(!session.can_redo());
    assert_eq!(edited.animations, original.animations);
    session.undo();
    assert_eq!(session.document(), &original);
    assert_eq!(session.pose_edits(), &pending);
    session.redo();
    assert_eq!(session.document(), &edited);
    assert_eq!(session.pose_edits(), &pending);
}

#[test]
fn live_previews_restore_baseline_for_zero_and_rounded_tiny_deltas_without_history() {
    let shape = mesh(&[[10., 10.], [15., 10.], [17.5, 10.], [20., 10.]]);
    let original = rig(shape.clone());
    let translation = MeshTranslation::new(&shape, identity(), 0, settings(10., 0.5)).unwrap();
    for world in [false, true] {
        for restore_delta in [[0., -0.], [f32::MIN_POSITIVE, f32::MIN_POSITIVE]] {
            let command = |delta| {
                if world {
                    translation.world_command(reference(), delta).unwrap()
                } else {
                    translation.command(reference(), delta).unwrap()
                }
            };
            let mut session = pending_with_redo(original.clone());
            let pending = session.pose_edits().clone();
            session.begin_transaction();
            session.dispatch(command([10., 10.])).unwrap();
            assert_ne!(session.document(), &original);
            session.dispatch(command(restore_delta)).unwrap();
            assert_eq!(session.document(), &original);
            session.commit_transaction();
            assert_eq!(session.document(), &original);
            assert_eq!(session.pose_edits(), &pending);
            assert!(session.can_redo());
            // The preexisting pending-pose edit is still the sole Undo step.
            session.undo();
            assert_eq!(session.document(), &original);
            assert!(session.pose_edits().is_empty());
            assert!(!session.can_undo());
        }
    }
}
