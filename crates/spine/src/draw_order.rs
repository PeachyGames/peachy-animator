//! Animation `drawOrder` keys. Offsets are kept exactly as written; invalid
//! lists (out of setup order, repeated, colliding or out of range) are refused
//! because the external runtime throws or produces empty draw positions for them.
use super::{number, unknown_fields};
use peachy_document::{Document, DrawOrderKey, DrawOrderOffset};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) fn read(
    animation_name: &str,
    animation: &Value,
    slots: &BTreeMap<String, usize>,
) -> Result<Vec<DrawOrderKey>, String> {
    let context = format!("Animation {animation_name}");
    let Some(keys) = animation.get("drawOrder") else {
        return Ok(vec![]);
    };
    let keys = keys.as_array().ok_or_else(|| {
        format!("{context}: drawOrder must be an array (draw order folders are unsupported)")
    })?;
    let mut out = vec![];
    for key in keys {
        let mut issues = vec![];
        unknown_fields(key, &["time", "offsets"], &context, &mut issues);
        if !issues.is_empty() {
            return Err(issues.join("; "));
        }
        let mut offsets = vec![];
        if let Some(entries) = key.get("offsets") {
            for entry in entries
                .as_array()
                .ok_or_else(|| format!("{context}: drawOrder offsets must be an array"))?
            {
                let mut issues = vec![];
                unknown_fields(entry, &["slot", "offset"], &context, &mut issues);
                if !issues.is_empty() {
                    return Err(issues.join("; "));
                }
                let slot = entry["slot"]
                    .as_str()
                    .and_then(|name| slots.get(name))
                    .ok_or_else(|| {
                        format!("{context}: drawOrder offsets must name a known slot")
                    })?;
                let offset = entry["offset"]
                    .as_i64()
                    .and_then(|v| i32::try_from(v).ok())
                    .ok_or_else(|| format!("{context}: drawOrder offset must be an integer"))?;
                offsets.push(DrawOrderOffset {
                    slot: *slot,
                    offset,
                });
            }
        }
        let key = DrawOrderKey {
            time: number(key, "time", 0.0),
            offsets,
        };
        key.order(slots.len())
            .map_err(|error| format!("{context}: {error}"))?;
        out.push(key);
    }
    Ok(out)
}

pub(super) fn write(doc: &Document, keys: &[DrawOrderKey]) -> Value {
    Value::Array(
        keys.iter()
            .map(|key| {
                let mut value = json!({"time": key.time});
                if !key.offsets.is_empty() {
                    value["offsets"] = key
                        .offsets
                        .iter()
                        .map(|o| json!({"slot": doc.slots[o.slot].name, "offset": o.offset}))
                        .collect();
                }
                value
            })
            .collect(),
    )
}
