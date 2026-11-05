//! Editable animation data. This crate knows nothing about files, windows or GPUs.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const FORMAT_VERSION: u32 = 1;
pub const KEY_TIME_EPSILON: f32 = 0.00001;
pub fn same_key_time(a: f32, b: f32) -> bool {
    (a - b).abs() < KEY_TIME_EPSILON
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub format: u32,
    pub name: String,
    pub fps: u32,
    pub bones: Vec<Bone>,
    pub slots: Vec<Slot>,
    pub skins: BTreeMap<String, Skin>,
    pub animations: Vec<Animation>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bone {
    pub name: String,
    pub parent: Option<usize>,
    pub length: f32,
    pub setup: Transform,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub name: String,
    pub bone: usize,
    pub attachment: Option<String>,
}

/// Slot name -> attachment placeholder -> region. Draw order is the slot vector.
pub type Skin = BTreeMap<String, BTreeMap<String, Region>>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub path: String,
    pub width: f32,
    pub height: f32,
    pub transform: Transform,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Animation {
    pub name: String,
    pub duration: f32,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub bone: usize,
    pub property: Property,
    pub keys: Vec<Keyframe>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Property {
    X,
    Y,
    Rotation,
    ScaleX,
    ScaleY,
}

impl Property {
    pub const ALL: [Self; 5] = [Self::X, Self::Y, Self::Rotation, Self::ScaleX, Self::ScaleY];
    pub fn is_scale(self) -> bool {
        matches!(self, Self::ScaleX | Self::ScaleY)
    }
    pub fn neutral(self) -> f32 {
        if matches!(self, Self::ScaleX | Self::ScaleY) {
            1.0
        } else {
            0.0
        }
    }
    pub fn read(self, t: Transform) -> f32 {
        match self {
            Self::X => t.x,
            Self::Y => t.y,
            Self::Rotation => t.rotation,
            Self::ScaleX => t.scale_x,
            Self::ScaleY => t.scale_y,
        }
    }
    pub fn write(self, t: &mut Transform, v: f32) {
        match self {
            Self::X => t.x = v,
            Self::Y => t.y = v,
            Self::Rotation => t.rotation = v,
            Self::ScaleX => t.scale_x = v,
            Self::ScaleY => t.scale_y = v,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Keyframe {
    pub time: f32,
    /// Translation/rotation are offsets from setup; scales multiply setup.
    pub value: f32,
    pub curve: Curve,
}

#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Curve {
    #[default]
    Linear,
    Stepped,
    /// Normalized time/value control points.
    Bezier([f32; 4]),
}

impl Document {
    pub fn validate(&self) -> Result<(), String> {
        if self.format != FORMAT_VERSION {
            return Err(format!("Unsupported project format {}", self.format));
        }
        if self.name.trim().is_empty() || self.fps == 0 || self.fps > 240 {
            return Err("Name is required; FPS must be 1–240".into());
        }
        if self.bones.is_empty() {
            return Err("A document needs at least one root bone".into());
        }
        let mut names = BTreeSet::new();
        for (i, b) in self.bones.iter().enumerate() {
            if b.name.trim().is_empty() || !names.insert(&b.name) {
                return Err("Bone names must be nonempty and unique".into());
            }
            if b.parent.is_some_and(|p| p >= i) {
                return Err(format!("Bone {} must follow its parent", b.name));
            }
            if !b.length.is_finite() || b.length < 0.0 || !finite_transform(b.setup) {
                return Err(format!("Invalid transform for {}", b.name));
            }
        }
        names.clear();
        for s in &self.slots {
            if s.name.trim().is_empty() || !names.insert(&s.name) || s.bone >= self.bones.len() {
                return Err("Invalid slot or duplicate slot name".into());
            }
        }
        for skin in self.skins.values() {
            for (slot, attachments) in skin {
                if !self.slots.iter().any(|s| &s.name == slot) {
                    return Err(format!("Unknown slot {slot}"));
                }
                for (placeholder, region) in attachments {
                    let name = region.name.as_ref().unwrap_or(placeholder);
                    if name.trim().is_empty() {
                        return Err("Attachment names must be nonempty".into());
                    }
                    if !region.width.is_finite()
                        || !region.height.is_finite()
                        || region.width <= 0.0
                        || region.height <= 0.0
                        || !finite_transform(region.transform)
                        || !safe_asset_path(&region.path)
                    {
                        return Err("Invalid region dimensions, transform or asset path".into());
                    }
                }
            }
        }
        let mut animations = BTreeSet::new();
        for a in &self.animations {
            if a.name.trim().is_empty()
                || !animations.insert(&a.name)
                || !a.duration.is_finite()
                || a.duration <= 0.0
            {
                return Err("Animation names must be unique; duration must be positive".into());
            }
            let mut tracks = BTreeSet::new();
            for t in &a.tracks {
                if t.bone >= self.bones.len() || !tracks.insert((t.bone, t.property)) {
                    return Err("Invalid or duplicate track".into());
                }
                let mut previous = -1.0;
                for k in &t.keys {
                    if !k.time.is_finite()
                        || !k.value.is_finite()
                        || k.time < 0.0
                        || k.time > a.duration
                        || k.time <= previous
                    {
                        return Err("Keys must be finite, ordered and inside the animation".into());
                    }
                    if let Curve::Bezier(c) = k.curve
                        && (!c.iter().all(|v| v.is_finite())
                            || !(0.0..=1.0).contains(&c[0])
                            || !(0.0..=1.0).contains(&c[2])
                            || c[0] > c[2])
                    {
                        return Err("Bezier time handles must be ordered within 0–1".into());
                    }
                    previous = k.time;
                }
            }
        }
        Ok(())
    }

    pub fn image_paths(&self) -> BTreeSet<&str> {
        self.skins
            .values()
            .flat_map(|s| s.values())
            .flat_map(|a| a.values())
            .map(|r| r.path.as_str())
            .collect()
    }
    pub fn region(&self, skin: &str, slot: usize) -> Option<&Region> {
        let slot = self.slots.get(slot)?;
        let attachment = slot.attachment.as_ref()?;
        self.skins
            .get(skin)
            .and_then(|s| s.get(&slot.name))
            .and_then(|r| r.get(attachment))
            .or_else(|| {
                self.skins
                    .get("default")
                    .and_then(|s| s.get(&slot.name))
                    .and_then(|r| r.get(attachment))
            })
    }
}

pub fn safe_asset_path(path: &str) -> bool {
    !path.is_empty()
        && !path.chars().any(char::is_control)
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

pub fn finite_transform(t: Transform) -> bool {
    [t.x, t.y, t.rotation, t.scale_x, t.scale_y]
        .iter()
        .all(|v| v.is_finite())
}
