//! Slot `attachment` and `rgba` timelines (`animations.<name>.slots.<slot>`).
//! Color curves hold four Bezier groups (R, G, B, A) of absolute time and
//! channel value, normalized natively like scalar curves. Observed through
//! installed Spine 4.3.26 and the external 4.3.13 runtime.
use super::{absolute_control, normalized_control, number, read_slot_color, unknown_fields};
use peachy_document::{AttachmentKey, AttachmentTrack, ColorKey, ColorTrack, Curve, Document};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) fn read(
    animation_name: &str,
    animation: &Value,
    slots: &BTreeMap<String, usize>,
) -> Result<(Vec<AttachmentTrack>, Vec<ColorTrack>), String> {
    let (mut attachments, mut colors) = (vec![], vec![]);
    let Some(entries) = animation.get("slots") else {
        return Ok((attachments, colors));
    };
    let context = format!("Animation {animation_name}");
    for (slot_name, timelines) in entries
        .as_object()
        .ok_or_else(|| format!("{context}: slots must be an object"))?
    {
        let slot = *slots
            .get(slot_name)
            .ok_or_else(|| format!("{context}: unknown slot {slot_name}"))?;
        for (timeline, keys) in timelines
            .as_object()
            .ok_or_else(|| format!("{context}: slots.{slot_name} must be an object"))?
        {
            let keys = keys
                .as_array()
                .ok_or_else(|| format!("{context}: {slot_name} {timeline} must be an array"))?;
            let mut issues = vec![];
            match timeline.as_str() {
                "attachment" => {
                    let mut out = vec![];
                    for key in keys {
                        unknown_fields(key, &["time", "name"], &context, &mut issues);
                        let attachment = match key.get("name") {
                            None | Some(Value::Null) => None,
                            Some(Value::String(name)) if !name.trim().is_empty() => {
                                Some(name.clone())
                            }
                            _ => {
                                return Err(format!(
                                    "{context}: {slot_name} attachment name must be a string or null"
                                ));
                            }
                        };
                        out.push(AttachmentKey {
                            time: number(key, "time", 0.0),
                            attachment,
                        });
                    }
                    if !issues.is_empty() {
                        return Err(issues.join("; "));
                    }
                    attachments.push(AttachmentTrack { slot, keys: out });
                }
                "rgba" => {
                    let mut out = vec![];
                    for (i, key) in keys.iter().enumerate() {
                        unknown_fields(key, &["time", "color", "curve"], &context, &mut issues);
                        if key.get("color").is_none() {
                            return Err(format!("{context}: {slot_name} rgba keys need a color"));
                        }
                        let time = number(key, "time", 0.0);
                        let color = read_slot_color(key).map_err(|e| format!("{context}: {e}"))?;
                        let curves = match key.get("curve") {
                            None => [Curve::Linear; 4],
                            Some(Value::String(s)) if s == "stepped" => [Curve::Stepped; 4],
                            Some(Value::Array(h)) if h.len() == 16 => {
                                let next = keys.get(i + 1).ok_or("Bezier on last key")?;
                                let next_time = number(next, "time", 0.0);
                                let next_color =
                                    read_slot_color(next).map_err(|e| format!("{context}: {e}"))?;
                                let h: Vec<f32> = h
                                    .iter()
                                    .filter_map(|v| v.as_f64().map(|v| v as f32))
                                    .collect();
                                if h.len() != 16
                                    || !h.iter().all(|v| v.is_finite())
                                    || next_time <= time
                                {
                                    return Err(format!("{context}: invalid rgba curve"));
                                }
                                let mut curves = [Curve::Linear; 4];
                                for channel in 0..4 {
                                    let c = &h[channel * 4..channel * 4 + 4];
                                    let (v0, v1) = (
                                        f32::from(color[channel]) / 255.,
                                        f32::from(next_color[channel]) / 255.,
                                    );
                                    curves[channel] = if (v1 - v0).abs() < 1e-8 {
                                        if (c[1] - v0).abs() > 1e-5 || (c[3] - v0).abs() > 1e-5 {
                                            return Err(format!(
                                                "{context}: color curve overshoot between equal values is unsupported"
                                            ));
                                        }
                                        Curve::Linear
                                    } else {
                                        Curve::Bezier([
                                            normalized_control(time, next_time, c[0])?,
                                            normalized_control(v0, v1, c[1])?,
                                            normalized_control(time, next_time, c[2])?,
                                            normalized_control(v0, v1, c[3])?,
                                        ])
                                    };
                                }
                                curves
                            }
                            _ => {
                                return Err(format!(
                                    "{context}: rgba curve must be stepped or 16 numbers"
                                ));
                            }
                        };
                        out.push(ColorKey {
                            time,
                            color,
                            curves,
                        });
                    }
                    if !issues.is_empty() {
                        return Err(issues.join("; "));
                    }
                    colors.push(ColorTrack { slot, keys: out });
                }
                other => {
                    return Err(format!(
                        "{context}: {slot_name} {other} timelines are unsupported"
                    ));
                }
            }
        }
    }
    Ok((attachments, colors))
}

pub(super) fn write(
    doc: &Document,
    animation: &peachy_document::Animation,
) -> Result<Value, String> {
    let mut out = json!({});
    let entry = |out: &mut Value, slot: usize| -> String {
        let name = doc.slots[slot].name.clone();
        if out.get(&name).is_none() {
            out[&name] = json!({});
        }
        name
    };
    for track in animation
        .attachment_tracks
        .iter()
        .filter(|t| !t.keys.is_empty())
    {
        let name = entry(&mut out, track.slot);
        out[&name]["attachment"] = Value::Array(
            track
                .keys
                .iter()
                .map(|k| match &k.attachment {
                    Some(attachment) => json!({"time":k.time,"name":attachment}),
                    None => json!({"time":k.time}),
                })
                .collect(),
        );
    }
    for track in animation.color_tracks.iter().filter(|t| !t.keys.is_empty()) {
        let name = entry(&mut out, track.slot);
        let mut keys = vec![];
        for (i, key) in track.keys.iter().enumerate() {
            let [r, g, b, a] = key.color;
            let mut k = json!({"time":key.time,"color":format!("{r:02x}{g:02x}{b:02x}{a:02x}")});
            if key.curves.iter().all(|c| *c == Curve::Stepped) {
                k["curve"] = json!("stepped");
            } else if let Some(next) = track.keys.get(i + 1)
                && key.curves.iter().any(|c| matches!(c, Curve::Bezier(_)))
            {
                let mut handles = vec![];
                for channel in 0..4 {
                    let (v0, v1) = (
                        f32::from(key.color[channel]) / 255.,
                        f32::from(next.color[channel]) / 255.,
                    );
                    // A linear channel is the Bezier with handles at thirds.
                    let [x1, y1, x2, y2] = match key.curves[channel] {
                        Curve::Bezier(c) => c,
                        _ => [1. / 3., 1. / 3., 2. / 3., 2. / 3.],
                    };
                    handles.extend([
                        absolute_control(key.time, next.time, x1)?,
                        absolute_control(v0, v1, y1)?,
                        absolute_control(key.time, next.time, x2)?,
                        absolute_control(v0, v1, y2)?,
                    ]);
                }
                k["curve"] = json!(handles);
            }
            keys.push(k);
        }
        out[&name]["rgba"] = json!(keys);
    }
    Ok(out)
}
