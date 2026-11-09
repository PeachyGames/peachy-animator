//! Physics data commands and the explicit no-simulation evaluation boundary.
use crate::*;
use peachy_document::*;

fn bone(name: &str, parent: Option<usize>) -> Bone {
    Bone {
        name: name.into(),
        parent,
        length: 40.,
        setup: Transform {
            x: 10.,
            rotation: 15.,
            ..Transform::default()
        },
    }
}

fn key(time: f32, value: f32) -> Keyframe {
    Keyframe {
        time,
        value,
        curve: Curve::Linear,
    }
}

fn document() -> Document {
    let mut sway = PhysicsConstraint::new("sway", 1);
    sway.influence.rotate = 1.;
    sway.influence.x = 0.5;
    Document {
        constraints: vec![Constraint::Physics(sway)],
        format: FORMAT_VERSION,
        name: "Physics".into(),
        fps: 30,
        bones: vec![bone("root", None), bone("tail", Some(0))],
        slots: vec![],
        skins: Default::default(),
        animations: vec![Animation {
            name: "swing".into(),
            tracks: vec![Track {
                bone: 0,
                property: Property::Rotation,
                keys: vec![key(0., 0.), key(1., 40.)],
            }],
            physics: vec![PhysicsTrack {
                constraint: Some(0),
                property: PhysicsProperty::Strength,
                keys: vec![key(0.25, 50.), key(1.75, 80.)],
            }],
            physics_resets: vec![PhysicsReset {
                constraint: None,
                times: vec![0.5, 2.],
            }],
            ik_tracks: vec![],
        }],
    }
}

#[test]
fn physics_leaves_authored_and_applied_poses_unchanged_without_simulation() {
    let doc = document();
    doc.validate().unwrap();
    let mut bare = doc.clone();
    bare.constraints.clear();
    bare.animations[0].physics.clear();
    bare.animations[0].physics_resets.clear();
    for time in [0., 0.3, 1., 2.] {
        let with = checked_evaluate_pose(&doc, Some(0), time).unwrap();
        let without = checked_evaluate_pose(&bare, Some(0), time).unwrap();
        assert_eq!(with.authored_locals(), without.authored_locals());
        assert_eq!(with.applied_locals(), without.applied_locals());
    }
}

#[test]
fn physics_and_reset_keys_extend_duration_and_key_navigation() {
    let doc = document();
    assert_eq!(doc.animations[0].duration(), 2.);
    assert_eq!(last_key_time(&doc, 0), 2.);
    let forward: Vec<_> = std::iter::successors(Some(0.), |&time| {
        adjacent_key_time(&doc, 0, time, PlayDirection::Forward)
    })
    .collect();
    assert_eq!(forward, [0., 0.25, 0.5, 1., 1.75, 2.]);
    assert_eq!(
        adjacent_key_time(&doc, 0, 2., PlayDirection::Backward),
        Some(1.75)
    );
}

#[test]
fn physics_commands_are_atomic_typed_and_validated() {
    let doc = document();
    let mut edited = doc.constraints[0].as_physics().unwrap().clone();
    edited.name = "renamed".into();
    edited.setup.mass = 3.;
    edited.global.wind = true;
    let next = apply_command(
        &doc,
        Command::SetPhysicsConstraint {
            constraint: 0,
            value: edited.clone(),
        },
    )
    .unwrap();
    assert_eq!(next.constraints[0], Constraint::Physics(edited.clone()));
    // Keyed timelines keep their index reference through a rename.
    assert_eq!(next.animations[0].physics[0].constraint, Some(0));

    let mut invalid = edited.clone();
    invalid.setup.mass = 0.;
    for bad in [
        Command::SetPhysicsConstraint {
            constraint: 0,
            value: invalid.clone(),
        },
        Command::SetPhysicsConstraint {
            constraint: 9,
            value: edited.clone(),
        },
        Command::AddPhysicsConstraint {
            constraint: PhysicsConstraint::new("sway", 0),
        },
        Command::AddPhysicsConstraint {
            constraint: PhysicsConstraint::new("ghost", 7),
        },
        Command::AddIkConstraint {
            constraint: IkConstraint {
                name: "sway".into(),
                bones: vec![1],
                target: 0,
                setup: IkSetup {
                    mix: 1.,
                    bend_positive: true,
                },
            },
        },
    ] {
        assert!(apply_command(&doc, bad).is_err());
    }
    // An IK edit can never silently replace a physics constraint, or vice versa.
    assert!(
        apply_command(
            &doc,
            Command::SetIkConstraint {
                constraint: 0,
                value: IkConstraint {
                    name: "ik".into(),
                    bones: vec![1],
                    target: 0,
                    setup: IkSetup {
                        mix: 1.,
                        bend_positive: true,
                    },
                },
            },
        )
        .unwrap_err()
        .contains("Unknown IK constraint")
    );
    let added = apply_command(
        &doc,
        Command::AddPhysicsConstraint {
            constraint: PhysicsConstraint::new("second", 0),
        },
    )
    .unwrap();
    assert_eq!(added.constraints.len(), 2);
    assert_eq!(added.constraints[1].name(), "second");
}

#[test]
fn physics_timelines_must_reference_physics_constraints_once() {
    let mut doc = document();
    doc.constraints.push(Constraint::Ik(IkConstraint {
        name: "aim".into(),
        bones: vec![1],
        target: 0,
        setup: IkSetup {
            mix: 0.,
            bend_positive: true,
        },
    }));
    doc.validate().unwrap();
    let mut wrong_kind = doc.clone();
    wrong_kind.animations[0].physics[0].constraint = Some(1);
    assert!(wrong_kind.validate().is_err());
    let mut duplicate = doc.clone();
    let copy = duplicate.animations[0].physics[0].clone();
    duplicate.animations[0].physics.push(copy);
    assert!(duplicate.validate().is_err());
    let mut unordered = doc;
    unordered.animations[0].physics_resets[0].times = vec![1., 0.5];
    assert!(unordered.validate().is_err());
}
