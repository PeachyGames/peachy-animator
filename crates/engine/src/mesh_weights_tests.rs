//! Original numeric native cases plus raw preservation and gesture lifecycle.
use crate::*;
use peachy_document::*;
use std::collections::BTreeMap;

fn attachment() -> AttachmentRef {
    AttachmentRef {
        skin: "default".into(),
        slot: "mesh".into(),
        placeholder: "shape".into(),
    }
}
fn selection(vertices: &[usize]) -> MeshWeightSelection {
    MeshWeightSelection {
        attachment: attachment(),
        vertices: vertices.to_vec(),
        bone: 1,
    }
}
fn fixture() -> Document {
    let mut doc = Document::new("Original multi Direct".into(), 60).unwrap();
    for (name, x, y) in [("A", 0., 0.), ("B", 50., 0.), ("C", 0., 40.)] {
        doc.bones.push(Bone {
            name: name.into(),
            parent: Some(0),
            length: 10.,
            inherit: Inherit::Normal,
            setup: Transform {
                x,
                y,
                ..Default::default()
            },
        });
    }
    doc.slots = vec![
        Slot {
            name: "mesh".into(),
            bone: 0,
            attachment: Some("shape".into()),
            color: WHITE,
            blend: BlendMode::Normal,
        },
        Slot {
            name: "hidden".into(),
            bone: 0,
            attachment: None,
            color: WHITE,
            blend: BlendMode::Normal,
        },
    ];
    let values = [
        [1., 0., 0.],
        [0., 1., 0.],
        [0.2, 0.3, 0.5],
        [0.65, 0.25, 0.1],
        [0.4, 0.6, -0.],
        [0.1, 0.4, 0.5],
    ];
    let influences = values
        .iter()
        .enumerate()
        .map(|(vertex, weights)| {
            let point = [vertex as f32 * 10., (vertex % 2) as f32 * 8.];
            if vertex == 0 {
                return vec![BoneInfluence {
                    bone: 1,
                    position: point,
                    weight: 1.,
                }];
            }
            weights
                .iter()
                .enumerate()
                .map(|(index, &weight)| BoneInfluence {
                    bone: index + 1,
                    position: [
                        point[0] - doc.bones[index + 1].setup.x,
                        point[1] - doc.bones[index + 1].setup.y,
                    ],
                    weight,
                })
                .collect()
        })
        .collect();
    let mesh = Mesh {
        name: None,
        path: "original-six".into(),
        vertices: MeshVertices::Weighted { influences },
        uvs: vec![
            [0., 0.],
            [1., 0.],
            [1., 1.],
            [0., 1.],
            [0.2, 0.5],
            [0.8, 0.5],
        ],
        triangles: vec![0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 5],
        hull: 6,
        width: 60.,
        height: 20.,
        edges: vec![],
        color: WHITE,
        link: None,
        sequence: None,
    };
    doc.skins = BTreeMap::from([(
        "default".into(),
        BTreeMap::from([(
            "mesh".into(),
            BTreeMap::from([("shape".into(), Attachment::Mesh(mesh))]),
        )]),
    )]);
    doc.animations.push(Animation::new("clip", vec![]));
    doc.validate().unwrap();
    doc
}
fn mesh(doc: &Document) -> &Mesh {
    let Some(Attachment::Mesh(mesh)) = doc.attachment(&attachment()) else {
        panic!()
    };
    mesh
}
fn mesh_mut(doc: &mut Document) -> &mut Mesh {
    let Some(Attachment::Mesh(mesh)) = doc.attachment_mut(&attachment()) else {
        panic!()
    };
    mesh
}
fn entries(doc: &Document, vertex: usize) -> &[BoneInfluence] {
    let MeshVertices::Weighted { influences } = &mesh(doc).vertices else {
        panic!()
    };
    &influences[vertex]
}
fn entries_mut(doc: &mut Document, vertex: usize) -> &mut Vec<BoneInfluence> {
    let MeshVertices::Weighted { influences } = &mut mesh_mut(doc).vertices else {
        panic!()
    };
    &mut influences[vertex]
}
fn ordered_fixture(sequences: &[&[usize]; 6]) -> Document {
    let mut doc = fixture();
    for (vertex, bones) in sequences.iter().enumerate() {
        *entries_mut(&mut doc, vertex) = bones
            .iter()
            .enumerate()
            .map(|(index, &bone)| BoneInfluence {
                bone,
                position: [vertex as f32 * 10. + bone as f32, -0.],
                weight: if index + 1 == bones.len() { 1. } else { -0. },
            })
            .collect();
    }
    doc.validate().unwrap();
    doc
}
fn bits(entries: &[BoneInfluence]) -> Vec<(usize, [u32; 2], u32)> {
    entries
        .iter()
        .map(|e| (e.bone, e.position.map(f32::to_bits), e.weight.to_bits()))
        .collect()
}
fn pose(doc: &Document) -> MeshBindPose {
    MeshBindPose {
        locals: doc.bones.iter().map(|b| b.setup).collect(),
        inherits: None,
    }
}
fn command(doc: &Document, vertices: &[usize], edit: MeshWeightEdit) -> Command {
    Command::EditMeshBoneWeights {
        attachment: attachment(),
        vertices: vertices.to_vec(),
        bone: 1,
        edit,
        pose: pose(doc),
    }
}
fn absolute(doc: &Document, vertices: &[usize], weight: f32) -> Command {
    command(doc, vertices, MeshWeightEdit::Absolute { weight })
}
fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 1e-6, "{actual} vs {expected}");
}
fn target() -> EditTarget {
    EditTarget::Animate {
        animation: 0,
        time: 0.137,
    }
}

