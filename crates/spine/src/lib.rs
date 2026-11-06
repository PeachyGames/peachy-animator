//! Independently written Spine 4.3 JSON adapter. No runtime implementation dependency.
mod attachments;
use attachments::{inspect_attachment, read_attachment, write_attachment};
use peachy_document::*;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub const TARGET_VERSION: &str = "4.3.26";

fn timeline_properties(name: &str) -> Option<&'static [(Property, &'static str)]> {
    match name {
        "rotate" => Some(&[(Property::Rotation, "value")]),
        "translate" => Some(&[(Property::X, "x"), (Property::Y, "y")]),
        "translatex" => Some(&[(Property::X, "value")]),
        "translatey" => Some(&[(Property::Y, "value")]),
        "scale" => Some(&[(Property::ScaleX, "x"), (Property::ScaleY, "y")]),
        "scalex" => Some(&[(Property::ScaleX, "value")]),
        "scaley" => Some(&[(Property::ScaleY, "value")]),
        "shear" => Some(&[(Property::ShearX, "x"), (Property::ShearY, "y")]),
        "shearx" => Some(&[(Property::ShearX, "value")]),
        "sheary" => Some(&[(Property::ShearY, "value")]),
        _ => None,
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    pub spine_version: String,
    pub bones: usize,
    pub slots: usize,
    pub skins: usize,
    pub attachments: usize,
    pub animations: usize,
    pub supported: bool,
    pub unsupported: Vec<String>,
}

pub fn inspect(value: &Value) -> Inspection {
    let mut unsupported = vec![];
    inspect_shapes(value, &mut unsupported);
    if !value.is_object() || !value["bones"].is_array() || array(&value["bones"]).is_empty() {
        unsupported.push("A skeleton object with a nonempty bones array is required".into());
    }
    for field in ["slots", "skins"] {
        if value.get(field).is_some_and(|v| !v.is_array()) {
            unsupported.push(format!("{field} must be an array"));
        }
    }
    let version = value["skeleton"]["spine"].as_str().unwrap_or("unknown");
    if !version.starts_with("4.3.") {
        unsupported.push(format!("Spine {version}: only 4.3.x JSON is supported"));
    }
    for field in [
        "constraints",
        "ik",
        "transform",
        "path",
        "physics",
        "events",
    ] {
        if nonempty(&value[field]) {
            unsupported.push(format!("{field}: not supported in slice 1"));
        }
    }
    for bone in array(&value["bones"]) {
        let name = bone["name"].as_str().unwrap_or("?");
        inspect_numbers(
            bone,
            &[
                "x", "y", "rotation", "scaleX", "scaleY", "length", "shearX", "shearY",
            ],
            &format!("Bone {name}"),
            &mut unsupported,
        );
        if bone["skin"].as_bool() == Some(true) {
            unsupported.push(format!("Bone {name}: skin-required bone"));
        }
        for field in ["transform", "inherit"] {
            if bone
                .get(field)
                .is_some_and(|v| v.as_str() != Some("normal"))
            {
                unsupported.push(format!("Bone {name}: non-normal inheritance"));
            }
        }
        unknown_fields(
            bone,
            &[
                "name",
                "parent",
                "length",
                "x",
                "y",
                "rotation",
                "scaleX",
                "scaleY",
                "shearX",
                "shearY",
                "skin",
                "transform",
                "inherit",
                "color",
                "icon",
                "visible",
            ],
            &format!("Bone {name}"),
            &mut unsupported,
        );
    }
    for slot in array(&value["slots"]) {
        let name = slot["name"].as_str().unwrap_or("?");
        if slot["blend"].as_str().is_some_and(|v| v != "normal") {
            unsupported.push(format!("Slot {name}: blend mode"));
        }
        if slot["color"].as_str().is_some_and(|v| v != "ffffffff") || slot.get("dark").is_some() {
            unsupported.push(format!("Slot {name}: tint"));
        }
        unknown_fields(
            slot,
            &["name", "bone", "attachment", "blend", "color", "dark"],
            &format!("Slot {name}"),
            &mut unsupported,
        );
    }
    let mut attachments = 0;
    for skin in array(&value["skins"]) {
        unknown_fields(
            skin,
            &[
                "name",
                "attachments",
                "bones",
                "constraints",
                "ik",
                "transform",
                "path",
                "physics",
                "color",
            ],
            "Skin",
            &mut unsupported,
        );
        for field in ["bones", "constraints", "ik", "transform", "path", "physics"] {
            if nonempty(&skin[field]) {
                unsupported.push(format!("Skin {}: {field}", skin["name"]));
            }
        }
        if let Some(slots) = skin["attachments"].as_object() {
            for (slot, regions) in slots {
                if let Some(regions) = regions.as_object() {
                    for (name, region) in regions {
                        attachments += 1;
                        inspect_attachment(
                            region,
                            slot,
                            name,
                            array(&value["bones"]).len(),
                            &mut unsupported,
                        );
                    }
                }
            }
        }
    }
    if let Some(animations) = value["animations"].as_object() {
        for (name, animation) in animations {
            if let Some(fields) = animation.as_object() {
                for (field, contents) in fields {
                    if field != "bones" && nonempty(contents) {
                        unsupported.push(format!("Animation {name}: {field} timelines"));
                    }
                }
            }
            if let Some(bones) = animation["bones"].as_object() {
                for (bone, timelines) in bones {
                    if let Some(timelines) = timelines.as_object() {
                        let mut properties = BTreeSet::new();
                        for (field, keys) in timelines {
                            let channels = timeline_properties(field);
                            if let Some(channels) = channels {
                                for &(property, _) in channels {
                                    if !properties.insert(property) {
                                        unsupported.push(format!(
                                            "Animation {name}/{bone}: duplicate {property:?} track"
                                        ));
                                    }
                                }
                            } else {
                                unsupported
                                    .push(format!("Animation {name}: {field} bone timeline"));
                            }
                            if !keys.is_array() {
                                unsupported
                                    .push(format!("Animation {name}: {field} must be an array"));
                            }
                            for key in array(keys) {
                                inspect_numbers(
                                    key,
                                    &["time", "value", "x", "y"],
                                    &format!("Animation {name}/{field}"),
                                    &mut unsupported,
                                );
                                let allowed: &[&str] =
                                    if channels.is_some_and(|channels| channels.len() == 2) {
                                        &["time", "x", "y", "curve"]
                                    } else {
                                        &["time", "value", "curve"]
                                    };
                                unknown_fields(
                                    key,
                                    allowed,
                                    &format!("Animation {name}/{field}"),
                                    &mut unsupported,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    unknown_fields(
        value,
        &[
            "skeleton",
            "bones",
            "slots",
            "skins",
            "animations",
            "constraints",
            "ik",
            "transform",
            "path",
            "physics",
            "events",
        ],
        "Skeleton",
        &mut unsupported,
    );
    unsupported.sort();
    unsupported.dedup();
    Inspection {
        spine_version: version.into(),
        bones: array(&value["bones"]).len(),
        slots: array(&value["slots"]).len(),
        skins: array(&value["skins"]).len(),
        attachments,
        animations: value["animations"].as_object().map_or(0, |v| v.len()),
        supported: unsupported.is_empty(),
        unsupported,
    }
}

pub fn import_json(value: &Value, name: &str) -> Result<Document, String> {
    let report = inspect(value);
    if !report.supported {
        return Err(format!(
            "Import blocked to prevent data loss:\n{}",
            report.unsupported.join("\n")
        ));
    }
    let mut bones = vec![];
    let mut indices = BTreeMap::new();
    for bone in array(&value["bones"]) {
        let name = required_string(bone, "name")?;
        let parent = if let Some(parent) = bone["parent"].as_str() {
            Some(
                *indices
                    .get(parent)
                    .ok_or("Parent must appear before child")?,
            )
        } else {
            None
        };
        indices.insert(name.to_owned(), bones.len());
        bones.push(Bone {
            name: name.into(),
            parent,
            length: number(bone, "length", 0.0),
            setup: bone_transform(bone),
        });
    }
    let mut slots = vec![];
    for slot in array(&value["slots"]) {
        slots.push(Slot {
            name: required_string(slot, "name")?.into(),
            bone: *indices
                .get(required_string(slot, "bone")?)
                .ok_or("Unknown slot bone")?,
            attachment: slot["attachment"].as_str().map(str::to_owned),
        });
    }
    let mut skins = BTreeMap::new();
    for skin in array(&value["skins"]) {
        let mut mapped = Skin::new();
        if let Some(attachments) = skin["attachments"].as_object() {
            for (slot, regions) in attachments {
                let mut mapped_regions = BTreeMap::new();
                for (name, region) in regions.as_object().ok_or("Invalid attachments")? {
                    mapped_regions
                        .insert(name.clone(), read_attachment(region, name, bones.len())?);
                }
                mapped.insert(slot.clone(), mapped_regions);
            }
        }
        let name = required_string(skin, "name")?;
        if skins.insert(name.into(), mapped).is_some() {
            return Err(format!("Duplicate skin name {name}"));
        }
    }
    let mut animations = vec![];
    if let Some(entries) = value["animations"].as_object() {
        for (name, animation) in entries {
            let mut tracks = vec![];
            if let Some(entries) = animation["bones"].as_object() {
                for (bone, timelines) in entries {
                    let bone = *indices.get(bone).ok_or("Unknown animated bone")?;
                    for (timeline, keys) in timelines.as_object().ok_or("Invalid bone timelines")? {
                        let properties = timeline_properties(timeline)
                            .ok_or_else(|| format!("Unsupported timeline {timeline}"))?;
                        let keys = keys.as_array().ok_or("Timeline must be an array")?;
                        for (channel, &(property, field)) in properties.iter().enumerate() {
                            let mut mapped_keys = vec![];
                            for (i, key) in keys.iter().enumerate() {
                                let time = number(key, "time", 0.0);
                                let value = number(key, field, property.neutral());
                                let curve = match key.get("curve") {
                                    None => Curve::Linear,
                                    Some(Value::String(v)) if v == "stepped" => Curve::Stepped,
                                    Some(Value::Array(handles)) => {
                                        let next = keys.get(i + 1).ok_or("Bezier on last key")?;
                                        let next_time = number(next, "time", 0.0);
                                        let next_value = number(next, field, property.neutral());
                                        if next_time <= time {
                                            return Err("Unordered keyframes".into());
                                        }
                                        let offset = channel * 4;
                                        let h: Vec<f32> = handles
                                            .iter()
                                            .skip(offset)
                                            .take(4)
                                            .filter_map(|v| v.as_f64().map(|v| v as f32))
                                            .collect();
                                        if h.len() != 4 {
                                            return Err(
                                                "Expected four Bezier handles per channel".into()
                                            );
                                        }
                                        let delta = next_value - value;
                                        if delta.abs() < 1e-8 {
                                            if (h[1] - value).abs() > 1e-5
                                                || (h[3] - value).abs() > 1e-5
                                            {
                                                return Err("Bezier overshoot between equal values is not supported".into());
                                            }
                                            Curve::Linear
                                        } else {
                                            Curve::Bezier([
                                                (h[0] - time) / (next_time - time),
                                                (h[1] - value) / delta,
                                                (h[2] - time) / (next_time - time),
                                                (h[3] - value) / delta,
                                            ])
                                        }
                                    }
                                    _ => return Err("Unsupported interpolation".into()),
                                };
                                mapped_keys.push(Keyframe { time, value, curve });
                            }
                            tracks.push(Track {
                                bone,
                                property,
                                keys: mapped_keys,
                            });
                        }
                    }
                }
            }
            animations.push(Animation {
                name: name.clone(),
                tracks,
            });
        }
    }
    let fps = value["skeleton"]["fps"].as_u64().unwrap_or(30);
    if fps > 240 || fps == 0 {
        return Err("FPS must be 1–240".into());
    }
    let doc = Document {
        format: FORMAT_VERSION,
        name: name.into(),
        fps: fps as u32,
        bones,
        slots,
        skins,
        animations,
    };
    doc.validate()?;
    Ok(doc)
}

pub fn export_json(doc: &Document) -> Result<Value, String> {
    doc.validate()?;
    validate_export_names(doc)?;
    let bones: Vec<_> = doc
        .bones
        .iter()
        .map(|b| {
            let mut bone = bone_transform_json(b.setup);
            bone["name"] = json!(b.name);
            bone["length"] = json!(b.length);
            if let Some(parent) = b.parent {
                bone["parent"] = json!(doc.bones[parent].name);
            }
            bone
        })
        .collect();
    let slots: Vec<_> = doc
        .slots
        .iter()
        .map(|s| {
            let mut slot = json!({"name":s.name,"bone":doc.bones[s.bone].name});
            if let Some(a) = &s.attachment {
                slot["attachment"] = json!(a);
            }
            slot
        })
        .collect();
    let skins: Vec<_> = doc
        .skins
        .iter()
        .map(|(name, skin)| {
            let mut attachments = json!({});
            for (slot, regions) in skin {
                attachments[slot] = json!({});
                for (name, region) in regions {
                    attachments[slot][name] = write_attachment(region);
                }
            }
            json!({"name":name,"attachments":attachments})
        })
        .collect();
    let mut animations = json!({});
    for animation in &doc.animations {
        let mut bone_timelines = json!({});
        for track in &animation.tracks {
            if track.keys.is_empty() {
                continue;
            }
            let bone = &doc.bones[track.bone].name;
            let timeline = match track.property {
                Property::X => "translatex",
                Property::Y => "translatey",
                Property::Rotation => "rotate",
                Property::ScaleX => "scalex",
                Property::ScaleY => "scaley",
                Property::ShearX => "shearx",
                Property::ShearY => "sheary",
            };
            let mut keys = vec![];
            for (i, key) in track.keys.iter().enumerate() {
                let mut k = json!({"time":key.time,"value":key.value});
                match key.curve {
                    Curve::Stepped => k["curve"] = json!("stepped"),
                    Curve::Bezier([x1, y1, x2, y2]) if i + 1 < track.keys.len() => {
                        let next = track.keys[i + 1];
                        let dt = next.time - key.time;
                        let dv = next.value - key.value;
                        k["curve"] = json!([
                            key.time + x1 * dt,
                            key.value + y1 * dv,
                            key.time + x2 * dt,
                            key.value + y2 * dv
                        ]);
                    }
                    _ => {}
                }
                keys.push(k);
            }
            if bone_timelines.get(bone).is_none() {
                bone_timelines[bone] = json!({});
            }
            bone_timelines[bone][timeline] = json!(keys);
        }
        animations[&animation.name] = json!({"bones":bone_timelines});
    }
    Ok(
        json!({"skeleton":{"spine":TARGET_VERSION,"images":"./images/","fps":doc.fps},"bones":bones,"slots":slots,"skins":skins,"animations":animations}),
    )
}

fn array(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

/// Spine Editor interchange restrictions belong to this adapter, not our model.
fn validate_export_names(doc: &Document) -> Result<(), String> {
    let mut names = BTreeSet::new();
    for (skin_name, skin) in &doc.skins {
        for (slot, attachments) in skin {
            for (placeholder, region) in attachments {
                if !names.insert(region.name().unwrap_or(placeholder)) {
                    return Err(
                        "Spine Editor export requires distinct attachment names across skins"
                            .into(),
                    );
                }
                if skin_name != "default"
                    && doc
                        .skins
                        .get("default")
                        .and_then(|s| s.get(slot))
                        .is_some_and(|a| a.contains_key(placeholder))
                {
                    return Err(format!(
                        "Spine Editor export: skin placeholder {slot}/{placeholder} collides with a direct default attachment"
                    ));
                }
            }
        }
    }
    Ok(())
}

fn inspect_shapes(root: &Value, issues: &mut Vec<String>) {
    fn object(value: &Value, context: &str, issues: &mut Vec<String>) {
        if !value.is_object() {
            issues.push(format!("{context} must be an object"));
        }
    }
    fn field(value: &Value, key: &str, context: &str, issues: &mut Vec<String>) {
        if let Some(v) = value.get(key) {
            object(v, &format!("{context}.{key}"), issues);
        }
    }
    fn strings(value: &Value, keys: &[&str], context: &str, issues: &mut Vec<String>) {
        for key in keys {
            if value.get(key).is_some_and(|v| !v.is_string()) {
                issues.push(format!("{context}.{key} must be a string"));
            }
        }
    }
    object(root, "Skeleton", issues);
    field(root, "skeleton", "Skeleton", issues);
    field(root, "animations", "Skeleton", issues);
    if root["skeleton"]
        .get("fps")
        .is_some_and(|v| v.as_u64().is_none_or(|v| v == 0 || v > 240))
    {
        issues.push("Skeleton FPS must be an integer between 1 and 240".into());
    }
    for bone in array(&root["bones"]) {
        object(bone, "Bone", issues);
        strings(bone, &["name", "parent"], "Bone", issues);
    }
    for slot in array(&root["slots"]) {
        object(slot, "Slot", issues);
        strings(
            slot,
            &["name", "bone", "blend", "color", "dark"],
            "Slot",
            issues,
        );
        if slot
            .get("attachment")
            .is_some_and(|v| !v.is_null() && !v.is_string())
        {
            issues.push("Slot attachment must be a string or null".into());
        }
    }
    let mut skin_names = BTreeSet::new();
    for skin in array(&root["skins"]) {
        object(skin, "Skin", issues);
        strings(skin, &["name"], "Skin", issues);
        field(skin, "attachments", "Skin", issues);
        if let Some(name) = skin["name"].as_str()
            && !skin_names.insert(name)
        {
            issues.push(format!("Duplicate skin name {name}"));
        }
        if let Some(slots) = skin["attachments"].as_object() {
            for regions in slots.values() {
                object(regions, "Skin slot attachments", issues);
                if let Some(regions) = regions.as_object() {
                    for region in regions.values() {
                        object(region, "Attachment", issues);
                        strings(region, &["type", "name", "path"], "Attachment", issues);
                    }
                }
            }
        }
    }
    if let Some(animations) = root["animations"].as_object() {
        for animation in animations.values() {
            object(animation, "Animation", issues);
            field(animation, "bones", "Animation", issues);
            if let Some(bones) = animation["bones"].as_object() {
                for timelines in bones.values() {
                    object(timelines, "Bone timelines", issues);
                    if let Some(timelines) = timelines.as_object() {
                        for (name, keys) in timelines {
                            for key in array(keys) {
                                object(key, "Keyframe", issues);
                                if let Some(curve) = key.get("curve") {
                                    let handles = timeline_properties(name)
                                        .map_or(4, |channels| channels.len() * 4);
                                    let valid = curve.as_str() == Some("stepped")
                                        || curve.as_array().is_some_and(|a| {
                                            a.len() == handles
                                                && a.iter().all(|v| {
                                                    v.as_f64().is_some_and(|n| {
                                                        n.is_finite() && (n as f32).is_finite()
                                                    })
                                                })
                                        });
                                    if !valid {
                                        issues.push(format!("{name} curve must be stepped or have {handles} finite Bezier coordinates"));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
fn nonempty(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
        _ => true,
    }
}
fn number(value: &Value, key: &str, default: f32) -> f32 {
    value[key].as_f64().map_or(default, |v| v as f32)
}
fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key].as_str().ok_or_else(|| format!("Missing {key}"))
}
fn bone_transform(value: &Value) -> Transform {
    Transform {
        x: number(value, "x", 0.0),
        y: number(value, "y", 0.0),
        rotation: number(value, "rotation", 0.0),
        scale_x: number(value, "scaleX", 1.0),
        scale_y: number(value, "scaleY", 1.0),
        shear_x: number(value, "shearX", 0.0),
        shear_y: number(value, "shearY", 0.0),
    }
}
fn region_transform(value: &Value) -> RegionTransform {
    RegionTransform {
        x: number(value, "x", 0.0),
        y: number(value, "y", 0.0),
        rotation: number(value, "rotation", 0.0),
        scale_x: number(value, "scaleX", 1.0),
        scale_y: number(value, "scaleY", 1.0),
    }
}
fn region_transform_json(t: RegionTransform) -> Value {
    json!({"x":t.x,"y":t.y,"rotation":t.rotation,"scaleX":t.scale_x,"scaleY":t.scale_y})
}
fn bone_transform_json(t: Transform) -> Value {
    let mut value = region_transform_json(RegionTransform {
        x: t.x,
        y: t.y,
        rotation: t.rotation,
        scale_x: t.scale_x,
        scale_y: t.scale_y,
    });
    if t.shear_x != 0.0 {
        value["shearX"] = json!(t.shear_x);
    }
    if t.shear_y != 0.0 {
        value["shearY"] = json!(t.shear_y);
    }
    value
}
fn unknown_fields(value: &Value, allowed: &[&str], context: &str, issues: &mut Vec<String>) {
    if let Some(o) = value.as_object() {
        for key in o.keys() {
            if !allowed.contains(&key.as_str()) {
                issues.push(format!("{context}: unsupported field {key}"));
            }
        }
    }
}
fn inspect_numbers(value: &Value, fields: &[&str], context: &str, issues: &mut Vec<String>) {
    for field in fields {
        if value.get(field).is_some_and(|v| {
            v.as_f64()
                .is_none_or(|number| !number.is_finite() || !(number as f32).is_finite())
        }) {
            issues.push(format!("{context}: {field} must be a finite f32 number"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mesh_fixture() -> Document {
        serde_json::from_str(include_str!("../../../examples/peach-bot/mesh-bot.peachy")).unwrap()
    }
    #[test]
    fn unweighted_meshes_roundtrip_topology_uvs_edges_and_image_metadata() {
        let doc = mesh_fixture();
        let output = export_json(&doc).unwrap();
        assert!(inspect(&output).supported);
        let imported = import_json(&output, &doc.name).unwrap();
        assert_eq!(imported.skins, doc.skins);
        assert_eq!(imported.bones, doc.bones);
        let mut runtime_only = output;
        let m = &mut runtime_only["skins"][0]["attachments"]["hand.R"]["hand.R"];
        for field in ["width", "height", "edges"] {
            m.as_object_mut().unwrap().remove(field);
        }
        assert!(import_json(&runtime_only, "runtime").is_ok());
    }
    #[test]
    fn invalid_or_unsupported_mesh_semantics_never_get_silently_imported() {
        let fixture = export_json(&mesh_fixture()).unwrap();
        for (field, bad) in [
            (
                "vertices",
                json!([
                    1, 99, 0, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 1
                ]),
            ),
            ("uvs", json!([0, 0, 1])),
            ("triangles", json!([0, 1, 99])),
            ("triangles", json!([0, 0, 1])),
            ("hull", json!(99)),
            ("uvs", json!([0, 0, 1, 0, 1, 1, 0, 1, "bad", 0.5])),
            ("edges", json!([0, 3])),
            ("color", json!("ff0000ff")),
            ("type", json!("linkedmesh")),
            ("sequence", json!({"count":2})),
        ] {
            let mut v = fixture.clone();
            v["skins"][0]["attachments"]["hand.R"]["hand.R"][field] = bad;
            assert!(!inspect(&v).supported, "{field}");
            assert!(import_json(&v, "bad").is_err(), "{field}");
        }
        let mut v = fixture;
        v["animations"]["wave"]["attachments"] = json!({"default":{}});
        assert!(import_json(&v, "deform").is_err());
    }
    fn fixture() -> Value {
        json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}],"slots":[],"skins":[],"animations":{"test":{"bones":{"root":{"rotate":[{"time":0,"value":0,"curve":[0.1,0,0.4,90]},{"time":0.5,"value":90}]}}}}})
    }
    #[test]
    fn bezier_handles_roundtrip_between_absolute_and_normalized() {
        let d = import_json(&fixture(), "test").unwrap();
        assert_eq!(
            d.animations[0].tracks[0].keys[0].curve,
            Curve::Bezier([0.2, 0.0, 0.8, 1.0])
        );
        let out = export_json(&d).unwrap();
        let d2 = import_json(&out, "test").unwrap();
        assert_eq!(d, d2);
    }

    #[test]
    fn empty_frame_zero_and_tiny_animations_have_no_duration_floor_or_synthetic_keys() {
        for (keys, duration) in [
            (json!([]), 0.0),
            (json!([{"time":0,"value":7}]), 0.0),
            (
                json!([{"time":0,"value":7},{"time":0.001,"value":9}]),
                0.001,
            ),
        ] {
            let mut value = fixture();
            value["animations"]["test"]["bones"]["root"]["rotate"] = keys.clone();
            let doc = import_json(&value, "duration").unwrap();
            assert_eq!(doc.animations[0].duration(), duration);
            let output = export_json(&doc).unwrap();
            let bones = &output["animations"]["test"]["bones"];
            if keys.as_array().unwrap().is_empty() {
                assert_eq!(bones, &json!({}));
            } else {
                assert_eq!(
                    bones["root"]["rotate"].as_array().unwrap().len(),
                    keys.as_array().unwrap().len()
                );
                assert_eq!(
                    bones["root"]["rotate"].as_array().unwrap().last().unwrap()["time"]
                        .as_f64()
                        .unwrap() as f32,
                    duration
                );
            }
            let reopened = import_json(&output, "duration").unwrap();
            assert_eq!(reopened.animations[0].duration(), duration);
        }
        let mut value = fixture();
        value["animations"]["test"] = json!({});
        let empty = import_json(&value, "empty").unwrap();
        assert_eq!(empty.animations[0].duration(), 0.0);
        assert!(empty.animations[0].tracks.is_empty());
        assert_eq!(
            export_json(&empty).unwrap()["animations"]["test"],
            json!({"bones":{}})
        );
    }

    #[test]
    fn export_keeps_only_authored_keys_for_tracks_with_different_ends() {
        let mut value = fixture();
        value["animations"]["test"]["bones"]["root"] = json!({
            "rotate":[{"time":0,"value":0},{"time":4,"value":20}],
            "translatex":[{"time":0,"value":1,"curve":"stepped"},{"time":1,"value":2,"curve":"stepped"}],
            "translatey":[]
        });
        let doc = import_json(&value, "different ends").unwrap();
        assert_eq!(doc.animations[0].duration(), 4.0);
        let output = export_json(&doc).unwrap();
        let timelines = &output["animations"]["test"]["bones"]["root"];
        assert_eq!(timelines["rotate"].as_array().unwrap().len(), 2);
        assert_eq!(
            timelines["translatex"],
            json!([
                {"time":0.0,"value":1.0,"curve":"stepped"},
                {"time":1.0,"value":2.0,"curve":"stepped"}
            ])
        );
        assert!(timelines.get("translatey").is_none());
        let reopened = import_json(&output, "different ends").unwrap();
        assert_eq!(reopened.animations[0].duration(), 4.0);
        let x = reopened.animations[0]
            .tracks
            .iter()
            .find(|track| track.property == Property::X)
            .unwrap();
        assert_eq!(x.keys.last().unwrap().time, 1.0);
    }
    #[test]
    fn refuses_constraints_and_unknown_semantics() {
        let mut v = fixture();
        v["constraints"] = json!([{"type":"physics"}]);
        assert!(import_json(&v, "test").is_err());
        v["constraints"] = json!([]);
        v["bones"][0]["inherit"] = json!("noScale");
        assert!(import_json(&v, "test").is_err());
        v["bones"][0]["inherit"] = json!("normal");
        v["bones"][0]["futureField"] = json!(1);
        assert!(import_json(&v, "test").is_err());
    }
    #[test]
    fn rejects_duplicate_tracks_instead_of_silently_overwriting() {
        let mut v = fixture();
        v["animations"]["test"]["bones"]["root"]["translate"] = json!([{}]);
        v["animations"]["test"]["bones"]["root"]["translatex"] = json!([{}]);
        assert!(import_json(&v, "test").is_err());
    }
    #[test]
    fn malformed_numeric_fields_and_unknown_keys_never_silently_default() {
        let mut v = fixture();
        v["bones"][0]["x"] = json!("20");
        assert!(import_json(&v, "test").is_err());
        v["bones"][0]["x"] = json!(20);
        v["animations"]["test"]["bones"]["root"]["rotate"][0]["angle"] = json!(50);
        assert!(import_json(&v, "test").is_err());
    }

    #[test]
    fn malformed_containers_duplicate_skins_and_extra_curve_values_are_rejected() {
        let v = fixture();
        for (field, bad) in [
            ("animations", json!([])),
            ("skins", json!([{"name":"a","attachments":[]}])),
        ] {
            let mut candidate = v.clone();
            candidate[field] = bad;
            assert!(!inspect(&candidate).supported);
            assert!(import_json(&candidate, "test").is_err());
        }
        let mut candidate = v.clone();
        candidate["skins"] = json!([{"name":"a"},{"name":"a"}]);
        assert!(import_json(&candidate, "test").is_err());
        let mut candidate = v.clone();
        candidate["animations"]["test"]["bones"]["root"]["rotate"][0]["curve"] =
            json!([0.1, 0, 0.4, 90, 999]);
        assert!(import_json(&candidate, "test").is_err());
        let mut candidate = v;
        candidate["skeleton"]["fps"] = json!("60");
        assert!(import_json(&candidate, "test").is_err());
    }

    #[test]
    fn target_naming_restrictions_do_not_leak_into_the_document_model() {
        let mut doc = import_json(&fixture(), "test").unwrap();
        doc.slots.push(Slot {
            name: "body".into(),
            bone: 0,
            attachment: Some("shared".into()),
        });
        let region = Region {
            name: None,
            path: "body".into(),
            width: 10.0,
            height: 10.0,
            transform: RegionTransform::default(),
        };
        let skin = BTreeMap::from([(
            "body".into(),
            BTreeMap::from([("shared".into(), Attachment::Region(region))]),
        )]);
        doc.skins.insert("default".into(), skin.clone());
        doc.skins.insert("variant".into(), skin);
        assert!(doc.validate().is_ok());
        assert!(export_json(&doc).is_err());
    }
}
