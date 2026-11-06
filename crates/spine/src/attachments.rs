use super::*;

pub(super) fn read_attachment(
    value: &Value,
    placeholder: &str,
    bone_count: usize,
) -> Result<Attachment, String> {
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
            let vertices = read_mesh_vertices(&value["vertices"], uvs.len(), bone_count)?;
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
    if let Attachment::Mesh(mesh) = &attachment {
        mesh.validate_bones(bone_count)?;
    }
    Ok(attachment)
}

fn read_mesh_vertices(
    value: &Value,
    vertex_count: usize,
    bone_count: usize,
) -> Result<MeshVertices, String> {
    let raw = value.as_array().ok_or("Mesh vertices must be an array")?;
    let coordinate_count = vertex_count
        .checked_mul(2)
        .ok_or("Too many mesh vertices")?;
    if raw.len() == coordinate_count {
        return Ok(MeshVertices::SlotLocal {
            positions: pairs(value, "Mesh vertices")?,
        });
    }
    if raw.len() < coordinate_count {
        return Err("Mesh vertex count does not match UVs".into());
    }
    let mut influences = Vec::with_capacity(vertex_count);
    let mut cursor = 0_usize;
    for _ in 0..vertex_count {
        let count = integer(
            raw.get(cursor).ok_or("Missing weighted vertex")?,
            "Influence count",
        )?;
        if count == 0 {
            return Err("A weighted vertex needs at least one influence".into());
        }
        cursor = cursor.checked_add(1).ok_or("Too many mesh influences")?;
        let end = count
            .checked_mul(4)
            .and_then(|size| cursor.checked_add(size))
            .ok_or("Too many mesh influences")?;
        let groups = raw
            .get(cursor..end)
            .ok_or("Truncated weighted vertex influences")?;
        let mut vertex = Vec::with_capacity(count);
        for group in groups.as_chunks::<4>().0 {
            let bone = integer(&group[0], "Influence bone")?;
            if bone >= bone_count {
                return Err("Influence references an unknown bone".into());
            }
            vertex.push(BoneInfluence {
                bone,
                position: [
                    finite_number(&group[1], "Influence position")?,
                    finite_number(&group[2], "Influence position")?,
                ],
                weight: finite_number(&group[3], "Influence weight")?,
            });
        }
        influences.push(vertex);
        cursor = end;
    }
    if cursor != raw.len() {
        return Err("Trailing weighted vertex data".into());
    }
    Ok(MeshVertices::Weighted { influences })
}

fn integer(value: &Value, context: &str) -> Result<usize, String> {
    let invalid = || format!("{context} must be a nonnegative integer within platform bounds");
    if let Some(number) = value.as_u64() {
        return usize::try_from(number).map_err(|_| invalid());
    }
    // JSON may spell integer-valued counts and indices as 1.0. Use an exact,
    // exclusive power-of-two bound: usize::MAX rounds up when cast to f64 on 64-bit.
    let upper_bound = 2.0_f64.powi(usize::BITS as i32);
    value
        .as_f64()
        .filter(|number| {
            number.is_finite() && *number >= 0.0 && number.fract() == 0.0 && *number < upper_bound
        })
        .map(|number| number as usize)
        .ok_or_else(invalid)
}

fn finite_number(value: &Value, context: &str) -> Result<f32, String> {
    value
        .as_f64()
        .map(|n| n as f32)
        .filter(|n| n.is_finite())
        .ok_or_else(|| format!("{context} must be finite numbers"))
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
            Ok([
                finite_number(&p[0], context)?,
                finite_number(&p[1], context)?,
            ])
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
            let vertices = match &m.vertices {
                MeshVertices::SlotLocal { positions } => {
                    json!(positions.iter().flatten().collect::<Vec<_>>())
                }
                MeshVertices::Weighted { influences } => {
                    let mut values = vec![];
                    for vertex in influences {
                        values.push(json!(vertex.len()));
                        for influence in vertex {
                            values.extend([
                                json!(influence.bone),
                                json!(influence.position[0]),
                                json!(influence.position[1]),
                                json!(influence.weight),
                            ]);
                        }
                    }
                    json!(values)
                }
            };
            let mut v = json!({"type":"mesh", "vertices":vertices, "uvs":m.uvs.iter().flatten().collect::<Vec<_>>(), "triangles":m.triangles, "hull":m.hull});
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

pub(super) fn inspect_attachment(
    region: &Value,
    slot: &str,
    name: &str,
    bone_count: usize,
    issues: &mut Vec<String>,
) {
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
    if let Err(error) = read_attachment(region, name, bone_count) {
        issues.push(format!("{context}: {error}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_tokens_keep_exact_integer_maximum_but_reject_the_float_upper_bound() {
        assert_eq!(integer(&json!(usize::MAX), "test").unwrap(), usize::MAX);
        let upper_bound = 2.0_f64.powi(usize::BITS as i32);
        assert!(integer(&json!(upper_bound), "test").is_err());
        assert!(integer(&json!(upper_bound * 2.0), "test").is_err());
        assert!(integer(&json!(-1.0), "test").is_err());
        assert!(integer(&json!(1.5), "test").is_err());
    }
}
