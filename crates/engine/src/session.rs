use crate::{
    AuthoredPose, Command, EditTarget, EvaluatedPose, IkPoseEdit, MeshBindPose, PoseEdits,
    apply_command, checked_pose_from_authored, ik_keyframe_at, sample_authored_pose,
    transform_group,
};
use peachy_document::{
    AttachmentRef, Constraint, Document, IkCurve, IkKeyframe, Property, Transform,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, PartialEq)]
struct WorkingState {
    document: Document,
    pose: PoseEdits,
}

impl WorkingState {
    fn authored_pose(&self, target: EditTarget) -> Result<AuthoredPose, String> {
        match target {
            EditTarget::Setup => sample_authored_pose(&self.document, None, 0.),
            EditTarget::Animate { animation, time } => {
                self.pose.authored(&self.document, Some(animation), time)
            }
        }
    }
    fn edit_ik_pose(
        &self,
        constraint: usize,
        target: EditTarget,
        edits: &[IkPoseEdit],
        auto_key: bool,
    ) -> Result<Self, String> {
        crate::authoring::validate_ik_edits(edits)?;
        let Constraint::Ik(ik) = self
            .document
            .constraints
            .get(constraint)
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
                        BTreeMap::new(),
                        BTreeSet::from([constraint]),
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
        bones: BTreeMap<usize, Vec<Property>>,
        iks: BTreeSet<usize>,
    ) -> Result<Self, String> {
        let target = EditTarget::Animate { animation, time };
        let authored = self.authored_pose(target)?;
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
        if !iks.is_empty() {
            let before = checked_pose_from_authored(&self.document, &authored)?;
            let after =
                checked_pose_from_authored(&candidate.document, &candidate.authored_pose(target)?)?;
            crate::ik::check_key_continuity(&self.document, &before, &after)?;
        }
        Ok(candidate)
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
}

pub struct Session {
    state: WorkingState,
    undo: Vec<WorkingState>,
    redo: Vec<WorkingState>,
    saved: Document,
    transaction: Option<WorkingState>,
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
        })
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
        self.replace(WorkingState {
            document,
            pose: self.state.pose.clone(),
        });
        Ok(())
    }
    /// Read authored setup or Animate+pending data through the checked applied-pose stage.
    /// This snapshots without keying, discarding a pose or changing history.
    pub fn evaluated_pose(&self, target: EditTarget) -> Result<EvaluatedPose, String> {
        checked_pose_from_authored(self.document(), &self.authored_pose(target)?)
    }
    /// Read the full authored state without solving, keying or discarding it.
    pub fn authored_pose(&self, target: EditTarget) -> Result<AuthoredPose, String> {
        self.state.authored_pose(target)
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
            .edit_ik_pose(constraint, target, edits, auto_key)?;
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
        let candidate = base.edit_ik_pose(constraint, target, edits, auto_key)?;
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
            BTreeMap::new(),
            BTreeSet::from([constraint]),
        )?;
        self.replace(candidate);
        Ok(())
    }
    /// Capture applied locals once, including pending edits. Binding reconstructs
    /// FK from this explicit snapshot and must never apply constraints again.
    pub fn mesh_bind_pose(&self, target: EditTarget) -> Result<MeshBindPose, String> {
        Ok(MeshBindPose {
            locals: self.evaluated_pose(target)?.applied_locals().to_vec(),
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
            BTreeMap::from([(bone, properties.to_vec())]),
            BTreeSet::new(),
        )?;
        self.replace(candidate);
        Ok(())
    }
    /// K keys every edited bone and IK constraint in one atomic transition.
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
        let candidate = self.state.key_targets(animation, time, targets, iks)?;
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
        self.state = WorkingState {
            document,
            pose: base.pose.clone(),
        };
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
