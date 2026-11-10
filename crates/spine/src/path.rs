//! Path attachments, path constraints and position/spacing timelines.
//! Defaults observed through the external 4.3.13 runtime's public data API:
//! paths are open with constant speed; constraints default to percent
//! position, length spacing, tangent rotation and mixes of 1.
use super::attachments::{read_mesh_vertices, write_vertices};
use super::{array, number, read_hex_color, read_keys, unknown_fields, write_keys};
use peachy_document::{
    Document, Path, PathConstraint, PathProperty, PathSetup, PathTrack, PositionMode, RotateMode,
    SpacingMode,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) const ATTACHMENT_FIELDS: &[&str] = &[
    "type",
    "name",
    "closed",
    "constantSpeed",
    "vertexCount",
    "vertices",
    "lengths",
    "color",
];
const CONSTRAINT_FIELDS: &[&str] = &[
    "type",
    "name",
    "order",
    "skin",
    "bones",
    "slot",
    "positionMode",
    "spacingMode",
    "rotateMode",
    "rotation",
    "position",
    "spacing",
    "mixRotate",
    "mixX",
    "mixY",
];

fn flag(value: &Value, field: &str, default: bool, context: &str) -> Result<bool, String> {
    match value.get(field) {
        None => Ok(default),
        Some(v) => v
            .as_bool()
            .ok_or_else(|| format!("{context}: {field} must be true or false")),
    }
}

pub(super) fn read_attachment(
    value: &Value,
    placeholder: &str,
    bone_count: usize,
) -> Result<Path, String> {
    let context = format!("Path {placeholder}");
    let mut issues = vec![];
    unknown_fields(value, ATTACHMENT_FIELDS, &context, &mut issues);
    if !issues.is_empty() {
        return Err(issues.join("; "));
    }
    let count = value["vertexCount"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| format!("{context}: vertexCount must be a count"))?;
    let path = Path {
        name: value["name"].as_str().map(str::to_owned),
        vertices: read_mesh_vertices(&value["vertices"], count, bone_count)?,
        closed: flag(value, "closed", false, &context)?,
        constant_speed: flag(value, "constantSpeed", true, &context)?,
        lengths: array(&value["lengths"])
            .iter()
            .map(|v| {
                v.as_f64()
                    .map(|v| v as f32)
                    .ok_or_else(|| format!("{context}: lengths must be numbers"))
            })
            .collect::<Result<_, _>>()?,
        color: value
            .get("color")
            .map(|_| read_hex_color(value, &format!("{context} color")))
            .transpose()?,
    };
    path.validate(bone_count)?;
    Ok(path)
}

pub(super) fn write_attachment(path: &Path) -> Value {
    let mut value = json!({
        "type": "path",
        "vertexCount": path.vertex_count(),
        "vertices": write_vertices(&path.vertices),
        "lengths": path.lengths,
    });
    if let Some(name) = &path.name {
        value["name"] = json!(name);
    }
    if path.closed {
        value["closed"] = json!(true);
    }
    if !path.constant_speed {
        value["constantSpeed"] = json!(false);
    }
    if let Some([r, g, b, a]) = path.color {
        value["color"] = json!(format!("{r:02x}{g:02x}{b:02x}{a:02x}"));
    }
    value
}

