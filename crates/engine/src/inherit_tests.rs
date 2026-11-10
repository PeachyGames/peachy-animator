//! Bone inheritance modes. The reference values are recorded black-box
//! observations of the external 4.3.13 runtime for original random poses (parent
//! world linear part, child local transform, child world linear part); see
//! docs/slices/inherit-modes.md. No runtime implementation was read.
use crate::pose::inherit_linear;
use crate::*;
use peachy_document::*;

type Observation = (&'static str, [f64; 4], [f64; 5], [f64; 4]);

const OBSERVED: [Observation; 8] = [
    (
        "noRotationOrReflection",
        [-0.881663025, 0.830390919, -0.794123006, 0.226338703],
        [
            -41.7367923,
            -1.87898588,
            -1.71202601,
            10.4210553,
            25.4408097,
        ],
        [-0.942227447, -1.05101023, 0.0901980149, -1.41582564],
    ),
    (
        "noRotationOrReflection",
        [-0.193893085, -0.238589467, -0.0410977581, 0.447109764],
        [
            117.891251,
            -0.847343445,
            -1.11431885,
            -2.69222081,
            16.4863229,
        ],
        [0.120876438, 0.1221093, -0.384890713, 0.323122584],
    ),
    (
        "noScale",
        [0.022133446, -0.128890602, 0.00927942462, 0.377933136],
        [
            -72.9696465,
            0.843446732,
            -1.36994743,
            -25.2689821,
            -20.899204,
        ],
        [-0.079139812, -1.36973284, -0.839725717, 0.0242471378],
    ),
    (
        "noScale",
        [-0.18440606, -0.077667371, -0.161395693, 0.146180791],
        [26.9995022, 1.61397076, -1.18409824, 7.26875961, -20.0348783],
        [-1.56643779, 0.780629427, -0.388811329, -0.89034058],
    ),
    (
        "noScaleOrReflection",
        [0.022133446, -0.128890602, 0.00927942462, 0.377933136],
        [
            -72.9696465,
            0.843446732,
            -1.36994743,
            -25.2689821,
            -20.899204,
        ],
        [-0.079139812, -1.36973284, -0.839725717, 0.0242471378],
    ),
    (
        "noScaleOrReflection",
        [-0.18440606, -0.077667371, -0.161395693, 0.146180791],
        [26.9995022, 1.61397076, -1.18409824, 7.26875961, -20.0348783],
        [-1.41868884, -0.0242556885, -0.769560647, 1.18384978],
    ),
    (
        "onlyTranslation",
        [-0.881663025, 0.830390919, -0.794123006, 0.226338703],
        [
            -41.7367923,
            -1.87898588,
            -1.71202601,
            10.4210553,
            25.4408097,
        ],
        [-1.60524789, -0.480393458, 0.97661003, -1.64324533],
    ),
    (
        "onlyTranslation",
        [-0.193893085, -0.238589467, -0.0410977581, 0.447109764],
        [
            117.891251,
            -0.847343445,
            -1.11431885,
            -2.69222081,
            16.4863229,
        ],
        [0.360768335, 0.796455431, -0.766705368, 0.779336411],
    ),
];

#[test]
fn every_mode_matches_recorded_runtime_observations_including_shear_and_reflection() {
    for (mode, parent, [rotation, scale_x, scale_y, shear_x, shear_y], expected) in OBSERVED {
        let mode = Inherit::from_spine_name(mode).unwrap();
        let local = Transform {
            x: 0.,
            y: 0.,
            rotation: rotation as f32,
            scale_x: scale_x as f32,
            scale_y: scale_y as f32,
            shear_x: shear_x as f32,
            shear_y: shear_y as f32,
        };
        let actual = inherit_linear(mode, parent, local).unwrap();
        for (a, e) in actual.iter().zip(expected) {
            assert!((a - e).abs() < 2e-6, "{mode:?}: {actual:?} vs {expected:?}");
        }
    }
}

fn bone(name: &str, parent: Option<usize>, setup: Transform, inherit: Inherit) -> Bone {
    Bone {
        name: name.into(),
        parent,
        length: 10.,
        setup,
        inherit,
    }
}

fn rig(child: Inherit) -> Document {
    Document {
        constraints: vec![],
        format: FORMAT_VERSION,
        name: "Inherit".into(),
        fps: 30,
        bones: vec![
            bone("root", None, Transform::default(), Inherit::Normal),
            bone(
                "parent",
                Some(0),
                Transform {
                    rotation: 30.,
                    scale_x: 2.,
                    scale_y: 0.5,
                    ..Transform::default()
                },
                Inherit::Normal,
            ),
            bone(
                "child",
                Some(1),
                Transform {
                    x: 3.,
                    y: 4.,
                    rotation: 20.,
                    ..Transform::default()
                },
                child,
            ),
        ],
        slots: vec![],
        skins: Default::default(),
        animations: vec![Animation {
            name: "switch".into(),
            tracks: vec![],
            ik_tracks: vec![],
            physics: vec![],
            physics_resets: vec![],
            inherit_tracks: vec![InheritTrack {
                bone: 2,
                keys: vec![
                    InheritKey {
                        time: 0.5,
                        inherit: Inherit::OnlyTranslation,
                    },
                    InheritKey {
                        time: 1.,
                        inherit: Inherit::Normal,
                    },
                ],
            }],
            deform_tracks: vec![],
            attachment_tracks: vec![],
            color_tracks: vec![],
            events: vec![],
            draw_order: vec![],
            path_tracks: vec![],
            sequence_tracks: vec![],
        }],
        skin_scope: Default::default(),
        events: vec![],
        reference_scale: 100.0,
    }
}

#[test]
fn translation_is_always_inherited_and_normal_keeps_plain_composition() {
    for mode in Inherit::ALL {
        let doc = rig(mode);
        doc.validate().unwrap();
        let pose = checked_evaluate_pose(&doc, None, 0.).unwrap();
        let [parent, child] = [pose.world()[1], pose.world()[2]];
        let [x, y] = parent.point(3., 4.);
        assert!(
            (child.x - x).abs() < 1e-5 && (child.y - y).abs() < 1e-5,
            "{mode:?}"
        );
    }
    let doc = rig(Inherit::Normal);
    let locals = local_pose(&doc, None, 0.);
    let plain = world_pose(&doc, &locals);
    let expected = plain[1].then(Affine::from_transform(locals[2]));
    assert_eq!(plain[2].a, expected.a);
    assert_eq!(plain[2].d, expected.d);
}

#[test]
fn keyed_modes_are_stepped_and_setup_applies_before_the_first_key() {
    let doc = rig(Inherit::NoScale);
    let modes = |time| inherit_pose(&doc, Some(0), time)[2];
    assert_eq!(modes(0.), Inherit::NoScale);
    assert_eq!(modes(0.49), Inherit::NoScale);
    assert_eq!(modes(0.5), Inherit::OnlyTranslation);
    assert_eq!(modes(0.99), Inherit::OnlyTranslation);
    assert_eq!(modes(1.), Inherit::Normal);
    assert_eq!(modes(9.), Inherit::Normal);
    assert_eq!(inherit_pose(&doc, None, 0.7)[2], Inherit::NoScale);
    // Keyed modes reach the checked evaluation and extend duration/navigation.
    let only = checked_evaluate_pose(&doc, Some(0), 0.7).unwrap().world()[2];
    let local = Affine::from_transform(doc.bones[2].setup);
    assert!((only.a - local.a).abs() < 1e-6 && (only.d - local.d).abs() < 1e-6);
    assert_eq!(doc.animations[0].duration(), 1.);
    assert_eq!(
        adjacent_key_time(&doc, 0, 0., PlayDirection::Forward),
        Some(0.5)
    );
}

#[test]
fn degenerate_parents_are_explicit_errors_not_guesses() {
    let mut doc = rig(Inherit::NoRotationOrReflection);
    doc.bones[1].setup.scale_x = 0.;
    assert!(
        checked_evaluate_pose(&doc, None, 0.)
            .unwrap_err()
            .contains("noRotationOrReflection needs a nondegenerate parent frame")
    );
    // Verified exact down to this tiny but nonzero parent axis.
    doc.bones[1].setup.scale_x = 0.001;
    assert!(checked_evaluate_pose(&doc, None, 0.).is_ok());
    let mut doc = rig(Inherit::NoScale);
    doc.bones[1].setup.scale_x = 0.;
    doc.bones[1].setup.scale_y = 0.;
    assert!(
        checked_evaluate_pose(&doc, None, 0.)
            .unwrap_err()
            .contains("noScale needs a nondegenerate parent frame")
    );
}

#[test]
fn ik_bones_must_inherit_normally_and_the_setup_mode_is_a_command() {
    let mut doc = rig(Inherit::Normal);
    // IK separately requires equal-magnitude scales on the chain's own bones.
    doc.bones[1].setup.scale_x = 1.;
    doc.bones[1].setup.scale_y = 1.;
    doc.bones.push(bone(
        "target",
        Some(0),
        Transform {
            x: 20.,
            ..Transform::default()
        },
        Inherit::Normal,
    ));
    doc.constraints.push(Constraint::Ik(IkConstraint {
        name: "aim".into(),
        bones: vec![2],
        target: 3,
        setup: IkSetup {
            mix: 1.,
            bend_positive: true,
        },
        stretch: false,
    }));
    doc.animations[0].inherit_tracks.clear();
    assert!(checked_evaluate_pose(&doc, None, 0.).is_ok());
    let changed = apply_command(
        &doc,
        Command::SetBoneInherit {
            bone: 2,
            inherit: Inherit::NoScale,
        },
    )
    .unwrap();
    assert_eq!(changed.bones[2].inherit, Inherit::NoScale);
    assert!(
        checked_evaluate_pose(&changed, None, 0.)
            .unwrap_err()
            .contains("must inherit normally")
    );
    assert!(
        apply_command(
            &doc,
            Command::SetBoneInherit {
                bone: 99,
                inherit: Inherit::NoScale,
            },
        )
        .is_err()
    );
}
