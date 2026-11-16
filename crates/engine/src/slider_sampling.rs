//! Reference-time sampling retains effective Slider times between stored f32
//! keys. The public f32 bone sampler keeps its existing arithmetic unchanged.
use peachy_document::{Curve, Keyframe};

pub(crate) fn value_between(curve: Curve, progress: f64, left: f32, right: f32) -> f64 {
    match curve {
        Curve::Linear => f64::from(left) + (f64::from(right) - f64::from(left)) * progress,
        Curve::Stepped => f64::from(left),
        // Keep the shared calibrated curve evaluator rather than introducing
        // another Bezier solver for referenced animation application.
        other => f64::from(crate::curve_value(other, progress as f32, left, right)),
    }
}

pub(crate) fn scalar(keys: &[Keyframe], time: f64) -> Option<f64> {
    let index = keys.iter().rposition(|key| f64::from(key.time) <= time)?;
    let left = keys[index];
    match keys.get(index + 1) {
        Some(right) => Some(value_between(
            left.curve,
            (time - f64::from(left.time)) / (f64::from(right.time) - f64::from(left.time)),
            left.value,
            right.value,
        )),
        None => Some(f64::from(left.value)),
    }
}