#[test]
fn mesh_weights_query_preserves_raw_nonunit_data_and_reports_uniform_or_mixed_without_average() {
    let mut doc = fixture();
    entries_mut(&mut doc, 4)[0].weight = 0.234567;
    entries_mut(&mut doc, 4)[1].weight = 0.33333334;
    let before = serde_json::to_string(&doc).unwrap();
    let queried = mesh_bone_weights(&doc, &selection(&[3, 2])).unwrap();
    assert_eq!(queried.selection.vertices, [2, 3]);
    assert_eq!(queried.value, MeshWeightValue::Mixed);
    assert_eq!(
        queried.weights,
        [
            VertexWeight {
                vertex: 2,
                weight: 0.2
            },
            VertexWeight {
                vertex: 3,
                weight: 0.65
            }
        ]
    );
    let raw = mesh_bone_weights(&doc, &selection(&[4])).unwrap();
    assert_eq!(raw.value, MeshWeightValue::Uniform(0.234567));
    assert_eq!(
        raw.weights[0].weight.to_bits(),
        entries(&doc, 4)[0].weight.to_bits()
    );
    assert_eq!(serde_json::to_string(&doc).unwrap(), before);
    let changed = apply_command(&doc, absolute(&doc, &[2, 3], 0.5)).unwrap();
    assert_eq!(
        mesh_bone_weights(&changed, &selection(&[2, 3]))
            .unwrap()
            .value,
        MeshWeightValue::Uniform(0.5)
    );
}

