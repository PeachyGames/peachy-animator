//! Spine clipping attachments. While a clipping attachment is shown, the
//! attachments drawn after its slot are cut to its polygon, through `end`
//! (inclusive) or to the end of drawing. Observed with the external 4.3.13
//! runtime's headless renderer (docs/slices/clipping.md).
use crate::MeshVertices;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clipping {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Last clipped slot (inclusive). `None`, or a slot drawn before this
    /// one, clips to the end of drawing.
    pub end: Option<usize>,
    /// Polygon in slot-local or weighted bone-local positions.
    pub vertices: MeshVertices,
    /// Clip to the polygon's convex hull.
    pub convex: bool,
    /// Nonessential polygon color from Spine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 4]>,
}

impl Clipping {
    pub fn vertex_count(&self) -> usize {
        match &self.vertices {
            MeshVertices::SlotLocal { positions } => positions.len(),
            MeshVertices::Weighted { influences } => influences.len(),
        }
    }

    pub fn validate(&self, slot_count: usize, bone_count: usize) -> Result<(), String> {
        let valid = match &self.vertices {
            MeshVertices::SlotLocal { positions } => {
                positions.iter().flatten().all(|v| v.is_finite())
            }
            MeshVertices::Weighted { influences } => influences.iter().all(|vertex| {
                !vertex.is_empty()
                    && vertex.iter().all(|i| {
                        i.bone < bone_count
                            && i.position.iter().all(|v| v.is_finite())
                            && i.weight.is_finite()
                            && (0.0..=1.0).contains(&i.weight)
                    })
            }),
        };
        if !valid || self.vertex_count() < 3 || self.end.is_some_and(|end| end >= slot_count) {
            return Err(
                "Clipping needs at least 3 finite vertices, known bones and a known end slot"
                    .into(),
            );
        }
        Ok(())
    }
}
