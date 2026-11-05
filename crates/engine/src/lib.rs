//! Shared commands, history and deterministic animation evaluation.
use peachy_document::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "camelCase")]
pub enum Command {
    SetSetup {
        bone: usize,
        transform: Transform,
    },
    SetKey {
        animation: usize,
        bone: usize,
        property: Property,
        key: Keyframe,
    },
    DeleteKey {
        animation: usize,
        bone: usize,
        property: Property,
        time: f32,
    },
    AddBone {
        bone: Bone,
    },
    AddAnimation {
        name: String,
        duration: f32,
    },
    Batch {
        commands: Vec<Command>,
    },
}

pub struct Session {
    document: Document,
    undo: Vec<Document>,
    redo: Vec<Document>,
    saved: Document,
}

impl Session {
    pub fn new(document: Document) -> Result<Self, String> {
        document.validate()?;
        Ok(Self {
            saved: document.clone(),
            document,
            undo: vec![],
            redo: vec![],
        })
    }
    pub fn document(&self) -> &Document {
        &self.document
    }
    pub fn is_dirty(&self) -> bool {
        self.document != self.saved
    }
    pub fn mark_saved(&mut self) {
        self.saved = self.document.clone();
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    /// Apply to a candidate first. Failed commands change neither data nor history.
    pub fn dispatch(&mut self, command: Command) -> Result<(), String> {
        let mut candidate = self.document.clone();
        apply(&mut candidate, command)?;
        candidate.validate()?;
        if candidate != self.document {
            self.undo
                .push(std::mem::replace(&mut self.document, candidate));
            if self.undo.len() > 128 {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
        Ok(())
    }
    pub fn undo(&mut self) {
        if let Some(previous) = self.undo.pop() {
            self.redo
                .push(std::mem::replace(&mut self.document, previous));
        }
    }
    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.document, next));
        }
    }
}

fn apply(doc: &mut Document, command: Command) -> Result<(), String> {
    match command {
        Command::SetSetup { bone, transform } => {
            doc.bones.get_mut(bone).ok_or("Unknown bone")?.setup = transform
        }
        Command::AddBone { bone } => doc.bones.push(bone),
        Command::AddAnimation { name, duration } => doc.animations.push(Animation {
            name,
            duration,
            tracks: vec![],
        }),
        Command::SetKey {
            animation,
            bone,
            property,
            key,
        } => {
            let a = doc
                .animations
                .get_mut(animation)
                .ok_or("Unknown animation")?;
            if !key.time.is_finite() || !key.value.is_finite() {
                return Err("Keyframe must be finite".into());
            }
            let index = match a
                .tracks
                .iter()
                .position(|t| t.bone == bone && t.property == property)
            {
                Some(i) => i,
                None => {
                    a.tracks.push(Track {
                        bone,
                        property,
                        keys: vec![],
                    });
                    a.tracks.len() - 1
                }
            };
            let keys = &mut a.tracks[index].keys;
            if let Some(i) = keys
                .iter()
                .position(|k| (k.time - key.time).abs() < 0.00001)
            {
                keys[i] = key;
            } else {
                keys.push(key);
            }
            keys.sort_by(|a, b| a.time.total_cmp(&b.time));
        }
        Command::DeleteKey {
            animation,
            bone,
            property,
            time,
        } => {
            let a = doc
                .animations
                .get_mut(animation)
                .ok_or("Unknown animation")?;
            for track in &mut a.tracks {
                if track.bone == bone && track.property == property {
                    track.keys.retain(|k| (k.time - time).abs() > 0.00001);
                }
            }
            a.tracks.retain(|t| !t.keys.is_empty());
        }
        Command::Batch { commands } => {
            for command in commands {
                apply(doc, command)?;
            }
        }
    }
    Ok(())
}

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
        let time = if time.is_finite() {
            time.clamp(0.0, a.duration)
        } else {
            0.0
        };
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

#[cfg(test)]
mod tests {
    use super::*;
    fn doc() -> Document {
        Document {
            format: 1,
            name: "fixture".into(),
            fps: 30,
            bones: vec![Bone {
                name: "root".into(),
                parent: None,
                length: 50.0,
                setup: Transform::default(),
            }],
            slots: vec![],
            skins: Default::default(),
            animations: vec![Animation {
                name: "wave".into(),
                duration: 1.0,
                tracks: vec![],
            }],
        }
    }
    fn key(time: f32, value: f32) -> Command {
        Command::SetKey {
            animation: 0,
            bone: 0,
            property: Property::Rotation,
            key: Keyframe {
                time,
                value,
                curve: Curve::Linear,
            },
        }
    }
    #[test]
    fn commands_undo_redo_and_failed_batches_are_atomic() {
        let mut s = Session::new(doc()).unwrap();
        let original = s.document().clone();
        assert!(
            s.dispatch(Command::Batch {
                commands: vec![key(0.0, 20.0), key(2.0, 90.0)]
            })
            .is_err()
        );
        assert_eq!(*s.document(), original);
        assert!(!s.can_undo());
        s.dispatch(key(0.0, 20.0)).unwrap();
        assert!(s.is_dirty());
        s.undo();
        assert_eq!(*s.document(), original);
        assert!(!s.is_dirty());
        s.redo();
        assert_eq!(local_pose(s.document(), Some(0), 0.0)[0].rotation, 20.0);
        s.mark_saved();
        assert!(!s.is_dirty());
        s.undo();
        assert!(s.is_dirty());
        s.dispatch(key(0.0, 30.0)).unwrap();
        assert!(!s.can_redo());
    }
    #[test]
    fn interpolation_is_deterministic_when_scrubbing_backwards() {
        let mut s = Session::new(doc()).unwrap();
        s.dispatch(Command::Batch {
            commands: vec![key(0.0, 0.0), key(1.0, 90.0)],
        })
        .unwrap();
        assert_eq!(local_pose(s.document(), Some(0), 0.5)[0].rotation, 45.0);
        let a = evaluate(s.document(), Some(0), 0.25);
        evaluate(s.document(), Some(0), 0.9);
        assert_eq!(a[0].a, evaluate(s.document(), Some(0), 0.25)[0].a);
    }
    #[test]
    fn hierarchy_composes_rotation_and_nonuniform_scale() {
        let mut d = doc();
        d.bones[0].setup.rotation = 90.0;
        d.bones[0].setup.scale_x = 2.0;
        d.bones.push(Bone {
            name: "child".into(),
            parent: Some(0),
            length: 10.0,
            setup: Transform {
                x: 10.0,
                ..Default::default()
            },
        });
        let w = evaluate(&d, None, 0.0);
        assert!(w[1].x.abs() < 0.0001);
        assert!((w[1].y - 20.0).abs() < 0.0001);
        let p = w[0].inverse_point(w[1].x, w[1].y).unwrap();
        assert!((p[0] - 10.0).abs() < 0.0001);
    }
    #[test]
    fn stepped_and_bezier_curves() {
        assert_eq!(curve_progress(Curve::Stepped, 0.8), 0.0);
        assert!((curve_progress(Curve::Bezier([0.42, 0.0, 0.58, 1.0]), 0.5) - 0.5).abs() < 0.0001);
        assert!(curve_progress(Curve::Bezier([0.42, 0.0, 0.58, 1.0]), 0.2) < 0.2);
    }
    #[test]
    fn validates_cycles_and_nonfinite_transforms() {
        let mut d = doc();
        d.bones[0].parent = Some(0);
        assert!(d.validate().is_err());
        d.bones[0].parent = None;
        d.bones[0].setup.x = f32::INFINITY;
        assert!(d.validate().is_err());
    }
}
