use crate::*;
use peachy_document::*;
use std::collections::BTreeMap;

fn influence(bone: usize, position: [f32; 2], weight: f32) -> BoneInfluence {
    BoneInfluence {
        bone,
        position,
        weight,
    }
}

fn mesh(influences: Vec<Vec<BoneInfluence>>) -> Mesh {
    Mesh {
        name: None,
        path: "original-weighted-triangle".into(),
        vertices: MeshVertices::Weighted { influences },
        uvs: vec![[0., 0.], [1., 0.], [0., 1.]],
        triangles: vec![0, 1, 2],
        hull: 3,
        width: 10.,
        height: 10.,
        edges: vec![0, 2, 2, 4, 4, 0],
    }
}

fn triangle() -> Mesh {
    mesh(
        [[0., 0.], [2., 0.], [0., 2.]]
            .map(|p| vec![influence(1, p, 1.)])
            .to_vec(),
    )
}

pub(super) fn rig() -> Document {
    let bone = |name: &str, parent, setup| Bone {
        name: name.into(),
        parent,
        length: 10.,
        setup,
    };
    Document {
        constraints: vec![],
        format: FORMAT_VERSION,
        name: "Original weighted rig".into(),
        fps: 30,
        bones: vec![
            bone(
                "root",
                None,
                Transform {
                    x: 10.,
                    y: 20.,
                    rotation: 90.,
                    scale_x: 2.,
                    scale_y: -3.,
                    shear_x: 0.0,
                    shear_y: 0.0,
                },
            ),
            bone(
                "driver",
                Some(0),
                Transform {
                    x: 5.,
                    y: -1.,
                    rotation: 90.,
                    scale_x: 0.5,
                    scale_y: 2.,
                    shear_x: 0.0,
                    shear_y: 0.0,
                },
            ),
            bone(
                "slot",
                Some(0),
                Transform {
                    x: 100.,
                    y: 50.,
                    ..Default::default()
                },
            ),
            bone(
                "overlay",
                None,
                Transform {
                    x: -8.,
                    y: 4.,
                    ..Default::default()
                },
            ),
        ],
        slots: vec![Slot {
            name: "weighted".into(),
            bone: 2,
            attachment: Some("shape".into()),
        }],
        skins: BTreeMap::from([(
            "default".into(),
            BTreeMap::from([(
                "weighted".into(),
                BTreeMap::from([("shape".into(), Attachment::Mesh(triangle()))]),
            )]),
        )]),
        animations: vec![Animation {
            ik_tracks: vec![],
            name: "turn".into(),
            tracks: vec![Track {
                bone: 1,
                property: Property::Rotation,
                keys: vec![
                    Keyframe {
                        time: 0.,
                        value: 0.,
                        curve: Curve::Linear,
                    },
                    Keyframe {
                        time: 1.,
                        value: 90.,
                        curve: Curve::Linear,
                    },
                ],
            }],
        }],
    }
}

pub(super) fn reference(slot: &str) -> AttachmentRef {
    AttachmentRef {
        skin: "default".into(),
        slot: slot.into(),
        placeholder: "shape".into(),
    }
}

pub(super) fn assert_points(actual: &[[f32; 2]], expected: &[[f32; 2]]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        for axis in 0..2 {
            assert!(
                (actual[axis] - expected[axis]).abs() < 0.0001,
                "{actual:?} != {expected:?}"
            );
        }
    }
}

