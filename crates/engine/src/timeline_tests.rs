use crate::{
    Command, EditTarget, KeyClipboard, Session, TimelineKey, apply_command, copy_keys, keyframe_at,
};
use peachy_document::*;
use std::collections::BTreeMap;

fn key(time: f32, value: f32, curve: Curve) -> Keyframe {
    Keyframe { time, value, curve }
}
fn reference(property: Property, time: f32) -> TimelineKey {
    TimelineKey {
        bone: 0,
        property,
        time,
    }
}
fn fixture() -> Document {
    Document {
        constraints: vec![],
        format: FORMAT_VERSION,
        name: "Timeline fixture".into(),
        fps: 30,
        bones: vec![
            Bone {
                name: "root".into(),
                parent: None,
                length: 10.,
                setup: Transform::default(),
                inherit: peachy_document::Inherit::Normal,
            },
            Bone {
                name: "child".into(),
                parent: Some(0),
                length: 5.,
                setup: Transform {
                    rotation: 90.,
                    ..Default::default()
                },
                inherit: peachy_document::Inherit::Normal,
            },
        ],
        slots: vec![],
        skins: BTreeMap::new(),
        animations: vec![
            Animation {
                ik_tracks: vec![],
                name: "source".into(),

                tracks: vec![
                    Track {
                        bone: 0,
                        property: Property::Rotation,
                        keys: vec![
                            key(0.12345, 10., Curve::Stepped),
                            key(0.123455, 20., Curve::Bezier([0.2, -0.3, 0.8, 1.3])),
                            key(0.5, 30., Curve::Linear),
                            key(1., 40., Curve::Bezier([0.1, 0.3, 0.7, 0.8])),
                        ],
                    },
                    Track {
                        bone: 0,
                        property: Property::X,
                        keys: vec![key(0.5, -3., Curve::Linear), key(1., 7., Curve::Stepped)],
                    },
                    Track {
                        bone: 1,
                        property: Property::Rotation,
                        keys: vec![key(0.5, 50., Curve::Linear)],
                    },
                ],
                physics: vec![],
                physics_resets: vec![],
                inherit_tracks: vec![],
                deform_tracks: vec![],
                attachment_tracks: vec![],
                color_tracks: vec![],
                events: vec![],
                draw_order: vec![],
                path_tracks: vec![],
                sequence_tracks: vec![],
            },
            Animation {
                ik_tracks: vec![],
                name: "target".into(),

                tracks: vec![Track {
                    bone: 1,
                    property: Property::Rotation,
                    keys: vec![key(0.25, 99., Curve::Linear)],
                }],
                physics: vec![],
                physics_resets: vec![],
                inherit_tracks: vec![],
                deform_tracks: vec![],
                attachment_tracks: vec![],
                color_tracks: vec![],
                events: vec![],
                draw_order: vec![],
                path_tracks: vec![],
                sequence_tracks: vec![],
            },
        ],
        skin_scope: Default::default(),
        events: vec![],
        reference_scale: 100.0,
    }
}
fn keys(doc: &Document, animation: usize, bone: usize, property: Property) -> &[Keyframe] {
    &doc.animations[animation]
        .tracks
        .iter()
        .find(|t| t.bone == bone && t.property == property)
        .unwrap()
        .keys
}
fn move_keys(references: Vec<TimelineKey>, delta: f32, copy: bool) -> Command {
    Command::MoveKeys {
        animation: 0,
        keys: references,
        delta,
        copy,
    }
}

#[test]
fn simultaneous_move_preserves_selected_ranges_and_close_unselected_keys() {
    let before = fixture();
    let moved = apply_command(
        &before,
        move_keys(
            vec![
                reference(Property::Rotation, 0.5),
                reference(Property::Rotation, 1.),
            ],
            0.5,
            false,
        ),
    )
    .unwrap();
    let old = keys(&before, 0, 0, Property::Rotation);
    let actual = keys(&moved, 0, 0, Property::Rotation);
    assert_eq!(
        actual,
        &[
            old[0],
            old[1],
            Keyframe { time: 1., ..old[2] },
            Keyframe {
                time: 1.5,
                ..old[3]
            }
        ]
    );
    assert_eq!(moved.animations[0].duration(), 1.5);
    assert_eq!(
        moved.animations[0].tracks[1..],
        before.animations[0].tracks[1..]
    );
    assert_eq!(moved.animations[1], before.animations[1]);
    assert_eq!(before, fixture());
}

