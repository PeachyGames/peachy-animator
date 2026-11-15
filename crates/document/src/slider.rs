//! Complete Slider data. Evaluation capability is separate from raw storage.
//! Fields/defaults are independently observed from original installed 4.3.26
//! exports and the external 4.3.13 runtime's public data API.
use crate::{Bone, Constraint, Document, Keyframe, TransformProperty};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SliderConstraint {
    pub name: String,
    /// Stable append-only animation index; interchange resolves its name.
    pub animation: usize,
    #[serde(deserialize_with = "required_optional")]
    pub bone: Option<usize>,
    #[serde(deserialize_with = "required_source_property")]
    pub property: Option<TransformProperty>,
    pub local: bool,
    #[serde(rename = "loop")]
    pub looped: bool,
    pub additive: bool,
    /// Source-property minimum, independent of the target-time minimum.
    pub from: f32,
    /// Target-time minimum in seconds; never the manual setup time.
    pub to: f32,
    pub scale: f32,
    /// Preserved range metadata, including zero when nonessential data is absent.
    pub max: f32,
    pub setup: SliderSetup,
}

// Complete native payloads require nullable fields to be present. An Option
// without this hook would accept an absent field as an implicit default.
fn required_optional<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

fn required_source_property<'de, D>(deserializer: D) -> Result<Option<TransformProperty>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?
        .map(|name| {
            TransformProperty::from_spine_name(&name).ok_or_else(|| {
                serde::de::Error::unknown_variant(
                    &name,
                    &["rotate", "x", "y", "scaleX", "scaleY", "shearY"],
                )
            })
        })
        .transpose()
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SliderSetup {
    /// Referenced animation time in seconds, independent of the host cursor.
    pub time: f32,
    pub mix: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SliderProperty {
    Time,
    Mix,
}

impl<'de> Deserialize<'de> for SliderProperty {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match String::deserialize(deserializer)?.as_str() {
            "time" => Ok(Self::Time),
            "mix" => Ok(Self::Mix),
            other => Err(serde::de::Error::unknown_variant(other, &["time", "mix"])),
        }
    }
}

impl SliderProperty {
    pub const ALL: [Self; 2] = [Self::Time, Self::Mix];

    pub fn read(self, setup: SliderSetup) -> f32 {
        match self {
            Self::Time => setup.time,
            Self::Mix => setup.mix,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SliderTrack {
    pub constraint: usize,
    pub property: SliderProperty,
    /// Absolute referenced seconds or Mix; values are not bone offsets.
    pub keys: Vec<Keyframe>,
}

impl SliderConstraint {
    /// Independently measured manual defaults. No implicit serde defaults.
    pub fn new(name: impl Into<String>, animation: usize) -> Self {
        Self {
            name: name.into(),
            animation,
            bone: None,
            property: None,
            local: false,
            looped: false,
            additive: false,
            from: 0.,
            to: 0.,
            scale: 0.,
            max: 0.,
            setup: SliderSetup { time: 0., mix: 1. },
        }
    }

    pub fn validate(&self, bones: &[Bone]) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Constraint names must be nonempty".into());
        }
        if self.bone.is_some_and(|bone| bone >= bones.len()) {
            return Err(format!("Slider {}: unknown source bone", self.name));
        }
        if self.bone.is_some() && self.property.is_none() {
            return Err(format!(
                "Slider {}: a source bone needs a property",
                self.name
            ));
        }
        if ![
            self.from,
            self.to,
            self.scale,
            self.max,
            self.setup.time,
            self.setup.mix,
        ]
        .into_iter()
        .all(f32::is_finite)
        {
            return Err(format!("Slider {}: settings must be finite", self.name));
        }
        Ok(())
    }
}

impl Document {
    pub(crate) fn validate_sliders(&self) -> Result<(), String> {
        for constraint in &self.constraints {
            if let Constraint::Slider(slider) = constraint
                && slider.animation >= self.animations.len()
            {
                return Err(format!(
                    "Slider {}: unknown referenced animation",
                    slider.name
                ));
            }
        }
        for animation in &self.animations {
            let mut seen = BTreeSet::new();
            for track in &animation.slider_tracks {
                if !matches!(
                    self.constraints.get(track.constraint),
                    Some(Constraint::Slider(_))
                ) || !seen.insert((track.constraint, track.property))
                {
                    return Err(format!(
                        "Animation {}: Slider tracks need a Slider constraint, once per property",
                        animation.name
                    ));
                }
                crate::physics::validate_keys(&track.keys)?;
            }
        }
        // A Slider animation may key other Sliders. No unobserved blanket
        // graph-cycle ban or recursive evaluation rule belongs in raw validation.
        Ok(())
    }
}

#[cfg(test)]
#[path = "slider_tests.rs"]
mod tests;
