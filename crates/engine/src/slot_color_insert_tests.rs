//! Color insertion preserves curve geometry through a shared native/MCP command.
//! Installed-editor time inversion is independently calibrated; cubic geometry
//! uses stable subdivision, separate from the runtime's ten-segment resampling.
use crate::*;
use peachy_document::*;

fn fixture(curves: [Curve; 4]) -> (Document, usize) {
    let mut doc: Document =
        crate::native_fixture::decode(include_str!("../../../examples/peach-bot/slot-bot.peachy"))
            .unwrap();
    let slot = doc
        .slots
        .iter()
        .position(|slot| slot.name == "head")
        .unwrap();
    doc.animations[0].color_tracks = vec![ColorTrack {
        slot,
        keys: vec![
            ColorKey {
                time: 0.,
                color: [255, 128, 64, 255],
                curves,
            },
            ColorKey {
                time: 0.8,
                color: [0, 128, 128, 0],
                curves: [Curve::Stepped; 4],
            },
        ],
    }];
    doc.validate().unwrap();
    (doc, slot)
}
fn keys(doc: &Document, slot: usize) -> &[ColorKey] {
    &doc.animations[0]
        .color_tracks
        .iter()
        .find(|track| track.slot == slot)
        .unwrap()
        .keys
}
fn controls(curve: Curve, from: f32, to: f32) -> [f64; 4] {
    match curve {
        Curve::Bezier([x1, y1, x2, y2]) => [
            f64::from(x1),
            f64::from(from) + f64::from(to - from) * f64::from(y1),
            f64::from(x2),
            f64::from(from) + f64::from(to - from) * f64::from(y2),
        ],
        Curve::BezierOffset([x1, y1, x2, y2]) => [
            f64::from(x1),
            f64::from(from) + f64::from(y1),
            f64::from(x2),
            f64::from(from) + f64::from(y2),
        ],
        _ => panic!("Expected cubic controls"),
    }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 2e-6, "{a} != {b}");
}

#[test]
fn slot_color_insert_splits_all_channel_curves_while_raw_setter_keeps_explicit_payload() {
    let curves = [
        Curve::Bezier([0.25, 0.2, 0.75, 0.8]),
        Curve::BezierOffset([0.25, 0., 0.75, 0.]),
        Curve::BezierOffset([0.25, 0.4, 0.75, -0.2]),
        Curve::Bezier([0.25, -0.2, 0.75, 1.2]),
    ];
    let (doc, slot) = fixture(curves);
    let session = Session::new(doc.clone()).unwrap();
    let raw = session
        .slot_pose(
            slot,
            EditTarget::Animate {
                animation: 0,
                time: 0.4,
            },
        )
        .unwrap()
        .color;
    let inserted = apply_command(
        &doc,
        Command::InsertSlotColorKey {
            animation: 0,
            slot,
            time: 0.4,
            color: raw,
        },
    )
    .unwrap();
    let out = keys(&inserted, slot);
    assert_eq!(out.len(), 3);
    assert_eq!(
        out.iter().map(|key| key.time).collect::<Vec<_>>(),
        vec![0., 0.4, 0.8]
    );
    assert_eq!(out[2], keys(&doc, slot)[1]);
    assert_eq!(out[1].color, slot_color_bytes(raw).unwrap());
    assert_ne!(out[0].curves, curves);
    assert!(out[1].curves.iter().all(|curve| curve.is_bezier()));
    assert!(matches!(out[0].curves[1], Curve::BezierOffset(_)));
    let middle = f32::from(out[1].color[0]) / 255.;
    let left = controls(out[0].curves[0], 1., middle);
    let right = controls(out[1].curves[0], middle, 0.);
    near(left[0] * 0.4, 0.1);
    near(left[2] * 0.4, 0.25);
    near(0.4 + right[0] * 0.4, 0.55);
    near(0.4 + right[2] * 0.4, 0.7);
    near(left[1], 0.9);
    near(left[3], 0.7);
    near(right[1], 0.3);
    near(right[3], 0.1);
    assert_eq!(inserted.skins, doc.skins);
    assert_eq!(inserted.slots, doc.slots);
    let explicit = apply_command(
        &doc,
        Command::SetSlotColorKey {
            animation: 0,
            slot,
            key: ColorKey {
                time: 0.4,
                color: out[1].color,
                curves: [Curve::Linear; 4],
            },
        },
    )
    .unwrap();
    assert_eq!(keys(&explicit, slot)[0], keys(&doc, slot)[0]);
    assert_eq!(keys(&explicit, slot)[1].curves, [Curve::Linear; 4]);
}

