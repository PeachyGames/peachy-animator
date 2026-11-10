//! Evaluated geometry shared by rendering, fit-to-view and future headless export.
use crate::{
    Affine, AuthoredPose, IkPose, checked_pose_from_authored,
    mesh::{REGION_TRIANGLES, REGION_UVS, region_vertices},
    mesh_world_vertices, sample_authored_pose,
};
use peachy_document::{Attachment, AttachmentRef, Document, Transform};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: [f32; 2],
    pub max: [f32; 2],
}
impl Bounds {
    pub fn width(self) -> f32 {
        self.max[0] - self.min[0]
    }
    pub fn height(self) -> f32 {
        self.max[1] - self.min[1]
    }
    pub fn center(self) -> [f32; 2] {
        [0, 1].map(|axis| {
            let center = (self.min[axis] + self.max[axis]) * 0.5;
            if center.is_finite() {
                center
            } else {
                ((f64::from(self.min[axis]) + f64::from(self.max[axis])) * 0.5) as f32
            }
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct AttachmentDraw {
    pub slot: usize,
    pub attachment: AttachmentRef,
    pub image: String,
    pub color: [u8; 4],
    pub vertices: Vec<[f32; 2]>,
    pub uvs: Vec<[f32; 2]>,
    pub triangles: Vec<u32>,
    pub hull: usize,
}
pub struct Scene {
    pub locals: Vec<Transform>,
    pub applied_locals: Vec<Transform>,
    pub world: Vec<Affine>,
    pub attachments: Vec<AttachmentDraw>,
    pub bounds: Option<Bounds>,
    pub ik_poses: Vec<IkPose>,
    /// Bones active under the scene's skin; inactive bones' slots are not drawn.
    pub active_bones: Vec<bool>,
}

/// The caller supplies a validated document, as held by Session.
/// A drag preview is transient data; it never mutates the document.
pub fn evaluate_scene(
    doc: &Document,
    animation: Option<usize>,
    time: f32,
    skin: &str,
    preview: Option<(usize, Transform)>,
) -> Result<Scene, String> {
    checked_evaluate_scene(doc, animation, time, skin, preview)
}

/// Checked evaluation for shells that serialize, draw or pick a scene.
/// The document must be validated, as in Session. Numeric overflow is reported
/// without clamping geometry or changing document/pending pose data.
pub fn checked_evaluate_scene(
    doc: &Document,
    animation: Option<usize>,
    time: f32,
    skin: &str,
    preview: Option<(usize, Transform)>,
) -> Result<Scene, String> {
    if !time.is_finite() || time < 0.0 {
        return Err("Pose time must be finite and nonnegative".into());
    }
    if animation.is_some_and(|index| index >= doc.animations.len()) {
        return Err("Unknown animation".into());
    }
    let mut authored = sample_authored_pose(doc, animation, time)?;
    if let Some((bone, transform)) = preview {
        *authored
            .locals
            .get_mut(bone)
            .ok_or("Unknown preview bone")? = transform;
    }
    checked_scene_from_authored(doc, skin, &authored)
}

/// Evaluate already posed locals through a finite-output boundary. In addition
/// to local count/values, check composed matrices and every emitted coordinate
/// before a shell can turn nonfinite numbers into JSON null or invalid drawing.
pub fn checked_scene_from_authored(
    doc: &Document,
    skin: &str,
    authored: &AuthoredPose,
) -> Result<Scene, String> {
    let authored = authored.clone().with_skin(doc, skin);
    let pose = checked_pose_from_authored(doc, &authored)?;
    let scene = scene_from_evaluated(doc, skin, pose);
    for draw in &scene.attachments {
        if !draw
            .vertices
            .iter()
            .flatten()
            .all(|coordinate| coordinate.is_finite())
        {
            return Err(format!(
                "Attachment {}/{}: evaluated geometry is nonfinite or outside the f32 range",
                draw.attachment.slot, draw.attachment.placeholder
            ));
        }
        if !draw
            .uvs
            .iter()
            .flatten()
            .all(|coordinate| coordinate.is_finite())
        {
            return Err(format!(
                "Attachment {}/{}: evaluated UVs are nonfinite",
                draw.attachment.slot, draw.attachment.placeholder
            ));
        }
    }
    if let Some(bounds) = scene.bounds
        && !bounds
            .min
            .into_iter()
            .chain(bounds.max)
            .chain(bounds.center())
            .chain([bounds.width(), bounds.height()])
            .all(f32::is_finite)
    {
        return Err("Evaluated bounds are nonfinite or outside the f32 rendering range".into());
    }
    Ok(scene)
}

/// Explicit Setup-only helper. Animated/pending callers pass a complete capsule.
pub fn checked_setup_scene_from_locals(
    doc: &Document,
    skin: &str,
    locals: Vec<Transform>,
) -> Result<Scene, String> {
    let mut authored = sample_authored_pose(doc, None, 0.)?;
    authored.locals = locals;
    checked_scene_from_authored(doc, skin, &authored)
}

fn scene_from_evaluated(doc: &Document, skin: &str, pose: crate::EvaluatedPose) -> Scene {
    let crate::EvaluatedPose {
        authored_locals: locals,
        applied_locals,
        world,
        authored_ik: ik_poses,
        active_bones,
        deforms,
        slot_attachments,
        slot_colors,
        draw_order,
    } = pose;
    let mut attachments = vec![];
    for slot_index in draw_order {
        let slot = &doc.slots[slot_index];
        if !active_bones[slot.bone] {
            continue;
        }
        let Some((key, attachment)) = slot_attachments[slot_index]
            .as_deref()
            .and_then(|name| doc.resolve_attachment(skin, slot_index, name))
        else {
            continue;
        };
        let (vertices, uvs, triangles, hull) = match attachment {
            Attachment::Region(r) => (
                region_vertices(r)
                    .into_iter()
                    .map(|point| world[slot.bone].point(point[0], point[1]))
                    .collect(),
                REGION_UVS.to_vec(),
                REGION_TRIANGLES.to_vec(),
                4,
            ),
            Attachment::Mesh(m) => (
                match deforms
                    .iter()
                    .find(|(target, _)| *target == doc.deform_target(&key))
                {
                    Some((_, offsets)) => {
                        crate::deformed_mesh_world_vertices(m, slot.bone, &world, offsets)
                    }
                    None => mesh_world_vertices(m, slot.bone, &world),
                },
                m.uvs.clone(),
                m.triangles.clone(),
                m.hull,
            ),
        };
        attachments.push(AttachmentDraw {
            slot: slot_index,
            attachment: key,
            image: attachment.path().into(),
            // Slot color times attachment color, as Spine renderers tint.
            color: {
                let tint = attachment.color();
                let mut color = [0; 4];
                for (channel, out) in color.iter_mut().enumerate() {
                    let value = slot_colors[slot_index][channel].clamp(0., 1.)
                        * f32::from(tint[channel])
                        / 255.;
                    *out = (value * 255.).round() as u8;
                }
                color
            },
            vertices,
            uvs,
            triangles,
            hull,
        });
    }
    let bounds = crate::attachment_bounds(&attachments);
    Scene {
        locals,
        applied_locals,
        world,
        attachments,
        bounds,
        ik_poses,
        active_bones,
    }
}
