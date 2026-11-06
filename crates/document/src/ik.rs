//! Ordered static IK data. Evaluation capabilities do not rewrite authored data.
use crate::Bone;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Constraint {
    Ik(IkConstraint),
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
        if !self.setup.mix.is_finite() || !(0.0..=1.0).contains(&self.setup.mix) {
            return Err(format!(
                "IK {}: mix must be finite and within 0–1",
                self.name
            ));
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
