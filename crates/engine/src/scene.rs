//! Evaluated geometry shared by rendering, fit-to-view and future headless export.
use crate::{
    Affine, local_pose,
    mesh::{REGION_TRIANGLES, REGION_UVS, region_vertices},
    mesh_world_vertices, world_pose,
};
use peachy_document::{Attachment, AttachmentRef, Document, Transform, finite_transform};
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
    pub vertices: Vec<[f32; 2]>,
    pub uvs: Vec<[f32; 2]>,
    pub triangles: Vec<u32>,
    pub hull: usize,
}
pub struct Scene {
    pub locals: Vec<Transform>,
    pub world: Vec<Affine>,
    pub attachments: Vec<AttachmentDraw>,
    pub bounds: Option<Bounds>,
}

/// The caller supplies a validated document, as held by Session.
/// A drag preview is transient data; it never mutates the document.
pub fn evaluate_scene(
    doc: &Document,
    animation: Option<usize>,
    time: f32,
    skin: &str,
    preview: Option<(usize, Transform)>,
) -> Scene {
    let mut locals = local_pose(doc, animation, time);
    if let Some((bone, transform)) = preview
        && let Some(local) = locals.get_mut(bone)
    {
        *local = transform;
    }
    scene_from_locals(doc, skin, locals)
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
    let mut locals = local_pose(doc, animation, time);
    if let Some((bone, transform)) = preview {
        *locals.get_mut(bone).ok_or("Unknown preview bone")? = transform;
    }
    checked_scene_from_locals(doc, skin, locals)
}

/// Evaluate already posed locals through a finite-output boundary. In addition
/// to local count/values, check composed matrices and every emitted coordinate
/// before a shell can turn nonfinite numbers into JSON null or invalid drawing.
pub fn checked_scene_from_locals(
    doc: &Document,
    skin: &str,
    locals: Vec<Transform>,
) -> Result<Scene, String> {
    if locals.len() != doc.bones.len() {
        return Err("Evaluation needs one local transform per bone".into());
    }
    for (bone, local) in doc.bones.iter().zip(&locals) {
        if !finite_transform(*local) {
            return Err(format!(
                "Bone {}: evaluated local transform is nonfinite or outside the f32 range",
                bone.name
            ));
        }
    }
    let scene = scene_from_locals(doc, skin, locals);
    for (bone, world) in doc.bones.iter().zip(&scene.world) {
        if ![world.a, world.b, world.c, world.d, world.x, world.y]
            .into_iter()
            .all(f32::is_finite)
        {
            return Err(format!(
                "Bone {}: evaluated world transform is nonfinite or outside the f32 range",
                bone.name
            ));
        }
    }
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

/// Evaluate an already posed rig, including transient multi-bone authoring edits.
pub fn scene_from_locals(doc: &Document, skin: &str, locals: Vec<Transform>) -> Scene {
    assert_eq!(locals.len(), doc.bones.len());
    let world = world_pose(doc, &locals);
    let mut attachments = vec![];
    for (slot_index, slot) in doc.slots.iter().enumerate() {
        let Some((key, attachment)) = doc.resolved_attachment(skin, slot_index) else {
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
                mesh_world_vertices(m, slot.bone, &world),
                m.uvs.clone(),
                m.triangles.clone(),
                m.hull,
            ),
        };
        attachments.push(AttachmentDraw {
            slot: slot_index,
            attachment: key,
            image: attachment.path().into(),
            vertices,
            uvs,
            triangles,
            hull,
        });
    }
    let bounds = crate::attachment_bounds(&attachments);
    Scene {
        locals,
        world,
        attachments,
        bounds,
    }
}