#[test]
fn mesh_weights_absolute_native_pair_and_six_vertex_sole_skip_keep_positions_and_other_vertices() {
    let before = fixture();
    let pair = apply_command(&before, absolute(&before, &[2, 3], 0.5)).unwrap();
    for (vertex, expected) in [(2, [0.5, 0.1875, 0.3125]), (3, [0.5, 5. / 14., 1. / 7.])] {
        for (entry, expected) in entries(&pair, vertex).iter().zip(expected) {
            close(entry.weight, expected);
        }
    }
    for vertex in [0, 1, 4, 5] {
        assert_eq!(bits(entries(&pair, vertex)), bits(entries(&before, vertex)));
    }
    for vertex in 0..6 {
        for (a, b) in entries(&before, vertex).iter().zip(entries(&pair, vertex)) {
            assert_eq!(a.bone, b.bone);
            assert_eq!(a.position.map(f32::to_bits), b.position.map(f32::to_bits));
        }
    }
    let all = apply_command(&before, absolute(&before, &[0, 1, 2, 3, 4, 5], 0.5)).unwrap();
    assert_eq!(bits(entries(&all, 0)), bits(entries(&before, 0)));
    assert_eq!(entries(&all, 1)[0].weight, 0.5);
    assert_eq!(entries(&all, 1)[1].weight, 0.5);
    assert_eq!(entries(&all, 4)[2].weight.to_bits(), (-0_f32).to_bits());
    let legacy = Command::SetMeshBoneWeight {
        attachment: attachment(),
        vertex: 0,
        bone: 1,
        weight: 0.5,
        pose: pose(&before),
    };
    assert!(
        apply_command(&before, legacy)
            .unwrap_err()
            .contains("positive recipient")
    );
    assert_eq!(before.animations, pair.animations);
}

#[test]
fn mesh_weights_relative_clamp_reversal_keeps_frozen_ratios_and_new_gesture_loses_them() {
    let before = fixture();
    let edited = apply_command(
        &before,
        command(
            &before,
            &[2, 3],
            MeshWeightEdit::Relative {
                deltas: vec![2., -1. / 7.],
            },
        ),
    )
    .unwrap();
    for (vertex, expected) in [
        (2, [6. / 7., 3. / 56., 5. / 56.]),
        (3, [6. / 7., 5. / 49., 2. / 49.]),
    ] {
        for (entry, expected) in entries(&edited, vertex).iter().zip(expected) {
            close(entry.weight, expected);
        }
    }
    let mut session = Session::new(before.clone()).unwrap();
    let base = MeshWeightGesture::new(&before, selection(&[2, 3]), pose(&before)).unwrap();
    session.begin_transaction();
    let clamped = base.relative(2.).unwrap();
    session.preview_mesh_bone_weights(&clamped).unwrap();
    assert_eq!(clamped.query().value, MeshWeightValue::Uniform(1.));
    assert!(clamped.matches_document(session.document()));
    let reversed = clamped.relative(-1. / 7.).unwrap();
    session.preview_mesh_bone_weights(&reversed).unwrap();
    assert_eq!(session.document(), &edited);
    assert!(reversed.matches_document(session.document()));
    session.commit_transaction();
    session.undo();
    assert_eq!(session.document(), &before);
    assert!(!session.can_undo());
    session.redo();
    assert_eq!(session.document(), &edited);
    let hundred = apply_command(&before, absolute(&before, &[2, 3], 1.)).unwrap();
    assert_eq!(
        apply_command(&hundred, absolute(&hundred, &[2, 3], 0.5)).unwrap(),
        hundred
    );
    assert_eq!(
        apply_command(
            &hundred,
            command(
                &hundred,
                &[2, 3],
                MeshWeightEdit::Relative { deltas: vec![-0.1] }
            )
        )
        .unwrap(),
        hundred
    );
    // A 2046-vertex selection exercises the prepared publication path. Binding
    // FK is captured once, not rebuilt once per vertex on every pointer frame.
    let mut large = fixture();
    let repeated = entries(&large, 2).to_vec();
    let MeshVertices::Weighted { influences } = &mut mesh_mut(&mut large).vertices else {
        panic!()
    };
    influences.resize(2048, repeated);
    mesh_mut(&mut large).uvs.resize(2048, [0.5, 0.5]);
    for vertex in 6..2048 {
        mesh_mut(&mut large).triangles.extend([0, 1, vertex]);
    }
    // Include sparse creation so the same large preview also exercises one
    // cached ordering plan, shared across candidate clones and pointer frames.
    entries_mut(&mut large, 2).remove(0);
    large.validate().unwrap();
    let selected = (2..2048).collect::<Vec<_>>();
    let count = crate::mesh_binding::bind_world_calls();
    let mut large_session = Session::new(large.clone()).unwrap();
    let original = MeshWeightGesture::new(&large, selection(&selected), pose(&large)).unwrap();
    large_session.begin_transaction();
    let next = original.relative(0.2).unwrap();
    large_session.preview_mesh_bone_weights(&next).unwrap();
    let next = next.relative(-0.1).unwrap();
    large_session.preview_mesh_bone_weights(&next).unwrap();
    assert_eq!(crate::mesh_binding::bind_world_calls() - count, 1);
    let count = crate::mesh_binding::bind_world_calls();
    let expected = apply_command(
        &large,
        command(
            &large,
            &selected,
            MeshWeightEdit::Relative {
                deltas: vec![0.2, -0.1],
            },
        ),
    )
    .unwrap();
    assert_eq!(crate::mesh_binding::bind_world_calls() - count, 1);
    assert_eq!(large_session.document(), &expected);
    assert!(
        mesh_world_vertices(
            mesh(&expected),
            0,
            &world_pose(&expected, &pose(&expected).locals)
        )
        .iter()
        .flatten()
        .all(|value| value.is_finite())
    );
    large_session.commit_transaction();
    large_session.undo();
    assert_eq!(large_session.document(), &large);
    large_session.redo();
    assert_eq!(large_session.document(), &expected);
}

