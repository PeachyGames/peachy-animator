//! Bone transform inheritance modes and their keyed (stepped) timelines.
//! Names and the omitted-key value were observed through installed Spine 4.3.26
//! and the external 4.3.13 runtime; see docs/slices/inherit-modes.md.
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub enum Inherit {
    #[default]
    Normal,
    OnlyTranslation,
    NoRotationOrReflection,
    NoScale,
    NoScaleOrReflection,
}

impl Inherit {
    pub const ALL: [Self; 5] = [
        Self::Normal,
        Self::OnlyTranslation,
        Self::NoRotationOrReflection,
        Self::NoScale,
        Self::NoScaleOrReflection,
    ];
    /// Spine 4.3 JSON spelling.
    pub fn spine_name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::OnlyTranslation => "onlyTranslation",
            Self::NoRotationOrReflection => "noRotationOrReflection",
            Self::NoScale => "noScale",
            Self::NoScaleOrReflection => "noScaleOrReflection",
        }
    }
    pub fn from_spine_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.spine_name() == name)
    }
    /// Spine's Tree terms: (inherit rotation, inherit scale, inherit reflection).
    pub fn flags(self) -> (bool, bool, bool) {
        match self {
            Self::Normal => (true, true, true),
            Self::OnlyTranslation => (false, false, false),
            Self::NoRotationOrReflection => (false, true, false),
            Self::NoScale => (true, false, true),
            Self::NoScaleOrReflection => (true, false, false),
        }
    }
    /// The mode for a checkbox combination. Reflection only distinguishes the
    /// two no-scale modes; the other combinations determine it.
    pub fn from_flags(rotation: bool, scale: bool, reflection: bool) -> Self {
        match (rotation, scale) {
            (true, true) => Self::Normal,
            (false, false) => Self::OnlyTranslation,
            (false, true) => Self::NoRotationOrReflection,
            (true, false) if reflection => Self::NoScale,
            (true, false) => Self::NoScaleOrReflection,
        }
    }
}

/// Stepped inherit keys for one bone. Before the first key the bone uses its
/// Setup mode; each key holds until the next.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InheritTrack {
    pub bone: usize,
    pub keys: Vec<InheritKey>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InheritKey {
    pub time: f32,
    pub inherit: Inherit,
}

impl InheritTrack {
    /// The keyed mode at `time`, or `None` before the first key.
    pub fn sample(&self, time: f32) -> Option<Inherit> {
        self.keys
            .iter()
            .take_while(|key| key.time <= time)
            .last()
            .map(|key| key.inherit)
    }
}
