//! Numeric goldens are original observed outputs, not regenerated blend formulas.
use super::*;
use crate::{SliderPose, checked_pose_from_authored, sample_authored_pose};
use peachy_document::*;

const SETUP: [f32; 7] = [12., -8., 25., 1.2, -0.8, 7., -11.];
const INCOMING: [f32; 7] = [31., -17., -45., -1.4, 0.6, 23., 15.];

fn transform(values: [f32; 7]) -> Transform {
    let mut transform = Transform::default();
    for (property, value) in Property::ALL.into_iter().zip(values) {
        property.write(&mut transform, value);
    }
    transform
}

fn document(additive: bool, mix: f32, time: f32) -> Document {
    let mut doc = Document::new("Original manual Slider".into(), 60).unwrap();
    doc.bones.push(Bone {
        name: "driver".into(),
        parent: Some(0),
        length: 0.,
        setup: transform(SETUP),
        inherit: Inherit::Normal,
    });
    let left = [-18., 22., -35., -0.5, 1.25, -13., 19.];
    let right = [42., -14., 85., 1.75, -0.75, 27., -31.];
    doc.animations.push(Animation::new(
        "reference",
        Property::ALL
            .into_iter()
            .zip(left)
            .zip(right)
            .map(|((property, left), right)| Track {
                bone: 1,
                property,
                keys: vec![
                    Keyframe {
                        time: 0.5,
                        value: left,
                        curve: Curve::Linear,
                    },
                    Keyframe {
                        time: 1.,
                        value: right,
                        curve: Curve::Linear,
                    },
                ],
            })
            .collect(),
    ));
    doc.animations.push(Animation::new("host", vec![]));
    let mut slider = SliderConstraint::new("manual", 0);
    slider.additive = additive;
    slider.setup = SliderSetup { time, mix };
    doc.constraints.push(Constraint::Slider(slider));
    doc.validate().unwrap();
    doc
}

fn evaluate(doc: &Document, incoming: [f32; 7]) -> crate::EvaluatedPose {
    let mut authored = sample_authored_pose(doc, Some(1), 0.).unwrap();
    authored.locals[1] = transform(incoming);
    let before = authored.clone();
    let pose = checked_pose_from_authored(doc, &authored).unwrap();
    assert_eq!(authored, before);
    pose
}

fn assert_values(actual: Transform, expected: [f32; 7]) {
    for (property, expected) in Property::ALL.into_iter().zip(expected) {
        let actual = property.read(actual);
        assert!(
            (actual - expected).abs() <= 0.00001,
            "{property:?}: {actual} != {expected}"
        );
    }
}

#[test]
fn slider_manual_seven_channels_match_observed_signed_mix_and_additive_cases() {
    // bone-time-01's original seven-channel midpoint cases, public 4.3.13.
    let cases = [
        (false, 0., INCOMING),
        (false, 0.5, [27.5, -10.5, 2.5, 1.075, -0.4, 18.5, -1.]),
        (false, 1., [24., -4., 50., 0.75, -0.2, 14., -17.]),
        (false, -0.5, [34.5, -23.5, -92.5, 1.725, -0.8, 27.5, 31.]),
        (false, 1.5, [20.5, 2.5, 97.5, 0.425, 0., 9.5, -33.]),
        (true, 0., INCOMING),
        (true, 0.5, [37., -15., -32.5, -1.625, 0.9, 26.5, 12.]),
        (true, 1., [43., -13., -20., -1.85, 1.2, 30., 9.]),
        (true, -0.5, [25., -19., -57.5, -1.175, 0.3, 19.5, 18.]),
        (true, 1.5, [49., -11., -7.5, -2.075, 1.5, 33.5, 6.]),
    ];
    for (additive, mix, expected) in cases {
        let doc = document(additive, mix, 0.75);
        let pose = evaluate(&doc, INCOMING);
        assert_values(pose.applied_locals()[1], expected);
        assert_eq!(pose.authored_locals()[1], transform(INCOMING));
        assert_eq!(
            pose.applied_slider_poses()[0],
            SliderPose {
                time: 0.75,
                mix: f64::from(mix)
            }
        );
    }
}

