//! Frozen, evaluated drawing frames for explicit multi-bone translation.
//!
//! This module never samples or solves a pose, and never publishes a Session edit.
//! Numeric assignment is separate: viewport translation suppresses selected
//! descendants, while a numeric assignment can deliberately edit every member.
use crate::{
    Affine, Axes, BonePoseEdit, EvaluatedPose, TransformGesture, TransformTool,
    drag_transform_in_frame, transform_axes_from_world,
};
use peachy_document::{Document, Inherit, Property, Transform, finite_transform};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug)]
struct BoneFrame {
    bone: usize,
    authored: Transform,
    applied: Transform,
    parent: Affine,
    world: Affine,
    inherit: Inherit,
    edit: bool,
}

/// One immutable drawing-frame snapshot. Selection order and logical primary
/// stay distinct from the effective Translate anchor and filtered edit roots.
/// Build this once from the same checked pose used to draw the scene, then send
/// its explicit results to `Session::preview_edit_poses` against a fixed baseline.
#[derive(Clone, Debug)]
pub struct BoneTransformFrame {
    members: Vec<usize>,
    selection_primary: usize,
    primary_frame: usize,
    frames: Vec<BoneFrame>,
    edit_bones: Vec<usize>,
}

impl BoneTransformFrame {
    /// Capture authored values and already applied frames without another solve.
    /// The supplied document and evaluated pose must belong to the same state.
    pub fn from_evaluated(
        doc: &Document,
        pose: &EvaluatedPose,
        bones: &[usize],
        primary: usize,
    ) -> Result<Self, String> {
        // Validate the relevant snapshot, without revalidating dense attachment
        // domains or solving a second pose just to draw a transform control.
        let count = doc.bones.len();
        if [
            pose.authored_locals().len(),
            pose.applied_locals().len(),
            pose.world().len(),
            pose.inherits().len(),
            pose.active_bones().len(),
        ]
        .into_iter()
        .any(|length| length != count)
        {
            return Err("Bone transform layout must match the evaluated pose".into());
        }
        if bones.is_empty() {
            return Err("A bone transform frame requires at least one selected bone".into());
        }
        let mut selected = BTreeSet::new();
        for &bone in bones {
            if bone >= doc.bones.len() {
                return Err(format!("Unknown selected bone {bone}"));
            }
            if !selected.insert(bone) {
                return Err(format!("Duplicate selected bone {bone}"));
            }
        }
        if !selected.contains(&primary) {
            return Err("The transform primary must be a selected bone".into());
        }
        let highest_selected_ancestor = |bone: usize| -> Result<usize, String> {
            let mut highest = bone;
            let mut ancestor = doc.bones[bone].parent;
            let mut remaining = count;
            while let Some(parent) = ancestor {
                if remaining == 0 {
                    return Err("Selected bone ancestry contains a cycle".into());
                }
                remaining -= 1;
                let definition = doc
                    .bones
                    .get(parent)
                    .ok_or("Unknown selected bone ancestor")?;
                if selected.contains(&parent) {
                    highest = parent;
                }
                ancestor = definition.parent;
            }
            Ok(highest)
        };
        let effective_primary = highest_selected_ancestor(primary)?;
        let identity = Affine::from_transform(Transform::default());
        let mut frames = Vec::with_capacity(bones.len());
        for &bone in bones {
            if !pose.active_bones()[bone] {
                return Err(format!(
                    "Selected bone {} is inactive in this skin",
                    doc.bones[bone].name
                ));
            }
            let effective = highest_selected_ancestor(bone)?;
            let frame = BoneFrame {
                bone,
                authored: pose.authored_locals()[bone],
                applied: pose.applied_locals()[bone],
                parent: doc.bones[bone]
                    .parent
                    .map_or(identity, |parent| pose.world()[parent]),
                world: pose.world()[bone],
                inherit: pose.inherits()[bone],
                edit: effective == bone,
            };
            if !finite_transform(frame.authored)
                || !finite_transform(frame.applied)
                || !finite_frame(frame.parent)
                || !finite_frame(frame.world)
            {
                return Err(format!(
                    "Selected bone {} requires a finite authored and evaluated frame",
                    doc.bones[bone].name
                ));
            }
            frames.push(frame);
        }
        let primary_frame = frames
            .iter()
            .position(|frame| frame.bone == effective_primary)
            .ok_or("The effective transform primary must be selected")?;
        let edit_bones = frames
            .iter()
            .filter(|frame| frame.edit)
            .map(|frame| frame.bone)
            .collect();
        Ok(Self {
            members: bones.to_vec(),
            selection_primary: primary,
            primary_frame,
            frames,
            edit_bones,
        })
    }

