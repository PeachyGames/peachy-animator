//! Pure attachment geometry and topology edits; no renderer or file dependencies.
use crate::Affine;
use peachy_document::{BoneInfluence, Mesh, MeshVertices, Region};

pub const REGION_UVS: [[f32; 2]; 4] = [[0., 1.], [1., 1.], [1., 0.], [0., 0.]];
pub const REGION_TRIANGLES: [u32; 6] = [0, 1, 2, 0, 2, 3];

pub fn region_vertices(region: &Region) -> [[f32; 2]; 4] {
    let (w, h) = (region.width * 0.5, region.height * 0.5);
    let matrix = Affine::from_transform(region.transform.into());
    [[-w, -h], [w, -h], [w, h], [-w, h]].map(|p| matrix.point(p[0], p[1]))
}

pub fn region_to_mesh(region: &Region) -> Mesh {
    Mesh {
        name: region.name.clone(),
        path: region.path.clone(),
        vertices: MeshVertices::SlotLocal {
            positions: region_vertices(region).to_vec(),
        },
        uvs: REGION_UVS.to_vec(),
        triangles: REGION_TRIANGLES.to_vec(),
        hull: 4,
        width: region.width,
        height: region.height,
        edges: vec![0, 2, 2, 4, 4, 6, 6, 0],
        color: region.color,
    }
}

/// Evaluate validated bind positions using the current world bone transforms.
/// Weighted positions already name their bones; the slot transform is not applied again.
pub fn mesh_world_vertices(mesh: &Mesh, slot_bone: usize, world: &[Affine]) -> Vec<[f32; 2]> {
    match &mesh.vertices {
        MeshVertices::SlotLocal { positions } => positions
            .iter()
            .map(|position| world[slot_bone].point(position[0], position[1]))
            .collect(),
        MeshVertices::Weighted { influences } => influences
            .iter()
            .map(|vertex| weighted_world_vertex(vertex, world))
            .collect(),
    }
}

/// World vertices with deform offsets: per vertex for unweighted meshes, per
/// influence (added to its bone-local position) for weighted meshes.
pub fn deformed_mesh_world_vertices(
    mesh: &Mesh,
    slot_bone: usize,
    world: &[Affine],
    offsets: &[f32],
) -> Vec<[f32; 2]> {
    if offsets.len() != mesh.deform_len() {
        return mesh_world_vertices(mesh, slot_bone, world);
    }
    match &mesh.vertices {
        MeshVertices::SlotLocal { positions } => positions
            .iter()
            .zip(offsets.as_chunks::<2>().0)
            .map(|(p, d)| world[slot_bone].point(p[0] + d[0], p[1] + d[1]))
            .collect(),
        MeshVertices::Weighted { influences } => {
            let mut values = offsets.as_chunks::<2>().0.iter();
            influences
                .iter()
                .map(|vertex| {
                    let mut point = [0.0, 0.0];
                    for influence in vertex {
                        let d = values.next().unwrap_or(&[0.0, 0.0]);
                        let transformed = world[influence.bone]
                            .point(influence.position[0] + d[0], influence.position[1] + d[1]);
                        for axis in 0..2 {
                            point[axis] += influence.weight * transformed[axis];
                        }
                    }
                    point
                })
                .collect()
        }
    }
}

/// The same f32 evaluation is used for rendering and candidate weight validation.
pub(super) fn weighted_world_vertex(vertex: &[BoneInfluence], world: &[Affine]) -> [f32; 2] {
    let mut point = [0.0, 0.0];
    for influence in vertex {
        let transformed = world[influence.bone].point(influence.position[0], influence.position[1]);
        for axis in 0..2 {
            point[axis] += influence.weight * transformed[axis];
        }
    }
    point
}

/// Insert a triangle centroid without changing the hull, UV mapping or winding.
/// Existing vertex indices remain stable. This is a local subdivision, not Delaunay.
pub fn split_triangle(mesh: &mut Mesh, triangle: usize) -> Result<(), String> {
    let MeshVertices::SlotLocal { positions } = &mut mesh.vertices else {
        return Err("Weighted mesh subdivision is not implemented".into());
    };
    let start = triangle.checked_mul(3).ok_or("Unknown triangle")?;
    let t = mesh
        .triangles
        .get(start..start.saturating_add(3))
        .ok_or("Unknown triangle")?;
    let [a, b, c] = [t[0], t[1], t[2]];
    let average = |points: &[[f32; 2]]| {
        [0, 1].map(|axis| {
            (points[a as usize][axis] + points[b as usize][axis] + points[c as usize][axis]) / 3.0
        })
    };
    let position = average(positions);
    let uv = average(&mesh.uvs);
    let vertex = u32::try_from(positions.len()).map_err(|_| "Too many vertices")?;
    positions.push(position);
    mesh.uvs.push(uv);
    mesh.triangles
        .splice(start..start + 3, [a, b, vertex, b, c, vertex, c, a, vertex]);
    Ok(())
}
