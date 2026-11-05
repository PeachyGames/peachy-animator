//! Pure attachment geometry and topology edits; no renderer or file dependencies.
use crate::Affine;
use peachy_document::{Mesh, Region};

pub const REGION_UVS: [[f32; 2]; 4] = [[0., 1.], [1., 1.], [1., 0.], [0., 0.]];
pub const REGION_TRIANGLES: [u32; 6] = [0, 1, 2, 0, 2, 3];

pub fn region_vertices(region: &Region) -> [[f32; 2]; 4] {
    let (w, h) = (region.width * 0.5, region.height * 0.5);
    let matrix = Affine::from_transform(region.transform);
    [[-w, -h], [w, -h], [w, h], [-w, h]].map(|p| matrix.point(p[0], p[1]))
}

pub fn region_to_mesh(region: &Region) -> Mesh {
    Mesh {
        name: region.name.clone(),
        path: region.path.clone(),
        vertices: region_vertices(region).to_vec(),
        uvs: REGION_UVS.to_vec(),
        triangles: REGION_TRIANGLES.to_vec(),
        hull: 4,
        width: region.width,
        height: region.height,
        edges: vec![0, 2, 2, 4, 4, 6, 6, 0],
    }
}

/// Insert a triangle centroid without changing the hull, UV mapping or winding.
/// Existing vertex indices remain stable. This is a local subdivision, not Delaunay.
pub fn split_triangle(mesh: &mut Mesh, triangle: usize) -> Result<(), String> {
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
    let position = average(&mesh.vertices);
    let uv = average(&mesh.uvs);
    let vertex = u32::try_from(mesh.vertices.len()).map_err(|_| "Too many vertices")?;
    mesh.vertices.push(position);
    mesh.uvs.push(uv);
    mesh.triangles
        .splice(start..start + 3, [a, b, vertex, b, c, vertex, c, a, vertex]);
    Ok(())
}
