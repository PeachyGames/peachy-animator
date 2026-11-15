//! Exact typed key identities and shared timing transitions for bone/IK/deform/slot tracks.
use peachy_document::{
    Animation, Attachment, AttachmentKey, AttachmentRef, ColorKey, Constraint, Curve, DeformKey,
    DeformTrack, Document, IkCurve, IkKeyframe, IkTrack, Keyframe, MeshVertices, Property, Track,
};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::BTreeSet};

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum KeyOwner {
    Bone { bone: usize },
    Ik { constraint: usize },
    Deform { attachment: AttachmentRef },
    Slot { slot: usize },
}
impl KeyOwner {
    fn ordering_key(&self) -> (u8, usize, &str, &str, &str) {
        match self {
            Self::Bone { bone } => (0, *bone, "", "", ""),
            Self::Ik { constraint } => (1, *constraint, "", "", ""),
            Self::Slot { slot } => (3, *slot, "", "", ""),
            Self::Deform { attachment } => (
                2,
                0,
                &attachment.skin,
                &attachment.slot,
                &attachment.placeholder,
            ),
        }
    }
}
impl Ord for KeyOwner {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ordering_key().cmp(&other.ordering_key())
    }
}
impl PartialOrd for KeyOwner {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    Deform {
        attachment: AttachmentRef,
        time: f32,
    },
    SlotAttachment {
        slot: usize,
        time: f32,
    },
    SlotColor {
        slot: usize,
        time: f32,
    },
}

/// A presentation curve interval, separate from the selected source key.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveTarget {
    pub key: AnimationKey,
    pub next_time: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CurveData {
    Standard(Curve),
    Ik(IkCurve),
}

/// Raw channel data in its native value domain, with no flat normalization,
/// preset defaults or handle-coupling policy applied by this read-only query.
#[derive(Clone, Debug, PartialEq)]
pub struct CurveChannel {
    pub target: CurveTarget,
    pub channel: usize,
    pub values: [f64; 2],
    pub curve: CurveData,
}

