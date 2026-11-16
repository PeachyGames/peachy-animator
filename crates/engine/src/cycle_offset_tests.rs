use crate::*;
use peachy_document::*;

fn keys(times: &[f32], values: &[f32]) -> Vec<Keyframe> {
    times
        .iter()
        .zip(values)
        .map(|(&time, &value)| Keyframe {
            time,
            value,
            curve: Curve::Linear,
        })
        .collect()
}
fn absolute_curve(left: Keyframe, right: Keyframe, handles: [f32; 4]) -> Curve {
    Curve::Bezier([
        (handles[0] - left.time) / (right.time - left.time),
        (handles[1] - left.value) / (right.value - left.value),
        (handles[2] - left.time) / (right.time - left.time),
        (handles[3] - left.value) / (right.value - left.value),
    ])
}
fn fixture(bezier: bool) -> Document {
    let mut doc = Document::new("Original cycle Offset fixture".into(), 60).unwrap();
    for (name, rotation) in [("driver", 0.), ("follower", 30.)] {
        doc.bones.push(Bone {
            name: name.into(),
            parent: Some(0),
            length: 60.,
            setup: Transform {
                rotation,
                ..Transform::default()
            },
            inherit: Inherit::Normal,
        });
    }
    let mut driver = keys(&[0., 0.2, 0.6, 1.], &[8., 32., -16., 8.]);
    let mut follower = keys(&[0., 0.25, 0.65, 1.], &[15., 44., -12., 15.]);
    if bezier {
        driver[2].curve = absolute_curve(driver[2], driver[3], [0.64, -28., 0.9, 52.]);
        follower[2].curve = absolute_curve(follower[2], follower[3], [0.72, -60., 0.97, 35.]);
    }
    doc.animations.push(Animation::new(
        "offset",
        vec![
            Track {
                bone: 1,
                property: Property::Y,
                keys: driver,
            },
            Track {
                bone: 2,
                property: Property::Rotation,
                keys: follower,
            },
            Track {
                bone: 0,
                property: Property::Rotation,
                keys: keys(&[0., 1.], &[3., 9.]),
            },
        ],
    ));
    doc.animations.push(Animation::new(
        "untouched",
        vec![Track {
            bone: 1,
            property: Property::X,
            keys: keys(&[0., 2.], &[4., -8.]),
        }],
    ));
    doc.validate().unwrap();
    doc
}
fn range() -> CycleOffsetRange {
    CycleOffsetRange { start: 0., end: 1. }
}
fn selection(doc: &Document, tracks: &[usize]) -> Vec<AnimationKey> {
    tracks
        .iter()
        .flat_map(|&index| {
            let track = &doc.animations[0].tracks[index];
            track.keys.iter().map(|key| AnimationKey::Bone {
                bone: track.bone,
                property: track.property,
                time: key.time,
            })
        })
        .collect()
}
fn controls(left: Keyframe, right: Keyframe) -> [f32; 4] {
    let (h, offset) = match left.curve {
        Curve::Bezier(h) => (h, false),
        Curve::BezierOffset(h) => (h, true),
        _ => panic!("Expected split Bezier"),
    };
    [
        left.time + (right.time - left.time) * h[0],
        left.value
            + if offset {
                h[1]
            } else {
                (right.value - left.value) * h[1]
            },
        left.time + (right.time - left.time) * h[2],
        left.value
            + if offset {
                h[3]
            } else {
                (right.value - left.value) * h[3]
            },
    ]
}
fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 0.0001,
        "{actual} != {expected}"
    );
}

#[test]
fn cycle_offset_native_closing_beziers_repeated_and_reselected_use_originals() {
    // Installed Spine 4.3.26 original numeric controls: explicit two-row marquee,
    // +12 frames, then +6, deselect/reselect all current keys, then +6 again.
    // Native sampling/subdivision is approximate; this is not exact cubic inversion.
    let doc = fixture(true);
    let mut session = Session::new(doc.clone()).unwrap();
    let receipt = session
        .offset_animation_keys(0, &selection(&doc, &[0, 1]), range(), 0.2)
        .unwrap();
    assert_eq!(receipt.keys.len(), 10);
    assert_eq!(receipt.boundary_keys.len(), 4);
    let first = session.document().clone();
    for (index, value, after, before) in [
        (
            0,
            11.195514,
            [0.07645162, 22.091953, 0.15466665, 27.946667],
            [0.8218667, -22.56, 0.9094792, -1.944312],
        ),
        (
            1,
            -16.49734,
            [0.0890808, -1.1645336, 0.1824, 26.733334],
            [0.8789333, -31.84, 0.9386186, -27.3],
        ),
    ] {
        let keys = &first.animations[0].tracks[index].keys;
        assert_eq!(keys.len(), 5);
        close(keys[0].value, value);
        close(keys[4].value, value);
        for (actual, expected) in controls(keys[0], keys[1]).into_iter().zip(after) {
            close(actual, expected);
        }
        for (actual, expected) in controls(keys[3], keys[4]).into_iter().zip(before) {
            close(actual, expected);
        }
    }
    session
        .offset_animation_keys(0, &selection(&first, &[0, 1]), range(), 0.1)
        .unwrap();
    let second = session.document().clone();
    for (index, value) in [(0, -5.19033), (1, -24.695436)] {
        let ks = &second.animations[0].tracks[index].keys;
        assert_eq!(ks.len(), 5);
        close(ks[0].value, value);
        close(ks[4].value, value);
    }
    let mut reselected = selection(&second, &[1, 0]);
    reselected.reverse();
    session
        .offset_animation_keys(0, &reselected, range(), 0.1)
        .unwrap();
    let third = session.document();
    let driver = &third.animations[0].tracks[0].keys;
    let follower = &third.animations[0].tracks[1].keys;
    assert_eq!(driver.len(), 4);
    assert_eq!(follower.len(), 5);
    close(driver[0].value, -16.);
    close(follower[0].value, -4.999988);
    for (actual, expected) in controls(driver[0], driver[1])
        .into_iter()
        .zip([0.03999996, -28., 0.3, 52.])
    {
        close(actual, expected);
    }
    for (actual, expected) in controls(follower[1], follower[2])
        .into_iter()
        .zip([0.11999995, -60., 0.36999995, 35.])
    {
        close(actual, expected);
    }
    assert_eq!(third.animations[0].tracks[2], doc.animations[0].tracks[2]);
    assert_eq!(third.animations[1], doc.animations[1]);
    for _ in 0..3 {
        session.undo();
    }
    assert_eq!(session.document(), &doc);
    assert!(session.cycle_offset_memory().is_empty());
    for _ in 0..3 {
        session.redo();
    }
    assert_eq!(session.document().animations[0].tracks[0].keys.len(), 4);
}

