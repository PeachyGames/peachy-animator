//! Authored and applied poses are separate. FK reconstruction never solves IK.
use crate::{Affine, IkPose, local_pose, sample_ik, world_pose};
use peachy_document::{Constraint, Document, Transform, finite_transform};

#[derive(Clone, Debug)]
pub struct EvaluatedPose {
    pub(crate) authored_locals: Vec<Transform>,
    pub(crate) applied_locals: Vec<Transform>,
    pub(crate) world: Vec<Affine>,
    pub(crate) authored_ik: Vec<IkPose>,
}

/// Complete authored state. Bone-only input cannot silently select setup IK.
#[derive(Clone, Debug, PartialEq)]
pub struct AuthoredPose {
    pub(crate) locals: Vec<Transform>,
    pub(crate) ik: Vec<IkPose>,
}

impl AuthoredPose {
    pub fn bone_locals(&self) -> &[Transform] {
        &self.locals
    }
    pub fn ik_poses(&self) -> &[IkPose] {
        &self.ik
    }
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
    pub fn ik_poses(&self) -> &[IkPose] {
        &self.authored_ik
    }
}

pub fn sample_authored_pose(
    doc: &Document,
    animation: Option<usize>,
    time: f32,
) -> Result<AuthoredPose, String> {
    if !time.is_finite() || time < 0.0 {
        return Err("Pose time must be finite and nonnegative".into());
    }
    let clip = animation
        .map(|index| doc.animations.get(index).ok_or("Unknown animation"))
        .transpose()?;
    let ik = doc
        .constraints
        .iter()
        .enumerate()
        .map(|(index, constraint)| {
            let Constraint::Ik(ik) = constraint;
            match clip.and_then(|clip| clip.ik_tracks.iter().find(|t| t.constraint == index)) {
                Some(track) => sample_ik(track, ik.setup, time),
                None => Ok(ik.setup.into()),
            }
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(AuthoredPose {
        locals: local_pose(doc, animation, time),
        ik,
    })
}

/// The document is validated, as in Session. Queries never alter authored data.
pub fn checked_pose_from_authored(
    doc: &Document,
    authored: &AuthoredPose,
) -> Result<EvaluatedPose, String> {
    let authored_locals = &authored.locals;
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
    if authored.ik.len() != doc.constraints.len()
        || authored.ik.iter().any(|ik| !ik.mix.is_finite())
    {
        return Err("Evaluation needs finite authored IK settings for every constraint".into());
    }
    for (bone, local) in doc.bones.iter().zip(authored_locals) {
        if !finite_transform(*local) {
            return Err(format!(
                "Bone {}: evaluated local transform is nonfinite or outside the f32 range",
                bone.name
            ));
        }
    }
    let mut applied_locals = authored_locals.clone();
    for constraint in &doc.constraints {
        let Constraint::Ik(ik) = constraint;
        ik.validate(&doc.bones)?;
    }
    let mut world = world_pose(doc, &applied_locals);
    check_world(doc, &world)?;
    crate::ik_solver::apply(doc, &authored.ik, &mut applied_locals, &mut world)?;
    Ok(EvaluatedPose {
        authored_locals: authored_locals.clone(),
        applied_locals,
        world,
        authored_ik: authored.ik.clone(),
    })
}

/// Deliberately Setup-only: callers with animation/pending state use AuthoredPose.
pub fn checked_setup_pose_from_locals(
    doc: &Document,
    locals: Vec<Transform>,
) -> Result<EvaluatedPose, String> {
    let mut authored = sample_authored_pose(doc, None, 0.)?;
    authored.locals = locals;
    checked_pose_from_authored(doc, &authored)
}

pub fn checked_evaluate_pose(
    doc: &Document,
    animation: Option<usize>,
    time: f32,
) -> Result<EvaluatedPose, String> {
    checked_pose_from_authored(doc, &sample_authored_pose(doc, animation, time)?)
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

/// Key conversion must retain actual evaluated geometry, not merely a finite Mix.
pub(crate) fn check_key_continuity(
    doc: &Document,
    before: &EvaluatedPose,
    after: &EvaluatedPose,
) -> Result<(), String> {
    for ((bone, before), after) in doc.bones.iter().zip(before.world()).zip(after.world()) {
        let components = |m: &Affine| {
            let tip = m.point(bone.length, 0.);
            [m.a, m.b, m.c, m.d, m.x, m.y, tip[0], tip[1]]
        };
        if components(before)
            .into_iter()
            .zip(components(after))
            .any(|(a, b)| {
                !a.is_finite()
                    || !b.is_finite()
                    || (f64::from(a) - f64::from(b)).abs()
                        > crate::ik_solver::IK_WORLD_COORDINATE_TOLERANCE
            })
        {
            return Err(format!(
                "Keying IK would move bone {} beyond coordinate precision (0.001); keep the pending pose or choose a representable key time/value",
                bone.name
            ));
        }
    }
    Ok(())
}
