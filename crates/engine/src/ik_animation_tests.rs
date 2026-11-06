use crate::*;
use peachy_document::*;
use std::collections::BTreeMap;

fn frame(time: f32, mix: f32, bend_positive: bool, curve: IkCurve) -> IkKeyframe {
    IkKeyframe {
        time,
        mix,
        bend_positive,
        curve,
    }
}
fn track(keys: Vec<IkKeyframe>) -> IkTrack {
    IkTrack {
        constraint: 0,
        keys,
    }
}
fn setup() -> IkSetup {
    IkSetup {
        mix: 0.25,
        bend_positive: false,
    }
}

fn rig() -> Document {
    Document {
        format: FORMAT_VERSION,
        name: "Original animated IK".into(),
        fps: 30,
        bones: vec![
            Bone {
                name: "root".into(),
                parent: None,
                length: 0.,
                setup: Transform::default(),
            },
            Bone {
                name: "arm".into(),
                parent: Some(0),
                length: 10.,
                setup: Transform::default(),
            },
            Bone {
                name: "target".into(),
                parent: Some(0),
                length: 0.,
                setup: Transform {
                    y: 10.,
                    ..Transform::default()
                },
            },
        ],
        constraints: vec![Constraint::Ik(IkConstraint {
            name: "aim".into(),
            bones: vec![1],
            target: 2,
            setup: IkSetup {
                mix: 0.,
                bend_positive: true,
            },
        })],
        slots: vec![],
        skins: BTreeMap::new(),
        animations: vec![Animation {
            name: "mix".into(),
            tracks: vec![],
            ik_tracks: vec![track(vec![
                frame(0., 0., true, IkCurve::Linear),
                frame(1., 1., false, IkCurve::Linear),
            ])],
        }],
    }
}

#[test]
fn grouped_absolute_settings_hold_setup_before_first_and_flip_bend_at_exact_key() {
    let mut keys = track(vec![
        frame(0.25, 1., true, IkCurve::Linear),
        frame(1., 0.5, false, IkCurve::Linear),
    ]);
    assert_eq!(
        sample_ik(&keys, setup(), f32::from_bits(0.25_f32.to_bits() - 1)).unwrap(),
        IkPose::from(setup())
    );
    assert_eq!(sample_ik(&keys, setup(), 0.25).unwrap().mix, 1.);
    assert_eq!(
        sample_ik(&keys, setup(), 0.5).unwrap().mix,
        0.8333333333333334
    );
    assert!(
        sample_ik(&keys, setup(), f32::from_bits(1_f32.to_bits() - 1))
            .unwrap()
            .bend_positive
    );
    assert!(!sample_ik(&keys, setup(), 1.).unwrap().bend_positive);
    assert_eq!(sample_ik(&keys, setup(), 2.).unwrap().mix, 0.5);
    keys.keys[0].curve = IkCurve::Stepped;
    assert_eq!(sample_ik(&keys, setup(), 0.9).unwrap().mix, 1.);
    assert_eq!(
        sample_ik(&track(vec![]), setup(), 0.5).unwrap(),
        setup().into()
    );
}

#[test]
fn absolute_mix_controls_preserve_equal_endpoint_excursions_and_unclamped_samples() {
    let mut keys = track(vec![
        frame(
            0.,
            0.5,
            true,
            IkCurve::Bezier {
                time: [0.25, 0.75],
                mix: [0.9, 0.9],
            },
        ),
        frame(1., 0.5, true, IkCurve::Linear),
    ]);
    assert_eq!(
        sample_ik(&keys, setup(), 0.5).unwrap().mix,
        0.7999999523162842
    );
    keys.keys[0].mix = 0.;
    keys.keys[1].mix = 1.;
    keys.keys[0].curve = IkCurve::Bezier {
        time: [0.25, 0.75],
        mix: [2., 2.],
    };
    assert_eq!(sample_ik(&keys, setup(), 0.5).unwrap().mix, 1.625);
    keys.keys[0].curve = IkCurve::Bezier {
        time: [0.25, 0.75],
        mix: [-1., -1.],
    };
    assert_eq!(sample_ik(&keys, setup(), 0.5).unwrap().mix, -0.625);
}

#[test]
fn independently_observed_large_cancellation_is_retained_instead_of_idealized_zero() {
    let m = 2e38_f32;
    let mut keys = track(vec![
        frame(
            0.,
            -m,
            true,
            IkCurve::Bezier {
                time: [0.25, 0.75],
                mix: [-m, m],
            },
        ),
        frame(1., m, true, IkCurve::Linear),
    ]);
    let before = keys.clone();
    assert_eq!(
        sample_ik(&keys, setup(), 0.5).unwrap().mix,
        -7.999997664749159e28
    );
    assert_eq!(keys, before);
    keys.keys[0].mix = 0.5;
    keys.keys[1].mix = 0.5;
    assert_eq!(
        sample_ik(&keys, setup(), 0.5).unwrap().mix,
        -1.1999996969360386e29
    );
    keys.keys[0].mix = -m;
    keys.keys[1].mix = m;
    keys.keys[0].curve = IkCurve::Linear;
    assert_eq!(sample_ik(&keys, setup(), 0.5).unwrap().mix, 0.);
}

