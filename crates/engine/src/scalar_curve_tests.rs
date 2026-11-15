//! Curve-only commands preserve exact scalar identity and value, independently
//! of the presentation policy that chooses an editable curve interval.
use crate::{Command, EditTarget, Session, apply_command};
use peachy_document::*;

fn fixture() -> Document {
    let doc = Document::new("Exact scalar curves".into(), 30).unwrap();
    let mut doc = apply_command(
        &doc,
        Command::AddAnimation {
            name: "clip".into(),
            duration: 0.,
        },
    )
    .unwrap();
    let first = 0.25_f32;
    doc.animations[0].tracks = Property::ALL
        .into_iter()
        .map(|property| Track {
            bone: 0,
            property,
            keys: vec![
                Keyframe {
                    time: first,
                    value: f32::from_bits(0x4140_0001),
                    curve: Curve::BezierOffset([-0.2, 1e-7, 1.2, -1e-7]),
                },
                Keyframe {
                    time: f32::from_bits(first.to_bits() + 1),
                    value: -0.,
                    curve: Curve::Bezier([0.2, -0.3, 0.8, 1.3]),
                },
                Keyframe {
                    time: 0.8,
                    value: f32::from_bits(0x4140_0001),
                    curve: Curve::Stepped,
                },
            ],
        })
        .collect();
    doc.validate().unwrap();
    doc
}

fn command(property: Property, time: f32, curve: Curve) -> Command {
    Command::SetKeyCurve {
        animation: 0,
        bone: 0,
        property,
        time,
        curve,
    }
}

#[test]
fn scalar_curve_edits_preserve_neighbor_f32_identities_values_and_all_sibling_properties() {
    let before = fixture();
    let time = f32::from_bits(0.25_f32.to_bits() + 1);
    let curve = Curve::BezierOffset([1.3, 0.000001, -0.3, -0.000002]);
    for property in Property::ALL {
        let after = apply_command(&before, command(property, time, curve)).unwrap();
        let mut expected = before.clone();
        expected.animations[0]
            .tracks
            .iter_mut()
            .find(|t| t.property == property)
            .unwrap()
            .keys[1]
            .curve = curve;
        assert_eq!(after, expected);
        for (old, new) in before.animations[0]
            .tracks
            .iter()
            .zip(&after.animations[0].tracks)
        {
            for (old, new) in old.keys.iter().zip(&new.keys) {
                assert_eq!(old.time.to_bits(), new.time.to_bits());
                assert_eq!(old.value.to_bits(), new.value.to_bits());
            }
        }
    }
}

#[test]
fn scalar_curve_raw_last_key_edits_change_only_that_stored_key() {
    let before = fixture();
    let curve = Curve::Bezier([-0.5, 0.4, 1.5, 0.6]);
    let after = apply_command(&before, command(Property::X, 0.8, curve)).unwrap();
    let mut expected = before.clone();
    expected.animations[0].tracks[0].keys[2].curve = curve;
    assert_eq!(
        after, expected,
        "Raw authoring must not silently choose the previous interval"
    );
    assert_eq!(
        crate::sample(&after.animations[0].tracks[0], 1.),
        before.animations[0].tracks[0].keys[2].value
    );
}