#[test]
fn mesh_weights_sparse_origin_is_created_only_by_positive_edit_and_explicit_zero_keeps_position() {
    let explicit = fixture();
    let mut sparse = explicit.clone();
    entries_mut(&mut sparse, 1).remove(0);
    let raw = bits(entries(&sparse, 1));
    assert_eq!(
        mesh_bone_weights(&sparse, &selection(&[1])).unwrap().value,
        MeshWeightValue::Uniform(0.)
    );
    for edit in [
        MeshWeightEdit::Absolute { weight: 0. },
        MeshWeightEdit::Relative {
            deltas: vec![0., -0.5],
        },
    ] {
        let same = apply_command(&sparse, command(&sparse, &[1], edit)).unwrap();
        assert_eq!(bits(entries(&same, 1)), raw);
    }
    let positive = apply_command(&sparse, absolute(&sparse, &[1], 0.5)).unwrap();
    let new = &entries(&positive, 1)[0];
    assert_eq!(new.bone, 1);
    assert_eq!(new.position, [0., 0.]);
    assert_eq!(new.weight, 0.5);
    assert_eq!(entries(&positive, 1)[1].bone, 2);
    assert_eq!(
        entries(&positive, 1)[1].position,
        entries(&sparse, 1)[0].position
    );
    assert_eq!(entries(&positive, 1)[1].weight, 0.5);
    let retained = apply_command(&explicit, absolute(&explicit, &[1], 0.5)).unwrap();
    assert_eq!(entries(&retained, 1)[0].position, [10., 8.]);
    let world = world_pose(&sparse, &pose(&sparse).locals);
    let moved = mesh_world_vertices(mesh(&positive), 0, &world);
    assert_eq!(moved[1], [5., 4.]);
    let returned = apply_command(
        &sparse,
        command(
            &sparse,
            &[1],
            MeshWeightEdit::Relative {
                deltas: vec![0.3, -0.3],
            },
        ),
    )
    .unwrap();
    assert_eq!(bits(entries(&returned, 1)), raw);
    let mut session = Session::new(sparse.clone()).unwrap();
    let base = MeshWeightGesture::new(&sparse, selection(&[1]), pose(&sparse)).unwrap();
    session.begin_transaction();
    let positive = base.relative(0.3).unwrap();
    session.preview_mesh_bone_weights(&positive).unwrap();
    let returned = positive.relative(-0.3).unwrap();
    session.preview_mesh_bone_weights(&returned).unwrap();
    assert_eq!(session.document(), &sparse);
    session.commit_transaction();
    assert!(!session.can_undo());
    let mut unbound = selection(&[1]);
    unbound.bone = 0;
    assert!(
        mesh_bone_weights(&sparse, &unbound)
            .unwrap_err()
            .contains("Bind")
    );
    let mut pending_deform = Session::new(sparse.clone()).unwrap();
    let mut offsets = vec![0.; mesh(&sparse).deform_len()];
    offsets[0] = 0.1;
    pending_deform
        .edit_deform_pose(attachment(), target(), offsets, false)
        .unwrap();
    let pending = pending_deform.pose_edits().clone();
    let gesture = MeshWeightGesture::new(&sparse, selection(&[1]), pose(&sparse))
        .unwrap()
        .absolute(0.5)
        .unwrap();
    pending_deform.begin_transaction();
    assert!(
        pending_deform
            .preview_mesh_bone_weights(&gesture)
            .unwrap_err()
            .contains("binding layout")
    );
    assert_eq!(pending_deform.document(), &sparse);
    assert_eq!(pending_deform.pose_edits(), &pending);
    pending_deform.cancel_transaction();
    let length = mesh(&sparse).deform_len();
    sparse.animations[0].deform_tracks.push(DeformTrack {
        attachment: attachment(),
        keys: vec![DeformKey {
            time: 0.,
            offsets: vec![0.; length],
            curve: Curve::Linear,
        }],
    });
    assert!(
        apply_command(&sparse, absolute(&sparse, &[1], 0.5))
            .unwrap_err()
            .contains("binding layout")
    );
    let mut keyed_deform = Session::new(sparse.clone()).unwrap();
    keyed_deform.begin_transaction();
    assert!(
        keyed_deform
            .preview_mesh_bone_weights(&gesture)
            .unwrap_err()
            .contains("binding layout")
    );
    assert_eq!(keyed_deform.document(), &sparse);
    assert!(
        apply_command(
            &sparse,
            command(&sparse, &[1], MeshWeightEdit::Relative { deltas: vec![0.] })
        )
        .is_ok()
    );
}

