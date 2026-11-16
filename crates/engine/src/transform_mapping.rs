//! Semantic range edits for the measured world X-to-Rotate mapping. The native
//! format stores offsets/max/scale, not four independent range endpoints.
use peachy_document::{Constraint, Document, TransformConstraint, TransformProperty};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransformMappingRangeField {
    SourceMin,
    SourceMax,
    TargetMin,
    TargetMax,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformMappingRange {
    pub source_min: f64,
    pub source_max: Option<f64>,
    pub target_min: f64,
    pub target_max: f64,
}

impl TransformMappingRange {
    pub fn value(self, field: TransformMappingRangeField) -> Option<f64> {
        match field {
            TransformMappingRangeField::SourceMin => Some(self.source_min),
            TransformMappingRangeField::SourceMax => self.source_max,
            TransformMappingRangeField::TargetMin => Some(self.target_min),
            TransformMappingRangeField::TargetMax => Some(self.target_max),
        }
    }
}

fn mapping_index(
    constraint: &TransformConstraint,
    from: TransformProperty,
    to: TransformProperty,
) -> Result<usize, String> {
    if from != TransformProperty::X || to != TransformProperty::Rotate {
        return Err("Only X to Rotate mapping ranges are supported".into());
    }
    constraint.supported_subset()?;
    let mut sources = BTreeSet::new();
    for mapping in &constraint.mappings {
        if !sources.insert(mapping.from)
            || !mapping.offset.is_finite()
            || mapping.to.iter().any(|target| {
                ![target.offset, target.max, target.scale]
                    .into_iter()
                    .all(f32::is_finite)
            })
        {
            return Err("Transform mapping sources must be distinct and finite".into());
        }
    }
    constraint
        .mappings
        .iter()
        .position(|mapping| mapping.from == from && mapping.to[0].property == to)
        .ok_or("Unknown X to Rotate mapping".into())
}

fn range_at(constraint: &TransformConstraint, index: usize) -> TransformMappingRange {
    let mapping = &constraint.mappings[index];
    let target = mapping.to[0];
    let source_min = f64::from(mapping.offset);
    let target_min = f64::from(target.offset);
    let target_max = f64::from(target.max);
    TransformMappingRange {
        source_min,
        source_max: (target.scale != 0.)
            .then(|| source_min + (target_max - target_min) / f64::from(target.scale)),
        target_min,
        target_max,
    }
}

/// Read exact raw-derived endpoints without normalizing, rounding or solving a
/// pose. At zero scale the source maximum cannot be recovered from native data.
pub fn transform_mapping_range(
    doc: &Document,
    constraint: usize,
    from: TransformProperty,
    to: TransformProperty,
) -> Result<TransformMappingRange, String> {
    let value = doc
        .constraints
        .get(constraint)
        .and_then(Constraint::as_transform)
        .ok_or("Unknown transform constraint")?;
    value.validate(&doc.bones)?;
    Ok(range_at(value, mapping_index(value, from, to)?))
}

fn encoded_endpoint(value: f64) -> Result<f32, String> {
    let encoded = value as f32;
    if !value.is_finite() || !encoded.is_finite() {
        return Err("Transform mapping endpoints must be within the finite f32 range".into());
    }
    Ok(encoded)
}

/// Advance one accepted candidate, preserving the partner endpoint after a
/// measured equal-endpoint repair. UI previews and persistent commands use the
/// same pure transition; no command replay or UI range arithmetic is needed.
pub fn edited_transform_mapping_range(
    constraint: &TransformConstraint,
    from: TransformProperty,
    to: TransformProperty,
    field: TransformMappingRangeField,
    value: f64,
) -> Result<TransformConstraint, String> {
    let index = mapping_index(constraint, from, to)?;
    let mut range = range_at(constraint, index);
    if !value.is_finite() {
        return Err("Transform mapping range edits must be finite".into());
    }
    // In particular, viewing a rounded field must not normalize imported data
    // or repair a pre-existing degenerate range merely because it was queried.
    if range.value(field) == Some(value) {
        return Ok(constraint.clone());
    }
    if range.source_max.is_none() && field != TransformMappingRangeField::SourceMax {
        return Err(
            "Transform mapping source range is undefined at zero scale; set SourceMax with a nonzero target span first"
                .into(),
        );
    }
    match field {
        TransformMappingRangeField::SourceMin => range.source_min = value,
        TransformMappingRangeField::SourceMax => range.source_max = Some(value),
        TransformMappingRangeField::TargetMin => range.target_min = value,
        TransformMappingRangeField::TargetMax => range.target_max = value,
    }
    let mut source_max = range
        .source_max
        .ok_or("Undefined Transform mapping source range")?;
    match field {
        TransformMappingRangeField::SourceMin | TransformMappingRangeField::SourceMax => {
            if range.source_min == source_max {
                // Observed for this X-to-Rotate slice, regardless of which
                // source endpoint was edited. This is not an import default.
                source_max = range.source_min + 100.;
            }
        }
        TransformMappingRangeField::TargetMin | TransformMappingRangeField::TargetMax => {
            if range.target_min == range.target_max {
                range.target_max = range.target_min + 100.;
            }
        }
    }
    let source_min = encoded_endpoint(range.source_min)?;
    encoded_endpoint(source_max)?;
    let target_min = encoded_endpoint(range.target_min)?;
    let target_max = encoded_endpoint(range.target_max)?;
    let source_span = source_max - range.source_min;
    let target_span = range.target_max - range.target_min;
    if source_span == 0. || target_span == 0. {
        return Err("Transform mapping range remains undefined at a zero span".into());
    }
    let scale = (target_span / source_span) as f32;
    if !scale.is_finite() || scale == 0. {
        return Err("Transform mapping scale is outside the finite nonzero f32 range".into());
    }
    // A source maximum is encoded indirectly by scale. Reject edits that lose
    // the interval in storage rather than inventing a replacement on readback.
    let decoded_max =
        f64::from(source_min) + (f64::from(target_max) - f64::from(target_min)) / f64::from(scale);
    if target_min == target_max || decoded_max == f64::from(source_min) {
        return Err("Transform mapping range collapses at native storage precision".into());
    }
    let mut candidate = constraint.clone();
    let mapping = &mut candidate.mappings[index];
    mapping.offset = source_min;
    mapping.to[0].offset = target_min;
    mapping.to[0].max = target_max;
    mapping.to[0].scale = scale;
    Ok(candidate)
}

#[cfg(test)]
#[path = "transform_mapping_tests.rs"]
mod tests;