impl AnimationKey {
    pub fn owner(&self) -> KeyOwner {
        match self {
            Self::Bone { bone, .. } => KeyOwner::Bone { bone: *bone },
            Self::Ik { constraint, .. } => KeyOwner::Ik {
                constraint: *constraint,
            },
            Self::Deform { attachment, .. } => KeyOwner::Deform {
                attachment: attachment.clone(),
            },
            Self::SlotAttachment { slot, .. } | Self::SlotColor { slot, .. } => {
                KeyOwner::Slot { slot: *slot }
            }
        }
    }
    pub fn time(&self) -> f32 {
        match self {
            Self::Bone { time, .. }
            | Self::Ik { time, .. }
            | Self::Deform { time, .. }
            | Self::SlotAttachment { time, .. }
            | Self::SlotColor { time, .. } => *time,
        }
    }
    pub(crate) fn at(&self, time: f32) -> Self {
        let mut key = self.clone();
        match &mut key {
            Self::Bone { time: at, .. }
            | Self::Ik { time: at, .. }
            | Self::Deform { time: at, .. }
            | Self::SlotAttachment { time: at, .. }
            | Self::SlotColor { time: at, .. } => *at = time,
        }
        key
    }
    pub(crate) fn id(&self) -> (KeyOwner, u8, Option<Property>, u32) {
        (
            self.owner(),
            match self {
                Self::Bone { .. } => 0,
                Self::Ik { .. } => 1,
                Self::Deform { .. } => 2,
                Self::SlotAttachment { .. } => 3,
                Self::SlotColor { .. } => 4,
            },
            match self {
                Self::Bone { property, .. } => Some(*property),
                Self::Ik { .. }
                | Self::Deform { .. }
                | Self::SlotAttachment { .. }
                | Self::SlotColor { .. } => None,
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

/// Dense offsets belong to logical vertices, or ordered influences within each
/// vertex. Equal flat lengths alone do not establish a compatible target layout.
/// Clipboard values remain local to their target's vertex/binding axes. A
/// one-influence-per-vertex weighted layout also accepts the corresponding
/// unweighted offsets verbatim (and vice versa), as observed in Spine 4.3.26.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum DeformLayout {
    Unweighted { vertices: usize },
    Weighted { influence_counts: Vec<usize> },
}
impl DeformLayout {
    fn from_vertices(vertices: &MeshVertices) -> Self {
        match vertices {
            MeshVertices::SlotLocal { positions } => Self::Unweighted {
                vertices: positions.len(),
            },
            MeshVertices::Weighted { influences } => Self::Weighted {
                influence_counts: influences.iter().map(Vec::len).collect(),
            },
        }
    }
    fn compatible_with(&self, target: &Self) -> bool {
        match (self, target) {
            (Self::Unweighted { vertices: a }, Self::Unweighted { vertices: b }) => a == b,
            (
                Self::Weighted {
                    influence_counts: a,
                },
                Self::Weighted {
                    influence_counts: b,
                },
            ) => a == b,
            (Self::Unweighted { vertices }, Self::Weighted { influence_counts })
            | (Self::Weighted { influence_counts }, Self::Unweighted { vertices }) => {
                *vertices == influence_counts.len()
                    && influence_counts.iter().all(|&count| count == 1)
            }
        }
    }
    fn offset_len(&self) -> Result<usize, String> {
        let count = match self {
            Self::Unweighted { vertices } if *vertices > 0 => *vertices,
            Self::Weighted { influence_counts }
                if !influence_counts.is_empty() && !influence_counts.contains(&0) =>
            {
                influence_counts
                    .iter()
                    .try_fold(0usize, |total, &count| total.checked_add(count))
                    .ok_or("Deform clipboard layout exceeds the offset range")?
            }
            _ => return Err("Deform clipboard needs a nonempty vertex/influence layout".into()),
        };
        count
            .checked_mul(2)
            .ok_or_else(|| "Deform clipboard layout exceeds the offset range".into())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum AnimationKeyClipboard {
    Bone {
        keys: Vec<ClipboardKey>,
    },
    Ik {
        keys: Vec<IkKeyframe>,
    },
    Deform {
        layout: DeformLayout,
        keys: Vec<DeformKey>,
    },
    Slot {
        attachment_keys: Vec<AttachmentKey>,
        color_keys: Vec<ColorKey>,
    },
}

#[derive(Clone)]
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
    Deform {
        attachment: AttachmentRef,
        key: DeformKey,
    },
    SlotAttachment {
        slot: usize,
        key: AttachmentKey,
    },
    SlotColor {
        slot: usize,
        key: ColorKey,
    },
}
impl OwnedKey {
    fn identity(&self) -> AnimationKey {
        match self {
            Self::Bone {
                bone,
                property,
                key,
            } => AnimationKey::Bone {
                bone: *bone,
                property: *property,
                time: key.time,
            },
            Self::Ik { constraint, key } => AnimationKey::Ik {
                constraint: *constraint,
                time: key.time,
            },
            Self::Deform { attachment, key } => AnimationKey::Deform {
                attachment: attachment.clone(),
                time: key.time,
            },
            Self::SlotAttachment { slot, key } => AnimationKey::SlotAttachment {
                slot: *slot,
                time: key.time,
            },
            Self::SlotColor { slot, key } => AnimationKey::SlotColor {
                slot: *slot,
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
            Self::Deform { attachment, key } => Self::Deform {
                attachment,
                key: DeformKey { time, ..key },
            },
            Self::SlotAttachment { slot, key } => Self::SlotAttachment {
                slot,
                key: AttachmentKey { time, ..key },
            },
            Self::SlotColor { slot, key } => Self::SlotColor {
                slot,
                key: ColorKey { time, ..key },
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

/// Resolve an explicitly selected deformable attachment to the owner of its
/// timeline. Linked meshes with shared timelines use their source attachment;
/// fallback skins must be resolved by the caller before constructing this ref.
pub fn deform_key_owner(
    document: &Document,
    attachment: &AttachmentRef,
) -> Result<KeyOwner, String> {
    document
        .attachment(attachment)
        .and_then(Attachment::vertices)
        .ok_or("Deform timeline needs an existing mesh, path or clipping attachment")?;
    let canonical = document.deform_target(attachment);
    document
        .attachment(&canonical)
        .and_then(Attachment::vertices)
        .ok_or("Deform timeline source is missing or not deformable")?;
    Ok(KeyOwner::Deform {
        attachment: canonical,
    })
}
fn canonical_owner(document: &Document, owner: KeyOwner) -> Result<KeyOwner, String> {
    match &owner {
        KeyOwner::Bone { bone } if *bone >= document.bones.len() => Err("Unknown bone".into()),
        KeyOwner::Slot { slot } if *slot >= document.slots.len() => Err("Unknown slot".into()),
        KeyOwner::Ik { constraint }
            if !matches!(
                document.constraints.get(*constraint),
                Some(Constraint::Ik(_))
            ) =>
        {
            Err("Unknown IK constraint".into())
        }
        KeyOwner::Deform { attachment } => deform_key_owner(document, attachment),
        _ => Ok(owner),
    }
}

pub(crate) fn canonical_key(
    document: &Document,
    key: &AnimationKey,
) -> Result<AnimationKey, String> {
    let time = checked_time(key.time())?;
    match canonical_owner(document, key.owner())? {
        KeyOwner::Deform { attachment } => Ok(AnimationKey::Deform { attachment, time }),
        _ => Ok(key.clone()),
    }
}

fn exact_interval<T>(
    keys: Option<&[T]>,
    time: f32,
    read: fn(&T) -> f32,
    single: bool,
) -> Result<Option<(f32, f32)>, String> {
    let keys = keys.ok_or("Unknown exact source key")?;
    let index = exact_index(keys, time, read)?;
    if let Some(next) = keys.get(index + 1) {
        Ok(Some((read(&keys[index]), read(next))))
    } else if single && index > 0 {
        Ok(Some((read(&keys[index - 1]), read(&keys[index]))))
    } else {
        Ok(None)
    }
}

pub(crate) fn exact_index<T>(keys: &[T], time: f32, read: fn(&T) -> f32) -> Result<usize, String> {
    keys.binary_search_by(|key| {
        let at = read(key);
        if at == time {
            Ordering::Equal
        } else {
            at.total_cmp(&time)
        }
    })
    .map_err(|_| "Unknown exact source key".into())
}

fn exact_pair<T>(
    keys: Option<&[T]>,
    time: f32,
    next_time: f32,
    read: fn(&T) -> f32,
) -> Result<(&T, &T), String> {
    let keys = keys.ok_or("Unknown exact source key")?;
    let index = exact_index(keys, time, read)?;
    let next = keys
        .get(index + 1)
        .filter(|next| read(next) == next_time)
        .ok_or("Stale curve interval; resolve the exact adjacent keys again")?;
    Ok((&keys[index], next))
}

/// Resolve Curves presentation without changing selection, cursor or data.
/// Canonical duplicate identities count once. Only a single logical selection
/// can use its previous interval when the selected key is terminal. Attachment
/// keys participate in that count but have no interpolated curve themselves.
pub fn curve_targets(
    document: &Document,
    animation: usize,
    selected: &[AnimationKey],
) -> Result<Vec<CurveTarget>, String> {
    document.validate()?;
    let clip = document
        .animations
        .get(animation)
        .ok_or("Unknown animation")?;
    let mut identities = BTreeSet::new();
    let mut canonical = vec![];
    for key in selected {
        let key = canonical_key(document, key)?;
        if identities.insert(key.id()) {
            canonical.push(key);
        }
    }
    let single = canonical.len() == 1;
    let mut result = vec![];
    for key in canonical {
        let interval = match &key {
            AnimationKey::Bone {
                bone,
                property,
                time,
            } => exact_interval(
                clip.tracks
                    .iter()
                    .find(|t| t.bone == *bone && t.property == *property)
                    .map(|t| t.keys.as_slice()),
                *time,
                |k| k.time,
                single,
            )?,
            AnimationKey::Ik { constraint, time } => exact_interval(
                clip.ik_tracks
                    .iter()
                    .find(|t| t.constraint == *constraint)
                    .map(|t| t.keys.as_slice()),
                *time,
                |k| k.time,
                single,
            )?,
            AnimationKey::Deform { attachment, time } => exact_interval(
                clip.deform_tracks
                    .iter()
                    .find(|t| t.attachment == *attachment)
                    .map(|t| t.keys.as_slice()),
                *time,
                |k| k.time,
                single,
            )?,
            AnimationKey::SlotColor { slot, time } => exact_interval(
                clip.color_tracks
                    .iter()
                    .find(|t| t.slot == *slot)
                    .map(|t| t.keys.as_slice()),
                *time,
                |k| k.time,
                single,
            )?,
            AnimationKey::SlotAttachment { slot, time } => {
                exact_interval(
                    clip.attachment_tracks
                        .iter()
                        .find(|t| t.slot == *slot)
                        .map(|t| t.keys.as_slice()),
                    *time,
                    |k| k.time,
                    false,
                )?;
                None
            }
        };
        if let Some((time, next_time)) = interval {
            result.push(CurveTarget {
                key: key.at(time),
                next_time,
            });
        }
    }
    Ok(result)
}

/// Inspect exact adjacent source keys without replacing values or converting
/// IK's f64 controls. A stale or fabricated end time is rejected explicitly.
pub fn curve_channels(
    document: &Document,
    animation: usize,
    target: &CurveTarget,
) -> Result<Vec<CurveChannel>, String> {
    curve_channels_for_targets(document, animation, std::slice::from_ref(target))
}

/// Read a bounded render snapshot after one shared validation. Callers retain
/// the complete selection and choose rendering targets before this query.
pub fn curve_channels_for_targets(
    document: &Document,
    animation: usize,
    targets: &[CurveTarget],
) -> Result<Vec<CurveChannel>, String> {
    document.validate()?;
    let clip = document
        .animations
        .get(animation)
        .ok_or("Unknown animation")?;
    let mut result = vec![];
    for target in targets {
        result.extend(channels_for_target(document, clip, target)?);
    }
    Ok(result)
}

pub(crate) fn channels_for_target(
    document: &Document,
    clip: &Animation,
    target: &CurveTarget,
) -> Result<Vec<CurveChannel>, String> {
    checked_time(target.next_time)?;
    let target = CurveTarget {
        key: canonical_key(document, &target.key)?,
        next_time: target.next_time,
    };
    let time = target.key.time();
    let channel = |values, curve| {
        vec![CurveChannel {
            target: target.clone(),
            channel: 0,
            values,
            curve,
        }]
    };
    match &target.key {
        AnimationKey::Bone { bone, property, .. } => {
            let (left, right) = exact_pair(
                clip.tracks
                    .iter()
                    .find(|t| t.bone == *bone && t.property == *property)
                    .map(|t| t.keys.as_slice()),
                time,
                target.next_time,
                |k| k.time,
            )?;
            Ok(channel(
                [f64::from(left.value), f64::from(right.value)],
                CurveData::Standard(left.curve),
            ))
        }
        AnimationKey::Ik { constraint, .. } => {
            let (left, right) = exact_pair(
                clip.ik_tracks
                    .iter()
                    .find(|t| t.constraint == *constraint)
                    .map(|t| t.keys.as_slice()),
                time,
                target.next_time,
                |k| k.time,
            )?;
            Ok(channel(
                [f64::from(left.mix), f64::from(right.mix)],
                CurveData::Ik(left.curve),
            ))
        }
        AnimationKey::Deform { attachment, .. } => {
            let (left, _) = exact_pair(
                clip.deform_tracks
                    .iter()
                    .find(|t| t.attachment == *attachment)
                    .map(|t| t.keys.as_slice()),
                time,
                target.next_time,
                |k| k.time,
            )?;
            Ok(channel([0., 1.], CurveData::Standard(left.curve)))
        }
        AnimationKey::SlotColor { slot, .. } => {
            let (left, right) = exact_pair(
                clip.color_tracks
                    .iter()
                    .find(|t| t.slot == *slot)
                    .map(|t| t.keys.as_slice()),
                time,
                target.next_time,
                |k| k.time,
            )?;
            Ok((0..4)
                .map(|channel| CurveChannel {
                    target: target.clone(),
                    channel,
                    values: [
                        f64::from(left.color[channel]) / 255.,
                        f64::from(right.color[channel]) / 255.,
                    ],
                    curve: CurveData::Standard(left.curves[channel]),
                })
                .collect())
        }
        AnimationKey::SlotAttachment { .. } => {
            Err("Attachment keys have no interpolated curve".into())
        }
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
        .map(|source| {
            let canonical = canonical_key(document, source)?;
            if !identities.insert(canonical.id()) {
                return Err("Duplicate source key".into());
            }
            match canonical {
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
                AnimationKey::Deform { attachment, time } => clip
                    .deform_tracks
                    .iter()
                    .find(|t| t.attachment == attachment)
                    .and_then(|t| t.keys.iter().find(|k| k.time == time))
                    .cloned()
                    .map(|key| OwnedKey::Deform { attachment, key }),
                AnimationKey::SlotAttachment { slot, time } => clip
                    .attachment_tracks
                    .iter()
                    .find(|track| track.slot == slot)
                    .and_then(|track| track.keys.iter().find(|key| key.time == time))
                    .cloned()
                    .map(|key| OwnedKey::SlotAttachment { slot, key }),
                AnimationKey::SlotColor { slot, time } => clip
                    .color_tracks
                    .iter()
                    .find(|track| track.slot == slot)
                    .and_then(|track| track.keys.iter().find(|key| key.time == time))
                    .cloned()
                    .map(|key| OwnedKey::SlotColor { slot, key }),
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
    let clipboard = match &owner {
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
        KeyOwner::Slot { .. } => {
            let mut attachment_keys = vec![];
            let mut color_keys = vec![];
            for source in sources {
                match source {
                    OwnedKey::SlotAttachment { key, .. } => attachment_keys.push(AttachmentKey {
                        time: key.time - first,
                        ..key
                    }),
                    OwnedKey::SlotColor { key, .. } => color_keys.push(ColorKey {
                        time: key.time - first,
                        ..key
                    }),
                    _ => unreachable!("Slot owner was checked above"),
                }
            }
            attachment_keys.sort_by(|a, b| a.time.total_cmp(&b.time));
            color_keys.sort_by(|a, b| a.time.total_cmp(&b.time));
            AnimationKeyClipboard::Slot {
                attachment_keys,
                color_keys,
            }
        }
        KeyOwner::Deform { attachment } => {
            let layout = DeformLayout::from_vertices(
                document
                    .attachment(attachment)
                    .and_then(Attachment::vertices)
                    .ok_or("Unknown deform target")?,
            );
            let mut keys = vec![];
            for source in sources {
                if let OwnedKey::Deform { key, .. } = source {
                    keys.push(DeformKey {
                        time: key.time - first,
                        ..key
                    });
                }
            }
            keys.sort_by(|a, b| a.time.total_cmp(&b.time));
            AnimationKeyClipboard::Deform { layout, keys }
        }
    };
    clipboard_sources(&clipboard, &owner)?;
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
    owner: &KeyOwner,
) -> Result<Vec<OwnedKey>, String> {
    let sources: Vec<_> = match (clipboard, owner) {
        (AnimationKeyClipboard::Bone { keys }, KeyOwner::Bone { bone }) => {
            let mut result = vec![];
            for entry in keys {
                checked_time(entry.key.time)?;
                if !entry.key.value.is_finite() {
                    return Err("Clipboard values must be finite".into());
                }
                if !entry.key.curve.handles_valid() {
                    return Err("Clipboard Bezier handles must be finite".into());
                }
                result.push(OwnedKey::Bone {
                    bone: *bone,
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
                result.push(OwnedKey::Ik {
                    constraint: *constraint,
                    key,
                });
            }
            result
        }
        (AnimationKeyClipboard::Deform { layout, keys }, KeyOwner::Deform { attachment }) => {
            let length = layout.offset_len()?;
            let mut result = vec![];
            for key in keys {
                checked_time(key.time)?;
                if !key.curve.handles_valid() {
                    return Err("Clipboard Bezier handles must be finite".into());
                }
                if key.offsets.len() != length || key.offsets.iter().any(|v| !v.is_finite()) {
                    return Err(format!(
                        "Deform clipboard needs {length} finite offsets per key"
                    ));
                }
                result.push(OwnedKey::Deform {
                    attachment: attachment.clone(),
                    key: key.clone(),
                });
            }
            result
        }
        (
            AnimationKeyClipboard::Slot {
                attachment_keys,
                color_keys,
            },
            KeyOwner::Slot { slot },
        ) => {
            let mut result = vec![];
            for key in attachment_keys {
                crate::slot_authoring::validate_attachment_key(key)?;
                result.push(OwnedKey::SlotAttachment {
                    slot: *slot,
                    key: key.clone(),
                });
            }
            for key in color_keys {
                crate::slot_authoring::validate_color_key(key)?;
                result.push(OwnedKey::SlotColor {
                    slot: *slot,
                    key: key.clone(),
                });
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
        .map(|source| {
            let target = source
                .clone()
                .at(checked_time(source.identity().time() + offset)?);
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
            OwnedKey::SlotAttachment { slot, key } => {
                crate::slot_authoring::insert_attachment_key(clip, slot, key)
            }
            OwnedKey::SlotColor { slot, key } => {
                crate::slot_authoring::insert_color_key(clip, slot, key)
            }
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
            OwnedKey::Deform { attachment, key } => {
                let index = clip
                    .deform_tracks
                    .iter()
                    .position(|t| t.attachment == attachment)
                    .unwrap_or_else(|| {
                        clip.deform_tracks.push(DeformTrack {
                            attachment,
                            keys: vec![],
                        });
                        clip.deform_tracks.len() - 1
                    });
                let keys = &mut clip.deform_tracks[index].keys;
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
    for track in &mut clip.deform_tracks {
        track.keys.retain(|key| {
            !identities.contains(
                &AnimationKey::Deform {
                    attachment: track.attachment.clone(),
                    time: key.time,
                }
                .id(),
            )
        });
    }
    for track in &mut clip.attachment_tracks {
        track.keys.retain(|key| {
            !identities.contains(
                &AnimationKey::SlotAttachment {
                    slot: track.slot,
                    time: key.time,
                }
                .id(),
            )
        });
    }
    for track in &mut clip.color_tracks {
        track.keys.retain(|key| {
            !identities.contains(
                &AnimationKey::SlotColor {
                    slot: track.slot,
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
    clip.deform_tracks.retain(|t| !t.keys.is_empty());
    clip.attachment_tracks.retain(|t| !t.keys.is_empty());
    clip.color_tracks.retain(|t| !t.keys.is_empty());
    Ok(())
}
fn prepared_paste_keys(
    document: &Document,
    animation: usize,
    owner: KeyOwner,
    time: f32,
    clipboard: &AnimationKeyClipboard,
) -> Result<Vec<OwnedKey>, String> {
    checked_time(time)?;
    document
        .animations
        .get(animation)
        .ok_or("Unknown animation")?;
    let owner = canonical_owner(document, owner)?;
    if let (AnimationKeyClipboard::Deform { layout, .. }, KeyOwner::Deform { attachment }) =
        (clipboard, &owner)
    {
        let actual = DeformLayout::from_vertices(
            document
                .attachment(attachment)
                .and_then(Attachment::vertices)
                .ok_or("Unknown deform target")?,
        );
        if !layout.compatible_with(&actual) {
            return Err(
                "Deform clipboard vertex/influence layout does not match the paste target".into(),
            );
        }
    }
    let mut sources = clipboard_sources(clipboard, &owner)?;
    if let KeyOwner::Slot { slot } = owner {
        let name = &document.slots[slot].name;
        // Native 4.3 skips only unavailable named attachment keys; hiding and
        // other copied slot properties remain valid and keep their origin.
        sources.retain(|source| match source {
            OwnedKey::SlotAttachment {
                key:
                    AttachmentKey {
                        attachment: Some(placeholder),
                        ..
                    },
                ..
            } => document.skins.values().any(|skin| {
                skin.get(name)
                    .is_some_and(|attachments| attachments.contains_key(placeholder))
            }),
            _ => true,
        });
    }
    checked_targets(&sources, time)
}

/// Exact incoming identities accepted by the corresponding paste command.
/// Includes valid overwrites even when their values are unchanged; skipped
/// attachment names never select an existing, untouched destination key.
pub fn pasted_animation_keys(
    document: &Document,
    animation: usize,
    target: KeyOwner,
    time: f32,
    clipboard: &AnimationKeyClipboard,
) -> Result<Vec<AnimationKey>, String> {
    Ok(
        prepared_paste_keys(document, animation, target, time, clipboard)?
            .iter()
            .map(OwnedKey::identity)
            .collect(),
    )
}

pub(super) fn paste_animation_keys(
    document: &mut Document,
    animation: usize,
    owner: KeyOwner,
    time: f32,
    clipboard: &AnimationKeyClipboard,
) -> Result<(), String> {
    let targets = prepared_paste_keys(document, animation, owner, time, clipboard)?;
    insert_keys(document, animation, targets);
    Ok(())
}

pub(super) fn set_deform_curve(
    document: &mut Document,
    animation: usize,
    attachment: &AttachmentRef,
    time: f32,
    curve: Curve,
) -> Result<(), String> {
    checked_time(time)?;
    if !curve.handles_valid() {
        return Err("Deform curve handles must be finite".into());
    }
    let KeyOwner::Deform { attachment } = deform_key_owner(document, attachment)? else {
        unreachable!()
    };
    let key = document
        .animations
        .get_mut(animation)
        .ok_or("Unknown animation")?
        .deform_tracks
        .iter_mut()
        .find(|t| t.attachment == attachment)
        .and_then(|t| t.keys.iter_mut().find(|k| k.time == time))
        .ok_or("Unknown exact deform key")?;
    key.curve = curve;
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
