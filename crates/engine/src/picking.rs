//! Geometry queries shared by interactive selection and headless consumers.
use crate::{AttachmentDraw, Bounds};

pub fn attachment_bounds<'a>(
    draws: impl IntoIterator<Item = &'a AttachmentDraw>,
) -> Option<Bounds> {
    let mut bounds: Option<Bounds> = None;
    for &point in draws.into_iter().flat_map(|draw| &draw.vertices) {
        let b = bounds.get_or_insert(Bounds {
            min: point,
            max: point,
        });
        for (axis, value) in point.into_iter().enumerate() {
            b.min[axis] = b.min[axis].min(value);
            b.max[axis] = b.max[axis].max(value);
        }
    }
    bounds
}

/// Topmost triangulated attachment, using slot draw order. Alpha is not sampled.
pub fn pick_attachment(draws: &[AttachmentDraw], point: [f32; 2]) -> Option<&AttachmentDraw> {
    if point.iter().any(|v| !v.is_finite()) {
        return None;
    }
    draws.iter().rev().find(|draw| {
        draw.triangles.as_chunks::<3>().0.iter().any(|t| {
            let [a, b, c] = t.map(|i| draw.vertices[i as usize]);
            let cross = |a: [f32; 2], b: [f32; 2], p: [f32; 2]| {
                (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
            };
            if cross(a, b, c).abs() < 1e-8 {
                return false;
            }
            let signs = [cross(a, b, point), cross(b, c, point), cross(c, a, point)];
            signs.iter().all(|&v| v >= -1e-5) || signs.iter().all(|&v| v <= 1e-5)
        })
    })
}