#[test]
fn moving_and_copying_exact_fractional_keys_never_merge_neighbors() {
    let before = fixture();
    for copy in [false, true] {
        let result = apply_command(
            &before,
            move_keys(vec![reference(Property::Rotation, 0.12345)], 0.5, copy),
        )
        .unwrap();
        let actual = keys(&result, 0, 0, Property::Rotation);
        assert!(actual.contains(&keys(&before, 0, 0, Property::Rotation)[1]));
        assert!(actual.contains(&key(0.12345 + 0.5, 10., Curve::Stepped)));
        assert_eq!(actual.len(), if copy { 5 } else { 4 });
        assert_eq!(actual.iter().any(|k| k.time == 0.12345), copy);
    }
    assert!(
        apply_command(
            &before,
            move_keys(vec![reference(Property::Rotation, 0.123452)], 1., false)
        )
        .is_err()
    );
    let shifted = apply_command(
        &before,
        move_keys(vec![reference(Property::Rotation, 1.)], 5., false),
    )
    .unwrap();
    assert_eq!(shifted.animations[0].duration(), 6.);
}

#[test]
fn exact_destination_is_replaced_with_the_incoming_value_and_curve() {
    let before = fixture();
    let result = apply_command(
        &before,
        move_keys(vec![reference(Property::Rotation, 0.5)], 0.5, false),
    )
    .unwrap();
    let actual = keys(&result, 0, 0, Property::Rotation);
    assert_eq!(actual.len(), 3);
    assert_eq!(actual[2], key(1., 30., Curve::Linear));
    let result = apply_command(
        &before,
        move_keys(
            vec![reference(Property::Rotation, 0.12345)],
            0.123455 - 0.12345,
            true,
        ),
    )
    .unwrap();
    let actual = keys(&result, 0, 0, Property::Rotation);
    assert_eq!(actual.len(), 4);
    assert_eq!(actual[0], keys(&before, 0, 0, Property::Rotation)[0]);
    assert_eq!(actual[1], key(0.123455, 10., Curve::Stepped));
}

#[test]
fn invalid_moves_are_atomic_and_keep_an_existing_redo_branch() {
    let before = fixture();
    let mut session = Session::new(before.clone()).unwrap();
    session
        .dispatch(Command::SetBoneLength {
            bone: 0,
            length: 99.,
        })
        .unwrap();
    session.undo();
    let valid = reference(Property::Rotation, 0.5);
    for command in [
        move_keys(vec![], 1., false),
        move_keys(vec![valid, valid], 1., false),
        move_keys(vec![valid], -1., false),
        move_keys(vec![valid], f32::NAN, false),
        move_keys(
            vec![reference(Property::Rotation, f32::INFINITY)],
            1.,
            false,
        ),
        move_keys(
            vec![TimelineKey {
                bone: usize::MAX,
                ..valid
            }],
            1.,
            false,
        ),
        move_keys(vec![reference(Property::Y, 0.5)], 1., false),
        move_keys(
            vec![
                reference(Property::Rotation, 0.12345),
                reference(Property::Rotation, 0.123455),
            ],
            1e30,
            false,
        ),
        Command::MoveKeys {
            animation: usize::MAX,
            keys: vec![valid],
            delta: 1.,
            copy: false,
        },
        Command::Batch {
            commands: vec![
                move_keys(vec![valid], 1., false),
                move_keys(vec![reference(Property::Y, 0.5)], 1., false),
            ],
        },
    ] {
        assert!(session.dispatch(command).is_err());
        assert_eq!(session.document(), &before);
        assert!(session.can_redo());
        assert!(!session.can_undo());
    }
    session.redo();
    assert_eq!(session.document().bones[0].length, 99.);
}

