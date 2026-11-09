//! Pure viewport coordinates for the compact Timeline, independent of editing/history.
use peachy_document::Document;

const MAX_TIME: f64 = f32::MAX as f64;
const MAX_TICKS: usize = 127;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TimelineRange {
    pub start: f64,
    pub span: f64,
}

impl Default for TimelineRange {
    fn default() -> Self {
        Self {
            start: 0.,
            span: 1.,
        }
    }
}

impl TimelineRange {
    fn valid(self) -> bool {
        self.start.is_finite()
            && self.start >= 0.
            && self.span.is_finite()
            && self.span > 0.
            && self.end() > self.start
            && self.end() <= MAX_TIME
    }

    fn end(self) -> f64 {
        self.start + self.span
    }

    /// Show every key, with a small authoring margin. Empty clips retain a second.
    pub fn fit(end: f32, fps: u32) -> Self {
        if !valid_fps(fps) || !end.is_finite() || end <= 0. {
            return Self::default();
        }
        Self {
            start: 0.,
            span: (f64::from(end).max(1. / f64::from(fps)) * 1.1).min(MAX_TIME),
        }
    }

    /// A rightward pointer drag reveals earlier time, as with viewport panning.
    pub fn pan(self, delta_pixels: f32, width: f32) -> Self {
        if !self.valid() || !delta_pixels.is_finite() || !valid_width(width) {
            return self;
        }
        let candidate = Self {
            start: (self.start - f64::from(delta_pixels) / f64::from(width) * self.span)
                .clamp(0., MAX_TIME - self.span),
            ..self
        };
        // A tiny span cannot be represented at an arbitrarily large offset.
        // Refuse that pan instead of producing an empty or nonfinite view.
        if candidate.valid() { candidate } else { self }
    }

    /// Factor > 1 zooms out. Preserve the time under a normalized pointer anchor
    /// until the beginning/end of the representable timeline constrains the view.
    pub fn zoom(self, factor: f64, anchor: f64, fps: u32) -> Self {
        if !self.valid()
            || !factor.is_finite()
            || factor <= 0.
            || !anchor.is_finite()
            || !(0.0..=1.0).contains(&anchor)
            || !valid_fps(fps)
        {
            return self;
        }
        let span = (self.span * factor).clamp(1. / f64::from(fps), MAX_TIME);
        let anchored_time = self.start + self.span * anchor;
        let candidate = Self {
            start: (anchored_time - span * anchor).clamp(0., MAX_TIME - span),
            span,
        };
        if candidate.valid() { candidate } else { self }
    }
}

#[derive(Clone, Copy)]
pub(super) struct TimelineMap {
    pub range: TimelineRange,
    pub left: f32,
    pub width: f32,
}

impl TimelineMap {
    fn valid(self) -> bool {
        let right = f64::from(self.left) + f64::from(self.width);
        self.range.valid()
            && self.left.is_finite()
            && valid_width(self.width)
            && right <= MAX_TIME
            && (right as f32).is_finite()
            && (right as f32) > self.left
    }

    /// Clamp pointer travel to this frozen ruler, then optionally snap to frames.
    /// Wide arithmetic also permits authoring cursors near the native time limit.
    pub fn time_at(self, x: f32, fps: u32, snap: bool) -> Option<f32> {
        if !self.valid() || !x.is_finite() || !valid_fps(fps) {
            return None;
        }
        let fraction =
            ((f64::from(x) - f64::from(self.left)) / f64::from(self.width)).clamp(0., 1.);
        let mut time = self.range.start + self.range.span * fraction;
        if snap {
            time = (time * f64::from(fps)).round() / f64::from(fps);
        }
        let time = time.min(MAX_TIME) as f32;
        (time.is_finite() && time >= 0.).then_some(time)
    }

