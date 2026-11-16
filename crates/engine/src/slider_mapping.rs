//! Raw Slider mapping endpoints. Reading editor range metadata never solves a
//! pose, changes the manual Time baseline, or repairs imported data.
use peachy_document::{Constraint, Document, SliderConstraint};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SliderMappingRangeField {
    SourceMin,
    SourceMax,
    TargetMin,
    TargetMax,
}

impl<'de> Deserialize<'de> for SliderMappingRangeField {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match String::deserialize(deserializer)?.as_str() {
            "sourceMin" => Ok(Self::SourceMin),
            "sourceMax" => Ok(Self::SourceMax),
            "targetMin" => Ok(Self::TargetMin),
            "targetMax" => Ok(Self::TargetMax),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["sourceMin", "sourceMax", "targetMin", "targetMax"],
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SliderMappingRange {
    pub source_min: f64,
    /// Undefined when the stored scale is zero.
    pub source_max: Option<f64>,
    /// Referenced animation seconds, independent of the document FPS.
    pub target_min: f64,
    pub target_max: f64,
}

impl SliderMappingRange {
    pub fn value(self, field: SliderMappingRangeField) -> Option<f64> {
        match field {
            SliderMappingRangeField::SourceMin => Some(self.source_min),
            SliderMappingRangeField::SourceMax => self.source_max,
            SliderMappingRangeField::TargetMin => Some(self.target_min),
            SliderMappingRangeField::TargetMax => Some(self.target_max),
        }
    }
}

fn range_at(slider: &SliderConstraint) -> Result<SliderMappingRange, String> {
    if slider.bone.is_none() || slider.property.is_none() {
        return Err("Slider mapping ranges require a source bone and property".into());
    }
    if ![
        slider.from,
        slider.to,
        slider.max,
        slider.scale,
        slider.setup.time,
        slider.setup.mix,
    ]
    .into_iter()
    .all(f32::is_finite)
    {
        return Err("Slider mapping settings must be finite".into());
    }
    let source_min = f64::from(slider.from);
    let target_min = f64::from(slider.to);
    let target_max = f64::from(slider.max);
    let source_max = (slider.scale != 0.)
        .then(|| source_min + (target_max - target_min) / f64::from(slider.scale));
    if source_max.is_some_and(|value| !value.is_finite()) {
        return Err("Slider mapping endpoints must be finite".into());
    }
    Ok(SliderMappingRange {
        source_min,
        source_max,
        target_min,
        target_max,
    })
}

/// Read exact raw-derived endpoints for a bone-driven Slider. A zero scale
/// does not encode a recoverable source maximum, so it is returned as `None`.
pub fn slider_mapping_range(
    doc: &Document,
    constraint: usize,
) -> Result<SliderMappingRange, String> {
    let slider = doc
        .constraints
        .get(constraint)
        .and_then(Constraint::as_slider)
        .ok_or("Unknown Slider constraint")?;
    slider.validate(&doc.bones)?;
    if slider.animation >= doc.animations.len() {
        return Err("Slider mapping references an unknown animation".into());
    }
    range_at(slider)
}

fn encoded_endpoint(value: f64) -> Result<f32, String> {
    let encoded = value as f32;
    if !value.is_finite() || !encoded.is_finite() {
        return Err("Slider mapping endpoints must be within the finite f32 range".into());
    }
    Ok(encoded)
}

/// Apply one measured endpoint edit to a candidate. Readback remains raw f64;
/// actual endpoint edits compare finite f32 endpoints, including an indirectly
/// encoded partner such as 100.0000009 that represents the native field 100.
/// No-op reads preserve raw fields, signed zero, and the hidden manual Time.
pub fn edited_slider_mapping_range(
    slider: &SliderConstraint,
    reference_duration: f32,
    field: SliderMappingRangeField,
    value: f64,
) -> Result<SliderConstraint, String> {
    let range = range_at(slider)?;
    if !value.is_finite() {
        return Err("Slider mapping range edits must be finite".into());
    }
    if range.value(field) == Some(value) {
        return Ok(slider.clone());
    }
    let encoded = encoded_endpoint(value)?;
    if range.value(field).is_some_and(|current| {
        let current = current as f32;
        current.is_finite() && current == encoded
    }) {
        return Ok(slider.clone());
    }
    if !reference_duration.is_finite() || reference_duration < 0. {
        return Err("Slider reference duration must be finite and nonnegative".into());
    }
    if range.source_max.is_none() && field != SliderMappingRangeField::SourceMax {
        return Err(
            "Slider source range is undefined at zero scale; set SourceMax with a nonzero target span first"
                .into(),
        );
    }
    let mut source_min = encoded_endpoint(range.source_min)?;
    let mut source_max = match field {
        SliderMappingRangeField::SourceMax => encoded,
        _ => encoded_endpoint(range.source_max.ok_or("Undefined Slider source range")?)?,
    };
    let mut target_min = encoded_endpoint(range.target_min)?;
    let mut target_max = encoded_endpoint(range.target_max)?;
    match field {
        SliderMappingRangeField::SourceMin => source_min = encoded,
        SliderMappingRangeField::SourceMax => {}
        SliderMappingRangeField::TargetMin => target_min = encoded,
        SliderMappingRangeField::TargetMax => target_max = encoded,
    }
    match field {
        SliderMappingRangeField::SourceMin | SliderMappingRangeField::SourceMax => {
            if source_min == source_max {
                source_max = encoded_endpoint(f64::from(source_min) + 100.)?;
            }
        }
        SliderMappingRangeField::TargetMin | SliderMappingRangeField::TargetMax => {
            if target_min == target_max {
                if reference_duration == 0. {
                    return Err(
                        "Equal Slider target endpoints require a positive reference duration"
                            .into(),
                    );
                }
                target_max =
                    encoded_endpoint(f64::from(target_min) + f64::from(reference_duration))?;
            }
        }
    }
    let source_span = f64::from(source_max) - f64::from(source_min);
    let target_span = f64::from(target_max) - f64::from(target_min);
    if source_span == 0. || target_span == 0. {
        return Err("Slider mapping range remains undefined at a zero span".into());
    }
    let scale = (target_span / source_span) as f32;
    if !scale.is_finite() || scale == 0. {
        return Err("Slider mapping scale is outside the finite nonzero f32 range".into());
    }
    let decoded_max = f64::from(source_min) + target_span / f64::from(scale);
    if !decoded_max.is_finite() || decoded_max == f64::from(source_min) {
        return Err("Slider mapping range collapses at native storage precision".into());
    }
    let mut candidate = slider.clone();
    candidate.from = source_min;
    candidate.to = target_min;
    candidate.max = target_max;
    candidate.scale = scale;
    Ok(candidate)
}

#[cfg(test)]
#[path = "slider_mapping_tests.rs"]
mod tests;
