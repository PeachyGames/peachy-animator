//! Mixed-owner inventory and fixed-base timing must preserve exact data and
//! never turn read-only timeline metadata into an editable key identity.
use crate::*;
use peachy_document::*;
use std::collections::BTreeMap;

fn reference(placeholder: &str) -> AttachmentRef {
    AttachmentRef {
        skin: "default".into(),
        slot: "body".into(),
        placeholder: placeholder.into(),
    }
}
fn time() -> f32 {
    0.25
}
fn neighbor() -> f32 {
    f32::from_bits(time().to_bits() + 2)
}
fn key(time: f32, value: f32) -> Keyframe {
    Keyframe {
        time,
        value,
        curve: Curve::BezierOffset([0.2, -0., 0.8, 0.375]),
    }
}
fn fixture() -> Document {
    let mut doc = Document::new("Original mixed-owner timing lab".into(), 60).unwrap();
    for (name, x) in [("chain", 0.), ("target", 20.)] {
        doc.bones.push(Bone {
            name: name.into(),
            parent: Some(0),
            length: 10.,
            setup: Transform {
                x,
                ..Transform::default()
            },
            inherit: Inherit::Normal,
        });
    }
    for (name, bone, attachment) in [("body", 1, Some("mesh".into())), ("extra", 0, None)] {
        doc.slots.push(Slot {
            name: name.into(),
            bone,
            attachment,
            color: WHITE,
            blend: BlendMode::Normal,
        });
    }
    let mesh = region_to_mesh(&Region {
        name: None,
        path: "blue_cutout".into(),
        width: 128.,
        height: 128.,
        transform: RegionTransform::default(),
        color: WHITE,
        sequence: None,
    });
    let mut linked = mesh.clone();
    linked.link = Some(MeshLink {
        skin: "default".into(),
        source: "mesh".into(),
        timelines: true,
    });
    doc.skins.insert(
        "default".into(),
        BTreeMap::from([(
            "body".into(),
            BTreeMap::from([
                ("mesh".into(), Attachment::Mesh(mesh)),
                ("linked".into(), Attachment::Mesh(linked)),
            ]),
        )]),
    );
    let mut slider = SliderConstraint::new("timing", 1);
    slider.setup.mix = 0.;
    doc.constraints = vec![
        Constraint::Slider(slider),
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
    ];
    doc.animations = vec![
        Animation::new(
            "host",
            vec![
                Track {
                    bone: 2,
                    property: Property::X,
                    keys: vec![key(0.375, 7.)],
                },
                Track {
                    bone: 1,
                    property: Property::Rotation,
                    keys: vec![key(time(), 17.), key(neighbor(), -23.)],
                },
            ],
        ),
        Animation::new("reference", vec![]),
    ];
    let clip = &mut doc.animations[0];
    clip.ik_tracks.push(IkTrack {
        constraint: 1,
        keys: vec![
            IkKeyframe {
                time: time(),
                mix: 0.25,
                bend_positive: false,
                curve: IkCurve::Bezier {
                    time: [0.123456789012345, 0.876543210987654],
                    mix: [0.125, 0.875],
                },
            },
            IkKeyframe {
                time: 0.75,
                mix: 0.75,
                bend_positive: true,
                curve: IkCurve::Stepped,
            },
        ],
    });
    clip.slider_tracks = SliderProperty::ALL
        .into_iter()
        .map(|property| SliderTrack {
            constraint: 0,
            property,
            keys: vec![key(
                time(),
                if property == SliderProperty::Time {
                    0.5
                } else {
                    0.25
                },
            )],
        })
        .collect();
    clip.deform_tracks.push(DeformTrack {
        attachment: reference("mesh"),
        keys: vec![
            DeformKey {
                time: time(),
                offsets: vec![-0., 1., -2., 3., -4., 5., -6., 7.],
                curve: Curve::Stepped,
            },
            DeformKey {
                time: neighbor(),
                offsets: vec![7., -6., 5., -4., 3., -2., 1., -0.],
                curve: Curve::Bezier([0.1, -0.5, 0.8, 1.25]),
            },
        ],
    });
    clip.attachment_tracks.push(AttachmentTrack {
        slot: 0,
        keys: vec![
            AttachmentKey {
                time: time(),
                attachment: Some("linked".into()),
            },
            AttachmentKey {
                time: 0.75,
                attachment: None,
            },
        ],
    });
    for slot in 0..2 {
        clip.color_tracks.push(ColorTrack {
            slot,
            keys: vec![ColorKey {
                time: time(),
                color: [20, 40, 80, 160],
                curves: [
                    Curve::Linear,
                    Curve::Bezier([0.25, -0.5, 0.75, 1.5]),
                    Curve::BezierOffset([0.2, -0., 0.8, 0.375]),
                    Curve::Linear,
                ],
            }],
        });
    }
    clip.draw_order.push(DrawOrderKey {
        time: time(),
        offsets: vec![
            DrawOrderOffset { slot: 0, offset: 0 },
            DrawOrderOffset { slot: 1, offset: 0 },
        ],
    });
    doc.validate().unwrap();
    doc
}
fn all_keys(doc: &Document) -> Vec<AnimationKey> {
    let inventory = animation_key_inventory(doc, 0).unwrap();
    assert!(inventory.unsupported_timelines.is_empty());
    animation_keys(doc, 0, &inventory.owners).unwrap().keys
}
fn pending(session: &mut Session) {
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.125,
    };
    let mut value = session.document().bones[1].setup;
    value.x = 3.5;
    session
        .edit_pose(1, target, value, &[Property::X], false)
        .unwrap();
    session
        .edit_slider_pose(
            0,
            target,
            &[
                SliderPoseEdit::Time { value: 0.875 },
                SliderPoseEdit::Mix { value: 0.625 },
            ],
            false,
        )
        .unwrap();
}
fn move_keys(keys: Vec<AnimationKey>, delta: f32, copy: bool) -> Command {
    Command::MoveAnimationKeys {
        animation: 0,
        keys,
        delta,
        copy,
    }
}

