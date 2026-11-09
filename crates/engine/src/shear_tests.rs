use super::*;
use peachy_document::*;
use serde_json::json;

fn rig() -> Document {
    Document {
        constraints: vec![],
        format: FORMAT_VERSION,
        name: "Original shear contracts".into(),
        fps: 30,
        bones: vec![
            Bone {
                name: "root".into(),
                parent: None,
                length: 20.,
                setup: Transform::default(),
            },
            Bone {
                name: "child".into(),
                parent: Some(0),
                length: 10.,
                setup: Transform::default(),
            },
        ],
        slots: vec![],
        skins: Default::default(),
        animations: vec![Animation {
            name: "shear".into(),
            tracks: vec![],
            physics: vec![],
            physics_resets: vec![],
        }],
    }
}

fn assert_columns(matrix: Affine, observed: [f64; 4]) {
    for (actual, expected) in [matrix.a, matrix.b, matrix.c, matrix.d]
        .into_iter()
        .zip(observed)
    {
        assert!(
            (f64::from(actual) - expected).abs() <= 0.000001,
            "{actual} differs from {expected}"
        );
    }
}

#[test]
fn native_bone_shear_is_required_finite_and_region_transform_cannot_drop_it() {
    let mut value = json!(Transform::default());
    for field in ["shear_x", "shear_y"] {
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<Transform>(missing).is_err());
    }
    value["unknown"] = json!(0);
    assert!(serde_json::from_value::<Transform>(value).is_err());
    for (x, y) in [(f32::NAN, 0.), (0., f32::INFINITY), (f32::NEG_INFINITY, 0.)] {
        let transform = Transform {
            shear_x: x,
            shear_y: y,
            ..Default::default()
        };
        assert!(!finite_transform(transform));
        let mut doc = rig();
        doc.bones[0].setup = transform;
        assert!(doc.validate().is_err());
        assert!(RegionTransform::try_from(transform).is_err());
    }
    let region = RegionTransform {
        x: 3.,
        y: -7.,
        rotation: 45.,
        scale_x: -2.,
        scale_y: 0.3,
    };
    assert!(finite_region_transform(region));
    let promoted: Transform = region.into();
    assert_eq!((promoted.shear_x, promoted.shear_y), (0., 0.));
    assert_eq!(RegionTransform::try_from(promoted).unwrap(), region);
    for (x, y) in [(0.000001, 0.), (0., -720.)] {
        assert!(
            RegionTransform::try_from(Transform {
                shear_x: x,
                shear_y: y,
                ..promoted
            })
            .is_err()
        );
    }
    let mut encoded = json!(region);
    encoded["shear_x"] = json!(0);
    assert!(serde_json::from_value::<RegionTransform>(encoded.clone()).is_err());
    let payload = json!({"command":"setRegionTransform", "attachment":{"skin":"default","slot":"body","placeholder":"body"},"transform":encoded});
    assert!(serde_json::from_value::<Command>(payload).is_err());
}

