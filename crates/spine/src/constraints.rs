//! Independently written, bounded Spine 4.3 constraint interchange: IK with
//! grouped Mix/Positive timelines, and physics data. Source-feature checks
//! only: solving belongs to the engine.
use super::{array, ik_timelines, required_string, unknown_fields};
use peachy_document::{
    Constraint, Document, IkConstraint, IkSetup, PHYSICS_DEFAULT_FPS, PHYSICS_DEFAULT_LIMIT,
    PHYSICS_DEFAULT_SETUP, PhysicsConstraint, PhysicsGlobal, PhysicsInfluence, PhysicsProperty,
    PhysicsReset, PhysicsTrack, Property, ScaleYMode,
};
use peachy_document::{
    TransformConstraint, TransformMapping, TransformProperty, TransformTarget, TransformValues,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) fn inspect(root: &Value, issues: &mut Vec<String>) {
    if root.get("ik").is_some() {
        issues.push("Legacy root ik is unsupported; Spine 4.3 uses constraints".into());
    }
    let indices = array(&root["bones"])
        .iter()
        .enumerate()
        .filter_map(|(i, bone)| bone["name"].as_str().map(|name| (name.to_owned(), i)))
        .collect();
    let constraints = match read(root, &indices) {
        Ok(value) => value,
        Err(error) => {
            issues.push(error);
            vec![]
        }
    };
    let names = ik_timelines::indices(&constraints);
    if let Some(animations) = root["animations"].as_object() {
        for (name, animation) in animations {
            if let Err(error) = ik_timelines::read(animation, &names) {
                issues.push(format!("Animation {name}: {error}"));
            }
        }
    }
}

pub(super) fn read(
    root: &Value,
    indices: &BTreeMap<String, usize>,
) -> Result<Vec<Constraint>, String> {
    let Some(entries) = root.get("constraints") else {
        return Ok(vec![]);
    };
    let entries = entries.as_array().ok_or("constraints must be an array")?;
    let mut constraints = vec![];
    let mut errors = vec![];
    for entry in entries {
        if !entry.is_object() {
            errors.push("Constraint must be an object".to_owned());
            continue;
        }
        let result = match entry["type"].as_str() {
            Some("ik") => read_ik(entry, indices).map(Constraint::Ik),
            Some("physics") => read_physics(entry, indices).map(Constraint::Physics),
            Some("transform") => read_transform(entry, indices).map(Constraint::Transform),
            Some(kind @ ("path" | "slider")) => Err(format!(
                "{} constraint {}: {kind} constraints are unsupported",
                capitalized(kind),
                entry["name"].as_str().unwrap_or("?")
            )),
            _ => Err(format!(
                "Constraint {}: unknown or missing type",
                entry["name"].as_str().unwrap_or("?")
            )),
        };
        match result {
            Ok(constraint) => constraints.push(constraint),
            Err(error) => errors.push(error),
        }
    }
    if errors.is_empty() {
        Ok(constraints)
    } else {
        Err(errors.join("; "))
    }
}

