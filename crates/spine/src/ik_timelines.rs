//! Grouped IK animation interchange. No sampling or pose solving lives here.
use super::unknown_fields;
use peachy_document::{Animation, Constraint, Document, IkCurve, IkKeyframe, IkTrack};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) fn indices(constraints: &[Constraint]) -> BTreeMap<String, usize> {
    constraints
        .iter()
        .enumerate()
        .map(|(i, Constraint::Ik(ik))| (ik.name.clone(), i))
        .collect()
}

fn finite(value: &Value, context: &str) -> Result<f32, String> {
    let number = value
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{context} must be a finite number"))?;
    let native = number as f32;
    if !native.is_finite() {
        return Err(format!("{context} exceeds the finite native range"));
    }
    Ok(native)
}

fn number(frame: &Value, field: &str, default: f32) -> Result<f32, String> {
    frame
        .get(field)
        .map_or(Ok(default), |v| finite(v, &format!("IK {field}")))
}

fn time_ratio(start: f32, end: f32, control: f32) -> Result<f64, String> {
    // All absolute coordinates share the observed native-f32 space first.
    if end <= start || control < start || control > end {
        return Err(
            "IK Bezier time controls must lie within a strictly increasing key segment".into(),
        );
    }
    Ok((f64::from(control) - f64::from(start)) / (f64::from(end) - f64::from(start)))
}

fn curve(frame: &Value, next: Option<&Value>, time: f32) -> Result<IkCurve, String> {
    match frame.get("curve") {
        None => Ok(IkCurve::Linear),
        Some(Value::String(v)) if v == "stepped" => Ok(IkCurve::Stepped),
        Some(Value::Array(handles)) => {
            if handles.len() != 8 {
                return Err(
                    "IK Bezier requires eight finite coordinates: Mix four then softness four"
                        .into(),
                );
            }
            // Unsupported softness must be checked in source precision before
            // a tiny nonzero value could round down to apparently neutral zero.
            if handles[5].as_f64() != Some(0.0) || handles[7].as_f64() != Some(0.0) {
                return Err(
                    "IK Bezier nonzero or malformed softness controls are unsupported".into(),
                );
            }
            let h: Vec<f32> = handles
                .iter()
                .map(|v| finite(v, "IK Bezier coordinate"))
                .collect::<Result<_, _>>()?;
            let next = next.ok_or("IK Bezier on last key is unsupported without a next segment")?;
            let end = number(next, "time", 0.0)?;
            let normalized_time = [time_ratio(time, end, h[0])?, time_ratio(time, end, h[2])?];
            // Even a neutral, unstored second channel gets full time validation.
            time_ratio(time, end, h[4])?;
            time_ratio(time, end, h[6])?;
            Ok(IkCurve::Bezier {
                time: normalized_time,
                mix: [h[1], h[3]],
            })
        }
        _ => Err("IK curve must be stepped or an eight-coordinate Bezier".into()),
    }
}

pub(super) fn read(
    animation: &Value,
    names: &BTreeMap<String, usize>,
) -> Result<Vec<IkTrack>, String> {
    if animation.get("constraints").is_some() {
        return Err("Guessed animation constraints timelines are unsupported; use ik".into());
    }
    let Some(ik) = animation.get("ik") else {
        return Ok(vec![]);
    };
    let groups = ik.as_object().ok_or("Animation ik must be an object")?;
    groups
        .iter()
        .map(|(name, value)| {
            let constraint = *names
                .get(name)
                .ok_or_else(|| format!("Unknown animated IK constraint {name}"))?;
            let frames = value.as_array().ok_or("IK timeline must be an array")?;
            let mut keys = Vec::with_capacity(frames.len());
            for (i, frame) in frames.iter().enumerate() {
                if !frame.is_object() {
                    return Err("IK frame must be an object".into());
                }
                let mut issues = vec![];
                unknown_fields(
                    frame,
                    &[
                        "time",
                        "mix",
                        "bendPositive",
                        "softness",
                        "compress",
                        "stretch",
                        "curve",
                    ],
                    "IK frame",
                    &mut issues,
                );
                if !issues.is_empty() {
                    return Err(issues.join("; "));
                }
                if frame
                    .get("softness")
                    .is_some_and(|v| v.as_f64() != Some(0.0))
                {
                    return Err("IK nonzero or malformed softness is unsupported".into());
                }
                for field in ["compress", "stretch"] {
                    if frame.get(field).is_some_and(|v| v.as_bool() != Some(false)) {
                        return Err(format!("IK {field} must be omitted or false"));
                    }
                }
                let time = number(frame, "time", 0.0)?;
                let key = IkKeyframe {
                    time,
                    mix: number(frame, "mix", 1.0)?,
                    bend_positive: frame.get("bendPositive").map_or(Ok(true), |v| {
                        v.as_bool().ok_or("IK bendPositive must be boolean")
                    })?,
                    curve: curve(frame, frames.get(i + 1), time)?,
                };
                key.validate()?;
                if keys
                    .last()
                    .is_some_and(|last: &IkKeyframe| last.time >= key.time)
                {
                    return Err(
                        "IK key times must remain strictly increasing after native-f32 conversion"
                            .into(),
                    );
                }
                keys.push(key);
            }
            Ok(IkTrack { constraint, keys })
        })
        .collect()
}

fn absolute_time(start: f32, end: f32, ratio: f64) -> Result<f32, String> {
    let value = f64::from(start) + (f64::from(end) - f64::from(start)) * ratio;
    let native = value as f32;
    if !value.is_finite() || !native.is_finite() {
        return Err("IK Bezier export time exceeds the finite native range".into());
    }
    Ok(native)
}

pub(super) fn write(doc: &Document, animation: &Animation) -> Result<Value, String> {
    let mut groups = json!({});
    for track in &animation.ik_tracks {
        if track.keys.is_empty() {
            continue;
        }
        let Constraint::Ik(ik) = &doc.constraints[track.constraint];
        let mut frames = vec![];
        for (i, key) in track.keys.iter().enumerate() {
            let mut frame = json!({"time":key.time,"mix":key.mix,"bendPositive":key.bend_positive});
            match key.curve {
                IkCurve::Stepped => frame["curve"] = json!("stepped"),
                IkCurve::Bezier { time, mix } if i + 1 < track.keys.len() => {
                    let next = track.keys[i + 1];
                    let x1 = absolute_time(key.time, next.time, time[0])?;
                    let x2 = absolute_time(key.time, next.time, time[1])?;
                    frame["curve"] = json!([x1, mix[0], x2, mix[1], x1, 0.0, x2, 0.0]);
                }
                _ => {}
            }
            frames.push(frame);
        }
        groups[&ik.name] = json!(frames);
    }
    Ok(groups)
}

pub(super) fn potentially_active(setup: f32, track: Option<&IkTrack>) -> bool {
    let Some(track) = track.filter(|t| !t.keys.is_empty()) else {
        return setup != 0.0;
    };
    (track.keys[0].time > 0.0 && setup != 0.0)
        || track.keys.iter().any(|key| key.mix != 0.0)
        || track.keys.iter().take(track.keys.len() - 1).any(|key| {
            matches!(key.curve, IkCurve::Bezier { mix, .. } if mix.into_iter().any(|v| v != 0.0))
        })
}
