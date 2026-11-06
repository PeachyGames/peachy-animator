use crate::*;
use peachy_document::*;
use serde_json::json;

fn document() -> Document {
    serde_json::from_str(include_str!("../../../examples/peach-bot/peach-bot.peachy")).unwrap()
}

#[test]
fn slot_color_command_has_strict_byte_channels_and_unknown_field_refusal() {
    let valid = json!({"command":"setSlotColor","slot":0,"color":[1,127,128,255]});
    let command: Command = serde_json::from_value(valid.clone()).unwrap();
    assert_eq!(serde_json::to_value(command).unwrap(), valid);
    for color in [
        json!([]),
        json!([1, 2, 3]),
        json!([1, 2, 3, 4, 5]),
        json!([-1, 0, 0, 0]),
        json!([256, 0, 0, 0]),
        json!([0.0, 0, 0, 0]),
        json!(null),
        json!("ffffffff"),
    ] {
        let mut invalid = valid.clone();
        invalid["color"] = color;
        assert!(serde_json::from_value::<Command>(invalid).is_err());
    }
    let mut invalid = valid.clone();
    invalid["future"] = json!(true);
    assert!(serde_json::from_value::<Command>(invalid).is_err());
    let mut missing = valid;
    missing.as_object_mut().unwrap().remove("color");
    assert!(serde_json::from_value::<Command>(missing).is_err());
}

#[test]
fn setup_tint_is_atomic_undoable_and_preserves_pending_pose_and_redo() {
    let original = document();
    let mut session = Session::new(original.clone()).unwrap();
    let mut posed = session.local_pose(Some(0), 0.3)[7];
    posed.rotation += 13.;
    session
        .edit_pose(
            7,
            EditTarget::Animate {
                animation: 0,
                time: 0.3,
            },
            posed,
            &[Property::Rotation],
            false,
        )
        .unwrap();
    let pending = session.pose_edits().clone();
    session
        .dispatch(Command::SetSlotColor {
            slot: 0,
            color: [18, 52, 86, 120],
        })
        .unwrap();
    assert_eq!(session.pose_edits(), &pending);
    let colored = session.document().clone();
    let mut expected = original.clone();
    expected.slots[0].color = [18, 52, 86, 120];
    assert_eq!(colored, expected);
    assert!(session.is_dirty());
    session.undo();
    assert_eq!(session.document(), &original);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
    let flags = (session.can_undo(), session.can_redo());
    assert!(
        session
            .dispatch(Command::Batch {
                commands: vec![
                    Command::SetSlotColor {
                        slot: 0,
                        color: [0, 0, 0, 0]
                    },
                    Command::SetSlotColor {
                        slot: usize::MAX,
                        color: [255; 4]
                    }
                ]
            })
            .is_err()
    );
    assert_eq!(session.document(), &original);
    assert_eq!(session.pose_edits(), &pending);
    assert_eq!((session.can_undo(), session.can_redo()), flags);
    session
        .dispatch(Command::SetSlotColor {
            slot: 0,
            color: [255; 4],
        })
        .unwrap();
    assert_eq!((session.can_undo(), session.can_redo()), flags);
    session.begin_transaction();
    session
        .preview_transaction(Command::SetSlotColor {
            slot: 0,
            color: [5, 6, 7, 8],
        })
        .unwrap();
    session
        .preview_transaction(Command::SetSlotColor {
            slot: 0,
            color: [9, 10, 11, 12],
        })
        .unwrap();
    assert_eq!(session.pose_edits(), &pending);
    session.cancel_transaction();
    assert_eq!(session.document(), &original);
    assert!(session.can_redo());
    session.redo();
    assert_eq!(session.document(), &colored);
    assert_eq!(session.pose_edits(), &pending);
}

#[test]
fn scene_draw_tint_follows_the_slot_across_skins_meshes_and_alpha_zero_without_geometry_changes() {
    for input in [
        include_str!("../../../examples/peach-bot/peach-bot.peachy"),
        include_str!("../../../examples/peach-bot/weighted-bot.peachy"),
    ] {
        let original: Document = serde_json::from_str(input).unwrap();
        for skin in ["default", "peach", "mint"] {
            let before = checked_evaluate_scene(&original, Some(0), 0.3, skin, None).unwrap();
            for slot in 0..original.slots.len() {
                for color in [[18, 52, 86, 120], [255, 0, 127, 0]] {
                    let colored =
                        apply_command(&original, Command::SetSlotColor { slot, color }).unwrap();
                    let after = checked_evaluate_scene(&colored, Some(0), 0.3, skin, None).unwrap();
                    assert_eq!(before.locals, after.locals);
                    assert_eq!(
                        serde_json::to_value(&before.world).unwrap(),
                        serde_json::to_value(&after.world).unwrap()
                    );
                    assert_eq!(before.attachments.len(), after.attachments.len());
                    for (old, new) in before.attachments.iter().zip(&after.attachments) {
                        assert_eq!(old.attachment, new.attachment);
                        assert_eq!(old.vertices, new.vertices);
                        assert_eq!(old.uvs, new.uvs);
                        assert_eq!(old.triangles, new.triangles);
                        assert_eq!(
                            new.color,
                            if new.slot == slot {
                                color
                            } else {
                                original.slots[new.slot].color
                            }
                        );
                    }
                }
            }
        }
    }
}
