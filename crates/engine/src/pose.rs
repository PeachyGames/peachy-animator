use peachy_document::*;
use serde::Serialize;
pub(crate) const OBSERVED_DEGREES_TO_RADIANS: f64 = f64::from_bits(0x3f91_df46_a6c0_950b);

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Affine {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub x: f32,
    pub y: f32,
}

impl Affine {
    pub fn from_transform(t: Transform) -> Self {
        // Derived from original public unit-bone matrix observations, not runtime
        // constants or implementation. See ../SHEAR_PROVENANCE.md and shear tests.
        let x = (f64::from(t.rotation) + f64::from(t.shear_x)) * OBSERVED_DEGREES_TO_RADIANS;
        // Preserve association: (r + 90) + hy differs from (r + hy) + 90 for huge values.
        let y = (f64::from(t.rotation) + 90.0 + f64::from(t.shear_y)) * OBSERVED_DEGREES_TO_RADIANS;
        Self {
            a: (x.cos() * f64::from(t.scale_x)) as f32,
            b: (y.cos() * f64::from(t.scale_y)) as f32,
            c: (x.sin() * f64::from(t.scale_x)) as f32,
            d: (y.sin() * f64::from(t.scale_y)) as f32,
            x: t.x,
            y: t.y,
        }
    }
    pub fn then(self, local: Self) -> Self {
        Self {
            a: self.a * local.a + self.b * local.c,
            b: self.a * local.b + self.b * local.d,
            c: self.c * local.a + self.d * local.c,
            d: self.c * local.b + self.d * local.d,
            x: self.a * local.x + self.b * local.y + self.x,
            y: self.c * local.x + self.d * local.y + self.y,
        }
    }
    pub fn point(self, x: f32, y: f32) -> [f32; 2] {
        [
            self.a * x + self.b * y + self.x,
            self.c * x + self.d * y + self.y,
        ]
    }
    /// Invert a point with finite f64 intermediates, rejecting only a singular
    /// linear transform or a result outside the finite f32 coordinate range.
    pub fn inverse_point(self, x: f32, y: f32) -> Option<[f32; 2]> {
        self.inverse_vector_f64(
            f64::from(x) - f64::from(self.x),
            f64::from(y) - f64::from(self.y),
        )
    }
    /// Invert a direction without adding/subtracting the parent translation.
    /// A large origin must not round a unit direction down to a zero vector.
    pub fn inverse_vector(self, x: f32, y: f32) -> Option<[f32; 2]> {
        self.inverse_vector_f64(f64::from(x), f64::from(y))
    }
    fn inverse_vector_f64(self, x: f64, y: f64) -> Option<[f32; 2]> {
        let [a, b, c, d] = [self.a, self.b, self.c, self.d].map(f64::from);
        if ![a, b, c, d, x, y].into_iter().all(f64::is_finite) {
            return None;
        }
        let determinant = a * d - b * c;
        if determinant == 0.0 {
            return None;
        }
        let result = [
            ((d * x - b * y) / determinant) as f32,
            ((a * y - c * x) / determinant) as f32,
        ];
        result.into_iter().all(f32::is_finite).then_some(result)
    }
}

/// Preserve ordinary f32 interpolation exactly; widen only when an
/// intermediate overflow would hide a representable result. The final cast
/// intentionally remains nonfinite if the true result cannot fit in f32.
fn interpolate(left: f32, right: f32, progress: f32) -> f32 {
    let value = left + (right - left) * progress;
    if value.is_finite() {
        value
    } else {
        (f64::from(left) + (f64::from(right) - f64::from(left)) * f64::from(progress)) as f32
    }
}

pub fn sample(track: &Track, time: f32) -> f32 {
    if track.keys.is_empty() || time < track.keys[0].time {
        return track.property.neutral();
    }
    for pair in track.keys.windows(2) {
        let (left, right) = (pair[0], pair[1]);
        if time < right.time {
            let progress = (time - left.time) / (right.time - left.time);
            let t = curve_progress(left.curve, progress);
            return interpolate(left.value, right.value, t);
        }
    }
    track
        .keys
        .last()
        .map_or(track.property.neutral(), |k| k.value)
}

