use crate::*;
use peachy_document::*;
use std::collections::BTreeMap;

fn rig() -> Document {
    Document {
        format: FORMAT_VERSION,
        name: "Original normal IK fixture".into(),
        fps: 30,
        bones: vec![
            Bone {
                name: "root".into(),
                parent: None,
                length: 0.,
                setup: Transform::default(),
            },
            Bone {
                name: "upper".into(),
                parent: Some(0),
                length: 10.,
                setup: Transform::default(),
            },
            Bone {
                name: "lower".into(),
                parent: Some(1),
                length: 10.,
                setup: Transform {
                    x: 10.,
                    ..Transform::default()
                },
            },
            Bone {
                name: "target".into(),
                parent: Some(0),
                length: 0.,
                setup: Transform {
                    x: 10.,
                    y: 10.,
                    ..Transform::default()
                },
            },
        ],
        constraints: vec![],
        slots: vec![],
        skins: BTreeMap::new(),
        animations: vec![Animation {
            name: "reach".into(),
            tracks: vec![],
        }],
    }
}

fn constraint() -> IkConstraint {
    IkConstraint {
        name: "limb".into(),
        bones: vec![1, 2],
        target: 3,
        setup: IkSetup {
            mix: 1.,
            bend_positive: true,
        },
    }
}

#[test]
fn evaluated_draw_axes_use_the_supplied_world_without_recomposing_authored_rotation() {
    let local = Transform::default();
    let parent = Affine::from_transform(local);
    let world = Affine::from_transform(Transform {
        rotation: 90.,
        ..local
    });
    for tool in [TransformTool::Scale, TransformTool::Shear] {
        let axes = transform_axes_from_world(local, parent, world, Axes::World, tool);
        assert!(axes[0][0].abs() < 0.0001 && (axes[0][1] - 1.).abs() < 0.0001);
        assert!((axes[1][0] + 1.).abs() < 0.0001 && axes[1][1].abs() < 0.0001);
    }
    assert_eq!(
        transform_axes_from_world(local, parent, world, Axes::World, TransformTool::Translate),
        [[1., 0.], [0., 1.]]
    );
}

#[test]
fn native6_requires_strict_constraint_and_setup_data() {
    let mut document = rig();
    document.constraints.push(Constraint::Ik(constraint()));
    let value = serde_json::to_value(&document).unwrap();
    let decoded: Document = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(decoded, document);
    decoded.validate().unwrap();
    for path in [
        vec!["constraints"],
        vec!["constraints", "setup"],
        vec!["constraints", "mix"],
        vec!["constraints", "bend_positive"],
    ] {
        let mut invalid = value.clone();
        if path.len() == 1 {
            invalid.as_object_mut().unwrap().remove("constraints");
        } else if path[1] == "setup" {
            invalid["constraints"][0]
                .as_object_mut()
                .unwrap()
                .remove("setup");
        } else {
            invalid["constraints"][0]["setup"]
                .as_object_mut()
                .unwrap()
                .remove(path[1]);
        }
        assert!(serde_json::from_value::<Document>(invalid).is_err());
    }
    for field in ["order", "enabled", "softness"] {
        let mut invalid = value.clone();
        invalid["constraints"][0][field] = 0.into();
        assert!(serde_json::from_value::<Document>(invalid).is_err());
    }
    let mut invalid = value;
    invalid["constraints"][0]["setup"]["stretch"] = false.into();
    assert!(serde_json::from_value::<Document>(invalid).is_err());
}