#[test]
fn influence_blends_keep_six_bones_repeated_entries_zeros_order_and_rounded_nonunit_sums() {
    let weighted = mesh(vec![
        vec![influence(1, [1., 2.], 1.)],
        vec![influence(0, [2., 4.], 0.25), influence(1, [2., 4.], 0.75)],
        vec![
            influence(0, [1., 2.], 0.1),
            influence(1, [1., 2.], 0.1),
            influence(2, [1., 2.], 0.1),
            influence(3, [1., 2.], 0.1),
            influence(4, [1., 2.], 0.1),
            influence(5, [1., 2.], 0.3999),
            influence(0, [1., 2.], 0.1),
            influence(5, [-123., 456.], 0.),
        ],
    ]);
    weighted.validate().unwrap();
    weighted.validate_bones(7).unwrap();
    let before = weighted.clone();
    let mut world: Vec<_> = (0..6)
        .map(|i| {
            Affine::from_transform(Transform {
                x: i as f32 * 10.,
                ..Default::default()
            })
        })
        .collect();
    world.push(Affine::from_transform(Transform {
        x: 1000.,
        y: 2000.,
        ..Default::default()
    }));
    let expected = [[11., 2.], [9.5, 4.], [30.9949, 1.9998]];
    assert_points(&mesh_world_vertices(&weighted, 6, &world), &expected);
    assert_points(&mesh_world_vertices(&weighted, 0, &world), &expected);
    let rounded_above_one = mesh(vec![
        vec![
            influence(0, [0., 0.], 0.6),
            influence(1, [0., 0.], 0.4001)
        ];
        3
    ]);
    rounded_above_one.validate().unwrap();
    assert_points(
        &mesh_world_vertices(&rounded_above_one, 6, &world),
        &[[4.001, 0.]; 3],
    );
    assert_eq!(weighted, before);
    assert!(weighted.is_weighted());
    assert_eq!(weighted.vertex_count(), 3);
    assert!(weighted.slot_local_positions().is_none());
    let mut copy = weighted.clone();
    assert!(copy.slot_local_positions_mut().is_none());
    let native = serde_json::to_value(&weighted).unwrap();
    assert_eq!(native["vertices"]["mode"], "weighted");
    assert_eq!(
        native["vertices"]["influences"][2]
            .as_array()
            .unwrap()
            .len(),
        8
    );
    assert_eq!(serde_json::from_value::<Mesh>(native).unwrap(), weighted);
}

#[test]
fn single_influence_uses_the_named_bone_through_parent_rotation_reflection_and_nonuniform_scale() {
    let doc = rig();
    doc.validate().unwrap();
    let scene = evaluate_scene(&doc, Some(0), 0., "default", None).unwrap();
    assert_points(
        &scene.attachments[0].vertices,
        &[[7., 30.], [10., 30.], [7., 22.]],
    );
    // The slot lives at (160, 220); applying it to these already weighted vertices is wrong.
    assert_points(&[scene.world[2].point(0., 0.)], &[[160., 220.]]);
    let turned = evaluate_scene(&doc, Some(0), 1., "default", None).unwrap();
    assert_points(
        &turned.attachments[0].vertices,
        &[[7., 30.], [7., 28.], [-5., 30.]],
    );
    assert_eq!(turned.attachments[0].uvs, triangle().uvs);
    assert_eq!(turned.attachments[0].triangles, [0, 1, 2]);
    assert_eq!(turned.attachments[0].hull, 3);

    let zero_driver = Transform {
        scale_x: 0.,
        scale_y: 0.,
        ..doc.bones[1].setup
    };
    let collapsed = evaluate_scene(&doc, None, 0., "default", Some((1, zero_driver))).unwrap();
    assert_points(&collapsed.attachments[0].vertices, &[[7., 30.]; 3]);
    assert!(pick_attachment(&collapsed.attachments, [7., 30.]).is_none());
    let zero_parent = Transform {
        scale_x: 0.,
        scale_y: 0.,
        ..doc.bones[0].setup
    };
    let collapsed = evaluate_scene(&doc, None, 0., "default", Some((0, zero_parent))).unwrap();
    assert_points(&collapsed.attachments[0].vertices, &[[10., 20.]; 3]);
}

