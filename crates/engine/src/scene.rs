//! Evaluated geometry shared by rendering, fit-to-view and future headless export.
use crate::{
    Affine, local_pose,
    mesh::{REGION_TRIANGLES, REGION_UVS, region_vertices},
    world_pose,
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
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
        ]
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
                region_vertices(r).to_vec(),
                REGION_UVS.to_vec(),
                REGION_TRIANGLES.to_vec(),
                4,
            ),
            Attachment::Mesh(m) => (
                m.vertices.clone(),
                m.uvs.clone(),
                m.triangles.clone(),
                m.hull,
            ),
        };
        let vertices: Vec<_> = vertices
            .into_iter()
            .map(|p| world[slot.bone].point(p[0], p[1]))
            .collect();
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
