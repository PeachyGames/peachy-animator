//! Original public-pose observations select this independent cubic polyline model.
//! See IK_PROVENANCE.md and docs/slices/ik-animation-design.md; no runtime source.
use peachy_document::{IkCurve, IkSetup, IkTrack};
use serde::Serialize;

/// A sampled value stays wide until the checked solver converts its output frame.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct IkPose {
    pub mix: f64,
    pub bend_positive: bool,
}

impl From<IkSetup> for IkPose {
    fn from(value: IkSetup) -> Self {
        Self {
            mix: f64::from(value.mix),
            bend_positive: value.bend_positive,
        }
    }
}

/// Complete grouped settings: setup before the first key, absolute values after.
pub fn sample_ik(track: &IkTrack, setup: IkSetup, time: f32) -> Result<IkPose, String> {
    if !time.is_finite() || time < 0. || !setup.mix.is_finite() {
        return Err("IK sampling requires finite settings and nonnegative finite time".into());
    }
    let mut previous = -1.;
    for key in &track.keys {
        key.validate()?;
        if key.time <= previous {
            return Err("IK sampling requires strictly ordered keys".into());
        }
        previous = key.time;
    }
    let Some(index) = track.keys.iter().rposition(|key| key.time <= time) else {
        return Ok(setup.into());
    };
    let left = track.keys[index];
    let mut value = IkPose {
        mix: f64::from(left.mix),
        bend_positive: left.bend_positive,
    };
    let Some(right) = track.keys.get(index + 1) else {
        return Ok(value);
    };
    if time == left.time || left.curve == IkCurve::Stepped {
        return Ok(value);
    }
    let start = f64::from(left.time);
    let end = f64::from(right.time);
    let query = f64::from(time);
    value.mix = match left.curve {
        IkCurve::Linear => {
            f64::from(left.mix)
                + (f64::from(right.mix) - f64::from(left.mix)) * ((query - start) / (end - start))
        }
        IkCurve::Stepped => f64::from(left.mix),
        IkCurve::Bezier { time, mix } => {
            let xs = points([
                start,
                start + (end - start) * time[0],
                start + (end - start) * time[1],
                end,
            ])?;
            let ys = points([
                f64::from(left.mix),
                f64::from(mix[0]),
                f64::from(mix[1]),
                f64::from(right.mix),
            ])?;
            let segment = (1..=10)
                .find(|&i| query <= xs[i])
                .ok_or("IK curve cannot resolve this absolute query time")?;
            let width = xs[segment] - xs[segment - 1];
            if width <= 0. {
                return Err("IK curve time interval exceeds coordinate precision".into());
            }
            ys[segment - 1] + (ys[segment] - ys[segment - 1]) * ((query - xs[segment - 1]) / width)
        }
    };
    if !value.mix.is_finite() {
        return Err("Sampled IK Mix is nonfinite".into());
    }
    Ok(value)
}

/// Standard finite differences with an independently measured integration
/// coefficient. f64 derivatives and f32 interior point storage are observable.
fn points([p0, p1, p2, p3]: [f64; 4]) -> Result<[f64; 11], String> {
    let velocity = (p1 - p0) * 0.3;
    let acceleration = (p0 - 2. * p1 + p2) * 0.06;
    let jerk = (3. * (p1 - p2) + p3 - p0) * 0.006;
    let mut delta = (velocity + acceleration * 0.5) + jerk * 0.16666667;
    let mut delta2 = acceleration + jerk;
    let mut position = p0;
    let mut points = [0.; 11];
    points[0] = p0;
    points[10] = p3;
    for point in &mut points[1..10] {
        position += delta;
        delta += delta2;
        delta2 += jerk;
        *point = f64::from(position as f32);
    }
    if !points.into_iter().all(f64::is_finite) {
        return Err("IK curve points exceed finite coordinate precision".into());
    }
    Ok(points)
}
