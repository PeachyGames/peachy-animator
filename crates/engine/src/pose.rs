use peachy_document::*;
use serde::Serialize;

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
        let (s, c) = t.rotation.to_radians().sin_cos();
        Self {
            a: c * t.scale_x,
            b: -s * t.scale_y,
            c: s * t.scale_x,
            d: c * t.scale_y,
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
    pub fn inverse_point(self, x: f32, y: f32) -> Option<[f32; 2]> {
        let det = self.a * self.d - self.b * self.c;
        if det.abs() < 1e-8 {
            return None;
        }
        Some([
            (self.d * (x - self.x) - self.b * (y - self.y)) / det,
            (-self.c * (x - self.x) + self.a * (y - self.y)) / det,
        ])
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
            return left.value + (right.value - left.value) * t;
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
                    return previous_y + fraction * (y - previous_y);
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

pub fn world_pose(doc: &Document, locals: &[Transform]) -> Vec<Affine> {
    let mut world: Vec<Affine> = Vec::with_capacity(doc.bones.len());
    for (bone, local) in doc.bones.iter().zip(locals) {
        let matrix = Affine::from_transform(*local);
        world.push(
            bone.parent
                .map_or(matrix, |parent| world[parent].then(matrix)),
        );
    }
    world
}

pub fn evaluate(doc: &Document, animation: Option<usize>, time: f32) -> Vec<Affine> {
    world_pose(doc, &local_pose(doc, animation, time))
}