#[test]
fn cycle_offset_native_single_linear_key_seeds_whole_property_track_once() {
    // Installed Spine 4.3.26: plain-select driver frame12/value32 only,
    // enable Offset, drag +6. Every driver key moves; follower remains exact.
    let mut doc = fixture(false);
    doc.animations[0].tracks[1].keys = keys(&[0., 0.35, 0.8, 1.], &[15., -25., 45., 15.]);
    let selected = AnimationKey::Bone {
        bone: 1,
        property: Property::Y,
        time: 0.2,
    };
    let mut session = Session::new(doc.clone()).unwrap();
    let receipt = session
        .offset_animation_keys(0, std::slice::from_ref(&selected), range(), 0.1)
        .unwrap();
    let expected_times = [0., 0.1, 0.3, 0.7, 1.];
    let expected_values = [1.999998, 8., 32., -16., 1.999998];
    let first = session.document().clone();
    let shifted = &first.animations[0].tracks[0].keys;
    assert_eq!(shifted.len(), 5);
    for ((key, time), value) in shifted.iter().zip(expected_times).zip(expected_values) {
        close(key.time, time);
        close(key.value, value);
        assert_eq!(key.curve, Curve::Linear);
    }
    assert_eq!(receipt.keys.len(), 5);
    assert_eq!(receipt.boundary_keys.len(), 2);
    assert_eq!(receipt.phases.len(), 1);
    assert_eq!(
        session.document().animations[0].tracks[1],
        doc.animations[0].tracks[1]
    );
    assert_eq!(session.document().animations[1], doc.animations[1]);
    // D3 native control: selecting all five current keys of the SAME property
    // then +6 retains its original four-key source, rather than carrying a seam.
    session
        .offset_animation_keys(0, &selection(&first, &[0]), range(), 0.1)
        .unwrap();
    let expanded = &session.document().animations[0].tracks[0].keys;
    assert_eq!(expanded.len(), 5);
    for ((key, time), value) in expanded
        .iter()
        .zip([0., 0.2, 0.4, 0.8, 1.])
        .zip([-4.000004, 8., 32., -16., -4.000004])
    {
        close(key.time, time);
        close(key.value, value);
    }
    assert_eq!(
        session.document().animations[0].tracks[1],
        doc.animations[0].tracks[1]
    );
    session.undo();
    assert_eq!(session.document(), &first);
    session.undo();
    assert_eq!(session.document(), &doc);
    assert!(session.cycle_offset_memory().is_empty());
    // Different exact keys on the same property seed it once, not once per key.
    let more_seeds = [
        selected,
        AnimationKey::Bone {
            bone: 1,
            property: Property::Y,
            time: 0.6,
        },
    ];
    let receipt = session
        .offset_animation_keys(0, &more_seeds, range(), 0.1)
        .unwrap();
    assert_eq!(receipt.phases.len(), 1);
    assert_eq!(session.document(), &first);
}

