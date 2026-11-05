//! Functional animation core and the in-memory session that records its results.
mod commands;
mod mesh;
mod picking;
mod pose;
mod scene;
mod session;

pub use commands::{Command, EditTarget, apply_command};
pub use mesh::{region_to_mesh, region_vertices};
pub use picking::{attachment_bounds, pick_attachment};
pub use pose::{Affine, curve_progress, evaluate, local_pose, sample, world_pose};
pub use scene::{AttachmentDraw, Bounds, Scene, evaluate_scene};
pub use session::Session;

#[cfg(test)]
mod tests;