#[test]
fn multi_owner_inventory_is_authored_ordered_and_canonical_without_skin_filtering() {
    let mut doc = fixture();
    doc.skin_scope.required_bones.insert(1);
    doc.validate().unwrap();
    let expected = vec![
        KeyOwner::Bone { bone: 1 },
        KeyOwner::Bone { bone: 2 },
        KeyOwner::Slider { constraint: 0 },
        KeyOwner::Ik { constraint: 1 },
        KeyOwner::Deform {
            attachment: reference("mesh"),
        },
        KeyOwner::Slot { slot: 0 },
        KeyOwner::Slot { slot: 1 },
        KeyOwner::DrawOrder {},
    ];
    let inventory = animation_key_inventory(&doc, 0).unwrap();
    assert_eq!(inventory.owners, expected);
    assert_eq!(animation_key_owners(&doc, 0).unwrap(), expected);
    assert!(animation_keys(&doc, 0, &[]).unwrap().keys.is_empty());
    let owners = vec![
        KeyOwner::Slot { slot: 0 },
        KeyOwner::Deform {
            attachment: reference("linked"),
        },
        KeyOwner::Bone { bone: 1 },
    ];
    let query = animation_keys(&doc, 0, &owners).unwrap();
    assert_eq!(
        query
            .keys
            .iter()
            .map(AnimationKey::owner)
            .collect::<Vec<_>>(),
        vec![
            KeyOwner::Slot { slot: 0 },
            KeyOwner::Slot { slot: 0 },
            KeyOwner::Slot { slot: 0 },
            KeyOwner::Deform {
                attachment: reference("mesh")
            },
            KeyOwner::Deform {
                attachment: reference("mesh")
            },
            KeyOwner::Bone { bone: 1 },
            KeyOwner::Bone { bone: 1 }
        ]
    );
    assert_eq!(query.keys[3].time(), time());
    assert_eq!(query.keys[4].time(), neighbor());
    assert!(
        animation_keys(&doc, 0, &[KeyOwner::Bone { bone: 0 }])
            .unwrap()
            .keys
            .is_empty()
    );
    for bad in [
        vec![KeyOwner::Bone { bone: 99 }],
        vec![KeyOwner::Slot { slot: 99 }],
        vec![KeyOwner::Ik { constraint: 0 }],
        vec![KeyOwner::Slider { constraint: 1 }],
        vec![KeyOwner::Bone { bone: 1 }, KeyOwner::Bone { bone: 1 }],
        vec![
            KeyOwner::Deform {
                attachment: reference("mesh"),
            },
            KeyOwner::Deform {
                attachment: reference("linked"),
            },
        ],
    ] {
        assert!(animation_keys(&doc, 0, &bad).is_err());
    }
    assert!(animation_keys(&doc, 99, &[]).is_err());
    assert_eq!(doc.animations, fixture().animations);
}

