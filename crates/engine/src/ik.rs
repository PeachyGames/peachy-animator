//! Authored and applied poses are separate. FK reconstruction never solves IK.
use crate::{Affine, IkPose, inherit_pose, local_pose, sample_ik, world_pose_with};
use peachy_document::{
    Activity, AttachmentRef, ColorTrack, Constraint, DeformTrack, Document, Inherit, Transform,
    finite_transform,
};

#[derive(Clone, Debug)]
pub struct EvaluatedPose {
    pub(crate) authored_locals: Vec<Transform>,
    pub(crate) applied_locals: Vec<Transform>,
    pub(crate) world: Vec<Affine>,
    pub(crate) authored_ik: Vec<IkPose>,
    /// Bones active under the evaluated skin (skin-required bones may be off).
    pub(crate) active_bones: Vec<bool>,
    pub(crate) deforms: Vec<(AttachmentRef, Vec<f32>)>,
    pub(crate) slot_attachments: Vec<Option<String>>,
    pub(crate) slot_colors: Vec<[f32; 4]>,
    pub(crate) draw_order: Vec<usize>,
}

/// Complete authored state. Bone-only input cannot silently select setup IK.
#[derive(Clone, Debug, PartialEq)]
pub struct AuthoredPose {
    pub(crate) locals: Vec<Transform>,
    pub(crate) ik: Vec<IkPose>,
    /// Setup or keyed (stepped) inheritance mode per bone.
    pub(crate) inherits: Vec<Inherit>,
    /// Bones and constraints active under the evaluated skin.
    pub(crate) active: Activity,
    /// Sampled deform offsets for keyed meshes (absent = setup vertices).
    pub(crate) deforms: Vec<(AttachmentRef, Vec<f32>)>,
    /// Shown attachment placeholder per slot (Setup or keyed; `None` hidden).
    pub(crate) slot_attachments: Vec<Option<String>>,
    /// Slot color per slot as unpremultiplied 0–1 channels (Setup or keyed).
    pub(crate) slot_colors: Vec<[f32; 4]>,
    /// Slot indices back to front (Setup or keyed draw order).
    pub(crate) draw_order: Vec<usize>,
    /// Sampled position/spacing per constraint (Setup for non-path entries).
    pub(crate) paths: Vec<crate::PathPose>,
    /// Skin used to resolve path attachments.
    pub(crate) skin: String,
}