#[test]
fn scalar_shear_keys_are_setup_relative_and_default_authoring_keys_the_pair() {
    let mut document = rig();
    document.bones[0].setup.shear_x = 5.;
    document.bones[0].setup.shear_y = -7.;
    document.animations[0].tracks = vec![Track {
        bone: 0,
        property: Property::ShearX,
        keys: vec![
            Keyframe {
                time: 0.25,
                value: -20.,
                curve: Curve::Linear,
            },
            Keyframe {
                time: 1.,
                value: 40.,
                curve: Curve::Stepped,
            },
        ],
    }];
    assert_eq!(Property::ALL[5..], [Property::ShearX, Property::ShearY]);
    assert_eq!(
        transform_group(&[Property::ShearX]),
        vec![Property::ShearX, Property::ShearY]
    );
    assert_eq!(Property::ShearY.neutral(), 0.);
    assert_eq!(json!(Property::ShearX), json!("shearX"));
    assert_eq!(local_pose(&document, Some(0), 0.)[0].shear_x, 5.);
    assert_eq!(local_pose(&document, Some(0), 0.625)[0].shear_x, 15.);
    assert_eq!(local_pose(&document, Some(0), 2.)[0].shear_x, 45.);
    let mut session = Session::new(document.clone()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.37,
    };
    let mut posed = session.local_pose(Some(0), 0.37)[0];
    posed.shear_x = 721.;
    session
        .edit_pose(0, target, posed, &[Property::ShearX], false)
        .unwrap();
    assert_eq!(session.document(), &document);
    assert_eq!(session.local_pose(Some(0), 0.37)[0].shear_x, 721.);
    session.key_edited(0, 0.37, None).unwrap();
    let keys = &session.document().animations[0].tracks;
    let x = keys
        .iter()
        .find(|t| t.property == Property::ShearX)
        .unwrap();
    let y = keys
        .iter()
        .find(|t| t.property == Property::ShearY)
        .unwrap();
    assert_eq!(
        x.keys
            .iter()
            .find(|k| k.time == (11.0 / 30.0))
            .unwrap()
            .value,
        716.
    );
    assert_eq!(y.keys[0].value, 0.);
    session.undo();
    assert_eq!(session.document(), &document);
    assert_eq!(session.local_pose(Some(0), 0.37)[0].shear_x, 721.);
}

#[test]
fn original_unit_bone_calibration_columns_are_reproduced_by_the_compiled_evaluator() {
    // Four original public-API observations; calibration choices were frozen
    // before held-out fixtures. No official implementation or constants were read.
    for (rotation, columns) in [
        (
            1.,
            [
                0.9998476951518914,
                -0.017452429896647987,
                0.017452406695078727,
                0.9998476947469063,
            ],
        ),
        (
            10.,
            [
                0.9848077525644832,
                -0.17364820305867,
                0.17364818020610434,
                0.9848077485349589,
            ],
        ),
        (
            45.,
            [
                0.7071067729823045,
                -0.7071068057992762,
                0.7071067893907904,
                0.7071067565738179,
            ],
        ),
        (
            90.,
            [
                -2.3205103333142417e-8,
                -0.9999999999999989,
                0.9999999999999998,
                -4.641020666628482e-8,
            ],
        ),
    ] {
        assert_columns(
            Affine::from_transform(Transform {
                rotation,
                ..Default::default()
            }),
            columns,
        );
    }
}

#[test]
fn held_out_huge_sums_remain_finite_preserve_order_and_do_not_normalize_authored_angles() {
    let huge = 1e20_f32;
    assert_columns(
        Affine::from_transform(Transform {
            rotation: huge,
            shear_x: huge,
            shear_y: -huge,
            ..Default::default()
        }),
        [0.2030285548692655, 1., 0.979172817181777, 0.],
    );
    assert_columns(
        Affine::from_transform(Transform {
            rotation: f32::MAX,
            shear_x: f32::MAX,
            shear_y: -f32::MAX,
            ..Default::default()
        }),
        [0.7854046370641214, 1., 0.6189826783345199, 0.],
    );
    let transform = Transform {
        rotation: huge,
        shear_x: -huge,
        shear_y: -huge,
        ..Default::default()
    };
    assert_columns(Affine::from_transform(transform), [1., 1., 0., 0.]);
    let mut document = rig();
    document.bones[0].setup = transform;
    document.validate().unwrap();
    assert_eq!(local_pose(&document, None, 0.)[0], transform);
    let encoded = serde_json::to_string(&document).unwrap();
    assert_eq!(
        serde_json::from_str::<Document>(&encoded).unwrap(),
        document
    );
}

