//! Slider authored state, pending edits and exact keys. Constraint application
//! and bone-mode transitions remain separate from these authoring operations.
use crate::{AuthoredPose, Command};
use peachy_document::{Animation, Constraint, Document, Keyframe, SliderProperty, SliderTrack};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SliderPose {
    /// Referenced seconds, kept separate from the host animation cursor.
    pub time: f64,
    pub mix: f64,
}

impl SliderPose {
    /// Non-Slider placeholder; arrays retain unified constraint indices.
    pub(crate) const INACTIVE: Self = Self { time: 0., mix: 0. };

    pub fn value(self, property: SliderProperty) -> f64 {
        match property {
            SliderProperty::Time => self.time,
            SliderProperty::Mix => self.mix,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum SliderPoseEdit {
    Time { value: f32 },
    Mix { value: f32 },
}

impl SliderPoseEdit {
    pub fn property(self) -> SliderProperty {
        match self {
            Self::Time { .. } => SliderProperty::Time,
            Self::Mix { .. } => SliderProperty::Mix,
        }
    }
    pub fn value(self) -> f32 {
        match self {
            Self::Time { value } | Self::Mix { value } => value,
        }
    }
}

pub(crate) fn validate_edits(edits: &[SliderPoseEdit]) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for &edit in edits {
        if !seen.insert(edit.property()) {
            return Err("Duplicate Slider pose property".into());
        }
        let (SliderPoseEdit::Time { value } | SliderPoseEdit::Mix { value }) = edit;
        if !value.is_finite() {
            return Err("Slider pose settings must be finite".into());
        }
    }
    Ok(())
}

pub(crate) fn sample_poses(
    document: &Document,
    animation: Option<usize>,
    time: f32,
    activity: &peachy_document::Activity,
) -> Result<Vec<SliderPose>, String> {
    let clip = animation
        .map(|index| document.animations.get(index).ok_or("Unknown animation"))
        .transpose()?;
    document
        .constraints
        .iter()
        .enumerate()
        .map(|(index, constraint)| {
            let Some(slider) = constraint.as_slider() else {
                return Ok(SliderPose::INACTIVE);
            };
            let active = *activity
                .constraints
                .get(index)
                .ok_or("Missing Slider activity")?;
            let value = |property: SliderProperty| {
                clip.and_then(|clip| {
                    // Driven-Time 02 and Mix03 isolate host key application
                    // from explicit pending overrides: inactive tracks skip.
                    if !active {
                        return None;
                    }
                    clip.slider_tracks
                        .iter()
                        .find(|track| track.constraint == index && track.property == property)
                })
                .and_then(|track| crate::ik::sample_absolute(&track.keys, time))
                .map_or_else(
                    || {
                        // The public consumer ignores the inactive manual
                        // setup Time field when a source bone is present.
                        // Preserve that raw field in the document; explicit
                        // host keys and pending pose overrides stay separate.
                        if property == SliderProperty::Time && slider.bone.is_some() {
                            0.
                        } else {
                            f64::from(property.read(slider.setup))
                        }
                    },
                    f64::from,
                )
            };
            let pose = SliderPose {
                time: value(SliderProperty::Time),
                mix: value(SliderProperty::Mix),
            };
            if !pose.time.is_finite() || !pose.mix.is_finite() {
                return Err(format!(
                    "Slider {}: sampled settings must be finite",
                    slider.name
                ));
            }
            Ok(pose)
        })
        .collect()
}

pub(crate) fn validate_key(key: Keyframe) -> Result<(), String> {
    if !key.time.is_finite() || key.time < 0. || !key.value.is_finite() {
        return Err("Slider keys need finite values and nonnegative finite host times".into());
    }
    if !key.curve.handles_valid() {
        return Err("Slider Bezier handles must be finite".into());
    }
    Ok(())
}

pub(crate) fn insert_key(
    clip: &mut Animation,
    constraint: usize,
    property: SliderProperty,
    key: Keyframe,
) {
    let index = clip
        .slider_tracks
        .iter()
        .position(|track| track.constraint == constraint && track.property == property)
        .unwrap_or_else(|| {
            clip.slider_tracks.push(SliderTrack {
                constraint,
                property,
                keys: vec![],
            });
            clip.slider_tracks.len() - 1
        });
    let keys = &mut clip.slider_tracks[index].keys;
    if let Some(existing) = keys.iter_mut().find(|existing| existing.time == key.time) {
        *existing = key;
    } else {
        keys.push(key);
    }
    keys.sort_by(|left, right| left.time.total_cmp(&right.time));
}

/// Semantic insertion is distinct from the exact raw SetSliderKey. Its
/// subdivision reuses the independently calibrated scalar geometry, now also
/// observed on native Slider Time keys in 4.3.26.
pub(crate) fn insert_pose(
    doc: &mut Document,
    animation: usize,
    constraint: usize,
    property: SliderProperty,
    time: f32,
    value: f32,
) -> Result<(), String> {
    let slider = doc
        .constraints
        .get(constraint)
        .and_then(Constraint::as_slider)
        .ok_or("Unknown Slider constraint")?;
    if property == SliderProperty::Time && slider.bone.is_some() {
        return Err("Slider Frame is driven by its source bone".into());
    }
    validate_key(Keyframe {
        time,
        value,
        curve: peachy_document::Curve::Linear,
    })?;
    let clip = doc.animations.get(animation).ok_or("Unknown animation")?;
    let keys = clip
        .slider_tracks
        .iter()
        .find(|track| track.constraint == constraint && track.property == property)
        .map_or(&[][..], |track| track.keys.as_slice());
    let mut incoming = Keyframe {
        time,
        value,
        curve: peachy_document::Curve::Linear,
    };
    let mut previous = None;
    if let Some(existing) = keys.iter().find(|key| key.time == time) {
        incoming.curve = existing.curve;
    } else {
        let after = keys.partition_point(|key| key.time < time);
        if let Some((left, right)) = after
            .checked_sub(1)
            .and_then(|before| keys.get(before).zip(keys.get(after)))
        {
            let (before, after) = crate::color_curve_split::split_slider(
                left.curve,
                [left.time, time, right.time],
                [left.value, value, right.value],
                value,
                doc.fps,
            )?;
            let mut changed = *left;
            changed.curve = before;
            previous = Some(changed);
            incoming.curve = after;
        }
    }
    // Compute both candidates before mutating even the command's private clone.
    let clip = doc
        .animations
        .get_mut(animation)
        .ok_or("Unknown animation")?;
    if let Some(previous) = previous {
        insert_key(clip, constraint, property, previous);
    }
    insert_key(clip, constraint, property, incoming);
    Ok(())
}

pub(crate) fn key_commands(
    document: &Document,
    authored: &AuthoredPose,
    animation: usize,
    time: f32,
    constraint: usize,
    properties: &[SliderProperty],
) -> Result<Vec<Command>, String> {
    if document
        .constraints
        .get(constraint)
        .and_then(Constraint::as_slider)
        .is_none()
    {
        return Err("Unknown Slider constraint".into());
    }
    let value = *authored
        .slider_poses()
        .get(constraint)
        .ok_or("Unknown Slider constraint")?;
    let clip = document
        .animations
        .get(animation)
        .ok_or("Unknown animation")?;
    let snapped = crate::commands::snapped_time(document.fps, time)?;
    let mut seen = BTreeSet::new();
    let mut commands = vec![];
    for &property in properties {
        if !seen.insert(property) {
            return Err("Duplicate Slider key property".into());
        }
        let keys = clip
            .slider_tracks
            .iter()
            .find(|track| track.constraint == constraint && track.property == property)
            .map_or(&[][..], |track| track.keys.as_slice());
        let existing = match crate::keyframe_at(keys, time)? {
            Some(key) => Some(key),
            None => crate::keyframe_at(keys, snapped)?,
        };
        let encoded = value.value(property) as f32;
        if !value.value(property).is_finite() || !encoded.is_finite() {
            return Err("Sampled Slider value cannot be represented by a finite stored key".into());
        }
        commands.push(Command::InsertSliderKey {
            animation,
            constraint,
            property,
            time: existing.map_or(snapped, |key| key.time),
            value: encoded,
        });
    }
    Ok(commands)
}

pub(crate) fn check_key_continuity(
    before: &AuthoredPose,
    after: &AuthoredPose,
    targets: &std::collections::BTreeMap<usize, Vec<SliderProperty>>,
) -> Result<(), String> {
    for (&constraint, properties) in targets {
        let before = before
            .slider_poses()
            .get(constraint)
            .ok_or("Unknown Slider constraint")?;
        let after = after
            .slider_poses()
            .get(constraint)
            .ok_or("Unknown Slider constraint")?;
        if properties
            .iter()
            .any(|&property| before.value(property) != after.value(property))
        {
            return Err("Keying Slider settings would lose the current authored pose; keep it pending or use a representable key time".into());
        }
    }
    Ok(())
}

pub fn slider_key_time(
    document: &Document,
    animation: usize,
    time: f32,
    constraint: usize,
    property: SliderProperty,
) -> Result<Option<f32>, String> {
    document.validate()?;
    if !matches!(
        document.constraints.get(constraint),
        Some(Constraint::Slider(_))
    ) {
        return Err("Unknown Slider constraint".into());
    }
    if !time.is_finite() || time < 0. {
        return Err("Slider host time must be finite and nonnegative".into());
    }
    let clip = document
        .animations
        .get(animation)
        .ok_or("Unknown animation")?;
    clip.slider_tracks
        .iter()
        .find(|track| track.constraint == constraint && track.property == property)
        .map(|track| crate::keyframe_at(&track.keys, time).map(|key| key.map(|key| key.time)))
        .transpose()
        .map(Option::flatten)
}
