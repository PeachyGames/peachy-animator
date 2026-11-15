//! Creation must preserve identities across every modeled reference domain.
use crate::*;
use peachy_document::*;
use std::collections::{BTreeMap, BTreeSet};

fn key(time: f32, value: f32) -> Keyframe {
    Keyframe {
        time,
        value,
        curve: Curve::BezierOffset([0.2, -0., 0.8, 0.75]),
    }
}
fn time() -> f32 {
    f32::from_bits(0.125f32.to_bits() + 1)
}
fn target() -> EditTarget {
    EditTarget::Animate {
        animation: 1,
        time: time(),
    }
}
fn document() -> Document {
    let mut doc = Document::new("Original Slider creation".into(), 60).unwrap();
    for name in ["chain", "target"] {
        doc.bones.push(Bone {
            name: name.into(),
            parent: Some(0),
            length: 10.,
            setup: Transform::default(),
            inherit: Inherit::Normal,
        });
    }
    doc.slots.push(Slot {
        name: "path slot".into(),
        bone: 0,
        attachment: None,
        color: WHITE,
        blend: BlendMode::Normal,
    });
    doc.animations = vec![
        Animation::new("calibration", vec![]),
        Animation::new("host", vec![]),
    ];
    doc.constraints = vec![
        Constraint::Ik(IkConstraint {
            name: "aim".into(),
            bones: vec![1],
            target: 2,
            setup: IkSetup {
                mix: 0.5,
                bend_positive: true,
            },
            stretch: false,
        }),
        Constraint::Physics(PhysicsConstraint::new("motion", 1)),
        Constraint::Transform(TransformConstraint {
            name: "follow".into(),
            bones: vec![1],
            source: 2,
            offsets: TransformValues::default(),
            mix: TransformValues::default(),
            mappings: vec![],
            local_source: false,
            local_target: false,
            additive: false,
            clamp: false,
        }),
        Constraint::Path(PathConstraint {
            name: "ride".into(),
            bones: vec![1],
            slot: 0,
            position_mode: PositionMode::Percent,
            spacing_mode: SpacingMode::Fixed,
            rotate_mode: RotateMode::Tangent,
            offset_rotation: 0.,
            setup: PathSetup::default(),
        }),
        Constraint::Slider(SliderConstraint::new("existing", 0)),
    ];
    let clip = &mut doc.animations[1];
    clip.ik_tracks.push(IkTrack {
        constraint: 0,
        keys: vec![IkKeyframe {
            time: time(),
            mix: 0.25,
            bend_positive: false,
            curve: IkCurve::Linear,
        }],
    });
    clip.physics = vec![
        PhysicsTrack {
            constraint: Some(1),
            property: PhysicsProperty::Wind,
            keys: vec![key(time(), -0.)],
        },
        PhysicsTrack {
            constraint: None,
            property: PhysicsProperty::Gravity,
            keys: vec![key(time(), 0.75)],
        },
    ];
    clip.physics_resets = vec![
        PhysicsReset {
            constraint: Some(1),
            times: vec![time()],
        },
        PhysicsReset {
            constraint: None,
            times: vec![time()],
        },
    ];
    clip.path_tracks.push(PathTrack {
        constraint: 3,
        property: PathProperty::Position,
        keys: vec![key(time(), -2.25)],
    });
    clip.slider_tracks = SliderProperty::ALL
        .into_iter()
        .map(|property| SliderTrack {
            constraint: 4,
            property,
            keys: vec![key(
                time(),
                if property == SliderProperty::Time {
                    -0.
                } else {
                    0.25
                },
            )],
        })
        .collect();
    doc.skin_scope.required_constraints = BTreeSet::from([0, 1, 2, 3, 4]);
    doc.skins.insert("awake".into(), BTreeMap::new());
    doc.skin_scope.skins.insert(
        "awake".into(),
        SkinMembers {
            bones: vec![],
            constraints: vec![4, 1, 0, 3, 2],
        },
    );
    doc.validate().unwrap();
    doc
}
fn pending(session: &mut Session) {
    session
        .edit_ik_pose(0, target(), &[IkPoseEdit::Mix { value: 0.375 }], false)
        .unwrap();
    session
        .edit_slider_pose(4, target(), &[SliderPoseEdit::Mix { value: 0.625 }], false)
        .unwrap();
    let mut local = session.document().bones[1].setup;
    local.x = 7.25;
    session
        .edit_pose(1, target(), local, &[Property::X], false)
        .unwrap();
}

