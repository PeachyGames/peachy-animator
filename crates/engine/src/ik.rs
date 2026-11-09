//! Authored and applied poses are separate. FK reconstruction never solves IK.
use crate::{Affine, local_pose, world_pose};
use peachy_document::{Document, Transform, finite_transform};

#[derive(Clone, Debug)]
pub struct EvaluatedPose {
    pub(crate) authored_locals: Vec<Transform>,
    pub(crate) applied_locals: Vec<Transform>,
    pub(crate) world: Vec<Affine>,
}

impl EvaluatedPose {
    pub fn authored_locals(&self) -> &[Transform] {
        &self.authored_locals
    }
    pub fn applied_locals(&self) -> &[Transform] {
        &self.applied_locals
    }
    pub fn world(&self) -> &[Affine] {
        &self.world
    }
}

/// The document is validated, as in Session. Queries never alter authored data.
pub fn checked_pose_from_locals(
    doc: &Document,
    authored_locals: Vec<Transform>,
) -> Result<EvaluatedPose, String> {
    if authored_locals.len() != doc.bones.len() {
        return Err("Evaluation needs one local transform per bone".into());
    }
    if doc
        .bones
        .iter()
        .enumerate()
        .any(|(i, bone)| bone.parent.is_some_and(|parent| parent >= i))
    {
        return Err("Evaluation requires a parent-first bone hierarchy".into());
    }
    for (bone, local) in doc.bones.iter().zip(&authored_locals) {
        if !finite_transform(*local) {
            return Err(format!(
                "Bone {}: evaluated local transform is nonfinite or outside the f32 range",
                bone.name
            ));
        }
    }
    let mut applied_locals = authored_locals.clone();
    // Physics constraints are validated authored data. This slice deliberately
    // has no editor simulation: they leave the evaluated pose unchanged, matching
    // the external runtime with physics disabled. See docs/slices/physics-data.md.
    for constraint in &doc.constraints {
        constraint.validate(&doc.bones)?;
    }
    let mut world = world_pose(doc, &applied_locals);
    check_world(doc, &world)?;
    crate::ik_solver::apply(doc, &mut applied_locals, &mut world)?;
    Ok(EvaluatedPose {
        authored_locals,
        applied_locals,
        world,
    })
}

pub fn checked_evaluate_pose(
    doc: &Document,
    animation: Option<usize>,
    time: f32,
) -> Result<EvaluatedPose, String> {
    if !time.is_finite() || time < 0.0 {
        return Err("Pose time must be finite and nonnegative".into());
    }
    if animation.is_some_and(|index| index >= doc.animations.len()) {
        return Err("Unknown animation".into());
    }
    checked_pose_from_locals(doc, local_pose(doc, animation, time))
}

pub(crate) fn check_world(doc: &Document, world: &[Affine]) -> Result<(), String> {
    for (bone, matrix) in doc.bones.iter().zip(world) {
        if ![matrix.a, matrix.b, matrix.c, matrix.d, matrix.x, matrix.y]
            .into_iter()
            .all(f32::is_finite)
        {
            return Err(format!(
                "Bone {}: evaluated world transform is nonfinite or outside the f32 range",
                bone.name
            ));
        }
    }
    Ok(())
}
