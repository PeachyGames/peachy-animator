//! Evaluated geometry shared by rendering, fit-to-view and future headless export.
use crate::{Affine, local_pose, world_pose};
use peachy_document::{Document, Transform};

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

#[derive(Clone, Debug)]
pub struct RegionDraw {
    pub slot: usize,
    pub image: String,
    /// Bottom-left, bottom-right, top-right, top-left in world space.
    pub vertices: [[f32; 2]; 4],
}
pub struct Scene {
    pub locals: Vec<Transform>,
    pub world: Vec<Affine>,
    pub regions: Vec<RegionDraw>,
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
    let world = world_pose(doc, &locals);
    let mut regions = vec![];
    let mut bounds: Option<Bounds> = None;
    for (slot_index, slot) in doc.slots.iter().enumerate() {
        let Some(region) = doc.region(skin, slot_index) else {
            continue;
        };
        let matrix = world[slot.bone].then(Affine::from_transform(region.transform));
        let (w, h) = (region.width * 0.5, region.height * 0.5);
        let vertices = [[-w, -h], [w, -h], [w, h], [-w, h]].map(|p| matrix.point(p[0], p[1]));
        for point in vertices {
            let bounds = bounds.get_or_insert(Bounds {
                min: point,
                max: point,
            });
            for (i, value) in point.into_iter().enumerate() {
                bounds.min[i] = bounds.min[i].min(value);
                bounds.max[i] = bounds.max[i].max(value);
            }
        }
        regions.push(RegionDraw {
            slot: slot_index,
            image: region.path.clone(),
            vertices,
        });
    }
    Scene {
        locals,
        world,
        regions,
        bounds,
    }
}