#[test]
fn mesh_weights_sparse_insertion_preserves_mesh_precedence_with_first_appearance_ties() {
    // Installed 4.3.26 accepts consistent A,B,C and C,A,B lists; inserting A
    // after B in an otherwise A,B,C mesh is rejected. No imported list is sorted.
    let cases: [([&[usize]; 6], &[usize]); 4] = [
        (
            [&[1], &[2], &[1, 2, 3], &[1, 2, 3], &[1, 3], &[1, 2, 3]],
            &[1, 2],
        ),
        (
            [&[3], &[3, 2], &[3, 1, 2], &[3, 1, 2], &[3, 2], &[3, 1, 2]],
            &[3, 1, 2],
        ),
        // No single source lists C,A,B: precedence is transitive across vertices.
        ([&[3, 1], &[3, 2], &[1, 2], &[3], &[1], &[2]], &[3, 1, 2]),
        // Disconnected components use C,B,A first appearance, not bone indices.
        ([&[3], &[2], &[1], &[3], &[2], &[1]], &[2, 1]),
    ];
    for (sequences, expected) in cases {
        let doc = ordered_fixture(&sequences);
        let edited = apply_command(&doc, absolute(&doc, &[1], 0.5)).unwrap();
        let actual = entries(&edited, 1);
        assert_eq!(
            actual.iter().map(|entry| entry.bone).collect::<Vec<_>>(),
            expected
        );
        let old = entries(&doc, 1);
        let retained: Vec<_> = actual.iter().filter(|entry| entry.bone != 1).collect();
        assert_eq!(retained.len(), old.len());
        for (actual, old) in retained.iter().zip(old) {
            assert_eq!(actual.bone, old.bone);
            assert_eq!(
                actual.position.map(f32::to_bits),
                old.position.map(f32::to_bits)
            );
            if old.weight == 0. {
                assert_eq!(actual.weight.to_bits(), old.weight.to_bits());
            }
        }
        assert_eq!(
            actual
                .iter()
                .find(|entry| entry.bone == 1)
                .unwrap()
                .position,
            [0., 0.]
        );
        for vertex in [0, 2, 3, 4, 5] {
            assert_eq!(bits(entries(&edited, vertex)), bits(entries(&doc, vertex)));
        }
    }
}