fn capitalized(kind: &str) -> String {
    let mut chars = kind.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn resolve_bone(
    indices: &BTreeMap<String, usize>,
    value: &Value,
    context: &str,
) -> Result<usize, String> {
    let bone = value
        .as_str()
        .ok_or_else(|| format!("{context}: bone references must be names"))?;
    indices
        .get(bone)
        .copied()
        .ok_or_else(|| format!("{context}: unknown bone {bone}"))
}

/// Obsolete editor metadata is recognized but never cast, stored, sorted or
/// exported. Array order is the observed 4.3 evaluation order.
fn check_order(entry: &Value, context: &str) -> Result<(), String> {
    if entry.get("order").is_some_and(|v| {
        v.as_f64()
            .is_none_or(|n| !n.is_finite() || n < 0.0 || n.fract() != 0.0)
    }) {
        return Err(format!(
            "{context}: order must be a finite nonnegative integer"
        ));
    }
    Ok(())
}

fn read_ik(entry: &Value, indices: &BTreeMap<String, usize>) -> Result<IkConstraint, String> {
    let mut issues = vec![];
    unknown_fields(
        entry,
        &[
            "type",
            "name",
            "bones",
            "target",
            "mix",
            "bendPositive",
            "softness",
            "compress",
            "stretch",
            "scaleY",
            "order",
            "skin",
        ],
        &format!("IK constraint {}", entry["name"].as_str().unwrap_or("?")),
        &mut issues,
    );
    if !issues.is_empty() {
        return Err(issues.join("; "));
    }
    let name = required_string(entry, "name")?;
    let context = || format!("IK {name}");
    let mix = match entry.get("mix") {
        None => 1.0,
        Some(value) => {
            let number = value
                .as_f64()
                .filter(|v| v.is_finite() && (*v as f32).is_finite())
                .ok_or_else(|| format!("{}: mix must be a finite native number", context()))?;
            number as f32
        }
    };
    let bend_positive = match entry.get("bendPositive") {
        None => true,
        Some(value) => value
            .as_bool()
            .ok_or_else(|| format!("{}: bendPositive must be boolean", context()))?,
    };
    // Test the source number, before any f32 rounding/underflow. Unsupported
    // nonzero softness must never become an apparently neutral zero.
    if entry
        .get("softness")
        .is_some_and(|v| v.as_f64() != Some(0.0))
    {
        return Err(format!(
            "{}: nonzero or malformed softness is unsupported",
            context()
        ));
    }
    if entry.get("skin").is_some_and(|v| !v.is_boolean()) {
        return Err(format!("{}: skin must be boolean", context()));
    }
    for field in ["compress", "stretch"] {
        if entry.get(field).is_some_and(|v| v.as_bool() != Some(false)) {
            return Err(format!("{}: {field} must be omitted or false", context()));
        }
    }
    if entry
        .get("scaleY")
        .is_some_and(|v| v.as_str() != Some("none"))
    {
        return Err(format!("{}: scaleY must be omitted or none", context()));
    }
    check_order(entry, &context())?;
    let bones = entry["bones"]
        .as_array()
        .ok_or_else(|| format!("{}: bones must be an array", context()))?
        .iter()
        .map(|value| resolve_bone(indices, value, &context()))
        .collect::<Result<Vec<_>, _>>()?;
    let target = resolve_bone(indices, &entry["target"], &context())?;
    Ok(IkConstraint {
        name: name.into(),
        bones,
        target,
        setup: IkSetup { mix, bend_positive },
    })
}

fn read_transform(
    entry: &Value,
    indices: &BTreeMap<String, usize>,
) -> Result<TransformConstraint, String> {
    let name = required_string(entry, "name")?;
    let context = format!("Transform {name}");
    let mut issues = vec![];
    let mut allowed = vec![
        "type",
        "name",
        "order",
        "skin",
        "bones",
        "source",
        "properties",
        "localSource",
        "localTarget",
        "additive",
        "clamp",
    ];
    for property in TransformProperty::ALL {
        allowed.push(property.offset_name());
        allowed.push(property.mix_name());
    }
    unknown_fields(entry, &allowed, &context, &mut issues);
    if !issues.is_empty() {
        return Err(issues.join("; "));
    }
    if entry.get("skin").is_some_and(|v| !v.is_boolean()) {
        return Err(format!("{context}: skin must be boolean"));
    }
    check_order(entry, &context)?;
    let finite = |value: &Value, what: &str| -> Result<f32, String> {
        value
            .as_f64()
            .map(|v| v as f32)
            .filter(|v| v.is_finite())
            .ok_or_else(|| format!("{context}: {what} must be a finite f32 number"))
    };
    let number = |field: &str, default: f32| -> Result<f32, String> {
        entry.get(field).map_or(Ok(default), |v| finite(v, field))
    };
    let flag = |field: &str| -> Result<bool, String> {
        entry.get(field).map_or(Ok(false), |v| {
            v.as_bool()
                .ok_or_else(|| format!("{context}: {field} must be boolean"))
        })
    };
    let mut mappings = vec![];
    if let Some(properties) = entry.get("properties") {
        let properties = properties
            .as_object()
            .ok_or_else(|| format!("{context}: properties must be an object"))?;
        for (from_name, mapping) in properties {
            let from = TransformProperty::from_spine_name(from_name)
                .ok_or_else(|| format!("{context}: unknown property {from_name}"))?;
            let mut issues = vec![];
            unknown_fields(mapping, &["offset", "to"], &context, &mut issues);
            if !issues.is_empty() {
                return Err(issues.join("; "));
            }
            let to = mapping["to"]
                .as_object()
                .ok_or_else(|| format!("{context}: {from_name}.to must be an object"))?;
            let mut targets = vec![];
            for (to_name, target) in to {
                let property = TransformProperty::from_spine_name(to_name)
                    .ok_or_else(|| format!("{context}: unknown property {to_name}"))?;
                let mut issues = vec![];
                unknown_fields(target, &["offset", "max", "scale"], &context, &mut issues);
                if !issues.is_empty() || !target.is_object() {
                    return Err(format!("{context}: invalid {from_name}.to.{to_name}"));
                }
                let field = |key: &str, default: f32| {
                    target.get(key).map_or(Ok(default), |v| finite(v, key))
                };
                targets.push(TransformTarget {
                    property,
                    offset: field("offset", 0.0)?,
                    max: field("max", 1.0)?,
                    scale: field("scale", 1.0)?,
                });
            }
            mappings.push(TransformMapping {
                from,
                offset: mapping
                    .get("offset")
                    .map_or(Ok(0.0), |v| finite(v, "offset"))?,
                to: targets,
            });
        }
    }
    let constraint_targets: Vec<_> = mappings
        .iter()
        .flat_map(|m| m.to.iter().map(|t| t.property))
        .collect();
    // Observed: a mapped property's mix defaults to 1, mixY to mixX and
    // mixScaleY to mixScaleX; the runtime forces unmapped mixes to zero, so a
    // nonzero unmapped mix would be data the runtime ignores and is refused.
    let raw = |p: TransformProperty, default: f32| number(p.mix_name(), default);
    let mix_x = raw(TransformProperty::X, 1.0)?;
    let mix_scale_x = raw(TransformProperty::ScaleX, 1.0)?;
    let mut mix = TransformValues::default();
    let mut unmapped = None;
    for property in TransformProperty::ALL {
        let default = match property {
            TransformProperty::Y => mix_x,
            TransformProperty::ScaleY => mix_scale_x,
            _ => 1.0,
        };
        let value = raw(property, default)?;
        if constraint_targets.contains(&property) {
            mix.set(property, value);
        } else if entry.get(property.mix_name()).is_some() && value != 0.0 && unmapped.is_none() {
            unmapped = Some(format!(
                "{context}: {} has no effect without a {} mapping",
                property.mix_name(),
                property.spine_name()
            ));
        }
    }
    let mut offsets = TransformValues::default();
    for property in TransformProperty::ALL {
        offsets.set(property, number(property.offset_name(), 0.0)?);
    }
    let bones = entry["bones"]
        .as_array()
        .ok_or_else(|| format!("{context}: bones must be an array"))?
        .iter()
        .map(|value| resolve_bone(indices, value, &context))
        .collect::<Result<Vec<_>, _>>()?;
    let constraint = TransformConstraint {
        name: name.into(),
        bones,
        source: resolve_bone(indices, &entry["source"], &context)?,
        offsets,
        mix,
        mappings,
        local_source: flag("localSource")?,
        local_target: flag("localTarget")?,
        additive: flag("additive")?,
        clamp: flag("clamp")?,
    };
    // Report an unsupported mapping before a mix it would make meaningless.
    constraint.supported_subset()?;
    match unmapped {
        Some(error) => Err(error),
        None => Ok(constraint),
    }
}

const PHYSICS_NUMBERS: [&str; 13] = [
    "x", "y", "rotate", "scaleX", "shearX", "limit", "fps", "inertia", "strength", "damping",
    "mass", "wind", "gravity",
];

fn read_physics(
    entry: &Value,
    indices: &BTreeMap<String, usize>,
) -> Result<PhysicsConstraint, String> {
    let name = required_string(entry, "name")?;
    let context = format!("Physics {name}");
    let mut issues = vec![];
    let mut allowed = vec!["type", "name", "order", "skin", "bone", "scaleY", "mix"];
    allowed.extend(PHYSICS_NUMBERS);
    let globals = PhysicsProperty::ALL.map(|p| format!("{}Global", p.timeline()));
    allowed.extend(globals.iter().map(String::as_str));
    unknown_fields(entry, &allowed, &context, &mut issues);
    if !issues.is_empty() {
        return Err(issues.join("; "));
    }
    // Check source numbers before f32 conversion so an overflow can never
    // become an apparently valid infinity or a silently rounded zero.
    let number = |field: &str, default: f32| -> Result<f32, String> {
        match entry.get(field) {
            None => Ok(default),
            Some(value) => value
                .as_f64()
                .map(|v| v as f32)
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("{context}: {field} must be a finite f32 number")),
        }
    };
    if entry.get("skin").is_some_and(|v| !v.is_boolean()) {
        return Err(format!("{context}: skin must be boolean"));
    }
    check_order(entry, &context)?;
    let scale_y_mode = match entry.get("scaleY").map(Value::as_str) {
        None | Some(Some("none")) => ScaleYMode::None,
        Some(Some("uniform")) => ScaleYMode::Uniform,
        Some(Some("volume")) => ScaleYMode::Volume,
        _ => {
            return Err(format!("{context}: scaleY must be none, uniform or volume"));
        }
    };
    let mut setup = PHYSICS_DEFAULT_SETUP;
    let mut global = PhysicsGlobal::default();
    for property in PhysicsProperty::ALL {
        let value = number(property.timeline(), property.read(PHYSICS_DEFAULT_SETUP))?;
        property.write(&mut setup, value);
        let field = format!("{}Global", property.timeline());
        let flag = match entry.get(&field) {
            None => false,
            Some(value) => value
                .as_bool()
                .ok_or_else(|| format!("{context}: {field} must be boolean"))?,
        };
        property.set_global(&mut global, flag);
    }
    let constraint = PhysicsConstraint {
        name: name.into(),
        bone: resolve_bone(indices, &entry["bone"], &context)?,
        influence: PhysicsInfluence {
            x: number("x", 0.0)?,
            y: number("y", 0.0)?,
            rotate: number("rotate", 0.0)?,
            scale_x: number("scaleX", 0.0)?,
            shear_x: number("shearX", 0.0)?,
        },
        scale_y_mode,
        limit: number("limit", PHYSICS_DEFAULT_LIMIT)?,
        fps: number("fps", PHYSICS_DEFAULT_FPS)?,
        setup,
        global,
    };
    // Range checks share the native validation (positive mass/FPS, etc.).
    constraint.validate_settings()?;
    Ok(constraint)
}

