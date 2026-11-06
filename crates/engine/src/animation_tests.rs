use crate::{Command, Session, apply_command};
use peachy_document::*;

fn fixture() -> Document {
    serde_json::from_str(include_str!("../../../examples/peach-bot/peach-bot.peachy")).unwrap()
}

#[test]
fn rename_preserves_animation_identity_tracks_pose_and_history() {
    let original = fixture();
    let mut session = Session::new(original.clone()).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "renamed".into(),
        })
        .unwrap();
    let renamed = session.document().clone();
    let mut expected = original.clone();
    expected.animations[0].name = "renamed".into();
    assert_eq!(renamed, expected);
    assert!(session.is_dirty());
    for time in [0.0, 0.35, original.animations[0].duration] {
        assert_eq!(
            crate::local_pose(&original, Some(0), time),
            crate::local_pose(&renamed, Some(0), time)
        );
    }
    session.undo();
    assert_eq!(session.document(), &original);
    assert!(!session.is_dirty());
    session.redo();
    assert_eq!(session.document(), &renamed);
    // Renaming to the existing name does not add a redundant history entry.
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "renamed".into(),
        })
        .unwrap();
    session.undo();
    assert_eq!(session.document(), &original);
}

#[test]
fn duplicate_is_an_independent_clip_with_exact_duration_keys_and_curves() {
    let mut original = fixture();
    // Exercise a reserved duration beyond the last key, fractional times and each curve.
    let source = &mut original.animations[0];
    source.duration += 2.0;
    source.tracks[0].keys = vec![
        Keyframe {
            time: 0.0,
            value: 3.0,
            curve: Curve::Linear,
        },
        Keyframe {
            time: 0.123,
            value: 8.0,
            curve: Curve::Stepped,
        },
        Keyframe {
            time: 0.719,
            value: 13.0,
            curve: Curve::Bezier([0.2, -0.3, 0.7, 1.3]),
        },
    ];
    let count = original.animations.len();
    let mut session = Session::new(original.clone()).unwrap();
    session
        .dispatch(Command::DuplicateAnimation {
            animation: 0,
            name: "copy".into(),
        })
        .unwrap();
    let copied = session.document().clone();
    assert_eq!(copied.animations.len(), count + 1);
    assert_eq!(&copied.animations[..count], &original.animations);
    let mut expected = original.animations[0].clone();
    expected.name = "copy".into();
    assert_eq!(copied.animations[count], expected);
    let key = copied.animations[count].tracks[0].keys[0];
    session
        .dispatch(Command::SetKey {
            animation: count,
            bone: copied.animations[count].tracks[0].bone,
            property: copied.animations[count].tracks[0].property,
            key: Keyframe {
                value: key.value + 100.0,
                ..key
            },
        })
        .unwrap();
    assert_eq!(session.document().animations[0], original.animations[0]);
    assert_ne!(session.document().animations[count], expected);
    session.undo();
    assert_eq!(session.document(), &copied);
    session.undo();
    assert_eq!(session.document(), &original);
    session.redo();
    assert_eq!(session.document(), &copied);
    let native = serde_json::to_string(&copied).unwrap();
    assert_eq!(serde_json::from_str::<Document>(&native).unwrap(), copied);
}

#[test]
fn animation_management_failures_are_atomic_and_preserve_redo() {
    let original = fixture();
    let mut session = Session::new(original.clone()).unwrap();
    session
        .dispatch(Command::DuplicateAnimation {
            animation: 0,
            name: "copy".into(),
        })
        .unwrap();
    session.undo();
    for command in [
        Command::RenameAnimation {
            animation: usize::MAX,
            name: "new".into(),
        },
        Command::DuplicateAnimation {
            animation: usize::MAX,
            name: "new".into(),
        },
        Command::RenameAnimation {
            animation: 0,
            name: " \t".into(),
        },
        Command::DuplicateAnimation {
            animation: 0,
            name: "".into(),
        },
        Command::DuplicateAnimation {
            animation: 0,
            name: original.animations[0].name.clone(),
        },
        Command::Batch {
            commands: vec![
                Command::RenameAnimation {
                    animation: 0,
                    name: "new".into(),
                },
                Command::DuplicateAnimation {
                    animation: 0,
                    name: "new".into(),
                },
            ],
        },
    ] {
        assert!(session.dispatch(command).is_err());
        assert_eq!(session.document(), &original);
        assert!(!session.is_dirty());
        assert!(!session.can_undo());
        assert!(session.can_redo());
    }
    session.redo();
    let with_copy = session.document().clone();
    assert!(
        session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "copy".into()
            })
            .is_err()
    );
    assert_eq!(session.document(), &with_copy);
}

#[test]
fn animation_commands_are_serializable_and_reject_unknown_fields() {
    for command in [
        Command::RenameAnimation {
            animation: 0,
            name: "new".into(),
        },
        Command::DuplicateAnimation {
            animation: 0,
            name: "new".into(),
        },
    ] {
        let encoded = serde_json::to_value(&command).unwrap();
        let decoded: Command = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(
            apply_command(&fixture(), command).unwrap(),
            apply_command(&fixture(), decoded).unwrap()
        );
        let mut invalid = encoded;
        invalid["unexpected"] = true.into();
        assert!(serde_json::from_value::<Command>(invalid).is_err());
    }
}
