use crate::{KeyClipboard, TimelineKey};
use peachy_document::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "camelCase", deny_unknown_fields)]
pub enum Command {
    SetPose {
        bone: usize,
        target: EditTarget,
        transform: Transform,
        properties: Vec<Property>,
    },
    SetSetup {
        bone: usize,
        transform: Transform,
    },
    RenameBone {
        bone: usize,
        name: String,
    },
    RenameSlot {
        slot: usize,
        name: String,
    },
    SetBoneLength {
        bone: usize,
        length: f32,
    },
    SetRegionTransform {
        attachment: AttachmentRef,
        transform: Transform,
    },
    SetSlotAttachment {
        slot: usize,
        attachment: Option<String>,
    },
    SetKey {
        animation: usize,
        bone: usize,
        property: Property,
        key: Keyframe,
    },
    DeleteKey {
        animation: usize,
        bone: usize,
        property: Property,
        time: f32,
    },
    MoveKeys {
        animation: usize,
        keys: Vec<TimelineKey>,
        delta: f32,
        copy: bool,
    },
    PasteKeys {
        animation: usize,
        bone: usize,
        time: f32,
        clipboard: KeyClipboard,
    },
    AddBone {
        bone: Bone,
    },
    AddAnimation {
        name: String,
        duration: f32,
    },
    RenameAnimation {
        animation: usize,
        name: String,
    },
    DuplicateAnimation {
        animation: usize,
        name: String,
    },
    ConvertRegionToMesh {
        attachment: AttachmentRef,
    },
    SetMeshVertex {
        attachment: AttachmentRef,
        vertex: usize,
        position: [f32; 2],
    },
    SplitMeshTriangle {
        attachment: AttachmentRef,
        triangle: usize,
    },
    Batch {
        commands: Vec<Command>,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase", deny_unknown_fields)]
pub enum EditTarget {
    Setup,
    Animate { animation: usize, time: f32 },
}

/// Pure document transition. Neither the input nor history changes on failure.
pub fn apply_command(document: &Document, command: Command) -> Result<Document, String> {
    document.validate()?;
    let mut candidate = document.clone();
    apply(&mut candidate, command)?;
    candidate.validate()?;
    Ok(candidate)
}

fn apply(doc: &mut Document, command: Command) -> Result<(), String> {
    match command {
        Command::SetPose {
            bone,
            target,
            transform,
            properties,
        } => {
            let setup = doc.bones.get(bone).ok_or("Unknown bone")?.setup;
            if !finite_transform(transform) {
                return Err("Pose must be finite".into());
            }
            match target {
                EditTarget::Setup => doc.bones[bone].setup = transform,
                EditTarget::Animate { animation, time } => {
                    let clip = doc.animations.get(animation).ok_or("Unknown animation")?;
                    if !time.is_finite() || time < 0.0 {
                        return Err("Key time must be finite and nonnegative".into());
                    }
                    // Wider intermediates keep every finite authoring time valid,
                    // even when its frame number exceeds the f32 range.
                    let time = ((f64::from(time) * f64::from(doc.fps)).round() / f64::from(doc.fps))
                        as f32;
                    let commands: Result<Vec<_>, String> = properties
                        .into_iter()
                        .map(|property| {
                            let base = property.read(setup);
                            let value = if property.is_scale() {
                                if base.abs() < 1e-8 {
                                    return Err("Cannot animate a zero setup scale".into());
                                }
                                property.read(transform) / base
                            } else {
                                property.read(transform) - base
                            };
                            let curve = match clip
                                .tracks
                                .iter()
                                .find(|t| t.bone == bone && t.property == property)
                            {
                                Some(track) => key_index(&track.keys, time)?
                                    .map_or(Curve::Linear, |index| track.keys[index].curve),
                                None => Curve::Linear,
                            };
                            Ok(Command::SetKey {
                                animation,
                                bone,
                                property,
                                key: Keyframe { time, value, curve },
                            })
                        })
                        .collect();
                    for command in commands? {
                        apply(doc, command)?;
                    }
                }
            }
        }
        Command::SetSetup { bone, transform } => {
            doc.bones.get_mut(bone).ok_or("Unknown bone")?.setup = transform
        }
        Command::RenameBone { bone, name } => crate::names::rename_bone(doc, bone, name)?,
        Command::RenameSlot { slot, name } => crate::names::rename_slot(doc, slot, name)?,
        Command::SetBoneLength { bone, length } => {
            doc.bones.get_mut(bone).ok_or("Unknown bone")?.length = length;
        }
        Command::SetRegionTransform {
            attachment,
            transform,
        } => {
            let Some(Attachment::Region(region)) = doc.attachment_mut(&attachment) else {
                return Err("Unknown region attachment".into());
            };
            region.transform = transform;
        }
        Command::SetSlotAttachment { slot, attachment } => {
            let target = doc.slots.get(slot).ok_or("Unknown slot")?;
            if let Some(placeholder) = &attachment
                && !doc.skins.values().any(|skin| {
                    skin.get(&target.name)
                        .is_some_and(|entries| entries.contains_key(placeholder))
                })
            {
                return Err("Unknown attachment placeholder for slot".into());
            }
            doc.slots[slot].attachment = attachment;
        }
        Command::ConvertRegionToMesh { attachment } => {
            let value = doc
                .attachment_mut(&attachment)
                .ok_or("Unknown attachment")?;
            let Attachment::Region(region) = value else {
                return Err("Attachment is already a mesh".into());
            };
            *value = Attachment::Mesh(crate::region_to_mesh(region));
        }
        Command::SetMeshVertex {
            attachment,
            vertex,
            position,
        } => {
            let Some(Attachment::Mesh(mesh)) = doc.attachment_mut(&attachment) else {
                return Err("Unknown mesh attachment".into());
            };
            let positions = mesh
                .slot_local_positions_mut()
                .ok_or("Weighted mesh vertex editing is not implemented")?;
            *positions.get_mut(vertex).ok_or("Unknown mesh vertex")? = position;
        }
        Command::SplitMeshTriangle {
            attachment,
            triangle,
        } => {
            let Some(Attachment::Mesh(mesh)) = doc.attachment_mut(&attachment) else {
                return Err("Unknown mesh attachment".into());
            };
            crate::mesh::split_triangle(mesh, triangle)?;
        }
        Command::AddBone { bone } => doc.bones.push(bone),
        Command::AddAnimation { name, duration } => {
            if !duration.is_finite() || duration < 0.0 {
                return Err("Animation duration must be finite and nonnegative".into());
            }
            let tracks = if duration == 0.0 {
                vec![]
            } else {
                vec![Track {
                    bone: 0,
                    property: Property::Rotation,
                    keys: vec![Keyframe {
                        time: duration,
                        value: Property::Rotation.neutral(),
                        curve: Curve::Linear,
                    }],
                }]
            };
            doc.animations.push(Animation { name, tracks });
        }
        Command::RenameAnimation { animation, name } => {
            doc.animations
                .get_mut(animation)
                .ok_or("Unknown animation")?
                .name = name;
        }
        Command::DuplicateAnimation { animation, name } => {
            let mut duplicate = doc
                .animations
                .get(animation)
                .ok_or("Unknown animation")?
                .clone();
            duplicate.name = name;
            doc.animations.push(duplicate);
        }
        Command::SetKey {
            animation,
            bone,
            property,
            key,
        } => {
            let a = doc
                .animations
                .get_mut(animation)
                .ok_or("Unknown animation")?;
            if !key.time.is_finite() || !key.value.is_finite() {
                return Err("Keyframe must be finite".into());
            }
            let index = match a
                .tracks
                .iter()
                .position(|t| t.bone == bone && t.property == property)
            {
                Some(i) => i,
                None => {
                    a.tracks.push(Track {
                        bone,
                        property,
                        keys: vec![],
                    });
                    a.tracks.len() - 1
                }
            };
            let keys = &mut a.tracks[index].keys;
            if let Some(i) = key_index(keys, key.time)? {
                keys[i] = key;
            } else {
                keys.push(key);
            }
            keys.sort_by(|a, b| a.time.total_cmp(&b.time));
        }
        Command::DeleteKey {
            animation,
            bone,
            property,
            time,
        } => {
            if bone >= doc.bones.len() {
                return Err("Unknown bone".into());
            }
            let a = doc
                .animations
                .get_mut(animation)
                .ok_or("Unknown animation")?;
            // This is a source query, not an authoring cursor. Reject a query
            // past the end before a nearby final key could satisfy tolerance.
            if !time.is_finite() || time < 0.0 || time > a.duration() {
                return Err("Delete time must be finite and inside the animation".into());
            }
            for track in &mut a.tracks {
                if track.bone == bone
                    && track.property == property
                    && let Some(index) = key_index(&track.keys, time)?
                {
                    track.keys.remove(index);
                }
            }
            a.tracks.retain(|t| !t.keys.is_empty());
        }
        Command::MoveKeys {
            animation,
            keys,
            delta,
            copy,
        } => {
            crate::timeline::move_keys(doc, animation, &keys, delta, copy)?;
        }
        Command::PasteKeys {
            animation,
            bone,
            time,
            clipboard,
        } => {
            crate::timeline::paste_keys(doc, animation, bone, time, &clipboard)?;
        }
        Command::Batch { commands } => {
            for command in commands {
                apply(doc, command)?;
            }
        }
    }
    Ok(())
}

/// Query an exact key first, or one unambiguous nearby key without changing its source time.
pub fn keyframe_at(keys: &[Keyframe], time: f32) -> Result<Option<Keyframe>, String> {
    if !time.is_finite() || time < 0.0 {
        return Err("Key time must be finite and nonnegative".into());
    }
    key_index(keys, time).map(|index| index.map(|index| keys[index]))
}

/// Exact identities win; tolerance is only safe for one unambiguous nearby key.
fn key_index(keys: &[Keyframe], time: f32) -> Result<Option<usize>, String> {
    if let Some(index) = keys.iter().position(|key| key.time == time) {
        return Ok(Some(index));
    }
    let mut nearby = keys
        .iter()
        .enumerate()
        .filter(|(_, key)| same_key_time(key.time, time));
    let index = nearby.next().map(|(index, _)| index);
    if nearby.next().is_some() {
        return Err("Ambiguous key time; select an exact source key".into());
    }
    Ok(index)
}
