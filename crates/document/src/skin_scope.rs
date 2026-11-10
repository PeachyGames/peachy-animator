//! Skin-required bones and constraints, and which named skins activate them.
//! Activity rules were observed with the external 4.3.13 runtime: a required
//! bone is active when the active skin lists it or one of its descendants; a
//! constraint is active when it is not required or its skin lists it, and its
//! referenced bones are active. See docs/slices/skin-scope.md.
use crate::{Constraint, Document};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkinScope {
    /// Bones marked `skin: true`.
    pub required_bones: BTreeSet<usize>,
    /// Constraints marked `skin: true`, by constraint index.
    pub required_constraints: BTreeSet<usize>,
    /// Named skin -> the required bones and constraints it lists.
    pub skins: BTreeMap<String, SkinMembers>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkinMembers {
    pub bones: Vec<usize>,
    pub constraints: Vec<usize>,
}

/// Which bones and constraints take part in evaluation for one active skin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Activity {
    pub bones: Vec<bool>,
    pub constraints: Vec<bool>,
}

impl SkinScope {
    pub fn is_empty(&self) -> bool {
        self.required_bones.is_empty() && self.required_constraints.is_empty()
    }
}

impl Document {
    /// Bones and constraints active under `skin` (any name; unknown or the
    /// default skin activates no required item).
    pub fn activity(&self, skin: &str) -> Activity {
        let scope = &self.skin_scope;
        let members = scope.skins.get(skin);
        let mut bones = vec![true; self.bones.len()];
        for &bone in &scope.required_bones {
            bones[bone] = false;
        }
        for &listed in members.map_or(&[][..], |m| &m.bones[..]) {
            // Listing a bone activates it and its ancestors.
            let mut current = Some(listed);
            while let Some(index) = current {
                bones[index] = true;
                current = self.bones[index].parent;
            }
        }
        let listed: BTreeSet<_> = members
            .map(|m| m.constraints.iter().copied().collect())
            .unwrap_or_default();
        let constraints = self
            .constraints
            .iter()
            .enumerate()
            .map(|(index, constraint)| {
                (!scope.required_constraints.contains(&index) || listed.contains(&index))
                    && constraint_bones(constraint).iter().all(|&b| bones[b])
            })
            .collect();
        Activity { bones, constraints }
    }

    pub(crate) fn validate_skin_scope(&self) -> Result<(), String> {
        let scope = &self.skin_scope;
        if scope.required_bones.iter().any(|&b| b >= self.bones.len())
            || scope
                .required_constraints
                .iter()
                .any(|&c| c >= self.constraints.len())
        {
            return Err("Skin-required bones and constraints must exist".into());
        }
        // Observed editor/game invariant: descendants of required bones are
        // required too; the runtime result otherwise is not modeled.
        for (index, bone) in self.bones.iter().enumerate() {
            if let Some(parent) = bone.parent
                && scope.required_bones.contains(&parent)
                && !scope.required_bones.contains(&index)
            {
                return Err(format!(
                    "Bone {} must be skin-required like its parent",
                    bone.name
                ));
            }
        }
        for (skin, members) in &scope.skins {
            if !self.skins.contains_key(skin) || skin == "default" {
                return Err(format!("Skin scope refers to unknown named skin {skin}"));
            }
            let mut seen = BTreeSet::new();
            if members
                .bones
                .iter()
                .any(|b| !scope.required_bones.contains(b) || !seen.insert(*b))
            {
                return Err(format!(
                    "Skin {skin}: listed bones must be distinct skin-required bones"
                ));
            }
            let mut seen = BTreeSet::new();
            if members
                .constraints
                .iter()
                .any(|c| !scope.required_constraints.contains(c) || !seen.insert(*c))
            {
                return Err(format!(
                    "Skin {skin}: listed constraints must be distinct skin-required constraints"
                ));
            }
        }
        Ok(())
    }
}

/// Bones a constraint reads or writes; all must be active for it to run.
fn constraint_bones(constraint: &Constraint) -> Vec<usize> {
    match constraint {
        Constraint::Ik(ik) => ik.bones.iter().copied().chain([ik.target]).collect(),
        Constraint::Physics(physics) => vec![physics.bone],
        Constraint::Transform(t) => t.bones.iter().copied().chain([t.source]).collect(),
    }
}