    /// Native endpoint identity keeps a clicked edge visible after f64->f32
    /// conversion. Distinct next-native times outside that envelope stay hidden.
    pub fn x_at(self, time: f32) -> Option<f32> {
        if !self.valid()
            || !time.is_finite()
            || time < 0.
            || time < self.range.start as f32
            || time > self.range.end() as f32
        {
            return None;
        }
        let fraction = ((f64::from(time) - self.range.start) / self.range.span).clamp(0., 1.);
        let x = (f64::from(self.left) + fraction * f64::from(self.width)) as f32;
        x.is_finite().then_some(x)
    }
}

fn valid_fps(fps: u32) -> bool {
    (1..=240).contains(&fps)
}

fn valid_width(width: f32) -> bool {
    width.is_finite() && width > 0.
}

/// Summary diamonds describe every bone, physics and reset key, irrespective of
/// selection, matching W/S key navigation. The caller supplies a validated document.
pub(super) fn key_times(doc: &Document, animation: usize) -> Vec<f32> {
    let Some(animation) = doc.animations.get(animation) else {
        return Vec::new();
    };
    let mut times: Vec<_> = animation.key_times().collect();
    times.sort_by(f32::total_cmp);
    times.dedup();
    times
}

/// Major ticks use integer project frames, with 1/2/5 decade spacing. Generate
/// by bounded index rather than repeatedly adding to a large timeline offset.
pub(super) fn ticks(range: TimelineRange, width: f32, fps: u32) -> Vec<(f32, String)> {
    if !range.valid() || !valid_width(width) || !valid_fps(fps) {
        return Vec::new();
    }
    let fps = f64::from(fps);
    let start_frame = range.start * fps;
    let end_frame = range.end() * fps;
    let pixels = if end_frame >= 100_000. { 104. } else { 60. };
    let slots = (f64::from(width) / pixels).clamp(1., (MAX_TICKS - 1) as f64);
    let desired = (range.span * fps / slots).max(1.);
    let decade = 10f64.powf(desired.log10().floor());
    let unit = [1., 2., 5., 10.]
        .into_iter()
        .map(|multiple| decade * multiple)
        .find(|&unit| unit >= desired)
        .unwrap_or(decade * 10.);
    let first = (start_frame / unit).ceil() * unit;
    let mut result = Vec::new();
    let mut previous_time = None;
    let mut previous_x = f64::NEG_INFINITY;
    for index in 0..MAX_TICKS {
        let frame = first + index as f64 * unit;
        let seconds = frame / fps;
        if seconds > range.end() {
            break;
        }
        if seconds < range.start {
            continue;
        }
        let time = seconds as f32;
        if !time.is_finite() || previous_time == Some(time) {
            continue;
        }
        let x = (f64::from(time) - range.start) / range.span * f64::from(width);
        if x - previous_x < pixels * 0.95 {
            continue;
        }
        let label = if frame < 100_000. {
            format!("{frame:.0}")
        } else {
            format!("{frame:.6e}")
        };
        result.push((time, label));
        previous_time = Some(time);
        previous_x = x;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use peachy_document::{Curve, Keyframe, Property, Track};

    fn map(range: TimelineRange) -> TimelineMap {
        TimelineMap {
            range,
            left: 100.,
            width: 600.,
        }
    }

    #[test]
    fn summary_merges_only_exact_key_times_across_all_bones_and_tracks() {
        let mut doc: Document = serde_json::from_str(include_str!(
            "../../../../examples/peach-bot/peach-bot.peachy"
        ))
        .unwrap();
        let track = |bone, property, times: &[f32]| Track {
            bone,
            property,
            keys: times
                .iter()
                .map(|&time| Keyframe {
                    time,
                    value: 0.,
                    curve: Curve::Linear,
                })
                .collect(),
        };
        doc.animations[0].tracks = vec![
            track(0, Property::X, &[0., 0.3, 0.6]),
            track(8, Property::Rotation, &[0.3, 0.300005, 0.8]),
        ];
        let original = doc.clone();
        assert_eq!(key_times(&doc, 0), vec![0., 0.3, 0.300005, 0.6, 0.8]);
        assert!(key_times(&doc, usize::MAX).is_empty());
        doc.animations[0].tracks.clear();
        assert!(key_times(&doc, 0).is_empty());
        // Physics and reset keys appear like bone keys, matching W/S navigation.
        doc.constraints.push(peachy_document::Constraint::Physics(
            peachy_document::PhysicsConstraint::new("sway", 8),
        ));
        doc.animations[0]
            .physics
            .push(peachy_document::PhysicsTrack {
                constraint: Some(doc.constraints.len() - 1),
                property: peachy_document::PhysicsProperty::Wind,
                keys: vec![Keyframe {
                    time: 0.45,
                    value: 2.,
                    curve: Curve::Stepped,
                }],
            });
        doc.animations[0]
            .physics_resets
            .push(peachy_document::PhysicsReset {
                constraint: None,
                times: vec![0.2, 0.45],
            });
        doc.validate().unwrap();
        assert_eq!(key_times(&doc, 0), vec![0.2, 0.45]);
        doc.constraints.pop();
        doc.animations[0] = original.animations[0].clone();
        assert_eq!(doc, original);
    }

    #[test]
    fn fit_and_pan_keep_empty_and_short_clips_authorable_without_crossing_zero() {
        assert_eq!(TimelineRange::fit(0., 30), TimelineRange::default());
        let short = TimelineRange::fit(0.5, 30);
        assert_eq!(short.start, 0.);
        assert!((short.span - 0.55).abs() < 1e-12);
        assert!(TimelineRange::fit(0.001, 30).span >= 1. / 30.);
        let range = TimelineRange {
            start: 2.,
            span: 4.,
        };
        assert_eq!(
            range.pan(300., 600.),
            TimelineRange {
                start: 0.,
                span: 4.
            }
        );
        assert_eq!(
            range.pan(-150., 600.),
            TimelineRange {
                start: 3.,
                span: 4.
            }
        );
        assert_eq!(range.pan(f32::MAX, 600.).start, 0.);
    }

    #[test]
    fn zoom_keeps_pointer_time_fixed_and_clamps_to_one_frame_or_timeline_bounds() {
        let range = TimelineRange {
            start: 2.,
            span: 4.,
        };
        let closer = range.zoom(0.5, 0.25, 30);
        assert_eq!(
            closer,
            TimelineRange {
                start: 2.5,
                span: 2.
            }
        );
        assert_eq!(
            range.start + range.span * 0.25,
            closer.start + closer.span * 0.25
        );
        let wider = range.zoom(2., 0.25, 30);
        assert_eq!(
            wider,
            TimelineRange {
                start: 1.,
                span: 8.
            }
        );
        let minimum = TimelineRange::default().zoom(1e-30, 0., 30);
        assert_eq!(minimum.start, 0.);
        assert_eq!(minimum.span, 1. / 30.);
        assert_eq!(
            range.zoom(f64::MAX, 1., 30),
            TimelineRange {
                start: 0.,
                span: MAX_TIME
            }
        );
    }

    #[test]
    fn pointer_mapping_supports_unsnapped_fractional_frames_and_native_edge_identity() {
        let ruler = map(TimelineRange::default());
        assert_eq!(ruler.time_at(115., 30, true), Some(1. / 30.));
        assert_eq!(ruler.time_at(115., 30, false), Some(0.025));
        assert_eq!(ruler.time_at(-500., 30, false), Some(0.));
        assert_eq!(ruler.time_at(1200., 30, false), Some(1.));
        assert_eq!(ruler.x_at(-1.), None);
        assert_eq!(ruler.x_at(1.1), None);
        let edge = map(TimelineRange {
            start: 0.,
            span: 0.7,
        });
        let time = edge.time_at(700., 30, false).unwrap();
        assert_eq!(time, 0.7);
        assert_eq!(edge.x_at(time), Some(700.));
        assert_eq!(edge.x_at(time.next_up()), None);
        let shifted = map(TimelineRange {
            start: 0.7,
            span: 0.2,
        });
        assert_eq!(shifted.x_at(0.7), Some(100.));
        assert_eq!(shifted.x_at(0.7f32.next_down()), None);
    }

    #[test]
    fn invalid_input_never_changes_a_valid_range_or_emits_coordinates() {
        let range = TimelineRange {
            start: 2.,
            span: 4.,
        };
        for delta in [f32::NAN, f32::INFINITY] {
            assert_eq!(range.pan(delta, 600.), range);
        }
        for width in [0., -1., f32::NAN, f32::INFINITY] {
            assert_eq!(range.pan(10., width), range);
            assert!(
                TimelineMap {
                    width,
                    ..map(range)
                }
                .time_at(100., 30, false)
                .is_none()
            );
            assert!(ticks(range, width, 30).is_empty());
        }
        for factor in [0., -1., f64::NAN, f64::INFINITY] {
            assert_eq!(range.zoom(factor, 0.5, 30), range);
        }
        for anchor in [-1., 2., f64::NAN, f64::INFINITY] {
            assert_eq!(range.zoom(0.5, anchor, 30), range);
        }
        for fps in [0, 241, u32::MAX] {
            assert_eq!(range.zoom(0.5, 0.5, fps), range);
            assert!(map(range).time_at(100., fps, true).is_none());
            assert!(ticks(range, 600., fps).is_empty());
        }
        assert!(map(range).time_at(f32::NAN, 30, false).is_none());
        assert!(map(range).x_at(f32::NAN).is_none());
        assert!(
            map(TimelineRange {
                start: MAX_TIME,
                span: 1.
            })
            .x_at(f32::MAX)
            .is_none()
        );
    }

    #[test]
    fn ticks_use_project_frame_labels_without_dense_or_duplicate_markers() {
        let range = TimelineRange::default();
        let ruler = map(range);
        let marks = ticks(range, 600., 30);
        assert_eq!(
            marks
                .iter()
                .map(|(_, label)| label.as_str())
                .collect::<Vec<_>>(),
            ["0", "5", "10", "15", "20", "25", "30"]
        );
        for pair in marks.windows(2) {
            assert!(ruler.x_at(pair[1].0).unwrap() - ruler.x_at(pair[0].0).unwrap() >= 59.9);
        }
        assert!(
            ticks(
                TimelineRange {
                    start: 0.,
                    span: 1e-8
                },
                20.,
                30
            )
            .len()
                <= 1
        );
    }

    #[test]
    fn extreme_native_times_keep_mapping_and_ticks_finite_and_bounded() {
        let range = TimelineRange::fit(f32::MAX, 240);
        assert_eq!(
            range,
            TimelineRange {
                start: 0.,
                span: MAX_TIME
            }
        );
        let ruler = map(range);
        assert_eq!(ruler.time_at(700., 240, true), Some(f32::MAX));
        assert_eq!(ruler.x_at(f32::MAX), Some(700.));
        assert!(ruler.time_at(400., 240, false).unwrap().is_finite());
        for width in [f32::MIN_POSITIVE, 1., 600., f32::MAX] {
            let marks = ticks(range, width, 240);
            assert!(marks.len() < 128);
            assert!(
                marks
                    .iter()
                    .all(|(time, label)| time.is_finite() && label.len() < 24)
            );
        }
        let near_end = TimelineRange {
            start: MAX_TIME * 0.5,
            span: MAX_TIME * 0.5,
        }
        .zoom(0.5, 1., 240);
        assert!(near_end.valid());
        assert_eq!(map(near_end).x_at(f32::MAX), Some(700.));
        let unrepresentable = TimelineRange {
            start: MAX_TIME * 0.5,
            span: MAX_TIME * 0.25,
        };
        assert_eq!(unrepresentable.zoom(1e-50, 1., 240), unrepresentable);
        assert!(
            map(TimelineRange {
                start: f64::INFINITY,
                span: 1.
            })
            .x_at(1.)
            .is_none()
        );
        assert!(
            ticks(
                TimelineRange {
                    start: MAX_TIME,
                    span: 1.
                },
                600.,
                30
            )
            .is_empty()
        );
    }
}
