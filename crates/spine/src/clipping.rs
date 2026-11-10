//! `clipping` attachments: polygon vertices (plain or weighted), `end` slot,
//! `convex` flag and nonessential color. Inverse clipping is refused.
use super::attachments::{read_mesh_vertices, write_vertices};
use super::{read_hex_color, unknown_fields};
use peachy_document::{Clipping, Document};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) const FIELDS: &[&str] = &[
    "type",
    "name",
    "end",
    "vertexCount",
    "vertices",
    "convex",
    "color",
    "inverse",
];

pub(super) fn inspect(value: &Value, context: &str, issues: &mut Vec<String>) {
    unknown_fields(value, FIELDS, context, issues);
    if value.get("inverse").is_some_and(|v| v != &json!(false)) {
        issues.push(format!("{context}: inverse clipping is unsupported"));
    }
}

pub(super) fn read(
    value: &Value,
    placeholder: &str,
    slots: &BTreeMap<String, usize>,
    bone_count: usize,
) -> Result<Clipping, String> {
    let context = format!("Clipping {placeholder}");
    let mut issues = vec![];
    inspect(value, &context, &mut issues);
    if !issues.is_empty() {
        return Err(issues.join("; "));
    }
    let count = value["vertexCount"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| format!("{context}: vertexCount must be a count"))?;
    let end = match value.get("end") {
        None | Some(Value::Null) => None,
        Some(name) => Some(
            *name
                .as_str()
                .and_then(|name| slots.get(name))
                .ok_or_else(|| format!("{context}: end must name a slot"))?,
        ),
    };
    let convex = match value.get("convex") {
        None => false,
        Some(v) => v
            .as_bool()
            .ok_or_else(|| format!("{context}: convex must be true or false"))?,
    };
    let clip = Clipping {
        name: value["name"].as_str().map(str::to_owned),
        end,
        vertices: read_mesh_vertices(&value["vertices"], count, bone_count)?,
        convex,
        color: value
            .get("color")
            .map(|_| read_hex_color(value, &format!("{context} color")))
            .transpose()?,
    };
    clip.validate(slots.len(), bone_count)?;
    Ok(clip)
}

pub(super) fn write(clip: &Clipping, doc: &Document) -> Value {
    let mut value = json!({
        "type": "clipping",
        "vertexCount": clip.vertex_count(),
        "vertices": write_vertices(&clip.vertices),
    });
    if let Some(end) = clip.end {
        value["end"] = json!(doc.slots[end].name);
    }
    if let Some(name) = &clip.name {
        value["name"] = json!(name);
    }
    if clip.convex {
        value["convex"] = json!(true);
    }
    if let Some([r, g, b, a]) = clip.color {
        value["color"] = json!(format!("{r:02x}{g:02x}{b:02x}{a:02x}"));
    }
    value
}
