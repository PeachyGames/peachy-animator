//! Slot attachment/color poses and exact persistent keys. Sampled float colors
//! remain untouched until an explicit authoring/key transition encodes bytes.
use crate::{AuthoredPose, Command};
use peachy_document::{
    Animation, AttachmentKey, AttachmentTrack, ColorKey, ColorTrack, Curve, Document,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SlotProperty {
    Attachment,
    Color,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum SlotPoseEdit {
    Attachment { attachment: Option<String> },
    Color { color: [f32; 4] },
}
impl SlotPoseEdit {
    pub fn property(&self) -> SlotProperty {
        match self {
            Self::Attachment { .. } => SlotProperty::Attachment,
            Self::Color { .. } => SlotProperty::Color,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SlotPose {
    pub attachment: Option<String>,
    /// Unmultiplied normalized channels; sampled Bezier overshoot may leave 0–1.
    pub color: [f32; 4],
}
impl SlotPose {
    pub(crate) fn from_authored(pose: &AuthoredPose, slot: usize) -> Result<Self, String> {
        Ok(Self {
            attachment: pose
                .slot_attachments
                .get(slot)
                .ok_or("Unknown slot")?
                .clone(),
            color: *pose.slot_colors.get(slot).ok_or("Unknown slot")?,
        })
    }
}

/// Pure display/byte-key encoding, never a pose or document write. Keep sampled
/// float values separately; a field merely displaying this result must not key it.
pub fn slot_color_bytes(color: [f32; 4]) -> Result<[u8; 4], String> {
    if color.iter().any(|channel| !channel.is_finite()) {
        return Err("Slot color channels must be finite".into());
    }
    Ok(color.map(|channel| (channel.clamp(0., 1.) * 255.).round() as u8))
}

fn checked_time(time: f32) -> Result<(), String> {
    if !time.is_finite() || time < 0. {
        return Err("Slot key time must be finite and nonnegative".into());
    }
    Ok(())
}
fn validate_attachment(attachment: &Option<String>) -> Result<(), String> {
    if attachment
        .as_ref()
        .is_some_and(|name| name.trim().is_empty())
    {
        return Err("Slot attachment name must be nonempty, or null to hide".into());
    }
    Ok(())
}
pub(crate) fn validate_edits(
    doc: &Document,
    slot: usize,
    edits: &[SlotPoseEdit],
) -> Result<(), String> {
    doc.slots.get(slot).ok_or("Unknown slot")?;
    let mut properties = BTreeSet::new();
    for edit in edits {
        if !properties.insert(edit.property()) {
            return Err("Duplicate slot pose property".into());
        }
        match edit {
            SlotPoseEdit::Attachment { attachment } => validate_attachment(attachment)?,
            SlotPoseEdit::Color { color } => {
                if color.iter().any(|channel| !channel.is_finite()) {
                    return Err("Slot color channels must be finite".into());
                }
            }
        }
    }
    Ok(())
}
pub(crate) fn validate_attachment_key(key: &AttachmentKey) -> Result<(), String> {
    checked_time(key.time)?;
    validate_attachment(&key.attachment)
}
pub(crate) fn validate_color_curves(curves: &[Curve; 4]) -> Result<(), String> {
    let stepped = curves
        .iter()
        .filter(|curve| **curve == Curve::Stepped)
        .count();
    if stepped != 0 && stepped != 4 {
        return Err("A stepped color key must be stepped on every channel".into());
    }
    if curves.iter().any(|curve| !curve.handles_valid()) {
        return Err("Slot color curve handles must be finite".into());
    }
    Ok(())
}
pub(crate) fn validate_color_key(key: &ColorKey) -> Result<(), String> {
    checked_time(key.time)?;
    validate_color_curves(&key.curves)
}

pub(crate) fn insert_attachment_key(clip: &mut Animation, slot: usize, key: AttachmentKey) {
    let index = clip
        .attachment_tracks
        .iter()
        .position(|track| track.slot == slot)
        .unwrap_or_else(|| {
            clip.attachment_tracks
                .push(AttachmentTrack { slot, keys: vec![] });
            clip.attachment_tracks.len() - 1
        });
    let keys = &mut clip.attachment_tracks[index].keys;
    if let Some(previous) = keys.iter_mut().find(|previous| previous.time == key.time) {
        *previous = key;
    } else {
        keys.push(key);
    }
    keys.sort_by(|a, b| a.time.total_cmp(&b.time));
}
pub(crate) fn insert_color_key(clip: &mut Animation, slot: usize, key: ColorKey) {
    let index = clip
        .color_tracks
        .iter()
        .position(|track| track.slot == slot)
        .unwrap_or_else(|| {
            clip.color_tracks.push(ColorTrack { slot, keys: vec![] });
            clip.color_tracks.len() - 1
        });
    let keys = &mut clip.color_tracks[index].keys;
    if let Some(previous) = keys.iter_mut().find(|previous| previous.time == key.time) {
        *previous = key;
    } else {
        keys.push(key);
    }
    keys.sort_by(|a, b| a.time.total_cmp(&b.time));
}
pub(crate) fn set_attachment_key(
    doc: &mut Document,
    animation: usize,
    slot: usize,
    key: AttachmentKey,
) -> Result<(), String> {
    doc.slots.get(slot).ok_or("Unknown slot")?;
    validate_attachment_key(&key)?;
    let clip = doc
        .animations
        .get_mut(animation)
        .ok_or("Unknown animation")?;
    insert_attachment_key(clip, slot, key);
    Ok(())
}
pub(crate) fn set_color_key(
    doc: &mut Document,
    animation: usize,
    slot: usize,
    key: ColorKey,
) -> Result<(), String> {
    doc.slots.get(slot).ok_or("Unknown slot")?;
    validate_color_key(&key)?;
    let clip = doc
        .animations
        .get_mut(animation)
        .ok_or("Unknown animation")?;
    insert_color_key(clip, slot, key);
    Ok(())
}
/// Shared native/MCP insertion. Exact existing identities retain all curves;
/// a genuinely interior new key splits its prior interval before publication.
pub(crate) fn insert_color_pose(
    doc: &mut Document,
    animation: usize,
    slot: usize,
    time: f32,
    color: [f32; 4],
) -> Result<(), String> {
    doc.slots.get(slot).ok_or("Unknown slot")?;
    checked_time(time)?;
    let encoded = slot_color_bytes(color)?;
    let stored = encoded.map(|byte| f32::from(byte) / 255.);
    let tolerance = 0.5_f64 / 255. + 4. * f64::from(f32::EPSILON);
    if color
        .iter()
        .zip(stored)
        .any(|(&value, stored)| (f64::from(value) - f64::from(stored)).abs() > tolerance)
    {
        return Err("Inserted slot color cannot be represented at stored byte precision".into());
    }
    let clip = doc.animations.get(animation).ok_or("Unknown animation")?;
    let keys = clip
        .color_tracks
        .iter()
        .find(|track| track.slot == slot)
        .map_or(&[][..], |track| track.keys.as_slice());
    if let Some(existing) = keys.iter().find(|key| key.time == time) {
        let curves = existing.curves;
        return set_color_key(
            doc,
            animation,
            slot,
            ColorKey {
                time,
                color: encoded,
                curves,
            },
        );
    }
    let after = keys.partition_point(|key| key.time < time);
    let interval = after
        .checked_sub(1)
        .and_then(|index| keys.get(index).zip(keys.get(after)));
    let mut incoming = ColorKey {
        time,
        color: encoded,
        curves: [Curve::Linear; 4],
    };
    let previous = if let Some((left, right)) = interval {
        let mut previous = left.clone();
        for channel in 0..4 {
            let (before, after) = crate::color_curve_split::split(
                left.curves[channel],
                [left.time, time, right.time],
                [
                    f32::from(left.color[channel]) / 255.,
                    color[channel],
                    f32::from(right.color[channel]) / 255.,
                ],
                stored[channel],
            )?;
            previous.curves[channel] = before;
            incoming.curves[channel] = after;
        }
        Some(previous)
    } else {
        None
    };
    // Both candidates are fully computed before changing any track.
    if let Some(previous) = previous {
        set_color_key(doc, animation, slot, previous)?;
    }
    set_color_key(doc, animation, slot, incoming)
}

pub(crate) fn set_color_curves(
    doc: &mut Document,
    animation: usize,
    slot: usize,
    time: f32,
    curves: [Curve; 4],
) -> Result<(), String> {
    doc.slots.get(slot).ok_or("Unknown slot")?;
    checked_time(time)?;
    validate_color_curves(&curves)?;
    let key = doc
        .animations
        .get_mut(animation)
        .ok_or("Unknown animation")?
        .color_tracks
        .iter_mut()
        .find(|track| track.slot == slot)
        .and_then(|track| track.keys.iter_mut().find(|key| key.time == time))
        .ok_or("Unknown exact slot color key")?;
    key.curves = curves;
    Ok(())
}

/// Resolve an exact imported identity before trying one unambiguous nearby key.
pub fn slot_key_time(
    doc: &Document,
    animation: usize,
    time: f32,
    slot: usize,
    property: SlotProperty,
) -> Result<Option<f32>, String> {
    doc.slots.get(slot).ok_or("Unknown slot")?;
    checked_time(time)?;
    let clip = doc.animations.get(animation).ok_or("Unknown animation")?;
    match property {
        SlotProperty::Attachment => {
            let keys = clip
                .attachment_tracks
                .iter()
                .find(|track| track.slot == slot)
                .map_or(&[][..], |track| track.keys.as_slice());
            crate::commands::key_index_by_time(keys, time, |key| key.time)
                .map(|index| index.map(|index| keys[index].time))
        }
        SlotProperty::Color => {
            let keys = clip
                .color_tracks
                .iter()
                .find(|track| track.slot == slot)
                .map_or(&[][..], |track| track.keys.as_slice());
            crate::commands::key_index_by_time(keys, time, |key| key.time)
                .map(|index| index.map(|index| keys[index].time))
        }
    }
}

pub(crate) fn key_commands(
    doc: &Document,
    pose: &AuthoredPose,
    animation: usize,
    time: f32,
    slot: usize,
    properties: &[SlotProperty],
) -> Result<Vec<Command>, String> {
    let current = SlotPose::from_authored(pose, slot)?;
    let snapped = crate::commands::snapped_time(doc.fps, time)?;
    let mut seen = BTreeSet::new();
    let mut commands = vec![];
    for &property in properties {
        if !seen.insert(property) {
            return Err("Duplicate slot key property".into());
        }
        let exact = match slot_key_time(doc, animation, time, slot, property)? {
            Some(exact) => Some(exact),
            None => slot_key_time(doc, animation, snapped, slot, property)?,
        };
        let key_time = exact.unwrap_or(snapped);
        commands.push(match property {
            SlotProperty::Attachment => Command::SetSlotAttachmentKey {
                animation,
                slot,
                key: AttachmentKey {
                    time: key_time,
                    attachment: current.attachment.clone(),
                },
            },
            SlotProperty::Color => {
                let curves = exact
                    .and_then(|time| {
                        doc.animations[animation]
                            .color_tracks
                            .iter()
                            .find(|track| track.slot == slot)
                            .and_then(|track| track.keys.iter().find(|key| key.time == time))
                    })
                    .map_or([Curve::Linear; 4], |key| key.curves);
                if exact.is_some() {
                    Command::SetSlotColorKey {
                        animation,
                        slot,
                        key: ColorKey {
                            time: key_time,
                            color: slot_color_bytes(current.color)?,
                            curves,
                        },
                    }
                } else {
                    Command::InsertSlotColorKey {
                        animation,
                        slot,
                        time: key_time,
                        color: current.color,
                    }
                }
            }
        });
    }
    Ok(commands)
}

pub(crate) fn check_key_continuity(
    before: &AuthoredPose,
    after: &AuthoredPose,
    slots: &std::collections::BTreeMap<usize, Vec<SlotProperty>>,
) -> Result<(), String> {
    for (&slot, properties) in slots {
        let before = SlotPose::from_authored(before, slot)?;
        let after = SlotPose::from_authored(after, slot)?;
        if properties.contains(&SlotProperty::Attachment) && before.attachment != after.attachment {
            return Err("Keying slot attachment would lose the current pose; keep it pending or use a representable key time".into());
        }
        if properties.contains(&SlotProperty::Color) {
            // Native color keys currently encode bytes. Reading a sampled pose
            // never quantizes; an explicit key may incur at most half a byte.
            let tolerance = 0.5_f64 / 255. + 4. * f64::from(f32::EPSILON);
            if before
                .color
                .iter()
                .zip(after.color)
                .any(|(&a, b)| (f64::from(a) - f64::from(b)).abs() > tolerance)
            {
                return Err("Keying slot color exceeds byte precision; keep it pending or use a representable key time".into());
            }
        }
    }
    Ok(())
}