#[test]
fn affine_inverse_preserves_known_points_small_scales_and_translation_free_directions() {
    let matrix = Affine {
        a: 2.,
        b: 1.,
        c: 0.,
        d: 3.,
        x: 10.,
        y: -7.,
    };
    assert_eq!(matrix.point(4., -2.), [16., -13.]);
    assert_eq!(matrix.inverse_point(16., -13.), Some([4., -2.]));
    let distant = Affine {
        x: 1e8,
        y: -1e8,
        ..matrix
    };
    assert_eq!(distant.inverse_vector(4., 6.), Some([1., 2.]));
    let small = Affine {
        a: 1e-20,
        b: 0.,
        c: 0.,
        d: -2e-20,
        x: 0.,
        y: 0.,
    };
    let point = small.inverse_point(3e-20, 8e-20).unwrap();
    assert!((point[0] - 3.).abs() < 0.00001 && (point[1] + 4.).abs() < 0.00001);
    let singular = Affine {
        a: 1.,
        b: 2.,
        c: 3.,
        d: 6.,
        ..matrix
    };
    assert_eq!(singular.inverse_point(5., 7.), None);
    assert_eq!(singular.inverse_vector(5., 7.), None);
    assert_eq!(matrix.inverse_point(f32::NAN, 0.), None);
    assert_eq!(matrix.inverse_vector(0., f32::INFINITY), None);
    assert_eq!(
        Affine {
            a: f32::INFINITY,
            ..matrix
        }
        .inverse_vector(1., 0.),
        None
    );
    assert_eq!(
        Affine {
            x: f32::NAN,
            ..matrix
        }
        .inverse_point(1., 0.),
        None
    );
    assert_eq!(
        Affine {
            a: f32::from_bits(1),
            b: 0.,
            c: 0.,
            d: 1.,
            ..matrix
        }
        .inverse_vector(1., 0.),
        None
    );
}

#[test]
fn world_rotation_accounts_for_shear_reflection_and_large_parent_origins_without_redecomposition() {
    let local = Transform {
        x: 4.,
        y: -2.,
        rotation: 725.,
        scale_x: -1.2,
        scale_y: 0.8,
        shear_x: 18.,
        shear_y: -22.,
    };
    let parent = Affine::from_transform(Transform {
        x: 1e8,
        y: -1e8,
        rotation: 37.,
        scale_x: -2.,
        scale_y: 0.6,
        shear_x: 14.,
        shear_y: -11.,
    });
    let shown = transform_values(local, parent, Axes::World);
    assert_eq!(
        set_transform_value(
            local,
            parent,
            Axes::World,
            Property::Rotation,
            shown.rotation
        )
        .unwrap(),
        local
    );
    let changed = set_transform_value(local, parent, Axes::World, Property::Rotation, 62.).unwrap();
    let world = transform_values(changed, parent, Axes::World);
    assert!((world.rotation - 62.).abs() < 0.001, "{}", world.rotation);
    assert!((changed.rotation - local.rotation).abs() <= 180.);
    assert_eq!(
        Transform {
            rotation: local.rotation,
            ..changed
        },
        local
    );
    assert!(
        set_transform_value(
            Transform {
                scale_x: 0.,
                ..local
            },
            parent,
            Axes::World,
            Property::Rotation,
            62.
        )
        .is_err()
    );
    let tiny = Affine {
        a: 1e-20,
        b: 0.,
        c: 0.,
        d: -2e-20,
        x: 1e8,
        y: -1e8,
    };
    let changed = set_transform_value(local, tiny, Axes::World, Property::Rotation, 17.).unwrap();
    assert!((transform_values(changed, tiny, Axes::World).rotation - 17.).abs() < 0.001);
}

