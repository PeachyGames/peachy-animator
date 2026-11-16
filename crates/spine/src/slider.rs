//! Independent Slider constraint/timeline interchange from original 4.3.26
//! editor exports and the public API. No pose evaluation occurs in this layer.
use super::{required_string, unknown_fields};
use peachy_document::{
    Animation, Constraint, Document, SliderConstraint, SliderProperty, SliderSetup, SliderTrack,
    TransformProperty,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) fn animation_indices(root: &Value) -> BTreeMap<String, usize> {
    root["animations"]
        .as_object()
        .into_iter()
        .flat_map(|animations| animations.keys().enumerate())
        .map(|(index, name)| (name.clone(), index))
        .collect()
}

pub(super) fn indices(constraints: &[Constraint]) -> BTreeMap<String, usize> {
    constraints
        .iter()
        .enumerate()
        .filter_map(|(index, constraint)| Some((constraint.as_slider()?.name.clone(), index)))
        .collect()
}

fn finite(value: &Value, context: &str) -> Result<f32, String> {
    value
        .as_f64()
        .filter(|number| number.is_finite() && (*number as f32).is_finite())
        .map(|number| number as f32)
        .ok_or_else(|| format!("{context} must be a finite native number"))
}

pub(super) fn read_constraint(
    entry: &Value,
    bones: &BTreeMap<String, usize>,
    animations: &BTreeMap<String, usize>,
) -> Result<SliderConstraint, String> {
    let name = required_string(entry, "name")?;
    let context = format!("Slider {name}");
    let mut issues = vec![];
    unknown_fields(
        entry,
        &[
            "type",
            "name",
            "animation",
            "bone",
            "property",
            "local",
            "loop",
            "additive",
            "from",
            "to",
            "scale",
            "max",
            "time",
            "mix",
            "order",
            "skin",
        ],
        &context,
        &mut issues,
    );
    if !issues.is_empty() {
        return Err(issues.join("; "));
    }
    super::constraints::check_order(entry, &context)?;
    let animation = entry["animation"]
        .as_str()
        .ok_or_else(|| format!("{context}: animation must be a name"))?;
    let animation = *animations
        .get(animation)
        .ok_or_else(|| format!("{context}: unknown referenced animation {animation}"))?;
    let bone = entry
        .get("bone")
        .map(|value| {
            let bone = value
                .as_str()
                .ok_or_else(|| format!("{context}: bone must be a name"))?;
            bones
                .get(bone)
                .copied()
                .ok_or_else(|| format!("{context}: unknown source bone {bone}"))
        })
        .transpose()?;
    let property = entry
        .get("property")
        .map(|value| {
            value
                .as_str()
                .and_then(TransformProperty::from_spine_name)
                .ok_or_else(|| format!("{context}: unknown source property"))
        })
        .transpose()?;
    if bone.is_some() && property.is_none() {
        return Err(format!("{context}: a source bone needs a property"));
    }
    let number = |field: &str, default: f32| {
        entry.get(field).map_or(Ok(default), |value| {
            finite(value, &format!("{context}: {field}"))
        })
    };
    let flag = |field: &str| {
        entry.get(field).map_or(Ok(false), |value| {
            value
                .as_bool()
                .ok_or_else(|| format!("{context}: {field} must be boolean"))
        })
    };
    // Skin membership is represented by Document.skin_scope, not this payload.
    flag("skin")?;
    Ok(SliderConstraint {
        name: name.into(),
        animation,
        bone,
        property,
        local: flag("local")?,
        looped: flag("loop")?,
        additive: flag("additive")?,
        from: number("from", 0.)?,
        to: number("to", 0.)?,
        scale: number("scale", 0.)?,
        max: number("max", 0.)?,
        setup: SliderSetup {
            time: number("time", 0.)?,
            mix: number("mix", 1.)?,
        },
    })
}

