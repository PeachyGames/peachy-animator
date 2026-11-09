//! Exact typed key identities and shared timing transitions for bone/IK tracks.
use peachy_document::{Curve, Document, IkKeyframe, IkTrack, Keyframe, Property, Track};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineKey {
    pub bone: usize,
    pub property: Property,
    pub time: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipboardKey {
    pub property: Property,
    pub key: Keyframe,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyClipboard {
    pub keys: Vec<ClipboardKey>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum KeyOwner {
    Bone { bone: usize },
    Ik { constraint: usize },
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum AnimationKey {
    Bone {
        bone: usize,
        property: Property,
        time: f32,
    },
    Ik {
        constraint: usize,
        time: f32,
    },
}
impl AnimationKey {
    pub fn owner(self) -> KeyOwner {
        match self {
            Self::Bone { bone, .. } => KeyOwner::Bone { bone },
            Self::Ik { constraint, .. } => KeyOwner::Ik { constraint },
        }
    }
    pub fn time(self) -> f32 {
        match self {
            Self::Bone { time, .. } | Self::Ik { time, .. } => time,
        }
    }
    fn id(self) -> (KeyOwner, Option<Property>, u32) {
        (
            self.owner(),
            match self {
                Self::Bone { property, .. } => Some(property),
                Self::Ik { .. } => None,
            },
            time_id(self.time()),
        )
    }
}
impl From<TimelineKey> for AnimationKey {
    fn from(k: TimelineKey) -> Self {
        Self::Bone {
            bone: k.bone,
            property: k.property,
            time: k.time,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum AnimationKeyClipboard {
    Bone { keys: Vec<ClipboardKey> },
    Ik { keys: Vec<IkKeyframe> },
}

#[derive(Clone, Copy)]
enum OwnedKey {
    Bone {
        bone: usize,
        property: Property,
        key: Keyframe,
    },
    Ik {
        constraint: usize,
        key: IkKeyframe,
    },
}
impl OwnedKey {
    fn identity(self) -> AnimationKey {
        match self {
            Self::Bone {
                bone,
                property,
                key,
            } => AnimationKey::Bone {
                bone,
                property,
                time: key.time,
            },
            Self::Ik { constraint, key } => AnimationKey::Ik {
                constraint,
                time: key.time,
            },
        }
    }
    fn at(self, time: f32) -> Self {
        match self {
            Self::Bone {
                bone,
                property,
                key,
            } => Self::Bone {
                bone,
                property,
                key: Keyframe { time, ..key },
            },
            Self::Ik { constraint, key } => Self::Ik {
                constraint,
                key: IkKeyframe { time, ..key },
            },
        }
    }
}
fn time_id(time: f32) -> u32 {
    if time == 0. { 0 } else { time.to_bits() }
}
fn checked_time(time: f32) -> Result<f32, String> {
    if !time.is_finite() || time < 0. {
        return Err("Key time must be finite and nonnegative".into());
    }
    Ok(time)
}
fn validate_owner(document: &Document, owner: KeyOwner) -> Result<(), String> {
    match owner {
        KeyOwner::Bone { bone } if bone >= document.bones.len() => Err("Unknown bone".into()),
        KeyOwner::Ik { constraint } if constraint >= document.constraints.len() => {
            Err("Unknown IK constraint".into())
        }
        _ => Ok(()),
    }
}
fn sources(
    document: &Document,
    animation: usize,
    keys: &[AnimationKey],
) -> Result<Vec<OwnedKey>, String> {
    let clip = document
        .animations
        .get(animation)
        .ok_or("Unknown animation")?;
    if keys.is_empty() {
        return Err("Select at least one key".into());
    }
    let mut identities = BTreeSet::new();
    keys.iter()
        .map(|&source| {
            checked_time(source.time())?;
            validate_owner(document, source.owner())?;
            if !identities.insert(source.id()) {
                return Err("Duplicate source key".into());
            }
            match source {
                AnimationKey::Bone {
                    bone,
                    property,
                    time,
                } => clip
                    .tracks
                    .iter()
                    .find(|t| t.bone == bone && t.property == property)
                    .and_then(|t| t.keys.iter().find(|k| k.time == time))
                    .copied()
                    .map(|key| OwnedKey::Bone {
                        bone,
                        property,
                        key,
                    }),
                AnimationKey::Ik { constraint, time } => clip
                    .ik_tracks
                    .iter()
                    .find(|t| t.constraint == constraint)
                    .and_then(|t| t.keys.iter().find(|k| k.time == time))
                    .copied()
                    .map(|key| OwnedKey::Ik { constraint, key }),
            }
            .ok_or_else(|| "Unknown exact source key".into())
        })
        .collect()
}
/// Capture owned values/curves from one typed owner; relative times start at zero.
pub fn copy_animation_keys(
    document: &Document,
    animation: usize,
    keys: &[AnimationKey],
) -> Result<AnimationKeyClipboard, String> {
    document.validate()?;
    let sources = sources(document, animation, keys)?;
    let owner = sources[0].identity().owner();
    if sources.iter().any(|s| s.identity().owner() != owner) {
        return Err("Clipboard supports keys from one typed owner".into());
    }
    let first = sources
        .iter()
        .map(|s| s.identity().time())
        .min_by(f32::total_cmp)
        .ok_or("No keys")?;
    let clipboard = match owner {
        KeyOwner::Bone { .. } => {
            let mut keys = vec![];
            for source in sources {
                if let OwnedKey::Bone { property, key, .. } = source {
                    keys.push(ClipboardKey {
                        property,
                        key: Keyframe {
                            time: key.time - first,
                            ..key
                        },
                    });
                }
            }
            keys.sort_by(|a, b| {
                a.key
                    .time
                    .total_cmp(&b.key.time)
                    .then(a.property.cmp(&b.property))
            });
            AnimationKeyClipboard::Bone { keys }
        }
        KeyOwner::Ik { .. } => {
            let mut keys = vec![];
            for source in sources {
                if let OwnedKey::Ik { key, .. } = source {
                    keys.push(IkKeyframe {
                        time: key.time - first,
                        ..key
                    });
                }
            }
            keys.sort_by(|a, b| a.time.total_cmp(&b.time));
            AnimationKeyClipboard::Ik { keys }
        }
    };
    clipboard_sources(&clipboard, owner)?;
    Ok(clipboard)
}
/// Backwards-compatible bone-only entry point, using the same resolver/arithmetic.
pub fn copy_keys(
    document: &Document,
    animation: usize,
    keys: &[TimelineKey],
) -> Result<KeyClipboard, String> {
    let AnimationKeyClipboard::Bone { keys } = copy_animation_keys(
        document,
        animation,
        &keys.iter().copied().map(Into::into).collect::<Vec<_>>(),
    )?
    else {
        return Err("Expected bone clipboard".into());
    };
    Ok(KeyClipboard { keys })
}
fn clipboard_sources(
    clipboard: &AnimationKeyClipboard,
    owner: KeyOwner,
) -> Result<Vec<OwnedKey>, String> {
    let sources: Vec<_> = match (clipboard, owner) {
        (AnimationKeyClipboard::Bone { keys }, KeyOwner::Bone { bone }) => {
            let mut result = vec![];
            for entry in keys {
                checked_time(entry.key.time)?;
                if !entry.key.value.is_finite() {
                    return Err("Clipboard values must be finite".into());
                }
                if let Curve::Bezier(c) = entry.key.curve
                    && (!c.iter().all(|v| v.is_finite())
                        || !(0.0..=1.0).contains(&c[0])
                        || !(0.0..=1.0).contains(&c[2])
                        || c[0] > c[2])
                {
                    return Err("Clipboard Bezier time handles must be ordered within 0–1".into());
                }
                result.push(OwnedKey::Bone {
                    bone,
                    property: entry.property,
                    key: entry.key,
                });
            }
            result
        }
        (AnimationKeyClipboard::Ik { keys }, KeyOwner::Ik { constraint }) => {
            let mut result = vec![];
            for &key in keys {
                key.validate()?;
                result.push(OwnedKey::Ik { constraint, key });
            }
            result
        }
        _ => return Err("Clipboard and paste target must have the same kind".into()),
    };
    if sources.is_empty() || !sources.iter().any(|s| s.identity().time() == 0.) {
        return Err("Clipboard needs keys anchored at relative time zero".into());
    }
    let mut identities = BTreeSet::new();
    for source in &sources {
        if !identities.insert(source.identity().id()) {
            return Err("Duplicate clipboard key".into());
        }
    }
    Ok(sources)
}
fn checked_targets(keys: &[OwnedKey], offset: f32) -> Result<Vec<OwnedKey>, String> {
    let mut identities = BTreeSet::new();
    keys.iter()
        .map(|&source| {
            let target = source.at(checked_time(source.identity().time() + offset)?);
            if !identities.insert(target.identity().id()) {
                return Err("Key times collapse at this offset".into());
            }
            Ok(target)
        })
        .collect()
}
fn insert_keys(document: &mut Document, animation: usize, keys: Vec<OwnedKey>) {
    let clip = &mut document.animations[animation];
    for incoming in keys {
        match incoming {
            OwnedKey::Bone {
                bone,
                property,
                key,
            } => {
                let index = clip
                    .tracks
                    .iter()
                    .position(|t| t.bone == bone && t.property == property)
                    .unwrap_or_else(|| {
                        clip.tracks.push(Track {
                            bone,
                            property,
                            keys: vec![],
                        });
                        clip.tracks.len() - 1
                    });
                let keys = &mut clip.tracks[index].keys;
                if let Some(existing) = keys.iter_mut().find(|k| k.time == key.time) {
                    *existing = key;
                } else {
                    keys.push(key);
                }
                keys.sort_by(|a, b| a.time.total_cmp(&b.time));
            }
            OwnedKey::Ik { constraint, key } => {
                let index = clip
                    .ik_tracks
                    .iter()
                    .position(|t| t.constraint == constraint)
                    .unwrap_or_else(|| {
                        clip.ik_tracks.push(IkTrack {
                            constraint,
                            keys: vec![],
                        });
                        clip.ik_tracks.len() - 1
                    });
                let keys = &mut clip.ik_tracks[index].keys;
                if let Some(existing) = keys.iter_mut().find(|k| k.time == key.time) {
                    *existing = key;
                } else {
                    keys.push(key);
                }
                keys.sort_by(|a, b| a.time.total_cmp(&b.time));
            }
        }
    }
}
fn remove_sources(document: &mut Document, animation: usize, sources: &[OwnedKey]) {
    let identities: BTreeSet<_> = sources.iter().map(|s| s.identity().id()).collect();
    let clip = &mut document.animations[animation];
    for track in &mut clip.tracks {
        track.keys.retain(|key| {
            !identities.contains(
                &AnimationKey::Bone {
                    bone: track.bone,
                    property: track.property,
                    time: key.time,
                }
                .id(),
            )
        });
    }
    for track in &mut clip.ik_tracks {
        track.keys.retain(|key| {
            !identities.contains(
                &AnimationKey::Ik {
                    constraint: track.constraint,
                    time: key.time,
                }
                .id(),
            )
        });
    }
}
pub(super) fn move_animation_keys(
    document: &mut Document,
    animation: usize,
    keys: &[AnimationKey],
    delta: f32,
    copy: bool,
) -> Result<(), String> {
    if !delta.is_finite() {
        return Err("Key offset must be finite".into());
    }
    let sources = sources(document, animation, keys)?;
    let targets = checked_targets(&sources, delta)?;
    if !copy {
        remove_sources(document, animation, &sources);
    }
    insert_keys(document, animation, targets);
    Ok(())
}
pub(super) fn delete_animation_keys(
    document: &mut Document,
    animation: usize,
    keys: &[AnimationKey],
) -> Result<(), String> {
    let sources = sources(document, animation, keys)?;
    remove_sources(document, animation, &sources);
    let clip = &mut document.animations[animation];
    clip.tracks.retain(|t| !t.keys.is_empty());
    clip.ik_tracks.retain(|t| !t.keys.is_empty());
    Ok(())
}
pub(super) fn paste_animation_keys(
    document: &mut Document,
    animation: usize,
    owner: KeyOwner,
    time: f32,
    clipboard: &AnimationKeyClipboard,
) -> Result<(), String> {
    checked_time(time)?;
    document
        .animations
        .get(animation)
        .ok_or("Unknown animation")?;
    validate_owner(document, owner)?;
    let sources = clipboard_sources(clipboard, owner)?;
    let targets = checked_targets(&sources, time)?;
    insert_keys(document, animation, targets);
    Ok(())
}
pub(super) fn move_keys(
    document: &mut Document,
    animation: usize,
    keys: &[TimelineKey],
    delta: f32,
    copy: bool,
) -> Result<(), String> {
    move_animation_keys(
        document,
        animation,
        &keys.iter().copied().map(Into::into).collect::<Vec<_>>(),
        delta,
        copy,
    )
}
pub(super) fn paste_keys(
    document: &mut Document,
    animation: usize,
    bone: usize,
    time: f32,
    clipboard: &KeyClipboard,
) -> Result<(), String> {
    paste_animation_keys(
        document,
        animation,
        KeyOwner::Bone { bone },
        time,
        &AnimationKeyClipboard::Bone {
            keys: clipboard.keys.clone(),
        },
    )
}
