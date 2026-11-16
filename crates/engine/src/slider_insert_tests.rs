//! Actual 4.3.26 Slider Time insertion witness, independent of raw setters.
use crate::*;
use peachy_document::*;

fn fixture() -> Document {
    let mut doc = Document::new("Slider scalar insertion".into(), 60).unwrap();
    doc.animations.push(Animation::new("reference", vec![]));
    doc.animations.push(Animation::new("host", vec![]));
    doc.constraints
        .push(Constraint::Slider(SliderConstraint::new("slider", 0)));
    doc.animations[1].slider_tracks.push(SliderTrack {
        constraint: 0,
        property: SliderProperty::Time,
        keys: vec![
            Keyframe {
                time: 0.,
                value: 0.5,
                curve: Curve::Bezier([1. / 3., 0., 2. / 3., 1.]),
            },
            Keyframe {
                time: 0.25,
                value: 0.75,
                curve: Curve::Linear,
            },
        ],
    });
    doc
}

fn controls(left: Keyframe, right: Keyframe) -> [f32; 4] {
    let (handles, offset) = match left.curve {
        Curve::Bezier(h) => (h, false),
        Curve::BezierOffset(h) => (h, true),
        _ => panic!("Native insertion retains Bezier on both sides"),
    };
    let [x1, y1, x2, y2] = handles;
    let duration = right.time - left.time;
    let span = right.value - left.value;
    [
        left.time + duration * x1,
        left.value + if offset { y1 } else { span * y1 },
        left.time + duration * x2,
        left.value + if offset { y2 } else { span * y2 },
    ]
}

#[test]
fn slider_insert_splits_the_measured_native_time_curve_in_seconds() {
    let time = 7f32 / 60.;
    let mut session = Session::new(fixture()).unwrap();
    session
        .dispatch(Command::InsertSliderKey {
            animation: 1,
            constraint: 0,
            property: SliderProperty::Time,
            time,
            value: 0.6126666,
        })
        .unwrap();
    let keys = &session.document().animations[1].slider_tracks[0].keys;
    assert_eq!(keys.len(), 3);
    assert_eq!(keys[1].time, time);
    assert_eq!(keys[1].value, 0.6126666);
    assert_eq!(keys[2].time, 0.25);
    assert_eq!(keys[2].value, 0.75);
    for (actual, expected) in [
        (
            controls(keys[0], keys[1]),
            [0.03888889, 0.5, 0.07777777, 0.5545925],
        ),
        (
            controls(keys[1], keys[2]),
            [0.1611111, 0.679037, 0.20555554, 0.75],
        ),
    ] {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 0.000002,
                "{actual} != {expected}"
            );
        }
    }
    let saved = session.document().clone();
    session.undo();
    assert_eq!(session.document(), &fixture());
    session.redo();
    assert_eq!(session.document(), &saved);
}

#[test]
fn slider_insert_session_and_raw_setter_have_explicit_distinct_curve_policies() {
    let doc = fixture();
    let time = 7f32 / 60.;
    let mut session = Session::new(doc.clone()).unwrap();
    let before = session
        .slider_pose(0, EditTarget::Animate { animation: 1, time })
        .unwrap();
    session
        .key_slider_pose(1, time, 0, &[SliderProperty::Time])
        .unwrap();
    assert_eq!(
        session
            .slider_pose(0, EditTarget::Animate { animation: 1, time })
            .unwrap(),
        before
    );
    assert_ne!(
        session.document().animations[1].slider_tracks[0].keys[0].curve,
        doc.animations[1].slider_tracks[0].keys[0].curve
    );
    let raw = apply_command(
        &doc,
        Command::SetSliderKey {
            animation: 1,
            constraint: 0,
            property: SliderProperty::Time,
            key: Keyframe {
                time,
                value: 0.6126666,
                curve: Curve::Linear,
            },
        },
    )
    .unwrap();
    assert_eq!(
        raw.animations[1].slider_tracks[0].keys[0],
        doc.animations[1].slider_tracks[0].keys[0]
    );
    let existing = session.document().animations[1].slider_tracks[0].keys[1];
    session
        .dispatch(Command::InsertSliderKey {
            animation: 1,
            constraint: 0,
            property: SliderProperty::Time,
            time,
            value: 0.7,
        })
        .unwrap();
    assert_eq!(
        session.document().animations[1].slider_tracks[0].keys[1].curve,
        existing.curve
    );
}