pub(super) fn read_constraint(
    entry: &Value,
    bones: &BTreeMap<String, usize>,
    slots: &BTreeMap<String, usize>,
) -> Result<PathConstraint, String> {
    let name = entry["name"]
        .as_str()
        .ok_or("Path constraints need a name")?
        .to_owned();
    let context = format!("Path constraint {name}");
    let mut issues = vec![];
    unknown_fields(entry, CONSTRAINT_FIELDS, &context, &mut issues);
    if !issues.is_empty() {
        return Err(issues.join("; "));
    }
    let mode = |field: &str, options: &[&str]| -> Result<usize, String> {
        match entry.get(field) {
            None => Ok(usize::MAX),
            Some(v) => v
                .as_str()
                .and_then(|s| options.iter().position(|o| *o == s))
                .ok_or_else(|| format!("{context}: unknown {field}")),
        }
    };
    let position_mode = match mode("positionMode", &["fixed", "percent"])? {
        0 => PositionMode::Fixed,
        _ => PositionMode::Percent,
    };
    let spacing_mode = match mode(
        "spacingMode",
        &["length", "fixed", "percent", "proportional"],
    )? {
        1 => SpacingMode::Fixed,
        2 => SpacingMode::Percent,
        3 => SpacingMode::Proportional,
        _ => SpacingMode::Length,
    };
    let rotate_mode = match mode("rotateMode", &["tangent", "chain", "chainScale"])? {
        1 => RotateMode::Chain,
        2 => RotateMode::ChainScale,
        _ => RotateMode::Tangent,
    };
    let defaults = PathSetup::default();
    Ok(PathConstraint {
        bones: array(&entry["bones"])
            .iter()
            .map(|b| {
                b.as_str()
                    .and_then(|b| bones.get(b))
                    .copied()
                    .ok_or_else(|| format!("{context}: unknown bone"))
            })
            .collect::<Result<_, _>>()?,
        slot: *entry["slot"]
            .as_str()
            .and_then(|s| slots.get(s))
            .ok_or_else(|| format!("{context}: unknown slot"))?,
        position_mode,
        spacing_mode,
        rotate_mode,
        offset_rotation: number(entry, "rotation", 0.0),
        setup: PathSetup {
            position: number(entry, "position", defaults.position),
            spacing: number(entry, "spacing", defaults.spacing),
            mix_rotate: number(entry, "mixRotate", defaults.mix_rotate),
            mix_x: number(entry, "mixX", defaults.mix_x),
            mix_y: number(entry, "mixY", defaults.mix_y),
        },
        name,
    })
}

pub(super) fn write_constraint(path: &PathConstraint, doc: &Document) -> Value {
    json!({
        "type": "path", "name": path.name,
        "bones": path.bones.iter().map(|&b| &doc.bones[b].name).collect::<Vec<_>>(),
        "slot": doc.slots[path.slot].name,
        "positionMode": match path.position_mode { PositionMode::Fixed => "fixed", PositionMode::Percent => "percent" },
        "spacingMode": match path.spacing_mode {
            SpacingMode::Length => "length", SpacingMode::Fixed => "fixed",
            SpacingMode::Percent => "percent", SpacingMode::Proportional => "proportional",
        },
        "rotateMode": match path.rotate_mode {
            RotateMode::Tangent => "tangent", RotateMode::Chain => "chain", RotateMode::ChainScale => "chainScale",
        },
        "rotation": path.offset_rotation,
        "position": path.setup.position, "spacing": path.setup.spacing,
        "mixRotate": path.setup.mix_rotate, "mixX": path.setup.mix_x, "mixY": path.setup.mix_y,
    })
}

/// `animations.<name>.path`: position and spacing keys; mix keys are refused.
pub(super) fn read_timelines(
    animation_name: &str,
    animation: &Value,
    constraints: &BTreeMap<String, usize>,
) -> Result<Vec<PathTrack>, String> {
    let context = format!("Animation {animation_name}");
    let Some(entries) = animation.get("path") else {
        return Ok(vec![]);
    };
    let entries = entries
        .as_object()
        .ok_or_else(|| format!("{context}: path must be an object"))?;
    let mut tracks = vec![];
    for (name, timelines) in entries {
        let constraint = *constraints.get(name).ok_or_else(|| {
            format!("{context}: path timelines reference unknown path constraint {name}")
        })?;
        for (timeline, keys) in timelines
            .as_object()
            .ok_or_else(|| format!("{context}: path {name} must be an object"))?
        {
            let property = match timeline.as_str() {
                "position" => PathProperty::Position,
                "spacing" => PathProperty::Spacing,
                other => return Err(format!("{context}: path {other} timelines are unsupported")),
            };
            let keys = keys
                .as_array()
                .ok_or_else(|| format!("{context}: path {timeline} must be an array"))?;
            for key in keys {
                let mut issues = vec![];
                unknown_fields(key, &["time", "value", "curve"], &context, &mut issues);
                if !issues.is_empty() {
                    return Err(issues.join("; "));
                }
            }
            tracks.push(PathTrack {
                constraint,
                property,
                keys: read_keys(keys, "value", 0, 0.0).map_err(|e| format!("{context}: {e}"))?,
            });
        }
    }
    Ok(tracks)
}

pub(super) fn write_timelines(
    doc: &Document,
    animation: &peachy_document::Animation,
) -> Result<Value, String> {
    let mut out = json!({});
    for track in &animation.path_tracks {
        let name = doc.constraints[track.constraint].name();
        if out.get(name).is_none() {
            out[name] = json!({});
        }
        out[name][track.property.timeline()] = json!(write_keys(&track.keys)?);
    }
    Ok(out)
}