#[test]
fn oblique_axis_scaling_solves_bone_coordinates_and_preserves_other_authored_parameters() {
    let local = Transform {
        x: 4.,
        y: -2.,
        rotation: 23.,
        scale_x: 1.2,
        scale_y: -0.7,
        shear_x: 30.,
        shear_y: -15.,
    };
    let parent = Affine::from_transform(Transform {
        x: 17.,
        y: -6.,
        rotation: -39.,
        scale_x: -1.2,
        scale_y: 0.8,
        shear_x: 12.,
        shear_y: -5.,
    });
    let world = parent.then(Affine::from_transform(local));
    let axes = transform_axes(local, parent, Axes::World, TransformTool::Scale);
    for (axis, column) in axes
        .into_iter()
        .zip([[world.a, world.c], [world.b, world.d]])
    {
        let length = column[0].hypot(column[1]);
        assert!((axis[0] - column[0] / length).abs() < 0.00001);
        assert!((axis[1] - column[1] / length).abs() < 0.00001);
    }
    assert!((axes[0][0] * axes[1][0] + axes[0][1] * axes[1][1]).abs() > 0.1);
    for axis in [0, 1] {
        let start = world.point(4., 3.);
        let end = if axis == 0 {
            world.point(8., 3.)
        } else {
            world.point(4., 6.)
        };
        let changed = drag_transform(
            local,
            parent,
            TransformGesture {
                tool: TransformTool::Scale,
                axes: Axes::World,
                axis: Some(axis),
                start,
                end,
                snap_rotation: false,
            },
        )
        .unwrap();
        let actual = if axis == 0 {
            changed.scale_x / local.scale_x
        } else {
            changed.scale_y / local.scale_y
        };
        assert!((actual - 2.).abs() < 0.00001, "{actual}");
        assert_eq!(
            Transform {
                scale_x: local.scale_x,
                scale_y: local.scale_y,
                ..changed
            },
            local
        );
        if axis == 0 {
            assert_eq!(changed.scale_y, local.scale_y);
        } else {
            assert_eq!(changed.scale_x, local.scale_x);
        }
    }
    let small = Affine {
        a: 1e-20,
        b: 0.,
        c: 0.,
        d: 2e-20,
        x: 0.,
        y: 0.,
    };
    let world = small.then(Affine::from_transform(local));
    let changed = drag_transform(
        local,
        small,
        TransformGesture {
            tool: TransformTool::Scale,
            axes: Axes::Local,
            axis: Some(0),
            start: world.point(4., 3.),
            end: world.point(8., 3.),
            snap_rotation: false,
        },
    )
    .unwrap();
    assert!((changed.scale_x / local.scale_x - 2.).abs() < 0.00001);
    // A zero opposite scale must not prevent scaling the live axis.
    for axis in [0, 1] {
        let flat = if axis == 0 {
            Transform {
                scale_y: 0.,
                ..local
            }
        } else {
            Transform {
                scale_x: 0.,
                ..local
            }
        };
        let world = parent.then(Affine::from_transform(flat));
        let start = if axis == 0 {
            world.point(4., 0.)
        } else {
            world.point(0., 3.)
        };
        let end = if axis == 0 {
            world.point(8., 0.)
        } else {
            world.point(0., 6.)
        };
        let changed = drag_transform(
            flat,
            parent,
            TransformGesture {
                tool: TransformTool::Scale,
                axes: Axes::World,
                axis: Some(axis),
                start,
                end,
                snap_rotation: false,
            },
        )
        .unwrap();
        let ratio = if axis == 0 {
            changed.scale_x / flat.scale_x
        } else {
            changed.scale_y / flat.scale_y
        };
        assert!((ratio - 2.).abs() < 0.00001);
        assert_eq!(
            Transform {
                scale_x: flat.scale_x,
                scale_y: flat.scale_y,
                ..changed
            },
            flat
        );
    }
}

