//! Spine path attachments and path constraints as complete data, plus keyed
//! position and spacing. Defaults and field names were observed through the
//! external 4.3.13 runtime's public data API; evaluation is a bounded engine
//! capability fitted black-box (docs/slices/path-constraints.md).
use crate::{Bone, Keyframe, MeshVertices};
use serde::{Deserialize, Serialize};

/// A composite cubic Bezier: each knot is (in handle, point, out handle).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Path {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub vertices: MeshVertices,
    pub closed: bool,
    pub constant_speed: bool,
    /// Setup-pose cumulative lengths per curve, as exported by Spine.
    pub lengths: Vec<f32>,
    /// Nonessential editor color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<[u8; 4]>,
}

impl Path {
    pub fn vertex_count(&self) -> usize {
        match &self.vertices {
            MeshVertices::SlotLocal { positions } => positions.len(),
            MeshVertices::Weighted { influences } => influences.len(),
        }
    }

    pub fn validate(&self, bone_count: usize) -> Result<(), String> {
        let valid = match &self.vertices {
            MeshVertices::SlotLocal { positions } => {
                positions.iter().flatten().all(|v| v.is_finite())
            }
            MeshVertices::Weighted { influences } => influences.iter().all(|vertex| {
                !vertex.is_empty()
                    && vertex.iter().all(|i| {
                        i.bone < bone_count
                            && i.position.iter().all(|v| v.is_finite())
                            && i.weight.is_finite()
                    })
            }),
        };
        let n = self.vertex_count();
        if !valid || n < 6 || !n.is_multiple_of(3) || !self.lengths.iter().all(|l| l.is_finite()) {
            return Err("Paths need finite vertices in knots of three (at least two knots), known bones and finite lengths".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PositionMode {
    Fixed,
    #[default]
    Percent,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpacingMode {
    #[default]
    Length,
    Fixed,
    Percent,
    Proportional,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RotateMode {
    #[default]
    Tangent,
    Chain,
    ChainScale,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathSetup {
    pub position: f32,
    pub spacing: f32,
    pub mix_rotate: f32,
    pub mix_x: f32,
    pub mix_y: f32,
}

impl Default for PathSetup {
    fn default() -> Self {
        Self {
            position: 0.0,
            spacing: 0.0,
            mix_rotate: 1.0,
            mix_x: 1.0,
            mix_y: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathConstraint {
    pub name: String,
    pub bones: Vec<usize>,
    /// Slot whose shown path attachment the bones follow.
    pub slot: usize,
    pub position_mode: PositionMode,
    pub spacing_mode: SpacingMode,
    pub rotate_mode: RotateMode,
    pub offset_rotation: f32,
    pub setup: PathSetup,
}

impl PathConstraint {
    pub fn validate(&self, bones: &[Bone], slot_count: usize) -> Result<(), String> {
        let context = || format!("Path {}", self.name);
        if self.name.trim().is_empty() {
            return Err("Constraint names must be nonempty".into());
        }
        if self.bones.is_empty()
            || self.bones.iter().any(|&b| b >= bones.len())
            || self.slot >= slot_count
        {
            return Err(format!("{}: needs known bones and a known slot", context()));
        }
        let PathSetup {
            position,
            spacing,
            mix_rotate,
            mix_x,
            mix_y,
        } = self.setup;
        if ![
            self.offset_rotation,
            position,
            spacing,
            mix_rotate,
            mix_x,
            mix_y,
        ]
        .iter()
        .all(|v| v.is_finite())
        {
            return Err(format!("{}: values must be finite", context()));
        }
        Ok(())
    }

    /// The bounded subset the engine evaluates: tangent rotation and length,
    /// fixed or percent spacing on open, constant-speed paths (checked where
    /// the path is resolved).
    pub fn supported_subset(&self) -> Result<(), String> {
        if self.rotate_mode != RotateMode::Tangent || self.spacing_mode == SpacingMode::Proportional
        {
            return Err(format!(
                "Path {}: chain rotation and proportional spacing are unsupported",
                self.name
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PathProperty {
    Position,
    Spacing,
}

impl PathProperty {
    pub fn timeline(self) -> &'static str {
        match self {
            Self::Position => "position",
            Self::Spacing => "spacing",
        }
    }
}

/// Keyed position or spacing for one path constraint (absolute values).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathTrack {
    pub constraint: usize,
    pub property: PathProperty,
    pub keys: Vec<Keyframe>,
}