#[test]
fn slider_creation_prepends_defaults_and_remaps_all_persistent_references_without_reencoding_keys()
{
    let before = document();
    let outcome =
        apply_command_with_effects(&before, Command::CreateSliderConstraint { animation: 0 })
            .unwrap();
    let after = &outcome.document;
    assert_eq!(after.format, before.format);
    assert_eq!(
        after.constraints[0],
        Constraint::Slider(SliderConstraint::new("calibration", 0))
    );
    assert_eq!(&after.constraints[1..], before.constraints.as_slice());
    let map = outcome.effects.constraint_indices.unwrap();
    assert_eq!(
        map.old_to_new(),
        [Some(1), Some(2), Some(3), Some(4), Some(5)]
    );
    assert_eq!(map.new_len(), 6);
    let old = &before.animations[1];
    let new = &after.animations[1];
    assert_eq!(new.ik_tracks[0].constraint, 1);
    assert_eq!(new.ik_tracks[0].keys, old.ik_tracks[0].keys);
    assert_eq!(new.physics[0].constraint, Some(2));
    assert_eq!(new.physics[1], old.physics[1]);
    assert_eq!(new.physics[0].keys, old.physics[0].keys);
    assert_eq!(
        new.physics[0].keys[0].value.to_bits(),
        old.physics[0].keys[0].value.to_bits()
    );
    assert_eq!(new.physics_resets[0].constraint, Some(2));
    assert_eq!(new.physics_resets[0].times, old.physics_resets[0].times);
    assert_eq!(new.physics_resets[1], old.physics_resets[1]);
    assert_eq!(new.path_tracks[0].constraint, 4);
    assert_eq!(new.path_tracks[0].keys, old.path_tracks[0].keys);
    for (new, old) in new.slider_tracks.iter().zip(&old.slider_tracks) {
        assert_eq!(new.constraint, 5);
        assert_eq!(new.keys, old.keys);
        assert_eq!(new.property, old.property);
        assert_eq!(new.keys[0].time.to_bits(), time().to_bits());
        assert_eq!(
            serde_json::to_string(&new.keys).unwrap(),
            serde_json::to_string(&old.keys).unwrap()
        );
    }
    assert_eq!(
        after.skin_scope.required_constraints,
        BTreeSet::from([1, 2, 3, 4, 5])
    );
    assert_eq!(after.skin_scope.skins["awake"].constraints, [5, 2, 1, 4, 3]);
    assert_eq!(after.bones, before.bones);
    assert_eq!(after.slots, before.slots);
    assert_eq!(after.skins, before.skins);
    assert_eq!(after.animations[0], before.animations[0]);
    assert_eq!(before, document());
}

#[test]
fn slider_creation_session_pending_typed_refs_clipboard_and_history_keep_original_owners() {
    let mut session = Session::new(document()).unwrap();
    pending(&mut session);
    let before = session.document().clone();
    let pending_before = serde_json::to_value(session.pose_edits()).unwrap();
    let original = AnimationKey::Slider {
        constraint: 4,
        property: SliderProperty::Time,
        time: time(),
    };
    let clipboard = copy_animation_keys(&before, 1, std::slice::from_ref(&original)).unwrap();
    session
        .dispatch(Command::CreateSliderConstraint { animation: 0 })
        .unwrap();
    let map = session
        .last_edit_effects()
        .constraint_indices
        .as_ref()
        .unwrap();
    let shifted = map.map_key(&original).unwrap().unwrap();
    assert_eq!(
        shifted,
        AnimationKey::Slider {
            constraint: 5,
            property: SliderProperty::Time,
            time: time()
        }
    );
    assert_eq!(
        map.map_owner(&KeyOwner::Ik { constraint: 0 }).unwrap(),
        Some(KeyOwner::Ik { constraint: 1 })
    );
    assert_eq!(
        clipboard,
        copy_animation_keys(session.document(), 1, &[shifted]).unwrap()
    );
    assert!(session.pose_edits().has_ik(1, 1));
    assert!(session.pose_edits().has_slider(1, 5, SliderProperty::Mix));
    let posed = session.authored_pose(target()).unwrap();
    assert_eq!(posed.ik_poses()[1].mix, 0.375);
    assert_eq!(posed.slider_poses()[5].mix, 0.625);
    assert_eq!(posed.bone_locals()[1].x, 7.25);
    let created = session.document().clone();
    let receipt = session.last_edit_effects().clone();
    let pending_after = serde_json::to_value(session.pose_edits()).unwrap();
    assert!(
        session
            .dispatch(Command::CreateSliderConstraint {
                animation: usize::MAX
            })
            .is_err()
    );
    assert_eq!(session.document(), &created);
    assert_eq!(session.last_edit_effects(), &receipt);
    session.undo();
    let inverse = session
        .last_edit_effects()
        .constraint_indices
        .as_ref()
        .unwrap();
    assert_eq!(inverse.map_index(0).unwrap(), None);
    assert_eq!(inverse.map_index(1).unwrap(), Some(0));
    assert_eq!(inverse.map_index(5).unwrap(), Some(4));
    assert_eq!(session.document(), &before);
    assert_eq!(
        serde_json::to_value(session.pose_edits()).unwrap(),
        pending_before
    );
    assert!(!session.is_dirty());
    let inverse_receipt = session.last_edit_effects().clone();
    let history = (session.can_undo(), session.can_redo());
    assert!(
        session
            .dispatch(Command::Batch {
                commands: vec![
                    Command::CreateSliderConstraint { animation: 0 },
                    Command::SetIkKey {
                        animation: 1,
                        constraint: 0,
                        key: IkKeyframe {
                            time: 0.,
                            mix: 0.25,
                            bend_positive: true,
                            curve: IkCurve::Linear
                        }
                    },
                ]
            })
            .is_err()
    );
    assert_eq!(session.document(), &before);
    assert_eq!(
        serde_json::to_value(session.pose_edits()).unwrap(),
        pending_before
    );
    assert_eq!(session.last_edit_effects(), &inverse_receipt);
    assert_eq!((session.can_undo(), session.can_redo()), history);
    session.redo();
    assert_eq!(session.document(), &created);
    assert_eq!(session.last_edit_effects(), &receipt);
    assert_eq!(
        serde_json::to_value(session.pose_edits()).unwrap(),
        pending_after
    );
    let root_name = session.document().bones[0].name.clone();
    session
        .dispatch(Command::RenameBone {
            bone: 0,
            name: root_name,
        })
        .unwrap();
    assert!(session.last_edit_effects().constraint_indices.is_none());
}