#[test]
fn copy_is_owned_property_preserving_sorted_and_zero_anchored() {
    let mut before = fixture();
    let refs = [
        reference(Property::Rotation, 0.5),
        reference(Property::X, 0.5),
        reference(Property::Rotation, 0.12345),
        reference(Property::Rotation, 0.123455),
    ];
    let clipboard = copy_keys(&before, 0, &refs).unwrap();
    assert_eq!(clipboard.keys[0].key, key(0., 10., Curve::Stepped));
    assert_eq!(
        clipboard.keys[1].key,
        Keyframe {
            time: 0.123455 - 0.12345,
            ..keys(&before, 0, 0, Property::Rotation)[1]
        }
    );
    assert_eq!(clipboard.keys[2].property, Property::X);
    assert_eq!(clipboard.keys[3].property, Property::Rotation);
    assert_eq!(clipboard.keys[2].key.time, 0.5 - 0.12345);
    before.animations[0].tracks[0].keys[0].value = 999.;
    assert_eq!(clipboard.keys[0].key.value, 10.);
    let encoded = serde_json::to_string(&clipboard).unwrap();
    assert_eq!(
        serde_json::from_str::<KeyClipboard>(&encoded).unwrap(),
        clipboard
    );
    assert!(copy_keys(&before, 0, &[]).is_err());
    assert!(copy_keys(&before, 0, &[refs[0], refs[0]]).is_err());
    assert!(copy_keys(&before, 0, &[refs[0], TimelineKey { bone: 1, ..refs[0] }]).is_err());
}

#[test]
fn paste_to_another_bone_and_clip_uses_relative_timing_and_setup_relative_values() {
    let before = fixture();
    let clipboard = copy_keys(
        &before,
        0,
        &[
            reference(Property::Rotation, 0.12345),
            reference(Property::X, 1.),
        ],
    )
    .unwrap();
    let result = apply_command(
        &before,
        Command::PasteKeys {
            animation: 1,
            bone: 1,
            time: 0.25,
            clipboard: clipboard.clone(),
        },
    )
    .unwrap();
    assert_eq!(
        keys(&result, 1, 1, Property::Rotation),
        &[key(0.25, 10., Curve::Stepped)]
    );
    assert_eq!(
        keys(&result, 1, 1, Property::X),
        &[key((1. - 0.12345) + 0.25, 7., Curve::Stepped)]
    );
    assert_eq!(result.animations[1].duration(), (1. - 0.12345) + 0.25);
    assert_eq!(crate::local_pose(&result, Some(1), 0.25)[1].rotation, 100.);
    assert_eq!(result.animations[0], before.animations[0]);
    let mut session = Session::new(before.clone()).unwrap();
    session
        .dispatch(Command::PasteKeys {
            animation: 1,
            bone: 1,
            time: 0.25,
            clipboard,
        })
        .unwrap();
    assert_eq!(session.document(), &result);
    session.undo();
    assert_eq!(session.document(), &before);
    session.redo();
    assert_eq!(session.document(), &result);
}

#[test]
fn malformed_clipboards_and_target_overflow_fail_without_changing_data_or_redo() {
    let before = fixture();
    let good = copy_keys(
        &before,
        0,
        &[
            reference(Property::Rotation, 0.12345),
            reference(Property::Rotation, 0.123455),
        ],
    )
    .unwrap();
    let mut bad = vec![KeyClipboard { keys: vec![] }];
    for mutate in [
        |v: &mut KeyClipboard| v.keys[0].key.time = -1.,
        |v: &mut KeyClipboard| v.keys[0].key.time = 0.1,
        |v: &mut KeyClipboard| v.keys[0].key.value = f32::NAN,
        |v: &mut KeyClipboard| v.keys[0].key.time = f32::INFINITY,
        // Crossed or out-of-interval time handles are valid Spine data.
        |v: &mut KeyClipboard| v.keys[0].key.curve = Curve::Bezier([f32::INFINITY, 0., 0.2, 1.]),
        |v: &mut KeyClipboard| v.keys.push(v.keys[0]),
    ] {
        let mut payload = good.clone();
        mutate(&mut payload);
        bad.push(payload);
    }
    let mut session = Session::new(before.clone()).unwrap();
    session
        .dispatch(Command::SetBoneLength {
            bone: 0,
            length: 99.,
        })
        .unwrap();
    session.undo();
    for clipboard in bad {
        assert!(
            session
                .dispatch(Command::PasteKeys {
                    animation: 0,
                    bone: 0,
                    time: 1.,
                    clipboard
                })
                .is_err()
        );
        assert_eq!(session.document(), &before);
        assert!(session.can_redo());
        assert!(!session.can_undo());
    }
    for (animation, bone, time) in [
        (usize::MAX, 0, 0.),
        (0, usize::MAX, 0.),
        (0, 0, -1.),
        (0, 0, f32::NAN),
        (0, 0, 1e30),
    ] {
        assert!(
            session
                .dispatch(Command::PasteKeys {
                    animation,
                    bone,
                    time,
                    clipboard: good.clone()
                })
                .is_err()
        );
        assert_eq!(session.document(), &before);
        assert!(session.can_redo());
    }
    let mut overflow = good;
    overflow.keys[1].key.time = f32::MAX;
    assert!(
        session
            .dispatch(Command::PasteKeys {
                animation: 0,
                bone: 0,
                time: f32::MAX,
                clipboard: overflow
            })
            .is_err()
    );
    assert_eq!(session.document(), &before);
    assert!(session.can_redo());
}

