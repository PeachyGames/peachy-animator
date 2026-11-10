//! Image sequences on regions and meshes, their animation keys, and slot blend
//! modes. Observed with the external 4.3.13 runtime's public API
//! (docs/slices/sequences-and-blending.md).
use crate::AttachmentRef;
use serde::{Deserialize, Serialize};

/// Frames are the base path plus (start + index), zero-padded to `digits`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SequenceDef {
    pub count: u32,
    pub start: u32,
    pub digits: u32,
    /// Frame shown in Setup and before an animation's first key.
    pub setup_index: u32,
}

impl SequenceDef {
    pub fn frame_path(&self, path: &str, index: u32) -> String {
        format!(
            "{path}{:0width$}",
            self.start + index,
            width = self.digits as usize
        )
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.count == 0 || self.setup_index >= self.count {
            return Err("Sequences need at least one frame and a setup index inside them".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SequenceMode {
    #[default]
    Hold,
    Once,
    Loop,
    Pingpong,
    OnceReverse,
    LoopReverse,
    PingpongReverse,
}

impl SequenceMode {
    pub const ALL: [Self; 7] = [
        Self::Hold,
        Self::Once,
        Self::Loop,
        Self::Pingpong,
        Self::OnceReverse,
        Self::LoopReverse,
        Self::PingpongReverse,
    ];
    pub fn spine_name(self) -> &'static str {
        match self {
            Self::Hold => "hold",
            Self::Once => "once",
            Self::Loop => "loop",
            Self::Pingpong => "pingpong",
            Self::OnceReverse => "onceReverse",
            Self::LoopReverse => "loopReverse",
            Self::PingpongReverse => "pingpongReverse",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SequenceKey {
    pub time: f32,
    pub mode: SequenceMode,
    pub index: u32,
    /// Seconds per frame.
    pub delay: f32,
}

impl SequenceKey {
    /// Frame index at `time` (at or after this key). Observed: frames advance
    /// by floor(elapsed / delay + 1e-5); once clamps, loop wraps, pingpong
    /// reflects over 2·(count − 1); reverse modes map f to count − 1 − f.
    pub fn frame(&self, time: f32, count: u32) -> u32 {
        let steps = if self.delay > 0.0 {
            ((f64::from(time) - f64::from(self.time)) / f64::from(self.delay) + 1e-5)
                .floor()
                .max(0.0) as u64
        } else {
            0
        };
        let count = u64::from(count.max(1));
        let last = count - 1;
        let at = u64::from(self.index) + steps;
        let pingpong = |at: u64| {
            if count == 1 {
                return 0;
            }
            let period = 2 * last;
            let p = at % period;
            if p >= count { period - p } else { p }
        };
        let forward = match self.mode {
            SequenceMode::Hold => return self.index.min(last as u32),
            SequenceMode::Once | SequenceMode::OnceReverse => at.min(last),
            SequenceMode::Loop | SequenceMode::LoopReverse => at % count,
            SequenceMode::Pingpong | SequenceMode::PingpongReverse => pingpong(at),
        };
        let reverse = matches!(
            self.mode,
            SequenceMode::OnceReverse | SequenceMode::LoopReverse | SequenceMode::PingpongReverse
        );
        (if reverse { last - forward } else { forward }) as u32
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SequenceTrack {
    pub attachment: AttachmentRef,
    pub keys: Vec<SequenceKey>,
}

impl SequenceTrack {
    /// `None` before the first key (Setup frame).
    pub fn sample(&self, time: f32, count: u32) -> Option<u32> {
        self.keys
            .iter()
            .take_while(|key| key.time <= time)
            .last()
            .map(|key| key.frame(time, count))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BlendMode {
    #[default]
    Normal,
    Additive,
    Multiply,
    Screen,
}

impl BlendMode {
    pub fn spine_name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Additive => "additive",
            Self::Multiply => "multiply",
            Self::Screen => "screen",
        }
    }
    pub fn is_normal(&self) -> bool {
        *self == Self::Normal
    }
}