#[test]
fn cycle_offset_native_seven_steps_expand_shrink_return_and_history_match() {
    // Installed 4.3.26, explicit pointer move before every press. Selection sets:
    // Driver; both; both; both; both; Tree-scoped Driver; both. Earlier canvas
    // "subset" attempts actually moved both and are represented as both here.
    // These original exported numbers are observations, not generated formulas.
    let mut doc = fixture(false);
    doc.animations[0].tracks[1].keys = keys(&[0., 0.35, 0.8, 1.], &[15., -25., 45., 15.]);
    let goldens = [
        [
            keys(
                &[0., 0.1, 0.3, 0.7, 1.],
                &[1.999998, 8., 32., -16., 1.999998],
            ),
            keys(&[0., 0.35, 0.8, 1.], &[15., -25., 45., 15.]),
        ],
        [
            keys(
                &[0., 0.1, 0.2, 0.4, 0.8, 1.],
                &[-4.000004, 1.999998, 8., 32., -16., -4.000004],
            ),
            keys(
                &[0., 0.1, 0.45, 0.9, 1.],
                &[30.000004, 15., -25., 45., 30.000004],
            ),
        ],
        [
            keys(
                &[0., 0.2, 0.3, 0.5, 0.9, 1.],
                &[-10.000004, 1.999998, 8., 32., -16., -10.000004],
            ),
            keys(&[0., 0.2, 0.55, 1.], &[45., 15., -25., 45.]),
        ],
        [
            keys(&[0., 0.3, 0.4, 0.6, 1.], &[-16., 1.999998, 8., 32., -16.]),
            keys(
                &[0., 0.1, 0.3, 0.65, 1.],
                &[29.44444, 45., 15., -25., 29.44444],
            ),
        ],
        [
            keys(
                &[0., 0.1, 0.4, 0.5, 0.7, 1.],
                &[-3.999996, -16., 1.999998, 8., 32., -3.999996],
            ),
            keys(
                &[0., 0.2, 0.4, 0.75, 1.],
                &[13.888882, 45., 15., -25., 13.888882],
            ),
        ],
        [
            keys(
                &[0., 0.2, 0.5, 0.6, 0.8, 1.],
                &[8.000008, -16., 1.999998, 8., 32., 8.000008],
            ),
            keys(
                &[0., 0.2, 0.4, 0.75, 1.],
                &[13.888882, 45., 15., -25., 13.888882],
            ),
        ],
        [
            keys(
                &[0., 0.1, 0.3, 0.6, 0.7, 0.9, 1.],
                &[20.00001, 8.000008, -16., 1.999998, 8., 32., 20.00001],
            ),
            keys(
                &[0., 0.1, 0.3, 0.5, 0.85, 1.],
                &[-1.6666756, 13.888882, 45., 15., -25., -1.6666756],
            ),
        ],
    ];
    let cohorts: [&[usize]; 7] = [&[0], &[0, 1], &[0, 1], &[0, 1], &[0, 1], &[0], &[0, 1]];
    let phases = [0.1, 0.1, 0.2, 0.3, 0.4, 0.5, 0.1];
    let mut session = Session::new(doc.clone()).unwrap();
    let mut snapshots = vec![(doc.clone(), session.cycle_offset_memory().clone())];
    for (step, expected) in goldens.iter().enumerate() {
        let before = session.document().clone();
        let selected = if step == 0 {
            vec![AnimationKey::Bone {
                bone: 1,
                property: Property::Y,
                time: 0.2,
            }]
        } else {
            selection(&before, cohorts[step])
        };
        let receipt = session
            .offset_animation_keys(0, &selected, range(), 0.1)
            .unwrap();
        assert_eq!(receipt.boundary_keys.len(), cohorts[step].len() * 2);
        assert!(
            receipt
                .phases
                .iter()
                .all(|phase| phase.phase == phases[step])
        );
        for (index, reference) in expected.iter().enumerate() {
            let actual = &session.document().animations[0].tracks[index].keys;
            assert_eq!(
                actual.len(),
                reference.len(),
                "Native step {step}, track {index}"
            );
            for (key, reference) in actual.iter().zip(reference) {
                close(key.time, reference.time);
                close(key.value, reference.value);
                assert_eq!(key.curve, Curve::Linear);
            }
        }
        if step == 5 {
            assert_eq!(
                session.document().animations[0].tracks[1],
                before.animations[0].tracks[1]
            );
        }
        let mut untouched = session.document().clone();
        untouched.animations[0] = doc.animations[0].clone();
        assert_eq!(untouched, doc);
        snapshots.push((
            session.document().clone(),
            session.cycle_offset_memory().clone(),
        ));
    }
    for expected in snapshots[..7].iter().rev() {
        session.undo();
        assert_eq!(session.document(), &expected.0);
        assert_eq!(session.cycle_offset_memory(), &expected.1);
    }
    for expected in snapshots.iter().skip(1) {
        session.redo();
        assert_eq!(session.document(), &expected.0);
        assert_eq!(session.cycle_offset_memory(), &expected.1);
    }
}

#[test]
fn cycle_offset_context_order_transition_staleness_and_history_are_atomic() {
    // Canonical context and stale guards supplement the native seven-step
    // expansion/shrink/return control below, without claiming arbitrary edits.
    let doc = fixture(false);
    let mut session = Session::new(doc.clone()).unwrap();
    session
        .offset_animation_keys(0, &selection(&doc, &[0, 1]), range(), 0.1)
        .unwrap();
    let first = session.document().clone();
    let remembered = session.cycle_offset_memory().clone();
    let mut reversed = selection(&first, &[1, 0]);
    reversed.reverse();
    let receipt = session
        .offset_animation_keys(0, &reversed, range(), 0.1)
        .unwrap();
    assert!(receipt.phases.iter().all(|phase| phase.phase == 0.2));
    let second = session.document().clone();
    let fresh = CycleOffsetPlan::capture(&first, 0, &selection(&first, &[0, 1]), range())
        .unwrap()
        .advance(&first, 0.1)
        .unwrap();
    assert_ne!(
        second, fresh.document,
        "Same cohort must not rebase its generated seam"
    );
    session.undo();
    assert_eq!(session.document(), &first);
    assert_eq!(session.cycle_offset_memory(), &remembered);
    // Leaving a remembered property cannot hide an intervening stale edit on it.
    let mut key = first.animations[0].tracks[1].keys[1];
    key.value += 1.;
    session
        .dispatch(Command::SetKey {
            animation: 0,
            bone: 2,
            property: Property::Rotation,
            key,
        })
        .unwrap();
    let stale = session.document().clone();
    let memory = session.cycle_offset_memory().clone();
    assert!(
        session
            .offset_animation_keys(0, &selection(&stale, &[0]), range(), 0.1)
            .unwrap_err()
            .contains("Stale")
    );
    assert_eq!(session.document(), &stale);
    assert_eq!(session.cycle_offset_memory(), &memory);
    session.undo();
    // Shrinking retains the surviving original and phase; its old seam does
    // not become an extra interior source key.
    let shifted_original = CycleOffsetPlan::capture(&doc, 0, &selection(&doc, &[0]), range())
        .unwrap()
        .advance(&doc, 0.2)
        .unwrap();
    let mut expected = first.clone();
    expected.animations[0].tracks[0] = shifted_original.document.animations[0].tracks[0].clone();
    let receipt = session
        .offset_animation_keys(0, &selection(&first, &[0]), range(), 0.1)
        .unwrap();
    assert_eq!(session.document(), &expected);
    assert_eq!(receipt.phases[0].phase, 0.2);
    let subset = session.document().clone();
    let expected = CycleOffsetPlan::capture(&subset, 0, &selection(&subset, &[0, 1]), range())
        .unwrap()
        .advance(&subset, 0.1)
        .unwrap();
    let receipt = session
        .offset_animation_keys(0, &selection(&subset, &[0, 1]), range(), 0.1)
        .unwrap();
    assert_eq!(session.document(), &expected.document);
    assert!(receipt.phases.iter().all(|phase| phase.phase == 0.1));
    let mut noops = Session::new(doc.clone()).unwrap();
    noops
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "redo".into(),
        })
        .unwrap();
    noops.undo();
    for tracks in [&[0][..], &[0, 1][..], &[1][..], &[0, 1][..]] {
        noops
            .offset_animation_keys(0, &selection(&doc, tracks), range(), 0.)
            .unwrap();
        assert_eq!(noops.document(), &doc);
        assert!(!noops.can_undo());
        assert!(noops.can_redo());
        assert!(!noops.is_dirty());
    }
}