impl AuthoredPose {
    pub fn bone_locals(&self) -> &[Transform] {
        &self.locals
    }
    pub fn inherits(&self) -> &[Inherit] {
        &self.inherits
    }
    /// Evaluate skin-required bones and constraints for `skin` (the default
    /// skin, or an unknown name, activates none of them).
    pub fn with_skin(mut self, doc: &Document, skin: &str) -> Self {
        self.active = doc.activity(skin);
        self.skin = skin.to_owned();
        self
    }
    pub fn activity(&self) -> &Activity {
        &self.active
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
    pub fn active_bones(&self) -> &[bool] {
        &self.active_bones
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
            // Poses stay indexed by constraint. A physics constraint carries an
            // inactive placeholder that the solver never reads.
            let Constraint::Ik(ik) = constraint else {
                return Ok(IkPose::INACTIVE);
            };
            match clip.and_then(|clip| clip.ik_tracks.iter().find(|t| t.constraint == index)) {
                Some(track) => sample_ik(track, ik.setup, time),
                None => Ok(ik.setup.into()),
            }
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(AuthoredPose {
        locals: local_pose(doc, animation, time),
        ik,
        inherits: inherit_pose(doc, animation, time),
        active: doc.activity("default"),
        deforms: clip
            .map(|clip| {
                clip.deform_tracks
                    .iter()
                    .filter_map(|track| {
                        Some((track.attachment.clone(), sample_deform(track, time)?))
                    })
                    .collect()
            })
            .unwrap_or_default(),
        slot_attachments: doc
            .slots
            .iter()
            .enumerate()
            .map(|(index, slot)| {
                clip.and_then(|clip| clip.attachment_tracks.iter().find(|t| t.slot == index))
                    .and_then(|track| track.sample(time))
                    .map_or_else(|| slot.attachment.clone(), |keyed| keyed.map(str::to_owned))
            })
            .collect(),
        slot_colors: doc
            .slots
            .iter()
            .enumerate()
            .map(|(index, slot)| {
                clip.and_then(|clip| clip.color_tracks.iter().find(|t| t.slot == index))
                    .and_then(|track| sample_color(track, time))
                    .unwrap_or_else(|| slot.color.map(|byte| f32::from(byte) / 255.))
            })
            .collect(),
        draw_order: peachy_document::sample_draw_order(
            clip.map_or(&[][..], |clip| &clip.draw_order),
            doc.slots.len(),
            time,
        ),
        paths: doc
            .constraints
            .iter()
            .enumerate()
            .map(|(index, constraint)| {
                let setup = constraint.as_path().map(|p| p.setup).unwrap_or_default();
                let keyed = |property, fallback: f32| {
                    clip.and_then(|clip| {
                        clip.path_tracks
                            .iter()
                            .find(|t| t.constraint == index && t.property == property)
                    })
                    .and_then(|track| sample_absolute(&track.keys, time))
                    .unwrap_or(fallback)
                };
                crate::PathPose {
                    position: keyed(peachy_document::PathProperty::Position, setup.position),
                    spacing: keyed(peachy_document::PathProperty::Spacing, setup.spacing),
                }
            })
            .collect(),
        skin: "default".into(),
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
    if authored.active.bones.len() != doc.bones.len()
        || authored.active.constraints.len() != doc.constraints.len()
    {
        return Err("Evaluation needs skin activity for every bone and constraint".into());
    }
    if authored.inherits.len() != doc.bones.len() {
        return Err("Evaluation needs one inheritance mode per bone".into());
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
    // Physics constraints are validated authored data. This slice deliberately
    // has no editor simulation: they leave the evaluated pose unchanged, matching
    // the external runtime with physics disabled. See docs/slices/physics-data.md.
    for constraint in &doc.constraints {
        constraint.validate(&doc.bones)?;
    }
    let mut world = world_pose_with(doc, &applied_locals, &authored.inherits);
    check_inherit_domain(doc, &world, &applied_locals, &authored.inherits)?;
    check_world(doc, &world)?;
    crate::ik_solver::apply(
        doc,
        &authored.ik,
        &mut applied_locals,
        &mut world,
        &authored.inherits,
        &authored.active.constraints,
        &crate::ik_solver::PathContext {
            poses: &authored.paths,
            slot_attachments: &authored.slot_attachments,
            skin: &authored.skin,
        },
    )?;
    Ok(EvaluatedPose {
        authored_locals: authored_locals.clone(),
        applied_locals,
        world,
        authored_ik: authored.ik.clone(),
        active_bones: authored.active.bones.clone(),
        deforms: authored.deforms.clone(),
        slot_attachments: authored.slot_attachments.clone(),
        slot_colors: authored.slot_colors.clone(),
        draw_order: authored.draw_order.clone(),
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
                let axis_squared = a * a + c * c;
                axis_squared.is_nan() || axis_squared < crate::INHERIT_MIN_PARENT_AXIS_SQUARED
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

/// Deform offsets at `time`: none before the first key (setup vertices), the
/// last key after the end, and curve-weighted interpolation between keys.
pub(crate) fn sample_deform(track: &DeformTrack, time: f32) -> Option<Vec<f32>> {
    let first = track.keys.first()?;
    if time < first.time {
        return None;
    }
    for pair in track.keys.windows(2) {
        let (left, right) = (&pair[0], &pair[1]);
        if time < right.time {
            let t =
                crate::curve_progress(left.curve, (time - left.time) / (right.time - left.time));
            return Some(
                left.offsets
                    .iter()
                    .zip(&right.offsets)
                    .map(|(a, b)| a + (b - a) * t)
                    .collect(),
            );
        }
    }
    track.keys.last().map(|key| key.offsets.clone())
}

/// Slot color at `time`: none before the first key (Setup), the last key after
/// the end, and per-channel curve interpolation between keys.
pub(crate) fn sample_color(track: &ColorTrack, time: f32) -> Option<[f32; 4]> {
    let first = track.keys.first()?;
    if time < first.time {
        return None;
    }
    let channels = |c: [u8; 4]| c.map(|byte| f32::from(byte) / 255.);
    for pair in track.keys.windows(2) {
        let (left, right) = (&pair[0], &pair[1]);
        if time < right.time {
            let progress = (time - left.time) / (right.time - left.time);
            let (a, b) = (channels(left.color), channels(right.color));
            let mut out = [0.; 4];
            for channel in 0..4 {
                out[channel] =
                    crate::curve_value(left.curves[channel], progress, a[channel], b[channel]);
            }
            return Some(out);
        }
    }
    track.keys.last().map(|key| channels(key.color))
}

/// Absolute keyed value: `None` before the first key (Setup applies), the last
/// value after the end, and curve interpolation between keys.
pub(crate) fn sample_absolute(keys: &[peachy_document::Keyframe], time: f32) -> Option<f32> {
    let first = keys.first()?;
    if time < first.time {
        return None;
    }
    for pair in keys.windows(2) {
        let (left, right) = (pair[0], pair[1]);
        if time < right.time {
            let progress = (time - left.time) / (right.time - left.time);
            return Some(crate::curve_value(
                left.curve,
                progress,
                left.value,
                right.value,
            ));
        }
    }
    keys.last().map(|key| key.value)
}
