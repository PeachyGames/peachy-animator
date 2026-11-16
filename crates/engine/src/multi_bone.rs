//! Explicit authored-local requests for atomic multi-bone Session transitions.
//!
//! Group geometry, coordinate spaces and pivots belong to separate operations.
//! These requests carry complete local results and never infer a group transform.
use peachy_document::{Document, Property, Transform, finite_transform};
use std::collections::{BTreeMap, BTreeSet};

/// One bone's explicit local result. Animate edits use only `properties`;
/// Setup edits replace the complete transform, as the single-bone API does.
#[derive(Clone, Debug, PartialEq)]
pub struct BonePoseEdit {
    pub bone: usize,
    pub transform: Transform,
    pub properties: Vec<Property>,
}

/// Properties to key from one shared authored-pose snapshot, including pending edits.
#[derive(Clone, Debug, PartialEq)]
pub struct BonePoseKey {
    pub bone: usize,
    pub properties: Vec<Property>,
}

fn validate_members<'a>(
    doc: &Document,
    members: impl Iterator<Item = (usize, &'a [Property])>,
) -> Result<BTreeMap<usize, Vec<Property>>, String> {
    let mut result = BTreeMap::new();
    for (bone, properties) in members {
        if bone >= doc.bones.len() {
            return Err(format!("Unknown bone {bone} in multi-bone pose request"));
        }
        if properties.is_empty() {
            return Err(format!("Bone {bone} requires at least one pose property"));
        }
        let mut unique = BTreeSet::new();
        for property in properties {
            if !unique.insert(*property) {
                return Err(format!("Duplicate pose property for bone {bone}"));
            }
        }
        if result.insert(bone, properties.to_vec()).is_some() {
            return Err(format!("Duplicate bone {bone} in multi-bone pose request"));
        }
    }
    if result.is_empty() {
        return Err("Multi-bone pose requests require at least one bone".into());
    }
    Ok(result)
}

pub(crate) fn validate_edits(
    doc: &Document,
    edits: &[BonePoseEdit],
) -> Result<BTreeMap<usize, Vec<Property>>, String> {
    let members = validate_members(
        doc,
        edits
            .iter()
            .map(|edit| (edit.bone, edit.properties.as_slice())),
    )?;
    for edit in edits {
        if !finite_transform(edit.transform) {
            return Err(format!("Pose for bone {} must be finite", edit.bone));
        }
    }
    Ok(members)
}

pub(crate) fn validate_keys(
    doc: &Document,
    keys: &[BonePoseKey],
) -> Result<BTreeMap<usize, Vec<Property>>, String> {
    validate_members(
        doc,
        keys.iter().map(|key| (key.bone, key.properties.as_slice())),
    )
}