#[test]
fn slot_color_insert_session_uses_precise_pending_float_before_encoding_controls_and_is_one_undo() {
    let (doc, slot) = fixture([Curve::Bezier([0.25, 0.2, 0.75, 0.8]); 4]);
    let mut session = Session::new(doc.clone()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.4,
    };
    let color = [0.25749, 0.5, 0.5, 0.5];
    session
        .edit_slot_pose(slot, target, &[SlotPoseEdit::Color { color }], false)
        .unwrap();
    let pending = session.pose_edits().clone();
    assert_eq!(session.document(), &doc);
    session
        .key_slot_pose(0, 0.4, slot, &[SlotProperty::Color])
        .unwrap();
    let keyed = session.document().clone();
    let current = keys(&keyed, slot);
    let stored = f32::from(current[1].color[0]) / 255.;
    let left = controls(current[0].curves[0], 1., stored);
    near(left[3], 0.7 + (f64::from(color[0]) - 0.5));
    assert!(session.pose_edits().is_empty());
    assert_ne!(current[1].curves, [Curve::Linear; 4]);
    session.undo();
    assert_eq!(session.document(), &doc);
    assert_eq!(session.pose_edits(), &pending);
    session.redo();
    assert_eq!(session.document(), &keyed);
    assert!(session.pose_edits().is_empty());
    let mut auto = Session::new(doc).unwrap();
    auto.edit_slot_pose(slot, target, &[SlotPoseEdit::Color { color }], true)
        .unwrap();
    assert_eq!(auto.document(), &keyed);
}

#[test]
fn slot_color_insert_retains_stepped_group_and_existing_exact_key_curves() {
    let (doc, slot) = fixture([Curve::Stepped; 4]);
    let inserted = apply_command(
        &doc,
        Command::InsertSlotColorKey {
            animation: 0,
            slot,
            time: 0.4,
            color: [0.2, 0.3, 0.4, 0.5],
        },
    )
    .unwrap();
    assert_eq!(keys(&inserted, slot)[0].curves, [Curve::Stepped; 4]);
    assert_eq!(keys(&inserted, slot)[1].curves, [Curve::Stepped; 4]);
    let updated = apply_command(
        &inserted,
        Command::InsertSlotColorKey {
            animation: 0,
            slot,
            time: 0.4,
            color: [0.4, 0.3, 0.2, 1.],
        },
    )
    .unwrap();
    assert_eq!(keys(&updated, slot).len(), 3);
    assert_eq!(keys(&updated, slot)[1].curves, [Curve::Stepped; 4]);
    assert_eq!(keys(&updated, slot)[0], keys(&inserted, slot)[0]);
    assert_eq!(keys(&updated, slot)[2], keys(&inserted, slot)[2]);
}

