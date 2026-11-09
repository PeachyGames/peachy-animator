//! Authored and applied poses are separate. FK reconstruction never solves IK.
use crate::{Affine, inherit_pose, local_pose, world_pose_with};
use peachy_document::{Document, Inherit, Transform, finite_transform};

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
/// Uses each bone's Setup inheritance mode.
pub fn checked_pose_from_locals(
    doc: &Document,
    authored_locals: Vec<Transform>,
) -> Result<EvaluatedPose, String> {
    let inherits: Vec<_> = doc.bones.iter().map(|bone| bone.inherit).collect();
    checked_pose_from_locals_with(doc, authored_locals, &inherits)
}

/// As [`checked_pose_from_locals`], with explicit (keyed) inheritance modes.
pub fn checked_pose_from_locals_with(
    doc: &Document,
    authored_locals: Vec<Transform>,
    inherits: &[Inherit],
) -> Result<EvaluatedPose, String> {
    if inherits.len() != doc.bones.len() {
        return Err("Evaluation needs one inheritance mode per bone".into());
    }
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
    let mut world = world_pose_with(doc, &applied_locals, inherits);
    check_inherit_domain(doc, &world, &applied_locals, inherits)?;
    check_world(doc, &world)?;
    crate::ik_solver::apply(doc, &mut applied_locals, &mut world, inherits)?;
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
    checked_pose_from_locals_with(
        doc,
        local_pose(doc, animation, time),
        &inherit_pose(doc, animation, time),
    )
}

/// Explain degenerate parents that inheritance modes do not cover, before the
/// generic nonfinite-world check. Root bones have no parent frame to inherit.
fn check_inherit_domain(
    doc: &Document,
    world: &[Affine],
    locals: &[Transform],
    inherits: &[Inherit],
) -> Result<(), String> {
    for (index, bone) in doc.bones.iter().enumerate() {
        let (Some(parent), mode) = (bone.parent, inherits[index]) else {
            continue;
        };
        if mode == Inherit::Normal || mode == Inherit::OnlyTranslation {
            continue;
        }
        let p = world[parent];
        let [a, b, c, d] = [p.a, p.b, p.c, p.d].map(f64::from);
        let degenerate = match mode {
            Inherit::NoRotationOrReflection => {
                !(a * a + c * c >= crate::INHERIT_MIN_PARENT_AXIS_SQUARED)
            }
            _ => {
                let r =
                    f64::from(locals[index].rotation) * crate::pose::OBSERVED_DEGREES_TO_RADIANS;
                let (vx, vy) = (a * r.cos() + b * r.sin(), c * r.cos() + d * r.sin());
                vx == 0.0 && vy == 0.0
            }
        };
        if degenerate {
            return Err(format!(
                "Bone {}: {} needs a nondegenerate parent frame; this zero-scale case is unsupported",
                bone.name,
                mode.spine_name()
            ));
        }
    }
    Ok(())
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
