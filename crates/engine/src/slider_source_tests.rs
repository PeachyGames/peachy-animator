//! Original black-box source and ordered-writer observations, public 4.3.13.
//! Expected values are retained observations, not regenerated mapping formulas.
use super::*;
use crate::{SliderPose, checked_pose_from_authored, sample_authored_pose};
use peachy_document::*;

fn key(time: f32, value: f32) -> Keyframe {
    Keyframe {
        time,
        value,
        curve: Curve::Linear,
    }
}

fn document() -> Document {
    let mut doc = Document::new("Original Slider source lab".into(), 60).unwrap();
    for (name, parent, length, setup) in [
        (
            "source_parent",
            0,
            0.,
            Transform {
                x: 7.,
                y: -8.,
                rotation: 35.,
                scale_x: -1.2,
                scale_y: 0.8,
                shear_x: 9.,
                shear_y: -6.,
            },
        ),
        (
            "driver",
            1,
            60.,
            Transform {
                x: 20.,
                y: -15.,
                rotation: 30.,
                scale_x: 1.25,
                scale_y: -0.75,
                shear_x: 12.,
                shear_y: 18.,
            },
        ),
        (
            "follower_parent",
            0,
            0.,
            Transform {
                y: -100.,
                ..Transform::default()
            },
        ),
        (
            "follower",
            3,
            120.,
            Transform {
                rotation: 30.,
                ..Transform::default()
            },
        ),
        (
            "tip",
            4,
            20.,
            Transform {
                x: 120.,
                ..Transform::default()
            },
        ),
    ] {
        doc.bones.push(Bone {
            name: name.into(),
            parent: Some(parent),
            length,
            setup,
            inherit: Inherit::Normal,
        });
    }
    doc.animations = vec![
        Animation::new(
            "reference",
            vec![Track {
                bone: 4,
                property: Property::X,
                keys: vec![key(0., 0.), key(1., 1000.)],
            }],
        ),
        Animation::new("host", vec![]),
    ];
    let mut slider = SliderConstraint::new("calibration", 0);
    slider.bone = Some(2);
    slider.property = Some(TransformProperty::Rotate);
    slider.from = -100.;
    slider.scale = 0.005;
    slider.max = 1.;
    slider.setup.time = 0.375;
    doc.constraints.push(Constraint::Slider(slider));
    doc.validate().unwrap();
    doc
}

fn slider(doc: &mut Document) -> &mut SliderConstraint {
    doc.constraints
        .iter_mut()
        .find_map(|constraint| match constraint {
            Constraint::Slider(slider) => Some(slider),
            _ => None,
        })
        .unwrap()
}

fn evaluate(doc: &Document) -> crate::EvaluatedPose {
    let authored = sample_authored_pose(doc, Some(1), 0.).unwrap();
    let before = authored.clone();
    let evaluated = checked_pose_from_authored(doc, &authored).unwrap();
    assert_eq!(authored, before);
    evaluated
}

#[test]
fn slider_source_loop_refuses_observed_decimal_precision_discontinuities() {
    let mut config = SliderConstraint::new("precision", 0);
    config.local = true;
    config.looped = true;
    config.scale = 0.01;
    config.to = -0.25;
    for source in [25., 125., 225.] {
        assert!(
            derived_time(&config, source, 1.)
                .unwrap_err()
                .contains("precision boundary")
        );
    }
    assert!((derived_time(&config, 26., 1.).unwrap() - 1.01).abs() < 1e-7);
    config.scale = 0.5;
    config.to = 0.;
    assert_eq!(derived_time(&config, 2., 1.).unwrap(), 1.);
}