#[test]
fn explicit_shear_handles_change_one_angle_and_do_not_invent_empty_space_or_rotation_snap_behavior()
{
    let local = Transform {
        x: 4.,
        y: -2.,
        rotation: 725.,
        scale_x: -1.2,
        scale_y: 0.8,
        shear_x: -710.,
        shear_y: 1083.,
    };
    let parent = Affine::from_transform(Transform {
        rotation: -37.,
        scale_x: 0.7,
        scale_y: -1.3,
        shear_x: 8.,
        shear_y: -11.,
        ..Default::default()
    });
    let ray = |degrees: f32| {
        parent.point(
            local.x + 20. * degrees.to_radians().cos(),
            local.y + 20. * degrees.to_radians().sin(),
        )
    };
    let gesture = TransformGesture {
        tool: TransformTool::Shear,
        axes: Axes::Parent,
        axis: None,
        start: ray(10.),
        end: ray(40.),
        snap_rotation: false,
    };
    assert!(
        drag_transform(local, parent, gesture)
            .unwrap_err()
            .contains("explicit")
    );
    for axis in [0, 1] {
        let changed = drag_transform(
            local,
            parent,
            TransformGesture {
                axis: Some(axis),
                ..gesture
            },
        )
        .unwrap();
        let delta = if axis == 0 {
            changed.shear_x - local.shear_x
        } else {
            changed.shear_y - local.shear_y
        };
        assert!((delta - 30.).abs() < 0.001);
        assert_eq!(
            Transform {
                shear_x: local.shear_x,
                shear_y: local.shear_y,
                ..changed
            },
            local
        );
        assert_eq!(
            changed,
            drag_transform(
                local,
                parent,
                TransformGesture {
                    axis: Some(axis),
                    snap_rotation: true,
                    ..gesture
                }
            )
            .unwrap()
        );
        if axis == 0 {
            assert_eq!(changed.shear_y, local.shear_y);
        } else {
            assert_eq!(changed.shear_x, local.shear_x);
        }
    }
    assert!(
        drag_transform(
            local,
            parent,
            TransformGesture {
                axis: Some(0),
                start: parent.point(local.x, local.y),
                ..gesture
            }
        )
        .is_err()
    );
    assert!(
        drag_transform(
            local,
            parent,
            TransformGesture {
                axis: Some(2),
                ..gesture
            }
        )
        .is_err()
    );
}

#[test]
fn local_shear_entry_and_parent_world_translation_preserve_the_existing_local_frame() {
    let local = Transform {
        x: 4.,
        y: -2.,
        rotation: 23.,
        scale_x: 1.2,
        scale_y: -0.7,
        shear_x: 30.,
        shear_y: -15.,
    };
    let zero = Affine {
        a: 0.,
        b: 0.,
        c: 0.,
        d: 0.,
        x: 0.,
        y: 0.,
    };
    let changed = set_transform_value(local, zero, Axes::World, Property::ShearY, -450.).unwrap();
    assert_eq!(
        Transform {
            shear_y: local.shear_y,
            ..changed
        },
        local
    );
    assert_eq!(changed.shear_y, -450.);
    let parent = Affine::from_transform(Transform {
        rotation: 31.,
        scale_x: -1.3,
        scale_y: 0.7,
        shear_x: 17.,
        shear_y: -9.,
        x: 10.,
        y: 30.,
    });
    let shown = transform_values(local, parent, Axes::World);
    let changed =
        set_transform_value(local, parent, Axes::World, Property::X, shown.x + 7.).unwrap();
    assert!((transform_values(changed, parent, Axes::World).x - shown.x - 7.).abs() < 0.0001);
    assert_eq!(
        Transform {
            x: local.x,
            y: local.y,
            ..changed
        },
        local
    );
    let parent_changed = set_transform_value(local, parent, Axes::Parent, Property::Y, 8.).unwrap();
    assert_eq!(parent_changed.y, 8.);
    let rotation_only = Transform {
        shear_x: 0.,
        shear_y: 0.,
        ..local
    };
    assert_eq!(
        transform_values(local, parent, Axes::Local).x,
        transform_values(rotation_only, parent, Axes::Local).x
    );
    assert_eq!(
        transform_values(local, parent, Axes::Local).y,
        transform_values(rotation_only, parent, Axes::Local).y
    );
}