#[test]
fn mesh_weights_sparse_bad_order_only_rejects_positive_creation_atomically() {
    for (sequences, reason) in [
        (
            [
                &[1][..],
                &[2][..],
                &[1, 2][..],
                &[2, 1][..],
                &[1, 2][..],
                &[3][..],
            ],
            "contradictory",
        ),
        (
            [
                &[1][..],
                &[2][..],
                &[1, 3, 1][..],
                &[3][..],
                &[1, 2][..],
                &[3][..],
            ],
            "repeated",
        ),
    ] {
        let before = ordered_fixture(&sequences);
        assert_eq!(
            mesh_bone_weights(&before, &selection(&[1])).unwrap().value,
            MeshWeightValue::Uniform(0.)
        );
        let mut session = pending(before.clone());
        let pending = session.pose_edits().clone();
        let undo = session.can_undo();
        for edit in [
            MeshWeightEdit::Absolute { weight: 0. },
            MeshWeightEdit::Relative {
                deltas: vec![0., -0.25],
            },
        ] {
            session.dispatch(command(&before, &[1], edit)).unwrap();
            assert_eq!(session.document(), &before);
            assert_eq!(session.pose_edits(), &pending);
            assert_eq!(session.can_undo(), undo);
            assert!(session.can_redo());
        }
        let gesture = MeshWeightGesture::new(&before, selection(&[1]), pose(&before)).unwrap();
        assert!(gesture.absolute(0.5).unwrap_err().contains(reason));
        assert!(gesture.matches_document(&before));
        let error = session
            .dispatch(Command::Batch {
                commands: vec![
                    Command::RenameAnimation {
                        animation: 0,
                        name: "rollback".into(),
                    },
                    absolute(&before, &[1], 0.5),
                ],
            })
            .unwrap_err();
        assert!(error.contains(reason));
        assert_eq!(session.document(), &before);
        assert_eq!(session.pose_edits(), &pending);
        assert_eq!(session.can_undo(), undo);
        assert!(session.can_redo());
        // Unselected malformed ordering does not prevent an existing binding edit.
        let existing = apply_command(&before, absolute(&before, &[4], 0.25)).unwrap();
        assert_eq!(entries(&existing, 4)[0].weight, 0.25);
    }
}

#[test]
fn mesh_weights_sparse_gesture_freezes_unselected_bone_sequences_without_rewriting_them() {
    let mut before = fixture();
    entries_mut(&mut before, 1).remove(0);
    let base = MeshWeightGesture::new(&before, selection(&[1]), pose(&before)).unwrap();
    let next = base.absolute(0.5).unwrap();
    let mut session = Session::new(before.clone()).unwrap();
    session.begin_transaction();
    session.preview_mesh_bone_weights(&next).unwrap();
    assert!(next.matches_document(session.document()));
    session.cancel_transaction();
    assert_eq!(session.document(), &before);
    let mut changed = before.clone();
    entries_mut(&mut changed, 2).swap(1, 2);
    assert!(!base.matches_document(&changed));
    let mut stale = pending(changed.clone());
    let pending = stale.pose_edits().clone();
    stale.begin_transaction();
    assert!(
        stale
            .preview_mesh_bone_weights(&next)
            .unwrap_err()
            .contains("source changed")
    );
    assert_eq!(stale.document(), &changed);
    assert_eq!(stale.pose_edits(), &pending);
    stale.cancel_transaction();
    assert!(stale.can_redo());
    // Unselected numeric payload is not used to infer ordering.
    entries_mut(&mut before, 2)[0].position[0] += 1.;
    assert!(base.matches_document(&before));
}

