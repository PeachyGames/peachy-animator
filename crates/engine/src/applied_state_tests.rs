//! One ordered effective state supplies geometry, public readback and binding.
use crate::*;
use peachy_document::*;

#[test]
fn applied_state_same_evaluation_supplies_scene_metadata_and_bind_inheritance() {
    let doc: Document = crate::native_fixture::decode(include_str!(
        "../../../examples/peach-bot/deform-bot.peachy"
    ))
    .unwrap();
    let authored = sample_authored_pose_with_skin(&doc, Some(0), 0.6, "peach").unwrap();
    let before = authored.clone();
    let calls = crate::ik::checked_pose_calls();
    let pose = checked_pose_from_authored(&doc, &authored).unwrap();
    assert_eq!(crate::ik::checked_pose_calls(), calls + 1);
    assert_eq!(&authored, &before);
    assert_eq!(pose.inherits(), authored.inherits());
    assert_eq!(pose.slot_attachments(), authored.slot_attachments());
    assert_eq!(pose.slot_colors(), authored.slot_colors());
    assert_eq!(pose.draw_order(), authored.draw_order());
    assert_eq!(pose.deforms(), authored.deforms.as_slice());
    assert!(!pose.deforms().is_empty());
    assert_eq!(pose.path_poses(), authored.paths.as_slice());
    assert_eq!(pose.applied_ik_poses(), authored.ik_poses());
    let applied = pose.applied_locals().to_vec();
    let inherits = pose.inherits().to_vec();
    let world = serde_json::to_value(pose.world()).unwrap();
    let scene = checked_scene_from_evaluated(&doc, "peach", pose).unwrap();
    assert_eq!(
        crate::ik::checked_pose_calls(),
        calls + 1,
        "scene conversion must not solve again"
    );
    assert_eq!(serde_json::to_value(&scene.world).unwrap(), world);
    assert_eq!(scene.applied_locals, applied);
    assert!(!scene.attachments.is_empty());
    let mut session = Session::new(doc).unwrap();
    session.set_skin("peach");
    let binding = session
        .mesh_bind_pose(EditTarget::Animate {
            animation: 0,
            time: 0.6,
        })
        .unwrap();
    assert_eq!(binding.locals, applied);
    assert_eq!(binding.inherits, Some(inherits));
}

#[test]
fn applied_state_keeps_sequence_and_hidden_slot_state_in_the_same_capsule() {
    let doc: Document = crate::native_fixture::decode(include_str!(
        "../../../examples/peach-bot/sequence-bot.peachy"
    ))
    .unwrap();
    let time = doc.animations[0].duration() * 0.5;
    let authored = sample_authored_pose(&doc, Some(0), time).unwrap();
    let pose = checked_pose_from_authored(&doc, &authored).unwrap();
    assert_eq!(pose.sequence_frames(), authored.sequence_frames.as_slice());
    assert!(!pose.sequence_frames().is_empty());
    assert_eq!(pose.slot_attachments().len(), doc.slots.len());
    assert_eq!(pose.slot_colors().len(), doc.slots.len());
    assert_eq!(pose.draw_order().len(), doc.slots.len());
    let before = serde_json::to_string(&doc).unwrap();
    checked_scene_from_evaluated(&doc, "default", pose).unwrap();
    assert_eq!(serde_json::to_string(&doc).unwrap(), before);
}

#[test]
fn applied_state_checked_sequence_sampling_rejects_huge_phase_with_attachment_context() {
    let mut doc: Document = crate::native_fixture::decode(include_str!(
        "../../../examples/peach-bot/sequence-bot.peachy"
    ))
    .unwrap();
    doc.animations[0].tracks = vec![Track {
        bone: 0,
        property: Property::X,
        keys: vec![
            Keyframe {
                time: 0.,
                value: 0.,
                curve: Curve::Linear,
            },
            Keyframe {
                time: f32::MAX,
                value: 0.,
                curve: Curve::Linear,
            },
        ],
    }];
    let track = &mut doc.animations[0].sequence_tracks[0];
    track.keys = vec![SequenceKey {
        time: 0.,
        mode: SequenceMode::Loop,
        index: 1,
        delay: f32::MIN_POSITIVE,
    }];
    let slot = track.attachment.slot.clone();
    doc.validate().unwrap();
    let before = serde_json::to_string(&doc).unwrap();
    let error = sample_authored_pose(&doc, Some(0), f32::MAX).unwrap_err();
    assert!(error.contains("sequence") && error.contains(&slot) && error.contains("counter range"));
    assert_eq!(serde_json::to_string(&doc).unwrap(), before);
}

#[test]
fn applied_state_rejects_wrong_scene_skin_layout_and_nonfinite_metadata_before_geometry() {
    let doc: Document = crate::native_fixture::decode(include_str!(
        "../../../examples/peach-bot/deform-bot.peachy"
    ))
    .unwrap();
    let authored = sample_authored_pose_with_skin(&doc, Some(0), 0.6, "peach").unwrap();
    let pose = checked_pose_from_authored(&doc, &authored).unwrap();
    assert!(
        checked_scene_from_evaluated(&doc, "default", pose.clone())
            .err()
            .unwrap()
            .contains("skin")
    );
    let mut truncated = pose.clone();
    truncated.slot_attachments.pop();
    assert!(checked_scene_from_evaluated(&doc, "peach", truncated).is_err());
    let mut invalid = pose;
    invalid.slot_colors[0][0] = f32::NAN;
    assert!(checked_scene_from_evaluated(&doc, "peach", invalid).is_err());
    let mut invalid_authored = authored;
    invalid_authored.draw_order.clear();
    assert!(checked_pose_from_authored(&doc, &invalid_authored).is_err());
}