#[test]
fn cycle_offset_linear_and_stepped_follow_periodic_sample_math_in_both_directions() {
    let mut doc = fixture(false);
    doc.animations[0].tracks[0].keys[1].curve = Curve::Stepped;
    for delta in [0.1_f32, 0.25, -0.2, 1., 2.25, -3.25] {
        let plan = CycleOffsetPlan::capture(&doc, 0, &selection(&doc, &[0, 1]), range()).unwrap();
        let shifted = plan.advance(&doc, delta).unwrap();
        for index in [0, 1] {
            for step in 0..=200 {
                let time = step as f32 / 200.;
                let phase_time = (time - delta).rem_euclid(1.);
                // A separate mathematical phase comparison. Avoid exact step
                // boundaries here; their native f32 sidedness has its own gate.
                if doc.animations[0].tracks[index]
                    .keys
                    .iter()
                    .any(|key| (key.time - phase_time).abs() < 0.0001)
                {
                    continue;
                }
                close(
                    sample(&shifted.document.animations[0].tracks[index], time),
                    sample(&doc.animations[0].tracks[index], phase_time),
                );
            }
        }
    }
    // A whole-period delta must not swallow an already accepted fraction.
    let first = CycleOffsetPlan::capture(&doc, 0, &selection(&doc, &[0]), range())
        .unwrap()
        .advance(&doc, 0.25)
        .unwrap();
    let plan = CycleOffsetPlan::capture_with_memory(
        &first.document,
        0,
        &selection(&first.document, &[0]),
        range(),
        &first.memory,
    )
    .unwrap();
    let huge_whole_period = plan.advance(&first.document, 1e20).unwrap();
    assert_eq!(huge_whole_period.document, first.document);
    assert_eq!(huge_whole_period.phases[0].phase, 0.25);
}

#[test]
fn cycle_offset_finite_nonzero_range_and_flat_offset_curves_keep_explicit_scope() {
    // Mathematical range support is separate from pending native inner-range /
    // outside-key interaction calibration. This track exactly fills its range.
    let mut doc = fixture(false);
    for key in &mut doc.animations[0].tracks[0].keys {
        key.time += 2.;
    }
    let r = CycleOffsetRange { start: 2., end: 3. };
    let out = CycleOffsetPlan::capture(&doc, 0, &selection(&doc, &[0]), r)
        .unwrap()
        .advance(&doc, 0.25)
        .unwrap();
    assert_eq!(
        out.document.animations[0].tracks[0]
            .keys
            .first()
            .unwrap()
            .time,
        2.
    );
    assert_eq!(
        out.document.animations[0].tracks[0]
            .keys
            .last()
            .unwrap()
            .time,
        3.
    );
    assert_eq!(
        out.document.animations[0].tracks[1],
        doc.animations[0].tracks[1]
    );
    let mut doc = fixture(false);
    doc.animations[0].tracks[0].keys[1].value = 8.;
    doc.animations[0].tracks[0].keys[0].curve = Curve::BezierOffset([0.1, -20., 0.7, 30.]);
    let out = CycleOffsetPlan::capture(&doc, 0, &selection(&doc, &[0]), range())
        .unwrap()
        .advance(&doc, 0.9)
        .unwrap();
    let ks = &out.document.animations[0].tracks[0].keys;
    assert!(matches!(ks[0].curve, Curve::BezierOffset(_)));
    assert!(
        ks.iter()
            .filter(|key| matches!(key.curve, Curve::BezierOffset(_)))
            .count()
            >= 2
    );
    out.document.validate().unwrap();
}

