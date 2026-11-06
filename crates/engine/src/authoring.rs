//! Transient animation poses. These values never belong to a saved/exported document.
use crate::{Command, EditTarget, apply_command, local_pose};
use peachy_document::*;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct PoseEdits {
    animation: Option<usize>,
    // A sequence rather than a map keeps diagnostics valid JSON (tuple map keys are not strings).
    values: Vec<PoseValue>,
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
        self.values.is_empty()
    }
    pub fn has_bone(&self, animation: usize, bone: usize) -> bool {
        self.animation == Some(animation) && self.values.iter().any(|v| v.bone == bone)
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