    pub fn members(&self) -> &[usize] {
        &self.members
    }
    /// Ordered selected roots edited by a viewport gesture. Selected descendants
    /// inherit their ancestor's result without a second authored-local change.
    pub fn edit_bones(&self) -> &[usize] {
        &self.edit_bones
    }
    pub fn selection_primary(&self) -> usize {
        self.selection_primary
    }
    /// Highest selected ancestor of the logical primary, used for Translate drawing.
    pub fn primary(&self) -> usize {
        self.frames[self.primary_frame].bone
    }
    pub fn primary_authored(&self) -> Transform {
        self.frames[self.primary_frame].authored
    }
    pub fn primary_applied(&self) -> Transform {
        self.frames[self.primary_frame].applied
    }
    pub fn primary_parent(&self) -> Affine {
        self.frames[self.primary_frame].parent
    }
    pub fn primary_world(&self) -> Affine {
        self.frames[self.primary_frame].world
    }
    pub fn primary_inherit(&self) -> Inherit {
        self.frames[self.primary_frame].inherit
    }

    /// Return explicit authored-local results in filtered selection order.
    /// Only multi-bone Translate is calibrated here. One selected bone retains
    /// the existing single-bone operation and its exact arithmetic.
    pub fn drag(&self, gesture: TransformGesture) -> Result<Vec<BonePoseEdit>, String> {
        if self.members.len() == 1 {
            let frame = self.frames[0];
            let transform = drag_transform_in_frame(
                frame.authored,
                frame.applied,
                frame.parent,
                frame.world,
                gesture,
            )?;
            let properties = match gesture.tool {
                TransformTool::Translate => vec![Property::X, Property::Y],
                TransformTool::Rotate => vec![Property::Rotation],
                TransformTool::Scale => vec![Property::ScaleX, Property::ScaleY],
                TransformTool::Shear => match gesture.axis {
                    Some(0) => vec![Property::ShearX],
                    Some(1) => vec![Property::ShearY],
                    _ => return Err("Shear requires an explicit X or Y handle".into()),
                },
            };
            return Ok(vec![BonePoseEdit {
                bone: frame.bone,
                transform,
                properties,
            }]);
        }
        if gesture.tool != TransformTool::Translate {
            return Err("Multi-bone Rotate, Scale and Shear gestures are not yet supported".into());
        }
        if gesture.axis.is_some_and(|axis| axis > 1)
            || !gesture
                .start
                .into_iter()
                .chain(gesture.end)
                .all(f32::is_finite)
        {
            return Err("Invalid multi-bone translation gesture".into());
        }
        let primary = self.frames[self.primary_frame];
        let mut delta = [
            gesture.end[0] - gesture.start[0],
            gesture.end[1] - gesture.start[1],
        ];
        if let Some(axis) = gesture.axis {
            let basis = transform_axes_from_world(
                primary.applied,
                primary.parent,
                primary.world,
                gesture.axes,
                gesture.tool,
            )[axis];
            if basis == [0., 0.] || !basis.into_iter().all(f32::is_finite) {
                return Err("The primary translation axis cannot be represented".into());
            }
            let distance = delta[0] * basis[0] + delta[1] * basis[1];
            delta = [basis[0] * distance, basis[1] * distance];
        }
        let common = match gesture.axes {
            Axes::World => delta,
            Axes::Parent | Axes::Local => {
                let local = primary
                    .parent
                    .inverse_vector(delta[0], delta[1])
                    .ok_or("Cannot translate through the primary parent frame")?;
                if gesture.axes == Axes::Local {
                    rotate(local, -primary.applied.rotation)
                } else {
                    local
                }
            }
        };
        self.translate(gesture.axes, common)
    }

    /// Translate by an explicit delta expressed in the chosen axes, without a
    /// pointer gesture or another solve. Parent/Local deltas need no parent
    /// inverse; World deltas invert only the actual edit roots. Selected
    /// descendants remain unchanged in authored space and inherit the result.
    pub fn translate(&self, axes: Axes, delta: [f32; 2]) -> Result<Vec<BonePoseEdit>, String> {
        if !delta.into_iter().all(f32::is_finite) {
            return Err("Translation delta must be finite".into());
        }
        let mut edits = Vec::with_capacity(self.edit_bones.len());
        for frame in &self.frames {
            if !frame.edit {
                continue;
            }
            let translation = match axes {
                Axes::World => frame
                    .parent
                    .inverse_vector(delta[0], delta[1])
                    .ok_or("Cannot translate through a selected bone's parent frame")?,
                Axes::Parent => delta,
                Axes::Local => rotate(delta, frame.applied.rotation),
            };
            let mut transform = frame.authored;
            if translation != [0., 0.] {
                transform.x += translation[0];
                transform.y += translation[1];
            }
            if !finite_transform(transform) {
                return Err(format!(
                    "Translation result for bone {} must be finite",
                    frame.bone
                ));
            }
            edits.push(BonePoseEdit {
                bone: frame.bone,
                transform,
                properties: vec![Property::X, Property::Y],
            });
        }
        Ok(edits)
    }
}

fn rotate(point: [f32; 2], degrees: f32) -> [f32; 2] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    [
        point[0] * cos - point[1] * sin,
        point[0] * sin + point[1] * cos,
    ]
}

fn finite_frame(frame: Affine) -> bool {
    [frame.a, frame.b, frame.c, frame.d, frame.x, frame.y]
        .into_iter()
        .all(f32::is_finite)
}