#[test]
fn cycle_offset_stale_track_context_refuses_and_unrelated_edits_are_retained() {
    let doc = fixture(true);
    let plan = CycleOffsetPlan::capture(&doc, 0, &selection(&doc, &[0]), range()).unwrap();
    let mut unrelated = doc.clone();
    unrelated.animations[0].tracks[2].keys[0].value = 97.;
    let out = plan.advance(&unrelated, 0.2).unwrap();
    assert_eq!(
        out.document.animations[0].tracks[2],
        unrelated.animations[0].tracks[2]
    );
    let mut changed = doc.clone();
    changed.animations[0].tracks[0].keys[1].value += 1.;
    assert!(plan.advance(&changed, 0.2).unwrap_err().contains("Stale"));
    let mut changed = doc.clone();
    changed.fps = 30;
    assert!(plan.advance(&changed, 0.2).unwrap_err().contains("Stale"));
    let mut changed = doc.clone();
    changed.bones[1].name = "replacement identity".into();
    assert!(plan.advance(&changed, 0.2).unwrap_err().contains("Stale"));
    let mut raw = doc.clone();
    raw.animations[0].tracks[0].keys[0].curve = Curve::BezierOffset([0.1, -0., 0.8, 0.]);
    let raw_plan = CycleOffsetPlan::capture(&raw, 0, &selection(&raw, &[0]), range()).unwrap();
    let mut changed = raw.clone();
    changed.animations[0].tracks[0].keys[0].curve = Curve::BezierOffset([0.1, 0., 0.8, 0.]);
    // Equal numeric curves can still carry distinct imported handle bits.
    assert_eq!(changed, raw);
    assert!(
        raw_plan
            .advance(&changed, 0.)
            .unwrap_err()
            .contains("Stale")
    );
    let mut session = Session::new(doc.clone()).unwrap();
    session
        .offset_animation_keys(0, &selection(&doc, &[0]), range(), 0.2)
        .unwrap();
    let before = session.document().clone();
    session
        .dispatch(Command::SetKey {
            animation: 0,
            bone: 1,
            property: Property::Y,
            key: Keyframe {
                time: 0.4,
                value: 13.,
                curve: Curve::Linear,
            },
        })
        .unwrap();
    let changed = session.document().clone();
    let memory = session.cycle_offset_memory().clone();
    assert!(
        session
            .offset_animation_keys(0, &selection(&changed, &[0]), range(), 0.1)
            .is_err()
    );
    assert_eq!(session.document(), &changed);
    assert_eq!(session.cycle_offset_memory(), &memory);
    session.undo();
    assert_eq!(session.document(), &before);
    session
        .offset_animation_keys(0, &selection(&before, &[0]), range(), 0.1)
        .unwrap();
    session
        .dispatch(Command::SetKey {
            animation: 0,
            bone: 1,
            property: Property::Y,
            key: Keyframe {
                time: 0.4,
                value: 19.,
                curve: Curve::Linear,
            },
        })
        .unwrap();
    session.reset_cycle_offset_memory().unwrap();
    let recaptured = session.document().clone();
    session
        .offset_animation_keys(0, &selection(&recaptured, &[0]), range(), 0.1)
        .unwrap();
}

#[test]
fn cycle_offset_invalid_selection_range_and_collapsed_times_fail_atomically() {
    let doc = fixture(false);
    for mut selected in [vec![], selection(&doc, &[0]), selection(&doc, &[0, 1])] {
        if !selected.is_empty() {
            selected.push(selected[0].clone());
        }
        assert!(CycleOffsetPlan::capture(&doc, 0, &selected, range()).is_err());
    }
    let mut nonexistent = selection(&doc, &[0, 1]);
    let AnimationKey::Bone { time, .. } = nonexistent.last_mut().unwrap() else {
        unreachable!()
    };
    *time = time.next_up();
    assert!(
        CycleOffsetPlan::capture(&doc, 0, &nonexistent, range())
            .unwrap_err()
            .contains("Unknown exact")
    );
    let mut unsupported = selection(&doc, &[0]);
    unsupported.push(AnimationKey::DrawOrder { time: 0. });
    assert!(CycleOffsetPlan::capture(&doc, 0, &unsupported, range()).is_err());
    for r in [
        CycleOffsetRange { start: 0., end: 0. },
        CycleOffsetRange {
            start: -1.,
            end: 1.,
        },
        CycleOffsetRange {
            start: 0.,
            end: f32::INFINITY,
        },
        CycleOffsetRange {
            start: f32::NAN,
            end: 1.,
        },
    ] {
        assert!(CycleOffsetPlan::capture(&doc, 0, &selection(&doc, &[0]), r).is_err());
    }
    let plan = CycleOffsetPlan::capture(&doc, 0, &selection(&doc, &[0]), range()).unwrap();
    assert!(plan.advance(&doc, f32::NAN).is_err());
    assert!(plan.advance(&doc, f32::INFINITY).is_err());
    let mut unmatched = doc.clone();
    unmatched.animations[0].tracks[0].keys[3].value = 9.;
    assert!(
        CycleOffsetPlan::capture(&unmatched, 0, &selection(&unmatched, &[0]), range()).is_err()
    );
    let mut adjacent = doc.clone();
    adjacent.animations[0].tracks[0].keys = keys(&[0., f32::from_bits(1), 1.], &[8., 9., 8.]);
    let plan =
        CycleOffsetPlan::capture(&adjacent, 0, &selection(&adjacent, &[0]), range()).unwrap();
    assert!(
        plan.advance(&adjacent, 0.5)
            .unwrap_err()
            .contains("collapse")
    );
    // Negative subframes too small to encode next to the loop end explicitly
    // refuse instead of silently snapping to zero or producing duplicate keys.
    assert!(plan.advance(&adjacent, -f32::from_bits(1)).is_err());
}

