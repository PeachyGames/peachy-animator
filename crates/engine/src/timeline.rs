//! Exact key identities and pure timing/clipboard transitions for supported bone tracks.
use peachy_document::{Curve, Document, Keyframe, Property, Track};
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
    /// Time is relative to the first copied key; values remain setup-relative.
    pub key: Keyframe,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyClipboard {
    pub keys: Vec<ClipboardKey>,
}

#[derive(Clone, Copy)]
struct OwnedKey {
    bone: usize,
    property: Property,
    key: Keyframe,
}

// Positive finite f32 bit patterns preserve exact identity. Both zero signs address zero.
fn time_id(time: f32) -> u32 {
    if time == 0.0 { 0 } else { time.to_bits() }
}

fn checked_time(time: f32) -> Result<f32, String> {
    if !time.is_finite() || time < 0.0 {
        return Err("Key time must be finite and nonnegative".into());
    }
    Ok(time)
}

fn sources(
    document: &Document,
    animation: usize,
    keys: &[TimelineKey],
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
            checked_time(source.time)?;
            if source.bone >= document.bones.len() {
                return Err("Unknown bone".into());
            }
            if !identities.insert((source.bone, source.property, time_id(source.time))) {
                return Err("Duplicate source key".into());
            }
            let key = clip
                .tracks
                .iter()
                .find(|track| track.bone == source.bone && track.property == source.property)
                .and_then(|track| track.keys.iter().find(|key| key.time == source.time))
                .copied()
                .ok_or("Unknown exact source key")?;
            Ok(OwnedKey {
                bone: source.bone,
                property: source.property,
                key,
            })
        })
        .collect()
}

/// Capture owned scalar keys for one bone, with relative times anchored at zero.
pub fn copy_keys(
    document: &Document,
    animation: usize,
    keys: &[TimelineKey],
) -> Result<KeyClipboard, String> {
    document.validate()?;
    let sources = sources(document, animation, keys)?;
    let bone = sources[0].bone;
    if sources.iter().any(|source| source.bone != bone) {
        return Err("Clipboard currently supports keys from one bone".into());
    }
    let first = sources
        .iter()
        .map(|source| source.key.time)
        .min_by(f32::total_cmp)
        .expect("Sources are nonempty");
    let mut keys: Vec<_> = sources
        .into_iter()
        .map(|source| ClipboardKey {
            property: source.property,
            key: Keyframe {
                time: source.key.time - first,
                ..source.key
            },
        })
        .collect();
    keys.sort_by(|a, b| {
        a.key
            .time
            .total_cmp(&b.key.time)
            .then(a.property.cmp(&b.property))
    });
    let clipboard = KeyClipboard { keys };
    validate_clipboard(&clipboard)?;
    Ok(clipboard)
}

fn validate_clipboard(clipboard: &KeyClipboard) -> Result<(), String> {
    if clipboard.keys.is_empty() || !clipboard.keys.iter().any(|entry| entry.key.time == 0.0) {
        return Err("Clipboard needs keys anchored at relative time zero".into());
    }
    let mut identities = BTreeSet::new();
    for entry in &clipboard.keys {
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
        if !identities.insert((entry.property, time_id(entry.key.time))) {
            return Err("Duplicate clipboard key".into());
        }
    }
    Ok(())
}

fn checked_targets(keys: &[OwnedKey], offset: f32) -> Result<Vec<OwnedKey>, String> {
    let mut identities = BTreeSet::new();
    keys.iter()
        .map(|source| {
            let time = checked_time(source.key.time + offset)?;
            if !identities.insert((source.bone, source.property, time_id(time))) {
                return Err("Key times collapse at this offset".into());
            }
            Ok(OwnedKey {
                key: Keyframe { time, ..source.key },
                ..*source
            })
        })
        .collect()
}

fn insert_keys(document: &mut Document, animation: usize, keys: Vec<OwnedKey>) {
    let clip = &mut document.animations[animation];
    for incoming in keys {
        clip.duration = clip.duration.max(incoming.key.time);
        let index = clip
            .tracks
            .iter()
            .position(|track| track.bone == incoming.bone && track.property == incoming.property)
            .unwrap_or_else(|| {
                clip.tracks.push(Track {
                    bone: incoming.bone,
                    property: incoming.property,
                    keys: vec![],
                });
                clip.tracks.len() - 1
            });
        let keys = &mut clip.tracks[index].keys;
        if let Some(existing) = keys.iter_mut().find(|key| key.time == incoming.key.time) {
            *existing = incoming.key;
        } else {
            keys.push(incoming.key);
        }
        keys.sort_by(|a, b| a.time.total_cmp(&b.time));
    }
}

pub(super) fn move_keys(
    document: &mut Document,
    animation: usize,
    keys: &[TimelineKey],
    delta: f32,
    copy: bool,
) -> Result<(), String> {
    if !delta.is_finite() {
        return Err("Key offset must be finite".into());
    }
    let sources = sources(document, animation, keys)?;
    let targets = checked_targets(&sources, delta)?;
    if !copy {
        let identities: BTreeSet<_> = sources
            .iter()
            .map(|source| (source.bone, source.property, time_id(source.key.time)))
            .collect();
        for track in &mut document.animations[animation].tracks {
            track.keys.retain(|key| {
                !identities.contains(&(track.bone, track.property, time_id(key.time)))
            });
        }
    }
    insert_keys(document, animation, targets);
    Ok(())
}

pub(super) fn paste_keys(
    document: &mut Document,
    animation: usize,
    bone: usize,
    time: f32,
    clipboard: &KeyClipboard,
) -> Result<(), String> {
    checked_time(time)?;
    if document.animations.get(animation).is_none() {
        return Err("Unknown animation".into());
    }
    if bone >= document.bones.len() {
        return Err("Unknown bone".into());
    }
    validate_clipboard(clipboard)?;
    let sources: Vec<_> = clipboard
        .keys
        .iter()
        .map(|entry| OwnedKey {
            bone,
            property: entry.property,
            key: entry.key,
        })
        .collect();
    let targets = checked_targets(&sources, time)?;
    insert_keys(document, animation, targets);
    Ok(())
}
