//! Transient animation poses. These values never belong to a saved/exported document.
use crate::{
    AuthoredPose, Command, EditTarget, SliderPoseEdit, SlotPoseEdit, SlotProperty, apply_command,
    ik_keyframe_at, local_pose, sample_authored_pose,
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
    slider_values: Vec<SliderPoseValue>,
    deform_values: Vec<DeformPoseValue>,
    slot_attachments: Vec<SlotAttachmentPoseValue>,
    slot_colors: Vec<SlotColorPoseValue>,
    /// Complete global order for the current animation, never serialized in Document.
    draw_order: Option<Vec<usize>>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
struct SlotAttachmentPoseValue {
    slot: usize,
    attachment: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
struct SlotColorPoseValue {
    slot: usize,
    color: [f32; 4],
}
#[derive(Clone, Debug, PartialEq, Serialize)]
struct DeformPoseValue {
    attachment: AttachmentRef,
    offsets: Vec<f32>,
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
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
struct SliderPoseValue {
    constraint: usize,
    edit: SliderPoseEdit,
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
    pub(crate) fn after_pose_operations(
        &self,
        operations: &[crate::constraint_effects::PoseOperation],
    ) -> Result<Self, String> {
        let mut next = self.clone();
        for operation in operations {
            match operation {
                crate::constraint_effects::PoseOperation::Remap(effects) => {
                    next = next.remapped_constraints(effects)?
                }
                crate::constraint_effects::PoseOperation::ClearSliderTime { constraint } => {
                    next.slider_values.retain(|value| {
                        value.constraint != *constraint
                            || value.edit.property() != SliderProperty::Time
                    });
                    if next.is_empty() {
                        next.animation = None;
                    }
                }
            }
        }
        Ok(next)
    }
    pub(crate) fn remapped_constraints(
        &self,
        effects: &crate::EditEffects,
    ) -> Result<Self, String> {
        let Some(map) = &effects.constraint_indices else {
            return Ok(self.clone());
        };
        let mut next = self.clone();
        for value in &mut next.ik_values {
            value.constraint = map
                .map_index(value.constraint)?
                .ok_or("Pending IK constraint was removed")?;
        }
        for value in &mut next.slider_values {
            value.constraint = map
                .map_index(value.constraint)?
                .ok_or("Pending Slider constraint was removed")?;
        }
        Ok(next)
    }
    /// Independent Time/Mix key state, including an unkeyed authored override.
    pub fn slider_key_state(
        &self,
        doc: &Document,
        animation: usize,
        time: f32,
        constraint: usize,
        property: SliderProperty,
    ) -> Result<KeyState, String> {
        let key = crate::slider_key_time(doc, animation, time, constraint, property)?;
        Ok(if self.has_slider(animation, constraint, property) {
            KeyState::Edited
        } else if key.is_some() {
            KeyState::Keyed
        } else {
            KeyState::Absent
        })
    }
    pub fn has_slider(
        &self,
        animation: usize,
        constraint: usize,
        property: SliderProperty,
    ) -> bool {
        self.animation == Some(animation)
            && self
                .slider_values
                .iter()
                .any(|value| value.constraint == constraint && value.edit.property() == property)
    }
    pub(crate) fn edit_slider(
        &self,
        doc: &Document,
        animation: usize,
        time: f32,
        constraint: usize,
        edits: &[SliderPoseEdit],
        skin: &str,
    ) -> Result<Self, String> {
        crate::slider_authoring::validate_edits(edits)?;
        if doc
            .constraints
            .get(constraint)
            .and_then(Constraint::as_slider)
            .is_none()
        {
            return Err("Unknown Slider constraint".into());
        }
        let sampled = crate::sample_authored_pose_with_skin(doc, Some(animation), time, skin)?;
        let base = sampled
            .slider_poses()
            .get(constraint)
            .ok_or("Unknown Slider constraint")?;
        if edits.is_empty() {
            return Ok(self.clone());
        }
        let mut next = if self.animation == Some(animation) {
            self.clone()
        } else {
            Self::default()
        };
        for &edit in edits {
            next.slider_values.retain(|value| {
                value.constraint != constraint || value.edit.property() != edit.property()
            });
            if f64::from(edit.value()) != base.value(edit.property()) {
                next.slider_values
                    .push(SliderPoseValue { constraint, edit });
            }
        }
        next.slider_values
            .sort_by_key(|value| (value.constraint, value.edit.property()));
        next.animation = if next.is_empty() {
            None
        } else {
            Some(animation)
        };
        Ok(next)
    }
    pub(crate) fn remove_slider(
        &mut self,
        animation: usize,
        constraint: usize,
        properties: &[SliderProperty],
    ) {
        if self.animation == Some(animation) {
            self.slider_values.retain(|value| {
                value.constraint != constraint || !properties.contains(&value.edit.property())
            });
            if self.is_empty() {
                self.animation = None;
            }
        }
    }
    pub(crate) fn slider_targets(&self, animation: usize) -> BTreeMap<usize, Vec<SliderProperty>> {
        let mut targets: BTreeMap<usize, Vec<SliderProperty>> = BTreeMap::new();
        if self.animation == Some(animation) {
            for value in &self.slider_values {
                targets
                    .entry(value.constraint)
                    .or_default()
                    .push(value.edit.property());
            }
        }
        targets
    }
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
            && self.ik_values.is_empty()
            && self.slider_values.is_empty()
            && self.deform_values.is_empty()
            && self.slot_attachments.is_empty()
            && self.slot_colors.is_empty()
            && self.draw_order.is_none()
    }
    pub fn pending_draw_order(&self, animation: usize) -> Option<&[usize]> {
        (self.animation == Some(animation))
            .then_some(self.draw_order.as_deref())
            .flatten()
    }
    pub fn has_draw_order(&self, animation: usize) -> bool {
        self.pending_draw_order(animation).is_some()
    }
    pub fn draw_order_key_state(
        &self,
        doc: &Document,
        animation: usize,
        time: f32,
    ) -> Result<KeyState, String> {
        let key = crate::draw_order_key_time(doc, animation, time)?;
        Ok(if self.has_draw_order(animation) {
            KeyState::Edited
        } else if key.is_some() {
            KeyState::Keyed
        } else {
            KeyState::Absent
        })
    }
    pub(crate) fn edit_draw_order(
        &self,
        doc: &Document,
        animation: usize,
        time: f32,
        order: Vec<usize>,
    ) -> Result<Self, String> {
        crate::draw_order_authoring::validate_order(doc.slots.len(), &order)?;
        let sampled = sample_authored_pose(doc, Some(animation), time)?;
        let mut next = if self.animation == Some(animation) {
            self.clone()
        } else {
            Self::default()
        };
        next.draw_order = (order != sampled.draw_order()).then_some(order);
        next.animation = if next.is_empty() {
            None
        } else {
            Some(animation)
        };
        Ok(next)
    }
    pub(crate) fn remove_draw_order(&mut self, animation: usize) {
        if self.animation == Some(animation) {
            self.draw_order = None;
            if self.is_empty() {
                self.animation = None;
            }
        }
    }
    pub fn has_slot(&self, animation: usize, slot: usize, property: SlotProperty) -> bool {
        self.animation == Some(animation)
            && match property {
                SlotProperty::Attachment => {
                    self.slot_attachments.iter().any(|value| value.slot == slot)
                }
                SlotProperty::Color => self.slot_colors.iter().any(|value| value.slot == slot),
            }
    }
    pub fn slot_key_state(
        &self,
        doc: &Document,
        animation: usize,
        time: f32,
        slot: usize,
        property: SlotProperty,
    ) -> Result<KeyState, String> {
        let key = crate::slot_key_time(doc, animation, time, slot, property)?;
        Ok(if self.has_slot(animation, slot, property) {
            KeyState::Edited
        } else if key.is_some() {
            KeyState::Keyed
        } else {
            KeyState::Absent
        })
    }
    pub(crate) fn edit_slot(
        &self,
        doc: &Document,
        animation: usize,
        time: f32,
        slot: usize,
        edits: &[SlotPoseEdit],
        skin: &str,
    ) -> Result<Self, String> {
        crate::slot_authoring::validate_edits(doc, slot, edits)?;
        let sampled = crate::sample_authored_pose_with_skin(doc, Some(animation), time, skin)?;
        let base = crate::SlotPose::from_authored(&sampled, slot)?;
        let mut next = if self.animation == Some(animation) {
            self.clone()
        } else {
            Self::default()
        };
        for edit in edits {
            match edit {
                SlotPoseEdit::Attachment { attachment } => {
                    next.slot_attachments.retain(|value| value.slot != slot);
                    if *attachment != base.attachment {
                        next.slot_attachments.push(SlotAttachmentPoseValue {
                            slot,
                            attachment: attachment.clone(),
                        });
                    }
                }
                SlotPoseEdit::Color { color } => {
                    next.slot_colors.retain(|value| value.slot != slot);
                    if *color != base.color {
                        next.slot_colors.push(SlotColorPoseValue {
                            slot,
                            color: *color,
                        });
                    }
                }
            }
        }
        next.slot_attachments.sort_by_key(|value| value.slot);
        next.slot_colors.sort_by_key(|value| value.slot);
        next.animation = if next.is_empty() {
            None
        } else {
            Some(animation)
        };
        Ok(next)
    }
    pub(crate) fn remove_slot(
        &mut self,
        animation: usize,
        slot: usize,
        properties: &[SlotProperty],
    ) {
        if self.animation == Some(animation) {
            if properties.contains(&SlotProperty::Attachment) {
                self.slot_attachments.retain(|value| value.slot != slot);
            }
            if properties.contains(&SlotProperty::Color) {
                self.slot_colors.retain(|value| value.slot != slot);
            }
            if self.is_empty() {
                self.animation = None;
            }
        }
    }
    pub(crate) fn slot_targets(&self, animation: usize) -> BTreeMap<usize, Vec<SlotProperty>> {
        let mut targets: BTreeMap<usize, Vec<SlotProperty>> = BTreeMap::new();
        if self.animation == Some(animation) {
            for value in &self.slot_attachments {
                targets
                    .entry(value.slot)
                    .or_default()
                    .push(SlotProperty::Attachment);
            }
            for value in &self.slot_colors {
                targets
                    .entry(value.slot)
                    .or_default()
                    .push(SlotProperty::Color);
            }
        }
        targets
    }
    /// `attachment` is a canonical deform target (Document::deform_target).
    pub fn has_deform(&self, animation: usize, attachment: &AttachmentRef) -> bool {
        self.animation == Some(animation)
            && self
                .deform_values
                .iter()
                .any(|value| value.attachment == *attachment)
    }
    pub fn deform_key_state(
        &self,
        doc: &Document,
        animation: usize,
        time: f32,
        attachment: &AttachmentRef,
    ) -> Result<KeyState, String> {
        let (_, canonical, _) = crate::deform_authoring::mesh_target(doc, attachment)?;
        let clip = doc.animations.get(animation).ok_or("Unknown animation")?;
        if !time.is_finite() || time < 0. {
            return Err("Pose time must be finite and nonnegative".into());
        }
        if self.has_deform(animation, &canonical) {
            return Ok(KeyState::Edited);
        }
        let key = clip
            .deform_tracks
            .iter()
            .find(|track| track.attachment == canonical)
            .map(|track| crate::deform_keyframe_at(&track.keys, time))
            .transpose()?
            .flatten();
        Ok(if key.is_some() {
            KeyState::Keyed
        } else {
            KeyState::Absent
        })
    }
    pub(crate) fn edit_deform(
        &self,
        doc: &Document,
        animation: usize,
        time: f32,
        attachment: &AttachmentRef,
        offsets: Vec<f32>,
        skin: &str,
    ) -> Result<Self, String> {
        let canonical = crate::deform_authoring::validate_offsets(doc, attachment, &offsets)?;
        let sampled = crate::sample_authored_pose_with_skin(doc, Some(animation), time, skin)?;
        let base = crate::deform_authoring::offsets(doc, &sampled, &canonical)?;
        let mut next = if self.animation == Some(animation) {
            self.clone()
        } else {
            Self::default()
        };
        next.deform_values
            .retain(|value| value.attachment != canonical);
        if offsets != base {
            next.deform_values.push(DeformPoseValue {
                attachment: canonical,
                offsets,
            });
        }
        next.sort_deforms();
        next.animation = if next.is_empty() {
            None
        } else {
            Some(animation)
        };
        Ok(next)
    }
    fn sort_deforms(&mut self) {
        self.deform_values.sort_by(|a, b| {
            (
                &a.attachment.skin,
                &a.attachment.slot,
                &a.attachment.placeholder,
            )
                .cmp(&(
                    &b.attachment.skin,
                    &b.attachment.slot,
                    &b.attachment.placeholder,
                ))
        });
    }
    pub(crate) fn remove_deform(&mut self, animation: usize, attachment: &AttachmentRef) {
        if self.animation == Some(animation) {
            self.deform_values
                .retain(|value| value.attachment != *attachment);
            if self.is_empty() {
                self.animation = None;
            }
        }
    }
    pub(crate) fn deform_targets(&self, animation: usize) -> Vec<AttachmentRef> {
        if self.animation == Some(animation) {
            self.deform_values
                .iter()
                .map(|value| value.attachment.clone())
                .collect()
        } else {
            vec![]
        }
    }
    pub(crate) fn after_document(
        &self,
        before: &Document,
        after: &Document,
    ) -> Result<Self, String> {
        let mut next = self.clone();
        if let Some(order) = &next.draw_order {
            if before.slots.len() != after.slots.len() {
                return Err("Slot count cannot change while a draw order pose is pending".into());
            }
            crate::draw_order_authoring::validate_order(after.slots.len(), order)?;
        }
        for value in &mut next.deform_values {
            value.attachment =
                crate::deform_authoring::check_layout_transition(before, after, &value.attachment)?;
            value.attachment = crate::deform_authoring::validate_offsets(
                after,
                &value.attachment,
                &value.offsets,
            )?;
        }
        for value in &next.slot_attachments {
            crate::slot_authoring::validate_edits(
                after,
                value.slot,
                &[SlotPoseEdit::Attachment {
                    attachment: value.attachment.clone(),
                }],
            )?;
        }
        for value in &next.slot_colors {
            crate::slot_authoring::validate_edits(
                after,
                value.slot,
                &[SlotPoseEdit::Color { color: value.color }],
            )?;
        }
        for value in &next.slider_values {
            if after
                .constraints
                .get(value.constraint)
                .and_then(Constraint::as_slider)
                .is_none()
            {
                return Err("Pending Slider constraint is missing or changed kind".into());
            }
            crate::slider_authoring::validate_edits(&[value.edit])?;
        }
        next.sort_deforms();
        Ok(next)
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
    pub(crate) fn authored_with_skin(
        &self,
        doc: &Document,
        animation: Option<usize>,
        time: f32,
        skin: &str,
    ) -> Result<AuthoredPose, String> {
        let mut pose = crate::sample_authored_pose_with_skin(doc, animation, time, skin)?;
        if animation.is_some() && self.animation == animation {
            if let Some(order) = &self.draw_order {
                crate::draw_order_authoring::validate_order(doc.slots.len(), order)?;
                pose.draw_order = order.clone();
            }
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
            for value in &self.slider_values {
                let slider = pose
                    .sliders
                    .get_mut(value.constraint)
                    .ok_or("Unknown pending Slider constraint")?;
                match value.edit {
                    SliderPoseEdit::Time { value } => slider.time = f64::from(value),
                    SliderPoseEdit::Mix { value } => slider.mix = f64::from(value),
                }
            }
            for value in &self.slot_attachments {
                *pose
                    .slot_attachments
                    .get_mut(value.slot)
                    .ok_or("Unknown pending slot")? = value.attachment.clone();
            }
            for value in &self.slot_colors {
                *pose
                    .slot_colors
                    .get_mut(value.slot)
                    .ok_or("Unknown pending slot")? = value.color;
            }
            for value in &self.deform_values {
                crate::deform_authoring::validate_offsets(doc, &value.attachment, &value.offsets)?;
                pose.deforms.retain(|(key, _)| *key != value.attachment);
                pose.deforms
                    .push((value.attachment.clone(), value.offsets.clone()));
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
