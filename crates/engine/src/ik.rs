//! Authored and applied poses are separate. FK reconstruction never solves IK.
use crate::{Affine, IkPose, inherit_pose, local_pose, sample_ik, world_pose_with};
use peachy_document::{
    Activity, AttachmentRef, ColorTrack, Constraint, DeformTrack, Document, Inherit, Transform,
    finite_transform,
};

#[cfg(test)]
thread_local! {
    static CHECKED_POSE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
pub(crate) fn checked_pose_calls() -> usize {
    CHECKED_POSE_CALLS.with(std::cell::Cell::get)
}

#[derive(Clone, Debug)]
pub struct EvaluatedPose {
    pub(crate) authored_locals: Vec<Transform>,
    pub(crate) applied_locals: Vec<Transform>,
    pub(crate) world: Vec<Affine>,
    pub(crate) authored_ik: Vec<IkPose>,
    pub(crate) applied_ik: Vec<IkPose>,
    pub(crate) inherits: Vec<Inherit>,
    pub(crate) paths: Vec<crate::PathPose>,
    pub(crate) authored_sliders: Vec<crate::SliderPose>,
    pub(crate) applied_sliders: Vec<crate::SliderPose>,
    /// Bones active under the evaluated skin (skin-required bones may be off).
    pub(crate) active_bones: Vec<bool>,
    pub(crate) deforms: Vec<(AttachmentRef, Vec<f32>)>,
    pub(crate) slot_attachments: Vec<Option<String>>,
    pub(crate) slot_colors: Vec<[f32; 4]>,
    pub(crate) draw_order: Vec<usize>,
    pub(crate) sequence_frames: Vec<(peachy_document::AttachmentRef, u32)>,
    pub(crate) skin: String,
}

/// One mutable application state for the ordered constraint pass. It starts
/// from the authored capsule once, never from a recursively evaluated clip.
pub(crate) struct AppliedState {
    pub locals: Vec<Transform>,
    pub inherits: Vec<Inherit>,
    pub ik: Vec<IkPose>,
    pub sliders: Vec<crate::SliderPose>,
    pub paths: Vec<crate::PathPose>,
    pub active: Activity,
    pub deforms: Vec<(AttachmentRef, Vec<f32>)>,
    pub slot_attachments: Vec<Option<String>>,
    pub slot_colors: Vec<[f32; 4]>,
    pub draw_order: Vec<usize>,
    pub sequence_frames: Vec<(AttachmentRef, u32)>,
    pub skin: String,
}

impl AppliedState {
    fn from_authored(authored: &AuthoredPose) -> Self {
        Self {
            locals: authored.locals.clone(),
            inherits: authored.inherits.clone(),
            ik: authored.ik.clone(),
            sliders: authored.sliders.clone(),
            paths: authored.paths.clone(),
            active: authored.active.clone(),
            deforms: authored.deforms.clone(),
            slot_attachments: authored.slot_attachments.clone(),
            slot_colors: authored.slot_colors.clone(),
            draw_order: authored.draw_order.clone(),
            sequence_frames: authored.sequence_frames.clone(),
            skin: authored.skin.clone(),
        }
    }
    fn validate_domains(&self, doc: &Document) -> Result<(), String> {
        if self.locals.len() != doc.bones.len()
            || self.inherits.len() != doc.bones.len()
            || self.locals.iter().any(|local| !finite_transform(*local))
            || self.ik.len() != doc.constraints.len()
            || self.ik.iter().any(|pose| !pose.mix.is_finite())
            || self.sliders.len() != doc.constraints.len()
            || self
                .sliders
                .iter()
                .any(|pose| !pose.time.is_finite() || !pose.mix.is_finite())
        {
            return Err(
                "Applied bone and constraint state must match the document and remain finite"
                    .into(),
            );
        }
        if self.paths.len() != doc.constraints.len()
            || self
                .paths
                .iter()
                .any(|path| !path.position.is_finite() || !path.spacing.is_finite())
        {
            return Err("Evaluation needs finite path settings for every constraint".into());
        }
        validate_slot_domains(
            doc,
            &self.slot_attachments,
            &self.slot_colors,
            &self.draw_order,
        )?;
        validate_attachment_domains(doc, &self.deforms, &self.sequence_frames)
    }
}

fn validate_slot_domains(
    doc: &Document,
    attachments: &[Option<String>],
    colors: &[[f32; 4]],
    order: &[usize],
) -> Result<(), String> {
    if attachments.len() != doc.slots.len()
        || colors.len() != doc.slots.len()
        || attachments
            .iter()
            .flatten()
            .any(|name| name.trim().is_empty())
        || colors.iter().flatten().any(|value| !value.is_finite())
    {
        return Err(
            "Evaluation needs finite slot colors and valid attachment state for every slot".into(),
        );
    }
    crate::draw_order_authoring::validate_order(doc.slots.len(), order)
}

fn validate_attachment_domains(
    doc: &Document,
    deforms: &[(AttachmentRef, Vec<f32>)],
    frames: &[(AttachmentRef, u32)],
) -> Result<(), String> {
    for (key, offsets) in deforms {
        let vertices = doc
            .attachment(key)
            .and_then(peachy_document::Attachment::vertices)
            .ok_or("Evaluated deform target is missing or not deformable")?;
        if offsets.len() != vertices.deform_len() || offsets.iter().any(|value| !value.is_finite())
        {
            return Err(
                "Evaluated deform offsets must match their target layout and remain finite".into(),
            );
        }
    }
    for (key, frame) in frames {
        let sequence = doc
            .attachment(key)
            .and_then(peachy_document::Attachment::sequence)
            .ok_or("Evaluated sequence target is missing or not sequenced")?;
        if *frame >= sequence.count {
            return Err("Evaluated sequence frame is outside its attachment's frame range".into());
        }
    }
    Ok(())
}

/// Complete authored state. Bone-only input cannot silently select setup IK.
#[derive(Clone, Debug, PartialEq)]
pub struct AuthoredPose {
    pub(crate) locals: Vec<Transform>,
    pub(crate) ik: Vec<IkPose>,
    /// Manual/host-keyed authored settings; never overwritten with bone-derived time.
    pub(crate) sliders: Vec<crate::SliderPose>,
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
    /// Keyed sequence frame per sequenced attachment (absent = setup frame).
    pub(crate) sequence_frames: Vec<(peachy_document::AttachmentRef, u32)>,
}

impl AuthoredPose {
    pub fn bone_locals(&self) -> &[Transform] {
        &self.locals
    }
    pub fn inherits(&self) -> &[Inherit] {
        &self.inherits
    }
    /// Set activity/context only. This never resamples host tracks or drops
    /// explicit overrides. Use `sample_authored_pose_with_skin` when sampling
    /// an animation for a selected skin before applying pending edits.
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
    pub fn slider_poses(&self) -> &[crate::SliderPose] {
        &self.sliders
    }
    pub fn slot_attachments(&self) -> &[Option<String>] {
        &self.slot_attachments
    }
    /// Sampled unmultiplied float RGBA, without display clamping/quantization.
    pub fn slot_colors(&self) -> &[[f32; 4]] {
        &self.slot_colors
    }
    /// Complete back-to-front slot indices, including hidden and inactive slots.
    pub fn draw_order(&self) -> &[usize] {
        &self.draw_order
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
    pub fn applied_ik_poses(&self) -> &[IkPose] {
        &self.applied_ik
    }
    /// Effective modes used by this pose's FK, captured with its applied locals.
    pub fn inherits(&self) -> &[Inherit] {
        &self.inherits
    }
    pub fn path_poses(&self) -> &[crate::PathPose] {
        &self.paths
    }
    pub fn slot_attachments(&self) -> &[Option<String>] {
        &self.slot_attachments
    }
    /// Effective unmultiplied float color, including hidden/empty slots.
    pub fn slot_colors(&self) -> &[[f32; 4]] {
        &self.slot_colors
    }
    pub fn deforms(&self) -> &[(AttachmentRef, Vec<f32>)] {
        &self.deforms
    }
    pub fn sequence_frames(&self) -> &[(AttachmentRef, u32)] {
        &self.sequence_frames
    }
    pub fn skin(&self) -> &str {
        &self.skin
    }
    pub(crate) fn check_scene_context(&self, doc: &Document, skin: &str) -> Result<(), String> {
        if self.skin != skin {
            return Err("Scene skin must match the already evaluated pose".into());
        }
        if self.world.len() != doc.bones.len()
            || self.applied_locals.len() != doc.bones.len()
            || self.authored_locals.len() != doc.bones.len()
            || self.inherits.len() != doc.bones.len()
            || self.active_bones.len() != doc.bones.len()
            || self.applied_ik.len() != doc.constraints.len()
            || self.authored_ik.len() != doc.constraints.len()
            || self.paths.len() != doc.constraints.len()
            || self.authored_sliders.len() != doc.constraints.len()
            || self.applied_sliders.len() != doc.constraints.len()
        {
            return Err(
                "Scene bone/constraint layout must match the already evaluated pose".into(),
            );
        }
        if self
            .applied_locals
            .iter()
            .chain(&self.authored_locals)
            .any(|local| !finite_transform(*local))
        {
            return Err("Scene local transforms must remain finite".into());
        }
        check_world(doc, &self.world)?;
        validate_slot_domains(
            doc,
            &self.slot_attachments,
            &self.slot_colors,
            &self.draw_order,
        )?;
        validate_attachment_domains(doc, &self.deforms, &self.sequence_frames)
    }
    pub fn authored_slider_poses(&self) -> &[crate::SliderPose] {
        &self.authored_sliders
    }
    pub fn applied_slider_poses(&self) -> &[crate::SliderPose] {
        &self.applied_sliders
    }
    pub fn active_bones(&self) -> &[bool] {
        &self.active_bones
    }
    /// Complete back-to-front slot indices, including hidden and inactive slots.
    pub fn draw_order(&self) -> &[usize] {
        &self.draw_order
    }
}

pub fn sample_authored_pose(
    doc: &Document,
    animation: Option<usize>,
    time: f32,
) -> Result<AuthoredPose, String> {
    sample_authored_pose_with_skin(doc, animation, time, "default")
}

/// Sample the host in its skin context before any explicit pending overrides.
/// Inactive slots retain Setup attachment/color and receive no host Deform or
/// sequence keys. Bone/constraint sampling keeps its established f32 behavior.
pub fn sample_authored_pose_with_skin(
    doc: &Document,
    animation: Option<usize>,
    time: f32,
    skin: &str,
) -> Result<AuthoredPose, String> {
    if !time.is_finite() || time < 0.0 {
        return Err("Pose time must be finite and nonnegative".into());
    }
    let clip = animation
        .map(|index| doc.animations.get(index).ok_or("Unknown animation"))
        .transpose()?;
    let active = doc.activity(skin);
    let slot_active = |slot: usize| active.bones[doc.slots[slot].bone];
    let attachment_active = |key: &AttachmentRef| {
        doc.slots
            .iter()
            .position(|slot| slot.name == key.slot)
            .is_some_and(slot_active)
    };
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
        sliders: crate::slider_authoring::sample_poses(doc, animation, time, &active)?,
        inherits: inherit_pose(doc, animation, time),
        active: active.clone(),
        deforms: clip
            .map(|clip| {
                clip.deform_tracks
                    .iter()
                    .filter_map(|track| {
                        if !attachment_active(&track.attachment) {
                            return None;
                        }
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
                if !slot_active(index) {
                    return slot.attachment.clone();
                }
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
                if !slot_active(index) {
                    return slot.color.map(|byte| f32::from(byte) / 255.);
                }
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
        skin: skin.to_owned(),
        // Like the runtime's non-looping playback, clip time stops at the clip's
        // end: held poses past the end keep the last sequence frame.
        sequence_frames: clip
            .map(|clip| {
                let time = time.min(clip.duration());
                clip.sequence_tracks
                    .iter()
                    .filter_map(|track| {
                        if !attachment_active(&track.attachment) {
                            return None;
                        }
                        let count = doc.attachment(&track.attachment)?.sequence()?.count;
                        let key = track.keys.iter().rfind(|key| key.time <= time)?;
                        Some(
                            key.frame_at_time_checked(f64::from(time), count)
                                .map(|frame| (track.attachment.clone(), frame))
                                .map_err(|error| {
                                    format!(
                                        "Animation {}: sequence {}/{}/{}: {error}",
                                        clip.name,
                                        track.attachment.skin,
                                        track.attachment.slot,
                                        track.attachment.placeholder
                                    )
                                }),
                        )
                    })
                    .collect::<Result<Vec<_>, String>>()
            })
            .transpose()?
            .unwrap_or_default(),
    })
}

/// The document is validated, as in Session. Queries never alter authored data.
pub fn checked_pose_from_authored(
    doc: &Document,
    authored: &AuthoredPose,
) -> Result<EvaluatedPose, String> {
    #[cfg(test)]
    CHECKED_POSE_CALLS.with(|count| count.set(count.get() + 1));
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
    if authored.sliders.len() != doc.constraints.len()
        || authored
            .sliders
            .iter()
            .any(|pose| !pose.time.is_finite() || !pose.mix.is_finite())
    {
        return Err("Evaluation needs finite authored Slider settings for every constraint".into());
    }
    for (bone, local) in doc.bones.iter().zip(authored_locals) {
        if !finite_transform(*local) {
            return Err(format!(
                "Bone {}: evaluated local transform is nonfinite or outside the f32 range",
                bone.name
            ));
        }
    }
    let mut applied = AppliedState::from_authored(authored);
    applied.validate_domains(doc)?;
    // Physics constraints are validated authored data. This slice deliberately
    // has no editor simulation: they leave the evaluated pose unchanged, matching
    // the external runtime with physics disabled. See docs/slices/physics-data.md.
    for constraint in &doc.constraints {
        constraint.validate(&doc.bones)?;
    }
    let mut world = world_pose_with(doc, &applied.locals, &applied.inherits);
    check_inherit_domain(doc, &world, &applied.locals, &applied.inherits)?;
    check_world(doc, &world)?;
    crate::ik_solver::apply(doc, &mut applied, &mut world)?;
    applied.validate_domains(doc)?;
    check_inherit_domain(doc, &world, &applied.locals, &applied.inherits)?;
    check_world(doc, &world)?;
    Ok(EvaluatedPose {
        authored_locals: authored_locals.clone(),
        applied_locals: applied.locals,
        world,
        authored_ik: authored.ik.clone(),
        applied_ik: applied.ik,
        inherits: applied.inherits,
        paths: applied.paths,
        authored_sliders: authored.sliders.clone(),
        applied_sliders: applied.sliders,
        active_bones: applied.active.bones,
        deforms: applied.deforms,
        slot_attachments: applied.slot_attachments,
        slot_colors: applied.slot_colors,
        draw_order: applied.draw_order,
        sequence_frames: applied.sequence_frames,
        skin: applied.skin,
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
pub(crate) fn check_inherit_domain(
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