#[test]
fn slider_source_six_properties_and_five_inheritance_modes_match_observed_times() {
    let local = [0.65, 0.6, 0.425, 0.50625, 0.49625, 0.59];
    let cases = [
        (
            Inherit::Normal,
            [
                1.4476586828354374,
                0.47776780781024414,
                0.3241638130297764,
                0.5057101411590295,
                0.5043229490807218,
                0.35558980239696764,
            ],
        ),
        (
            Inherit::OnlyTranslation,
            [
                0.71,
                0.47776780781024414,
                0.3241638130297764,
                0.50625,
                0.50375,
                0.,
            ],
        ),
        (
            Inherit::NoRotationOrReflection,
            [
                0.713558964273553,
                0.47776780781024414,
                0.3241638130297764,
                0.5074973260056713,
                0.5029077122194225,
                0.,
            ],
        ),
        (
            Inherit::NoScale,
            [
                1.447816492515051,
                0.47776780781024414,
                0.3241638130297764,
                0.50625,
                0.50375,
                0.46999998670445536,
            ],
        ),
        (
            Inherit::NoScaleOrReflection,
            [
                1.5678164925150508,
                0.47776780781024414,
                0.3241638130297764,
                0.50625,
                0.50375,
                1.4299999867044553,
            ],
        ),
    ];
    for (inherit, world) in cases {
        for (local_mode, expected) in [(false, world), (true, local)] {
            for (property, expected) in TransformProperty::ALL.into_iter().zip(expected) {
                let mut doc = document();
                doc.bones[2].inherit = inherit;
                let source = slider(&mut doc);
                source.property = Some(property);
                source.local = local_mode;
                let pose = evaluate(&doc);
                let actual = pose.applied_slider_poses()[0].time;
                assert!(
                    (actual - expected).abs() < 0.0000001,
                    "{inherit:?} {property:?} local={local_mode}: {actual} != {expected}"
                );
                assert_eq!(pose.authored_slider_poses()[0].time, 0.);
            }
        }
    }
}

#[test]
fn slider_source_reads_current_local_winding_and_keeps_range_metadata_out_of_clamping() {
    let expected = [0.01, 0.442, 0.349, 0.39825, 0.398, 0.277];
    for (property, expected) in TransformProperty::ALL.into_iter().zip(expected) {
        let mut doc = document();
        let source = slider(&mut doc);
        source.property = Some(property);
        source.local = true;
        source.from = 0.;
        source.to = 0.4;
        source.scale = 0.001;
        source.max = 0.05;
        let mut authored = sample_authored_pose(&doc, Some(1), 0.).unwrap();
        authored.locals[2] = Transform {
            x: 42.,
            y: -51.,
            rotation: -390.,
            scale_x: -1.75,
            scale_y: -2.,
            shear_x: 47.,
            shear_y: -123.,
        };
        let pose = checked_pose_from_authored(&doc, &authored).unwrap();
        assert!((pose.applied_slider_poses()[0].time - expected).abs() < 0.0000001);
        assert_eq!(pose.authored_locals()[2].rotation, -390.);
    }
}

#[test]
fn slider_source_loop_metadata_differs_from_sampled_time_and_empty_loop_is_refused() {
    for (source_x, no_loop, looped_time, looped_x) in [
        (-1.25, 0., 0.75, 750.),
        (-0.25, 0., 0.75, 750.),
        (0., 0., 1., 0.),
        (0.25, 0.25, 1.25, 250.),
        (1., 1., 1., 0.),
        (2., 2., 1., 0.),
    ] {
        for (looped, expected) in [(false, no_loop), (true, looped_time)] {
            let mut doc = document();
            doc.bones[2].setup.x = source_x;
            let source = slider(&mut doc);
            source.property = Some(TransformProperty::X);
            source.local = true;
            source.from = 0.;
            source.to = 0.;
            source.scale = 1.;
            source.max = 0.;
            source.looped = looped;
            let pose = evaluate(&doc);
            assert_eq!(pose.applied_slider_poses()[0].time, expected);
            assert_eq!(pose.authored_slider_poses()[0].time, 0.);
            if looped {
                assert_eq!(pose.applied_locals()[4].x, looped_x);
            }
        }
    }
    let mut doc = document();
    doc.animations[0].tracks.clear();
    for looped in [false, true] {
        let source = slider(&mut doc);
        source.looped = looped;
        source.setup.mix = 0.;
        assert_eq!(
            evaluate(&doc).applied_slider_poses()[0],
            SliderPose { time: 0., mix: 0. }
        );
    }
    slider(&mut doc).setup.mix = 1.;
    let authored = sample_authored_pose(&doc, Some(1), 0.).unwrap();
    let error = checked_pose_from_authored(&doc, &authored).unwrap_err();
    assert!(
        error.contains("Slider calibration")
            && error.contains("Loop")
            && error.contains("nonempty")
    );
    slider(&mut doc).looped = false;
    assert!(evaluate(&doc).applied_slider_poses()[0].time.is_finite());
}

