//! Editable animation data. This crate knows nothing about files, windows or GPUs.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
mod deform;
mod ik;
mod inherit;
mod physics;
mod skin_scope;
mod transform;
pub use deform::{DeformKey, DeformTrack};
pub use ik::{Constraint, IkConstraint, IkCurve, IkKeyframe, IkSetup, IkTrack};
pub use inherit::{Inherit, InheritKey, InheritTrack};
pub use physics::{
    PHYSICS_DEFAULT_FPS, PHYSICS_DEFAULT_LIMIT, PHYSICS_DEFAULT_SETUP, PhysicsConstraint,
    PhysicsGlobal, PhysicsInfluence, PhysicsProperty, PhysicsReset, PhysicsSetup, PhysicsTrack,
    ScaleYMode,
};
pub use skin_scope::{Activity, SkinMembers, SkinScope};
pub use transform::{
    TransformConstraint, TransformMapping, TransformProperty, TransformTarget, TransformValues,
};

pub const FORMAT_VERSION: u32 = 13;
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
    pub constraints: Vec<Constraint>,
    pub slots: Vec<Slot>,
    pub skins: BTreeMap<String, Skin>,
    /// Skin-required bones/constraints and the named skins that list them.
    /// Required in native v12.
    pub skin_scope: SkinScope,
    pub animations: Vec<Animation>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bone {
    pub name: String,
    pub parent: Option<usize>,
    pub length: f32,
    pub setup: Transform,
    /// Setup transform inheritance. Required in native v10.
    pub inherit: Inherit,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub shear_x: f32,
    pub shear_y: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            shear_x: 0.0,
            shear_y: 0.0,
        }
    }
}

/// A region's own transform has no shear in the supported interchange format.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionTransform {
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
}

impl Default for RegionTransform {
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

impl From<RegionTransform> for Transform {
    fn from(region: RegionTransform) -> Self {
        Self {
            x: region.x,
            y: region.y,
            rotation: region.rotation,
            scale_x: region.scale_x,
            scale_y: region.scale_y,
            shear_x: 0.0,
            shear_y: 0.0,
        }
    }
}

impl TryFrom<Transform> for RegionTransform {
    type Error = String;
    fn try_from(transform: Transform) -> Result<Self, Self::Error> {
        if transform.shear_x != 0.0 || transform.shear_y != 0.0 {
            return Err("Region transforms do not support shear".into());
        }
        Ok(Self {
            x: transform.x,
            y: transform.y,
            rotation: transform.rotation,
            scale_x: transform.scale_x,
            scale_y: transform.scale_y,
        })
    }
}

pub fn finite_region_transform(transform: RegionTransform) -> bool {
    finite_transform(transform.into())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub name: String,
    pub bone: usize,
    pub attachment: Option<String>,
    /// Setup tint in exact, unpremultiplied RGBA bytes. Required in native v7.
    pub color: [u8; 4],
}

/// Slot name -> attachment placeholder -> attachment. Draw order is the slot vector.
pub type Skin = BTreeMap<String, BTreeMap<String, Attachment>>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Attachment {
    Region(Region),
    Mesh(Mesh),
}

impl Attachment {
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Region(r) => r.name.as_deref(),
            Self::Mesh(m) => m.name.as_deref(),
        }
    }
    pub fn path(&self) -> &str {
        match self {
            Self::Region(r) => &r.path,
            Self::Mesh(m) => &m.path,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if !safe_asset_path(self.path()) || self.name().is_some_and(|n| n.trim().is_empty()) {
            return Err("Invalid attachment name or asset path".into());
        }
        match self {
            Self::Region(r) => {
                if !r.width.is_finite()
                    || !r.height.is_finite()
                    || r.width <= 0.0
                    || r.height <= 0.0
                    || !finite_region_transform(r.transform)
                {
                    return Err("Invalid region dimensions or transform".into());
                }
                Ok(())
            }
            Self::Mesh(m) => m.validate(),
        }
    }
}

/// Bind positions are local to either the slot bone or each influencing bone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase", deny_unknown_fields)]
pub enum MeshVertices {
    SlotLocal { positions: Vec<[f32; 2]> },
    Weighted { influences: Vec<Vec<BoneInfluence>> },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoneInfluence {
    pub bone: usize,
    pub position: [f32; 2],
    pub weight: f32,
}

/// Triangulated mesh with one UV per logical vertex. UV origin is top-left.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mesh {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub path: String,
    pub vertices: MeshVertices,
    pub uvs: Vec<[f32; 2]>,
    pub triangles: Vec<u32>,
    pub hull: usize,
    /// Optional source-image size, zero when absent from a runtime-only export.
    #[serde(default)]
    pub width: f32,
    #[serde(default)]
    pub height: f32,
    /// Nonessential Spine edge data, in pairs of coordinate offsets (2 * vertex index).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<u32>,
}