#[test]
fn cycle_offset_preview_restores_memory_pending_and_redo_from_one_fixed_baseline() {
    let mut session = Session::new(fixture(true)).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.4,
    };
    let mut local = session.authored_pose(target).unwrap().bone_locals()[0];
    local.x = 17.;
    session
        .edit_pose(0, target, local, &[Property::X], false)
        .unwrap();
    let initial = session.document().clone();
    session
        .offset_animation_keys(0, &selection(&initial, &[0, 1]), range(), 0.2)
        .unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    let baseline = session.document().clone();
    let pending = session.pose_edits().clone();
    let memory = session.cycle_offset_memory().clone();
    let sources = selection(&baseline, &[0, 1]);
    session.begin_transaction();
    session
        .preview_offset_animation_keys(0, &sources, range(), 0.1)
        .unwrap();
    let out = session
        .preview_offset_animation_keys(0, &sources, range(), 0.2)
        .unwrap();
    assert_eq!(
        out.phases
            .iter()
            .map(|phase| phase.phase)
            .collect::<Vec<_>>(),
        [0.4, 0.4]
    );
    assert_eq!(session.document().animations[0].tracks[0].keys.len(), 4);
    let valid = session.document().clone();
    let valid_memory = session.cycle_offset_memory().clone();
    let mut bad = sources.clone();
    bad.push(AnimationKey::DrawOrder { time: 0. });
    assert!(
        session
            .preview_offset_animation_keys(0, &bad, range(), 0.2)
            .is_err()
    );
    assert_eq!(session.document(), &valid);
    assert_eq!(session.cycle_offset_memory(), &valid_memory);
    assert_eq!(session.pose_edits(), &pending);
    session.cancel_transaction();
    assert_eq!(session.document(), &baseline);
    assert_eq!(session.cycle_offset_memory(), &memory);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
    session.begin_transaction();
    session
        .preview_offset_animation_keys(0, &sources, range(), 0.1)
        .unwrap();
    session.commit_transaction();
    let committed = session.document().clone();
    let committed_memory = session.cycle_offset_memory().clone();
    session.undo();
    assert_eq!(session.document(), &baseline);
    assert_eq!(session.cycle_offset_memory(), &memory);
    session.redo();
    assert_eq!(session.document(), &committed);
    assert_eq!(session.cycle_offset_memory(), &committed_memory);
    assert_eq!(session.pose_edits(), &pending);
}

#[test]
fn cycle_offset_cache_capture_and_whole_period_noops_preserve_raw_data_and_redo() {
    let mut doc = fixture(false);
    doc.animations[0].tracks[0].keys[0].time = -0.;
    doc.animations[0].tracks[0].keys[0].curve = Curve::BezierOffset([0.1, -0., 0.7, 0.]);
    let raw = serde_json::to_string(&doc).unwrap();
    let mut session = Session::new(doc.clone()).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    let sources = selection(&doc, &[0]);
    session
        .offset_animation_keys(0, &sources, range(), 0.)
        .unwrap();
    assert!(!session.cycle_offset_memory().is_empty());
    assert!(session.can_redo());
    assert!(!session.can_undo());
    assert!(!session.is_dirty());
    session
        .offset_animation_keys(0, &sources, range(), -2.)
        .unwrap();
    assert_eq!(serde_json::to_string(session.document()).unwrap(), raw);
    assert!(session.can_redo());
    assert!(!session.can_undo());
    session.begin_transaction();
    session
        .preview_offset_animation_keys(0, &sources, range(), 1.)
        .unwrap();
    session.commit_transaction();
    assert!(session.can_redo());
    assert!(!session.can_undo());
    session.reset_cycle_offset_memory().unwrap();
    assert!(session.cycle_offset_memory().is_empty());
    assert!(session.can_redo());
    session.redo();
    assert_eq!(session.document().animations[1].name, "redo");
}

#[test]
fn cycle_offset_mappings_preserve_request_order_without_selecting_whole_tracks() {
    let doc = fixture(true);
    let sources = [
        AnimationKey::Bone {
            bone: 2,
            property: Property::Rotation,
            time: 0.65,
        },
        AnimationKey::Bone {
            bone: 1,
            property: Property::Y,
            time: 0.2,
        },
        AnimationKey::Bone {
            bone: 1,
            property: Property::Y,
            time: 0.6,
        },
    ];
    let outcome = CycleOffsetPlan::capture(&doc, 0, &sources, range())
        .unwrap()
        .advance(&doc, 0.2)
        .unwrap();
    assert_eq!(outcome.keys.len(), 10);
    assert_eq!(outcome.mappings.len(), 3);
    for ((mapping, source), time) in
        outcome
            .mappings
            .iter()
            .zip(&sources)
            .zip([0.65_f32 + 0.2, 0.2_f32 + 0.2, 0.6_f32 + 0.2])
    {
        assert_eq!(&mapping.source, source);
        let AnimationKey::Bone { bone, property, .. } = source else {
            unreachable!()
        };
        assert_eq!(
            mapping.target,
            Some(AnimationKey::Bone {
                bone: *bone,
                property: *property,
                time
            })
        );
        let actual = outcome.document.animations[0]
            .tracks
            .iter()
            .find(|track| track.bone == *bone && track.property == *property)
            .unwrap();
        assert!(
            actual
                .keys
                .iter()
                .any(|key| key.time.to_bits() == time.to_bits())
        );
    }
    assert_eq!(outcome.receipt().mappings, outcome.mappings);
}

#[test]
fn cycle_offset_mapping_original_endpoints_share_one_accepted_anchor() {
    // Native original first0 and terminal60 independently retain selection of
    // frame6 after +6. Backspace without reselection removes that same anchor.
    let doc = fixture(false);
    let sources = [
        AnimationKey::Bone {
            bone: 1,
            property: Property::Y,
            time: 1.,
        },
        AnimationKey::Bone {
            bone: 1,
            property: Property::Y,
            time: 0.,
        },
    ];
    let outcome = CycleOffsetPlan::capture(&doc, 0, &sources, range())
        .unwrap()
        .advance(&doc, 0.1)
        .unwrap();
    assert_eq!(outcome.keys.len(), 5);
    assert_eq!(outcome.mappings.len(), 2);
    for (mapping, source) in outcome.mappings.iter().zip(&sources) {
        assert_eq!(&mapping.source, source);
        assert_eq!(
            mapping.target,
            Some(AnimationKey::Bone {
                bone: 1,
                property: Property::Y,
                time: 0.1
            })
        );
    }
    assert_eq!(outcome.mappings[0].target, outcome.mappings[1].target);
}

