//! World-space transform constraints (bounded subset). The rules were fitted to
//! black-box observations of the external runtime on original random rigs; see
//! docs/slices/transform-constraints.md. No runtime source was read.
//! - Rotate: turn the bone's world frame toward the source's world X-axis angle
//!   plus the rotation offset (subtracted when the source frame is mirrored),
//!   by mixRotate, one ±180° correction.
//! - X/Y: move the bone's world origin toward the source frame applied to the
//!   (x, y) offset, by mixX/mixY. A mapping turns each coordinate p into
//!   (p − from offset) × scale + to offset, clamped between the to offset and
//!   max when the constraint clamps.
//!
//! The result is written back as the bone's applied local transform relative to
//! its parent, so children, bindings and tools see one consistent pose.
use crate::{Affine, pose::OBSERVED_DEGREES_TO_RADIANS, world_pose_with};
use peachy_document::{Document, Inherit, Transform, TransformConstraint, TransformProperty};

fn within(doc: &Document, bone: usize, ancestor: usize) -> bool {
    let mut current = Some(bone);
    while let Some(index) = current {
        if index == ancestor {
            return true;
        }
        current = doc.bones[index].parent;
    }
    false
}

/// Evaluation domain shared by import preflight and checked evaluation.
pub(crate) fn validate_domain(
    doc: &Document,
    constraint: &TransformConstraint,
    inherits: &[Inherit],
) -> Result<(), String> {
    constraint.supported_subset()?;
    for (i, &bone) in constraint.bones.iter().enumerate() {
        if inherits[bone] != Inherit::Normal {
            return Err(format!(
                "constrained bone {} must inherit normally",
                doc.bones[bone].name
            ));
        }
        if within(doc, constraint.source, bone) {
            return Err(format!(
                "source {} is inside constrained bone {}",
                doc.bones[constraint.source].name, doc.bones[bone].name
            ));
        }
        if constraint.bones[..i]
            .iter()
            .any(|&other| within(doc, bone, other) || within(doc, other, bone))
        {
            return Err("constrained bones must not contain one another".into());
        }
    }
    Ok(())
}

fn normalized(mut degrees: f64) -> f64 {
    if degrees > 180. {
        degrees -= 360.;
    } else if degrees <= -180. {
        degrees += 360.;
    }
    degrees
}

pub(crate) fn apply(
    doc: &Document,
    constraint: &TransformConstraint,
    locals: &mut [Transform],
    world: &mut Vec<Affine>,
    inherits: &[Inherit],
) -> Result<(), String> {
    validate_domain(doc, constraint, inherits)?;
    let targets = constraint.targets();
    let mix = |p: TransformProperty| {
        if targets.contains(&p) {
            f64::from(constraint.mix.get(p))
        } else {
            0.0
        }
    };
    let (mix_rotate, mix_x, mix_y) = (
        mix(TransformProperty::Rotate),
        mix(TransformProperty::X),
        mix(TransformProperty::Y),
    );
    if mix_rotate == 0. && mix_x == 0. && mix_y == 0. {
        return Ok(());
    }
    let wide = |m: Affine| [m.a, m.b, m.c, m.d, m.x, m.y].map(f64::from);
    let source = wide(world[constraint.source]);
    let offset = |p: TransformProperty| f64::from(constraint.offsets.get(p));
    let mirrored = source[0] * source[3] - source[1] * source[2] < 0.;
    let target_angle = source[2].atan2(source[0]) / OBSERVED_DEGREES_TO_RADIANS
        + if mirrored { -1. } else { 1. } * offset(TransformProperty::Rotate);
    let (ox, oy) = (offset(TransformProperty::X), offset(TransformProperty::Y));
    let mapped = |property: TransformProperty, value: f64| {
        let Some(mapping) = constraint.mappings.iter().find(|m| m.from == property) else {
            return value;
        };
        let to = mapping.to[0];
        let value =
            (value - f64::from(mapping.offset)) * f64::from(to.scale) + f64::from(to.offset);
        if constraint.clamp {
            let (low, high) = (f64::from(to.offset), f64::from(to.max));
            value.clamp(low.min(high), low.max(high))
        } else {
            value
        }
    };
    let target = [
        mapped(
            TransformProperty::X,
            source[0] * ox + source[1] * oy + source[4],
        ),
        mapped(
            TransformProperty::Y,
            source[2] * ox + source[3] * oy + source[5],
        ),
    ];
    for &bone in &constraint.bones {
        let [mut a, mut b, mut c, mut d, mut x, mut y] = wide(world[bone]);
        if mix_rotate != 0. {
            let current = c.atan2(a) / OBSERVED_DEGREES_TO_RADIANS;
            let turn =
                normalized(target_angle - current) * mix_rotate * OBSERVED_DEGREES_TO_RADIANS;
            let (s, k) = turn.sin_cos();
            (a, b, c, d) = (k * a - s * c, k * b - s * d, s * a + k * c, s * b + k * d);
        }
        x += (target[0] - x) * mix_x;
        y += (target[1] - y) * mix_y;
        locals[bone] = local_from_world(doc, world, bone, [a, b, c, d, x, y])?;
    }
    *world = world_pose_with(doc, locals, inherits);
    Ok(())
}

/// Decompose a constrained world frame into a local transform under the bone's
/// parent: rotation and Scale X from the X axis, Scale Y and Shear Y from the Y
/// axis, Shear X zero. Exact for any invertible parent and child frame.
fn local_from_world(
    doc: &Document,
    world: &[Affine],
    bone: usize,
    m: [f64; 6],
) -> Result<Transform, String> {
    let [a, b, c, d, x, y] = match doc.bones[bone].parent {
        None => m,
        Some(parent) => {
            let p = world[parent];
            let [pa, pb, pc, pd, px, py] = [p.a, p.b, p.c, p.d, p.x, p.y].map(f64::from);
            let det = pa * pd - pb * pc;
            if !det.is_finite() || det == 0. {
                return Err(format!(
                    "bone {} has a singular parent frame",
                    doc.bones[bone].name
                ));
            }
            let inv = [pd / det, -pb / det, -pc / det, pa / det];
            let (dx, dy) = (m[4] - px, m[5] - py);
            [
                inv[0] * m[0] + inv[1] * m[2],
                inv[0] * m[1] + inv[1] * m[3],
                inv[2] * m[0] + inv[3] * m[2],
                inv[2] * m[1] + inv[3] * m[3],
                inv[0] * dx + inv[1] * dy,
                inv[2] * dx + inv[3] * dy,
            ]
        }
    };
    let rotation = c.atan2(a) / OBSERVED_DEGREES_TO_RADIANS;
    let shear_y = normalized(d.atan2(b) / OBSERVED_DEGREES_TO_RADIANS - rotation - 90.);
    let local = Transform {
        x: x as f32,
        y: y as f32,
        rotation: rotation as f32,
        scale_x: a.hypot(c) as f32,
        scale_y: b.hypot(d) as f32,
        shear_x: 0.,
        shear_y: shear_y as f32,
    };
    if !peachy_document::finite_transform(local) {
        return Err(format!(
            "bone {}: constrained transform is outside the finite range",
            doc.bones[bone].name
        ));
    }
    Ok(local)
}
