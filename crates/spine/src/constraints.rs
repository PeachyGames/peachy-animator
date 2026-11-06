//! Independently written, bounded Spine 4.3 static IK interchange.
//! Source-feature checks only: dynamic solving belongs to the engine.
use super::{array, ik_timelines, required_string, unknown_fields};
use peachy_document::{Constraint, Document, IkConstraint, IkSetup, Property};
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
    entries
        .iter()
        .map(|entry| {
            if !entry.is_object() {
                return Err("Constraint must be an object".into());
            }
            if entry["type"].as_str() != Some("ik") {
                return Err("Only IK constraints are supported".into());
            }
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
                "IK constraint",
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
                        .ok_or_else(|| {
                            format!("{}: mix must be a finite native number", context())
                        })?;
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
            for field in ["compress", "stretch", "skin"] {
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
            // Obsolete editor metadata is recognized but never cast, stored, sorted
            // or exported. Array order is the observed 4.3 evaluation order.
            if entry.get("order").is_some_and(|v| {
                v.as_f64()
                    .is_none_or(|n| !n.is_finite() || n < 0.0 || n.fract() != 0.0)
            }) {
                return Err(format!(
                    "{}: order must be a finite nonnegative integer",
                    context()
                ));
            }
            let resolve = |value: &Value| -> Result<usize, String> {
                let bone = value
                    .as_str()
                    .ok_or_else(|| format!("{}: bone references must be names", context()))?;
                indices
                    .get(bone)
                    .copied()
                    .ok_or_else(|| format!("{}: unknown bone {bone}", context()))
            };
            let bones = entry["bones"]
                .as_array()
                .ok_or_else(|| format!("{}: bones must be an array", context()))?
                .iter()
                .map(resolve)
                .collect::<Result<Vec<_>, _>>()?;
            let target = resolve(&entry["target"])?;
            Ok(Constraint::Ik(IkConstraint {
                name: name.into(),
                bones,
                target,
                setup: IkSetup { mix, bend_positive },
            }))
        })
        .collect()
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
    let setup: Vec<_> = doc
        .constraints
        .iter()
        .map(|Constraint::Ik(ik)| ik)
        .filter(|ik| ik.setup.mix != 0.0)
        .collect();
    validate_context(doc, &setup, None)?;
    for animation in &doc.animations {
        let active: Vec<_> = doc
            .constraints
            .iter()
            .enumerate()
            .filter_map(|(index, Constraint::Ik(ik))| {
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
        if doc.bones[first].parent.is_none() {
            return Err(format!(
                "IK {}: root-constrained chains are unsupported",
                ik.name
            ));
        }
        for other in active.iter().skip(i + 1) {
            if inside(doc, first, other.bones[0])
                || inside(doc, other.bones[0], first)
                || inside(doc, ik.target, other.bones[0])
                || inside(doc, other.target, first)
            {
                return Err(format!(
                    "IK {} / {}: dependent constraint subtrees are unsupported",
                    ik.name, other.name
                ));
            }
        }
        let mut chain = Some(*ik.bones.last().unwrap());
        while let Some(index) = chain {
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
            if child.setup.x <= 0.0 || child.setup.y != 0.0 || child.length <= 0.0 {
                return Err(format!(
                    "IK {}: child needs positive setup X/length and zero Y",
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
    Value::Array(doc.constraints.iter().map(|Constraint::Ik(ik)| json!({
        "type":"ik", "name":ik.name,
        "bones":ik.bones.iter().map(|&index| &doc.bones[index].name).collect::<Vec<_>>(),
        "target":doc.bones[ik.target].name,
        "mix":ik.setup.mix, "bendPositive":ik.setup.bend_positive
    })).collect())
}