/// Decodes `animations.<name>.physics`: constraint name (or "" for global
/// timelines) -> setting -> keys, plus `reset` time-only keys.
pub(super) fn read_physics_timelines(
    animation_name: &str,
    animation: &Value,
    constraints: &BTreeMap<String, usize>,
) -> Result<(Vec<PhysicsTrack>, Vec<PhysicsReset>), String> {
    let (mut tracks, mut resets) = (vec![], vec![]);
    let Some(entries) = animation.get("physics") else {
        return Ok((tracks, resets));
    };
    let context = format!("Animation {animation_name}");
    let entries = entries
        .as_object()
        .ok_or_else(|| format!("{context}: physics must be an object"))?;
    for (constraint_name, timelines) in entries {
        let constraint = if constraint_name.is_empty() {
            None
        } else {
            Some(*constraints.get(constraint_name).ok_or_else(|| {
                format!(
                    "{context}: physics timelines reference unknown constraint {constraint_name}"
                )
            })?)
        };
        let timelines = timelines
            .as_object()
            .ok_or_else(|| format!("{context}: physics {constraint_name} must be an object"))?;
        for (timeline, keys) in timelines {
            let keys = keys
                .as_array()
                .ok_or_else(|| format!("{context}: physics {timeline} must be an array"))?;
            let mut issues = vec![];
            for key in keys {
                if !key.is_object() {
                    issues.push(format!(
                        "{context}: physics {timeline} keys must be objects"
                    ));
                }
                let numbers: &[&str] = if timeline == "reset" {
                    &["time"]
                } else {
                    &["time", "value"]
                };
                for field in numbers {
                    if key.get(field).is_some_and(|v| {
                        v.as_f64()
                            .is_none_or(|n| !n.is_finite() || !(n as f32).is_finite())
                    }) {
                        issues.push(format!(
                            "{context}: physics {timeline} {field} must be a finite f32 number"
                        ));
                    }
                }
                let allowed: &[&str] = if timeline == "reset" {
                    &["time"]
                } else {
                    &["time", "value", "curve"]
                };
                unknown_fields(
                    key,
                    allowed,
                    &format!("{context}/physics {timeline}"),
                    &mut issues,
                );
                if let Some(curve) = key.get("curve")
                    && curve.as_str() != Some("stepped")
                    && !curve.as_array().is_some_and(|handles| {
                        handles.len() == 4
                            && handles.iter().all(|v| {
                                v.as_f64()
                                    .is_some_and(|n| n.is_finite() && (n as f32).is_finite())
                            })
                    })
                {
                    issues.push(format!(
                        "{context}: physics {timeline} curve must be stepped or have 4 finite Bezier coordinates"
                    ));
                }
            }
            if !issues.is_empty() {
                return Err(issues.join("; "));
            }
            if timeline == "reset" {
                resets.push(PhysicsReset {
                    constraint,
                    times: keys
                        .iter()
                        .map(|key| key["time"].as_f64().unwrap_or(0.0) as f32)
                        .collect(),
                });
                continue;
            }
            let property = PhysicsProperty::from_timeline(timeline)
                .ok_or_else(|| format!("{context}: unsupported physics timeline {timeline}"))?;
            tracks.push(PhysicsTrack {
                constraint,
                property,
                keys: super::read_keys(keys, "value", 0, property.omitted_key_value())?,
            });
        }
    }
    Ok((tracks, resets))
}

