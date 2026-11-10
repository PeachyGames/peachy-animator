use crate::*;
use peachy_document::*;

fn document() -> Document {
    Document {
        constraints: vec![],
        format: FORMAT_VERSION,
        name: "Key-derived duration".into(),
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
                name: "other".into(),
                parent: Some(0),
                length: 5.,
                setup: Transform::default(),
                inherit: peachy_document::Inherit::Normal,
            },
        ],
        slots: vec![],
        skins: Default::default(),
        animations: vec![Animation {
            ik_tracks: vec![],
            name: "empty".into(),
            tracks: vec![],
            physics: vec![],
            physics_resets: vec![],
            inherit_tracks: vec![],
            deform_tracks: vec![],
            attachment_tracks: vec![],
            color_tracks: vec![],
            events: vec![],
            draw_order: vec![],
        }],
        skin_scope: Default::default(),
        events: vec![],
        reference_scale: 100.0,
    }
}

fn key(time: f32, value: f32) -> Keyframe {
    Keyframe {
        time,
        value,
        curve: Curve::Linear,
    }
}

fn set_key(time: f32, value: f32) -> Command {
    Command::SetKey {
        animation: 0,
        bone: 0,
        property: Property::Rotation,
        key: key(time, value),
    }
}

fn delete(bone: usize, property: Property, time: f32) -> Command {
    Command::DeleteKey {
        animation: 0,
        bone,
        property,
        time,
    }
}

fn move_last(delta: f32) -> Command {
    Command::MoveKeys {
        animation: 0,
        keys: vec![TimelineKey {
            bone: 0,
            property: Property::Rotation,
            time: 4.,
        }],
        delta,
        copy: false,
    }
}

fn with_keys() -> Document {
    let mut doc = document();
    doc.animations[0].tracks = vec![
        Track {
            bone: 0,
            property: Property::Rotation,
            keys: vec![key(0., 10.), key(2., 20.), key(4., 40.)],
        },
        Track {
            bone: 1,
            property: Property::X,
            keys: vec![key(3.5, 12.)],
        },
    ];
    doc
}

#[test]
fn duration_shrinks_to_the_last_remaining_key_across_bones_and_undo_restores_it() {
    let before = with_keys();
    let mut session = Session::new(before.clone()).unwrap();
    assert_eq!(before.animations[0].duration(), 4.);
    session.dispatch(move_last(-3.)).unwrap();
    assert_eq!(session.document().animations[0].duration(), 3.5);
    assert_eq!(last_key_time(session.document(), 0), 3.5);
    session.dispatch(delete(1, Property::X, 3.5)).unwrap();
    assert_eq!(session.document().animations[0].duration(), 2.);
    session.dispatch(delete(0, Property::Rotation, 2.)).unwrap();
    assert_eq!(session.document().animations[0].duration(), 1.);
    session.dispatch(delete(0, Property::Rotation, 1.)).unwrap();
    assert_eq!(session.document().animations[0].duration(), 0.);
    assert_eq!(
        session.document().animations[0].tracks[0].keys,
        [key(0., 10.)]
    );
    session.dispatch(delete(0, Property::Rotation, 0.)).unwrap();
    assert!(session.document().animations[0].tracks.is_empty());
    assert_eq!(session.document().animations[0].duration(), 0.);
    for expected in [0., 1., 2., 3.5, 4.] {
        session.undo();
        assert_eq!(session.document().animations[0].duration(), expected);
    }
    assert_eq!(session.document(), &before);
    assert!(!session.can_undo());
    assert!(!session.is_dirty());
    for expected in [3.5, 2., 1., 0., 0.] {
        session.redo();
        assert_eq!(session.document().animations[0].duration(), expected);
    }
    assert!(session.document().animations[0].tracks.is_empty());
}