pub fn curve_progress(curve: Curve, progress: f32) -> f32 {
    let t = progress.clamp(0.0, 1.0);
    match curve {
        Curve::Linear => t,
        Curve::Stepped => 0.0,
        Curve::Bezier([x1, y1, x2, y2]) => {
            let cubic = |p1: f32, p2: f32, u: f32| {
                3.0 * (1.0 - u).powi(2) * u * p1 + 3.0 * (1.0 - u) * u * u * p2 + u * u * u
            };
            // The external 4.3 oracle interpolates a ten-segment parametric
            // polyline. Exact cubic inversion visibly disagrees near endpoints.
            // This construction was inferred from original synthetic outputs;
            // see docs/PROVENANCE.md, not from runtime implementation source.
            let (mut previous_x, mut previous_y) = (0.0, 0.0);
            for segment in 1..=10 {
                let u = segment as f32 / 10.0;
                let x = cubic(x1, x2, u);
                let y = cubic(y1, y2, u);
                if t <= x {
                    let fraction = if x > previous_x {
                        (t - previous_x) / (x - previous_x)
                    } else {
                        0.0
                    };
                    return interpolate(previous_y, y, fraction);
                }
                previous_x = x;
                previous_y = y;
            }
            1.0
        }
    }
}

pub fn local_pose(doc: &Document, animation: Option<usize>, time: f32) -> Vec<Transform> {
    let mut pose: Vec<_> = doc.bones.iter().map(|b| b.setup).collect();
    if let Some(a) = animation.and_then(|i| doc.animations.get(i)) {
        let time = if time.is_finite() { time.max(0.0) } else { 0.0 };
        for track in &a.tracks {
            let value = sample(track, time);
            let base = track.property.read(doc.bones[track.bone].setup);
            let value = if matches!(track.property, Property::ScaleX | Property::ScaleY) {
                base * value
            } else {
                base + value
            };
            track.property.write(&mut pose[track.bone], value);
        }
    }
    pose
}

/// Forward kinematics with each bone's Setup inheritance mode.
pub fn world_pose(doc: &Document, locals: &[Transform]) -> Vec<Affine> {
    let inherits: Vec<_> = doc.bones.iter().map(|bone| bone.inherit).collect();
    world_pose_with(doc, locals, &inherits)
}

/// Forward kinematics with explicit (possibly keyed) inheritance modes.
pub fn world_pose_with(doc: &Document, locals: &[Transform], inherits: &[Inherit]) -> Vec<Affine> {
    let mut world: Vec<Affine> = Vec::with_capacity(doc.bones.len());
    for (index, (bone, local)) in doc.bones.iter().zip(locals).enumerate() {
        let matrix = Affine::from_transform(*local);
        let mode = inherits.get(index).copied().unwrap_or_default();
        world.push(match bone.parent {
            None => matrix,
            Some(parent) if mode == Inherit::Normal => world[parent].then(matrix),
            Some(parent) => inherited(world[parent], *local, mode),
        });
    }
    world
}

/// The keyed inheritance mode of every bone at `time`, falling back to Setup.
pub fn inherit_pose(doc: &Document, animation: Option<usize>, time: f32) -> Vec<Inherit> {
    let mut modes: Vec<_> = doc.bones.iter().map(|bone| bone.inherit).collect();
    if let Some(animation) = animation.and_then(|index| doc.animations.get(index)) {
        let time = if time.is_finite() { time.max(0.0) } else { 0.0 };
        for track in &animation.inherit_tracks {
            if let Some(mode) = track.sample(time)
                && let Some(slot) = modes.get_mut(track.bone)
            {
                *slot = mode;
            }
        }
    }
    modes
}

/// A non-normal child frame. Translation always passes through the full parent.
fn inherited(parent: Affine, local: Transform, mode: Inherit) -> Affine {
    let p = [parent.a, parent.b, parent.c, parent.d].map(f64::from);
    let [x, y] = parent.point(local.x, local.y);
    match inherit_linear(mode, p, local) {
        Some([a, b, c, d]) => Affine {
            a: a as f32,
            b: b as f32,
            c: c as f32,
            d: d as f32,
            x,
            y,
        },
        None => Affine {
            a: f32::NAN,
            b: f32::NAN,
            c: f32::NAN,
            d: f32::NAN,
            x,
            y,
        },
    }
}