fn inside(doc: &Document, mut bone: usize, root: usize) -> bool {
    loop {
        if bone == root {
            return true;
        }
        match doc.bones[bone].parent {
            Some(parent) => bone = parent,
            None => return false,
        }
    }
}

/// Decidable static/source restrictions, shared by import and export. The caller
/// validates the native graph first. There is deliberately no sampled solver or
/// claim that all intermediate target poses are numerically evaluable.
pub(super) fn validate_source(doc: &Document) -> Result<(), String> {
    for transform in doc.constraints.iter().filter_map(Constraint::as_transform) {
        let context = format!("Transform {}", transform.name);
        transform.supported_subset()?;
        for (i, &bone) in transform.bones.iter().enumerate() {
            let keyed = doc.animations.iter().any(|animation| {
                animation
                    .inherit_tracks
                    .iter()
                    .any(|track| track.bone == bone && !track.keys.is_empty())
            });
            if doc.bones[bone].inherit != peachy_document::Inherit::Normal || keyed {
                return Err(format!(
                    "{context}: constrained bone {} must inherit normally",
                    doc.bones[bone].name
                ));
            }
            if inside(doc, transform.source, bone)
                || transform.bones[..i]
                    .iter()
                    .any(|&other| inside(doc, bone, other) || inside(doc, other, bone))
            {
                return Err(format!(
                    "{context}: the source and constrained bones must not contain one another"
                ));
            }
        }
    }
    let setup: Vec<_> = doc
        .constraints
        .iter()
        .filter_map(Constraint::as_ik)
        .filter(|ik| ik.setup.mix != 0.0)
        .collect();
    validate_context(doc, &setup, None)?;
    for animation in &doc.animations {
        let active: Vec<_> = doc
            .constraints
            .iter()
            .enumerate()
            .filter_map(|(index, constraint)| {
                let ik = constraint.as_ik()?;
                let track = animation
                    .ik_tracks
                    .iter()
                    .find(|track| track.constraint == index);
                ik_timelines::potentially_active(ik.setup.mix, track).then_some(ik)
            })
            .collect();
        validate_context(doc, &active, Some(animation))?;
    }
    Ok(())
}

