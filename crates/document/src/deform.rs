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

impl Mesh {
    /// Number of deform values: two per vertex, or two per weighted influence.
    pub fn deform_len(&self) -> usize {
        2 * match &self.vertices {
            MeshVertices::SlotLocal { positions } => positions.len(),
            MeshVertices::Weighted { influences } => influences.iter().map(Vec::len).sum(),
        }
    }
}

impl Document {
    pub(crate) fn validate_deform(&self, animation: &crate::Animation) -> Result<(), String> {
        let mut seen = std::collections::BTreeSet::new();
        for track in &animation.deform_tracks {
            let key = &track.attachment;
            let Some(Attachment::Mesh(mesh)) = self.attachment(key) else {
                return Err(format!(
                    "Animation {}: deform keys need an existing mesh {}/{}/{}",
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
                if deform.offsets.len() != mesh.deform_len()
                    || deform.offsets.iter().any(|v| !v.is_finite())
                {
                    return Err(format!(
                        "Animation {}: deform key at {} needs {} finite offsets for {}/{}",
                        animation.name,
                        deform.time,
                        mesh.deform_len(),
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
