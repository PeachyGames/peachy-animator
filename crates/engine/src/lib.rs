//! Functional animation core and the in-memory session that records its results.
mod commands;
mod pose;
mod scene;
mod session;

pub use commands::{Command, EditTarget, apply_command};
pub use pose::{Affine, curve_progress, evaluate, local_pose, sample, world_pose};
pub use scene::{Bounds, RegionDraw, Scene, evaluate_scene};
pub use session::Session;

#[cfg(test)]
mod tests;
