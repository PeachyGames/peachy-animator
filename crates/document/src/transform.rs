//! Spine 4.3 transform constraints as complete data: source, constrained bones,
//! offsets, mixes, property mappings and mode flags. Evaluation support is a
//! separate, bounded engine capability (docs/slices/transform-constraints.md).
//! Field names and defaults were observed through installed Spine 4.3.26 and
//! the external 4.3.13 runtime's public data API.
use crate::Bone;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransformProperty {
    Rotate,
    X,
    Y,
    ScaleX,
    ScaleY,
    ShearY,
}

impl TransformProperty {
    pub const ALL: [Self; 6] = [
        Self::Rotate,
        Self::X,
        Self::Y,
        Self::ScaleX,
        Self::ScaleY,
        Self::ShearY,
    ];
    /// Spine JSON property key.
    pub fn spine_name(self) -> &'static str {
        match self {
            Self::Rotate => "rotate",
            Self::X => "x",
            Self::Y => "y",
            Self::ScaleX => "scaleX",
            Self::ScaleY => "scaleY",
            Self::ShearY => "shearY",
        }
    }
    /// Spine JSON offset key on the constraint.
    pub fn offset_name(self) -> &'static str {
        match self {
            Self::Rotate => "rotation",
            other => other.spine_name(),
        }
    }
    /// Spine JSON mix key on the constraint and its timeline keys.
    pub fn mix_name(self) -> &'static str {
        match self {
            Self::Rotate => "mixRotate",
            Self::X => "mixX",
            Self::Y => "mixY",
            Self::ScaleX => "mixScaleX",
            Self::ScaleY => "mixScaleY",
            Self::ShearY => "mixShearY",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Rotate => "Rotate",
            Self::X => "Translate X",
            Self::Y => "Translate Y",
            Self::ScaleX => "Scale X",
            Self::ScaleY => "Scale Y",
            Self::ShearY => "Shear Y",
        }
    }
    pub fn from_spine_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.spine_name() == name)
    }
}

/// One value per property, in [`TransformProperty::ALL`] order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransformValues {
    pub rotate: f32,
    pub x: f32,
    pub y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub shear_y: f32,
}

impl TransformValues {
    pub fn get(&self, property: TransformProperty) -> f32 {
        match property {
            TransformProperty::Rotate => self.rotate,
            TransformProperty::X => self.x,
            TransformProperty::Y => self.y,
            TransformProperty::ScaleX => self.scale_x,
            TransformProperty::ScaleY => self.scale_y,
            TransformProperty::ShearY => self.shear_y,
        }
    }
    pub fn set(&mut self, property: TransformProperty, value: f32) {
        match property {
            TransformProperty::Rotate => self.rotate = value,
            TransformProperty::X => self.x = value,
            TransformProperty::Y => self.y = value,
            TransformProperty::ScaleX => self.scale_x = value,
            TransformProperty::ScaleY => self.scale_y = value,
            TransformProperty::ShearY => self.shear_y = value,
        }
    }
}

/// A source property mapped to one or more constrained properties.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransformMapping {
    pub from: TransformProperty,
    /// Source value corresponding to each target's `offset`.
    pub offset: f32,
    pub to: Vec<TransformTarget>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransformTarget {
    pub property: TransformProperty,
    pub offset: f32,
    /// Upper bound when the constraint clamps. Observed default 1.
    pub max: f32,
    /// Scale of the source value. Observed default 1.
    pub scale: f32,
}

impl TransformTarget {
    pub fn identity(property: TransformProperty) -> Self {
        Self {
            property,
            offset: 0.0,
            max: 1.0,
            scale: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransformConstraint {
    pub name: String,
    pub bones: Vec<usize>,
    pub source: usize,
    /// Constraint offsets (`rotation`, `x`, `y`, `scaleX`, `scaleY`, `shearY`).
    pub offsets: TransformValues,
    /// Setup mixes, unbounded (negative mirrors).
    pub mix: TransformValues,
    pub mappings: Vec<TransformMapping>,
    pub local_source: bool,
    pub local_target: bool,
    pub additive: bool,
    pub clamp: bool,
}

impl TransformConstraint {
    /// Every constrained property, in mapping order without duplicates.
    pub fn targets(&self) -> Vec<TransformProperty> {
        let mut out = vec![];
        for target in self.mappings.iter().flat_map(|m| &m.to) {
            if !out.contains(&target.property) {
                out.push(target.property);
            }
        }
        out
    }

    /// The bounded subset the engine evaluates and the adapter imports: world
    /// mode, not additive or clamped, identity Rotate/X/Y mappings with neutral
    /// offsets and scale. Everything else is refused explicitly.
    pub fn supported_subset(&self) -> Result<(), String> {
        let context = || format!("Transform {}", self.name);
        if self.local_source || self.local_target || self.additive || self.clamp {
            return Err(format!(
                "{}: local, additive and clamped transform constraints are unsupported",
                context()
            ));
        }
        for mapping in &self.mappings {
            let identity = mapping.to.len() == 1
                && mapping.to[0].property == mapping.from
                && mapping.offset == 0.0
                && mapping.to[0].offset == 0.0
                && mapping.to[0].scale == 1.0;
            if !identity
                || !matches!(
                    mapping.from,
                    TransformProperty::Rotate | TransformProperty::X | TransformProperty::Y
                )
            {
                return Err(format!(
                    "{}: only identity Rotate/X/Y mappings are supported",
                    context()
                ));
            }
        }
        Ok(())
    }

    pub fn validate(&self, bones: &[Bone]) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Constraint names must be nonempty".into());
        }
        let context = || format!("Transform {}", self.name);
        if self.source >= bones.len()
            || self.bones.is_empty()
            || self.bones.iter().any(|&bone| bone >= bones.len())
        {
            return Err(format!(
                "{}: needs a valid source and at least one constrained bone",
                context()
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        if self
            .bones
            .iter()
            .any(|bone| !seen.insert(*bone) || *bone == self.source)
        {
            return Err(format!(
                "{}: constrained bones must be distinct and exclude the source",
                context()
            ));
        }
        let values =
            |v: TransformValues| TransformProperty::ALL.iter().all(|&p| v.get(p).is_finite());
        if !values(self.offsets) || !values(self.mix) {
            return Err(format!("{}: offsets and mixes must be finite", context()));
        }
        let mut from = std::collections::BTreeSet::new();
        for mapping in &self.mappings {
            if !from.insert(mapping.from) || mapping.to.is_empty() || !mapping.offset.is_finite() {
                return Err(format!(
                    "{}: each source property maps once, to at least one property",
                    context()
                ));
            }
            let mut to = std::collections::BTreeSet::new();
            for target in &mapping.to {
                if !to.insert(target.property)
                    || ![target.offset, target.max, target.scale]
                        .iter()
                        .all(|v| v.is_finite())
                {
                    return Err(format!(
                        "{}: mapped targets must be distinct and finite",
                        context()
                    ));
                }
            }
        }
        Ok(())
    }
}