#[test]
fn cycle_offset_mapping_previews_use_original_anchor_arithmetic_and_history() {
    let doc = fixture(false);
    let mut session = Session::new(doc).unwrap();
    let original = AnimationKey::Bone {
        bone: 1,
        property: Property::Y,
        time: 0.2,
    };
    session
        .offset_animation_keys(0, &[original], range(), 0.4)
        .unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    let baseline = session.document().clone();
    let memory = session.cycle_offset_memory().clone();
    let source_time = 0.2_f32 + 0.4;
    let source = AnimationKey::Bone {
        bone: 1,
        property: Property::Y,
        time: source_time,
    };
    session.begin_transaction();
    let first = session
        .preview_offset_animation_keys(0, std::slice::from_ref(&source), range(), 0.1)
        .unwrap();
    let target_time = 0.2_f32 + 0.5;
    assert_ne!(
        target_time.to_bits(),
        (source_time + 0.1).to_bits(),
        "This fixture must discriminate associated f32 sums"
    );
    assert_eq!(
        first.mappings,
        [CycleOffsetKeyMapping {
            source: source.clone(),
            target: Some(AnimationKey::Bone {
                bone: 1,
                property: Property::Y,
                time: target_time
            })
        }]
    );
    assert!(
        session.document().animations[0].tracks[0]
            .keys
            .iter()
            .any(|key| key.time.to_bits() == target_time.to_bits())
    );
    let second = session
        .preview_offset_animation_keys(0, std::slice::from_ref(&source), range(), 0.2)
        .unwrap();
    assert_eq!(
        second.mappings[0].target,
        Some(AnimationKey::Bone {
            bone: 1,
            property: Property::Y,
            time: 0.2_f32 + 0.6
        })
    );
    let valid = session.document().clone();
    let invalid = AnimationKey::Bone {
        bone: 1,
        property: Property::Y,
        time: source_time.next_up(),
    };
    assert!(
        session
            .preview_offset_animation_keys(0, &[invalid], range(), 0.2)
            .is_err()
    );
    assert_eq!(session.document(), &valid);
    session.cancel_transaction();
    assert_eq!(session.document(), &baseline);
    assert_eq!(session.cycle_offset_memory(), &memory);
    assert!(session.can_redo());
    let committed = session
        .offset_animation_keys(0, std::slice::from_ref(&source), range(), 0.1)
        .unwrap();
    let final_doc = session.document().clone();
    session.undo();
    assert_eq!(session.document(), &baseline);
    session.redo();
    assert_eq!(session.document(), &final_doc);
    let current = committed.mappings[0].target.as_ref().unwrap();
    let noop = session
        .offset_animation_keys(0, std::slice::from_ref(current), range(), 0.)
        .unwrap();
    assert_eq!(noop.mappings[0].source, *current);
    assert_eq!(noop.mappings[0].target, Some(current.clone()));
}

#[test]
fn cycle_offset_mapping_cache_only_and_full_period_noops_keep_exact_identities() {
    let doc = fixture(false);
    let mut session = Session::new(doc.clone()).unwrap();
    let initial = selection(&doc, &[0]);
    for delta in [0., 1., -2.] {
        let receipt = session
            .offset_animation_keys(0, &initial, range(), delta)
            .unwrap();
        assert_eq!(receipt.mappings.len(), initial.len());
        for (mapping, source) in receipt.mappings.iter().zip(&initial) {
            assert_eq!(&mapping.source, source);
            assert_eq!(mapping.target.as_ref(), Some(source));
        }
        assert!(!session.can_undo());
    }
    let seed = AnimationKey::Bone {
        bone: 1,
        property: Property::Y,
        time: 0.2,
    };
    session
        .offset_animation_keys(0, &[seed], range(), 0.1)
        .unwrap();
    let shifted = session.document().clone();
    let current = selection(&shifted, &[0]);
    let receipt = session
        .offset_animation_keys(0, &current, range(), 2.)
        .unwrap();
    for (index, (mapping, source)) in receipt.mappings.iter().zip(&current).enumerate() {
        assert_eq!(&mapping.source, source);
        if index == 0 || index == current.len() - 1 {
            assert!(
                mapping.target.is_none(),
                "Generated boundaries never become selected seeds"
            );
        } else {
            assert_eq!(mapping.target.as_ref(), Some(source));
        }
    }
    assert_eq!(session.document(), &shifted);
    session.undo();
    assert_eq!(session.document(), &doc);
}