#[test]
fn scene_uses_transient_posed_locals_and_the_same_weighted_geometry_for_bounds_and_picking() {
    let doc = rig();
    let mut session = Session::new(doc.clone()).unwrap();
    let mut pose = session.local_pose(Some(0), 0.)[1];
    pose.x += 10.;
    session
        .edit_pose(
            1,
            EditTarget::Animate {
                animation: 0,
                time: 0.,
            },
            pose,
            &[Property::X],
            false,
        )
        .unwrap();
    let scene = checked_setup_scene_from_locals(
        session.document(),
        "default",
        session.local_pose(Some(0), 0.),
    )
    .unwrap();
    assert_points(
        &scene.attachments[0].vertices,
        &[[7., 50.], [10., 50.], [7., 42.]],
    );
    let bounds = scene.bounds.unwrap();
    assert_points(&[bounds.min, bounds.max], &[[7., 42.], [10., 50.]]);
    assert_eq!(
        pick_attachment(&scene.attachments, [8., 49.]).unwrap().slot,
        0
    );
    assert!(pick_attachment(&scene.attachments, [160., 220.]).is_none());
    assert_eq!(session.document(), &doc);
    assert!(!session.is_dirty());
    session.undo();
    assert!(session.pose_edits().is_empty());
    let restored = checked_setup_scene_from_locals(
        session.document(),
        "default",
        session.local_pose(Some(0), 0.),
    )
    .unwrap();
    assert_points(
        &restored.attachments[0].vertices,
        &[[7., 30.], [10., 30.], [7., 22.]],
    );
    session.redo();
    let restored = checked_setup_scene_from_locals(
        session.document(),
        "default",
        session.local_pose(Some(0), 0.),
    )
    .unwrap();
    assert_points(
        &restored.attachments[0].vertices,
        &scene.attachments[0].vertices,
    );
}

#[test]
fn region_and_slot_local_mesh_paths_apply_the_slot_transform_once_and_keep_draw_order() {
    let mut doc = rig();
    doc.slots.push(Slot {
        name: "overlay".into(),
        bone: 3,
        attachment: Some("shape".into()),
    });
    let region = Region {
        name: None,
        path: "original-region".into(),
        width: 2.,
        height: 2.,
        transform: RegionTransform {
            x: 16.,
            y: 24.,
            ..Default::default()
        },
    };
    doc.skins.get_mut("default").unwrap().insert(
        "overlay".into(),
        BTreeMap::from([("shape".into(), Attachment::Region(region.clone()))]),
    );
    doc.validate().unwrap();
    let scene = evaluate_scene(&doc, Some(0), 0., "default", None).unwrap();
    assert_points(
        &scene.attachments[0].vertices,
        &[[7., 30.], [10., 30.], [7., 22.]],
    );
    assert_points(
        &scene.attachments[1].vertices,
        &[[7., 27.], [9., 27.], [9., 29.], [7., 29.]],
    );
    assert_eq!(
        pick_attachment(&scene.attachments, [8., 28.]).unwrap().slot,
        1
    );
    assert_points(
        &[scene.bounds.unwrap().min, scene.bounds.unwrap().max],
        &[[7., 22.], [10., 30.]],
    );
    let converted = apply_command(
        &doc,
        Command::ConvertRegionToMesh {
            attachment: reference("overlay"),
        },
    )
    .unwrap();
    let scene_mesh = evaluate_scene(&converted, Some(0), 0., "default", None).unwrap();
    assert_eq!(
        scene.attachments[0].vertices,
        scene_mesh.attachments[0].vertices
    );
    assert_eq!(
        scene.attachments[1].vertices,
        scene_mesh.attachments[1].vertices
    );
    let Attachment::Mesh(mesh) = converted.attachment(&reference("overlay")).unwrap() else {
        panic!("Expected converted region")
    };
    assert!(!mesh.is_weighted());
    assert_eq!(mesh.vertex_count(), 4);
    assert_eq!(
        mesh.slot_local_positions().unwrap(),
        region_vertices(&region)
    );
}

