//! Region/mesh `sequence` definitions, `sequence` timelines under animation
//! attachments, and slot `blend` modes. Defaults observed through the external
//! 4.3.13 runtime: start 1, digits 0, setup frame (`setup`) 0; keys default to
//! hold, index 0.
use super::{number, unknown_fields};
use peachy_document::{
    Attachment, AttachmentRef, BlendMode, Document, SequenceDef, SequenceKey, SequenceMode,
    SequenceTrack, Skin,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn count(value: &Value, field: &str, default: u32, context: &str) -> Result<u32, String> {
    match value.get(field) {
        None => Ok(default),
        Some(v) => v
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| format!("{context}: sequence {field} must be a whole number")),
    }
}

pub(super) fn read_def(value: &Value, context: &str) -> Result<Option<SequenceDef>, String> {
    let Some(sequence) = value.get("sequence") else {
        return Ok(None);
    };
    let mut issues = vec![];
    unknown_fields(
        sequence,
        &["count", "start", "digits", "setup"],
        context,
        &mut issues,
    );
    if !issues.is_empty() {
        return Err(issues.join("; "));
    }
    let def = SequenceDef {
        count: count(sequence, "count", 0, context)?,
        start: count(sequence, "start", 1, context)?,
        digits: count(sequence, "digits", 0, context)?,
        setup_index: count(sequence, "setup", 0, context)?,
    };
    def.validate().map_err(|e| format!("{context}: {e}"))?;
    Ok(Some(def))
}

pub(super) fn write_def(def: &SequenceDef) -> Value {
    let mut value = json!({"count": def.count});
    if def.start != 1 {
        value["start"] = json!(def.start);
    }
    if def.digits != 0 {
        value["digits"] = json!(def.digits);
    }
    if def.setup_index != 0 {
        value["setup"] = json!(def.setup_index);
    }
    value
}

pub(super) fn read_blend(slot: &Value) -> Result<BlendMode, String> {
    match slot.get("blend") {
        None => Ok(BlendMode::Normal),
        Some(v) => match v.as_str() {
            Some("normal") => Ok(BlendMode::Normal),
            Some("additive") => Ok(BlendMode::Additive),
            Some("multiply") => Ok(BlendMode::Multiply),
            Some("screen") => Ok(BlendMode::Screen),
            _ => Err("blend must be normal, additive, multiply or screen".into()),
        },
    }
}

pub(super) fn read_tracks(
    animation_name: &str,
    animation: &Value,
    skins: &BTreeMap<String, Skin>,
) -> Result<Vec<SequenceTrack>, String> {
    let context = format!("Animation {animation_name}");
    let mut tracks = vec![];
    let Some(by_skin) = animation.get("attachments").and_then(Value::as_object) else {
        return Ok(tracks);
    };
    for (skin, slots) in by_skin {
        for (slot, attachments) in slots.as_object().into_iter().flatten() {
            for (placeholder, timelines) in attachments.as_object().into_iter().flatten() {
                let Some(keys) = timelines.get("sequence") else {
                    continue;
                };
                let attachment = AttachmentRef {
                    skin: skin.clone(),
                    slot: slot.clone(),
                    placeholder: placeholder.clone(),
                };
                let frames = skins
                    .get(skin)
                    .and_then(|s| s.get(slot))
                    .and_then(|a| a.get(placeholder))
                    .and_then(Attachment::sequence)
                    .map(|s| s.count)
                    .ok_or_else(|| {
                        format!("{context}: sequence keys need a sequenced attachment {skin}/{slot}/{placeholder}")
                    })?;
                let mut out = vec![];
                // Observed: an omitted delay carries the previous key's delay
                // (0 for the first key); mode and index do not carry.
                let mut carried_delay = 0.0;
                for key in keys
                    .as_array()
                    .ok_or_else(|| format!("{context}: sequence keys must be an array"))?
                {
                    let mut issues = vec![];
                    unknown_fields(
                        key,
                        &["time", "mode", "index", "delay"],
                        &context,
                        &mut issues,
                    );
                    if !issues.is_empty() {
                        return Err(issues.join("; "));
                    }
                    let mode = match key.get("mode") {
                        None => SequenceMode::Hold,
                        Some(v) => *SequenceMode::ALL
                            .iter()
                            .find(|m| v.as_str() == Some(m.spine_name()))
                            .ok_or_else(|| format!("{context}: unknown sequence mode"))?,
                    };
                    let index = count(key, "index", 0, &context)?;
                    if index >= frames {
                        return Err(format!("{context}: sequence index outside the sequence"));
                    }
                    let delay = number(key, "delay", carried_delay);
                    carried_delay = delay;
                    out.push(SequenceKey {
                        time: number(key, "time", 0.0),
                        mode,
                        index,
                        delay,
                    });
                }
                tracks.push(SequenceTrack {
                    attachment,
                    keys: out,
                });
            }
        }
    }
    Ok(tracks)
}

/// Adds `sequence` timelines into an `attachments` object (shared with deform).
pub(super) fn write_tracks(attachments: &mut Value, tracks: &[SequenceTrack]) {
    for track in tracks.iter().filter(|t| !t.keys.is_empty()) {
        let a = &track.attachment;
        for (node, key) in [(0, &a.skin), (1, &a.slot), (2, &a.placeholder)] {
            let target = match node {
                0 => &mut *attachments,
                1 => &mut attachments[&a.skin],
                _ => &mut attachments[&a.skin][&a.slot],
            };
            if target.get(key).is_none() {
                target[key] = json!({});
            }
        }
        let mut carried_delay = 0.0;
        attachments[&a.skin][&a.slot][&a.placeholder]["sequence"] = track
            .keys
            .iter()
            .map(|key| {
                let mut k = json!({"time": key.time});
                if key.mode != SequenceMode::Hold {
                    k["mode"] = json!(key.mode.spine_name());
                }
                if key.index != 0 {
                    k["index"] = json!(key.index);
                }
                // Written whenever it differs from the carried delay.
                if key.delay != carried_delay {
                    k["delay"] = json!(key.delay);
                }
                carried_delay = key.delay;
                k
            })
            .collect();
    }
}

pub(super) fn blend_name(doc: &Document, slot: usize) -> Option<&'static str> {
    let blend = doc.slots[slot].blend;
    (!blend.is_normal()).then(|| blend.spine_name())
}
