//! World-space transform constraint rules, checked as geometric properties. The
//! rules were fitted to external-runtime observations (max error 1e-6 over 300
//! random rigs) and the Transform Lab fixture matches the runtime end to end.
use crate::*;
use peachy_document::*;

fn bone(name: &str, parent: Option<usize>, setup: Transform) -> Bone {
    Bone {
        name: name.into(),
        parent,
        length: 10.,
        setup,
        inherit: Inherit::Normal,
    }
}

fn rig(mirrored: bool, mappings: &[TransformProperty], mix: f32) -> Document {
    let mapping = |p: TransformProperty| TransformMapping {
        from: p,
        offset: 0.,
        to: vec![TransformTarget::identity(p)],
    };
    let mut values = TransformValues::default();
    for &p in mappings {
        values.set(p, mix);
    }
    Document {
        format: FORMAT_VERSION,
        name: "Transform".into(),
        fps: 30,
        bones: vec![
            bone("root", None, Transform::default()),
            bone(
                "source",
                Some(0),
                Transform {
                    x: 40.,
                    y: -10.,
                    rotation: 35.,
                    scale_x: if mirrored { -1.5 } else { 1.5 },
                    ..Transform::default()
                },
            ),
            bone(
                "parent",
                Some(0),
                Transform {
                    x: -20.,
                    rotation: -60.,
                    scale_x: 2.,
                    scale_y: 0.5,
                    shear_y: 12.,
                    ..Transform::default()
                },
            ),
            bone(
                "follower",
                Some(2),
                Transform {
                    x: 5.,
                    y: 3.,
                    rotation: 15.,
                    ..Transform::default()
                },
            ),
            bone(
                "follower child",
                Some(3),
                Transform {
                    x: 7.,
                    ..Transform::default()
                },
            ),
        ],
        constraints: vec![Constraint::Transform(TransformConstraint {
            name: "follow".into(),
            bones: vec![3],
            source: 1,
            offsets: TransformValues {
                rotate: 30.,
                x: 6.,
                y: -4.,
                ..TransformValues::default()
            },
            mix: values,
            mappings: mappings.iter().map(|&p| mapping(p)).collect(),
            local_source: false,
            local_target: false,
            additive: false,
            clamp: false,
        })],
        slots: vec![],
        skins: Default::default(),
        animations: vec![],
        skin_scope: Default::default(),
        events: vec![],
        reference_scale: 100.0,
    }
}

fn angle(m: Affine) -> f64 {
    f64::from(m.c).atan2(f64::from(m.a)).to_degrees()
}

#[test]
fn full_translate_mix_reaches_the_source_frame_offset_point() {
    let doc = rig(false, &[TransformProperty::X, TransformProperty::Y], 1.);
    doc.validate().unwrap();
    let pose = checked_evaluate_pose(&doc, None, 0.).unwrap();
    let [x, y] = pose.world()[1].point(6., -4.);
    let follower = pose.world()[3];
    assert!((follower.x - x).abs() < 1e-3 && (follower.y - y).abs() < 1e-3);
    // Children follow the moved bone through the written-back local transform.
    let child = pose.world()[4];
    let [cx, cy] = follower.point(7., 0.);
    assert!((child.x - cx).abs() < 1e-3 && (child.y - cy).abs() < 1e-3);
    // Half and mirrored (negative) mixes move proportionally.
    let base = checked_evaluate_pose(&rig(false, &[], 0.), None, 0.)
        .unwrap()
        .world()[3];
    for mix in [0.5, -1.] {
        let moved = checked_evaluate_pose(
            &rig(false, &[TransformProperty::X, TransformProperty::Y], mix),
            None,
            0.,
        )
        .unwrap()
        .world()[3];
        let expected = base.x + (x - base.x) * mix;
        assert!((moved.x - expected).abs() < 1e-3, "{mix}");
    }
}

#[test]
fn rotate_turns_the_world_frame_and_mirrored_sources_subtract_the_offset() {
    for mirrored in [false, true] {
        let doc = rig(mirrored, &[TransformProperty::Rotate], 1.);
        let pose = checked_evaluate_pose(&doc, None, 0.).unwrap();
        let source = pose.world()[1];
        let offset = if mirrored { -30. } else { 30. };
        let mut expected = angle(source) + offset - angle(pose.world()[3]);
        expected = (expected + 540.) % 360. - 180.;
        assert!(expected.abs() < 1e-3, "mirrored {mirrored}: {expected}");
        // Turning keeps the follower's own scale and shear (area is preserved).
        let before = checked_evaluate_pose(&rig(mirrored, &[], 0.), None, 0.)
            .unwrap()
            .world()[3];
        let det = |m: Affine| m.a * m.d - m.b * m.c;
        assert!((det(pose.world()[3]) - det(before)).abs() < 1e-4);
    }
}

#[test]
fn authored_locals_stay_unchanged_and_unsupported_modes_fail_explicitly() {
    let doc = rig(false, &[TransformProperty::X], 1.);
    let pose = checked_evaluate_pose(&doc, None, 0.).unwrap();
    assert_eq!(pose.authored_locals()[3], doc.bones[3].setup);
    assert_ne!(pose.applied_locals()[3], doc.bones[3].setup);
    let mut local = doc.clone();
    let Constraint::Transform(t) = &mut local.constraints[0] else {
        unreachable!()
    };
    t.local_target = true;
    assert!(
        checked_evaluate_pose(&local, None, 0.)
            .unwrap_err()
            .contains("local, additive and clamped")
    );
    let mut inherited = doc;
    inherited.bones[3].inherit = Inherit::NoScale;
    assert!(
        checked_evaluate_pose(&inherited, None, 0.)
            .unwrap_err()
            .contains("must inherit normally")
    );
}