/// Parent X-axis squared length below which `noRotationOrReflection` is
/// refused. The external runtime switches to a different degenerate branch
/// between 1e-10 and 4e-10; the model below was verified exact down to 4e-10,
/// so this keeps a margin. See docs/slices/inherit-modes.md.
pub const INHERIT_MIN_PARENT_AXIS_SQUARED: f64 = 1e-8;

fn local_linear(t: Transform, rotation: f64) -> [f64; 4] {
    let x = (rotation + f64::from(t.shear_x)) * OBSERVED_DEGREES_TO_RADIANS;
    let y = (rotation + 90.0 + f64::from(t.shear_y)) * OBSERVED_DEGREES_TO_RADIANS;
    [
        x.cos() * f64::from(t.scale_x),
        y.cos() * f64::from(t.scale_y),
        x.sin() * f64::from(t.scale_x),
        y.sin() * f64::from(t.scale_y),
    ]
}

fn multiply(p: [f64; 4], q: [f64; 4]) -> [f64; 4] {
    [
        p[0] * q[0] + p[1] * q[2],
        p[0] * q[1] + p[1] * q[3],
        p[2] * q[0] + p[3] * q[2],
        p[2] * q[1] + p[3] * q[3],
    ]
}

/// Child world linear part (a, b, c, d) under a non-normal mode, or `None` for a
/// degenerate parent this model does not cover. Independently derived from
/// black-box observations of the external runtime (docs/PROVENANCE.md):
/// - onlyTranslation: the local matrix alone;
/// - noRotationOrReflection: the parent's stretch along its own X axis angle θ,
///   R(θ)·diag(|X|, |det|/|X|)·R(−θ), times the local matrix;
/// - noScale / noScaleOrReflection: the local matrix without rotation, turned to
///   the angle the parent maps the local rotation direction to, mirrored first
///   for noScale when the parent frame is reflected.
pub(crate) fn inherit_linear(mode: Inherit, p: [f64; 4], local: Transform) -> Option<[f64; 4]> {
    let rotation = f64::from(local.rotation);
    let result = match mode {
        Inherit::Normal => multiply(p, local_linear(local, rotation)),
        Inherit::OnlyTranslation => local_linear(local, rotation),
        Inherit::NoRotationOrReflection => {
            let axis_squared = p[0] * p[0] + p[2] * p[2];
            if !(axis_squared >= INHERIT_MIN_PARENT_AXIS_SQUARED) {
                return None;
            }
            let length = axis_squared.sqrt();
            let (sin, cos) = p[2].atan2(p[0]).sin_cos();
            let other = (p[0] * p[3] - p[1] * p[2]).abs() / length;
            let stretch = [
                cos * cos * length + sin * sin * other,
                cos * sin * (length - other),
                cos * sin * (length - other),
                sin * sin * length + cos * cos * other,
            ];
            multiply(stretch, local_linear(local, rotation))
        }
        Inherit::NoScale | Inherit::NoScaleOrReflection => {
            let (sin, cos) = (rotation * OBSERVED_DEGREES_TO_RADIANS).sin_cos();
            let (vx, vy) = (p[0] * cos + p[1] * sin, p[2] * cos + p[3] * sin);
            if !(vx.is_finite() && vy.is_finite()) || (vx == 0.0 && vy == 0.0) {
                return None;
            }
            let (s, c) = vy.atan2(vx).sin_cos();
            let mut unrotated = local_linear(local, 0.0);
            if mode == Inherit::NoScale && p[0] * p[3] - p[1] * p[2] < 0.0 {
                unrotated[2] = -unrotated[2];
                unrotated[3] = -unrotated[3];
            }
            multiply([c, -s, s, c], unrotated)
        }
    };
    result.iter().all(|v| v.is_finite()).then_some(result)
}

pub fn evaluate(
    doc: &Document,
    animation: Option<usize>,
    time: f32,
) -> Result<Vec<Affine>, String> {
    Ok(crate::checked_evaluate_pose(doc, animation, time)?
        .world()
        .to_vec())
}
