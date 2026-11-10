//! Transient animation poses. These values never belong to a saved/exported document.
use crate::{
    AuthoredPose, Command, EditTarget, apply_command, ik_keyframe_at, local_pose,
    sample_authored_pose,
};
use peachy_document::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct PoseEdits {
    animation: Option<usize>,
    // A sequence rather than a map keeps diagnostics valid JSON (tuple map keys are not strings).
    values: Vec<PoseValue>,
    ik_values: Vec<IkPoseValue>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum IkPoseEdit {
    Mix { value: f32 },
    BendPositive { value: bool },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum IkProperty {
    Mix,
    BendPositive,
}
impl IkPoseEdit {
    fn property(self) -> IkProperty {
        match self {
            Self::Mix { .. } => IkProperty::Mix,
            Self::BendPositive { .. } => IkProperty::BendPositive,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
struct IkPoseValue {
    constraint: usize,
    edit: IkPoseEdit,
}

pub(crate) fn validate_ik_edits(edits: &[IkPoseEdit]) -> Result<(), String> {
    let mut properties = BTreeSet::new();
    for &edit in edits {
        if !properties.insert(edit.property()) {
            return Err("Duplicate IK pose property".into());
        }
        if let IkPoseEdit::Mix { value } = edit
            && !value.is_finite()
        {
            return Err("IK Mix must be finite".into());
        }
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
struct PoseValue {
    bone: usize,
    property: Property,
    value: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyState {
    Absent,
    Edited,
    Keyed,
}

/// Spine's default transform groups. Low-level SetPose/SetKey still support scalar tracks.
pub fn transform_group(properties: &[Property]) -> Vec<Property> {
    Property::ALL
        .into_iter()
        .filter(|p| {
            properties.iter().any(|q| {
                p == q
                    || matches!(
                        (p, q),
                        (Property::X, Property::Y)
                            | (Property::Y, Property::X)
                            | (Property::ScaleX, Property::ScaleY)
                            | (Property::ScaleY, Property::ScaleX)
                            | (Property::ShearX, Property::ShearY)
                            | (Property::ShearY, Property::ShearX)
                    )
            })
        })
        .collect()
}

impl PoseEdits {
    pub fn is_empty(&self) -> bool {
        self.values.is_empty() && self.ik_values.is_empty()
    }
    pub fn has_bone(&self, animation: usize, bone: usize) -> bool {
        self.animation == Some(animation) && self.values.iter().any(|v| v.bone == bone)
    }
    pub fn has_ik(&self, animation: usize, constraint: usize) -> bool {
        self.animation == Some(animation)
            && self.ik_values.iter().any(|v| v.constraint == constraint)
    }
    pub fn ik_key_state(
        &self,
        doc: &Document,
        animation: usize,
        time: f32,
        constraint: usize,
    ) -> Result<KeyState, String> {
        if constraint >= doc.constraints.len() {
            return Err("Unknown IK constraint".into());
        }
        let clip = doc.animations.get(animation).ok_or("Unknown animation")?;
        if !time.is_finite() || time < 0. {
            return Err("Pose time must be finite and nonnegative".into());
        }
        if self.has_ik(animation, constraint) {
            return Ok(KeyState::Edited);
        }
        let key = clip
            .ik_tracks
            .iter()
            .find(|t| t.constraint == constraint)
            .map(|t| ik_keyframe_at(&t.keys, time))
            .transpose()?
            .flatten();
        Ok(if key.is_some() {
            KeyState::Keyed
        } else {
            KeyState::Absent
        })
    }
    pub(crate) fn authored(
        &self,
        doc: &Document,
        animation: Option<usize>,
        time: f32,
    ) -> Result<AuthoredPose, String> {
        let mut pose = sample_authored_pose(doc, animation, time)?;
        if animation.is_some() && self.animation == animation {
            for value in &self.values {
                let local = pose
                    .locals
                    .get_mut(value.bone)
                    .ok_or("Unknown pending bone")?;
                value.property.write(local, value.value);
            }
            for value in &self.ik_values {
                let ik = pose
                    .ik
                    .get_mut(value.constraint)
                    .ok_or("Unknown pending IK constraint")?;
                match value.edit {
                    IkPoseEdit::Mix { value } => ik.mix = f64::from(value),
                    IkPoseEdit::BendPositive { value } => ik.bend_positive = value,
                }
            }
        }
        Ok(pose)
    }
    pub(crate) fn edit_ik(
        &self,
        doc: &Document,
        animation: usize,
        time: f32,
        constraint: usize,
        edits: &[IkPoseEdit],
    ) -> Result<Self, String> {
        validate_ik_edits(edits)?;
        let sampled = sample_authored_pose(doc, Some(animation), time)?;
        let base = *sampled
            .ik_poses()
            .get(constraint)
            .ok_or("Unknown IK constraint")?;
        if edits.is_empty() {
            return Ok(self.clone());
        }
        let mut next = if self.animation == Some(animation) {
            self.clone()
        } else {
            Self::default()
        };
        next.animation = Some(animation);
        for &edit in edits {
            next.ik_values
                .retain(|v| v.constraint != constraint || v.edit.property() != edit.property());
            let differs = match edit {
                IkPoseEdit::Mix { value } => f64::from(value) != base.mix,
                IkPoseEdit::BendPositive { value } => value != base.bend_positive,
            };
            if differs {
                next.ik_values.push(IkPoseValue { constraint, edit });
            }
        }
        next.ik_values
            .sort_by_key(|v| (v.constraint, v.edit.property()));
        if next.is_empty() {
            next.animation = None;
        }
        Ok(next)
    }
    pub(crate) fn remove_ik(&mut self, animation: usize, constraint: usize) {
        if self.animation == Some(animation) {
            self.ik_values.retain(|v| v.constraint != constraint);
            if self.is_empty() {
                self.animation = None;
            }
        }
    }
    pub(crate) fn ik_targets(&self, animation: usize) -> BTreeSet<usize> {
        if self.animation == Some(animation) {
            self.ik_values.iter().map(|v| v.constraint).collect()
        } else {
            BTreeSet::new()
        }
    }
    pub fn locals(&self, doc: &Document, animation: Option<usize>, time: f32) -> Vec<Transform> {
        let mut locals = local_pose(doc, animation, time);
        if animation.is_some() && self.animation == animation {
            for v in &self.values {
                if let Some(local) = locals.get_mut(v.bone) {
                    v.property.write(local, v.value);
                }
            }
        }
        locals
    }
    pub fn key_state(
        &self,
        doc: &Document,
        animation: usize,
        time: f32,
        bone: usize,
        properties: &[Property],
    ) -> KeyState {
        if self.animation == Some(animation)
            && self
                .values
                .iter()
                .any(|v| v.bone == bone && properties.contains(&v.property))
        {
            KeyState::Edited
        } else if !properties.is_empty()
            && doc.animations.get(animation).is_some_and(|a| {
                properties.iter().all(|p| {
                    a.tracks.iter().any(|t| {
                        t.bone == bone
                            && t.property == *p
                            && t.keys.iter().any(|k| same_key_time(k.time, time))
                    })
                })
            })
        {
            KeyState::Keyed
        } else {
            KeyState::Absent
        }
    }
    pub(crate) fn edit(
        &self,
        doc: &Document,
        animation: usize,
        time: f32,
        bone: usize,
        transform: Transform,
        properties: &[Property],
    ) -> Result<Self, String> {
        // Reuse the command boundary's validation, including setup scales and finite authoring time.
        apply_command(
            doc,
            Command::SetPose {
                bone,
                target: EditTarget::Animate { animation, time },
                transform,
                properties: properties.to_vec(),
            },
        )?;
        let base = local_pose(doc, Some(animation), time)[bone];
        let mut next = if self.animation == Some(animation) {
            self.clone()
        } else {
            Self::default()
        };
        next.animation = Some(animation);
        for &property in properties {
            next.values
                .retain(|v| v.bone != bone || v.property != property);
            let value = property.read(transform);
            if value != property.read(base) {
                next.values.push(PoseValue {
                    bone,
                    property,
                    value,
                });
            }
        }
        next.values.sort_by_key(|v| (v.bone, v.property));
        if next.is_empty() {
            next.animation = None;
        }
        Ok(next)
    }
    pub(crate) fn remove(&mut self, animation: usize, bone: usize, properties: &[Property]) {
        if self.animation == Some(animation) {
            self.values
                .retain(|v| v.bone != bone || !properties.contains(&v.property));
            if self.is_empty() {
                self.animation = None;
            }
        }
    }
    pub(crate) fn targets(
        &self,
        animation: usize,
        bone: Option<usize>,
    ) -> BTreeMap<usize, Vec<Property>> {
        let mut targets: BTreeMap<usize, Vec<Property>> = BTreeMap::new();
        if self.animation == Some(animation) {
            for v in &self.values {
                if bone.is_none_or(|b| b == v.bone) {
                    targets.entry(v.bone).or_default().push(v.property);
                }
            }
        }
        for properties in targets.values_mut() {
            *properties = transform_group(properties);
        }
        targets
    }
}
