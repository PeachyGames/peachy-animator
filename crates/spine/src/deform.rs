//! Mesh deform timelines (`animations.<name>.attachments.<skin>.<slot>.<placeholder>.deform`).
//! Keys are sparse in JSON (`offset` + `vertices`, omitted values zero) and dense
//! natively. Bezier curves store absolute times and progress fractions. Format
//! observed through installed Spine 4.3.26 and the external 4.3.13 runtime.
use super::{absolute_control, array, normalized_control, number, unknown_fields};
use peachy_document::{Attachment, AttachmentRef, Curve, DeformKey, DeformTrack, Skin};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) fn read(
    animation_name: &str,
    animation: &Value,
    skins: &BTreeMap<String, Skin>,
) -> Result<Vec<DeformTrack>, String> {
    let Some(entries) = animation.get("attachments") else {
        return Ok(vec![]);
    };
    let context = format!("Animation {animation_name}");
    let mut tracks = vec![];
    for (skin, slots) in entries
        .as_object()
        .ok_or_else(|| format!("{context}: attachments must be an object"))?
    {
        for (slot, attachments) in slots
            .as_object()
            .ok_or_else(|| format!("{context}: attachments.{skin} must be an object"))?
        {
            for (placeholder, timelines) in attachments
                .as_object()
                .ok_or_else(|| format!("{context}: attachments.{skin}.{slot} must be an object"))?
            {
                let timelines = timelines.as_object().ok_or_else(|| {
                    format!("{context}: {slot}/{placeholder} timelines must be an object")
                })?;
                for (timeline, keys) in timelines {
                    if timeline != "deform" {
                        return Err(format!(
                            "{context}: {slot}/{placeholder} {timeline} timelines are unsupported"
                        ));
                    }
                    let attachment = AttachmentRef {
                        skin: skin.clone(),
                        slot: slot.clone(),
                        placeholder: placeholder.clone(),
                    };
                    let mesh = match skins
                        .get(skin)
                        .and_then(|s| s.get(slot))
                        .and_then(|a| a.get(placeholder))
                    {
                        Some(Attachment::Mesh(mesh)) => mesh,
                        Some(_) => {
                            return Err(format!(
                                "{context}: deform keys on non-mesh {slot}/{placeholder} are unsupported"
                            ));
                        }
                        None => {
                            return Err(format!(
                                "{context}: deform keys for unknown attachment {skin}/{slot}/{placeholder}"
                            ));
                        }
                    };
                    let count = mesh.deform_len();
                    let keys = keys
                        .as_array()
                        .ok_or_else(|| format!("{context}: deform keys must be an array"))?;
                    let mut out = vec![];
                    for (i, key) in keys.iter().enumerate() {
                        let mut issues = vec![];
                        unknown_fields(
                            key,
                            &["time", "offset", "vertices", "curve"],
                            &context,
                            &mut issues,
                        );
                        if !issues.is_empty() {
                            return Err(issues.join("; "));
                        }
                        let time = number(key, "time", 0.0);
                        let offset = match key.get("offset") {
                            None => 0,
                            Some(v) => v.as_u64().ok_or_else(|| {
                                format!("{context}: deform offset must be a nonnegative integer")
                            })? as usize,
                        };
                        let values = array(&key["vertices"]);
                        if key.get("vertices").is_some_and(|v| !v.is_array())
                            || offset + values.len() > count
                        {
                            return Err(format!(
                                "{context}: deform key at {time} exceeds {slot}/{placeholder}'s {count} values"
                            ));
                        }
                        let mut offsets = vec![0.0; count];
                        for (j, value) in values.iter().enumerate() {
                            offsets[offset + j] = value
                                .as_f64()
                                .map(|v| v as f32)
                                .filter(|v| v.is_finite())
                                .ok_or_else(|| {
                                    format!("{context}: deform values must be finite f32 numbers")
                                })?;
                        }
                        let curve = match key.get("curve") {
                            None => Curve::Linear,
                            Some(Value::String(s)) if s == "stepped" => Curve::Stepped,
                            Some(Value::Array(h)) if h.len() == 4 => {
                                let next = keys.get(i + 1).ok_or("Bezier on last key")?;
                                let next_time = number(next, "time", 0.0);
                                let h: Vec<f32> = h
                                    .iter()
                                    .filter_map(|v| v.as_f64().map(|v| v as f32))
                                    .collect();
                                if h.len() != 4
                                    || !h.iter().all(|v| v.is_finite())
                                    || next_time <= time
                                {
                                    return Err(format!("{context}: invalid deform curve"));
                                }
                                Curve::Bezier([
                                    normalized_control(time, next_time, h[0])?,
                                    h[1],
                                    normalized_control(time, next_time, h[2])?,
                                    h[3],
                                ])
                            }
                            _ => {
                                return Err(format!(
                                    "{context}: deform curve must be stepped or 4 numbers"
                                ));
                            }
                        };
                        out.push(DeformKey {
                            time,
                            curve,
                            offsets,
                        });
                    }
                    tracks.push(DeformTrack {
                        attachment,
                        keys: out,
                    });
                }
            }
        }
    }
    Ok(tracks)
}

/// Sparse keys like the installed editor writes: zeros trimmed into `offset`.
pub(super) fn write(tracks: &[DeformTrack]) -> Result<Value, String> {
    let mut out = json!({});
    for track in tracks.iter().filter(|t| !t.keys.is_empty()) {
        let a = &track.attachment;
        let mut keys = vec![];
        for (i, key) in track.keys.iter().enumerate() {
            let mut k = json!({"time":key.time});
            if let (Some(first), Some(last)) = (
                key.offsets.iter().position(|v| *v != 0.0),
                key.offsets.iter().rposition(|v| *v != 0.0),
            ) {
                if first > 0 {
                    k["offset"] = json!(first);
                }
                k["vertices"] = json!(key.offsets[first..=last]);
            }
            match key.curve {
                Curve::Stepped => k["curve"] = json!("stepped"),
                Curve::Bezier([x1, y1, x2, y2]) if i + 1 < track.keys.len() => {
                    let next = track.keys[i + 1].time;
                    k["curve"] = json!([
                        absolute_control(key.time, next, x1)?,
                        y1,
                        absolute_control(key.time, next, x2)?,
                        y2
                    ]);
                }
                _ => {}
            }
            keys.push(k);
        }
        if out.get(&a.skin).is_none() {
            out[&a.skin] = json!({});
        }
        if out[&a.skin].get(&a.slot).is_none() {
            out[&a.skin][&a.slot] = json!({});
        }
        out[&a.skin][&a.slot][&a.placeholder] = json!({"deform":keys});
    }
    Ok(out)
}