#[test]
fn preview_recomputes_from_base_restores_overwrites_and_commits_one_undo_entry() {
    let before = fixture();
    let mut session = Session::new(before.clone()).unwrap();
    session
        .edit_pose(
            0,
            EditTarget::Animate {
                animation: 0,
                time: 0.,
            },
            Transform {
                rotation: 123.,
                ..Default::default()
            },
            &[Property::Rotation],
            false,
        )
        .unwrap();
    let base_pose = session.pose_edits().clone();
    session.begin_transaction();
    session
        .preview_transaction(move_keys(
            vec![reference(Property::Rotation, 0.5)],
            0.5,
            false,
        ))
        .unwrap();
    assert_eq!(
        keys(session.document(), 0, 0, Property::Rotation)
            .last()
            .unwrap()
            .value,
        30.
    );
    let prior = session.document().clone();
    assert!(
        session
            .preview_transaction(move_keys(
                vec![reference(Property::Rotation, 0.5)],
                -1.,
                false
            ))
            .is_err()
    );
    assert_eq!(session.document(), &prior);
    assert_eq!(session.pose_edits(), &base_pose);
    session
        .preview_transaction(move_keys(
            vec![reference(Property::Rotation, 0.5)],
            1.5,
            false,
        ))
        .unwrap();
    let result = session.document().clone();
    assert!(keys(&result, 0, 0, Property::Rotation).contains(&key(
        1.,
        40.,
        Curve::Bezier([0.1, 0.3, 0.7, 0.8])
    )));
    assert!(keys(&result, 0, 0, Property::Rotation).contains(&key(2., 30., Curve::Linear)));
    assert_eq!(session.pose_edits(), &base_pose);
    session.commit_transaction();
    session.undo();
    assert_eq!(session.document(), &before);
    assert_eq!(session.pose_edits(), &base_pose);
    session.redo();
    assert_eq!(session.document(), &result);
}

#[test]
fn cancelled_previews_and_zero_offset_keep_saved_state_and_redo() {
    let before = fixture();
    let command = || move_keys(vec![reference(Property::Rotation, 0.5)], 1., false);
    let mut session = Session::new(before.clone()).unwrap();
    assert!(session.preview_transaction(command()).is_err());
    session
        .dispatch(Command::SetBoneLength {
            bone: 0,
            length: 99.,
        })
        .unwrap();
    session.undo();
    session.begin_transaction();
    session.preview_transaction(command()).unwrap();
    session.cancel_transaction();
    assert_eq!(session.document(), &before);
    assert!(!session.is_dirty());
    assert!(session.can_redo());
    for copy in [false, true] {
        session
            .dispatch(move_keys(
                vec![reference(Property::Rotation, 0.5)],
                0.,
                copy,
            ))
            .unwrap();
        assert_eq!(session.document(), &before);
        assert!(!session.can_undo());
        assert!(session.can_redo());
    }
}