#[test]
fn shrinking_collision_previews_restore_keys_duration_pose_and_redo_until_commit() {
    let mut before = with_keys();
    before.animations[0].tracks[0].keys[1].time = 1.;
    let mut session = Session::new(before.clone()).unwrap();
    let mut posed = session.local_pose(Some(0), 0.)[0];
    posed.rotation += 12.;
    session
        .edit_pose(
            0,
            EditTarget::Animate {
                animation: 0,
                time: 0.,
            },
            posed,
            &[Property::Rotation],
            false,
        )
        .unwrap();
    let pose = session.pose_edits().clone();
    session
        .dispatch(Command::SetBoneLength {
            bone: 0,
            length: 11.,
        })
        .unwrap();
    session.undo();
    assert!(session.can_redo());
    session.begin_transaction();
    session.preview_transaction(move_last(-3.)).unwrap();
    assert_eq!(session.document().animations[0].duration(), 3.5);
    assert_eq!(
        session.document().animations[0].tracks[0].keys[1],
        key(1., 40.)
    );
    session.preview_transaction(move_last(-2.)).unwrap();
    assert_eq!(
        session.document().animations[0].tracks[0].keys[1],
        key(1., 20.)
    );
    assert_eq!(session.document().animations[0].duration(), 3.5);
    assert_eq!(session.pose_edits(), &pose);
    let valid_preview = session.document().clone();
    assert!(session.preview_transaction(move_last(-5.)).is_err());
    assert_eq!(session.document(), &valid_preview);
    session.cancel_transaction();
    assert_eq!(session.document(), &before);
    assert_eq!(session.document().animations[0].duration(), 4.);
    assert_eq!(session.pose_edits(), &pose);
    assert!(session.can_redo());
    assert!(!session.is_dirty());

    session.begin_transaction();
    session.preview_transaction(move_last(-2.)).unwrap();
    session.commit_transaction();
    assert_eq!(session.document().animations[0].duration(), 3.5);
    assert!(!session.can_redo());
    session.undo();
    assert_eq!(session.document(), &before);
    assert_eq!(session.pose_edits(), &pose);
    session.redo();
    assert_eq!(session.document().animations[0].duration(), 3.5);
}

#[test]
fn explicit_holds_survive_extension_native_roundtrip_and_deleting_the_extension() {
    let with_hold = apply_command(
        &document(),
        Command::AddAnimation {
            name: "hold".into(),
            duration: 4.,
        },
    )
    .unwrap();
    let hold = &with_hold.animations[1];
    assert_eq!(hold.duration(), 4.);
    assert_eq!(hold.tracks.len(), 1);
    assert_eq!(hold.tracks[0].bone, 0);
    assert_eq!(hold.tracks[0].property, Property::Rotation);
    assert_eq!(hold.tracks[0].keys, [key(4., 0.)]);
    let extended = apply_command(
        &with_hold,
        Command::SetKey {
            animation: 1,
            bone: 0,
            property: Property::Rotation,
            key: key(6., 40.),
        },
    )
    .unwrap();
    assert_eq!(extended.animations[1].duration(), 6.);
    let native = serde_json::to_value(&extended).unwrap();
    assert_eq!(native["format"], FORMAT_VERSION);
    assert!(native["animations"][1].get("duration").is_none());
    let reopened: Document = serde_json::from_value(native).unwrap();
    assert_eq!(reopened, extended);
    let mut session = Session::new(reopened).unwrap();
    session
        .dispatch(Command::DeleteKey {
            animation: 1,
            bone: 0,
            property: Property::Rotation,
            time: 6.,
        })
        .unwrap();
    assert_eq!(session.document().animations[1], *hold);
    session
        .dispatch(Command::DeleteKey {
            animation: 1,
            bone: 0,
            property: Property::Rotation,
            time: 4.,
        })
        .unwrap();
    assert_eq!(session.document().animations[1].duration(), 0.);
    assert!(session.document().animations[1].tracks.is_empty());
    session.undo();
    assert_eq!(session.document().animations[1], *hold);
    session.undo();
    assert_eq!(session.document(), &extended);
}

#[test]
fn adding_empty_and_tiny_clips_keeps_real_keys_and_invalid_durations_preserve_redo() {
    let before = document();
    let mut session = Session::new(before.clone()).unwrap();
    session
        .dispatch(Command::AddAnimation {
            name: "new".into(),
            duration: 0.,
        })
        .unwrap();
    assert!(session.document().animations[1].tracks.is_empty());
    assert_eq!(session.document().animations[1].duration(), 0.);
    session.undo();
    for duration in [-1., f32::NEG_INFINITY, f32::INFINITY, f32::NAN] {
        assert!(
            session
                .dispatch(Command::AddAnimation {
                    name: "invalid".into(),
                    duration
                })
                .is_err()
        );
        assert_eq!(session.document(), &before);
        assert!(!session.can_undo());
        assert!(session.can_redo());
        assert!(!session.is_dirty());
    }
    session.redo();
    session
        .dispatch(Command::AddAnimation {
            name: "tiny".into(),
            duration: 1e-8,
        })
        .unwrap();
    assert_eq!(session.document().animations[2].duration(), 1e-8);
    assert_eq!(
        session.document().animations[2].tracks[0].keys[0].time,
        1e-8
    );
}

