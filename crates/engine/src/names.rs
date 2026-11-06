//! Pure name changes preserve index identity and all references to the renamed slot.
use peachy_document::Document;

pub(super) fn rename_bone(doc: &mut Document, bone: usize, name: String) -> Result<(), String> {
    let target = doc.bones.get(bone).ok_or("Unknown bone")?;
    if target.name == name {
        return Ok(());
    }
    if name.trim().is_empty() || doc.bones.iter().any(|b| b.name == name) {
        return Err("Bone names must be nonempty and unique".into());
    }
    doc.bones[bone].name = name;
    Ok(())
}

pub(super) fn rename_slot(doc: &mut Document, slot: usize, name: String) -> Result<(), String> {
    let old_name = &doc.slots.get(slot).ok_or("Unknown slot")?.name;
    if old_name == &name {
        return Ok(());
    }
    if name.trim().is_empty() || doc.slots.iter().any(|s| s.name == name) {
        return Err("Slot names must be nonempty and unique".into());
    }
    for skin in doc.skins.values_mut() {
        if let Some(attachments) = skin.remove(old_name) {
            skin.insert(name.clone(), attachments);
        }
    }
    doc.slots[slot].name = name;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{Command, Session, apply_command, evaluate_scene};
    use peachy_document::Document;

    fn original() -> Document {
        serde_json::from_str(include_str!("../../../examples/peach-bot/mesh-bot.peachy")).unwrap()
    }

    #[test]
    fn names_preserve_indices_all_skins_and_evaluated_geometry_and_undo() {
        let before = original();
        let slot = before.slots.iter().position(|s| s.name == "head").unwrap();
        let bone = before.slots[slot].bone;
        let mut session = Session::new(before.clone()).unwrap();
        session
            .dispatch(Command::Batch {
                commands: vec![
                    Command::RenameBone {
                        bone,
                        name: "face-bone".into(),
                    },
                    Command::RenameSlot {
                        slot,
                        name: "face-slot".into(),
                    },
                ],
            })
            .unwrap();
        let after = session.document().clone();
        assert!(session.is_dirty());
        assert_eq!(after.bones[bone].name, "face-bone");
        assert_eq!(after.slots[slot].name, "face-slot");
        for (index, value) in after.bones.iter().enumerate() {
            assert_eq!(value.parent, before.bones[index].parent);
            assert_eq!(value.setup, before.bones[index].setup);
        }
        assert_eq!(after.animations, before.animations);
        for (skin, entries) in &before.skins {
            if let Some(attachments) = entries.get("head") {
                assert_eq!(&after.skins[skin]["face-slot"], attachments);
                assert!(!after.skins[skin].contains_key("head"));
            }
            let old = evaluate_scene(&before, Some(0), 0.35, skin, None).unwrap();
            let new = evaluate_scene(&after, Some(0), 0.35, skin, None).unwrap();
            let affine_values = |world: &[crate::Affine]| {
                world
                    .iter()
                    .map(|p| [p.a, p.b, p.c, p.d, p.x, p.y])
                    .collect::<Vec<_>>()
            };
            assert_eq!(affine_values(&old.world), affine_values(&new.world));
            assert_eq!(old.attachments.len(), new.attachments.len());
            for (old, new) in old.attachments.iter().zip(&new.attachments) {
                assert_eq!(old.slot, new.slot);
                assert_eq!(old.vertices, new.vertices);
                assert_eq!(old.uvs, new.uvs);
                assert_eq!(old.image, new.image);
            }
        }
        let encoded = serde_json::to_string(&after).unwrap();
        assert_eq!(serde_json::from_str::<Document>(&encoded).unwrap(), after);
        session.undo();
        assert_eq!(session.document(), &before);
        assert!(!session.is_dirty());
        session.redo();
        assert_eq!(session.document(), &after);
    }

    #[test]
    fn invalid_names_and_targets_preserve_document_and_redo_branch() {
        let before = original();
        let mut session = Session::new(before.clone()).unwrap();
        session
            .dispatch(Command::RenameBone {
                bone: 0,
                name: "renamed-root".into(),
            })
            .unwrap();
        session.undo();
        for command in [
            Command::RenameBone {
                bone: 0,
                name: "  ".into(),
            },
            Command::RenameBone {
                bone: 0,
                name: before.bones[1].name.clone(),
            },
            Command::RenameBone {
                bone: usize::MAX,
                name: "root".into(),
            },
            Command::RenameSlot {
                slot: 0,
                name: "".into(),
            },
            Command::RenameSlot {
                slot: 0,
                name: before.slots[1].name.clone(),
            },
            Command::RenameSlot {
                slot: usize::MAX,
                name: "slot".into(),
            },
            Command::Batch {
                commands: vec![
                    Command::RenameSlot {
                        slot: 0,
                        name: "safe-first-step".into(),
                    },
                    Command::RenameBone {
                        bone: 0,
                        name: "".into(),
                    },
                ],
            },
        ] {
            assert!(session.dispatch(command).is_err());
            assert_eq!(session.document(), &before);
            assert!(session.can_redo());
            assert!(!session.can_undo());
        }
        let command = Command::RenameSlot {
            slot: 0,
            name: "new-slot".into(),
        };
        let value = serde_json::to_value(command).unwrap();
        assert_eq!(value["command"], "renameSlot");
        let decoded: Command = serde_json::from_value(value).unwrap();
        assert!(apply_command(&before, decoded).is_ok());
    }
}