#[test]
fn absolute_time_point_quantization_and_wide_ratio_serde_match_original_inputs() {
    let keys = track(vec![
        frame(
            100000.,
            0.5,
            true,
            IkCurve::Bezier {
                time: [0.25, 0.75],
                mix: [0.9, 0.9],
            },
        ),
        frame(100000.125, 0.5, true, IkCurve::Linear),
    ]);
    assert_eq!(
        sample_ik(&keys, setup(), (100000_f64 + 0.015625) as f32)
            .unwrap()
            .mix,
        0.6499999761581421
    );
    assert_eq!(
        sample_ik(&keys, setup(), (100000_f64 + 0.03125) as f32)
            .unwrap()
            .mix,
        0.7519999742507935
    );
    let curve = IkCurve::Bezier {
        time: [
            (f64::from(0.4_f32) - 0.25) / 0.75,
            (f64::from(0.775_f32) - 0.25) / 0.75,
        ],
        mix: [-1., 1.],
    };
    let bytes = serde_json::to_vec(&curve).unwrap();
    let recovered: IkCurve = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(recovered, curve);
    let keys = track(vec![
        frame(0.25, -1., true, recovered),
        frame(1., 1., true, IkCurve::Linear),
    ]);
    assert_eq!(
        sample_ik(&keys, setup(), 0.625).unwrap().mix,
        0.09779735142944698
    );
}

#[test]
fn complete_authored_capsule_carries_ik_only_animation_and_binding_solves_it_once() {
    let mut session = Session::new(rig()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    let zero = session
        .authored_pose(EditTarget::Animate {
            animation: 0,
            time: 0.,
        })
        .unwrap();
    let half = session.authored_pose(target).unwrap();
    assert_eq!(zero.bone_locals(), half.bone_locals());
    assert_eq!(half.ik_poses()[0].mix, 0.5);
    assert_eq!(session.document().animations[0].duration(), 1.);
    let solved = session.evaluated_pose(target).unwrap();
    assert_eq!(solved.authored_locals()[1].rotation, 0.);
    assert!((solved.applied_locals()[1].rotation - 45.).abs() < 0.00001);
    let bind = session.mesh_bind_pose(target).unwrap();
    for (snapshot, solved) in world_pose(session.document(), &bind.locals)
        .iter()
        .zip(solved.world())
    {
        assert_eq!(
            [
                snapshot.a, snapshot.b, snapshot.c, snapshot.d, snapshot.x, snapshot.y
            ],
            [solved.a, solved.b, solved.c, solved.d, solved.x, solved.y]
        );
    }
    session
        .edit_pose(
            2,
            target,
            Transform {
                x: 10.,
                y: 10.,
                ..Transform::default()
            },
            &[Property::X],
            false,
        )
        .unwrap();
    let pending = session.evaluated_pose(target).unwrap();
    assert!((pending.applied_locals()[1].rotation - 22.5).abs() < 0.00001);
    assert_eq!(pending.ik_poses()[0].mix, 0.5);
    assert!(!session.is_dirty());
}

#[test]
fn native7_validation_covers_group_identity_required_fields_and_finite_unbounded_mix() {
    let mut document = rig();
    let Constraint::Ik(ik) = &mut document.constraints[0];
    ik.setup.mix = -1.5;
    document.animations[0].ik_tracks[0].keys[0].mix = 2.;
    document.validate().unwrap();
    let mut value = serde_json::to_value(&document).unwrap();
    value["animations"][0]
        .as_object_mut()
        .unwrap()
        .remove("ik_tracks");
    assert!(serde_json::from_value::<Document>(value).is_err());
    let bytes = serde_json::to_vec(&document).unwrap();
    assert_eq!(
        serde_json::from_slice::<Document>(&bytes).unwrap(),
        document
    );
    let duplicate = document.animations[0].ik_tracks[0].clone();
    document.animations[0].ik_tracks.push(duplicate);
    assert!(document.validate().is_err());
    for invalid in [
        r#"{"type":"linear","softness":0}"#,
        r#"{"type":"bezier","time":[0,1,1],"mix":[0,1]}"#,
        r#"{"type":"bezier","time":[0,1],"mix":[0,1],"unknown":0}"#,
    ] {
        assert!(serde_json::from_str::<IkCurve>(invalid).is_err());
    }
}
