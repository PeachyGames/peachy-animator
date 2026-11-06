//! Coordinate conversion and transform gestures, independent of mouse/window APIs.
use crate::Affine;
use peachy_document::{Property, Transform, finite_transform};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Axes {
    Local,
    #[default]
    Parent,
    World,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformTool {
    Rotate,
    Translate,
    Scale,
    Shear,
}

fn rotate(p: [f32; 2], degrees: f32) -> [f32; 2] {
    let (s, c) = degrees.to_radians().sin_cos();
    [p[0] * c - p[1] * s, p[0] * s + p[1] * c]
}
fn unit(p: [f32; 2]) -> [f32; 2] {
    let norm = f64::from(p[0]).hypot(f64::from(p[1]));
    if norm > 0.0 && norm.is_finite() {
        [
            (f64::from(p[0]) / norm) as f32,
            (f64::from(p[1]) / norm) as f32,
        ]
    } else {
        [0., 0.]
    }
}
fn direction(parent: Affine, p: [f32; 2]) -> [f32; 2] {
    [
        parent.a * p[0] + parent.b * p[1],
        parent.c * p[0] + parent.d * p[1],
    ]
}
fn signed_delta(a: f32, b: f32) -> f32 {
    (b - a + 180.).rem_euclid(360.) - 180.
}

pub fn transform_values(local: Transform, parent: Affine, axes: Axes) -> Transform {
    let mut value = local;
    match axes {
        Axes::Local => {
            [value.x, value.y] = rotate([local.x, local.y], -local.rotation);
        }
        Axes::Parent => {}
        Axes::World => {
            let world = parent.then(Affine::from_transform(local));
            value.x = world.x;
            value.y = world.y;
            value.rotation = world.c.atan2(world.a).to_degrees().rem_euclid(360.);
            value.scale_x = world.a.hypot(world.c);
            value.scale_y = world.b.hypot(world.d);
        }
    }
    value
}

/// Scale entry is always local, even when inspecting World axes, as in Spine.
pub fn set_transform_value(
    local: Transform,
    parent: Affine,
    axes: Axes,
    property: Property,
    value: f32,
) -> Result<Transform, String> {
    if !value.is_finite() {
        return Err("Transform value must be finite".into());
    }
    // An unchanged display value is an exact no-op. Re-inverting the same
    // world point can otherwise introduce float drift after an Undo/focus event.
    let current = property.read(if property.is_scale() {
        local
    } else {
        transform_values(local, parent, axes)
    });
    if value == current {
        return Ok(local);
    }
    let mut result = local;
    match property {
        Property::X | Property::Y => {
            let mut shown = transform_values(local, parent, axes);
            property.write(&mut shown, value);
            [result.x, result.y] = match axes {
                Axes::Local => rotate([shown.x, shown.y], local.rotation),
                Axes::Parent => [shown.x, shown.y],
                Axes::World => parent
                    .inverse_point(shown.x, shown.y)
                    .ok_or("Cannot edit World position through a zero-scale parent")?,
            };
        }
        Property::Rotation if axes == Axes::World => {
            if local.scale_x == 0.0 {
                return Err("Cannot edit World rotation of a zero X axis".into());
            }
            let p = parent
                .inverse_vector(value.to_radians().cos(), value.to_radians().sin())
                .ok_or("Cannot edit World rotation through a singular parent")?;
            let desired = p[1].atan2(p[0]).to_degrees()
                + if local.scale_x < 0. { 180. } else { 0. }
                - local.shear_x;
            result.rotation += signed_delta(local.rotation, desired);
        }
        _ => property.write(&mut result, value),
    }
    if !finite_transform(result) {
        return Err("Transform result must be finite".into());
    }
    Ok(result)
}

/// Two normalized world directions for axis-restricted gestures and drawing.
pub fn transform_axes(
    local: Transform,
    parent: Affine,
    axes: Axes,
    tool: TransformTool,
) -> [[f32; 2]; 2] {
    transform_axes_from_world(
        local,
        parent,
        parent.then(Affine::from_transform(local)),
        axes,
        tool,
    )
}

/// Draw a checked applied pose without recomposing its world frame from an
/// authored local transform. Translation retains the existing rotation-frame
/// convention; this does not define unobserved constrained-tool edit semantics.
pub fn transform_axes_from_world(
    local: Transform,
    parent: Affine,
    world: Affine,
    axes: Axes,
    tool: TransformTool,
) -> [[f32; 2]; 2] {
    if matches!(tool, TransformTool::Scale | TransformTool::Shear) {
        return [unit([world.a, world.c]), unit([world.b, world.d])];
    }
    if axes == Axes::World {
        return [[1., 0.], [0., 1.]];
    }
    // Retain the existing rotation-based Local translation frame until its
    // authored-shear convention is observed; do not invent a decomposed frame.
    let rotation = if axes == Axes::Local {
        local.rotation
    } else {
        0.
    };
    [
        unit(direction(parent, rotate([1., 0.], rotation))),
        unit(direction(parent, rotate([0., 1.], rotation))),
    ]
}

/// A collapsed opposite axis contributes nothing, so the live axis still has
/// one recoverable coordinate. Parallel nonzero axes remain ambiguous.
fn scale_coordinate(world: Affine, point: [f32; 2], axis: usize) -> Option<f32> {
    if let Some(coordinates) = world.inverse_vector(point[0], point[1]) {
        return Some(coordinates[axis]);
    }
    let columns = [[world.a, world.c], [world.b, world.d]];
    if columns[1 - axis] != [0., 0.] {
        return None;
    }
    let [x, y] = columns[axis].map(f64::from);
    let squared = x * x + y * y;
    if squared == 0.0 || !squared.is_finite() {
        return None;
    }
    let coordinate = ((f64::from(point[0]) * x + f64::from(point[1]) * y) / squared) as f32;
    coordinate.is_finite().then_some(coordinate)
}

#[derive(Clone, Copy, Debug)]
pub struct TransformGesture {
    pub tool: TransformTool,
    pub axes: Axes,
    pub axis: Option<usize>,
    pub start: [f32; 2],
    pub end: [f32; 2],
    pub snap_rotation: bool,
}
pub fn drag_transform(
    local: Transform,
    parent: Affine,
    gesture: TransformGesture,
) -> Result<Transform, String> {
    let TransformGesture {
        tool,
        axes,
        axis,
        start,
        end,
        snap_rotation,
    } = gesture;
    if axis.is_some_and(|a| a > 1) || !start.into_iter().chain(end).all(f32::is_finite) {
        return Err("Invalid transform gesture".into());
    }
    let mut result = local;
    match tool {
        TransformTool::Translate => {
            let mut delta = [end[0] - start[0], end[1] - start[1]];
            if let Some(axis) = axis {
                let basis = transform_axes(local, parent, axes, tool)[axis];
                let distance = delta[0] * basis[0] + delta[1] * basis[1];
                delta = [basis[0] * distance, basis[1] * distance];
            }
            let p = parent
                .inverse_vector(delta[0], delta[1])
                .ok_or("Cannot translate through a zero-scale parent")?;
            result.x += p[0];
            result.y += p[1];
        }
        TransformTool::Rotate => {
            let a = parent
                .inverse_point(start[0], start[1])
                .ok_or("Cannot rotate through a zero-scale parent")?;
            let b = parent
                .inverse_point(end[0], end[1])
                .ok_or("Cannot rotate through a zero-scale parent")?;
            let a = [a[0] - local.x, a[1] - local.y];
            let b = [b[0] - local.x, b[1] - local.y];
            if a[0].hypot(a[1]) > 1e-5 && b[0].hypot(b[1]) > 1e-5 {
                result.rotation +=
                    signed_delta(a[1].atan2(a[0]).to_degrees(), b[1].atan2(b[0]).to_degrees());
            }
            if snap_rotation {
                result.rotation = (result.rotation / 15.).round() * 15.;
            }
        }
        TransformTool::Shear => {
            let axis = axis.ok_or("Shear requires an explicit X or Y handle")?;
            let a = parent
                .inverse_point(start[0], start[1])
                .ok_or("Cannot shear through a singular parent")?;
            let b = parent
                .inverse_point(end[0], end[1])
                .ok_or("Cannot shear through a singular parent")?;
            let a = [a[0] - local.x, a[1] - local.y];
            let b = [b[0] - local.x, b[1] - local.y];
            if a[0].hypot(a[1]) <= 1e-5 || b[0].hypot(b[1]) <= 1e-5 {
                return Err("Shear handle needs a nonzero pointer direction".into());
            }
            let delta = signed_delta(a[1].atan2(a[0]).to_degrees(), b[1].atan2(b[0]).to_degrees());
            if axis == 0 {
                result.shear_x += delta;
            } else {
                result.shear_y += delta;
            }
        }
        TransformTool::Scale => {
            let origin = parent.point(local.x, local.y);
            let a = [start[0] - origin[0], start[1] - origin[1]];
            let b = [end[0] - origin[0], end[1] - origin[1]];
            let dot = |v: [f32; 2], u: [f32; 2]| v[0] * u[0] + v[1] * u[1];
            let factor = if let Some(i) = axis {
                // A dot projection mixes the other coordinate on an oblique
                // basis. Solve actual bone coordinates before taking the ratio.
                let world = parent.then(Affine::from_transform(local));
                let a = scale_coordinate(world, a, i)
                    .ok_or("Cannot scale through an ambiguous or zero bone axis")?;
                let b = scale_coordinate(world, b, i)
                    .ok_or("Cannot scale through an ambiguous or zero bone axis")?;
                if a.abs() < 1e-5 { 1.0 } else { b / a }
            } else {
                let denominator = dot(a, a);
                if denominator < 1e-8 {
                    1.0
                } else {
                    dot(a, b) / denominator
                }
            };
            if axis.is_none_or(|a| a == 0) {
                result.scale_x *= factor;
            }
            if axis.is_none_or(|a| a == 1) {
                result.scale_y *= factor;
            }
        }
    }
    if !finite_transform(result) {
        return Err("Transform result must be finite".into());
    }
    Ok(result)
}

/// Spine numeric-entry operators are relative to the value when entry starts.
pub fn parse_transform_input(text: &str, base: f32) -> Option<f32> {
    let text = text.trim();
    let value = match text.as_bytes().first()? {
        b'+' => base + text[1..].trim().parse::<f32>().ok()?,
        b'*' => base * text[1..].trim().parse::<f32>().ok()?,
        b'/' => base / text[1..].trim().parse::<f32>().ok()?,
        _ => text.parse::<f32>().ok()?,
    };
    value.is_finite().then_some(value)
}