#[test]
fn malformed_weights_positions_vertex_counts_and_topology_are_rejected_without_normalization() {
    let original = triangle();
    for influences in [
        vec![],
        vec![vec![influence(1, [0., 0.], 1.)]; 2],
        vec![vec![]; 3],
        vec![vec![influence(1, [0., 0.], 0.)]; 3],
        vec![vec![influence(1, [0., 0.], 0.), influence(1, [1., 2.], 0.)]; 3],
    ] {
        let mut invalid = original.clone();
        invalid.vertices = MeshVertices::Weighted { influences };
        assert!(invalid.validate().is_err());
    }
    for weight in [-0.1, 1.1, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut invalid = original.clone();
        let MeshVertices::Weighted { influences } = &mut invalid.vertices else {
            unreachable!()
        };
        influences[1][0].weight = weight;
        assert!(invalid.validate().is_err());
    }
    for invalid_coordinate in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut invalid = original.clone();
        let MeshVertices::Weighted { influences } = &mut invalid.vertices else {
            unreachable!()
        };
        influences[2][0].position[0] = invalid_coordinate;
        assert!(invalid.validate().is_err());
    }
    for (uvs, triangles, hull, edges) in [
        (vec![[0., 0.]; 2], vec![0, 1, 2], 3, vec![]),
        (original.uvs.clone(), vec![0, 1, 3], 3, vec![]),
        (original.uvs.clone(), vec![0, 1, 1], 3, vec![]),
        (original.uvs.clone(), vec![0, 1, 2], 4, vec![]),
        (original.uvs.clone(), vec![0, 1, 2], 3, vec![0, 6]),
    ] {
        let invalid = Mesh {
            uvs,
            triangles,
            hull,
            edges,
            ..original.clone()
        };
        assert!(invalid.validate().is_err());
    }
    // Validation accepts even a positive subnormal total, without f32 underflow or normalization.
    let tiny = mesh(vec![
        vec![
            influence(1, [0., 0.], f32::from_bits(1)),
            influence(1, [9., 7.], 0.)
        ];
        3
    ]);
    let before = tiny.clone();
    tiny.validate().unwrap();
    assert_eq!(tiny, before);
    let nonunit = mesh(vec![
        vec![
            influence(1, [0., 0.], 0.1),
            influence(1, [2., 3.], 0.2)
        ];
        3
    ]);
    nonunit.validate().unwrap();
}

#[test]
fn all_skin_bone_references_are_checked_including_zero_weight_entries() {
    let mut doc = rig();
    doc.skins
        .insert("inactive".into(), doc.skins["default"].clone());
    doc.validate().unwrap();
    let Attachment::Mesh(mesh) = doc
        .skins
        .get_mut("inactive")
        .unwrap()
        .get_mut("weighted")
        .unwrap()
        .get_mut("shape")
        .unwrap()
    else {
        unreachable!()
    };
    let MeshVertices::Weighted { influences } = &mut mesh.vertices else {
        unreachable!()
    };
    influences[0].push(influence(usize::MAX, [0., 0.], 0.));
    mesh.validate().unwrap();
    assert!(mesh.validate_bones(doc.bones.len()).is_err());
    assert!(doc.validate().is_err());
    assert!(Session::new(doc).is_err());
}

#[test]
fn weighted_edit_and_subdivision_rejection_preserve_document_pending_pose_and_history() {
    let before = rig();
    let mut session = Session::new(before.clone()).unwrap();
    let mut pose = session.local_pose(Some(0), 0.)[1];
    pose.x += 3.;
    session
        .edit_pose(
            1,
            EditTarget::Animate {
                animation: 0,
                time: 0.,
            },
            pose,
            &[Property::X],
            false,
        )
        .unwrap();
    let pending = session.pose_edits().clone();
    session
        .dispatch(Command::RenameBone {
            bone: 1,
            name: "rename for redo".into(),
        })
        .unwrap();
    session.undo();
    let undo_available = session.can_undo();
    for command in [
        Command::SetMeshVertex {
            attachment: reference("weighted"),
            vertex: 0,
            position: [9., 9.],
        },
        Command::SplitMeshTriangle {
            attachment: reference("weighted"),
            triangle: 0,
        },
        Command::ConvertRegionToMesh {
            attachment: reference("weighted"),
        },
        Command::Batch {
            commands: vec![
                Command::RenameSlot {
                    slot: 0,
                    name: "temporary".into(),
                },
                Command::SetMeshVertex {
                    attachment: reference("temporary"),
                    vertex: 0,
                    position: [9., 9.],
                },
            ],
        },
    ] {
        assert!(session.dispatch(command).is_err());
        assert_eq!(session.document(), &before);
        assert_eq!(session.pose_edits(), &pending);
        assert!(!session.is_dirty());
        assert_eq!(session.can_undo(), undo_available);
        assert!(session.can_redo());
    }
    session.redo();
    assert_eq!(session.document().bones[1].name, "rename for redo");
    assert_eq!(session.pose_edits(), &pending);
    session.undo();
    assert_eq!(session.document(), &before);
}