#[test]
fn failed_singular_tool_conversion_keeps_valid_pose_preview_and_redo_until_cancel() {
    let mut document = rig();
    document.bones[0].setup.shear_x = 14.;
    let mut session = Session::new(document.clone()).unwrap();
    session
        .dispatch(Command::SetBoneLength {
            bone: 1,
            length: 37.,
        })
        .unwrap();
    session.undo();
    session.begin_transaction();
    let local = Transform {
        shear_x: 32.,
        ..document.bones[0].setup
    };
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.137,
    };
    session
        .preview_edit_pose(0, target, local, &[Property::ShearX], false)
        .unwrap();
    let pending = session.pose_edits().clone();
    let singular = Affine {
        a: 1.,
        b: 2.,
        c: 3.,
        d: 6.,
        x: 0.,
        y: 0.,
    };
    let result = set_transform_value(local, singular, Axes::World, Property::Rotation, 43.)
        .and_then(|pose| session.preview_edit_pose(0, target, pose, &[Property::Rotation], false));
    assert!(result.is_err());
    assert_eq!(session.document(), &document);
    assert_eq!(session.pose_edits(), &pending);
    session.cancel_transaction();
    assert!(session.can_redo());
    assert!(!session.can_undo());
    assert!(session.pose_edits().is_empty());
    session.redo();
    assert_eq!(session.document().bones[1].length, 37.);
}

#[test]
fn sheared_weighted_and_region_scene_matches_original_external_observation_points() {
    use std::collections::BTreeMap;
    let mut document = rig();
    document.bones = vec![
        Bone {
            name: "root".into(),
            parent: None,
            length: 0.,
            setup: Transform {
                x: 13.,
                y: -8.,
                rotation: 23.,
                scale_x: -1.7,
                scale_y: 0.6,
                shear_x: 11.,
                shear_y: -17.,
            },
        },
        Bone {
            name: "child".into(),
            parent: Some(0),
            length: 20.,
            setup: Transform {
                x: 31.,
                y: -7.,
                rotation: -29.,
                scale_x: -0.9,
                scale_y: 1.4,
                shear_x: 26.,
                shear_y: -13.,
            },
        },
        Bone {
            name: "tip".into(),
            parent: Some(1),
            length: 10.,
            setup: Transform {
                x: 17.,
                y: -11.,
                rotation: 41.,
                scale_x: 0.7,
                scale_y: -1.2,
                shear_x: -9.,
                shear_y: 4.,
            },
        },
    ];
    let influence = |bone, position, weight| BoneInfluence {
        bone,
        position,
        weight,
    };
    let mesh = Mesh {
        name: None,
        path: "original-mesh".into(),
        uvs: vec![[0., 0.], [1., 0.], [0., 1.]],
        vertices: MeshVertices::Weighted {
            influences: vec![
                vec![influence(1, [2., 3.], 1.)],
                vec![influence(2, [-1., 4.], 1.)],
                vec![
                    influence(0, [3., -2.], 0.25),
                    influence(1, [2., 3.], 0.5),
                    influence(2, [100., -30.], 0.),
                    influence(1, [2., 3.], 0.25),
                ],
            ],
        },
        triangles: vec![0, 1, 2],
        hull: 3,
        edges: vec![],
        width: 0.,
        height: 0.,
    };
    document.slots = vec![
        Slot {
            color: [255; 4],
            name: "mesh".into(),
            bone: 0,
            attachment: Some("mesh".into()),
        },
        Slot {
            color: [255; 4],
            name: "region".into(),
            bone: 2,
            attachment: Some("region".into()),
        },
    ];
    document.skins = BTreeMap::from([(
        "default".into(),
        BTreeMap::from([
            (
                "mesh".into(),
                BTreeMap::from([("mesh".into(), Attachment::Mesh(mesh.clone()))]),
            ),
            (
                "region".into(),
                BTreeMap::from([(
                    "region".into(),
                    Attachment::Region(Region {
                        name: None,
                        path: "original-region".into(),
                        width: 2.,
                        height: 4.,
                        transform: RegionTransform::default(),
                    }),
                )]),
            ),
        ]),
    )]);
    document.validate().unwrap();
    let scene = evaluate_scene(&document, None, 0., "default", None).unwrap();
    // Expected points come from recorded public-API parent/child world columns,
    // not from a second implementation of the new evaluator.
    crate::weighted_tests::assert_points(
        &scene.attachments[0].vertices,
        &[
            [-31.88034812527825_f64 as f32, -40.69059070758602_f64 as f32],
            [15.228562246602912_f64 as f32, -20.19026929319162_f64 as f32],
            [
                -21.685925474753564_f64 as f32,
                -33.52927059156919_f64 as f32,
            ],
        ],
    );
    crate::weighted_tests::assert_points(
        &scene.attachments[1].vertices,
        &[
            [1.7407625995715375_f64 as f32, -26.05636106082978_f64 as f32],
            [2.213255439072432_f64 as f32, -25.208692866826713_f64 as f32],
            [
                11.205121870426682_f64 as f32,
                -21.297965021734605_f64 as f32,
            ],
            [10.732629030925787_f64 as f32, -22.14563321573767_f64 as f32],
        ],
    );
    let centroid = scene.attachments[0]
        .vertices
        .iter()
        .fold([0., 0.], |a, p| [a[0] + p[0] / 3., a[1] + p[1] / 3.]);
    assert_eq!(
        pick_attachment(&scene.attachments, centroid).unwrap().slot,
        0
    );
    let bounds = scene.bounds.unwrap();
    assert!(bounds.min[0] < -31.88 && bounds.max[0] > 15.22);
    assert_eq!(
        document.attachment(&AttachmentRef {
            skin: "default".into(),
            slot: "mesh".into(),
            placeholder: "mesh".into()
        }),
        Some(&Attachment::Mesh(mesh))
    );
}

