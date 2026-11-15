use crate::{
    AuthoredPose, Command, EditTarget, EvaluatedPose, IkPoseEdit, MeshBindPose, MeshWeightEdit,
    MeshWeightGesture, MeshWeightSelection, PoseEdits, SliderPose, SliderPoseEdit, SlotPose,
    SlotPoseEdit, SlotProperty, apply_command, checked_pose_from_authored, ik_keyframe_at,
    transform_group,
};
use peachy_document::{
    AttachmentRef, Constraint, Document, IkCurve, IkKeyframe, Property, SliderProperty, Transform,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, PartialEq)]
struct WorkingState {
    document: Document,
    pose: PoseEdits,
}

#[derive(Default)]
struct KeyTargets {
    bones: BTreeMap<usize, Vec<Property>>,
    iks: BTreeSet<usize>,
    sliders: BTreeMap<usize, Vec<SliderProperty>>,
    deforms: Vec<AttachmentRef>,
    slots: BTreeMap<usize, Vec<SlotProperty>>,
    draw_order: bool,
}

impl WorkingState {
    fn authored_pose(&self, target: EditTarget, skin: &str) -> Result<AuthoredPose, String> {
        match target {
            EditTarget::Setup => {
                crate::sample_authored_pose_with_skin(&self.document, None, 0., skin)
            }
            EditTarget::Animate { animation, time } => {
                self.pose
                    .authored_with_skin(&self.document, Some(animation), time, skin)
            }
        }
    }
    fn edit_slider_pose(
        &self,
        constraint: usize,
        target: EditTarget,
        edits: &[SliderPoseEdit],
        auto_key: bool,
        skin: &str,
    ) -> Result<Self, String> {
        crate::slider_authoring::validate_edits(edits)?;
        let slider = self
            .document
            .constraints
            .get(constraint)
            .and_then(Constraint::as_slider)
            .ok_or("Unknown Slider constraint")?;
        let current = *self
            .authored_pose(target, skin)?
            .slider_poses()
            .get(constraint)
            .ok_or("Unknown Slider constraint")?;
        let changed: Vec<_> = edits
            .iter()
            .copied()
            .filter(|edit| f64::from(edit.value()) != current.value(edit.property()))
            .collect();
        if changed.is_empty() {
            return Ok(self.clone());
        }
        match target {
            EditTarget::Setup => {
                let mut value = slider.clone();
                for edit in changed {
                    match edit {
                        SliderPoseEdit::Time { value: time } => value.setup.time = time,
                        SliderPoseEdit::Mix { value: mix } => value.setup.mix = mix,
                    }
                }
                Ok(Self {
                    document: apply_command(
                        &self.document,
                        Command::SetSliderConstraint { constraint, value },
                    )?,
                    pose: self.pose.clone(),
                })
            }
            EditTarget::Animate { animation, time } => {
                let candidate = Self {
                    document: self.document.clone(),
                    pose: self.pose.edit_slider(
                        &self.document,
                        animation,
                        time,
                        constraint,
                        &changed,
                        skin,
                    )?,
                };
                if auto_key {
                    candidate.key_targets(
                        animation,
                        time,
                        KeyTargets {
                            sliders: BTreeMap::from([(
                                constraint,
                                changed.iter().map(|edit| edit.property()).collect(),
                            )]),
                            ..Default::default()
                        },
                        skin,
                    )
                } else {
                    Ok(candidate)
                }
            }
        }
    }
    fn edit_ik_pose(
        &self,
        constraint: usize,
        target: EditTarget,
        edits: &[IkPoseEdit],
        auto_key: bool,
        skin: &str,
    ) -> Result<Self, String> {
        crate::authoring::validate_ik_edits(edits)?;
        let ik = self
            .document
            .constraints
            .get(constraint)
            .and_then(Constraint::as_ik)
            .ok_or("Unknown IK constraint")?;
        match target {
            EditTarget::Setup => {
                let mut value = ik.clone();
                for edit in edits {
                    match *edit {
                        IkPoseEdit::Mix { value: mix } => value.setup.mix = mix,
                        IkPoseEdit::BendPositive { value: bend } => {
                            value.setup.bend_positive = bend
                        }
                    }
                }
                Ok(Self {
                    document: apply_command(
                        &self.document,
                        Command::SetIkConstraint { constraint, value },
                    )?,
                    pose: self.pose.clone(),
                })
            }
            EditTarget::Animate { animation, time } => {
                let candidate = Self {
                    document: self.document.clone(),
                    pose: self
                        .pose
                        .edit_ik(&self.document, animation, time, constraint, edits)?,
                };
                if auto_key && !edits.is_empty() {
                    candidate.key_targets(
                        animation,
                        time,
                        KeyTargets {
                            iks: BTreeSet::from([constraint]),
                            ..Default::default()
                        },
                        skin,
                    )
                } else {
                    Ok(candidate)
                }
            }
        }
    }
    /// One pure candidate contains every selected bone/constraint key and removal
    /// of pending overrides. Any conversion/evaluation error publishes nothing.
    fn key_targets(
        &self,
        animation: usize,
        time: f32,
        targets: KeyTargets,
        skin: &str,
    ) -> Result<Self, String> {
        let KeyTargets {
            bones,
            iks,
            sliders,
            deforms,
            slots,
            draw_order,
        } = targets;
        let target = EditTarget::Animate { animation, time };
        let authored = self.authored_pose(target, skin)?;
        let mut commands = vec![];
        for (&bone, properties) in &bones {
            commands.push(Command::SetPose {
                bone,
                target,
                transform: *authored.bone_locals().get(bone).ok_or("Unknown bone")?,
                properties: properties.clone(),
            });
        }
        let key_time = crate::commands::snapped_time(self.document.fps, time)?;
        for &constraint in &iks {
            let value = *authored
                .ik_poses()
                .get(constraint)
                .ok_or("Unknown IK constraint")?;
            let mix = value.mix as f32;
            if !mix.is_finite() {
                return Err("Sampled IK Mix cannot be represented by a finite stored key".into());
            }
            let keys = self.document.animations[animation]
                .ik_tracks
                .iter()
                .find(|t| t.constraint == constraint)
                .map(|t| t.keys.as_slice())
                .unwrap_or(&[]);
            // Existing imported fractional identities take priority over the
            // frame grid; only a genuinely new key follows the snapping policy.
            let existing = match ik_keyframe_at(keys, time)? {
                Some(key) => Some(key),
                None => ik_keyframe_at(keys, key_time)?,
            };
            commands.push(Command::SetIkKey {
                animation,
                constraint,
                key: IkKeyframe {
                    time: existing.map_or(key_time, |key| key.time),
                    mix,
                    bend_positive: value.bend_positive,
                    curve: existing.map_or(IkCurve::Linear, |key| key.curve),
                },
            });
        }
        for (&constraint, properties) in &sliders {
            commands.extend(crate::slider_authoring::key_commands(
                &self.document,
                &authored,
                animation,
                time,
                constraint,
                properties,
            )?);
        }
        for attachment in &deforms {
            commands.push(crate::deform_authoring::key_command(
                &self.document,
                animation,
                time,
                attachment.clone(),
                crate::deform_authoring::offsets(&self.document, &authored, attachment)?,
            )?);
        }
        for (&slot, properties) in &slots {
            commands.extend(crate::slot_authoring::key_commands(
                &self.document,
                &authored,
                animation,
                time,
                slot,
                properties,
            )?);
        }
        if draw_order {
            commands.push(crate::draw_order_authoring::key_command(
                &self.document,
                animation,
                time,
                authored.draw_order(),
            )?);
        }
        let mut candidate = Self {
            document: apply_command(&self.document, Command::Batch { commands })?,
            pose: self.pose.clone(),
        };
        for (bone, properties) in bones {
            candidate.pose.remove(animation, bone, &properties);
        }
        for &constraint in &iks {
            candidate.pose.remove_ik(animation, constraint);
        }
        for (&constraint, properties) in &sliders {
            candidate
                .pose
                .remove_slider(animation, constraint, properties);
        }
        for attachment in &deforms {
            candidate.pose.remove_deform(animation, attachment);
        }
        for (&slot, properties) in &slots {
            candidate.pose.remove_slot(animation, slot, properties);
        }
        if draw_order {
            candidate.pose.remove_draw_order(animation);
            if candidate.authored_pose(target, skin)?.draw_order() != authored.draw_order() {
                return Err("Keying draw order would lose the current pose; keep it pending or use a representable key time".into());
            }
        }
        if !slots.is_empty() {
            crate::slot_authoring::check_key_continuity(
                &authored,
                &candidate.authored_pose(target, skin)?,
                &slots,
            )?;
        }
        if !sliders.is_empty() {
            crate::slider_authoring::check_key_continuity(
                &authored,
                &candidate.authored_pose(target, skin)?,
                &sliders,
            )?;
        }
        if !iks.is_empty() {
            let before = checked_pose_from_authored(&self.document, &authored)?;
            let after = checked_pose_from_authored(
                &candidate.document,
                &candidate.authored_pose(target, skin)?,
            )?;
            crate::ik::check_key_continuity(&self.document, &before, &after)?;
        }
        Ok(candidate)
    }
    fn edit_draw_order(
        &self,
        target: EditTarget,
        order: Vec<usize>,
        auto_key: bool,
        skin: &str,
    ) -> Result<Self, String> {
        let EditTarget::Animate { animation, time } = target else {
            return Err("Draw order pose editing requires Animate mode".into());
        };
        crate::draw_order_authoring::validate_order(self.document.slots.len(), &order)?;
        if self.authored_pose(target, skin)?.draw_order() == order {
            return Ok(self.clone());
        }
        let candidate = Self {
            document: self.document.clone(),
            pose: self
                .pose
                .edit_draw_order(&self.document, animation, time, order)?,
        };
        if auto_key {
            candidate.key_targets(
                animation,
                time,
                KeyTargets {
                    draw_order: true,
                    ..Default::default()
                },
                skin,
            )
        } else {
            Ok(candidate)
        }
    }
    /// Pure pose transition shared by ordinary edits and fixed-base previews.
    fn edit_pose(
        &self,
        bone: usize,
        target: EditTarget,
        transform: Transform,
        properties: &[Property],
        auto_key: bool,
    ) -> Result<Self, String> {
        match target {
            EditTarget::Setup => Ok(Self {
                document: apply_command(
                    &self.document,
                    Command::SetPose {
                        bone,
                        target,
                        transform,
                        properties: properties.to_vec(),
                    },
                )?,
                pose: self.pose.clone(),
            }),
            EditTarget::Animate { animation, time } => {
                let pose =
                    self.pose
                        .edit(&self.document, animation, time, bone, transform, properties)?;
                let mut candidate = WorkingState {
                    document: self.document.clone(),
                    pose,
                };
                if auto_key {
                    let properties = transform_group(properties);
                    let transform =
                        candidate
                            .pose
                            .locals(&candidate.document, Some(animation), time)[bone];
                    candidate.document = apply_command(
                        &candidate.document,
                        Command::SetPose {
                            bone,
                            target,
                            transform,
                            properties: properties.clone(),
                        },
                    )?;
                    candidate.pose.remove(animation, bone, &properties);
                }
                Ok(candidate)
            }
        }
    }
    fn edit_slot_pose(
        &self,
        slot: usize,
        target: EditTarget,
        edits: &[SlotPoseEdit],
        auto_key: bool,
        skin: &str,
    ) -> Result<Self, String> {
        crate::slot_authoring::validate_edits(&self.document, slot, edits)?;
        let current = SlotPose::from_authored(&self.authored_pose(target, skin)?, slot)?;
        let changed: Vec<_> = edits
            .iter()
            .filter(|edit| match edit {
                SlotPoseEdit::Attachment { attachment } => *attachment != current.attachment,
                SlotPoseEdit::Color { color } => *color != current.color,
            })
            .cloned()
            .collect();
        if changed.is_empty() {
            return Ok(self.clone());
        }
        let edits = changed.as_slice();
        match target {
            EditTarget::Setup => {
                let commands = edits
                    .iter()
                    .map(|edit| match edit {
                        SlotPoseEdit::Attachment { attachment } => Ok(Command::SetSlotAttachment {
                            slot,
                            attachment: attachment.clone(),
                        }),
                        SlotPoseEdit::Color { color } => Ok(Command::SetSlotColor {
                            slot,
                            color: crate::slot_color_bytes(*color)?,
                        }),
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                Ok(Self {
                    document: apply_command(&self.document, Command::Batch { commands })?,
                    pose: self.pose.clone(),
                })
            }
            EditTarget::Animate { animation, time } => {
                let candidate = Self {
                    document: self.document.clone(),
                    pose: self.pose.edit_slot(
                        &self.document,
                        animation,
                        time,
                        slot,
                        edits,
                        skin,
                    )?,
                };
                if auto_key && !edits.is_empty() {
                    candidate.key_targets(
                        animation,
                        time,
                        KeyTargets {
                            slots: BTreeMap::from([(
                                slot,
                                edits.iter().map(SlotPoseEdit::property).collect(),
                            )]),
                            ..Default::default()
                        },
                        skin,
                    )
                } else {
                    Ok(candidate)
                }
            }
        }
    }
    fn edit_deform_pose(
        &self,
        attachment: AttachmentRef,
        target: EditTarget,
        offsets: Vec<f32>,
        auto_key: bool,
        skin: &str,
    ) -> Result<Self, String> {
        let EditTarget::Animate { animation, time } = target else {
            return Err("Mesh deform poses require Animate mode".into());
        };
        let canonical =
            crate::deform_authoring::validate_offsets(&self.document, &attachment, &offsets)?;
        let current = self.authored_pose(target, skin)?;
        if crate::deform_authoring::offsets(&self.document, &current, &canonical)? == offsets {
            return Ok(self.clone());
        }
        let posed = Self {
            document: self.document.clone(),
            pose: self.pose.edit_deform(
                &self.document,
                animation,
                time,
                &attachment,
                offsets,
                skin,
            )?,
        };
        let before = posed.authored_pose(target, skin)?;
        crate::deform_authoring::geometry(&posed.document, &before, &attachment)?;
        if auto_key {
            let candidate = posed.key_targets(
                animation,
                time,
                KeyTargets {
                    deforms: vec![canonical],
                    ..Default::default()
                },
                skin,
            )?;
            crate::deform_authoring::check_key_continuity(
                &posed.document,
                &before,
                &candidate.document,
                &candidate.authored_pose(target, skin)?,
                &[attachment],
            )?;
            Ok(candidate)
        } else {
            Ok(posed)
        }
    }
}

pub struct Session {
    state: WorkingState,
    undo: Vec<WorkingState>,
    redo: Vec<WorkingState>,
    saved: Document,
    transaction: Option<WorkingState>,
    /// Active skin for skin-required bones/constraints. View state, outside
    /// history and the document, like the shell's skin selection.
    skin: String,
}

impl Session {
    pub fn new(document: Document) -> Result<Self, String> {
        document.validate()?;
        Ok(Self {
            saved: document.clone(),
            state: WorkingState {
                document,
                pose: PoseEdits::default(),
            },
            undo: vec![],
            redo: vec![],
            transaction: None,
            skin: "default".into(),
        })
    }
    /// Select the skin whose skin-required bones and constraints are evaluated.
    pub fn set_skin(&mut self, skin: &str) {
        if self.skin != skin {
            self.skin = skin.into();
        }
    }
    pub fn skin(&self) -> &str {
        &self.skin
    }
    pub fn document(&self) -> &Document {
        &self.state.document
    }
    pub fn pose_edits(&self) -> &PoseEdits {
        &self.state.pose
    }
    pub fn local_pose(&self, animation: Option<usize>, time: f32) -> Vec<Transform> {
        self.state.pose.locals(self.document(), animation, time)
    }
    /// Only committed animation keys and setup data belong to a saved project.
    pub fn is_dirty(&self) -> bool {
        self.document() != &self.saved
    }
    pub fn mark_saved(&mut self) {
        self.commit_transaction();
        self.saved = self.document().clone();
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
            || self
                .transaction
                .as_ref()
                .is_some_and(|base| base != &self.state)
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
            && self
                .transaction
                .as_ref()
                .is_none_or(|base| base == &self.state)
    }
    /// Apply to a candidate first. Failed commands change neither data nor history.
    pub fn dispatch(&mut self, command: Command) -> Result<(), String> {
        let document = apply_command(self.document(), command)?;
        let pose = self.state.pose.after_document(self.document(), &document)?;
        self.replace(WorkingState { document, pose });
        Ok(())
    }
    /// Read authored setup or Animate+pending data through the checked applied-pose stage.
    /// This snapshots without keying, discarding a pose or changing history.
    pub fn evaluated_pose(&self, target: EditTarget) -> Result<EvaluatedPose, String> {
        checked_pose_from_authored(self.document(), &self.authored_pose(target)?)
    }
    /// Read the full authored state without solving, keying or discarding it.
    pub fn authored_pose(&self, target: EditTarget) -> Result<AuthoredPose, String> {
        self.state.authored_pose(target, &self.skin)
    }
    /// Full sampled/pending slot state, before attachment tint or display quantization.
    pub fn slot_pose(&self, slot: usize, target: EditTarget) -> Result<SlotPose, String> {
        SlotPose::from_authored(&self.authored_pose(target)?, slot)
    }
    /// Authored manual/host-keyed settings. Applied bone-driven time is separate.
    pub fn slider_pose(&self, constraint: usize, target: EditTarget) -> Result<SliderPose, String> {
        if self
            .document()
            .constraints
            .get(constraint)
            .and_then(Constraint::as_slider)
            .is_none()
        {
            return Err("Unknown Slider constraint".into());
        }
        self.authored_pose(target)?
            .slider_poses()
            .get(constraint)
            .copied()
            .ok_or_else(|| "Unknown Slider constraint".into())
    }
    /// Edit only authored Time/Mix. Bone-derived effective values stay separate.
    pub fn edit_slider_pose(
        &mut self,
        constraint: usize,
        target: EditTarget,
        edits: &[SliderPoseEdit],
        auto_key: bool,
    ) -> Result<(), String> {
        let candidate = self
            .state
            .edit_slider_pose(constraint, target, edits, auto_key, &self.skin)?;
        self.replace(candidate);
        Ok(())
    }
    pub fn preview_edit_slider_pose(
        &mut self,
        constraint: usize,
        target: EditTarget,
        edits: &[SliderPoseEdit],
        auto_key: bool,
    ) -> Result<(), String> {
        let base = self.transaction.as_ref().ok_or("No active transaction")?;
        let candidate = base.edit_slider_pose(constraint, target, edits, auto_key, &self.skin)?;
        self.state = candidate;
        Ok(())
    }
    pub fn key_slider_pose(
        &mut self,
        animation: usize,
        time: f32,
        constraint: usize,
        properties: &[SliderProperty],
    ) -> Result<(), String> {
        self.slider_pose(constraint, EditTarget::Animate { animation, time })?;
        if properties.is_empty()
            || properties.iter().collect::<BTreeSet<_>>().len() != properties.len()
        {
            return Err("Choose distinct Slider properties to key".into());
        }
        let candidate = self.state.key_targets(
            animation,
            time,
            KeyTargets {
                sliders: BTreeMap::from([(constraint, properties.to_vec())]),
                ..Default::default()
            },
            &self.skin,
        )?;
        self.replace(candidate);
        Ok(())
    }
    /// Complete sampled plus pending order, without sampling any other pose data.
    /// Setup always reads setup slot order, including hidden/inactive slots.
    pub fn draw_order(&self, target: EditTarget) -> Result<Vec<usize>, String> {
        match target {
            EditTarget::Setup => Ok((0..self.document().slots.len()).collect()),
            EditTarget::Animate { animation, time } => {
                if !time.is_finite() || time < 0. {
                    return Err("Pose time must be finite and nonnegative".into());
                }
                let clip = self
                    .document()
                    .animations
                    .get(animation)
                    .ok_or("Unknown animation")?;
                if let Some(order) = self.pose_edits().pending_draw_order(animation) {
                    crate::draw_order_authoring::validate_order(
                        self.document().slots.len(),
                        order,
                    )?;
                    Ok(order.to_vec())
                } else {
                    Ok(peachy_document::sample_draw_order(
                        &clip.draw_order,
                        self.document().slots.len(),
                        time,
                    ))
                }
            }
        }
    }
    pub fn edit_draw_order(
        &mut self,
        target: EditTarget,
        order: Vec<usize>,
        auto_key: bool,
    ) -> Result<(), String> {
        let candidate = self
            .state
            .edit_draw_order(target, order, auto_key, &self.skin)?;
        self.replace(candidate);
        Ok(())
    }
    pub fn preview_edit_draw_order(
        &mut self,
        target: EditTarget,
        order: Vec<usize>,
        auto_key: bool,
    ) -> Result<(), String> {
        let base = self.transaction.as_ref().ok_or("No active transaction")?;
        self.state = base.edit_draw_order(target, order, auto_key, &self.skin)?;
        Ok(())
    }
    /// Explicit keying creates a hold/reset even without a pending edit.
    pub fn key_draw_order(&mut self, animation: usize, time: f32) -> Result<(), String> {
        let candidate = self.state.key_targets(
            animation,
            time,
            KeyTargets {
                draw_order: true,
                ..Default::default()
            },
            &self.skin,
        )?;
        self.replace(candidate);
        Ok(())
    }
    pub fn edit_slot_pose(
        &mut self,
        slot: usize,
        target: EditTarget,
        edits: &[SlotPoseEdit],
        auto_key: bool,
    ) -> Result<(), String> {
        let candidate = self
            .state
            .edit_slot_pose(slot, target, edits, auto_key, &self.skin)?;
        self.replace(candidate);
        Ok(())
    }
    /// Recompute a picker/hex preview from the entire fixed transaction baseline.
    pub fn preview_edit_slot_pose(
        &mut self,
        slot: usize,
        target: EditTarget,
        edits: &[SlotPoseEdit],
        auto_key: bool,
    ) -> Result<(), String> {
        let base = self.transaction.as_ref().ok_or("No active transaction")?;
        let candidate = base.edit_slot_pose(slot, target, edits, auto_key, &self.skin)?;
        self.state = candidate;
        Ok(())
    }
    pub fn key_slot_pose(
        &mut self,
        animation: usize,
        time: f32,
        slot: usize,
        properties: &[SlotProperty],
    ) -> Result<(), String> {
        self.document().slots.get(slot).ok_or("Unknown slot")?;
        let candidate = self.state.key_targets(
            animation,
            time,
            KeyTargets {
                slots: BTreeMap::from([(slot, properties.to_vec())]),
                ..Default::default()
            },
            &self.skin,
        )?;
        self.replace(candidate);
        Ok(())
    }
    /// Dense sampled offsets, including pending mesh edits and linked-mesh following.
    pub fn deform_offsets(
        &self,
        attachment: &AttachmentRef,
        target: EditTarget,
    ) -> Result<Vec<f32>, String> {
        crate::deform_authoring::offsets(self.document(), &self.authored_pose(target)?, attachment)
    }
    /// Store an unkeyed mesh pose, or key it explicitly when Auto Key is enabled.
    /// Setup geometry, UVs and binding data are never modified by this operation.
    pub fn edit_deform_pose(
        &mut self,
        attachment: AttachmentRef,
        target: EditTarget,
        offsets: Vec<f32>,
        auto_key: bool,
    ) -> Result<(), String> {
        let candidate = self
            .state
            .edit_deform_pose(attachment, target, offsets, auto_key, &self.skin)?;
        self.replace(candidate);
        Ok(())
    }
    pub fn preview_edit_deform_pose(
        &mut self,
        attachment: AttachmentRef,
        target: EditTarget,
        offsets: Vec<f32>,
        auto_key: bool,
    ) -> Result<(), String> {
        let base = self.transaction.as_ref().ok_or("No active transaction")?;
        let candidate = base.edit_deform_pose(attachment, target, offsets, auto_key, &self.skin)?;
        self.state = candidate;
        Ok(())
    }
    /// Move one posed vertex by a world delta through checked engine inverses.
    pub fn translate_deform_vertex(
        &mut self,
        attachment: AttachmentRef,
        vertex: usize,
        target: EditTarget,
        world_delta: [f32; 2],
        auto_key: bool,
    ) -> Result<(), String> {
        let authored = self.authored_pose(target)?;
        let offsets = crate::deform_authoring::translated_vertex(
            self.document(),
            &authored,
            &attachment,
            vertex,
            world_delta,
        )?;
        self.edit_deform_pose(attachment, target, offsets, auto_key)
    }
    /// Recompute the entire drag from its transaction's frozen authored pose.
    /// Callers freeze attachment, vertex, clip/time and Auto Key for the gesture.
    pub fn preview_translate_deform_vertex(
        &mut self,
        attachment: AttachmentRef,
        vertex: usize,
        target: EditTarget,
        world_delta: [f32; 2],
        auto_key: bool,
    ) -> Result<(), String> {
        let base = self.transaction.as_ref().ok_or("No active transaction")?;
        let authored = base.authored_pose(target, &self.skin)?;
        let offsets = crate::deform_authoring::translated_vertex(
            &base.document,
            &authored,
            &attachment,
            vertex,
            world_delta,
        )?;
        let candidate = base.edit_deform_pose(attachment, target, offsets, auto_key, &self.skin)?;
        self.state = candidate;
        Ok(())
    }
    pub fn key_deform_pose(
        &mut self,
        animation: usize,
        time: f32,
        attachment: AttachmentRef,
    ) -> Result<(), String> {
        let (_, canonical, _) = crate::deform_authoring::mesh_target(self.document(), &attachment)?;
        let candidate = self.state.key_targets(
            animation,
            time,
            KeyTargets {
                deforms: vec![canonical.clone()],
                ..Default::default()
            },
            &self.skin,
        )?;
        self.check_deform_key_candidate(
            &candidate,
            EditTarget::Animate { animation, time },
            &[canonical],
        )?;
        self.replace(candidate);
        Ok(())
    }
    fn check_deform_key_candidate(
        &self,
        candidate: &WorkingState,
        target: EditTarget,
        deforms: &[AttachmentRef],
    ) -> Result<(), String> {
        crate::deform_authoring::check_key_continuity(
            self.document(),
            &self.authored_pose(target)?,
            &candidate.document,
            &candidate.authored_pose(target, &self.skin)?,
            deforms,
        )
    }
    pub fn edit_ik_pose(
        &mut self,
        constraint: usize,
        target: EditTarget,
        edits: &[IkPoseEdit],
        auto_key: bool,
    ) -> Result<(), String> {
        let candidate = self
            .state
            .edit_ik_pose(constraint, target, edits, auto_key, &self.skin)?;
        self.replace(candidate);
        Ok(())
    }
    /// Full WorkingState base, including both kinds of pending values; no UI cache.
    pub fn preview_edit_ik_pose(
        &mut self,
        constraint: usize,
        target: EditTarget,
        edits: &[IkPoseEdit],
        auto_key: bool,
    ) -> Result<(), String> {
        let base = self.transaction.as_ref().ok_or("No active transaction")?;
        let candidate = base.edit_ik_pose(constraint, target, edits, auto_key, &self.skin)?;
        self.state = candidate;
        Ok(())
    }
    pub fn key_ik_pose(
        &mut self,
        animation: usize,
        time: f32,
        constraint: usize,
    ) -> Result<(), String> {
        let candidate = self.state.key_targets(
            animation,
            time,
            KeyTargets {
                iks: BTreeSet::from([constraint]),
                ..Default::default()
            },
            &self.skin,
        )?;
        self.replace(candidate);
        Ok(())
    }
    /// Capture applied locals once, including pending edits. Binding reconstructs
    /// FK from this explicit snapshot and must never apply constraints again.
    pub fn mesh_bind_pose(&self, target: EditTarget) -> Result<MeshBindPose, String> {
        let authored = self.authored_pose(target)?;
        let evaluated = checked_pose_from_authored(self.document(), &authored)?;
        Ok(MeshBindPose {
            locals: evaluated.applied_locals().to_vec(),
            inherits: Some(evaluated.inherits().to_vec()),
        })
    }
    /// Bind at the explicit setup or posed animation state without keying or discarding it.
    pub fn bind_mesh_to_bone(
        &mut self,
        attachment: AttachmentRef,
        bone: usize,
        target: EditTarget,
    ) -> Result<(), String> {
        let pose = self.mesh_bind_pose(target)?;
        self.dispatch(Command::BindMeshToBone {
            attachment,
            bone,
            pose,
        })
    }
    /// Remove every influence, retaining current geometry in the slot bone's local space.
    pub fn unbind_mesh(
        &mut self,
        attachment: AttachmentRef,
        target: EditTarget,
    ) -> Result<(), String> {
        let pose = self.mesh_bind_pose(target)?;
        self.dispatch(Command::UnbindMesh { attachment, pose })
    }
    /// Append a genuinely new zero-weight binding at the explicit current pose.
    pub fn add_mesh_bone_binding(
        &mut self,
        attachment: AttachmentRef,
        bone: usize,
        target: EditTarget,
    ) -> Result<(), String> {
        let pose = self.mesh_bind_pose(target)?;
        self.dispatch(Command::AddMeshBoneBinding {
            attachment,
            bone,
            pose,
        })
    }
    /// Set one vertex/bone weight without keying, rebinding or discarding pending poses.
    pub fn set_mesh_bone_weight(
        &mut self,
        attachment: AttachmentRef,
        vertex: usize,
        bone: usize,
        weight: f32,
        target: EditTarget,
    ) -> Result<(), String> {
        let pose = self.mesh_bind_pose(target)?;
        self.dispatch(Command::SetMeshBoneWeight {
            attachment,
            vertex,
            bone,
            weight,
            pose,
        })
    }
    pub fn set_mesh_bone_weights(
        &mut self,
        selection: MeshWeightSelection,
        weight: f32,
        target: EditTarget,
    ) -> Result<(), String> {
        let pose = self.mesh_bind_pose(target)?;
        self.dispatch(Command::EditMeshBoneWeights {
            attachment: selection.attachment,
            vertices: selection.vertices,
            bone: selection.bone,
            edit: MeshWeightEdit::Absolute { weight },
            pose,
        })
    }
    /// Publish a candidate gesture only against its original transaction source.
    pub fn preview_mesh_bone_weights(&mut self, gesture: &MeshWeightGesture) -> Result<(), String> {
        let base = self.transaction.as_ref().ok_or("No active transaction")?;
        if !gesture.matches_baseline(&base.document) {
            return Err("Weight gesture source changed".into());
        }
        let document = crate::commands::apply_prepared(&base.document, |doc| gesture.apply(doc))?;
        let pose = base.pose.after_document(&base.document, &document)?;
        self.state = WorkingState { document, pose };
        Ok(())
    }
    /// Pose with Auto Key on or off through the same validation/undo boundary.
    pub fn edit_pose(
        &mut self,
        bone: usize,
        target: EditTarget,
        transform: Transform,
        properties: &[Property],
        auto_key: bool,
    ) -> Result<(), String> {
        let candidate = self
            .state
            .edit_pose(bone, target, transform, properties, auto_key)?;
        self.replace(candidate);
        Ok(())
    }
    /// Recompute a pose gesture from its transaction's fixed document and pending pose.
    /// Requires an active transaction. Auto Key off never creates animation keys;
    /// failures preserve the previous valid preview, baseline, saved state and history.
    /// Commit creates at most one Undo entry; cancel restores the exact base and redo.
    pub fn preview_edit_pose(
        &mut self,
        bone: usize,
        target: EditTarget,
        transform: Transform,
        properties: &[Property],
        auto_key: bool,
    ) -> Result<(), String> {
        let base = self.transaction.as_ref().ok_or("No active transaction")?;
        let candidate = base.edit_pose(bone, target, transform, properties, auto_key)?;
        self.state = candidate;
        Ok(())
    }
    /// Explicit key buttons and L always key, independent of Auto Key.
    pub fn key_pose(
        &mut self,
        animation: usize,
        time: f32,
        bone: usize,
        properties: &[Property],
    ) -> Result<(), String> {
        let candidate = self.state.key_targets(
            animation,
            time,
            KeyTargets {
                bones: BTreeMap::from([(bone, properties.to_vec())]),
                ..Default::default()
            },
            &self.skin,
        )?;
        self.replace(candidate);
        Ok(())
    }
    /// K keys every edited bone, IK/Slider constraint, mesh deform, slot property and
    /// global draw order atomically. History retains the pre-key pending pose;
    /// native Spine's observed Undo re-evaluation instead clears that pending pose.
    /// A Tree bone's orange key button supplies an explicit bone instead.
    pub fn key_edited(
        &mut self,
        animation: usize,
        time: f32,
        bone: Option<usize>,
    ) -> Result<(), String> {
        let targets = self.state.pose.targets(animation, bone);
        let iks = if bone.is_none() {
            self.state.pose.ik_targets(animation)
        } else {
            BTreeSet::new()
        };
        let deforms = if bone.is_none() {
            self.state.pose.deform_targets(animation)
        } else {
            vec![]
        };
        let candidate = self.state.key_targets(
            animation,
            time,
            KeyTargets {
                bones: targets,
                iks,
                sliders: if bone.is_none() {
                    self.state.pose.slider_targets(animation)
                } else {
                    BTreeMap::new()
                },
                deforms: deforms.clone(),
                slots: if bone.is_none() {
                    self.state.pose.slot_targets(animation)
                } else {
                    BTreeMap::new()
                },
                draw_order: bone.is_none() && self.state.pose.has_draw_order(animation),
            },
            &self.skin,
        )?;
        self.check_deform_key_candidate(
            &candidate,
            EditTarget::Animate { animation, time },
            &deforms,
        )?;
        self.replace(candidate);
        Ok(())
    }
    /// Moving the timeline drops unkeyed edits. With no redo branch, Undo recovers
    /// that pose at the new position, matching Spine's documented recovery workflow.
    /// Clip/mode changes pass false and remove stale pose snapshots from history.
    pub fn discard_pose(&mut self, recoverable: bool) {
        self.commit_transaction();
        if recoverable && self.redo.is_empty() {
            let mut next = self.state.clone();
            next.pose = PoseEdits::default();
            self.replace(next);
        } else {
            self.state.pose = PoseEdits::default();
            for state in self.undo.iter_mut().chain(&mut self.redo) {
                state.pose = PoseEdits::default();
            }
        }
    }
    pub fn undo(&mut self) {
        if self.transaction.is_some() {
            self.cancel_transaction();
            return;
        }
        if let Some(previous) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.state, previous));
        }
    }
    pub fn redo(&mut self) {
        self.commit_transaction();
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.state, next));
        }
    }
    /// UI gestures may make many changes but create one undo entry, keyed or unkeyed.
    pub fn begin_transaction(&mut self) {
        if self.transaction.is_none() {
            self.transaction = Some(self.state.clone());
        }
    }
    /// Recompute a gesture preview from its fixed baseline, including overwritten keys.
    /// A failed preview leaves the last valid preview and history intact.
    pub fn preview_transaction(&mut self, command: Command) -> Result<(), String> {
        let base = self.transaction.as_ref().ok_or("No active transaction")?;
        let document = apply_command(&base.document, command)?;
        let pose = base.pose.after_document(&base.document, &document)?;
        self.state = WorkingState { document, pose };
        Ok(())
    }
    pub fn commit_transaction(&mut self) {
        if let Some(previous) = self.transaction.take()
            && previous != self.state
        {
            self.record(previous);
        }
    }
    pub fn cancel_transaction(&mut self) {
        if let Some(previous) = self.transaction.take() {
            self.state = previous;
        }
    }
    fn replace(&mut self, candidate: WorkingState) {
        if candidate != self.state {
            let previous = std::mem::replace(&mut self.state, candidate);
            if self.transaction.is_none() {
                self.record(previous);
            }
        }
    }
    fn record(&mut self, previous: WorkingState) {
        self.undo.push(previous);
        if self.undo.len() > 128 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
}
