//! Clipping attachments cut later draws to a polygon. A concave polygon is
//! split into triangles (ear clipping); a `convex` clip uses the convex hull.
//! Each drawn triangle is clipped against each convex piece (Sutherland–
//! Hodgman), so the visible area is exactly triangle ∩ polygon. The runtime's
//! own triangulation may differ, so checks compare covered area, not vertices.
use serde::Serialize;

/// Geometry drawn in place of an attachment's own triangles while clipped.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ClippedGeometry {
    pub vertices: Vec<[f32; 2]>,
    pub uvs: Vec<[f32; 2]>,
    pub triangles: Vec<u32>,
}

/// An active clip: convex pieces (counter-clockwise) and the inclusive end slot.
#[derive(Clone, Debug)]
pub(crate) struct ClipRegion {
    pieces: Vec<Vec<[f64; 2]>>,
    pub(crate) end: Option<usize>,
}

fn cross(o: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

fn signed_area(points: &[[f64; 2]]) -> f64 {
    let n = points.len();
    (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.
}

fn convex_hull(points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut sorted = points.to_vec();
    sorted.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    sorted.dedup();
    if sorted.len() < 3 {
        return sorted;
    }
    let mut hull: Vec<[f64; 2]> = vec![];
    for pass in 0..2 {
        let start = hull.len();
        let iter: Box<dyn Iterator<Item = &[f64; 2]>> = if pass == 0 {
            Box::new(sorted.iter())
        } else {
            Box::new(sorted.iter().rev())
        };
        for &p in iter {
            while hull.len() >= start + 2
                && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.
            {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
    }
    hull
}

fn is_convex(points: &[[f64; 2]]) -> bool {
    let n = points.len();
    (0..n).all(|i| cross(points[i], points[(i + 1) % n], points[(i + 2) % n]) >= 0.)
}

/// Ear clipping of a simple counter-clockwise polygon into triangles.
fn triangulate(points: &[[f64; 2]]) -> Vec<Vec<[f64; 2]>> {
    let mut index: Vec<usize> = (0..points.len()).collect();
    let mut out = vec![];
    let mut guard = 0;
    while index.len() > 3 && guard < points.len() * points.len() {
        guard += 1;
        let n = index.len();
        let ear = (0..n).find(|&i| {
            let (a, b, c) = (
                points[index[(i + n - 1) % n]],
                points[index[i]],
                points[index[(i + 1) % n]],
            );
            cross(a, b, c) > 0.
                && index.iter().all(|&j| {
                    let p = points[j];
                    p == a
                        || p == b
                        || p == c
                        || !(cross(a, b, p) >= 0. && cross(b, c, p) >= 0. && cross(c, a, p) >= 0.)
                })
        });
        let Some(i) = ear else { break };
        let n = index.len();
        out.push(vec![
            points[index[(i + n - 1) % n]],
            points[index[i]],
            points[index[(i + 1) % n]],
        ]);
        index.remove(i);
    }
    if index.len() == 3 {
        out.push(index.iter().map(|&i| points[i]).collect());
    }
    out.retain(|t: &Vec<[f64; 2]>| signed_area(t) > 0.);
    out
}

impl ClipRegion {
    pub(crate) fn new(polygon: &[[f32; 2]], convex: bool, end: Option<usize>) -> Self {
        let mut points: Vec<[f64; 2]> = polygon
            .iter()
            .map(|p| [f64::from(p[0]), f64::from(p[1])])
            .collect();
        if signed_area(&points) < 0. {
            points.reverse();
        }
        let pieces = if convex {
            vec![convex_hull(&points)]
        } else if is_convex(&points) {
            vec![points]
        } else {
            triangulate(&points)
        };
        Self {
            pieces: pieces.into_iter().filter(|p| p.len() >= 3).collect(),
            end,
        }
    }

    /// Clip triangulated geometry; UVs are interpolated from each source triangle.
    pub(crate) fn clip(
        &self,
        vertices: &[[f32; 2]],
        uvs: &[[f32; 2]],
        triangles: &[u32],
    ) -> ClippedGeometry {
        let mut out = ClippedGeometry::default();
        for t in triangles.as_chunks::<3>().0 {
            let corner = |k: usize| {
                let v = vertices[t[k] as usize];
                [f64::from(v[0]), f64::from(v[1])]
            };
            let (a, b, c) = (corner(0), corner(1), corner(2));
            let area = cross(a, b, c);
            if area == 0. {
                continue;
            }
            for piece in &self.pieces {
                let mut polygon = vec![a, b, c];
                if area < 0. {
                    polygon.reverse();
                }
                for i in 0..piece.len() {
                    let (p, q) = (piece[i], piece[(i + 1) % piece.len()]);
                    let input = std::mem::take(&mut polygon);
                    for j in 0..input.len() {
                        let (s, e) = (input[j], input[(j + 1) % input.len()]);
                        let (ds, de) = (cross(p, q, s), cross(p, q, e));
                        if ds >= 0. {
                            polygon.push(s);
                        }
                        if (ds >= 0.) != (de >= 0.) {
                            let k = ds / (ds - de);
                            polygon.push([s[0] + (e[0] - s[0]) * k, s[1] + (e[1] - s[1]) * k]);
                        }
                    }
                    if polygon.len() < 3 {
                        break;
                    }
                }
                if polygon.len() < 3 {
                    continue;
                }
                let base = out.vertices.len() as u32;
                for point in &polygon {
                    // Barycentric UVs from the source triangle.
                    let wa = cross(*point, b, c) / area;
                    let wb = cross(a, *point, c) / area;
                    let wc = 1. - wa - wb;
                    let uv = |k: usize| uvs[t[k] as usize].map(f64::from);
                    let (ua, ub, uc) = (uv(0), uv(1), uv(2));
                    out.vertices.push([point[0] as f32, point[1] as f32]);
                    out.uvs.push([
                        (ua[0] * wa + ub[0] * wb + uc[0] * wc) as f32,
                        (ua[1] * wa + ub[1] * wb + uc[1] * wc) as f32,
                    ]);
                }
                for k in 1..polygon.len() as u32 - 1 {
                    out.triangles.extend([base, base + k, base + k + 1]);
                }
            }
        }
        out
    }
}
