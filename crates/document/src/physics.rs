//! Physics constraint data and keyed settings. This slice stores, validates and
//! edits authored data only; it does not claim an editor simulation.
//!
//! Defaults and field names were observed from installed Spine 4.3.26 JSON
//! import/re-export and the separately licensed 4.3.13 runtime's public data API,
//! using original probe files. See docs/PROVENANCE.md.
use crate::{Bone, Keyframe};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicsConstraint {
    pub name: String,
    pub bone: usize,
    /// How much of each bone property physics affects. Zero follows the
    /// unconstrained pose for that property.
    pub influence: PhysicsInfluence,
    pub scale_y_mode: ScaleYMode,
    /// Maximum translation speed that affects physics.
    pub limit: f32,
    /// Physics update rate in steps per second.
    pub fps: f32,
    pub setup: PhysicsSetup,
    /// Settings controlled by global ("") physics timelines.
    pub global: PhysicsGlobal,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicsInfluence {
    pub x: f32,
    pub y: f32,
    pub rotate: f32,
    pub scale_x: f32,
    pub shear_x: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScaleYMode {
    #[default]
    None,
    Uniform,
    Volume,
}

/// The keyable settings. Mass is stored as mass, never as its inverse.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicsSetup {
    pub inertia: f32,
    pub strength: f32,
    pub damping: f32,
    pub mass: f32,
    pub wind: f32,
    pub gravity: f32,
    pub mix: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicsGlobal {
    pub inertia: bool,
    pub strength: bool,
    pub damping: bool,
    pub mass: bool,
    pub wind: bool,
    pub gravity: bool,
    pub mix: bool,
}

/// Observed Spine 4.3 defaults for an omitted field.
pub const PHYSICS_DEFAULT_LIMIT: f32 = 5000.0;
pub const PHYSICS_DEFAULT_FPS: f32 = 60.0;
pub const PHYSICS_DEFAULT_SETUP: PhysicsSetup = PhysicsSetup {
    inertia: 0.5,
    strength: 100.0,
    damping: 0.85,
    mass: 1.0,
    wind: 0.0,
    gravity: 0.0,
    mix: 1.0,
};

impl PhysicsConstraint {
    /// A new constraint with Spine's observed defaults and no affected property.
    pub fn new(name: impl Into<String>, bone: usize) -> Self {
        Self {
            name: name.into(),
            bone,
            influence: PhysicsInfluence {
                x: 0.0,
                y: 0.0,
                rotate: 0.0,
                scale_x: 0.0,
                shear_x: 0.0,
            },
            scale_y_mode: ScaleYMode::None,
            limit: PHYSICS_DEFAULT_LIMIT,
            fps: PHYSICS_DEFAULT_FPS,
            setup: PHYSICS_DEFAULT_SETUP,
            global: PhysicsGlobal::default(),
        }
    }

    pub fn validate(&self, bones: &[Bone]) -> Result<(), String> {
        if self.bone >= bones.len() {
            return Err(format!("Physics {}: unknown bone", self.name));
        }
        self.validate_settings()
    }

    /// Name and value checks that need no bone context.
    pub fn validate_settings(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Constraint names must be nonempty".into());
        }
        let PhysicsInfluence {
            x,
            y,
            rotate,
            scale_x,
            shear_x,
        } = self.influence;
        if ![x, y, rotate, scale_x, shear_x]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err(format!(
                "Physics {}: property influences must be finite",
                self.name
            ));
        }
        if !self.limit.is_finite() || self.limit < 0.0 {
            return Err(format!(
                "Physics {}: limit must be finite and nonnegative",
                self.name
            ));
        }
        if !self.fps.is_finite() || self.fps <= 0.0 {
            return Err(format!(
                "Physics {}: FPS must be finite and positive",
                self.name
            ));
        }
        for property in PhysicsProperty::ALL {
            property
                .check(property.read(self.setup))
                .map_err(|error| format!("Physics {}: {error}", self.name))?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PhysicsProperty {
    Inertia,
    Strength,
    Damping,
    Mass,
    Wind,
    Gravity,
    Mix,
}

impl PhysicsProperty {
    pub const ALL: [Self; 7] = [
        Self::Inertia,
        Self::Strength,
        Self::Damping,
        Self::Mass,
        Self::Wind,
        Self::Gravity,
        Self::Mix,
    ];
    /// Spine timeline name and observed omitted key value.
    pub fn timeline(self) -> &'static str {
        match self {
            Self::Inertia => "inertia",
            Self::Strength => "strength",
            Self::Damping => "damping",
            Self::Mass => "mass",
            Self::Wind => "wind",
            Self::Gravity => "gravity",
            Self::Mix => "mix",
        }
    }
    pub fn from_timeline(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.timeline() == name)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Inertia => "Inertia",
            Self::Strength => "Strength",
            Self::Damping => "Damping",
            Self::Mass => "Mass",
            Self::Wind => "Wind",
            Self::Gravity => "Gravity",
            Self::Mix => "Mix",
        }
    }
    /// Value used by Spine 4.3 for a key without `value`.
    pub fn omitted_key_value(self) -> f32 {
        if self == Self::Mix { 1.0 } else { 0.0 }
    }
    pub fn read(self, setup: PhysicsSetup) -> f32 {
        match self {
            Self::Inertia => setup.inertia,
            Self::Strength => setup.strength,
            Self::Damping => setup.damping,
            Self::Mass => setup.mass,
            Self::Wind => setup.wind,
            Self::Gravity => setup.gravity,
            Self::Mix => setup.mix,
        }
    }
    pub fn write(self, setup: &mut PhysicsSetup, value: f32) {
        match self {
            Self::Inertia => setup.inertia = value,
            Self::Strength => setup.strength = value,
            Self::Damping => setup.damping = value,
            Self::Mass => setup.mass = value,
            Self::Wind => setup.wind = value,
            Self::Gravity => setup.gravity = value,
            Self::Mix => setup.mix = value,
        }
    }
    pub fn global(self, global: PhysicsGlobal) -> bool {
        match self {
            Self::Inertia => global.inertia,
            Self::Strength => global.strength,
            Self::Damping => global.damping,
            Self::Mass => global.mass,
            Self::Wind => global.wind,
            Self::Gravity => global.gravity,
            Self::Mix => global.mix,
        }
    }
    pub fn set_global(self, global: &mut PhysicsGlobal, value: bool) {
        match self {
            Self::Inertia => global.inertia = value,
            Self::Strength => global.strength = value,
            Self::Damping => global.damping = value,
            Self::Mass => global.mass = value,
            Self::Wind => global.wind = value,
            Self::Gravity => global.gravity = value,
            Self::Mix => global.mix = value,
        }
    }
    /// Setup values: finite, with positive mass and nonnegative mix. Bounded
    /// local policy; Spine's documented mix is "a percentage (0+)".
    pub fn check(self, value: f32) -> Result<(), String> {
        if !value.is_finite() {
            return Err(format!("{} must be finite", self.label()));
        }
        match self {
            Self::Mass if value <= 0.0 => Err("Mass must be positive".into()),
            Self::Mix if value < 0.0 => Err("Mix must be nonnegative".into()),
            _ => Ok(()),
        }
    }
}

/// Keys for one setting. `constraint: None` is Spine's global ("") timeline,
/// which affects constraints whose matching Global flag is set. Values are
/// absolute settings, not offsets from Setup.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicsTrack {
    pub constraint: Option<usize>,
    pub property: PhysicsProperty,
    pub keys: Vec<Keyframe>,
}

/// Reset key times for one constraint, or all physics constraints when `None`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicsReset {
    pub constraint: Option<usize>,
    pub times: Vec<f32>,
}

pub(crate) fn validate_keys(keys: &[Keyframe]) -> Result<(), String> {
    let mut previous = -1.0;
    for k in keys {
        if !k.time.is_finite() || !k.value.is_finite() || k.time < 0.0 || k.time <= previous {
            return Err("Keys must be finite, nonnegative and ordered".into());
        }
        if !k.curve.handles_valid() {
            return Err("Bezier handles must be finite".into());
        }
        previous = k.time;
    }
    Ok(())
}