#[test]
fn bind_add_direct_and_unbind_share_sheared_pending_pose_geometry_and_one_step_history() {
    use crate::binding_tests::{mesh, pending_with_redo, slot_local_rig};
    use crate::weighted_tests::{assert_points, reference};
    let mut document = slot_local_rig();
    document.bones[0].setup.shear_x = 11.;
    document.bones[0].setup.shear_y = -18.;
    document.bones[1].setup.shear_x = -23.;
    document.bones[1].setup.shear_y = 8.;
    document.bones[2].setup.shear_y = 13.;
    let mut session = pending_with_redo(document.clone());
    let pending = session.pose_edits().clone();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.137,
    };
    let geometry = |session: &Session| {
        scene_from_locals(
            session.document(),
            "default",
            session.local_pose(Some(0), 0.137),
        )
        .unwrap()
        .attachments[0]
            .vertices
            .clone()
    };
    let before = geometry(&session);
    session
        .bind_mesh_to_bone(reference("weighted"), 1, target)
        .unwrap();
    assert_points(&geometry(&session), &before);
    session
        .add_mesh_bone_binding(reference("weighted"), 2, target)
        .unwrap();
    assert_points(&geometry(&session), &before);
    session
        .set_mesh_bone_weight(reference("weighted"), 0, 2, 0.3, target)
        .unwrap();
    assert_points(&geometry(&session), &before);
    let weighted = session.document().clone();
    let before_unbind = geometry(&session);
    session.unbind_mesh(reference("weighted"), target).unwrap();
    assert_points(&geometry(&session), &before_unbind);
    assert!(!mesh(session.document()).is_weighted());
    assert_eq!(session.pose_edits(), &pending);
    assert_eq!(session.document().animations, document.animations);
    session.undo();
    assert_eq!(session.document(), &weighted);
    assert_eq!(session.pose_edits(), &pending);
    session.redo();
    assert!(!mesh(session.document()).is_weighted());
    assert_eq!(session.pose_edits(), &pending);
}

