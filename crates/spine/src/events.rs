//! Event definitions (root `events`) and animation `events` keys. Keys keep only
//! the values they override, exactly as written (installed 4.3.26 round-trips
//! them unchanged). Events do not affect the pose.
use super::{array, number, unknown_fields};
use peachy_document::{EventDef, EventKey};
use serde_json::{Value, json};

pub(super) fn read_defs(root: &Value) -> Result<Vec<EventDef>, String> {
    let Some(entries) = root.get("events") else {
        return Ok(vec![]);
    };
    let entries = entries.as_object().ok_or("events must be an object")?;
    let mut defs = vec![];
    for (name, entry) in entries {
        let context = format!("Event {name}");
        let mut issues = vec![];
        unknown_fields(
            entry,
            &["int", "float", "string", "audio", "volume", "balance"],
            &context,
            &mut issues,
        );
        if !issues.is_empty() || !entry.is_object() {
            return Err(if issues.is_empty() {
                format!("{context} must be an object")
            } else {
                issues.join("; ")
            });
        }
        let mut def = EventDef::new(name.clone());
        if let Some(v) = entry.get("int") {
            def.int = v
                .as_i64()
                .and_then(|v| i32::try_from(v).ok())
                .ok_or_else(|| format!("{context}: int must be a 32-bit integer"))?;
        }
        def.float = finite(entry, "float", 0.0, &context)?;
        def.volume = finite(entry, "volume", 1.0, &context)?;
        def.balance = finite(entry, "balance", 0.0, &context)?;
        if let Some(v) = entry.get("string") {
            def.string = v
                .as_str()
                .ok_or_else(|| format!("{context}: string must be text"))?
                .into();
        }
        if let Some(v) = entry.get("audio") {
            def.audio = Some(
                v.as_str()
                    .ok_or_else(|| format!("{context}: audio must be a path"))?
                    .into(),
            );
        }
        defs.push(def);
    }
    Ok(defs)
}

fn finite(entry: &Value, field: &str, default: f32, context: &str) -> Result<f32, String> {
    match entry.get(field) {
        None => Ok(default),
        Some(v) => v
            .as_f64()
            .map(|v| v as f32)
            .filter(|v| v.is_finite())
            .ok_or_else(|| format!("{context}: {field} must be a finite f32 number")),
    }
}

pub(super) fn read_keys(
    animation_name: &str,
    animation: &Value,
    defs: &[EventDef],
) -> Result<Vec<EventKey>, String> {
    let context = format!("Animation {animation_name}");
    if animation.get("events").is_some_and(|v| !v.is_array()) {
        return Err(format!("{context}: events must be an array"));
    }
    let mut keys = vec![];
    for key in array(&animation["events"]) {
        let mut issues = vec![];
        unknown_fields(
            key,
            &[
                "time", "name", "int", "float", "string", "volume", "balance",
            ],
            &context,
            &mut issues,
        );
        if !issues.is_empty() {
            return Err(issues.join("; "));
        }
        let name = key["name"]
            .as_str()
            .ok_or_else(|| format!("{context}: event keys need a name"))?;
        let event = defs
            .iter()
            .position(|d| d.name == name)
            .ok_or_else(|| format!("{context}: unknown event {name}"))?;
        let optional = |field: &str| -> Result<Option<f32>, String> {
            key.get(field)
                .map(|_| finite(key, field, 0.0, &context))
                .transpose()
        };
        keys.push(EventKey {
            time: number(key, "time", 0.0),
            event,
            int: key
                .get("int")
                .map(|v| {
                    v.as_i64()
                        .and_then(|v| i32::try_from(v).ok())
                        .ok_or_else(|| format!("{context}: int must be a 32-bit integer"))
                })
                .transpose()?,
            float: optional("float")?,
            string: key
                .get("string")
                .map(|v| {
                    v.as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| format!("{context}: string must be text"))
                })
                .transpose()?,
            volume: optional("volume")?,
            balance: optional("balance")?,
        });
    }
    Ok(keys)
}

pub(super) fn write_defs(defs: &[EventDef]) -> Value {
    let mut out = json!({});
    for def in defs {
        let mut value = json!({});
        if def.int != 0 {
            value["int"] = json!(def.int);
        }
        if def.float != 0.0 {
            value["float"] = json!(def.float);
        }
        if !def.string.is_empty() {
            value["string"] = json!(def.string);
        }
        if let Some(audio) = &def.audio {
            value["audio"] = json!(audio);
        }
        if def.volume != 1.0 {
            value["volume"] = json!(def.volume);
        }
        if def.balance != 0.0 {
            value["balance"] = json!(def.balance);
        }
        out[&def.name] = value;
    }
    out
}

pub(super) fn write_keys(keys: &[EventKey], defs: &[EventDef]) -> Value {
    Value::Array(
        keys.iter()
            .map(|key| {
                let mut value = json!({"time":key.time,"name":defs[key.event].name});
                if let Some(v) = key.int {
                    value["int"] = json!(v);
                }
                if let Some(v) = key.float {
                    value["float"] = json!(v);
                }
                if let Some(v) = &key.string {
                    value["string"] = json!(v);
                }
                if let Some(v) = key.volume {
                    value["volume"] = json!(v);
                }
                if let Some(v) = key.balance {
                    value["balance"] = json!(v);
                }
                value
            })
            .collect(),
    )
}