#[test]
fn slider_insert_rejects_driven_frame_and_nonfinite_values_without_partial_history() {
    let mut doc = fixture();
    let Constraint::Slider(slider) = &mut doc.constraints[0] else {
        unreachable!()
    };
    slider.bone = Some(0);
    slider.property = Some(TransformProperty::Rotate);
    let mut session = Session::new(doc.clone()).unwrap();
    assert!(
        session
            .dispatch(Command::Batch {
                commands: vec![
                    Command::RenameBone {
                        bone: 0,
                        name: "rollback".into()
                    },
                    Command::InsertSliderKey {
                        animation: 1,
                        constraint: 0,
                        property: SliderProperty::Time,
                        time: 0.1,
                        value: 0.5
                    },
                ]
            })
            .unwrap_err()
            .contains("driven by its source bone")
    );
    assert_eq!(session.document(), &doc);
    assert!(!session.can_undo());
    assert!(
        session
            .dispatch(Command::InsertSliderKey {
                animation: 1,
                constraint: 0,
                property: SliderProperty::Mix,
                time: 0.1,
                value: f32::NAN,
            })
            .is_err()
    );
    assert_eq!(session.document(), &doc);
    assert!(!session.can_undo());
}

#[test]
fn slider_insert_native_holdouts_cover_frame_eight_asymmetric_and_nonzero_mix_origin() {
    let observations = [
        (
            SliderProperty::Time,
            [0., 8. / 60., 0.25],
            [0.5, 0.6373334, 0.75],
            Curve::Bezier([1. / 3., 0., 2. / 3., 1.]),
            [0.04444444, 0.5, 0.0888889, 0.570963],
            [0.17222223, 0.6954075, 0.2111111, 0.75],
        ),
        (
            SliderProperty::Time,
            [0., 7. / 60., 0.25],
            [0.5, 0.6352588, 0.75],
            Curve::Bezier([0.06002825 / 0.25, (0.5259644 - 0.5) / 0.25, 2. / 3., 1.]),
            [0.03021422, 0.5130688, 0.07223687, 0.5756552],
            [0.16147336, 0.6940729, 0.2086111, 0.75],
        ),
        (
            SliderProperty::Mix,
            [0.25, 22. / 60., 0.5],
            [0.25, 0.588, 1.],
            Curve::Bezier([1. / 3., 0., 2. / 3., 1.]),
            [0.2891667, 0.25, 0.32833335, 0.41238445],
            [0.4116667, 0.7860344, 0.45583335, 1.],
        ),
    ];
    for (property, [start, time, end], [left, value, right], curve, before, after) in observations {
        let mut doc = fixture();
        doc.animations[1].slider_tracks = vec![SliderTrack {
            constraint: 0,
            property,
            keys: vec![
                Keyframe {
                    time: start,
                    value: left,
                    curve,
                },
                Keyframe {
                    time: end,
                    value: right,
                    curve: Curve::Linear,
                },
            ],
        }];
        let changed = apply_command(
            &doc,
            Command::InsertSliderKey {
                animation: 1,
                constraint: 0,
                property,
                time,
                value,
            },
        )
        .unwrap();
        let keys = &changed.animations[1].slider_tracks[0].keys;
        for (actual, expected) in [
            (controls(keys[0], keys[1]), before),
            (controls(keys[1], keys[2]), after),
        ] {
            for (actual, expected) in actual.into_iter().zip(expected) {
                assert!(
                    (actual - expected).abs() < 0.000002,
                    "{property:?} {time}: {actual} != {expected}"
                );
            }
        }
    }
}
