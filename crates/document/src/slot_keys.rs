//! Slot attachment and color keys. Attachment keys are stepped (a missing name
//! hides the slot); color keys interpolate each channel with its own curve.
//! Before an animation's first key the slot shows its Setup attachment/color.
//! Observed with installed Spine 4.3.26 and the external 4.3.13 runtime
//! (docs/slices/slot-keys.md).
use crate::{Curve, Document};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentTrack {
    pub slot: usize,
    pub keys: Vec<AttachmentKey>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentKey {
    pub time: f32,
    /// Placeholder name to show, or `None` to hide the slot.
    pub attachment: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColorTrack {
    pub slot: usize,
    pub keys: Vec<ColorKey>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ColorKey {
    pub time: f32,
    /// Exact unpremultiplied RGBA bytes, like setup slot color.
    pub color: [u8; 4],
    /// One curve per channel (R, G, B, A) to the next key.
    pub curves: [Curve; 4],
}

impl AttachmentTrack {
    /// `None` before the first key (Setup); otherwise the held key's value.
    pub fn sample(&self, time: f32) -> Option<Option<&str>> {
        self.keys
            .iter()
            .take_while(|key| key.time <= time)
            .last()
            .map(|key| key.attachment.as_deref())
    }
}

impl Document {
    pub(crate) fn validate_slot_keys(&self, animation: &crate::Animation) -> Result<(), String> {
        let context = || format!("Animation {}", animation.name);
        fn ordered(times: impl Iterator<Item = f32>) -> bool {
            let mut previous = -1.0;
            for time in times {
                if !time.is_finite() || time < 0.0 || time <= previous {
                    return false;
                }
                previous = time;
            }
            true
        }
        let mut seen = BTreeSet::new();
        for track in &animation.attachment_tracks {
            if track.slot >= self.slots.len() || !seen.insert(track.slot) {
                return Err(format!(
                    "{}: invalid or duplicate attachment track",
                    context()
                ));
            }
            if !ordered(track.keys.iter().map(|k| k.time))
                || track
                    .keys
                    .iter()
                    .any(|k| k.attachment.as_ref().is_some_and(|n| n.trim().is_empty()))
            {
                return Err(format!(
                    "{}: attachment keys must be ordered with nonempty names",
                    context()
                ));
            }
        }
        let mut seen = BTreeSet::new();
        for track in &animation.color_tracks {
            if track.slot >= self.slots.len() || !seen.insert(track.slot) {
                return Err(format!("{}: invalid or duplicate color track", context()));
            }
            if !ordered(track.keys.iter().map(|k| k.time)) {
                return Err(format!("{}: color keys must be ordered", context()));
            }
            for key in &track.keys {
                let stepped = key.curves.iter().filter(|c| **c == Curve::Stepped).count();
                if stepped != 0 && stepped != 4 {
                    return Err(format!(
                        "{}: a stepped color key must be stepped on every channel",
                        context()
                    ));
                }
                for curve in key.curves {
                    if let Curve::Bezier(c) = curve
                        && (!c.iter().all(|v| v.is_finite())
                            || !(0.0..=1.0).contains(&c[0])
                            || !(0.0..=1.0).contains(&c[2]))
                    {
                        return Err(
                            "Bezier time handles must lie within the key interval (0–1)".into()
                        );
                    }
                }
            }
        }
        Ok(())
    }
}