impl Mesh {
    pub fn vertex_count(&self) -> usize {
        match &self.vertices {
            MeshVertices::SlotLocal { positions } => positions.len(),
            MeshVertices::Weighted { influences } => influences.len(),
        }
    }
    pub fn is_weighted(&self) -> bool {
        matches!(self.vertices, MeshVertices::Weighted { .. })
    }
    pub fn slot_local_positions(&self) -> Option<&[[f32; 2]]> {
        match &self.vertices {
            MeshVertices::SlotLocal { positions } => Some(positions),
            MeshVertices::Weighted { .. } => None,
        }
    }
    pub fn slot_local_positions_mut(&mut self) -> Option<&mut Vec<[f32; 2]>> {
        match &mut self.vertices {
            MeshVertices::SlotLocal { positions } => Some(positions),
            MeshVertices::Weighted { .. } => None,
        }
    }
    /// Bone-reference validation needs document context, including inactive skins.
    pub fn validate_bones(&self, bone_count: usize) -> Result<(), String> {
        if let MeshVertices::Weighted { influences } = &self.vertices
            && influences
                .iter()
                .flatten()
                .any(|influence| influence.bone >= bone_count)
        {
            return Err("Weighted mesh references an unknown bone".into());
        }
        Ok(())
    }
    pub fn validate(&self) -> Result<(), String> {
        let n = self.vertex_count();
        match &self.vertices {
            MeshVertices::SlotLocal { positions } => {
                if positions.iter().flatten().any(|v| !v.is_finite()) {
                    return Err("Mesh positions must be finite".into());
                }
            }
            MeshVertices::Weighted { influences } => {
                for vertex in influences {
                    if vertex.is_empty()
                        || vertex.iter().any(|influence| {
                            influence.position.iter().any(|v| !v.is_finite())
                                || !influence.weight.is_finite()
                                || !(0.0..=1.0).contains(&influence.weight)
                        })
                        || vertex
                            .iter()
                            .map(|influence| f64::from(influence.weight))
                            .sum::<f64>()
                            <= 0.0
                    {
                        return Err("Weighted vertices need finite bone-local positions and weights in 0–1 with a positive total".into());
                    }
                }
            }
        }
        if n < 3
            || n > u32::MAX as usize / 2
            || self.uvs.len() != n
            || !(3..=n).contains(&self.hull)
            || self
                .uvs
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || !self.width.is_finite()
            || !self.height.is_finite()
            || self.width < 0.0
            || self.height < 0.0
        {
            return Err("Mesh requires finite vertices, matching UVs in 0–1, a valid hull and nonnegative image size".into());
        }
        if self.triangles.is_empty() || !self.triangles.len().is_multiple_of(3) {
            return Err("Mesh triangles must contain index triplets".into());
        }
        let mut used = BTreeSet::new();
        let mut triangles = BTreeSet::new();
        for t in self.triangles.as_chunks::<3>().0 {
            if t.iter().any(|&i| i as usize >= n) || t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
                return Err("Invalid mesh triangle indices".into());
            }
            let mut sorted = [t[0], t[1], t[2]];
            sorted.sort_unstable();
            if !triangles.insert(sorted) {
                return Err("Duplicate mesh triangle".into());
            }
            used.extend(t.iter().copied());
        }
        if used.len() != n {
            return Err("Every mesh vertex must belong to a triangle".into());
        }
        if !self.edges.len().is_multiple_of(2)
            || self
                .edges
                .iter()
                .any(|&i| !i.is_multiple_of(2) || i as usize / 2 >= n)
        {
            return Err("Mesh edges must contain pairs of valid coordinate offsets".into());
        }
        Ok(())
    }
}

/// Explicit key used by commands; named skin references never silently edit a fallback.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentRef {
    pub skin: String,
    pub slot: String,
    pub placeholder: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub path: String,
    pub width: f32,
    pub height: f32,
    pub transform: RegionTransform,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Animation {
    pub name: String,
    pub tracks: Vec<Track>,
    /// Keyed IK Mix/Positive per constraint. Required in native v9.
    pub ik_tracks: Vec<IkTrack>,
    /// Keyed physics settings. Required in native v8.
    pub physics: Vec<PhysicsTrack>,
    /// Physics reset keys. Required in native v8.
    pub physics_resets: Vec<PhysicsReset>,
    /// Stepped bone inheritance keys. Required in native v10.
    pub inherit_tracks: Vec<InheritTrack>,
    /// Mesh deform keys. Required in native v13.
    pub deform_tracks: Vec<DeformTrack>,
}