#[test]
fn slider_manual_absent_before_first_held_and_loop_boundaries_preserve_incoming_domains() {
    let mut doc = document(false, 1., 0.25);
    let before_first = evaluate(&doc, INCOMING);
    assert_eq!(before_first.applied_locals()[1], transform(INCOMING));
    doc.animations[0]
        .tracks
        .retain(|track| track.property == Property::X);
    let Constraint::Slider(slider) = &mut doc.constraints[0] else {
        unreachable!()
    };
    slider.setup.time = 0.75;
    assert_values(
        evaluate(&doc, INCOMING).applied_locals()[1],
        [24., -17., -45., -1.4, 0.6, 23., 15.],
    );
    doc.bones[1].setup.x = 20.;
    doc.animations[0].tracks[0].keys = vec![
        Keyframe {
            time: 0.,
            value: 0.,
            curve: Curve::Linear,
        },
        Keyframe {
            time: 0.5,
            value: 80.,
            curve: Curve::Linear,
        },
        Keyframe {
            time: 1.,
            value: 20.,
            curve: Curve::Linear,
        },
    ];
    // Signed remainder is observed: negative nonmultiples remain before-first;
    // exact -duration hits zero. The raw effective Slider time is not wrapped.
    for (looped, time, expected) in [
        (false, -1., 31.),
        (false, 1., 40.),
        (false, 1.25, 40.),
        (true, -1., 20.),
        (true, -0.25, 31.),
        (true, -1.000001, 31.),
        (true, 1., 20.),
        (true, 1.25, 60.),
        (true, 2., 20.),
    ] {
        let Constraint::Slider(slider) = &mut doc.constraints[0] else {
            unreachable!()
        };
        slider.looped = looped;
        slider.setup.time = time;
        let pose = evaluate(&doc, INCOMING);
        assert_eq!(pose.applied_locals()[1].x, expected);
        assert_eq!(pose.applied_slider_poses()[0].time, f64::from(time));
    }
    doc.animations[0].tracks.clear();
    assert_eq!(
        evaluate(&doc, INCOMING).applied_locals()[1],
        transform(INCOMING)
    );
}

#[test]
fn slider_manual_winding_full_mix_and_zero_signs_follow_independent_holdouts() {
    for (offset, incoming, mix, additive, expected) in [
        (180., -180., 0.5, false, 12.5),
        (180., -180., 1., false, 205.),
        (-180., 180., 0.5, false, 12.5),
        (-180., 180., 1., false, -155.),
        (720., 30., 0.5, false, 387.5),
        (720., 30., 0.5, true, 390.),
    ] {
        assert_eq!(
            blend(Property::Rotation, incoming, 25., offset, mix, additive).unwrap(),
            expected
        );
    }
    // manual-bone-holdout-report: full Mix keeps scalar cancellation while
    // scales use direct targets; zero signs disambiguate interpolation order.
    assert_eq!(blend(Property::X, 1e30, 12., 12., 1., false).unwrap(), 0.);
    assert_eq!(
        blend(Property::ScaleX, -1e30, 1.2, 0.625, 1., false).unwrap(),
        0.75
    );
    assert_eq!(
        blend(Property::ScaleY, 1e30, -0.8, 0.25, 1., false).unwrap(),
        -0.2
    );
    for (mix, negative_y) in [(-0.5, true), (1.5, false)] {
        let x = blend(Property::ScaleX, -1.4, 1.2, 0., mix, false).unwrap();
        let y = blend(Property::ScaleY, 0.6, -0.8, 0., mix, false).unwrap();
        assert_eq!(x.to_bits(), 0f32.to_bits());
        assert_eq!(
            y.to_bits(),
            if negative_y {
                (-0f32).to_bits()
            } else {
                0f32.to_bits()
            }
        );
    }
    assert_eq!(
        blend(Property::ScaleX, -0., 1.2, -1.5, 0., false)
            .unwrap()
            .to_bits(),
        (-0f32).to_bits()
    );
    assert!((blend(Property::ScaleX, 0., 1.2, -1.5, -0.5, false).unwrap() - 0.9).abs() < 0.000001);
    assert!((blend(Property::ScaleY, 0., -0.8, 0.5, 0.25, false).unwrap() + 0.1).abs() < 0.000001);
}

#[test]
fn slider_manual_rebuilds_descendant_fk_once_without_mutating_authored_pose() {
    let mut doc = document(false, 1., 0.75);
    doc.bones.push(Bone {
        name: "tip".into(),
        parent: Some(1),
        length: 3.,
        setup: Transform {
            x: 5.,
            ..Transform::default()
        },
        inherit: Inherit::Normal,
    });
    let pose = evaluate(&doc, INCOMING);
    assert_values(
        pose.applied_locals()[1],
        [24., -4., 50., 0.75, -0.2, 14., -17.],
    );
    let frame = world_pose_with(&doc, pose.applied_locals(), pose.inherits());
    assert_eq!(
        serde_json::to_value(pose.world()).unwrap(),
        serde_json::to_value(frame).unwrap()
    );
    assert_ne!(pose.world()[2].x, 5.);
    assert_eq!(pose.authored_locals()[1], transform(INCOMING));
}