#[test]
fn beyond_end_unkeyed_poses_do_not_extend_clips_until_keyed_at_the_authoring_time() {
    for fps in [24, 30, 60] {
        let mut before = document();
        before.fps = fps;
        let mut session = Session::new(before.clone()).unwrap();
        let time = 2.137;
        let snapped = ((f64::from(time) * f64::from(fps)).round() / f64::from(fps)) as f32;
        let pose = Transform {
            rotation: 35.,
            ..Default::default()
        };
        session
            .edit_pose(
                0,
                EditTarget::Animate { animation: 0, time },
                pose,
                &[Property::Rotation],
                false,
            )
            .unwrap();
        assert_eq!(session.document(), &before);
        assert_eq!(session.document().animations[0].duration(), 0.);
        assert!(!session.is_dirty());
        assert_eq!(session.local_pose(Some(0), time)[0].rotation, 35.);
        session.key_edited(0, time, None).unwrap();
        assert!(session.pose_edits().is_empty());
        assert_eq!(session.document().animations[0].duration(), snapped);
        assert_eq!(
            session.document().animations[0].tracks[0].keys,
            [key(snapped, 35.)]
        );
        session.undo();
        assert_eq!(session.document(), &before);
        assert!(!session.pose_edits().is_empty());
        session.redo();
        session.key_pose(0, 5., 0, &[Property::Rotation]).unwrap();
        assert_eq!(session.document().animations[0].duration(), 5.);
        assert_eq!(session.local_pose(Some(0), 20.)[0].rotation, 35.);
        session
            .edit_pose(
                0,
                EditTarget::Animate {
                    animation: 0,
                    time: 7.,
                },
                Transform {
                    rotation: 45.,
                    ..pose
                },
                &[Property::Rotation],
                true,
            )
            .unwrap();
        assert_eq!(session.document().animations[0].duration(), 7.);
        assert!(session.pose_edits().is_empty());
    }
}

#[test]
fn empty_frame_zero_and_close_tiny_keys_keep_distinct_data_and_exact_ends_at_every_fps() {
    for fps in [24, 30, 60] {
        let mut empty = document();
        empty.fps = fps;
        let zero = apply_command(&empty, set_key(0., 45.)).unwrap();
        assert_eq!(empty.animations[0].duration(), 0.);
        assert_eq!(zero.animations[0].duration(), 0.);
        assert_eq!(local_pose(&empty, Some(0), 10.)[0].rotation, 0.);
        assert_eq!(local_pose(&zero, Some(0), 10.)[0].rotation, 45.);
        let tiny = apply_command(
            &zero,
            Command::PasteKeys {
                animation: 0,
                bone: 0,
                time: 0.,
                clipboard: KeyClipboard {
                    keys: vec![
                        ClipboardKey {
                            property: Property::Rotation,
                            key: key(0., 45.),
                        },
                        ClipboardKey {
                            property: Property::Rotation,
                            key: key(1e-8, 60.),
                        },
                    ],
                },
            },
        )
        .unwrap();
        assert_eq!(tiny.animations[0].duration(), 1e-8);
        assert_eq!(tiny.animations[0].tracks[0].keys.len(), 2);
        assert_eq!(local_pose(&tiny, Some(0), 1.)[0].rotation, 60.);
        let encoded = serde_json::to_string(&tiny).unwrap();
        let reopened: Document = serde_json::from_str(&encoded).unwrap();
        reopened.validate().unwrap();
        assert_eq!(reopened, tiny);
        assert!(apply_command(&tiny, delete(0, Property::Rotation, 1.)).is_err());
    }
}

#[test]
fn delete_rejects_queries_just_past_the_end_but_accepts_exact_final_and_frame_zero_keys() {
    let before = apply_command(&document(), set_key(1., 35.)).unwrap();
    let mut session = Session::new(before.clone()).unwrap();
    session
        .dispatch(Command::SetBoneLength {
            bone: 0,
            length: 11.,
        })
        .unwrap();
    session.undo();
    let just_past = f32::from_bits(1f32.to_bits() + 1);
    assert!(same_key_time(just_past, 1.));
    for time in [just_past, 2.] {
        assert!(
            session
                .dispatch(delete(0, Property::Rotation, time))
                .is_err()
        );
        assert_eq!(session.document(), &before);
        assert!(!session.is_dirty());
        assert!(!session.can_undo());
        assert!(session.can_redo());
    }
    session.dispatch(delete(0, Property::Rotation, 1.)).unwrap();
    assert_eq!(session.document().animations[0].duration(), 0.);
    assert!(session.document().animations[0].tracks.is_empty());
    session.undo();
    assert_eq!(session.document(), &before);

    let zero = apply_command(&document(), set_key(0., 45.)).unwrap();
    let mut session = Session::new(zero.clone()).unwrap();
    session.dispatch(delete(0, Property::Rotation, 0.)).unwrap();
    assert_eq!(session.document().animations[0].duration(), 0.);
    assert!(session.document().animations[0].tracks.is_empty());
    session.undo();
    assert_eq!(session.document(), &zero);
    assert_eq!(session.document().animations[0].duration(), 0.);
    assert_eq!(session.local_pose(Some(0), 0.)[0].rotation, 45.);
}