#[test]
fn slot_color_insert_flat_endpoints_overshoot_and_adjacent_f32_times_keep_finite_controls() {
    let (mut doc, slot) = fixture([Curve::BezierOffset([0.1, 0.2, 0.9, -0.2]); 4]);
    let first = 0.25_f32;
    let middle = f32::from_bits(first.to_bits() + 1);
    let last = f32::from_bits(first.to_bits() + 2);
    let track = doc.animations[0]
        .color_tracks
        .iter_mut()
        .find(|track| track.slot == slot)
        .unwrap();
    for key in &mut track.keys {
        key.color = [128; 4];
    }
    track.keys[0].time = first;
    track.keys[1].time = last;
    let inserted = apply_command(
        &doc,
        Command::InsertSlotColorKey {
            animation: 0,
            slot,
            time: middle,
            color: [128. / 255.; 4],
        },
    )
    .unwrap();
    assert_eq!(
        keys(&inserted, slot)
            .iter()
            .map(|key| key.time)
            .collect::<Vec<_>>(),
        vec![first, middle, last]
    );
    for key in &keys(&inserted, slot)[..2] {
        assert!(key.curves.iter().all(|curve| curve.handles_valid()));
        assert!(
            key.curves
                .iter()
                .all(|curve| matches!(curve, Curve::BezierOffset(_)))
        );
    }
    assert_ne!(keys(&inserted, slot)[0].curves, [Curve::Linear; 4]);
    let tiny = f32::from_bits(1);
    let split = crate::color_curve_split::split(
        Curve::Bezier([0.25, 0.2, 0.75, 0.8]),
        [0., tiny, f32::MAX],
        [0., 0.1, 1.],
        26. / 255.,
    )
    .unwrap();
    assert!(split.0.handles_valid());
    assert!(split.1.handles_valid());
}

#[test]
fn slot_color_insert_nonmonotone_time_uses_first_branch_and_does_not_clamp_handles() {
    let a = 1.3_f32;
    let b = -0.3_f32;
    let expected = (1. - (1. - 4. / (6. * f64::from(a) - 2.)).sqrt()) / 2.;
    let value = (3. * (1. - expected).powi(2) * expected * (-0.2)
        + 3. * (1. - expected) * expected * expected * 1.2
        + expected.powi(3)) as f32;
    let (left, right) = crate::color_curve_split::split(
        Curve::Bezier([a, -0.2, b, 1.2]),
        [0., 0.4, 0.8],
        [0., value, 1.],
        value,
    )
    .unwrap();
    let controls = controls(left, 0., value);
    let inferred = controls[0] * 0.5 / f64::from(a);
    assert!(inferred >= expected - 1e-6);
    assert!(inferred - expected < 1. / 300. + 1e-6);
    assert!(inferred < 0.3, "Must not select the later central branch");
    assert!(left.handles_valid());
    assert!(right.handles_valid());
    let (left, right) = crate::color_curve_split::split(
        Curve::Bezier([-0.3, -0.2, 1.3, 1.2]),
        [0., 0.4, 0.8],
        [0., 0.5, 1.],
        0.5,
    )
    .unwrap();
    assert!(left.handles_valid());
    assert!(right.handles_valid());
}

#[test]
fn slot_color_insert_unrepresentable_color_and_bad_times_fail_atomically_in_mixed_batches() {
    let (doc, slot) = fixture([Curve::Bezier([0.25, 0.2, 0.75, 0.8]); 4]);
    let mut session = Session::new(doc.clone()).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "temporary".into(),
        })
        .unwrap();
    session.undo();
    for (time, color) in [
        (0.4, [1.1, 0., 0., 1.]),
        (0.4, [f32::NAN; 4]),
        (-0.1, [0.; 4]),
        (f32::INFINITY, [0.; 4]),
    ] {
        assert!(
            session
                .dispatch(Command::Batch {
                    commands: vec![
                        Command::RenameAnimation {
                            animation: 1,
                            name: "must-not-publish".into()
                        },
                        Command::InsertSlotColorKey {
                            animation: 0,
                            slot,
                            time,
                            color
                        },
                    ]
                })
                .is_err()
        );
        assert_eq!(session.document(), &doc);
        assert!(session.can_redo());
        assert!(!session.can_undo());
    }
    let wire = serde_json::to_value(Command::InsertSlotColorKey {
        animation: 0,
        slot,
        time: 0.4,
        color: [0.5; 4],
    })
    .unwrap();
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<Command>(wire.clone()).unwrap()).unwrap(),
        wire
    );
    let mut unknown = wire;
    unknown["curves"] = serde_json::json!(["linear", "linear", "linear", "linear"]);
    assert!(serde_json::from_value::<Command>(unknown).is_err());
}