#[test]
fn slider_nested_time_preserves_an_interval_between_adjacent_f32_keys() {
    let left = 1f32;
    let right = f32::from_bits(left.to_bits() + 1);
    let midpoint = (f64::from(left) + f64::from(right)) * 0.5;
    let mut doc = document(false, 1., left);
    doc.animations[0].tracks = vec![Track {
        bone: 1,
        property: Property::X,
        keys: vec![
            Keyframe {
                time: left,
                value: 0.,
                curve: Curve::Linear,
            },
            Keyframe {
                time: right,
                value: 100.,
                curve: Curve::Linear,
            },
        ],
    }];
    doc.animations[1] = Animation::new("control time", vec![]);
    doc.animations[1].slider_tracks = vec![SliderTrack {
        constraint: 1,
        property: SliderProperty::Time,
        keys: vec![Keyframe {
            time: 0.,
            value: right,
            curve: Curve::Linear,
        }],
    }];
    doc.animations.push(Animation::new("empty host", vec![]));
    let driven = doc.constraints.pop().unwrap();
    let mut controller = SliderConstraint::new("controller", 1);
    controller.setup.mix = 0.5;
    doc.constraints = vec![Constraint::Slider(controller), driven];
    doc.validate().unwrap();

    let authored = sample_authored_pose(&doc, Some(2), 0.).unwrap();
    let pose = checked_pose_from_authored(&doc, &authored).unwrap();
    assert_eq!(pose.applied_slider_poses()[1].time, midpoint);
    assert_eq!(pose.authored_slider_poses()[1].time, f64::from(left));
    assert_eq!(pose.applied_locals()[1].x, 62.);
    assert_eq!(pose.authored_locals()[1].x, SETUP[0]);
    // The existing f32 query deliberately keeps its original identity policy;
    // only the reference-time path can represent this effective midpoint.
    assert_eq!(
        crate::sample(&doc.animations[0].tracks[0], midpoint as f32),
        0.
    );
}

#[test]
fn slider_inactive_bone_tracks_preserve_incoming_values_while_active_targets_apply() {
    for mix in [-0.5, 0.5, 1.5] {
        let mut doc = document(false, mix, 0.75);
        doc.skin_scope.required_bones.insert(1);
        doc.bones.push(Bone {
            name: "active target".into(),
            parent: Some(0),
            length: 0.,
            setup: Transform {
                x: 4.,
                ..Transform::default()
            },
            inherit: Inherit::Normal,
        });
        doc.animations[0].tracks.push(Track {
            bone: 2,
            property: Property::X,
            keys: vec![Keyframe {
                time: 0.,
                value: 8.,
                curve: Curve::Linear,
            }],
        });
        doc.validate().unwrap();
        let pose = evaluate(&doc, INCOMING);
        assert_eq!(pose.applied_locals()[1], transform(INCOMING));
        assert_eq!(pose.applied_locals()[2].x, 4. + 8. * mix);
        assert_eq!(pose.authored_locals()[2].x, 4.);
    }
}

#[test]
fn slider_manual_uncalibrated_domains_and_nonfinite_results_fail_explicitly() {
    let mut doc = document(false, 1., 0.75);
    let Constraint::Slider(slider) = &mut doc.constraints[0] else {
        unreachable!()
    };
    slider.setup.mix = 0.;
    doc.animations[0].physics.push(PhysicsTrack {
        constraint: None,
        property: PhysicsProperty::Mix,
        keys: vec![Keyframe {
            time: 0.,
            value: 0.5,
            curve: Curve::Linear,
        }],
    });
    let authored = sample_authored_pose(&doc, Some(1), 0.).unwrap();
    assert!(
        checked_pose_from_authored(&doc, &authored)
            .unwrap_err()
            .contains("physics")
    );
    doc.animations[0].physics.clear();
    let Constraint::Slider(slider) = &mut doc.constraints[0] else {
        unreachable!()
    };
    slider.setup.mix = 1.;
    doc.bones[1].setup.scale_x = f32::MAX;
    doc.animations[0]
        .tracks
        .retain(|track| track.property == Property::ScaleX);
    doc.animations[0].tracks[0].keys = vec![Keyframe {
        time: 0.,
        value: f32::MAX,
        curve: Curve::Linear,
    }];
    let authored = sample_authored_pose(&doc, Some(1), 0.).unwrap();
    let before = authored.clone();
    assert!(
        checked_pose_from_authored(&doc, &authored)
            .unwrap_err()
            .contains("finite f32")
    );
    assert_eq!(authored, before);
}