#[test]
fn slider_creation_nested_batches_compose_prepend_and_raw_append_with_sequential_command_indices() {
    let before = document();
    let outcome = apply_command_with_effects(
        &before,
        Command::Batch {
            commands: vec![
                Command::CreateSliderConstraint { animation: 0 },
                Command::Batch {
                    commands: vec![
                        Command::AddSliderConstraint {
                            constraint: SliderConstraint::new("tail", 0),
                        },
                        Command::CreateSliderConstraint { animation: 0 },
                    ],
                },
                Command::SetSliderKey {
                    animation: 1,
                    constraint: 7,
                    property: SliderProperty::Mix,
                    key: key(0., 0.875),
                },
            ],
        },
    )
    .unwrap();
    let map = outcome.effects.constraint_indices.as_ref().unwrap();
    assert_eq!(
        map.old_to_new(),
        [Some(2), Some(3), Some(4), Some(5), Some(6)]
    );
    assert_eq!(map.new_len(), 8);
    assert_eq!(outcome.document.constraints[0].name(), "calibration2");
    assert_eq!(outcome.document.constraints[1].name(), "calibration");
    assert_eq!(outcome.document.constraints[7].name(), "tail");
    assert!(
        outcome.document.animations[1]
            .slider_tracks
            .iter()
            .any(|track| track.constraint == 7 && track.keys[0].value == 0.875)
    );
    let inverse = map.inverse();
    assert_eq!(inverse.map_index(0).unwrap(), None);
    assert_eq!(inverse.map_index(1).unwrap(), None);
    assert_eq!(inverse.map_index(7).unwrap(), None);
    assert_eq!(inverse.map_index(2).unwrap(), Some(0));
    let appended = apply_command_with_effects(
        &before,
        Command::AddSliderConstraint {
            constraint: SliderConstraint::new("tail", 0),
        },
    )
    .unwrap();
    assert_eq!(
        &appended.document.constraints[..5],
        before.constraints.as_slice()
    );
    assert_eq!(appended.document.animations, before.animations);
    assert_eq!(
        appended.effects.constraint_indices.unwrap().old_to_new(),
        [Some(0), Some(1), Some(2), Some(3), Some(4)]
    );
}