#[test]
fn multi_owner_previews_match_raw_command_and_keep_input_order_and_exact_payloads() {
    let original = fixture();
    let mut sources = all_keys(&original);
    sources.reverse();
    for copy in [false, true] {
        let mut session = Session::new(original.clone()).unwrap();
        pending(&mut session);
        let pose = serde_json::to_value(session.pose_edits()).unwrap();
        session.begin_transaction();
        let accepted = session
            .preview_move_animation_keys(0, &sources, 0.125, copy)
            .unwrap();
        assert_eq!(
            accepted,
            sources
                .iter()
                .map(|source| source.at(source.time() + 0.125))
                .collect::<Vec<_>>()
        );
        let expected = apply_command(&original, move_keys(sources.clone(), 0.125, copy)).unwrap();
        assert_eq!(session.document(), &expected);
        assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pose);
        // Copy back the relocated owner payloads: shared-origin subtraction
        // exposes exact values/curves, including IK's sub-f32 f64 handles.
        for owner in animation_key_owners(&original, 0).unwrap() {
            let prior = animation_keys(&original, 0, std::slice::from_ref(&owner))
                .unwrap()
                .keys;
            let moved: Vec<_> = prior.iter().map(|key| key.at(key.time() + 0.125)).collect();
            assert_eq!(
                copy_animation_keys(&original, 0, &prior).unwrap(),
                copy_animation_keys(session.document(), 0, &moved).unwrap()
            );
        }
        let after = session.document().clone();
        session.commit_transaction();
        session.undo();
        assert_eq!(session.document(), &original);
        assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pose);
        session.redo();
        assert_eq!(session.document(), &after);
        assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pose);
    }
}

#[test]
fn multi_owner_failed_second_preview_keeps_last_valid_state_then_replacement_uses_original_base() {
    let original = fixture();
    let sources = all_keys(&original);
    let mut session = Session::new(original.clone()).unwrap();
    pending(&mut session);
    let pose = serde_json::to_value(session.pose_edits()).unwrap();
    session
        .dispatch(Command::RenameBone {
            bone: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    assert!(session.can_redo());
    assert!(
        session
            .preview_move_animation_keys(0, &sources, 0.125, false)
            .is_err()
    );
    session.begin_transaction();
    session
        .preview_move_animation_keys(0, &sources, 0.125, false)
        .unwrap();
    let valid = session.document().clone();
    let effects = session.last_edit_effects().clone();
    let history = (session.can_undo(), session.can_redo());
    for delta in [-1., f32::NAN, 16.] {
        assert!(
            session
                .preview_move_animation_keys(0, &sources, delta, false)
                .is_err()
        );
        assert_eq!(session.document(), &valid);
        assert_eq!(session.last_edit_effects(), &effects);
        assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pose);
        assert_eq!((session.can_undo(), session.can_redo()), history);
    }
    let accepted = session
        .preview_move_animation_keys(0, &sources, 0.25, false)
        .unwrap();
    assert_eq!(
        accepted,
        sources
            .iter()
            .map(|source| source.at(source.time() + 0.25))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        session.document(),
        &apply_command(&original, move_keys(sources, 0.25, false)).unwrap()
    );
    session.cancel_transaction();
    assert_eq!(session.document(), &original);
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pose);
    assert!(session.can_redo());
}

