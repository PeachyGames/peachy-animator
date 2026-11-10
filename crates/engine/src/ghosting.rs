//! Pure, bounded onion-skin sampling. Frames are project frames, not wall-clock time.
use peachy_document::Document;

pub const MAX_GHOST_SAMPLES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GhostRange {
    pub before: f32,
    pub after: f32,
    pub step: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GhostSampling {
    pub frames: GhostRange,
    pub key_frames: GhostRange,
    pub current: bool,
    pub anchor: bool,
    /// The shell combines the Ghosting Loop setting with transport Repeat/Loop.
    pub looping: bool,
}

impl Default for GhostSampling {
    fn default() -> Self {
        Self {
            frames: GhostRange {
                before: 6.,
                after: 6.,
                step: 3.,
            },
            key_frames: GhostRange {
                before: 0.,
                after: 0.,
                step: 1.,
            },
            current: false,
            anchor: false,
            looping: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GhostSource {
    Frames,
    KeyFrames,
    Current,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GhostSide {
    Before,
    Current,
    After,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GhostSample {
    /// Evaluation time in seconds. Key ghosts preserve the exact native key time.
    pub time: f32,
    /// Signed, unwrapped distance from the cursor, in project frames.
    pub frame_offset: f32,
    pub source: GhostSource,
    pub side: GhostSide,
}

/// Sample committed animation times without changing the document or a Session.
///
/// Frames and Key Frames are independent sources, ordered Frames then Key Frames,
/// Before then After, farthest first within each side. Optional Current is last.
/// Zero reach disables that side. Key Step is a whole number: step N selects the
/// Nth, 2Nth, ... nearest unique authored key on each side. Close but distinct key
/// times remain distinct; this intentionally does not use authoring's epsilon.
///
/// Looping repeats keys across the key-derived duration. Authored 0/end keys at
/// the same repeated boundary count once: prefer the end pose at a positive
/// unwrapped boundary, otherwise frame zero when it exists. Frame ghosts preserve
/// an exact in-range end; out-of-range frames wrap through zero. Current uses the
/// exact cursor even past the last key (held pose); zero-duration clips only emit
/// Current at zero. More than 64 visible samples is an error, never truncation.
pub fn ghost_samples(
    doc: &Document,
    animation: usize,
    time: f32,
    settings: &GhostSampling,
) -> Result<Vec<GhostSample>, String> {
    if !time.is_finite() || time < 0. {
        return Err("Ghost cursor time must be finite and nonnegative".into());
    }
    if doc.fps == 0 || doc.fps > 240 {
        return Err("Ghost sampling requires project FPS in 1..=240".into());
    }
    let animation = doc
        .animations
        .get(animation)
        .ok_or("Unknown ghost animation")?;
    for range in [settings.frames, settings.key_frames] {
        if !range.before.is_finite()
            || !range.after.is_finite()
            || range.before < 0.
            || range.after < 0.
            || !range.step.is_finite()
            || range.step <= 0.
        {
            return Err("Ghost ranges must be finite and nonnegative, with a positive step".into());
        }
    }
    if settings.key_frames.step < 1. || settings.key_frames.step.fract() != 0. {
        return Err("Key frame step must be a positive whole number".into());
    }
    // A validated Session supplies ordered keys. Inspect their numeric domain too
    // so a malformed independent caller cannot hide NaN behind duration's max.
    let mut keys = Vec::new();
    for key in animation.tracks.iter().flat_map(|track| &track.keys) {
        if !key.time.is_finite() || key.time < 0. {
            return Err("Ghost key times must be finite and nonnegative".into());
        }
        if settings.key_frames.before > 0. || settings.key_frames.after > 0. {
            keys.push(key.time);
        }
    }
    let end = animation.duration();
    if !end.is_finite() || end < 0. {
        return Err("Ghost animation duration must be finite and nonnegative".into());
    }
    let mut result = Vec::new();
    if end > 0. {
        let fps = f64::from(doc.fps);
        for side in [GhostSide::Before, GhostSide::After] {
            frame_samples(&mut result, time, end, fps, settings, side)?;
        }
        keys.sort_by(f32::total_cmp);
        keys.dedup();
        for side in [GhostSide::Before, GhostSide::After] {
            key_samples(&mut result, &keys, time, end, fps, settings, side)?;
        }
    }
    if settings.current {
        push(
            &mut result,
            GhostSample {
                time: if end == 0. { 0. } else { time },
                frame_offset: 0.,
                source: GhostSource::Current,
                side: GhostSide::Current,
            },
        )?;
    }
    Ok(result)
}

fn push(samples: &mut Vec<GhostSample>, sample: GhostSample) -> Result<(), String> {
    if samples.len() == MAX_GHOST_SAMPLES {
        return Err(format!(
            "Ghosting exceeds the {MAX_GHOST_SAMPLES}-pose limit; reduce ranges or increase steps"
        ));
    }
    if !sample.time.is_finite() || !sample.frame_offset.is_finite() {
        return Err("Ghost sample is outside the finite timeline range".into());
    }
    samples.push(sample);
    Ok(())
}

fn reach(range: GhostRange, side: GhostSide) -> f64 {
    f64::from(if side == GhostSide::Before {
        range.before
    } else {
        range.after
    })
}

fn sign(side: GhostSide) -> f64 {
    if side == GhostSide::Before { -1. } else { 1. }
}

fn sampling_frame(time: f32, fps: f64, step: f64, anchor: bool) -> f64 {
    let frame = f64::from(time) * fps;
    if anchor {
        let lattice = (frame / step).round() * step;
        // A native time can exactly identify a lattice pose even though its
        // widened frame product is just above/below that pose (0.6 * 30, for
        // example). Only canonicalize when the lattice round-trips exactly;
        // nearby distinct native times must remain distinct, with no epsilon.
        if (lattice / fps) as f32 == time {
            return lattice;
        }
    }
    frame
}

fn frame_samples(
    result: &mut Vec<GhostSample>,
    time: f32,
    end: f32,
    fps: f64,
    settings: &GhostSampling,
    side: GhostSide,
) -> Result<(), String> {
    let step = f64::from(settings.frames.step);
    let frame = sampling_frame(time, fps, step, settings.anchor);
    let end_frame = sampling_frame(end, fps, step, settings.anchor);
    let requested = reach(settings.frames, side);
    if requested == 0. {
        return Ok(());
    }
    // Reach controls and emitted offsets are native f32 frame values. Allow
    // their rounding boundary when a snapped cursor's f64 product is just
    // above a whole frame, then check the candidate's native offset below.
    let native_reach = requested as f32;
    let next = native_reach.next_up();
    let spacing = if next.is_finite() {
        f64::from(next) - requested
    } else {
        requested - f64::from(native_reach.next_down())
    };
    let mut limit = requested + spacing * 0.5;
    let remainder = frame.rem_euclid(step);
    let mut first = if !settings.anchor || remainder == 0. {
        step
    } else if side == GhostSide::Before {
        remainder
    } else {
        step - remainder
    };
    // A snapped native cursor such as 0.3 seconds is not exactly 3.0 in
    // f64 frame arithmetic. Exclude the grid pose that rounds back to that
    // same native time instead of drawing a duplicate or rejecting Anchor.
    if settings.anchor && ((frame + sign(side) * first) / fps) as f32 == time {
        first += step;
    }
    let mut minimum = first;
    if !settings.looping {
        if side == GhostSide::Before {
            limit = limit.min(frame);
            minimum = minimum.max(frame - end_frame);
        } else {
            limit = limit.min(end_frame - frame);
        }
    }
    if limit < minimum {
        return Ok(());
    }
    let skipped = ((minimum - first) / step).ceil().max(0.);
    let nearest = first + skipped * step;
    if nearest > limit {
        return Ok(());
    }
    let mut count = ((limit - nearest) / step).floor() + 1.;
    if (nearest + (count - 1.) * step) as f32 > native_reach {
        count -= 1.;
    }
    if count > (MAX_GHOST_SAMPLES - result.len()) as f64 {
        return Err(format!(
            "Ghosting exceeds the {MAX_GHOST_SAMPLES}-pose limit; reduce ranges or increase steps"
        ));
    }
    for index in (0..count as usize).rev() {
        let distance = nearest + index as f64 * step;
        let offset = sign(side) * distance;
        let candidate_frame = frame + offset;
        let candidate = candidate_frame / fps;
        let sample_time = if settings.looping && !(0.0..=end_frame).contains(&candidate_frame) {
            // Reduce the cursor before adding a small delta: MAX + one frame
            // cannot represent that delta, even in f64, but its phase can.
            let (cursor, period) = if settings.anchor {
                (frame / fps, end_frame / fps)
            } else {
                (f64::from(time), f64::from(end))
            };
            (cursor.rem_euclid(period) + offset / fps).rem_euclid(period) as f32
        } else if candidate_frame == end_frame {
            end
        } else {
            candidate as f32
        };
        if sample_time < 0. || (!settings.looping && sample_time > end) {
            return Err("Ghost interval is outside supported timeline precision".into());
        }
        // A nonlooped fractional step below the native time resolution must not
        // quietly produce many copies of the current pose.
        if sample_time == time && !settings.looping {
            return Err("Ghost frame step is below timeline precision at this cursor".into());
        }
        push(
            result,
            GhostSample {
                time: sample_time,
                frame_offset: offset as f32,
                source: GhostSource::Frames,
                side,
            },
        )?;
    }
    Ok(())
}

fn key_samples(
    result: &mut Vec<GhostSample>,
    keys: &[f32],
    time: f32,
    end: f32,
    fps: f64,
    settings: &GhostSampling,
    side: GhostSide,
) -> Result<(), String> {
    let limit = reach(settings.key_frames, side) / fps;
    if limit == 0. || keys.is_empty() {
        return Ok(());
    }
    let stride = settings.key_frames.step as u128;
    let mut samples = Vec::new();
    if settings.looping {
        let has_zero = keys.first() == Some(&0.);
        let period = f64::from(end);
        let phase = f64::from(time).rem_euclid(period);
        let mut nearest: Vec<_> = keys
            .iter()
            .copied()
            .filter(|&key| !(has_zero && key == 0.))
            .map(|key| {
                let key_phase = if key == end { 0. } else { f64::from(key) };
                let distance = if side == GhostSide::Before {
                    (phase - key_phase).rem_euclid(period)
                } else {
                    (key_phase - phase).rem_euclid(period)
                };
                (if distance == 0. { period } else { distance }, key)
            })
            .collect();
        nearest.sort_by(|a, b| a.0.total_cmp(&b.0));
        let width = nearest.len() as u128;
        let base_cycle = (stride - 1) / width;
        let base_index = (stride - 1) % width;
        // Compute at most 65 selected occurrences, regardless of timeline reach
        // or stride. Integer remainders preserve huge exact f32 integer strides.
        for ordinal in 0..=MAX_GHOST_SAMPLES - result.len() {
            let index_sum = base_index + ordinal as u128 * (stride % width);
            let index = (index_sum % width) as usize;
            let cycles = base_cycle as f64
                + ordinal as f64 * (stride / width) as f64
                + (index_sum / width) as f64;
            let (base_distance, mut key_time) = nearest[index];
            let distance = base_distance + cycles * period;
            // Compare reaches at native time precision. A three-frame reach
            // from native 0.3 must include its native 0.6 key without merging
            // that key with any nearby, distinct authored time.
            if distance as f32 > limit as f32 {
                break;
            }
            if key_time == end
                && has_zero
                && side == GhostSide::Before
                && distance >= f64::from(time)
            {
                key_time = 0.;
            }
            samples.push(GhostSample {
                time: key_time,
                frame_offset: (sign(side) * distance * fps) as f32,
                source: GhostSource::KeyFrames,
                side,
            });
        }
    } else {
        let mut nearest: Vec<_> = keys
            .iter()
            .copied()
            .filter_map(|key| {
                let distance = sign(side) * (f64::from(key) - f64::from(time));
                (distance > 0. && distance as f32 <= limit as f32).then_some((distance, key))
            })
            .collect();
        nearest.sort_by(|a, b| {
            a.0.total_cmp(&b.0).then_with(|| {
                if side == GhostSide::Before {
                    b.1.total_cmp(&a.1)
                } else {
                    a.1.total_cmp(&b.1)
                }
            })
        });
        for (index, (distance, key)) in nearest.into_iter().enumerate() {
            if (index as u128 + 1).is_multiple_of(stride) {
                if samples.len() > MAX_GHOST_SAMPLES - result.len() {
                    break;
                }
                samples.push(GhostSample {
                    time: key,
                    frame_offset: (sign(side) * distance * fps) as f32,
                    source: GhostSource::KeyFrames,
                    side,
                });
            }
        }
    }
    for sample in samples.into_iter().rev() {
        push(result, sample)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use peachy_document::{
        Animation, Bone, Curve, FORMAT_VERSION, Keyframe, Property, Track, Transform,
    };

    fn document(times: &[f32]) -> Document {
        Document {
            format: FORMAT_VERSION,
            name: "Original ghost timeline".into(),
            fps: 10,
            bones: vec![Bone {
                name: "root".into(),
                parent: None,
                length: 10.,
                setup: Transform::default(),
                inherit: peachy_document::Inherit::Normal,
            }],
            constraints: vec![],
            slots: vec![],
            skins: Default::default(),
            animations: vec![Animation {
                name: "ghost-test".into(),
                tracks: vec![Track {
                    bone: 0,
                    property: Property::Rotation,
                    keys: times
                        .iter()
                        .map(|&time| Keyframe {
                            time,
                            value: time * 10.,
                            curve: Curve::Linear,
                        })
                        .collect(),
                }],
                physics: vec![],
                physics_resets: vec![],
                ik_tracks: vec![],
                inherit_tracks: vec![],
                deform_tracks: vec![],
                attachment_tracks: vec![],
                color_tracks: vec![],
                events: vec![],
                draw_order: vec![],
            }],
            skin_scope: Default::default(),
            events: vec![],
            reference_scale: 100.0,
        }
    }

    fn disabled() -> GhostSampling {
        GhostSampling {
            frames: GhostRange {
                before: 0.,
                after: 0.,
                step: 1.,
            },
            key_frames: GhostRange {
                before: 0.,
                after: 0.,
                step: 1.,
            },
            current: false,
            anchor: false,
            looping: false,
        }
    }

    fn close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.000001,
            "{actual} != {expected}"
        );
    }

    #[test]
    fn relative_fractional_frames_clip_and_preserve_endpoint_without_mutation() {
        let doc = document(&[0., 1.]);
        let original = doc.clone();
        let settings = GhostSampling {
            frames: GhostRange {
                before: 2.,
                after: 2.,
                step: 0.5,
            },
            ..disabled()
        };
        let result = ghost_samples(&doc, 0, 0.9, &settings).unwrap();
        assert_eq!(result.len(), 6);
        for (sample, expected) in result.iter().zip([0.7, 0.75, 0.8, 0.85, 1., 0.95]) {
            close(sample.time, expected);
        }
        assert_eq!(result[4].time, 1.);
        close(result[0].frame_offset, -2.);
        assert_eq!(doc, original);
    }

    #[test]
    fn anchor_uses_the_frame_zero_lattice_and_current_remains_the_live_cursor() {
        let doc = document(&[0., 2.]);
        let settings = GhostSampling {
            frames: GhostRange {
                before: 4.,
                after: 4.,
                step: 2.,
            },
            anchor: true,
            current: true,
            ..disabled()
        };
        let result = ghost_samples(&doc, 0, 0.55, &settings).unwrap();
        for (sample, expected) in result.iter().zip([0.2, 0.4, 0.8, 0.6, 0.55]) {
            close(sample.time, expected);
        }
        assert_eq!(result.len(), 5);
        assert_eq!(result[4].source, GhostSource::Current);
        close(result[0].frame_offset, -3.5);
        let beyond = ghost_samples(
            &doc,
            0,
            3.,
            &GhostSampling {
                current: true,
                ..disabled()
            },
        )
        .unwrap();
        assert_eq!(beyond[0].time, 3.);
        let settings = GhostSampling {
            frames: GhostRange {
                before: 2.,
                after: 2.,
                step: 1.,
            },
            anchor: true,
            ..disabled()
        };
        let snapped = ghost_samples(&doc, 0, 0.3, &settings).unwrap();
        assert_eq!(
            snapped.iter().map(|sample| sample.time).collect::<Vec<_>>(),
            [0.1, 0.2, 0.5, 0.4]
        );
    }

    #[test]
    fn anchored_native_lattice_cursor_keeps_both_reach_endpoints_at_thirty_fps() {
        let mut doc = document(&[0., 2.]);
        doc.fps = 30;
        for looping in [false, true] {
            let settings = GhostSampling {
                frames: GhostRange {
                    before: 6.,
                    after: 6.,
                    step: 1.5,
                },
                anchor: true,
                looping,
                ..disabled()
            };
            let result = ghost_samples(&doc, 0, 0.6, &settings).unwrap();
            assert_eq!(result.len(), 8);
            assert_eq!(
                result
                    .iter()
                    .map(|sample| sample.frame_offset)
                    .collect::<Vec<_>>(),
                [-6., -4.5, -3., -1.5, 6., 4.5, 3., 1.5]
            );
            assert_eq!(
                result.iter().map(|sample| sample.time).collect::<Vec<_>>(),
                [0.4, 0.45, 0.5, 0.55, 0.8, 0.75, 0.7, 0.65]
            );
            let settings = GhostSampling {
                frames: GhostRange {
                    step: 3.,
                    ..settings.frames
                },
                ..settings
            };
            let result = ghost_samples(&doc, 0, 0.3, &settings).unwrap();
            assert_eq!(
                result
                    .iter()
                    .map(|sample| sample.frame_offset)
                    .collect::<Vec<_>>(),
                [-6., -3., 6., 3.]
            );
            assert_eq!(
                result.iter().map(|sample| sample.time).collect::<Vec<_>>(),
                [0.1, 0.2, 0.5, 0.4]
            );
        }
        let settings = GhostSampling {
            frames: GhostRange {
                before: 6.,
                after: 6.,
                step: 1.5,
            },
            anchor: true,
            ..disabled()
        };
        let distinct = f32::from_bits(0.6f32.to_bits() + 1);
        let result = ghost_samples(&doc, 0, distinct, &settings).unwrap();
        let nearest_before = result
            .iter()
            .rfind(|sample| sample.side == GhostSide::Before)
            .unwrap();
        assert_eq!(nearest_before.time, 0.6);
        assert!(nearest_before.frame_offset < 0. && nearest_before.frame_offset > -0.0001);
        let mut endpoint_doc = document(&[0., 0.7]);
        endpoint_doc.fps = 30;
        for looping in [false, true] {
            let result = ghost_samples(
                &endpoint_doc,
                0,
                0.6,
                &GhostSampling {
                    looping,
                    ..settings
                },
            )
            .unwrap();
            assert!(
                result
                    .iter()
                    .any(|sample| sample.time == 0.7 && sample.frame_offset == 3.)
            );
        }
    }

    #[test]
    fn exact_unique_keys_and_whole_key_stride_preserve_close_native_identities() {
        let mut doc = document(&[0., 0.3, 0.300005, 0.6, 1.]);
        let mut duplicate = doc.animations[0].tracks[0].clone();
        duplicate.property = Property::X;
        doc.animations[0].tracks.push(duplicate);
        let settings = GhostSampling {
            key_frames: GhostRange {
                before: 10.,
                after: 10.,
                step: 1.,
            },
            ..disabled()
        };
        let result = ghost_samples(&doc, 0, 0.2, &settings).unwrap();
        assert_eq!(
            result.iter().map(|sample| sample.time).collect::<Vec<_>>(),
            [0., 1., 0.6, 0.300005, 0.3]
        );
        let settings = GhostSampling {
            key_frames: GhostRange {
                step: 2.,
                ..settings.key_frames
            },
            ..settings
        };
        let result = ghost_samples(&doc, 0, 0.2, &settings).unwrap();
        assert_eq!(
            result.iter().map(|sample| sample.time).collect::<Vec<_>>(),
            [1., 0.300005]
        );
        let doc = document(&[0., 0.3, 0.6, 1.]);
        let settings = GhostSampling {
            key_frames: GhostRange {
                before: 3.,
                after: 3.,
                step: 1.,
            },
            ..disabled()
        };
        let result = ghost_samples(&doc, 0, 0.3, &settings).unwrap();
        assert_eq!(
            result.iter().map(|sample| sample.time).collect::<Vec<_>>(),
            [0., 0.6]
        );
    }

    #[test]
    fn loops_wrap_frames_and_canonicalize_repeated_zero_end_key_boundaries() {
        let doc = document(&[0., 0.5, 1.]);
        let settings = GhostSampling {
            frames: GhostRange {
                before: 2.,
                after: 2.,
                step: 1.,
            },
            looping: true,
            ..disabled()
        };
        let result = ghost_samples(&doc, 0, 0., &settings).unwrap();
        for (sample, expected) in result.iter().zip([0.8, 0.9, 0.2, 0.1]) {
            close(sample.time, expected);
        }
        close(result[0].frame_offset, -2.);
        let settings = GhostSampling {
            key_frames: GhostRange {
                before: 20.,
                after: 20.,
                step: 1.,
            },
            looping: true,
            ..disabled()
        };
        let result = ghost_samples(&doc, 0, 0.5, &settings).unwrap();
        let before: Vec<_> = result
            .iter()
            .filter(|sample| sample.side == GhostSide::Before)
            .map(|sample| (sample.time, sample.frame_offset))
            .collect();
        assert_eq!(before, [(0.5, -20.), (0., -15.), (0.5, -10.), (0., -5.)]);
        let after: Vec<_> = result
            .iter()
            .filter(|sample| sample.side == GhostSide::After)
            .map(|sample| (sample.time, sample.frame_offset))
            .collect();
        assert_eq!(after, [(0.5, 20.), (1., 15.), (0.5, 10.), (1., 5.)]);
    }

    #[test]
    fn empty_and_frame_zero_clips_only_have_optional_zero_current() {
        for doc in [document(&[]), document(&[0.])] {
            let settings = GhostSampling {
                current: true,
                looping: true,
                ..Default::default()
            };
            let result = ghost_samples(&doc, 0, 12., &settings).unwrap();
            assert_eq!(
                result,
                [GhostSample {
                    time: 0.,
                    frame_offset: 0.,
                    source: GhostSource::Current,
                    side: GhostSide::Current
                }]
            );
        }
    }

    #[test]
    fn malformed_controls_and_over_budget_sampling_fail_without_truncating() {
        let doc = document(&[0., 1.]);
        for time in [-1., f32::NAN, f32::INFINITY] {
            assert!(ghost_samples(&doc, 0, time, &disabled()).is_err());
        }
        assert!(ghost_samples(&doc, 1, 0., &disabled()).is_err());
        for fps in [0, 241] {
            let mut invalid = doc.clone();
            invalid.fps = fps;
            assert!(ghost_samples(&invalid, 0, 0., &disabled()).is_err());
        }
        for step in [0., -1., f32::NAN, f32::INFINITY] {
            let settings = GhostSampling {
                frames: GhostRange {
                    step,
                    ..disabled().frames
                },
                ..disabled()
            };
            assert!(ghost_samples(&doc, 0, 0., &settings).is_err());
        }
        for step in [0.5, 1.5] {
            let settings = GhostSampling {
                key_frames: GhostRange {
                    step,
                    ..disabled().key_frames
                },
                ..disabled()
            };
            assert!(ghost_samples(&doc, 0, 0., &settings).is_err());
        }
        let settings = GhostSampling {
            frames: GhostRange {
                before: 0.,
                after: f32::MAX,
                step: f32::MIN_POSITIVE,
            },
            looping: true,
            ..disabled()
        };
        assert!(
            ghost_samples(&doc, 0, 0., &settings)
                .unwrap_err()
                .contains("64-pose")
        );
        let settings = GhostSampling {
            key_frames: GhostRange {
                before: 0.,
                after: f32::MAX,
                step: 1.,
            },
            looping: true,
            ..disabled()
        };
        assert!(
            ghost_samples(&doc, 0, 0., &settings)
                .unwrap_err()
                .contains("64-pose")
        );
    }

    #[test]
    fn wide_finite_cursors_and_integer_strides_stay_bounded() {
        let doc = document(&[0., 0.5, 1.]);
        assert!(
            ghost_samples(&doc, 0, f32::MAX, &GhostSampling::default())
                .unwrap()
                .is_empty()
        );
        let settings = GhostSampling {
            looping: true,
            frames: GhostRange {
                before: 0.,
                after: 1.,
                step: 1.,
            },
            ..disabled()
        };
        let result = ghost_samples(&doc, 0, f32::MAX, &settings).unwrap();
        assert_eq!(result.len(), 1);
        close(result[0].time, 0.1);
        assert_eq!(result[0].frame_offset, 1.);
        let settings = GhostSampling {
            looping: true,
            key_frames: GhostRange {
                before: 0.,
                after: f32::MAX,
                step: f32::MAX,
            },
            ..disabled()
        };
        assert!(ghost_samples(&doc, 0, 0., &settings).unwrap().is_empty());
    }
}