pub(super) fn write_constraint(slider: &SliderConstraint, doc: &Document) -> Value {
    // Preserve complete stored settings even when bone is absent. The public
    // consumer ignores inactive range fields; interchange never normalizes them.
    let mut entry = json!({
        "type":"slider", "name":slider.name,
        "animation":doc.animations[slider.animation].name,
        "time":slider.setup.time, "mix":slider.setup.mix,
        "from":slider.from, "to":slider.to, "scale":slider.scale, "max":slider.max,
        "local":slider.local, "loop":slider.looped, "additive":slider.additive,
    });
    if let Some(bone) = slider.bone {
        entry["bone"] = json!(doc.bones[bone].name);
    }
    if let Some(property) = slider.property {
        entry["property"] = json!(property.spine_name());
    }
    entry
}

fn property_name(property: SliderProperty) -> &'static str {
    match property {
        SliderProperty::Time => "time",
        SliderProperty::Mix => "mix",
    }
}

pub(super) fn read_timelines(
    animation_name: &str,
    animation: &Value,
    names: &BTreeMap<String, usize>,
) -> Result<Vec<SliderTrack>, String> {
    let Some(groups) = animation.get("slider") else {
        return Ok(vec![]);
    };
    let groups = groups
        .as_object()
        .ok_or_else(|| format!("Animation {animation_name}: slider must be an object"))?;
    let mut tracks = vec![];
    for (name, timelines) in groups {
        let context = format!("Animation {animation_name}/Slider {name}");
        let constraint = *names
            .get(name)
            .ok_or_else(|| format!("{context}: unknown Slider constraint"))?;
        let timelines = timelines
            .as_object()
            .ok_or_else(|| format!("{context}: timelines must be an object"))?;
        for (timeline, frames) in timelines {
            let property = match timeline.as_str() {
                "time" => SliderProperty::Time,
                "mix" => SliderProperty::Mix,
                _ => return Err(format!("{context}: unknown timeline {timeline}")),
            };
            let frames = frames
                .as_array()
                .ok_or_else(|| format!("{context}/{timeline}: keys must be an array"))?;
            for frame in frames {
                if !frame.is_object() {
                    return Err(format!("{context}/{timeline}: key must be an object"));
                }
                let mut issues = vec![];
                unknown_fields(frame, &["time", "value", "curve"], &context, &mut issues);
                if !issues.is_empty() {
                    return Err(issues.join("; "));
                }
                for field in ["time", "value"] {
                    if let Some(value) = frame.get(field) {
                        finite(value, &format!("{context}/{timeline}: {field}"))?;
                    }
                }
                if let Some(curve) = frame.get("curve")
                    && curve.as_str() != Some("stepped")
                    && !curve.as_array().is_some_and(|handles| {
                        handles.len() == 4
                            && handles.iter().all(|handle| {
                                finite(handle, &format!("{context}: Bezier coordinate")).is_ok()
                            })
                    })
                {
                    return Err(format!(
                        "{context}/{timeline}: curve must be stepped or four finite Bezier coordinates"
                    ));
                }
            }
            tracks.push(SliderTrack {
                constraint,
                property,
                // Installed 4.3.26 exports omit value at 1 for BOTH Time and
                // Mix. Time is absolute referenced seconds, not a zero-offset
                // bone scalar. Explicit zero must therefore remain explicit.
                keys: super::read_keys(frames, "value", 0, 1.)?,
            });
        }
    }
    Ok(tracks)
}

pub(super) fn write_timelines(doc: &Document, animation: &Animation) -> Result<Value, String> {
    let mut groups = json!({});
    for track in &animation.slider_tracks {
        if track.keys.is_empty() {
            continue;
        }
        let slider = doc.constraints[track.constraint]
            .as_slider()
            .ok_or("Slider tracks must reference Slider constraints")?;
        if groups.get(&slider.name).is_none() {
            groups[&slider.name] = json!({});
        }
        groups[&slider.name][property_name(track.property)] =
            json!(super::write_keys(&track.keys)?);
    }
    Ok(groups)
}