impl Animation {
    pub fn new(name: impl Into<String>, tracks: Vec<Track>) -> Self {
        Self {
            name: name.into(),
            tracks,
            ik_tracks: vec![],
            physics: vec![],
            physics_resets: vec![],
            inherit_tracks: vec![],
            deform_tracks: vec![],
        }
    }
    /// The highest authored key time across validated, ordered timelines, or zero.
    pub fn duration(&self) -> f32 {
        let bones = self.tracks.iter().filter_map(|track| track.keys.last());
        let physics = self.physics.iter().filter_map(|track| track.keys.last());
        bones
            .chain(physics)
            .map(|key| key.time)
            .chain(
                self.ik_tracks
                    .iter()
                    .filter_map(|track| track.keys.last().map(|key| key.time)),
            )
            .chain(
                self.physics_resets
                    .iter()
                    .filter_map(|reset| reset.times.last().copied()),
            )
            .chain(
                self.inherit_tracks
                    .iter()
                    .filter_map(|track| track.keys.last().map(|key| key.time)),
            )
            .chain(
                self.deform_tracks
                    .iter()
                    .filter_map(|track| track.keys.last().map(|key| key.time)),
            )
            .fold(0.0, f32::max)
    }
    /// Every authored key time across bone, IK, physics and reset timelines.
    pub fn key_times(&self) -> impl Iterator<Item = f32> + '_ {
        self.tracks
            .iter()
            .flat_map(|track| track.keys.iter().map(|key| key.time))
            .chain(
                self.ik_tracks
                    .iter()
                    .flat_map(|track| track.keys.iter().map(|key| key.time)),
            )
            .chain(
                self.physics
                    .iter()
                    .flat_map(|track| track.keys.iter().map(|key| key.time)),
            )
            .chain(
                self.physics_resets
                    .iter()
                    .flat_map(|reset| reset.times.iter().copied()),
            )
            .chain(
                self.inherit_tracks
                    .iter()
                    .flat_map(|track| track.keys.iter().map(|key| key.time)),
            )
            .chain(
                self.deform_tracks
                    .iter()
                    .flat_map(|track| track.keys.iter().map(|key| key.time)),
            )
    }
    /// Whether any timeline other than bone transforms has keys.
    pub fn has_constraint_keys(&self) -> bool {
        self.ik_tracks.iter().any(|track| !track.keys.is_empty())
            || self.physics.iter().any(|track| !track.keys.is_empty())
            || self
                .physics_resets
                .iter()
                .any(|reset| !reset.times.is_empty())
            || self
                .inherit_tracks
                .iter()
                .any(|track| !track.keys.is_empty())
    }
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
    ShearX,
    ShearY,
}