#[test]
fn renaming_bone_and_slot_preserves_weighted_geometry_and_all_skin_payloads_and_undo() {
    let mut before = rig();
    let mut alternate = before.skins["default"].clone();
    let Attachment::Mesh(mesh) = alternate
        .get_mut("weighted")
        .unwrap()
        .get_mut("shape")
        .unwrap()
    else {
        unreachable!()
    };
    let MeshVertices::Weighted { influences } = &mut mesh.vertices else {
        unreachable!()
    };
    influences[0][0].position = [1., 0.];
    before.skins.insert("alternate".into(), alternate);
    let mut session = Session::new(before.clone()).unwrap();
    session
        .dispatch(Command::Batch {
            commands: vec![
                Command::RenameBone {
                    bone: 1,
                    name: "renamed driver".into(),
                },
                Command::RenameSlot {
                    slot: 0,
                    name: "renamed slot".into(),
                },
            ],
        })
        .unwrap();
    let renamed = session.document().clone();
    for skin in ["default", "alternate"] {
        assert_eq!(
            before.skins[skin]["weighted"],
            renamed.skins[skin]["renamed slot"]
        );
        for time in [0., 0.5, 1.] {
            let old = evaluate_scene(&before, Some(0), time, skin, None).unwrap();
            let new = evaluate_scene(&renamed, Some(0), time, skin, None).unwrap();
            assert_eq!(old.attachments[0].vertices, new.attachments[0].vertices);
            assert_eq!(old.attachments[0].uvs, new.attachments[0].uvs);
            assert_eq!(old.attachments[0].triangles, new.attachments[0].triangles);
        }
    }
    let serialized = serde_json::to_string(&renamed).unwrap();
    assert_eq!(
        serde_json::from_str::<Document>(&serialized).unwrap(),
        renamed
    );
    session.undo();
    assert_eq!(session.document(), &before);
    session.redo();
    assert_eq!(session.document(), &renamed);
}

#[test]
fn native_mesh_modes_and_influences_reject_unknown_fields_and_legacy_arrays() {
    let original = triangle();
    let encoded = serde_json::to_value(&original).unwrap();
    for vertices in [
        serde_json::json!([[0, 0], [2, 0], [0, 2]]),
        serde_json::json!({"mode":"slotLocal","positions":[[0,0],[2,0],[0,2]],"influences":[]}),
        serde_json::json!({"mode":"weighted","influences":[[],[],[]],"positions":[]}),
        serde_json::json!({"mode":"unknown","influences":[]}),
        serde_json::json!({"influences":[]}),
    ] {
        let mut invalid = encoded.clone();
        invalid["vertices"] = vertices;
        assert!(serde_json::from_value::<Mesh>(invalid).is_err());
    }
    let mut invalid = encoded;
    invalid["vertices"]["influences"][0][0]["extra"] = true.into();
    assert!(serde_json::from_value::<Mesh>(invalid).is_err());
    let mut legacy = serde_json::to_value(rig()).unwrap();
    legacy["format"] = 3.into();
    assert!(
        serde_json::from_value::<Document>(legacy)
            .unwrap()
            .validate()
            .is_err()
    );
}
