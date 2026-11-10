//! Ordered IK data. Evaluation capabilities do not rewrite authored data.
use crate::{Bone, PhysicsConstraint, TransformConstraint};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Constraint {
    Ik(IkConstraint),
    Physics(PhysicsConstraint),
    Transform(TransformConstraint),
}

impl Constraint {
    pub fn name(&self) -> &str {
        match self {
            Self::Ik(ik) => &ik.name,
            Self::Physics(physics) => &physics.name,
            Self::Transform(transform) => &transform.name,
        }
    }
    pub fn set_name(&mut self, name: String) {
        match self {
            Self::Ik(ik) => ik.name = name,
            Self::Physics(physics) => physics.name = name,
            Self::Transform(transform) => transform.name = name,
        }
    }
    pub fn as_ik(&self) -> Option<&IkConstraint> {
        match self {
            Self::Ik(ik) => Some(ik),
            Self::Physics(_) | Self::Transform(_) => None,
        }
    }
    pub fn as_transform(&self) -> Option<&TransformConstraint> {
        match self {
            Self::Transform(transform) => Some(transform),
            Self::Ik(_) | Self::Physics(_) => None,
        }
    }
    pub fn as_physics(&self) -> Option<&PhysicsConstraint> {
        match self {
            Self::Physics(physics) => Some(physics),
            Self::Ik(_) | Self::Transform(_) => None,
        }
    }
    /// Spine 4.3's unified constraint `type` and Tree terminology.
    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::Ik(_) => "IK Constraint",
            Self::Physics(_) => "Physics Constraint",
            Self::Transform(_) => "Transform Constraint",
        }
    }
    pub fn validate(&self, bones: &[Bone]) -> Result<(), String> {
        match self {
            Self::Ik(ik) => ik.validate(bones),
            Self::Physics(physics) => physics.validate(bones),
            Self::Transform(transform) => transform.validate(bones),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IkConstraint {
    pub name: String,
    pub bones: Vec<usize>,
    pub target: usize,
    pub setup: IkSetup,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IkSetup {
    pub mix: f32,
    pub bend_positive: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IkTrack {
    pub constraint: usize,
    pub keys: Vec<IkKeyframe>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IkKeyframe {
    pub time: f32,
    pub mix: f32,
    pub bend_positive: bool,
    pub curve: IkCurve,
}

/// Unlike bone curves, Mix controls are absolute, including equal-key excursions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum IkCurve {
    #[default]
    Linear,
    Stepped,
    /// Relative time survives retiming; f64 retains absolute-time reconstruction.
    Bezier {
        time: [f64; 2],
        mix: [f32; 2],
    },
}

// Internally tagged unit variants otherwise discard unknown fields. Empty
// struct payloads make Linear/Stepped as strict as the Bezier payload.
impl<'de> Deserialize<'de> for IkCurve {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Empty {}
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
        enum Wire {
            Linear(Empty),
            Stepped(Empty),
            Bezier { time: [f64; 2], mix: [f32; 2] },
        }
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Linear(_) => Self::Linear,
            Wire::Stepped(_) => Self::Stepped,
            Wire::Bezier { time, mix } => Self::Bezier { time, mix },
        })
    }
}

impl IkCurve {
    pub fn validate(self) -> Result<(), String> {
        if let Self::Bezier { time, mix } = self
            && (!time
                .into_iter()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(&v))
                || !mix.into_iter().all(f32::is_finite))
        {
            return Err("IK curve needs finite Mix controls and time handles within 0–1".into());
        }
        Ok(())
    }
}

impl IkKeyframe {
    pub fn validate(self) -> Result<(), String> {
        if !self.time.is_finite() || self.time < 0. || !self.mix.is_finite() {
            return Err("IK keys need finite Mix and nonnegative finite time".into());
        }
        self.curve.validate()
    }
}

impl IkConstraint {
    /// Structural validation only: raw finite authored poses remain serializable
    /// even when a checked evaluation reports an unsupported current pose.
    pub fn validate(&self, bones: &[Bone]) -> Result<(), String> {
        if bones
            .iter()
            .enumerate()
            .any(|(i, bone)| bone.parent.is_some_and(|parent| parent >= i))
        {
            return Err("IK requires a parent-first bone hierarchy".into());
        }
        if self.name.trim().is_empty() {
            return Err("Constraint names must be nonempty".into());
        }
        if !self.setup.mix.is_finite() {
            return Err(format!("IK {}: mix must be finite", self.name));
        }
        if !(1..=2).contains(&self.bones.len())
            || self.bones.iter().any(|&bone| bone >= bones.len())
            || self.target >= bones.len()
        {
            return Err(format!(
                "IK {}: expected one or two valid bones and a target",
                self.name
            ));
        }
        if self.bones.len() == 2 && bones[self.bones[1]].parent != Some(self.bones[0]) {
            return Err(format!(
                "IK {}: child must immediately follow its constrained parent",
                self.name
            ));
        }
        let mut current = Some(self.target);
        while let Some(bone) = current {
            if bone == self.bones[0] {
                return Err(format!(
                    "IK {}: target cannot be a constrained bone or its descendant",
                    self.name
                ));
            }
            // Document validates parent-first ordering before constraints.
            current = bones[bone].parent;
        }
        Ok(())
    }
}
