//! Functional animation core and the in-memory session that records its results.
mod authoring;
mod clipping;
mod color_curve_split;
mod commands;
mod constraint_effects;
mod curve_editing;
mod deform_authoring;
mod draw_order_authoring;
mod ghosting;
mod ik;
mod ik_sampling;
mod ik_solver;
mod mesh;
mod mesh_binding;
mod mesh_soft_selection;
mod mesh_weights;
mod names;
mod path_constraint;
mod picking;
mod playback;
mod pose;
mod scene;
mod session;
mod slider_authoring;
mod slider_constraint;
mod slider_creation;
mod slider_domains;
mod slider_evaluation;
mod slider_mapping;
mod slider_sampling;
mod slider_setup;
mod slider_source;
mod slot_authoring;
mod timeline;
mod tools;
mod transform_constraint;
mod transform_mapping;

pub use authoring::{IkPoseEdit, KeyState, PoseEdits, transform_group};
pub use clipping::ClippedGeometry;
pub use commands::{
    Command, CommandOutcome, EditTarget, apply_command, apply_command_with_effects,
    deform_keyframe_at, ik_keyframe_at, keyframe_at,
};
pub use constraint_effects::{ConstraintIndexRemap, EditEffects};
pub use curve_editing::{CurvePreset, CurveSelection, curve_edit_targets};
pub use draw_order_authoring::{
    DrawOrderPlacement, draw_order_key_time, move_draw_order_slot, place_draw_order_slot,
};
pub use ghosting::{
    GhostRange, GhostSample, GhostSampling, GhostSide, GhostSource, MAX_GHOST_SAMPLES,
    ghost_samples,
};
pub use ik::{
    AuthoredPose, EvaluatedPose, checked_evaluate_pose, checked_pose_from_authored,
    checked_setup_pose_from_locals, sample_authored_pose, sample_authored_pose_with_skin,
};
pub use ik_sampling::{IkPose, sample_ik};
pub use mesh::{
    deformed_mesh_world_vertices, deformed_world_vertices, mesh_world_vertices, region_to_mesh,
    region_vertices, world_vertices,
};
pub use mesh_binding::MeshBindPose;
pub use mesh_soft_selection::{MeshSoftSelection, MeshTranslation};
pub use mesh_weights::{
    MeshWeightEdit, MeshWeightGesture, MeshWeightQuery, MeshWeightSelection, MeshWeightValue,
    VertexWeight, mesh_bone_weights,
};
pub use path_constraint::{PathPose, path_cubics};
pub use picking::{attachment_bounds, geometry_bounds, pick_attachment};
pub use playback::{
    PlayDirection, PlaybackStep, adjacent_key_time, advance_playback, last_key_time,
};
pub use pose::{
    Affine, INHERIT_MIN_PARENT_AXIS_SQUARED, curve_progress, curve_value, evaluate, inherit_pose,
    local_pose, sample, world_pose, world_pose_with,
};
pub use scene::{
    AttachmentDraw, AttachmentOverlay, Bounds, OverlayKind, Scene, checked_evaluate_scene,
    checked_scene_from_authored, checked_scene_from_evaluated, checked_setup_scene_from_locals,
    evaluate_scene,
};
pub use session::Session;
pub use slider_authoring::{SliderPose, SliderPoseEdit, slider_key_time};
pub use slider_evaluation::{SliderEligibility, SliderEvaluation};
pub use slider_mapping::{
    SliderMappingRange, SliderMappingRangeField, edited_slider_mapping_range, slider_mapping_range,
};
pub use slider_source::{SliderSourcePreferences, SliderSourceRange};
pub use slot_authoring::{SlotPose, SlotPoseEdit, SlotProperty, slot_color_bytes, slot_key_time};
pub use timeline::{
    AnimationKey, AnimationKeyClipboard, ClipboardKey, CurveChannel, CurveData, CurveTarget,
    DeformLayout, KeyClipboard, KeyOwner, SliderClipboardKey, TimelineKey, copy_animation_keys,
    copy_keys, curve_channels, curve_channels_for_targets, curve_targets, deform_key_owner,
    pasted_animation_keys,
};
pub use tools::{
    Axes, TransformGesture, TransformTool, drag_transform, drag_transform_in_frame,
    parse_transform_input, set_transform_value, transform_axes, transform_axes_from_world,
    transform_values,
};
pub use transform_mapping::{
    TransformMappingRange, TransformMappingRangeField, edited_transform_mapping_range,
    transform_mapping_range,
};

#[cfg(test)]
mod animation_tests;
#[cfg(test)]
mod applied_state_tests;
#[cfg(test)]
mod bind_pose_inherit_tests;
#[cfg(test)]
mod binding_tests;
#[cfg(test)]
mod clipping_tests;
#[cfg(test)]
mod creation_tests;
#[cfg(test)]
mod curve_action_tests;
#[cfg(test)]
mod curve_target_tests;
#[cfg(test)]
mod deform_authoring_tests;
#[cfg(test)]
mod deform_tests;
#[cfg(test)]
mod deform_timeline_tests;
#[cfg(test)]
mod direct_weight_tests;
#[cfg(test)]
mod draw_order_authoring_tests;
#[cfg(test)]
mod draw_order_tests;
#[cfg(test)]
mod duration_tests;
#[cfg(test)]
mod ik_animation_tests;
#[cfg(test)]
mod ik_scale_tests;
#[cfg(test)]
mod ik_tests;
#[cfg(test)]
mod inherit_tests;
#[cfg(test)]
mod linked_mesh_tests;
#[cfg(test)]
mod mesh_soft_selection_tests;
#[cfg(test)]
mod mesh_weights_tests;
#[cfg(test)]
mod overlay_tests;
#[cfg(test)]
mod path_tests;
#[cfg(test)]
mod physics_tests;
#[cfg(test)]
mod pose_preview_tests;
#[cfg(test)]
mod scalar_curve_tests;
#[cfg(test)]
mod scene_checks_tests;
#[cfg(test)]
mod shear_tests;
#[cfg(test)]
mod skin_sampling_tests;
#[cfg(test)]
mod skin_scope_tests;
#[cfg(test)]
mod slider_creation_tests;
#[cfg(test)]
mod slider_setup_tests;
#[cfg(test)]
mod slider_source_authoring_tests;
#[cfg(test)]
mod slider_tests;
#[cfg(test)]
mod slot_authoring_tests;
#[cfg(test)]
mod slot_color_insert_observations;
#[cfg(test)]
mod slot_color_insert_tests;
#[cfg(test)]
mod slot_color_tests;
#[cfg(test)]
mod slot_keys_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod timeline_tests;
#[cfg(test)]
mod tint_tests;
#[cfg(test)]
mod transform_tests;
#[cfg(test)]
mod weighted_tests;

#[cfg(test)]
mod wide_interpolation_tests;

#[cfg(test)]
#[path = "../../document/test_support/native_fixture.rs"]
mod native_fixture;

#[cfg(test)]
mod slider_insert_tests;