#[test]
fn constraint_commands_validate_structure_atomically_and_preserve_order() {
    let document = rig();
    let first = apply_command(
        &document,
        Command::AddIkConstraint {
            constraint: constraint(),
        },
    )
    .unwrap();
    let mut other = constraint();
    other.name = "second".into();
    other.bones = vec![1];
    let second = apply_command(
        &first,
        Command::AddIkConstraint {
            constraint: other.clone(),
        },
    )
    .unwrap();
    assert_eq!(
        second.constraints,
        vec![Constraint::Ik(constraint()), Constraint::Ik(other)]
    );
    for bones in [vec![], vec![1, 1], vec![2, 1], vec![0, 1, 2], vec![99]] {
        let mut invalid = constraint();
        invalid.bones = bones;
        assert!(
            apply_command(
                &first,
                Command::SetIkConstraint {
                    constraint: 0,
                    value: invalid
                }
            )
            .is_err()
        );
    }
    for target in [1, 2, 99] {
        let mut invalid = constraint();
        invalid.target = target;
        assert!(
            apply_command(
                &first,
                Command::SetIkConstraint {
                    constraint: 0,
                    value: invalid
                }
            )
            .is_err()
        );
    }
    for mix in [-0.1, 1.1, f32::NAN, f32::INFINITY] {
        let mut invalid = constraint();
        invalid.setup.mix = mix;
        assert!(
            apply_command(
                &first,
                Command::SetIkConstraint {
                    constraint: 0,
                    value: invalid
                }
            )
            .is_err()
        );
    }
    assert!(
        apply_command(
            &first,
            Command::AddIkConstraint {
                constraint: constraint()
            }
        )
        .is_err()
    );
    assert!(
        apply_command(
            &first,
            Command::SetIkConstraint {
                constraint: 99,
                value: constraint()
            }
        )
        .is_err()
    );
    assert_eq!(first.constraints, vec![Constraint::Ik(constraint())]);
    let command = Command::SetIkConstraint {
        constraint: 0,
        value: constraint(),
    };
    let mut payload = serde_json::to_value(command).unwrap();
    payload["value"]["setup"]["uniform"] = true.into();
    assert!(serde_json::from_value::<Command>(payload).is_err());
}

#[test]
fn checked_fk_and_disabled_ik_keep_authored_bits_and_reject_invalid_queries() {
    let mut document = rig();
    document.bones[1].setup.scale_x = -0.0;
    document.bones[1].setup.shear_y = 35.;
    let mut disabled = constraint();
    disabled.setup.mix = 0.;
    document.constraints.push(Constraint::Ik(disabled));
    document.validate().unwrap();
    let before = document.clone();
    let authored = local_pose(&document, Some(0), 0.137);
    let expected = world_pose(&document, &authored);
    let evaluated = checked_evaluate_pose(&document, Some(0), 0.137).unwrap();
    assert_eq!(evaluated.authored_locals(), authored);
    assert_eq!(evaluated.applied_locals(), authored);
    for (actual, expected) in evaluated.world().iter().zip(expected) {
        assert_eq!(
            [actual.a, actual.b, actual.c, actual.d, actual.x, actual.y].map(f32::to_bits),
            [
                expected.a, expected.b, expected.c, expected.d, expected.x, expected.y
            ]
            .map(f32::to_bits)
        );
    }
    assert!(checked_evaluate_pose(&document, Some(99), 0.).is_err());
    assert!(checked_evaluate_pose(&document, None, f32::NAN).is_err());
    assert!(checked_evaluate_pose(&document, None, -0.1).is_err());
    assert!(checked_pose_from_locals(&document, vec![]).is_err());
    assert_eq!(document, before);
    let mut session = Session::new(document).unwrap();
    let mut target = session.local_pose(Some(0), 0.137)[3];
    target.x = 12.5;
    session
        .edit_pose(
            3,
            EditTarget::Animate {
                animation: 0,
                time: 0.137,
            },
            target,
            &[Property::X],
            false,
        )
        .unwrap();
    let pending = session.pose_edits().clone();
    let posed = session
        .evaluated_pose(EditTarget::Animate {
            animation: 0,
            time: 0.137,
        })
        .unwrap();
    assert_eq!(posed.authored_locals()[3].x, 12.5);
    assert_eq!(
        session
            .mesh_bind_pose(EditTarget::Animate {
                animation: 0,
                time: 0.137
            })
            .unwrap()
            .locals,
        posed.applied_locals()
    );
    assert_eq!(session.pose_edits(), &pending);
    assert!(!session.is_dirty());
}