#[test]
fn multi_owner_collisions_and_bad_identity_failures_use_existing_atomic_timing_policy() {
    let mut original = fixture();
    original.animations[0]
        .tracks
        .iter_mut()
        .find(|track| track.bone == 1)
        .unwrap()
        .keys
        .push(key(0.75, 99.));
    original.validate().unwrap();
    let source = AnimationKey::Bone {
        bone: 1,
        property: Property::Rotation,
        time: time(),
    };
    let mut session = Session::new(original.clone()).unwrap();
    session.begin_transaction();
    session
        .preview_move_animation_keys(0, std::slice::from_ref(&source), 0.5, false)
        .unwrap();
    let keys = &session.document().animations[0]
        .tracks
        .iter()
        .find(|track| track.bone == 1)
        .unwrap()
        .keys;
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[1].value, 17.);
    assert_eq!(keys[1].time, 0.75);
    session.cancel_transaction();
    for invalid in [
        vec![source.clone(), source.clone()],
        vec![
            source.clone(),
            AnimationKey::Ik {
                constraint: 0,
                time: time(),
            },
        ],
        vec![
            source.clone(),
            AnimationKey::Bone {
                bone: 2,
                property: Property::X,
                time: time(),
            },
        ],
        vec![
            AnimationKey::Deform {
                attachment: reference("mesh"),
                time: time(),
            },
            AnimationKey::Deform {
                attachment: reference("linked"),
                time: time(),
            },
        ],
    ] {
        session.begin_transaction();
        assert!(
            session
                .preview_move_animation_keys(0, &invalid, 0.5, false)
                .is_err()
        );
        assert_eq!(session.document(), &original);
        session.cancel_transaction();
        assert!(apply_command(&original, move_keys(invalid, 0.5, false)).is_err());
    }
    session
        .dispatch(Command::RenameBone {
            bone: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    for copy in [false, true] {
        session.begin_transaction();
        session
            .preview_move_animation_keys(0, std::slice::from_ref(&source), 0., copy)
            .unwrap();
        session.commit_transaction();
        assert!(session.can_redo());
        assert_eq!(session.document(), &original);
    }
}

#[test]
fn multi_owner_structural_remaps_keep_both_constraint_kinds_and_pending_stable() {
    let original = fixture();
    let original_keys = all_keys(&original);
    let mut session = Session::new(original.clone()).unwrap();
    pending(&mut session);
    let pending_before = serde_json::to_value(session.pose_edits()).unwrap();
    session
        .dispatch(Command::CreateSliderConstraint { animation: 1 })
        .unwrap();
    let map = session
        .last_edit_effects()
        .constraint_indices
        .as_ref()
        .unwrap()
        .clone();
    let sources: Vec<_> = original_keys
        .iter()
        .map(|key| map.map_key(key).unwrap().unwrap())
        .collect();
    assert!(sources.contains(&AnimationKey::Slider {
        constraint: 1,
        property: SliderProperty::Time,
        time: time()
    }));
    assert!(sources.contains(&AnimationKey::Ik {
        constraint: 2,
        time: time()
    }));
    let after_creation = session.document().clone();
    let remapped_pose = serde_json::to_value(session.pose_edits()).unwrap();
    session.begin_transaction();
    session
        .preview_move_animation_keys(0, &sources, 0.125, false)
        .unwrap();
    session.commit_transaction();
    session.undo();
    assert_eq!(session.document(), &after_creation);
    assert_eq!(
        serde_json::to_value(session.pose_edits()).unwrap(),
        remapped_pose
    );
    session.undo();
    assert_eq!(session.document(), &original);
    assert_eq!(
        serde_json::to_value(session.pose_edits()).unwrap(),
        pending_before
    );
    session.redo();
    assert_eq!(session.document(), &after_creation);
    assert_eq!(
        animation_keys(
            session.document(),
            0,
            &animation_key_owners(session.document(), 0).unwrap()
        )
        .unwrap()
        .keys,
        sources
    );
}

#[test]
fn multi_owner_unrepresented_timelines_are_explicit_readonly_metadata_beside_editable_keys() {
    let mut doc = fixture();
    doc.constraints
        .push(Constraint::Physics(PhysicsConstraint::new("physics", 1)));
    doc.constraints.push(Constraint::Path(PathConstraint {
        name: "path".into(),
        bones: vec![1],
        slot: 0,
        position_mode: PositionMode::Percent,
        spacing_mode: SpacingMode::Fixed,
        rotate_mode: RotateMode::Tangent,
        offset_rotation: 0.,
        setup: PathSetup::default(),
    }));
    doc.events.push(EventDef::new("tick"));
    let Some(Attachment::Mesh(mesh)) = doc.attachment_mut(&reference("mesh")) else {
        unreachable!()
    };
    mesh.sequence = Some(SequenceDef {
        count: 3,
        start: 1,
        digits: 2,
        setup_index: 1,
    });
    let clip = &mut doc.animations[0];
    clip.inherit_tracks.push(InheritTrack {
        bone: 1,
        keys: vec![InheritKey {
            time: neighbor(),
            inherit: Inherit::NoScale,
        }],
    });
    clip.sequence_tracks.push(SequenceTrack {
        attachment: reference("mesh"),
        keys: vec![SequenceKey {
            time: time(),
            mode: SequenceMode::Hold,
            index: 1,
            delay: 0.25,
        }],
    });
    clip.path_tracks.push(PathTrack {
        constraint: 3,
        property: PathProperty::Position,
        keys: vec![key(time(), 0.25)],
    });
    clip.physics = vec![
        PhysicsTrack {
            constraint: Some(2),
            property: PhysicsProperty::Wind,
            keys: vec![key(time(), 1.)],
        },
        PhysicsTrack {
            constraint: None,
            property: PhysicsProperty::Gravity,
            keys: vec![key(time(), 2.)],
        },
    ];
    clip.physics_resets = vec![
        PhysicsReset {
            constraint: Some(2),
            times: vec![time()],
        },
        PhysicsReset {
            constraint: None,
            times: vec![neighbor()],
        },
    ];
    clip.events = vec![EventKey {
        time: time(),
        event: 0,
        int: None,
        float: None,
        string: None,
        volume: None,
        balance: None,
    }];
    doc.validate().unwrap();
    let before = doc.clone();
    let inventory = animation_key_inventory(&doc, 0).unwrap();
    assert_eq!(
        inventory.owners,
        animation_key_owners(&fixture(), 0).unwrap()
    );
    assert_eq!(inventory.unsupported_timelines.len(), 8);
    assert!(
        inventory
            .unsupported_timelines
            .iter()
            .all(|track| !track.times().is_empty())
    );
    let bone = animation_keys(&doc, 0, &[KeyOwner::Bone { bone: 1 }]).unwrap();
    assert_eq!(bone.keys.len(), 2);
    assert_eq!(
        bone.unsupported_timelines,
        vec![UnsupportedAnimationTimeline::Inherit {
            bone: 1,
            times: vec![f64::from(neighbor())]
        }]
    );
    let mesh = animation_keys(
        &doc,
        0,
        &[KeyOwner::Deform {
            attachment: reference("linked"),
        }],
    )
    .unwrap();
    assert_eq!(mesh.keys.len(), 2);
    assert_eq!(mesh.unsupported_timelines.len(), 1);
    assert!(
        matches!(&mesh.unsupported_timelines[0], UnsupportedAnimationTimeline::Sequence { attachment, .. } if attachment == &reference("mesh"))
    );
    assert_eq!(
        animation_keys(&doc, 0, &[KeyOwner::Slot { slot: 0 }])
            .unwrap()
            .unsupported_timelines,
        mesh.unsupported_timelines
    );
    assert!(
        animation_keys(&doc, 0, &[])
            .unwrap()
            .unsupported_timelines
            .is_empty()
    );
    let moved = apply_command(&doc, move_keys(bone.keys, 0.125, false)).unwrap();
    assert_eq!(
        moved.animations[0].inherit_tracks,
        doc.animations[0].inherit_tracks
    );
    assert_eq!(moved.animations[0].physics, doc.animations[0].physics);
    assert_eq!(
        moved.animations[0].sequence_tracks,
        doc.animations[0].sequence_tracks
    );
    assert_eq!(doc, before);
}
