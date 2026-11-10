//! `linkedmesh` attachments: resolved after every skin is read (sources may
//! live in later skins), written back as links. Observed with installed Spine
//! 4.3.26 and the external 4.3.13 runtime (docs/slices/linked-mesh.md):
//! `source` names a mesh in the same slot of `skin` (omitted: "default");
//! `timelines` defaults to true; color is the linked mesh's own.
use super::{read_hex_color, unknown_fields};
use peachy_document::{Attachment, Mesh, MeshLink, Skin};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) const FIELDS: &[&str] = &[
    "type",
    "name",
    "path",
    "skin",
    "source",
    "timelines",
    "color",
    "width",
    "height",
];

pub(super) struct Pending {
    pub skin: String,
    pub slot: String,
    pub placeholder: String,
    pub value: Value,
}

pub(super) fn resolve(
    skins: &mut BTreeMap<String, Skin>,
    pending: Vec<Pending>,
) -> Result<(), String> {
    for Pending {
        skin,
        slot,
        placeholder,
        value,
    } in pending
    {
        let context = format!("Linked mesh {skin}/{slot}/{placeholder}");
        let mut issues = vec![];
        unknown_fields(&value, FIELDS, &context, &mut issues);
        if !issues.is_empty() {
            return Err(issues.join("; "));
        }
        let source_skin = match value.get("skin") {
            None => "default".to_owned(),
            Some(v) => v
                .as_str()
                .ok_or_else(|| format!("{context}: skin must be a name"))?
                .to_owned(),
        };
        let source = value["source"]
            .as_str()
            .ok_or_else(|| format!("{context}: source must name a mesh"))?
            .to_owned();
        let timelines = match value.get("timelines") {
            None => true,
            Some(v) => v
                .as_bool()
                .ok_or_else(|| format!("{context}: timelines must be true or false"))?,
        };
        let source_mesh = match skins
            .get(&source_skin)
            .and_then(|s| s.get(&slot))
            .and_then(|a| a.get(&source))
        {
            Some(Attachment::Mesh(mesh)) if mesh.link.is_none() => mesh.clone(),
            _ => {
                return Err(format!(
                    "{context}: source {source_skin}/{source} must be a mesh (not a linked mesh) in the same slot"
                ));
            }
        };
        for (field, shared) in [("width", source_mesh.width), ("height", source_mesh.height)] {
            if let Some(v) = value.get(field)
                && v.as_f64().is_none_or(|v| (v as f32 - shared).abs() > 1e-4)
            {
                return Err(format!("{context}: {field} must match the source mesh"));
            }
        }
        let name = value["name"].as_str().map(str::to_owned);
        let path = value["path"]
            .as_str()
            .or(name.as_deref())
            .unwrap_or(&placeholder)
            .to_owned();
        let mesh = Mesh {
            name,
            path,
            color: read_hex_color(&value, &format!("{context} color"))?,
            link: Some(MeshLink {
                skin: source_skin,
                source,
                timelines,
            }),
            ..source_mesh
        };
        skins
            .get_mut(&skin)
            .ok_or("Unknown skin")?
            .entry(slot)
            .or_default()
            .insert(placeholder, Attachment::Mesh(mesh));
    }
    Ok(())
}

pub(super) fn write(mesh: &Mesh, link: &MeshLink) -> Value {
    let mut value = json!({"type": "linkedmesh", "source": link.source});
    if link.skin != "default" {
        value["skin"] = json!(link.skin);
    }
    if !link.timelines {
        value["timelines"] = json!(false);
    }
    if mesh.width > 0.0 {
        value["width"] = json!(mesh.width);
    }
    if mesh.height > 0.0 {
        value["height"] = json!(mesh.height);
    }
    value
}
