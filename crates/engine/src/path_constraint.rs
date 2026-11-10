//! Path constraints (bounded subset), fitted to black-box observations of the
//! external runtime on original random paths (docs/slices/path-constraints.md):
//! - Distances along the path use a 4-segment polyline length per curve; inside
//!   a curve, the fraction travelled maps to the curve parameter through a
//!   10-segment arc-length table (linear in the parameter within a segment).
//! - Percent position and spacing scale the summed 4-segment length; length
//!   and fixed spacing scale with the previous bone's world X scale.
//! - Beyond either end, bones continue in a straight line along the end handle.
//! - The tangent is the curve derivative, except below t = 0.001 where the
//!   curve's start direction is used.
//! - Tangent rotation turns the bone's world frame toward the curve direction
//!   plus the offset rotation, by mixRotate; X/Y move toward the point by mixX/Y.
//!
//! Bones are applied in list order, rebuilding world frames after each so a
//! chain sees its constrained parents.
use crate::{Affine, pose::OBSERVED_DEGREES_TO_RADIANS, world_pose_with, world_vertices};
use peachy_document::{
    Document, Inherit, Path, PathConstraint, PositionMode, SpacingMode, Transform,
};

type P = [f64; 2];

/// Cubic segments of the native (in handle, knot, out handle) representation.
/// Shared by constraint evaluation and editor overlays. Open paths omit the
/// unused first in handle and last out handle; closed paths join the last knot
/// back to the first. Invalid topology emits no segments.
pub fn path_cubics(points: &[[f32; 2]], closed: bool) -> Vec<[[f32; 2]; 4]> {
    if points.len() < 6 || !points.len().is_multiple_of(3) {
        return vec![];
    }
    let knots = points.len() / 3;
    (0..if closed { knots } else { knots - 1 })
        .map(|i| {
            let next = (i + 1) % knots;
            [
                points[3 * i + 1],
                points[3 * i + 2],
                points[3 * next],
                points[3 * next + 1],
            ]
        })
        .collect()
}

fn cubic(c: &[P; 4], t: f64) -> P {
    let u = 1. - t;
    [0, 1].map(|k| {
        u * u * u * c[0][k]
            + 3. * u * u * t * c[1][k]
            + 3. * u * t * t * c[2][k]
            + t * t * t * c[3][k]
    })
}

fn derivative(c: &[P; 4], t: f64) -> P {
    let u = 1. - t;
    [0, 1].map(|k| {
        3. * u * u * (c[1][k] - c[0][k])
            + 6. * u * t * (c[2][k] - c[1][k])
            + 3. * t * t * (c[3][k] - c[2][k])
    })
}

fn polyline(c: &[P; 4], segments: usize) -> Vec<f64> {
    let mut cum = vec![0.];
    let mut previous = c[0];
    for k in 1..=segments {
        let p = cubic(c, k as f64 / segments as f64);
        cum.push(cum[k - 1] + (p[0] - previous[0]).hypot(p[1] - previous[1]));
        previous = p;
    }
    cum
}

struct Curves {
    curves: Vec<[P; 4]>,
    coarse: Vec<f64>,
    fine: Vec<Vec<f64>>,
    total: f64,
}

impl Curves {
    fn new(points: &[[f32; 2]]) -> Self {
        let curves: Vec<[P; 4]> = path_cubics(points, false)
            .into_iter()
            .map(|curve| curve.map(|point| point.map(f64::from)))
            .collect();
        let coarse: Vec<f64> = curves.iter().map(|c| polyline(c, 4)[4]).collect();
        let fine = curves.iter().map(|c| polyline(c, 10)).collect();
        let total = coarse.iter().sum();
        Self {
            curves,
            coarse,
            fine,
            total,
        }
    }

    /// World point and direction (degrees) at distance `s` along the path.
    fn at(&self, s: f64) -> (P, f64) {
        let angle = |d: P| d[1].atan2(d[0]) / OBSERVED_DEGREES_TO_RADIANS;
        if s < 0. {
            let c = &self.curves[0];
            let d = [c[1][0] - c[0][0], c[1][1] - c[0][1]];
            let len = d[0].hypot(d[1]);
            let unit = if len > 0. {
                [d[0] / len, d[1] / len]
            } else {
                [0., 0.]
            };
            return ([c[0][0] + unit[0] * s, c[0][1] + unit[1] * s], angle(d));
        }
        if s > self.total {
            let c = self.curves.last().expect("open paths have a curve");
            let d = [c[3][0] - c[2][0], c[3][1] - c[2][1]];
            let len = d[0].hypot(d[1]);
            let unit = if len > 0. {
                [d[0] / len, d[1] / len]
            } else {
                [0., 0.]
            };
            let over = s - self.total;
            return (
                [c[3][0] + unit[0] * over, c[3][1] + unit[1] * over],
                angle(d),
            );
        }
        let mut s = s;
        let mut index = 0;
        while index + 1 < self.curves.len() && s > self.coarse[index] {
            s -= self.coarse[index];
            index += 1;
        }
        let cum = &self.fine[index];
        let local = if self.coarse[index] > 0. {
            s / self.coarse[index] * cum[10]
        } else {
            0.
        };
        let mut k = 1;
        while k < 10 && cum[k] < local {
            k += 1;
        }
        let span = cum[k] - cum[k - 1];
        let t = (k as f64 - 1.
            + if span > 0. {
                (local - cum[k - 1]) / span
            } else {
                0.
            })
            / 10.;
        let curve = &self.curves[index];
        // Observed: below t = 0.001 the runtime uses the curve's start
        // direction instead of the derivative at t.
        let direction = if t < 0.001 {
            [curve[1][0] - curve[0][0], curve[1][1] - curve[0][1]]
        } else {
            derivative(curve, t)
        };
        (cubic(curve, t), angle(direction))
    }
}