impl Property {
    pub const ALL: [Self; 7] = [
        Self::X,
        Self::Y,
        Self::Rotation,
        Self::ScaleX,
        Self::ScaleY,
        Self::ShearX,
        Self::ShearY,
    ];
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
            Self::ShearX => t.shear_x,
            Self::ShearY => t.shear_y,
        }
    }
    pub fn write(self, t: &mut Transform, v: f32) {
        match self {
            Self::X => t.x = v,
            Self::Y => t.y = v,
            Self::Rotation => t.rotation = v,
            Self::ScaleX => t.scale_x = v,
            Self::ScaleY => t.scale_y = v,
            Self::ShearX => t.shear_x = v,
            Self::ShearY => t.shear_y = v,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Keyframe {
    pub time: f32,
    /// Translation/rotation/shear are offsets from setup; scales multiply setup.
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
        let mut constraint_names = BTreeSet::new();
        for constraint in &self.constraints {
            if !constraint_names.insert(constraint.name()) {
                return Err("Constraint names must be unique".into());
            }
            constraint.validate(&self.bones)?;
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
                for (placeholder, attachment) in attachments {
                    let name = attachment.name().unwrap_or(placeholder);
                    if name.trim().is_empty() {
                        return Err("Attachment names must be nonempty".into());
                    }
                    if placeholder.trim().is_empty() {
                        return Err("Attachment placeholders must be nonempty".into());
                    }
                    attachment.validate()?;
                    if let Attachment::Mesh(mesh) = attachment {
                        mesh.validate_bones(self.bones.len())?;
                    }
                }
            }
        }
        let mut animations = BTreeSet::new();
        for a in &self.animations {
            if a.name.trim().is_empty() || !animations.insert(&a.name) {
                return Err("Animation names must be nonempty and unique".into());
            }
            let mut tracks = BTreeSet::new();
            for t in &a.tracks {
                if t.bone >= self.bones.len() || !tracks.insert((t.bone, t.property)) {
                    return Err("Invalid or duplicate track".into());
                }
                physics::validate_keys(&t.keys)?;
            }
            let physics_target = |constraint: Option<usize>| match constraint {
                None => Ok(()),
                Some(index) => match self.constraints.get(index) {
                    Some(Constraint::Physics(_)) => Ok(()),
                    _ => Err(format!(
                        "Animation {}: physics keys must reference a physics constraint",
                        a.name
                    )),
                },
            };
            let mut physics = BTreeSet::new();
            for t in &a.physics {
                physics_target(t.constraint)?;
                if !physics.insert((t.constraint, t.property)) {
                    return Err(format!("Animation {}: duplicate physics track", a.name));
                }
                physics::validate_keys(&t.keys)?;
            }
            let mut resets = BTreeSet::new();
            for reset in &a.physics_resets {
                physics_target(reset.constraint)?;
                if !resets.insert(reset.constraint) {
                    return Err(format!(
                        "Animation {}: duplicate physics reset track",
                        a.name
                    ));
                }
                let mut previous = -1.0;
                for &time in &reset.times {
                    if !time.is_finite() || time < 0.0 || time <= previous {
                        return Err("Reset keys must be finite, nonnegative and ordered".into());
                    }
                    previous = time;
                }
            }
            let mut ik_tracks = BTreeSet::new();
            for track in &a.ik_tracks {
                if !matches!(
                    self.constraints.get(track.constraint),
                    Some(Constraint::Ik(_))
                ) || !ik_tracks.insert(track.constraint)
                {
                    return Err("IK tracks must reference distinct IK constraints".into());
                }
                let mut previous = -1.;
                for key in &track.keys {
                    key.validate()?;
                    if key.time <= previous {
                        return Err("IK keys must be strictly ordered".into());
                    }
                    previous = key.time;
                }
            }
            self.validate_deform(a)?;
            let mut inherit_bones = BTreeSet::new();
            for track in &a.inherit_tracks {
                if track.bone >= self.bones.len() || !inherit_bones.insert(track.bone) {
                    return Err(format!(
                        "Animation {}: invalid or duplicate inherit track",
                        a.name
                    ));
                }
                let mut previous = -1.0;
                for key in &track.keys {
                    if !key.time.is_finite() || key.time < 0.0 || key.time <= previous {
                        return Err("Inherit keys must be finite, nonnegative and ordered".into());
                    }
                    previous = key.time;
                }
            }
        }
        self.validate_skin_scope()?;
        Ok(())
    }

    pub fn image_paths(&self) -> BTreeSet<&str> {
        self.skins
            .values()
            .flat_map(|s| s.values())
            .flat_map(|a| a.values())
            .map(Attachment::path)
            .collect()
    }
    pub fn attachment(&self, key: &AttachmentRef) -> Option<&Attachment> {
        self.skins
            .get(&key.skin)?
            .get(&key.slot)?
            .get(&key.placeholder)
    }
    pub fn attachment_mut(&mut self, key: &AttachmentRef) -> Option<&mut Attachment> {
        self.skins
            .get_mut(&key.skin)?
            .get_mut(&key.slot)?
            .get_mut(&key.placeholder)
    }
    pub fn resolved_attachment(
        &self,
        skin: &str,
        slot: usize,
    ) -> Option<(AttachmentRef, &Attachment)> {
        let attachment = self.slots.get(slot)?.attachment.as_ref()?;
        self.resolve_attachment(skin, slot, attachment)
    }

    /// Resolve any placeholder, including ones not currently shown by the slot.
    pub fn resolve_attachment(
        &self,
        skin: &str,
        slot: usize,
        placeholder: &str,
    ) -> Option<(AttachmentRef, &Attachment)> {
        let slot = self.slots.get(slot)?;
        for name in [skin, "default"] {
            let key = AttachmentRef {
                skin: name.into(),
                slot: slot.name.clone(),
                placeholder: placeholder.into(),
            };
            if let Some(value) = self.attachment(&key) {
                return Some((key, value));
            }
        }
        None
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
    [
        t.x, t.y, t.rotation, t.scale_x, t.scale_y, t.shear_x, t.shear_y,
    ]
    .iter()
    .all(|v| v.is_finite())
}
