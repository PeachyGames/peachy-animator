//! Own-turn observations from the single ordered constraint pass.
use crate::SliderPose;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SliderEligibility {
    Eligible,
    InactiveConstraint,
    InactiveSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SliderEvaluation {
    /// Effective settings immediately before this Slider's ordered turn.
    pub incoming: SliderPose,
    /// This actor's settings after deriving Time, before its referenced
    /// animation or later constraints can overwrite the same Slider again.
    pub applied: SliderPose,
    /// Actual reference query time after Loop's final modulo. None when no
    /// application occurs, including exact zero incoming Mix.
    pub sample_time: Option<f64>,
    pub eligibility: SliderEligibility,
}

impl SliderEvaluation {
    pub(crate) fn skipped(incoming: SliderPose, eligibility: SliderEligibility) -> Self {
        Self {
            incoming,
            applied: incoming,
            sample_time: None,
            eligibility,
        }
    }
}
