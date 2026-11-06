//! Functional animation core and the in-memory session that records its results.
mod authoring;
mod commands;
mod ik;
mod ik_sampling;
mod ik_solver;
mod mesh;
mod mesh_binding;
mod names;
mod picking;
mod playback;
mod pose;
mod scene;
mod session;
mod timeline;
mod tools;

pub use authoring::{KeyState, PoseEdits, transform_group};
pub use commands::{Command, EditTarget, apply_command, ik_keyframe_at, keyframe_at};
pub use ik::{
    AuthoredPose, EvaluatedPose, checked_evaluate_pose, checked_pose_from_authored,
    checked_setup_pose_from_locals, sample_authored_pose,
};
pub use ik_sampling::{IkPose, sample_ik};
pub use mesh::{mesh_world_vertices, region_to_mesh, region_vertices};
pub use mesh_binding::MeshBindPose;
pub use picking::{attachment_bounds, pick_attachment};
pub use playback::{
    PlayDirection, PlaybackStep, adjacent_key_time, advance_playback, last_key_time,
};
pub use pose::{Affine, curve_progress, evaluate, local_pose, sample, world_pose};
pub use scene::{
    AttachmentDraw, Bounds, Scene, checked_evaluate_scene, checked_scene_from_authored,
    checked_setup_scene_from_locals, evaluate_scene,
};
pub use session::Session;
pub use timeline::{
    AnimationKey, AnimationKeyClipboard, ClipboardKey, KeyClipboard, KeyOwner, TimelineKey,
    copy_animation_keys, copy_keys,
};
pub use tools::{
    Axes, TransformGesture, TransformTool, drag_transform, drag_transform_in_frame,
    parse_transform_input, set_transform_value, transform_axes, transform_axes_from_world,
    transform_values,
};

#[cfg(test)]
mod animation_tests;
#[cfg(test)]
mod binding_tests;
#[cfg(test)]
mod direct_weight_tests;
#[cfg(test)]
mod duration_tests;
#[cfg(test)]
mod ik_animation_tests;
#[cfg(test)]
mod ik_tests;
#[cfg(test)]
mod pose_preview_tests;
#[cfg(test)]
mod scene_checks_tests;
#[cfg(test)]
mod shear_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod timeline_tests;
#[cfg(test)]
mod weighted_tests;

#[cfg(test)]
mod wide_interpolation_tests;
