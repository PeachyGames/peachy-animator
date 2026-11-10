//! Mesh deform keys. Offsets are dense: two values per vertex for unweighted
//! meshes (added to setup positions) and two per bone influence for weighted
//! meshes (added to each influence's bone-local position before weighting), as
//! observed with the external 4.3.13 runtime (docs/slices/deform-keys.md).
use crate::{Attachment, AttachmentRef, Curve, Document, Mesh, MeshVertices};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeformTrack {
    pub attachment: AttachmentRef,
    pub keys: Vec<DeformKey>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeformKey {
    pub time: f32,
    /// Interpolation to the next key; Bezier values are progress fractions.
    pub curve: Curve,
    pub offsets: Vec<f32>,
}

impl MeshVertices {
    /// Number of deform values: two per vertex, or two per weighted influence.
    pub fn deform_len(&self) -> usize {
        2 * match self {
            MeshVertices::SlotLocal { positions } => positions.len(),
            MeshVertices::Weighted { influences } => influences.iter().map(Vec::len).sum(),
        }
    }
}

impl Mesh {
    pub fn deform_len(&self) -> usize {
        self.vertices.deform_len()
    }
}

impl Attachment {
    /// Deformable vertices: meshes, paths and clipping polygons.
    pub fn vertices(&self) -> Option<&MeshVertices> {
        match self {
            Attachment::Mesh(mesh) => Some(&mesh.vertices),
            Attachment::Path(path) => Some(&path.vertices),
            Attachment::Clipping(clip) => Some(&clip.vertices),
            Attachment::Region(_) => None,
        }
    }
}

impl Document {
    pub(crate) fn validate_deform(&self, animation: &crate::Animation) -> Result<(), String> {
        let mut seen = std::collections::BTreeSet::new();
        for track in &animation.deform_tracks {
            let key = &track.attachment;
            let Some(vertices) = self.attachment(key).and_then(Attachment::vertices) else {
                return Err(format!(
                    "Animation {}: deform keys need an existing mesh, path or clipping polygon {}/{}/{}",
                    animation.name, key.skin, key.slot, key.placeholder
                ));
            };
            if !seen.insert((key.skin.clone(), key.slot.clone(), key.placeholder.clone())) {
                return Err(format!(
                    "Animation {}: duplicate deform track",
                    animation.name
                ));
            }
            let mut previous = -1.0;
            for deform in &track.keys {
                if !deform.time.is_finite() || deform.time < 0.0 || deform.time <= previous {
                    return Err("Deform keys must be finite, nonnegative and ordered".into());
                }
                previous = deform.time;
                if deform.offsets.len() != vertices.deform_len()
                    || deform.offsets.iter().any(|v| !v.is_finite())
                {
                    return Err(format!(
                        "Animation {}: deform key at {} needs {} finite offsets for {}/{}",
                        animation.name,
                        deform.time,
                        vertices.deform_len(),
                        key.slot,
                        key.placeholder
                    ));
                }
                if !deform.curve.handles_valid() {
                    return Err("Bezier handles must be finite".into());
                }
            }
        }
        Ok(())
    }
}