#[test]
fn cycle_offset_generated_seed_mask_noops_and_mixed_tracks_preserve_state() {
    let doc = fixture(false);
    let mut session = Session::new(doc.clone()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.4,
    };
    let mut pending_local = session.authored_pose(target).unwrap().bone_locals()[0];
    pending_local.x = 17.;
    session
        .edit_pose(0, target, pending_local, &[Property::X], false)
        .unwrap();
    session
        .offset_animation_keys(0, &selection(&doc, &[0, 1]), range(), 0.1)
        .unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    let before = session.document().clone();
    let memory = session.cycle_offset_memory().clone();
    let pose = session.pose_edits().clone();
    let generation = session.document_generation();
    let fake = [
        AnimationKey::Bone {
            bone: 1,
            property: Property::Y,
            time: 1.,
        },
        AnimationKey::Bone {
            bone: 1,
            property: Property::Y,
            time: 0.,
        },
        AnimationKey::Bone {
            bone: 2,
            property: Property::Rotation,
            time: 0.,
        },
    ];
    assert!(
        session
            .cycle_offset_selectable_keys(0, &fake, range())
            .unwrap()
            .is_empty()
    );
    assert!(
        session
            .cycle_offset_selectable_keys(0, &[], range())
            .unwrap()
            .is_empty()
    );
    let frozen = CycleOffsetPlan::capture_with_memory(&before, 0, &fake, range(), &memory).unwrap();
    let mut stale = before.clone();
    stale.animations[0].tracks[0].keys[1].value += 1.;
    assert!(frozen.advance(&stale, 0.1).unwrap_err().contains("Stale"));
    assert!(
        session
            .offset_animation_keys(0, &fake, range(), f32::NAN)
            .is_err()
    );
    let receipt = session
        .offset_animation_keys(0, &fake, range(), 0.1)
        .unwrap();
    assert!(receipt.keys.is_empty());
    assert!(receipt.boundary_keys.is_empty());
    assert!(receipt.phases.is_empty());
    assert_eq!(receipt.mappings.len(), fake.len());
    for (mapping, source) in receipt.mappings.iter().zip(&fake) {
        assert_eq!(&mapping.source, source);
        assert!(mapping.target.is_none());
    }
    assert_eq!(session.document(), &before);
    assert_eq!(session.cycle_offset_memory(), &memory);
    assert_eq!(session.pose_edits(), &pose);
    assert_eq!(session.document_generation(), generation);
    assert!(session.can_redo());
    let real = AnimationKey::Bone {
        bone: 2,
        property: Property::Rotation,
        time: 0.1,
    };
    let mixed = [fake[0].clone(), real.clone(), fake[1].clone()];
    assert_eq!(
        session
            .cycle_offset_selectable_keys(0, &mixed, range())
            .unwrap()
            .as_slice(),
        std::slice::from_ref(&real)
    );
    assert_eq!(session.document(), &before);
    assert_eq!(session.cycle_offset_memory(), &memory);
    assert!(
        session
            .cycle_offset_selectable_keys(0, &[fake[0].clone(), fake[0].clone()], range())
            .is_err()
    );
    let receipt = session
        .offset_animation_keys(0, &mixed, range(), 0.1)
        .unwrap();
    assert_eq!(receipt.phases.len(), 1);
    assert_eq!(
        receipt.phases[0].source,
        AnimationKey::Bone {
            bone: 2,
            property: Property::Rotation,
            time: 0.
        }
    );
    assert_eq!(receipt.phases[0].phase, 0.2);
    assert_eq!(
        receipt.mappings,
        [
            CycleOffsetKeyMapping {
                source: mixed[0].clone(),
                target: None
            },
            CycleOffsetKeyMapping {
                source: real,
                target: Some(AnimationKey::Bone {
                    bone: 2,
                    property: Property::Rotation,
                    time: 0.2
                })
            },
            CycleOffsetKeyMapping {
                source: mixed[2].clone(),
                target: None
            },
        ]
    );
    assert_eq!(
        session.document().animations[0].tracks[0],
        before.animations[0].tracks[0]
    );
    assert_eq!(session.pose_edits(), &pose);
    session.undo();
    assert_eq!(session.document(), &before);
    assert_eq!(session.cycle_offset_memory(), &memory);
}

#[test]
fn cycle_offset_real_zero_phase_return_and_explicit_reset_follow_native_controls() {
    let doc = fixture(false);
    let mut session = Session::new(doc.clone()).unwrap();
    session
        .offset_animation_keys(
            0,
            &[AnimationKey::Bone {
                bone: 1,
                property: Property::Y,
                time: 0.2,
            }],
            range(),
            0.4,
        )
        .unwrap();
    let zero = AnimationKey::Bone {
        bone: 1,
        property: Property::Y,
        time: 0.,
    };
    let end = AnimationKey::Bone {
        bone: 1,
        property: Property::Y,
        time: 1.,
    };
    assert_eq!(
        session
            .cycle_offset_selectable_keys(0, &[end.clone(), zero.clone()], range())
            .unwrap()
            .as_slice(),
        std::slice::from_ref(&zero)
    );
    let receipt = session
        .offset_animation_keys(0, &[end.clone(), zero], range(), 0.1)
        .unwrap();
    assert!(receipt.mappings[0].target.is_none());
    let moved = (0.6_f32 + 0.5).rem_euclid(1.);
    assert_eq!(
        receipt.mappings[1].target,
        Some(AnimationKey::Bone {
            bone: 1,
            property: Property::Y,
            time: moved
        })
    );
    let seed = AnimationKey::Bone {
        bone: 1,
        property: Property::Y,
        time: 0.5,
    };
    session
        .offset_animation_keys(0, &[seed], range(), 0.5)
        .unwrap();
    assert_eq!(session.document(), &doc);
    assert_eq!(
        session
            .cycle_offset_selectable_keys(0, std::slice::from_ref(&end), range())
            .unwrap()
            .as_slice(),
        std::slice::from_ref(&end)
    );
    let receipt = session
        .offset_animation_keys(0, &[end], range(), 0.1)
        .unwrap();
    assert_eq!(
        receipt.mappings[0].target,
        Some(AnimationKey::Bone {
            bone: 1,
            property: Property::Y,
            time: 0.1
        })
    );
    // Native Off/On recaptures current data: the previous generated seam is
    // carried into the interior after another real seed moves +6.
    session.reset_cycle_offset_memory().unwrap();
    let seed = AnimationKey::Bone {
        bone: 1,
        property: Property::Y,
        time: 0.2_f32 + 0.1,
    };
    session
        .offset_animation_keys(0, &[seed], range(), 0.1)
        .unwrap();
    let shifted = &session.document().animations[0].tracks[0].keys;
    assert_eq!(shifted.len(), 6);
    for ((key, time), value) in shifted
        .iter()
        .zip([0., 0.1, 0.2, 0.4, 0.8, 1.])
        .zip([-4.000004, 1.999998, 8., 32., -16., -4.000004])
    {
        close(key.time, time);
        close(key.value, value);
    }
}
