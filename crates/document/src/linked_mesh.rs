//! Spine linked meshes. A linked mesh shares its source mesh's bones, vertices,
//! UVs, triangles, hull, edges and size, but has its own name, path (image)
//! and color. Natively it keeps a copy of the source geometry plus a
//! `MeshLink`; validation requires the copy to equal the source, and commands
//! propagate source edits (docs/slices/linked-mesh.md).
use crate::{Attachment, AttachmentRef, Document, Mesh};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshLink {
    /// Skin holding the source mesh (Spine's omitted `skin` is "default").
    pub skin: String,
    /// Source placeholder in the same slot.
    pub source: String,
    /// When true, the source's deform keys apply to this mesh.
    pub timelines: bool,
}

impl Mesh {
    /// Geometry a linked mesh shares with its source.
    pub fn same_geometry(&self, other: &Mesh) -> bool {
        self.vertices == other.vertices
            && self.uvs == other.uvs
            && self.triangles == other.triangles
            && self.hull == other.hull
            && self.edges == other.edges
            && self.width == other.width
            && self.height == other.height
    }

    /// Replace this mesh's shared geometry with `source`'s.
    pub fn copy_geometry_from(&mut self, source: &Mesh) {
        self.vertices = source.vertices.clone();
        self.uvs = source.uvs.clone();
        self.triangles = source.triangles.clone();
        self.hull = source.hull;
        self.edges = source.edges.clone();
        self.width = source.width;
        self.height = source.height;
    }
}

impl Document {
    /// The source mesh of a linked mesh in `slot`, if it exists and is not
    /// itself linked.
    pub fn link_source(&self, slot: &str, link: &MeshLink) -> Option<&Mesh> {
        match self.skins.get(&link.skin)?.get(slot)?.get(&link.source)? {
            Attachment::Mesh(mesh) if mesh.link.is_none() => Some(mesh),
            _ => None,
        }
    }

    /// The attachment whose deform keys drive `key` (its source when linked
    /// with timelines, else itself).
    pub fn deform_target(&self, key: &AttachmentRef) -> AttachmentRef {
        match self.attachment(key) {
            Some(Attachment::Mesh(Mesh {
                link: Some(link), ..
            })) if link.timelines => AttachmentRef {
                skin: link.skin.clone(),
                slot: key.slot.clone(),
                placeholder: link.source.clone(),
            },
            _ => key.clone(),
        }
    }

    pub(crate) fn validate_linked_meshes(&self) -> Result<(), String> {
        for (skin, slots) in &self.skins {
            for (slot, attachments) in slots {
                for (placeholder, attachment) in attachments {
                    let Attachment::Mesh(mesh) = attachment else {
                        continue;
                    };
                    let Some(link) = &mesh.link else {
                        continue;
                    };
                    let context = format!("Linked mesh {skin}/{slot}/{placeholder}");
                    let source = self.link_source(slot, link).ok_or_else(|| {
                        format!(
                            "{context}: source {}/{} must be an unlinked mesh in the same slot",
                            link.skin, link.source
                        )
                    })?;
                    if !mesh.same_geometry(source) {
                        return Err(format!(
                            "{context} shares its source mesh's geometry; edit the source mesh instead"
                        ));
                    }
                }
            }
        }
        for animation in &self.animations {
            for track in &animation.deform_tracks {
                if self.deform_target(&track.attachment) != track.attachment {
                    return Err(format!(
                        "Animation {}: linked mesh {}/{} follows its source's deform keys",
                        animation.name, track.attachment.slot, track.attachment.placeholder
                    ));
                }
            }
        }
        Ok(())
    }

    /// After a command: copy edited source geometry into its linked meshes.
    /// A linked mesh edited on its own is left for validation to refuse.
    pub fn propagate_linked_geometry(&mut self, before: &Document) {
        let mut updates = vec![];
        for (skin, slots) in &self.skins {
            for (slot, attachments) in slots {
                for (placeholder, attachment) in attachments {
                    let Attachment::Mesh(Mesh {
                        link: Some(link), ..
                    }) = attachment
                    else {
                        continue;
                    };
                    let (Some(now), Some(then)) =
                        (self.link_source(slot, link), before.link_source(slot, link))
                    else {
                        continue;
                    };
                    if !now.same_geometry(then) {
                        updates.push((
                            skin.clone(),
                            slot.clone(),
                            placeholder.clone(),
                            now.clone(),
                        ));
                    }
                }
            }
        }
        for (skin, slot, placeholder, source) in updates {
            if let Some(Attachment::Mesh(mesh)) = self
                .skins
                .get_mut(&skin)
                .and_then(|slots| slots.get_mut(&slot))
                .and_then(|attachments| attachments.get_mut(&placeholder))
            {
                mesh.copy_geometry_from(&source);
            }
        }
    }
}