#[test]
fn finite_extreme_authoring_times_do_not_overflow_frame_snapping_and_invalid_times_are_atomic() {
    let before = document();
    let mut session = Session::new(before.clone()).unwrap();
    let pose = Transform {
        rotation: 20.,
        ..Default::default()
    };
    session
        .edit_pose(
            0,
            EditTarget::Animate {
                animation: 0,
                time: f32::MAX,
            },
            pose,
            &[Property::Rotation],
            false,
        )
        .unwrap();
    assert_eq!(session.document(), &before);
    session
        .key_pose(0, f32::MAX, 0, &[Property::Rotation])
        .unwrap();
    assert_eq!(session.document().animations[0].duration(), f32::MAX);
    assert_eq!(
        local_pose(session.document(), Some(0), f32::MAX)[0].rotation,
        20.
    );
    session.undo();
    let pending = session.pose_edits().clone();
    assert!(session.can_redo());
    for time in [-1., f32::NEG_INFINITY, f32::INFINITY, f32::NAN] {
        assert!(
            session
                .edit_pose(
                    0,
                    EditTarget::Animate { animation: 0, time },
                    pose,
                    &[Property::Rotation],
                    false
                )
                .is_err()
        );
        assert!(session.key_pose(0, time, 0, &[Property::Rotation]).is_err());
        assert!(session.dispatch(set_key(time, 10.)).is_err());
        assert_eq!(session.document(), &before);
        assert_eq!(session.pose_edits(), &pending);
        assert!(session.can_redo());
    }
}

#[test]
fn zero_transport_stops_at_zero_and_tiny_positive_intervals_still_loop_and_stop_finitely() {
    for direction in [PlayDirection::Forward, PlayDirection::Backward] {
        for looping in [false, true] {
            for time in [0., 1., f32::MAX] {
                for elapsed in [0., 1e-8, 1., f32::MAX] {
                    let step = advance_playback(time, elapsed, 0., looping, direction);
                    assert_eq!(
                        step,
                        PlaybackStep {
                            time: 0.,
                            playing: false
                        }
                    );
                }
            }
        }
        let end = 1e-8;
        let step = advance_playback(end / 2., end * 2., end, true, direction);
        assert!(step.time.is_finite());
        assert!((0. ..end).contains(&step.time));
        assert!(step.playing);
        let step = advance_playback(end / 2., end * 2., end, false, direction);
        assert_eq!(
            step.time,
            if direction == PlayDirection::Forward {
                end
            } else {
                0.
            }
        );
        assert!(!step.playing);
    }
}

#[test]
fn native_rejects_stored_duration_and_old_format_and_keeps_fixture_keys_valid() {
    let before = with_keys();
    let mut value = serde_json::to_value(&before).unwrap();
    value["animations"][0]["duration"] = 4.into();
    assert!(serde_json::from_value::<Document>(value.clone()).is_err());
    value["animations"][0]
        .as_object_mut()
        .unwrap()
        .remove("duration");
    value["format"] = 2.into();
    assert!(
        serde_json::from_value::<Document>(value)
            .unwrap()
            .validate()
            .is_err()
    );
    for (source, expected_ends) in [
        (
            include_str!("../../../examples/peach-bot/peach-bot.peachy"),
            [2.4, 2.].as_slice(),
        ),
        (
            include_str!("../../../examples/peach-bot/mesh-bot.peachy"),
            [2.4, 2.].as_slice(),
        ),
        (
            include_str!("../../../examples/peach-bot/contract-cases.peachy"),
            [2.].as_slice(),
        ),
        (
            include_str!("../../../examples/peach-bot/mesh-contract.peachy"),
            [2.].as_slice(),
        ),
    ] {
        let fixture: Document = serde_json::from_str(source).unwrap();
        fixture.validate().unwrap();
        assert_eq!(fixture.format, FORMAT_VERSION);
        assert_eq!(
            fixture
                .animations
                .iter()
                .map(Animation::duration)
                .collect::<Vec<_>>(),
            expected_ends
        );
    }
}
