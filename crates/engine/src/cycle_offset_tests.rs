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
fn cycle_offset_memory_tracks_have_independent_phases_across_owner_subsets() {
    let doc = fixture(true);
    let mut session = Session::new(doc.clone()).unwrap();
    session
        .offset_animation_keys(0, &selection(&doc, &[0, 1]), range(), 0.2)
        .unwrap();
    let both = session.document().clone();
    session
        .offset_animation_keys(0, &selection(&both, &[0]), range(), 0.1)
        .unwrap();
    assert_eq!(
        session.document().animations[0].tracks[1],
        both.animations[0].tracks[1]
    );
    let one = session.document().clone();
    let receipt = session
        .offset_animation_keys(0, &selection(&one, &[0, 1]), range(), 0.1)
        .unwrap();
    assert_eq!(
        receipt
            .phases
            .iter()
            .map(|phase| phase.phase)
            .collect::<Vec<_>>(),
        [0.4, 0.3]
    );
    for (index, phase) in [(0, 0.4), (1, 0.3)] {
        let expected = CycleOffsetPlan::capture(&doc, 0, &selection(&doc, &[index]), range())
            .unwrap()
            .advance(&doc, phase)
            .unwrap();
        assert_eq!(
            session.document().animations[0].tracks[index],
            expected.document.animations[0].tracks[index]
        );
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
    let mut partial = selection(&doc, &[0, 1]);
    partial.pop();
    assert!(CycleOffsetPlan::capture(&doc, 0, &partial, range()).is_err());
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