fn pending(doc: Document) -> Session {
    let mut session = Session::new(doc).unwrap();
    let mut transform = session.authored_pose(target()).unwrap().bone_locals()[0];
    transform.x = 13.;
    session
        .edit_pose(0, target(), transform, &[Property::X], false)
        .unwrap();
    session
        .edit_slot_pose(
            0,
            target(),
            &[SlotPoseEdit::Color {
                color: [0.7, 0.8, 0.9, 1.],
            }],
            false,
        )
        .unwrap();
    let mut offsets = vec![0.; mesh(session.document()).deform_len()];
    offsets[0] = 0.1;
    session
        .edit_deform_pose(attachment(), target(), offsets, false)
        .unwrap();
    session
        .edit_draw_order(target(), vec![1, 0], false)
        .unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    session
}

#[test]
fn mesh_weights_invalid_requests_are_atomic_and_zero_relative_preserves_nonunit_bits_and_redo() {
    let mut before = fixture();
    entries_mut(&mut before, 2)[0].weight = 0.234567;
    entries_mut(&mut before, 2)[1].weight = 0.33333334;
    let mut session = pending(before.clone());
    let pending = session.pose_edits().clone();
    let undo = session.can_undo();
    let zero = command(
        &before,
        &[2, 3],
        MeshWeightEdit::Relative {
            deltas: vec![0., -0.],
        },
    );
    session.dispatch(zero).unwrap();
    assert_eq!(
        bits(entries(session.document(), 2)),
        bits(entries(&before, 2))
    );
    assert!(session.can_redo());
    assert_eq!(session.pose_edits(), &pending);
    let mut invalid = vec![
        absolute(&before, &[], 0.5),
        absolute(&before, &[2, 2], 0.5),
        absolute(&before, &[99], 0.5),
        absolute(&before, &[2], f32::NAN),
        absolute(&before, &[2], -0.1),
        absolute(&before, &[2], 1.1),
        command(&before, &[2], MeshWeightEdit::Relative { deltas: vec![] }),
        command(
            &before,
            &[2],
            MeshWeightEdit::Relative {
                deltas: vec![0.1, f32::INFINITY],
            },
        ),
    ];
    let mut bad_pose = pose(&before);
    bad_pose.inherits = Some(vec![]);
    invalid.push(Command::EditMeshBoneWeights {
        attachment: attachment(),
        vertices: vec![2],
        bone: 1,
        edit: MeshWeightEdit::Absolute { weight: 0.5 },
        pose: bad_pose,
    });
    for request in invalid {
        assert!(
            session
                .dispatch(Command::Batch {
                    commands: vec![
                        Command::RenameAnimation {
                            animation: 0,
                            name: "rollback".into()
                        },
                        request
                    ]
                })
                .is_err()
        );
        assert_eq!(session.document(), &before);
        assert_eq!(session.pose_edits(), &pending);
        assert_eq!(session.can_undo(), undo);
        assert!(session.can_redo());
        assert!(!session.is_dirty());
    }
    session
        .set_mesh_bone_weights(selection(&[2, 3]), 0.5, target())
        .unwrap();
    let edited = session.document().clone();
    assert_eq!(session.pose_edits(), &pending);
    assert_eq!(edited.animations, before.animations);
    session.undo();
    assert_eq!(session.document(), &before);
    assert_eq!(session.pose_edits(), &pending);
    session.redo();
    assert_eq!(session.document(), &edited);
    assert_eq!(session.pose_edits(), &pending);
}

