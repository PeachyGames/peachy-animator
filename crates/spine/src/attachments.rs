use super::*;

pub(super) fn read_attachment(value: &Value, placeholder: &str) -> Result<Attachment, String> {
    let name = value["name"].as_str().map(str::to_owned);
    let path = value["path"]
        .as_str()
        .or(name.as_deref())
        .unwrap_or(placeholder)
        .into();
    let attachment = match value["type"].as_str().unwrap_or("region") {
        "region" => Attachment::Region(Region {
            name,
            path,
            width: number(value, "width", 0.0),
            height: number(value, "height", 0.0),
            transform: transform(value),
        }),
        "mesh" => {
            let uvs = pairs(&value["uvs"], "Mesh UVs")?;
            let vertices = pairs(&value["vertices"], "Mesh vertices")?;
            if vertices.len() != uvs.len() {
                return Err("Weighted meshes are not supported yet".into());
            }
            Attachment::Mesh(Mesh {
                name,
                path,
                vertices,
                uvs,
                triangles: indices(&value["triangles"], "Mesh triangles")?,
                hull: value["hull"]
                    .as_u64()
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or("Mesh hull must be a vertex count")?,
                edges: value
                    .get("edges")
                    .map(|v| indices(v, "Mesh edges"))
                    .transpose()?
                    .unwrap_or_default(),
                width: number(value, "width", 0.0),
                height: number(value, "height", 0.0),
            })
        }
        kind => return Err(format!("Unsupported attachment type {kind}")),
    };
    attachment.validate()?;
    Ok(attachment)
}

fn pairs(value: &Value, context: &str) -> Result<Vec<[f32; 2]>, String> {
    let array = value
        .as_array()
        .ok_or_else(|| format!("{context} must be an array"))?;
    if !array.len().is_multiple_of(2) {
        return Err(format!("{context} must contain coordinate pairs"));
    }
    array
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| {
            let n = |v: &Value| {
                v.as_f64()
                    .map(|n| n as f32)
                    .filter(|n| n.is_finite())
                    .ok_or_else(|| format!("{context} must be finite numbers"))
            };
            Ok([n(&p[0])?, n(&p[1])?])
        })
        .collect()
}
fn indices(value: &Value, context: &str) -> Result<Vec<u32>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{context} must be an array"))?
        .iter()
        .map(|n| {
            n.as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| format!("{context} must be nonnegative integer indices"))
        })
        .collect()
}

pub(super) fn write_attachment(attachment: &Attachment) -> Value {
    let mut value = match attachment {
        Attachment::Region(r) => {
            let mut v = transform_json(r.transform);
            v["width"] = json!(r.width);
            v["height"] = json!(r.height);
            v
        }
        Attachment::Mesh(m) => {
            let mut v = json!({"type":"mesh", "vertices":m.vertices.iter().flatten().collect::<Vec<_>>(), "uvs":m.uvs.iter().flatten().collect::<Vec<_>>(), "triangles":m.triangles, "hull":m.hull});
            if m.width > 0.0 {
                v["width"] = json!(m.width);
            }
            if m.height > 0.0 {
                v["height"] = json!(m.height);
            }
            if !m.edges.is_empty() {
                v["edges"] = json!(m.edges);
            }
            v
        }
    };
    value["path"] = json!(attachment.path());
    if let Some(name) = attachment.name() {
        value["name"] = json!(name);
    }
    value
}

pub(super) fn inspect_attachment(region: &Value, slot: &str, name: &str, issues: &mut Vec<String>) {
    let context = format!("Attachment {slot}/{name}");
    inspect_numbers(
        region,
        &["x", "y", "rotation", "scaleX", "scaleY", "width", "height"],
        &context,
        issues,
    );
    if region.get("sequence").is_some() {
        issues.push(format!("{context}: image sequence"));
    }
    if region.get("color").is_some_and(|v| {
        !v.as_str()
            .is_some_and(|s| s.eq_ignore_ascii_case("ffffffff"))
    }) {
        issues.push(format!("{context}: tint"));
    }
    let fields: &[&str] = match region["type"].as_str().unwrap_or("region") {
        "region" => &[
            "type", "name", "path", "x", "y", "rotation", "scaleX", "scaleY", "width", "height",
            "color",
        ],
        "mesh" => &[
            "type",
            "name",
            "path",
            "vertices",
            "uvs",
            "triangles",
            "hull",
            "edges",
            "width",
            "height",
            "color",
        ],
        kind => {
            issues.push(format!("{context}: unsupported {kind}"));
            return;
        }
    };
    unknown_fields(region, fields, &context, issues);
    if let Err(error) = read_attachment(region, name) {
        issues.push(format!("{context}: {error}"));
    }
}
