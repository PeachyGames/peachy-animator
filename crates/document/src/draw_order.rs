//! Draw order keys. Each key lists slots, in setup order, with the number of
//! places to move each from its setup index; unlisted slots fill the remaining
//! positions in setup order. A key without offsets restores setup order.
//! Observed against the external 4.3.13 runtime and installed Spine 4.3.26;
//! offsets are kept exactly as written (docs/slices/draw-order.md).
use crate::Document;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrawOrderKey {
    pub time: f32,
    /// Empty restores the setup draw order.
    pub offsets: Vec<DrawOrderOffset>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrawOrderOffset {
    pub slot: usize,
    pub offset: i32,
}

impl DrawOrderKey {
    /// Slot indices in drawing order (back to front).
    pub fn order(&self, slot_count: usize) -> Result<Vec<usize>, String> {
        let mut order = vec![usize::MAX; slot_count];
        let mut previous = None;
        for entry in &self.offsets {
            if entry.slot >= slot_count || previous.is_some_and(|p| entry.slot <= p) {
                return Err(
                    "Draw order offsets must name known slots in setup order, once each".into(),
                );
            }
            previous = Some(entry.slot);
            let target =
                i64::try_from(entry.slot).map_err(|_| "Too many slots")? + i64::from(entry.offset);
            let target = usize::try_from(target)
                .ok()
                .filter(|&t| t < slot_count && order[t] == usize::MAX)
                .ok_or(
                    "Draw order offsets must move slots to distinct positions within the slot list",
                )?;
            order[target] = entry.slot;
        }
        let listed: Vec<usize> = self.offsets.iter().map(|entry| entry.slot).collect();
        let mut unchanged = (0..slot_count).filter(|slot| !listed.contains(slot));
        for position in order.iter_mut().filter(|position| **position == usize::MAX) {
            *position = unchanged.next().ok_or("Draw order is not a permutation")?;
        }
        Ok(order)
    }
}

/// Slot draw order at `time`: the held key, or setup order before the first.
pub fn sample_draw_order(keys: &[DrawOrderKey], slot_count: usize, time: f32) -> Vec<usize> {
    keys.iter()
        .take_while(|key| key.time <= time)
        .last()
        .and_then(|key| key.order(slot_count).ok())
        .unwrap_or_else(|| (0..slot_count).collect())
}

impl Document {
    pub(crate) fn validate_draw_order(&self, animation: &crate::Animation) -> Result<(), String> {
        let mut previous = -1.0;
        for key in &animation.draw_order {
            if !key.time.is_finite() || key.time < 0.0 || key.time <= previous {
                return Err(format!(
                    "Animation {}: draw order key times must be finite, nonnegative and increasing",
                    animation.name
                ));
            }
            previous = key.time;
            key.order(self.slots.len())
                .map_err(|error| format!("Animation {}: {error}", animation.name))?;
        }
        Ok(())
    }
}