#[test]
fn mesh_weights_gesture_failure_and_stale_source_do_not_advance_candidate_or_publish() {
    let before = fixture();
    let mut session = pending(before.clone());
    let pending = session.pose_edits().clone();
    let base = MeshWeightGesture::new(
        session.document(),
        selection(&[2, 3]),
        session.mesh_bind_pose(target()).unwrap(),
    )
    .unwrap();
    session.begin_transaction();
    let valid = base.absolute(0.5).unwrap();
    session.preview_mesh_bone_weights(&valid).unwrap();
    let accepted = session.document().clone();
    assert!(valid.relative(f32::NAN).is_err());
    assert!(valid.matches_document(&accepted));
    assert!(!base.matches_document(&accepted));
    session.cancel_transaction();
    assert_eq!(session.document(), &before);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
    let mut changed = before.clone();
    entries_mut(&mut changed, 2)[0].position[0] += 1.;
    let mut stale = Session::new(changed.clone()).unwrap();
    stale.begin_transaction();
    assert!(
        stale
            .preview_mesh_bone_weights(&valid)
            .unwrap_err()
            .contains("source changed")
    );
    assert_eq!(stale.document(), &changed);
    let mut hierarchy = before.clone();
    hierarchy.bones[3].parent = Some(1);
    let mut stale_hierarchy = Session::new(hierarchy.clone()).unwrap();
    stale_hierarchy.begin_transaction();
    assert!(!valid.matches_document(&hierarchy));
    assert!(
        stale_hierarchy
            .preview_mesh_bone_weights(&valid)
            .unwrap_err()
            .contains("source changed")
    );
    assert_eq!(stale_hierarchy.document(), &hierarchy);
    let legacy = MeshWeightGesture::new(&before, selection(&[2, 3]), pose(&before)).unwrap();
    let mut modes = before.clone();
    modes.bones[1].inherit = Inherit::OnlyTranslation;
    let mut stale_modes = Session::new(modes.clone()).unwrap();
    stale_modes.begin_transaction();
    assert!(!legacy.matches_document(&modes));
    assert!(
        stale_modes
            .preview_mesh_bone_weights(&legacy)
            .unwrap_err()
            .contains("source changed")
    );
    assert_eq!(stale_modes.document(), &modes);
    // Explicit captured modes are authoritative; document Setup modes are not
    // an input to this captured FK and do not invalidate an otherwise same base.
    assert!(base.matches_document(&modes));
    let mut repeated = before.clone();
    entries_mut(&mut repeated, 3).push(BoneInfluence {
        bone: 2,
        position: [0., 0.],
        weight: 0.,
    });
    assert!(
        mesh_bone_weights(&repeated, &selection(&[2, 3]))
            .unwrap_err()
            .contains("ambiguous")
    );
    assert!(mesh_bone_weights(&repeated, &selection(&[2])).is_ok());
    let mut huge = before;
    huge.bones[1].setup.scale_x = 2.;
    entries_mut(&mut huge, 2)[0].position = [f32::MAX, f32::MAX];
    let gesture = MeshWeightGesture::new(&huge, selection(&[2]), pose(&huge)).unwrap();
    assert!(
        gesture
            .absolute(0.5)
            .unwrap_err()
            .contains("nonfinite world")
    );
}

#[test]
fn mesh_weights_semantic_command_and_modes_reject_unknown_fields_and_roundtrip() {
    let doc = fixture();
    for edit in [
        MeshWeightEdit::Absolute { weight: 0.5 },
        MeshWeightEdit::Relative {
            deltas: vec![2., -1. / 7.],
        },
    ] {
        let wire = serde_json::to_value(command(&doc, &[2, 3], edit)).unwrap();
        let decoded: Command = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), wire);
        let mut bad = wire.clone();
        bad["edit"]["extra"] = true.into();
        assert!(serde_json::from_value::<Command>(bad).is_err());
        let mut bad = wire;
        bad["vertex"] = 2.into();
        assert!(serde_json::from_value::<Command>(bad).is_err());
    }
}
