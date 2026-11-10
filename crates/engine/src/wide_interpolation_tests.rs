use crate::*;
use peachy_document::*;

fn track(property: Property, left: f32, right: f32, curve: Curve) -> Track {
    Track {
        bone: 0,
        property,
        keys: vec![
            Keyframe {
                time: 0.,
                value: left,
                curve,
            },
            Keyframe {
                time: 1.,
                value: right,
                curve: Curve::Linear,
            },
        ],
    }
}

fn close(actual: f32, expected: f64) {
    assert!(actual.is_finite());
    let tolerance = expected.abs().max(1.0) * 0.000001;
    assert!(
        (f64::from(actual) - expected).abs() <= tolerance,
        "{actual} differs from {expected}"
    );
}

#[test]
fn finite_extreme_linear_and_stepped_intervals_keep_exact_quarters_and_endpoints() {
    let left = -2e38_f32;
    let right = 2e38_f32;
    for property in Property::ALL {
        let linear = track(property, left, right, Curve::Linear);
        for (time, expected) in [
            (0., left),
            (0.25, left * 0.5),
            (0.5, 0.),
            (0.75, right * 0.5),
            (1., right),
            (2., right),
        ] {
            assert_eq!(
                sample(&linear, time).to_bits(),
                expected.to_bits(),
                "{property:?} at {time}"
            );
        }
        assert_eq!(sample(&linear, -1.), property.neutral());
        let stepped = track(property, left, right, Curve::Stepped);
        for time in [0., 0.25, 0.5, 0.75] {
            assert_eq!(sample(&stepped, time), left);
        }
        assert_eq!(sample(&stepped, 1.), right);
    }
}

#[test]
fn wide_bezier_interval_follows_the_existing_ten_segment_polyline_without_value_clamping() {
    let left = -2e38_f32;
    let right = 2e38_f32;
    let curve = Curve::Bezier([0.25, 0., 0.75, 1.]);
    let key_track = track(Property::ShearX, left, right, curve);
    assert_eq!(sample(&key_track, 0.), left);
    assert_eq!(sample(&key_track, 0.5), 0.);
    assert_eq!(sample(&key_track, 1.), right);
    // Independently known parametric samples bracket time .25:
    // u=.2 => (x=.176,y=.104); u=.3 => (x=.279,y=.216).
    // This intentionally expects the ten-segment chord, not exact cubic inversion.
    let progress = 0.104 + (0.25 - 0.176) / (0.279 - 0.176) * (0.216 - 0.104);
    close(
        sample(&key_track, 0.25),
        f64::from(left) + (f64::from(right) - f64::from(left)) * progress,
    );
    close(
        sample(&key_track, 0.75),
        f64::from(left) + (f64::from(right) - f64::from(left)) * (1. - progress),
    );
    // Existing ordinary paths retain their familiar exact results.
    assert_eq!(
        sample(&track(Property::Rotation, 14., 28., Curve::Linear), 0.25),
        17.5
    );
    assert_eq!(
        sample(&track(Property::Rotation, 14., 28., Curve::Stepped), 0.75),
        14.
    );
    assert_eq!(
        sample(&track(Property::Rotation, 14., 28., curve), 0.5),
        21.
    );
}

#[test]
fn wide_fallback_handles_finite_span_product_overflow_and_preserves_genuine_final_overflow() {
    // At u=.5 these handles produce y=.75*6.5+.125=5.
    // A finite 1e38 span times5 overflows f32, but adding -2e38 leaves a representable result.
    let curve = Curve::Bezier([0.25, 6.5, 0.75, 6.5]);
    assert_eq!(curve_progress(curve, 0.5), 5.);
    let key_track = track(Property::ShearY, -2e38, -1e38, curve);
    close(
        sample(&key_track, 0.5),
        f64::from(-2e38_f32) + (f64::from(-1e38_f32) - f64::from(-2e38_f32)) * 5.,
    );
    let outside = track(Property::ShearY, 0., 2e38, curve);
    assert_eq!(sample(&outside, 0.5), f32::INFINITY);
    // Large valid normalized control values retain the same sampling algorithm.
    let large = Curve::Bezier([0.25, f32::MAX, 0.75, f32::MAX]);
    close(curve_progress(large, 0.5), f64::from(f32::MAX) * 0.75);
    let tiny_span = track(Property::X, 0., 1e-38, large);
    close(
        sample(&tiny_span, 0.5),
        f64::from(1e-38_f32) * f64::from(f32::MAX) * 0.75,
    );
}

#[test]
fn extreme_shear_sampling_and_pose_preview_preserve_original_keys_curves_and_history() {
    let curve = Curve::Bezier([0.25, 0., 0.75, 1.]);
    let document = Document {
        constraints: vec![],
        format: FORMAT_VERSION,
        name: "Original wide shear interval".into(),
        fps: 30,
        bones: vec![Bone {
            name: "root".into(),
            parent: None,
            length: 10.,
            setup: Transform::default(),
            inherit: peachy_document::Inherit::Normal,
        }],
        slots: vec![],
        skins: Default::default(),
        animations: vec![Animation {
            ik_tracks: vec![],
            name: "extreme".into(),
            tracks: vec![track(Property::ShearX, -2e38, 2e38, curve)],
            physics: vec![],
            physics_resets: vec![],
            inherit_tracks: vec![],
            deform_tracks: vec![],
            attachment_tracks: vec![],
            color_tracks: vec![],
        }],
        skin_scope: Default::default(),
    };
    let mut session = Session::new(document.clone()).unwrap();
    session
        .dispatch(Command::SetBoneLength {
            bone: 0,
            length: 37.,
        })
        .unwrap();
    session.undo();
    for time in [0., 0.25, 0.5, 0.75, 1., 2.] {
        let locals = session.local_pose(Some(0), time);
        assert!(locals[0].shear_x.is_finite());
        for world in world_pose(session.document(), &locals) {
            assert!(
                [world.a, world.b, world.c, world.d, world.x, world.y]
                    .into_iter()
                    .all(f32::is_finite)
            );
        }
    }
    assert_eq!(session.local_pose(Some(0), 0.5)[0].shear_x, 0.);
    assert_eq!(session.document(), &document);
    assert!(session.can_redo());
    assert!(!session.can_undo());
    assert!(!session.is_dirty());
    session.begin_transaction();
    let posed = Transform {
        shear_x: 15.,
        ..session.local_pose(Some(0), 0.5)[0]
    };
    session
        .preview_edit_pose(
            0,
            EditTarget::Animate {
                animation: 0,
                time: 0.5,
            },
            posed,
            &[Property::ShearX],
            false,
        )
        .unwrap();
    session.commit_transaction();
    session.undo();
    assert_eq!(session.document(), &document);
    assert!(session.pose_edits().is_empty());
    assert!(!session.is_dirty());
    session.redo();
    assert_eq!(session.local_pose(Some(0), 0.5)[0].shear_x, 15.);
    assert_eq!(
        session.document().animations[0].tracks[0].keys[0].curve,
        curve
    );
    assert_eq!(session.document().animations, document.animations);
}
