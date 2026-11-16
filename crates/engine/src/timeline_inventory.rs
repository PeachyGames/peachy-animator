//! Authored typed-key inventory, independent of skin, visibility and selection.
//! Unrepresented timeline families remain explicit read-only metadata.
use crate::{AnimationKey, KeyOwner};
use peachy_document::{Animation, AttachmentRef, Document, PathProperty, PhysicsProperty};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// Authored timelines without an editable AnimationKey identity. Times are the
/// exact stored f32 values widened to f64; these descriptors are not edit IDs.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UnsupportedAnimationTimeline {
    Inherit {
        bone: usize,
        times: Vec<f64>,
    },
    Sequence {
        attachment: AttachmentRef,
        times: Vec<f64>,
    },
    Path {
        constraint: usize,
        property: PathProperty,
        times: Vec<f64>,
    },
    Physics {
        constraint: Option<usize>,
        property: PhysicsProperty,
        times: Vec<f64>,
    },
    PhysicsReset {
        constraint: Option<usize>,
        times: Vec<f64>,
    },
    Event {
        times: Vec<f64>,
    },
}
impl UnsupportedAnimationTimeline {
    pub fn times(&self) -> &[f64] {
        match self {
            Self::Inherit { times, .. }
            | Self::Sequence { times, .. }
            | Self::Path { times, .. }
            | Self::Physics { times, .. }
            | Self::PhysicsReset { times, .. }
            | Self::Event { times } => times,
        }
    }
    fn belongs_to(&self, document: &Document, owners: &BTreeSet<KeyOwner>) -> bool {
        match self {
            Self::Inherit { bone, .. } => owners.contains(&KeyOwner::Bone { bone: *bone }),
            Self::Sequence { attachment, .. } => {
                owners.contains(&KeyOwner::Deform {
                    attachment: document.deform_target(attachment),
                }) || document
                    .slots
                    .iter()
                    .position(|slot| slot.name == attachment.slot)
                    .is_some_and(|slot| owners.contains(&KeyOwner::Slot { slot }))
            }
            // These families have no representable owner in KeyOwner. The
            // complete inventory still exposes their actual owners and times.
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationKeyInventory {
    pub owners: Vec<KeyOwner>,
    pub unsupported_timelines: Vec<UnsupportedAnimationTimeline>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationKeyQuery {
    pub keys: Vec<AnimationKey>,
    pub unsupported_timelines: Vec<UnsupportedAnimationTimeline>,
}

fn checked_clip(document: &Document, animation: usize) -> Result<&Animation, String> {
    document.validate()?;
    document
        .animations
        .get(animation)
        .ok_or_else(|| "Unknown animation".into())
}

fn unsupported(clip: &Animation) -> Vec<UnsupportedAnimationTimeline> {
    let mut result = Vec::new();
    for track in &clip.inherit_tracks {
        if !track.keys.is_empty() {
            result.push(UnsupportedAnimationTimeline::Inherit {
                bone: track.bone,
                times: track.keys.iter().map(|key| f64::from(key.time)).collect(),
            });
        }
    }
    for track in &clip.sequence_tracks {
        if !track.keys.is_empty() {
            result.push(UnsupportedAnimationTimeline::Sequence {
                attachment: track.attachment.clone(),
                times: track.keys.iter().map(|key| f64::from(key.time)).collect(),
            });
        }
    }
    for track in &clip.path_tracks {
        if !track.keys.is_empty() {
            result.push(UnsupportedAnimationTimeline::Path {
                constraint: track.constraint,
                property: track.property,
                times: track.keys.iter().map(|key| f64::from(key.time)).collect(),
            });
        }
    }
    for track in &clip.physics {
        if !track.keys.is_empty() {
            result.push(UnsupportedAnimationTimeline::Physics {
                constraint: track.constraint,
                property: track.property,
                times: track.keys.iter().map(|key| f64::from(key.time)).collect(),
            });
        }
    }
    for track in &clip.physics_resets {
        if !track.times.is_empty() {
            result.push(UnsupportedAnimationTimeline::PhysicsReset {
                constraint: track.constraint,
                times: track.times.iter().copied().map(f64::from).collect(),
            });
        }
    }
    if !clip.events.is_empty() {
        result.push(UnsupportedAnimationTimeline::Event {
            times: clip.events.iter().map(|key| f64::from(key.time)).collect(),
        });
    }
    result
}

fn owner_order(owner: &KeyOwner) -> (u8, usize, &str, &str, &str) {
    match owner {
        KeyOwner::Bone { bone } => (0, *bone, "", "", ""),
        KeyOwner::Ik { constraint } | KeyOwner::Slider { constraint } => {
            (1, *constraint, "", "", "")
        }
        KeyOwner::Deform { attachment } => (
            2,
            0,
            &attachment.skin,
            &attachment.slot,
            &attachment.placeholder,
        ),
        KeyOwner::Slot { slot } => (3, *slot, "", "", ""),
        KeyOwner::DrawOrder {} => (4, 0, "", "", ""),
    }
}

/// Represented authored owners in deterministic order: bone index, combined
/// IK/Slider constraint index, canonical Deform skin/slot/placeholder, slot
/// index, then global DrawOrder. Empty tracks contribute no owner. Unsupported
/// nonempty timelines are retained separately in family/serialized-track order.
/// This query never filters by active skin, shown attachment or hidden rows.
pub fn animation_key_inventory(
    document: &Document,
    animation: usize,
) -> Result<AnimationKeyInventory, String> {
    let clip = checked_clip(document, animation)?;
    let mut owners = BTreeSet::new();
    owners.extend(
        clip.tracks
            .iter()
            .filter(|track| !track.keys.is_empty())
            .map(|track| KeyOwner::Bone { bone: track.bone }),
    );
    owners.extend(
        clip.ik_tracks
            .iter()
            .filter(|track| !track.keys.is_empty())
            .map(|track| KeyOwner::Ik {
                constraint: track.constraint,
            }),
    );
    owners.extend(
        clip.slider_tracks
            .iter()
            .filter(|track| !track.keys.is_empty())
            .map(|track| KeyOwner::Slider {
                constraint: track.constraint,
            }),
    );
    for track in clip
        .deform_tracks
        .iter()
        .filter(|track| !track.keys.is_empty())
    {
        owners.insert(crate::timeline::canonical_owner(
            document,
            KeyOwner::Deform {
                attachment: track.attachment.clone(),
            },
        )?);
    }
    owners.extend(
        clip.attachment_tracks
            .iter()
            .filter(|track| !track.keys.is_empty())
            .map(|track| KeyOwner::Slot { slot: track.slot }),
    );
    owners.extend(
        clip.color_tracks
            .iter()
            .filter(|track| !track.keys.is_empty())
            .map(|track| KeyOwner::Slot { slot: track.slot }),
    );
    if !clip.draw_order.is_empty() {
        owners.insert(KeyOwner::DrawOrder {});
    }
    let mut owners: Vec<_> = owners.into_iter().collect();
    owners.sort_by(|a, b| owner_order(a).cmp(&owner_order(b)));
    Ok(AnimationKeyInventory {
        owners,
        unsupported_timelines: unsupported(clip),
    })
}

/// Represented-only convenience. A full Dopesheet must use the structured
/// inventory above so read-only timeline families cannot disappear silently.
pub fn animation_key_owners(
    document: &Document,
    animation: usize,
) -> Result<Vec<KeyOwner>, String> {
    Ok(animation_key_inventory(document, animation)?.owners)
}

fn extend_keys(
    selected: &mut BTreeMap<KeyOwner, Vec<AnimationKey>>,
    owner: KeyOwner,
    keys: impl IntoIterator<Item = AnimationKey>,
) {
    if let Some(values) = selected.get_mut(&owner) {
        values.extend(keys);
    }
}

/// Query represented keys for explicit owners, preserving owner request order,
/// then channel order and exact time within each owner. Unknown owners and
/// duplicate canonical owners (including linked aliases) fail atomically.
/// Empty requested owners mean no keys and no scoped read-only descriptors.
/// Inherit/Sequence metadata within requested owners is explicit; unrelated
/// unrepresented timelines remain available in animation_key_inventory.
pub fn animation_keys(
    document: &Document,
    animation: usize,
    owners: &[KeyOwner],
) -> Result<AnimationKeyQuery, String> {
    let clip = checked_clip(document, animation)?;
    let mut requested = Vec::with_capacity(owners.len());
    let mut selected = BTreeMap::new();
    for owner in owners {
        let canonical = crate::timeline::canonical_owner(document, owner.clone())?;
        if selected.insert(canonical.clone(), Vec::new()).is_some() {
            return Err("Duplicate canonical animation key owner".into());
        }
        requested.push(canonical);
    }
    for track in &clip.tracks {
        extend_keys(
            &mut selected,
            KeyOwner::Bone { bone: track.bone },
            track.keys.iter().map(|key| AnimationKey::Bone {
                bone: track.bone,
                property: track.property,
                time: key.time,
            }),
        );
    }
    for track in &clip.ik_tracks {
        extend_keys(
            &mut selected,
            KeyOwner::Ik {
                constraint: track.constraint,
            },
            track.keys.iter().map(|key| AnimationKey::Ik {
                constraint: track.constraint,
                time: key.time,
            }),
        );
    }
    for track in &clip.slider_tracks {
        extend_keys(
            &mut selected,
            KeyOwner::Slider {
                constraint: track.constraint,
            },
            track.keys.iter().map(|key| AnimationKey::Slider {
                constraint: track.constraint,
                property: track.property,
                time: key.time,
            }),
        );
    }
    for track in &clip.deform_tracks {
        extend_keys(
            &mut selected,
            KeyOwner::Deform {
                attachment: track.attachment.clone(),
            },
            track.keys.iter().map(|key| AnimationKey::Deform {
                attachment: track.attachment.clone(),
                time: key.time,
            }),
        );
    }
    for track in &clip.attachment_tracks {
        extend_keys(
            &mut selected,
            KeyOwner::Slot { slot: track.slot },
            track.keys.iter().map(|key| AnimationKey::SlotAttachment {
                slot: track.slot,
                time: key.time,
            }),
        );
    }
    for track in &clip.color_tracks {
        extend_keys(
            &mut selected,
            KeyOwner::Slot { slot: track.slot },
            track.keys.iter().map(|key| AnimationKey::SlotColor {
                slot: track.slot,
                time: key.time,
            }),
        );
    }
    extend_keys(
        &mut selected,
        KeyOwner::DrawOrder {},
        clip.draw_order
            .iter()
            .map(|key| AnimationKey::DrawOrder { time: key.time }),
    );
    let owner_set = requested.iter().cloned().collect();
    let unsupported_timelines = unsupported(clip)
        .into_iter()
        .filter(|track| track.belongs_to(document, &owner_set))
        .collect();
    let mut keys = Vec::new();
    for owner in requested {
        let mut values = selected
            .remove(&owner)
            .expect("Requested owner was inserted above");
        values.sort_by_key(AnimationKey::id);
        keys.extend(values);
    }
    Ok(AnimationKeyQuery {
        keys,
        unsupported_timelines,
    })
}