fn validate_context(
    doc: &Document,
    active: &[&IkConstraint],
    clip: Option<&peachy_document::Animation>,
) -> Result<(), String> {
    for (i, ik) in active.iter().enumerate() {
        let first = ik.bones[0];
        // Spine: bones affected by IK cannot disable transform inheritance.
        for &bone in &ik.bones {
            if doc.bones[bone].inherit != peachy_document::Inherit::Normal
                || doc.animations.iter().any(|animation| {
                    animation
                        .inherit_tracks
                        .iter()
                        .any(|track| track.bone == bone && !track.keys.is_empty())
                })
            {
                return Err(format!(
                    "IK {}: constrained bone {} must inherit normally",
                    ik.name, doc.bones[bone].name
                ));
            }
        }
        // Spine: bones affected by IK cannot disable transform inheritance.
        for &bone in &ik.bones {
            if doc.bones[bone].inherit != peachy_document::Inherit::Normal
                || doc.animations.iter().any(|animation| {
                    animation
                        .inherit_tracks
                        .iter()
                        .any(|track| track.bone == bone && !track.keys.is_empty())
                })
            {
                return Err(format!(
                    "IK {}: constrained bone {} must inherit normally",
                    ik.name, doc.bones[bone].name
                ));
            }
        }
        if doc.bones[first].parent.is_none() {
            return Err(format!(
                "IK {}: root-constrained chains are unsupported",
                ik.name
            ));
        }
        for other in active.iter().skip(i + 1) {
            // Nested chains are evaluated sequentially in array order. Shared
            // bones and targets inside another constrained subtree are refused.
            if ik.bones.iter().any(|bone| other.bones.contains(bone))
                || inside(doc, ik.target, other.bones[0])
                || inside(doc, other.target, first)
            {
                return Err(format!(
                    "IK {} / {}: shared bones or targets inside another constrained chain are unsupported",
                    ik.name, other.name
                ));
            }
        }
        // Only the constrained bones need unit axes; ancestors may be transformed.
        let mut chain = Some(*ik.bones.last().unwrap());
        while let Some(index) = chain.filter(|i| ik.bones.contains(i)) {
            let bone = &doc.bones[index];
            if bone.setup.scale_x != 1.0
                || bone.setup.scale_y != 1.0
                || bone.setup.shear_x != 0.0
                || bone.setup.shear_y != 0.0
            {
                return Err(format!(
                    "IK {}: bone {} requires unit positive axes and zero shear",
                    ik.name, bone.name
                ));
            }
            if let Some(animation) = clip {
                for track in animation.tracks.iter().filter(|track| track.bone == index) {
                    if matches!(
                        track.property,
                        Property::ScaleX | Property::ScaleY | Property::ShearX | Property::ShearY
                    ) && track
                        .keys
                        .iter()
                        .any(|key| key.value != track.property.neutral())
                    {
                        return Err(format!(
                            "IK {}: animation {} bone {} has a non-neutral scale/shear track",
                            ik.name, animation.name, bone.name
                        ));
                    }
                }
            }
            chain = bone.parent;
        }
        if ik.bones.len() == 2 {
            let index = ik.bones[1];
            let child = &doc.bones[index];
            if f64::from(child.setup.x).hypot(f64::from(child.setup.y)) == 0.0
                || child.length <= 0.0
            {
                return Err(format!(
                    "IK {}: child needs a nonzero setup offset and positive length",
                    ik.name
                ));
            }
            if let Some(animation) = clip
                && animation.tracks.iter().any(|track| {
                    track.bone == index
                        && matches!(track.property, Property::X | Property::Y)
                        && track.keys.iter().any(|key| key.value != 0.0)
                })
            {
                return Err(format!(
                    "IK {}: animation {} varies the child setup offset",
                    ik.name, animation.name
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn write(doc: &Document) -> Value {
    let bone = |index: usize| &doc.bones[index].name;
    Value::Array(
        doc.constraints
            .iter()
            .enumerate()
            .map(|(index, constraint)| (index, match constraint {
                Constraint::Ik(ik) => json!({
                    "type":"ik", "name":ik.name,
                    "bones":ik.bones.iter().map(|&index| bone(index)).collect::<Vec<_>>(),
                    "target":bone(ik.target),
                    "mix":ik.setup.mix, "bendPositive":ik.setup.bend_positive
                }),
                Constraint::Transform(transform) => {
                    let mut value = json!({
                        "type":"transform", "name":transform.name,
                        "bones":transform.bones.iter().map(|&index| bone(index)).collect::<Vec<_>>(),
                        "source":bone(transform.source),
                        "localSource":transform.local_source, "localTarget":transform.local_target,
                        "additive":transform.additive, "clamp":transform.clamp,
                    });
                    for property in TransformProperty::ALL {
                        value[property.offset_name()] = json!(transform.offsets.get(property));
                        value[property.mix_name()] = json!(transform.mix.get(property));
                    }
                    let mut properties = json!({});
                    for mapping in &transform.mappings {
                        let mut to = json!({});
                        for target in &mapping.to {
                            to[target.property.spine_name()] = json!({
                                "offset":target.offset, "max":target.max, "scale":target.scale
                            });
                        }
                        properties[mapping.from.spine_name()] =
                            json!({"offset":mapping.offset, "to":to});
                    }
                    value["properties"] = properties;
                    value
                }
                Constraint::Physics(physics) => {
                    // Every value is explicit, so consumers never depend on defaults.
                    let mut value = json!({
                        "type":"physics", "name":physics.name, "bone":bone(physics.bone),
                        "x":physics.influence.x, "y":physics.influence.y,
                        "rotate":physics.influence.rotate, "scaleX":physics.influence.scale_x,
                        "shearX":physics.influence.shear_x,
                        "scaleY":match physics.scale_y_mode {
                            ScaleYMode::None => "none",
                            ScaleYMode::Uniform => "uniform",
                            ScaleYMode::Volume => "volume",
                        },
                        "limit":physics.limit, "fps":physics.fps,
                    });
                    for property in PhysicsProperty::ALL {
                        value[property.timeline()] = json!(property.read(physics.setup));
                        value[format!("{}Global", property.timeline())] =
                            json!(property.global(physics.global));
                    }
                    value
                }
            }))
            .map(|(index, mut value)| {
                if doc.skin_scope.required_constraints.contains(&index) {
                    value["skin"] = json!(true);
                }
                value
            })
            .collect(),
    )
}

pub(super) fn write_physics_timelines(
    doc: &Document,
    animation: &peachy_document::Animation,
) -> Result<Value, String> {
    let name = |constraint: Option<usize>| {
        constraint.map_or(String::new(), |index| {
            doc.constraints[index].name().to_owned()
        })
    };
    let mut output = json!({});
    for track in animation
        .physics
        .iter()
        .filter(|track| !track.keys.is_empty())
    {
        let key = name(track.constraint);
        if output.get(&key).is_none() {
            output[&key] = json!({});
        }
        output[&key][track.property.timeline()] = json!(super::write_keys(&track.keys)?);
    }
    for reset in animation
        .physics_resets
        .iter()
        .filter(|reset| !reset.times.is_empty())
    {
        let key = name(reset.constraint);
        if output.get(&key).is_none() {
            output[&key] = json!({});
        }
        output[&key]["reset"] = Value::Array(
            reset
                .times
                .iter()
                .map(|time| json!({"time":time}))
                .collect(),
        );
    }
    Ok(output)
}