#[test]
fn scalar_curve_missing_exact_keys_and_invalid_batches_preserve_document_and_redo() {
    let before = fixture();
    let mut session = Session::new(before.clone()).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo survives".into(),
        })
        .unwrap();
    session.undo();
    let absent_neighbor = f32::from_bits(0.25_f32.to_bits() + 2);
    let mut bad = vec![
        command(Property::X, absent_neighbor, Curve::Linear),
        command(Property::X, -0.1, Curve::Linear),
        command(Property::X, f32::NAN, Curve::Linear),
        command(
            Property::X,
            0.25,
            Curve::Bezier([0., f32::INFINITY, 1., 1.]),
        ),
        command(
            Property::X,
            0.25,
            Curve::BezierOffset([0., 0., f32::NAN, 1.]),
        ),
        Command::SetKeyCurve {
            animation: 1,
            bone: 0,
            property: Property::X,
            time: 0.25,
            curve: Curve::Linear,
        },
        Command::SetKeyCurve {
            animation: 0,
            bone: 1,
            property: Property::X,
            time: 0.25,
            curve: Curve::Linear,
        },
    ];
    let mut missing = before.clone();
    missing.animations[0]
        .tracks
        .retain(|track| track.property != Property::Y);
    assert!(
        apply_command(&missing, command(Property::Y, 0.25, Curve::Linear))
            .unwrap_err()
            .contains("Unknown exact scalar key")
    );
    for bad in bad.drain(..) {
        assert!(
            session
                .dispatch(Command::Batch {
                    commands: vec![command(Property::Y, 0.25, Curve::Linear), bad,]
                })
                .is_err()
        );
        assert_eq!(session.document(), &before);
        assert!(!session.can_undo());
        assert!(session.can_redo());
    }
    session.redo();
    assert_eq!(session.document().animations[0].name, "redo survives");
}

#[test]
fn scalar_curve_previews_use_fixed_baselines_and_keep_pending_pose_in_one_history_step() {
    let before = fixture();
    let mut session = Session::new(before.clone()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    let mut pose = session.local_pose(Some(0), 0.5)[0];
    pose.y = 17.12345;
    session
        .edit_pose(0, target, pose, &[Property::Y], false)
        .unwrap();
    let pending = session.pose_edits().clone();
    assert!(!pending.is_empty());
    let curve = Curve::BezierOffset([0.125, 1e-7, 0.875, -1e-7]);
    session.begin_transaction();
    session
        .preview_transaction(command(Property::X, 0.25, Curve::Linear))
        .unwrap();
    session
        .preview_transaction(command(Property::Y, 0.25, curve))
        .unwrap();
    let mut expected = before.clone();
    expected.animations[0].tracks[1].keys[0].curve = curve;
    assert_eq!(session.document(), &expected);
    assert_eq!(session.pose_edits(), &pending);
    assert_eq!(session.local_pose(Some(0), 0.5)[0].y, pose.y);
    assert!(
        session
            .preview_transaction(command(Property::Y, 0.4, Curve::Linear))
            .is_err()
    );
    assert_eq!(session.document(), &expected);
    session.commit_transaction();
    session.undo();
    assert_eq!(session.document(), &before);
    assert_eq!(session.pose_edits(), &pending);
    session.begin_transaction();
    session
        .preview_transaction(command(Property::Y, 0.25, curve))
        .unwrap();
    session.cancel_transaction();
    assert_eq!(session.document(), &before);
    assert!(session.can_redo());
    session.redo();
    assert_eq!(session.document(), &expected);
    assert_eq!(session.pose_edits(), &pending);
    session.dispatch(command(Property::Y, 0.25, curve)).unwrap();
    session.undo();
    assert_eq!(
        session.document(),
        &before,
        "Unchanged curve must not create another undo step"
    );
}

#[test]
fn scalar_curve_wire_payload_is_strict_and_preserves_flat_offset_controls() {
    for curve in [
        Curve::Linear,
        Curve::Stepped,
        Curve::Bezier([-0.2, 0.4, 1.2, 0.6]),
        Curve::BezierOffset([0.2, 1e-7, 0.8, -1e-7]),
    ] {
        let wire = serde_json::to_value(command(Property::ShearX, 0.25, curve)).unwrap();
        assert_eq!(wire["command"], "setKeyCurve");
        let parsed: Command = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
        let mut unknown = wire;
        unknown["value"] = serde_json::json!(5);
        assert!(serde_json::from_value::<Command>(unknown).is_err());
        let before = fixture();
        let after = apply_command(&before, command(Property::ShearX, 0.25, curve)).unwrap();
        assert_eq!(
            after.animations[0]
                .tracks
                .iter()
                .find(|t| t.property == Property::ShearX)
                .unwrap()
                .keys[0]
                .curve,
            curve
        );
    }
}