#[test]
fn slider_creation_fixed_baseline_previews_cancel_and_commit_have_exact_inverse_receipts() {
    let mut session = Session::new(document()).unwrap();
    pending(&mut session);
    let original = session.document().clone();
    let pose = serde_json::to_value(session.pose_edits()).unwrap();
    session.begin_transaction();
    session
        .preview_transaction(Command::CreateSliderConstraint { animation: 0 })
        .unwrap();
    session
        .preview_transaction(Command::Batch {
            commands: vec![
                Command::CreateSliderConstraint { animation: 0 },
                Command::CreateSliderConstraint { animation: 0 },
            ],
        })
        .unwrap();
    let replacement = session
        .last_edit_effects()
        .constraint_indices
        .as_ref()
        .unwrap();
    assert_eq!(replacement.map_index(0).unwrap(), None);
    assert_eq!(replacement.map_index(1).unwrap(), Some(2));
    let prior = session.document().clone();
    let receipt = session.last_edit_effects().clone();
    assert!(
        session
            .preview_transaction(Command::CreateSliderConstraint {
                animation: usize::MAX
            })
            .is_err()
    );
    assert_eq!(session.document(), &prior);
    assert_eq!(session.last_edit_effects(), &receipt);
    session.cancel_transaction();
    assert_eq!(session.document(), &original);
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pose);
    let cancelled = session
        .last_edit_effects()
        .constraint_indices
        .as_ref()
        .unwrap();
    assert_eq!(cancelled.map_index(0).unwrap(), None);
    assert_eq!(cancelled.map_index(1).unwrap(), None);
    assert_eq!(cancelled.map_index(2).unwrap(), Some(0));
    session.begin_transaction();
    session
        .dispatch(Command::CreateSliderConstraint { animation: 0 })
        .unwrap();
    session
        .dispatch(Command::CreateSliderConstraint { animation: 0 })
        .unwrap();
    session
        .dispatch(Command::RenameBone {
            bone: 0,
            name: "renamed".into(),
        })
        .unwrap();
    session.commit_transaction();
    assert!(session.last_edit_effects().constraint_indices.is_none());
    let after = session.document().clone();
    session.undo();
    assert_eq!(session.document(), &original);
    assert_eq!(
        session
            .last_edit_effects()
            .constraint_indices
            .as_ref()
            .unwrap()
            .map_index(2)
            .unwrap(),
        Some(0)
    );
    session.redo();
    assert_eq!(session.document(), &after);
    assert_eq!(
        session
            .last_edit_effects()
            .constraint_indices
            .as_ref()
            .unwrap()
            .map_index(0)
            .unwrap(),
        Some(2)
    );
}

#[test]
fn slider_creation_bad_batches_and_wire_fields_are_atomic_and_successful_identity_resets_receipts()
{
    let mut empty = Document::new("Empty constraints".into(), 60).unwrap();
    empty.animations.push(Animation::new("calibration", vec![]));
    let mut empty_session = Session::new(empty).unwrap();
    empty_session
        .dispatch(Command::CreateSliderConstraint { animation: 0 })
        .unwrap();
    assert_eq!(
        empty_session
            .last_edit_effects()
            .constraint_indices
            .as_ref()
            .unwrap()
            .old_to_new(),
        []
    );
    empty_session.undo();
    assert_eq!(
        empty_session
            .last_edit_effects()
            .constraint_indices
            .as_ref()
            .unwrap()
            .map_index(0)
            .unwrap(),
        None
    );
    let before = document();
    let bad = Command::Batch {
        commands: vec![
            Command::CreateSliderConstraint { animation: 0 },
            Command::SetIkKey {
                animation: 1,
                constraint: 0,
                key: IkKeyframe {
                    time: 0.,
                    mix: 0.25,
                    bend_positive: true,
                    curve: IkCurve::Linear,
                },
            },
        ],
    };
    assert!(apply_command_with_effects(&before, bad).is_err());
    assert_eq!(before, document());
    assert!(
        serde_json::from_value::<Command>(
            serde_json::json!({"command":"createSliderConstraint","animation":0,"name":"override"})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<Command>(serde_json::json!({"command":"createSliderConstraint"}))
            .is_err()
    );
    let mut session = Session::new(before).unwrap();
    session.undo();
    assert!(session.last_edit_effects().constraint_indices.is_none());
    session.redo();
    assert!(session.last_edit_effects().constraint_indices.is_none());
    session
        .dispatch(Command::CreateSliderConstraint { animation: 0 })
        .unwrap();
    let map = session
        .last_edit_effects()
        .constraint_indices
        .as_ref()
        .unwrap();
    assert!(map.map_index(99).is_err());
    assert_eq!(
        map.map_key(&AnimationKey::DrawOrder { time: time() })
            .unwrap(),
        Some(AnimationKey::DrawOrder { time: time() })
    );
    session
        .dispatch(Command::Batch { commands: vec![] })
        .unwrap();
    assert!(session.last_edit_effects().constraint_indices.is_none());
}

#[test]
fn slider_creation_numbered_stems_increment_past_the_stem_without_filling_lower_gaps() {
    // Original native calibration2 collision =>calibration4; calibration4
    // collision with calibration2 free =>calibration5, not2 or42.
    for (stem, existing, expected) in [
        (
            "calibration2",
            vec!["calibration", "calibration2", "calibration3"],
            "calibration4",
        ),
        (
            "calibration4",
            vec!["calibration", "calibration3", "calibration4", "gap"],
            "calibration5",
        ),
    ] {
        let mut doc = document();
        doc.animations[0].name = stem.into();
        for name in existing {
            doc.constraints
                .push(Constraint::Slider(SliderConstraint::new(name, 0)));
        }
        doc.validate().unwrap();
        let after = apply_command(&doc, Command::CreateSliderConstraint { animation: 0 }).unwrap();
        assert_eq!(after.constraints[0].name(), expected);
        assert_eq!(&after.constraints[1..], doc.constraints.as_slice());
    }
}