#[test]
fn shear_singular_binding_rejects_only_required_inverses_and_preserves_redo() {
    use crate::binding_tests::{pending_with_redo, slot_local_rig};
    use crate::weighted_tests::reference;
    let mut document = slot_local_rig();
    document.bones[1].setup.shear_x = 90.; // Its two columns are parallel at any rotation.
    let mut session = pending_with_redo(document.clone());
    let pending = session.pose_edits().clone();
    assert!(
        session
            .bind_mesh_to_bone(reference("weighted"), 1, EditTarget::Setup)
            .unwrap_err()
            .contains("singular")
    );
    assert_eq!(session.document(), &document);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
    assert!(!session.is_dirty());
    let mut unrelated = slot_local_rig();
    unrelated.bones[3].setup.shear_x = 90.;
    let mut session = Session::new(unrelated).unwrap();
    session
        .bind_mesh_to_bone(reference("weighted"), 1, EditTarget::Setup)
        .unwrap();
    let mut weighted = session.document().clone();
    weighted.bones[2].setup.shear_x = 90.;
    let mut session = Session::new(weighted.clone()).unwrap();
    assert!(
        session
            .unbind_mesh(reference("weighted"), EditTarget::Setup)
            .unwrap_err()
            .contains("singular")
    );
    assert_eq!(session.document(), &weighted);
    assert!(!session.can_undo());
    // Direct has no slot inverse: finite weighted geometry can still be inspected/edited.
    session
        .set_mesh_bone_weight(reference("weighted"), 0, 1, 1., EditTarget::Setup)
        .unwrap();
    assert_eq!(session.document(), &weighted);
}

#[test]
fn shear_clipboard_and_retiming_keep_scalar_identity_curves_fractional_neighbors_and_setup_offsets()
{
    let mut document = rig();
    document.bones[0].setup.shear_x = 5.;
    document.bones[0].setup.shear_y = -7.;
    document.bones[1].setup.shear_x = 50.;
    document.bones[1].setup.shear_y = -20.;
    document.animations[0].tracks = vec![
        Track {
            bone: 0,
            property: Property::ShearX,
            keys: vec![
                Keyframe {
                    time: 0.1,
                    value: 4.,
                    curve: Curve::Bezier([0.2, 0.1, 0.8, 0.9]),
                },
                Keyframe {
                    time: 0.100005,
                    value: 6.,
                    curve: Curve::Stepped,
                },
            ],
        },
        Track {
            bone: 0,
            property: Property::ShearY,
            keys: vec![Keyframe {
                time: 0.1,
                value: -3.,
                curve: Curve::Stepped,
            }],
        },
    ];
    document.animations.push(Animation {
        name: "destination".into(),
        tracks: vec![],
        physics: vec![],
        physics_resets: vec![],
    });
    let keys = vec![
        TimelineKey {
            bone: 0,
            property: Property::ShearX,
            time: 0.1,
        },
        TimelineKey {
            bone: 0,
            property: Property::ShearY,
            time: 0.1,
        },
    ];
    let clipboard = copy_keys(&document, 0, &keys).unwrap();
    let mut session = Session::new(document.clone()).unwrap();
    session
        .dispatch(Command::PasteKeys {
            animation: 1,
            bone: 1,
            time: 1.5,
            clipboard,
        })
        .unwrap();
    let locals = session.local_pose(Some(1), 1.5);
    assert_eq!(locals[1].shear_x, 54.);
    assert_eq!(locals[1].shear_y, -23.);
    assert_eq!(session.document().animations[1].duration(), 1.5);
    assert_eq!(
        session.document().animations[1].tracks[0].keys[0].curve,
        document.animations[0].tracks[0].keys[0].curve
    );
    session.undo();
    assert_eq!(session.document(), &document);
    let moved = apply_command(
        &document,
        Command::MoveKeys {
            animation: 0,
            keys: vec![keys[0]],
            delta: 0.2,
            copy: false,
        },
    )
    .unwrap();
    assert_eq!(
        moved.animations[0].tracks[0].keys[0],
        document.animations[0].tracks[0].keys[1]
    );
    assert_eq!(
        moved.animations[0].tracks[0].keys[1].curve,
        document.animations[0].tracks[0].keys[0].curve
    );
    assert_eq!(
        moved.animations[0].tracks[1],
        document.animations[0].tracks[1]
    );
}