#[test]
fn slider_source_world_signed_zero_axes_keep_observed_angular_branches() {
    let mut source = SliderConstraint::new("source", 0);
    source.property = Some(TransformProperty::Rotate);
    let frame = Affine {
        a: -0.,
        b: -0.,
        c: -0.,
        d: 0.,
        x: 0.,
        y: 0.,
    };
    let rotation = source_value(&source, Transform::default(), frame).unwrap();
    assert!((rotation - 180.00000265910892).abs() < 0.00000001);
    source.property = Some(TransformProperty::ShearY);
    let frame = Affine {
        a: 1.,
        b: -0.,
        c: 0.,
        d: 0.,
        x: 0.,
        y: 0.,
    };
    let shear = source_value(&source, Transform::default(), frame).unwrap();
    assert!((shear - 89.99999734089108).abs() < 0.00000001);
    source.property = Some(TransformProperty::ScaleY);
    assert_eq!(
        source_value(&source, Transform::default(), frame)
            .unwrap()
            .to_bits(),
        0f64.to_bits()
    );
}

fn ordered_document(ik_writer: bool, writer_before: bool) -> Document {
    let mut doc = document();
    doc.bones[1].setup = Transform {
        y: 120.,
        ..Transform::default()
    };
    doc.bones[2].setup = Transform {
        x: 20.,
        ..Transform::default()
    };
    doc.animations[0].tracks = vec![Track {
        bone: 2,
        property: Property::X,
        keys: vec![key(0., 0.), key(0.5, 80.), key(1., -80.), key(1.5, 0.)],
    }];
    let source = slider(&mut doc);
    source.bone = Some(4);
    source.property = Some(TransformProperty::Rotate);
    source.local = true;
    source.from = 0.;
    source.scale = 0.01;
    let calibration = doc.constraints.pop().unwrap();
    let writer = if ik_writer {
        Constraint::Ik(IkConstraint {
            name: "writer".into(),
            bones: vec![4],
            target: 2,
            setup: IkSetup {
                mix: 1.,
                bend_positive: true,
            },
            stretch: false,
        })
    } else {
        doc.animations.push(Animation::new(
            "writer reference",
            vec![Track {
                bone: 4,
                property: Property::Rotation,
                keys: vec![key(0., 70.)],
            }],
        ));
        Constraint::Slider(SliderConstraint::new("writer", 2))
    };
    doc.constraints = if writer_before {
        vec![writer, calibration]
    } else {
        vec![calibration, writer]
    };
    doc.validate().unwrap();
    doc
}

#[test]
fn slider_source_uses_prior_slider_and_ik_writes_without_rerunning_later_writers() {
    for (ik_writer, before, expected_time, expected_driver_x) in [
        (false, true, 1., -60.),
        (false, false, 0.3, 68.),
        (true, true, 0.8480556852079066, -11.377819266530125),
        (true, false, 0.3, 68.),
    ] {
        let doc = ordered_document(ik_writer, before);
        let pose = evaluate(&doc);
        let index = usize::from(before);
        assert!((pose.applied_slider_poses()[index].time - expected_time).abs() < 0.0000001);
        assert!((f64::from(pose.applied_locals()[2].x) - expected_driver_x).abs() < 0.001);
        if !ik_writer {
            assert_eq!(pose.applied_locals()[4].rotation, 100.);
        }
    }
}