fn normalized(mut degrees: f64) -> f64 {
    if degrees > 180. {
        degrees -= 360.;
    } else if degrees <= -180. {
        degrees += 360.;
    }
    degrees
}

/// Sampled position/spacing for one path constraint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathPose {
    pub position: f32,
    pub spacing: f32,
}

pub(crate) fn validate_domain(
    doc: &Document,
    constraint: &PathConstraint,
    path: &Path,
    inherits: &[Inherit],
) -> Result<(), String> {
    constraint.supported_subset()?;
    if path.closed || !path.constant_speed {
        return Err("closed paths and paths without constant speed are unsupported".into());
    }
    if let Some(&bone) = constraint
        .bones
        .iter()
        .find(|&&bone| inherits[bone] != Inherit::Normal)
    {
        return Err(format!(
            "constrained bone {} must inherit normally",
            doc.bones[bone].name
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply(
    doc: &Document,
    constraint: &PathConstraint,
    pose: PathPose,
    path: &Path,
    deform: Option<&[f32]>,
    locals: &mut [Transform],
    world: &mut Vec<Affine>,
    inherits: &[Inherit],
) -> Result<(), String> {
    validate_domain(doc, constraint, path, inherits)?;
    let setup = constraint.setup;
    let (mix_rotate, mix_x, mix_y) = (
        f64::from(setup.mix_rotate),
        f64::from(setup.mix_x),
        f64::from(setup.mix_y),
    );
    if mix_rotate == 0. && mix_x == 0. && mix_y == 0. {
        return Ok(());
    }
    let slot_bone = doc.slots[constraint.slot].bone;
    let curves = Curves::new(&match deform {
        Some(offsets) => crate::deformed_world_vertices(&path.vertices, slot_bone, world, offsets),
        None => world_vertices(&path.vertices, slot_bone, world),
    });
    let (position, spacing) = (f64::from(pose.position), f64::from(pose.spacing));
    let mut s = match constraint.position_mode {
        PositionMode::Fixed => position,
        PositionMode::Percent => position * curves.total,
    };
    for (i, &bone) in constraint.bones.iter().enumerate() {
        if i > 0 {
            // Observed: length and fixed spacing scale with the previous bone's
            // world X scale; percent spacing does not.
            let previous = constraint.bones[i - 1];
            let scale = f64::from(world[previous].a).hypot(f64::from(world[previous].c));
            s += match constraint.spacing_mode {
                SpacingMode::Length => (f64::from(doc.bones[previous].length) + spacing) * scale,
                SpacingMode::Fixed => spacing * scale,
                SpacingMode::Percent => spacing * curves.total,
                SpacingMode::Proportional => unreachable!("refused by supported_subset"),
            };
        }
        let (point, tangent) = curves.at(s);
        let m = world[bone];
        let [mut a, mut b, mut c, mut d, mut x, mut y] =
            [m.a, m.b, m.c, m.d, m.x, m.y].map(f64::from);
        if mix_rotate != 0. {
            let current = c.atan2(a) / OBSERVED_DEGREES_TO_RADIANS;
            let target = tangent + f64::from(constraint.offset_rotation);
            let turn = normalized(target - current) * mix_rotate * OBSERVED_DEGREES_TO_RADIANS;
            let (sin, cos) = turn.sin_cos();
            (a, b, c, d) = (
                cos * a - sin * c,
                cos * b - sin * d,
                sin * a + cos * c,
                sin * b + cos * d,
            );
        }
        x += (point[0] - x) * mix_x;
        y += (point[1] - y) * mix_y;
        locals[bone] =
            crate::transform_constraint::local_from_world(doc, world, bone, [a, b, c, d, x, y])?;
        *world = world_pose_with(doc, locals, inherits);
    }
    Ok(())
}