#[test]
fn command_and_payload_serialization_rejects_unknown_fields() {
    let before = fixture();
    let clipboard = copy_keys(&before, 0, &[reference(Property::Rotation, 0.5)]).unwrap();
    for command in [
        move_keys(vec![reference(Property::Rotation, 0.5)], 1., false),
        Command::PasteKeys {
            animation: 1,
            bone: 1,
            time: 0.,
            clipboard,
        },
    ] {
        let mut value = serde_json::to_value(&command).unwrap();
        let decoded: Command = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(
            apply_command(&before, command).unwrap(),
            apply_command(&before, decoded).unwrap()
        );
        value["unexpected"] = true.into();
        assert!(serde_json::from_value::<Command>(value).is_err());
    }
    assert!(
        serde_json::from_str::<TimelineKey>(r#"{"bone":0,"property":"x","time":0,"extra":1}"#)
            .is_err()
    );
    assert!(serde_json::from_str::<KeyClipboard>(r#"{"keys":[],"extra":1}"#).is_err());
    assert!(
        serde_json::from_str::<KeyClipboard>(
            r#"{"keys":[{"property":"x","key":{"time":0,"value":1,"curve":"linear"},"extra":1}]}"#
        )
        .is_err()
    );
}

#[test]
fn simultaneous_move_can_span_bones_and_properties_without_retargeting() {
    let before = fixture();
    let result = apply_command(
        &before,
        move_keys(
            vec![
                reference(Property::X, 0.5),
                reference(Property::Rotation, 0.5),
                TimelineKey {
                    bone: 1,
                    property: Property::Rotation,
                    time: 0.5,
                },
            ],
            0.25,
            false,
        ),
    )
    .unwrap();
    assert_eq!(
        keys(&result, 0, 0, Property::X)[0],
        key(0.75, -3., Curve::Linear)
    );
    assert_eq!(
        keys(&result, 0, 0, Property::Rotation)[2],
        key(0.75, 30., Curve::Linear)
    );
    assert_eq!(
        keys(&result, 0, 1, Property::Rotation),
        &[key(0.75, 50., Curve::Linear)]
    );
    assert_eq!(result.bones, before.bones);
}

#[test]
fn later_pose_edits_and_set_key_choose_exact_neighbors_and_reject_ambiguous_times() {
    let mut before = fixture();
    let curve = Curve::Bezier([0.2, -0.1, 0.8, 1.2]);
    before.animations[0].tracks[0].keys =
        vec![key(0.499995, 1., Curve::Stepped), key(0.5, 2., curve)];
    let track = keys(&before, 0, 0, Property::Rotation);
    assert_eq!(keyframe_at(track, 0.5).unwrap(), Some(track[1]));
    assert!(keyframe_at(track, 0.4999975).is_err());
    assert_eq!(keyframe_at(&track[1..], 0.500005).unwrap(), Some(track[1]));
    assert_eq!(keyframe_at(track, 0.6).unwrap(), None);
    for invalid in [-1., f32::NEG_INFINITY, f32::INFINITY, f32::NAN] {
        assert!(keyframe_at(track, invalid).is_err());
        assert!(keyframe_at(&[], invalid).is_err());
    }
    let pose = || Command::SetPose {
        bone: 0,
        target: EditTarget::Animate {
            animation: 0,
            time: 0.5,
        },
        transform: Transform {
            rotation: 20.,
            ..Default::default()
        },
        properties: vec![Property::Rotation],
    };
    let result = apply_command(&before, pose()).unwrap();
    assert_eq!(
        keys(&result, 0, 0, Property::Rotation),
        &[before.animations[0].tracks[0].keys[0], key(0.5, 20., curve)]
    );
    let result = apply_command(
        &before,
        Command::SetKey {
            animation: 0,
            bone: 0,
            property: Property::Rotation,
            key: key(0.5, 99., Curve::Linear),
        },
    )
    .unwrap();
    assert_eq!(
        keys(&result, 0, 0, Property::Rotation),
        &[
            before.animations[0].tracks[0].keys[0],
            key(0.5, 99., Curve::Linear)
        ]
    );

    before.animations[0].tracks[0].keys[1].time = 0.500005;
    let mut session = Session::new(before.clone()).unwrap();
    session
        .dispatch(Command::SetBoneLength {
            bone: 0,
            length: 99.,
        })
        .unwrap();
    session.undo();
    for command in [
        pose(),
        Command::SetKey {
            animation: 0,
            bone: 0,
            property: Property::Rotation,
            key: key(0.5, 99., Curve::Linear),
        },
        Command::Batch {
            commands: vec![
                Command::SetBoneLength {
                    bone: 0,
                    length: 99.,
                },
                pose(),
            ],
        },
    ] {
        assert!(session.dispatch(command).is_err());
        assert_eq!(session.document(), &before);
        assert!(session.can_redo());
        assert!(!session.can_undo());
    }
}
