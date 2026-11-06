use super::*;
use peachy_document::*;
use serde_json::json;

fn rig() -> Document {
    Document {
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
