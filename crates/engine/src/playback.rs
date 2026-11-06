//! Deterministic bounded transport. The shell supplies elapsed time; no clock lives here.
use peachy_document::Document;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlayDirection {
    #[default]
    Forward,
    Backward,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaybackStep {
    pub time: f32,
    pub playing: bool,
}

pub fn advance_playback(
    time: f32,
    elapsed: f32,
    duration: f32,
    looping: bool,
    direction: PlayDirection,
) -> PlaybackStep {
    // Empty and frame-zero-only clips have no interval to traverse. Stop at
    // zero in either direction, including Loop, without division or remainder.
    if !time.is_finite()
        || !elapsed.is_finite()
        || elapsed < 0.
        || !duration.is_finite()
        || duration <= 0.
    {
        return PlaybackStep {
            time: 0.,
            playing: false,
        };
    }
    let time = time.clamp(0., duration);
    if elapsed == 0. {
        return PlaybackStep {
            time,
            playing: true,
        };
    }
    let end = f64::from(duration);
    let next = f64::from(time)
        + f64::from(elapsed)
            * if direction == PlayDirection::Forward {
                1.
            } else {
                -1.
            };
    if looping {
        PlaybackStep {
            time: next.rem_euclid(end) as f32,
            playing: true,
        }
    } else {
        PlaybackStep {
            time: next.clamp(0., end) as f32,
            playing: if direction == PlayDirection::Forward {
                next < end
            } else {
                next > 0.
            },
        }
    }
}

/// Nearest strict key time across the active animation's supported tracks.
pub fn adjacent_key_time(
    doc: &Document,
    animation: usize,
    time: f32,
    direction: PlayDirection,
) -> Option<f32> {
    if !time.is_finite() {
        return None;
    }
    let clip = doc.animations.get(animation)?;
    let times = clip
        .tracks
        .iter()
        .flat_map(|t| t.keys.iter())
        .map(|k| k.time)
        .chain(
            clip.ik_tracks
                .iter()
                .flat_map(|t| t.keys.iter())
                .map(|key| key.time),
        )
        .filter(|t| *t != time);
    match direction {
        PlayDirection::Forward => times.filter(|t| *t > time).min_by(f32::total_cmp),
        PlayDirection::Backward => times.filter(|t| *t < time).max_by(f32::total_cmp),
    }
}
pub fn last_key_time(doc: &Document, animation: usize) -> f32 {
    doc.animations
        .get(animation)
        .map_or(0., |animation| animation.duration())
}
