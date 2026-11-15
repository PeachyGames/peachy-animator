//! Host activity is resolved before pending data, independently of Slider writes.
use crate::*;
use peachy_document::*;
use std::collections::{BTreeMap, BTreeSet};

fn attachment(slot: &str, placeholder: &str) -> AttachmentRef {
    AttachmentRef {
        skin: "default".into(),
        slot: slot.into(),
        placeholder: placeholder.into(),
    }
}

fn document() -> Document {
    let mut doc = Document::new("Original skin sampling lab".into(), 60).unwrap();
    doc.bones.push(Bone {
        name: "optional".into(),
        parent: Some(0),
        length: 0.,
        setup: Transform::default(),
        inherit: Inherit::Normal,
    });
    for (name, bone, shown) in [
        ("marker", 1, "setup"),
        ("plain", 1, "quad"),
        ("global", 0, "setup"),
    ] {
        doc.slots.push(Slot {
            name: name.into(),
            bone,
            attachment: Some(shown.into()),
            color: [128, 64, 32, 128],
            blend: BlendMode::Normal,
        });
    }
    let region = || {
        Attachment::Region(Region {
            name: None,
            path: "blue_cutout".into(),
            width: 128.,
            height: 128.,
            transform: RegionTransform::default(),
            color: WHITE,
            sequence: Some(SequenceDef {
                count: 3,
                start: 0,
                digits: 1,
                setup_index: 1,
            }),
        })
    };
    let mesh = Attachment::Mesh(Mesh {
        name: None,
        path: "blue_cutout".into(),
        vertices: MeshVertices::SlotLocal {
            positions: vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
        },
        uvs: vec![[0., 1.], [1., 1.], [1., 0.], [0., 0.]],
        triangles: vec![0, 1, 2, 2, 3, 0],
        hull: 4,
        width: 128.,
        height: 128.,
        edges: vec![],
        color: WHITE,
        link: None,
        sequence: None,
    });
    doc.skins.insert(
        "default".into(),
        BTreeMap::from([
            (
                "marker".into(),
                BTreeMap::from([("setup".into(), region()), ("alternate".into(), region())]),
            ),
            ("plain".into(), BTreeMap::from([("quad".into(), mesh)])),
            (
                "global".into(),
                BTreeMap::from([("setup".into(), region())]),
            ),
        ]),
    );
    doc.skins.insert("awake".into(), BTreeMap::new());
    doc.skin_scope.required_bones = BTreeSet::from([1]);
    doc.skin_scope.skins.insert(
        "awake".into(),
        SkinMembers {
            bones: vec![1],
            constraints: vec![],
        },
    );
    let mut clip = Animation::new("host", vec![]);
    clip.attachment_tracks.push(AttachmentTrack {
        slot: 0,
        keys: vec![AttachmentKey {
            time: 0.,
            attachment: Some("alternate".into()),
        }],
    });
    clip.color_tracks = [0, 2]
        .into_iter()
        .map(|slot| ColorTrack {
            slot,
            keys: vec![ColorKey {
                time: 0.,
                color: [64, 128, 192, 224],
                curves: [Curve::Linear; 4],
            }],
        })
        .collect();
    clip.deform_tracks.push(DeformTrack {
        attachment: attachment("plain", "quad"),
        keys: vec![DeformKey {
            time: 0.,
            curve: Curve::Linear,
            offsets: vec![1., 2., 3., 4., 5., 6., 7., 8.],
        }],
    });
    clip.sequence_tracks.push(SequenceTrack {
        attachment: attachment("marker", "setup"),
        keys: vec![SequenceKey {
            time: 0.,
            mode: SequenceMode::Hold,
            index: 2,
            delay: 0.,
        }],
    });
    clip.draw_order.push(DrawOrderKey {
        time: 0.,
        offsets: vec![DrawOrderOffset {
            slot: 2,
            offset: -2,
        }],
    });
    doc.animations.push(clip);
    doc.validate().unwrap();
    doc
}

fn target() -> EditTarget {
    EditTarget::Animate {
        animation: 0,
        time: 0.,
    }
}

#[test]
fn skin_sampling_inactive_host_slots_keep_setup_and_named_activation_samples_their_keys() {
    let doc = document();
    let setup = doc.slots[0].color.map(|byte| f32::from(byte) / 255.);
    let default = sample_authored_pose_with_skin(&doc, Some(0), 0., "default").unwrap();
    assert_eq!(default.slot_attachments()[0].as_deref(), Some("setup"));
    assert_eq!(default.slot_colors()[0], setup);
    assert!(default.deforms.is_empty() && default.sequence_frames.is_empty());
    assert_eq!(default.draw_order(), [2, 0, 1]);
    assert_eq!(
        default.slot_colors()[2],
        [64_u8, 128, 192, 224].map(|byte| f32::from(byte) / 255.)
    );

    let awake = sample_authored_pose_with_skin(&doc, Some(0), 0., "awake").unwrap();
    assert_eq!(awake.slot_attachments()[0].as_deref(), Some("alternate"));
    assert_eq!(awake.slot_colors()[0], awake.slot_colors()[2]);
    assert_eq!(awake.deforms[0].1, [1., 2., 3., 4., 5., 6., 7., 8.]);
    assert_eq!(awake.sequence_frames[0].1, 2);
    let mut session = Session::new(doc.clone()).unwrap();
    session.set_skin("awake");
    assert_eq!(session.authored_pose(target()).unwrap(), awake);
    let scene = checked_evaluate_scene(&doc, Some(0), 0., "awake", None).unwrap();
    let from_capsule = checked_scene_from_authored(&doc, "awake", &awake).unwrap();
    assert_eq!(scene.locals, from_capsule.locals);
    assert_eq!(scene.applied_locals, from_capsule.applied_locals);
    assert_eq!(
        serde_json::to_value(scene.world).unwrap(),
        serde_json::to_value(from_capsule.world).unwrap()
    );
    assert_eq!(scene.bounds, from_capsule.bounds);
    assert_eq!(scene.active_bones, from_capsule.active_bones);
    assert_eq!(
        serde_json::to_value(scene.attachments).unwrap(),
        serde_json::to_value(from_capsule.attachments).unwrap()
    );
    assert!(
        checked_scene_from_authored(&doc, "awake", &default)
            .err()
            .unwrap()
            .contains("skin")
    );
}

#[test]
fn skin_sampling_pending_overlays_survive_skin_changes_history_and_document_save() {
    let doc = document();
    let mut session = Session::new(doc.clone()).unwrap();
    let pending_color = [0.125, 0.25, 0.5, 0.75];
    session
        .edit_slot_pose(
            0,
            target(),
            &[
                SlotPoseEdit::Color {
                    color: pending_color,
                },
                SlotPoseEdit::Attachment { attachment: None },
            ],
            false,
        )
        .unwrap();
    session
        .edit_deform_pose(attachment("plain", "quad"), target(), vec![9.; 8], false)
        .unwrap();
    let pending = serde_json::to_value(session.pose_edits()).unwrap();
    for skin in ["default", "awake", "default", "awake"] {
        session.set_skin(skin);
        let authored = session.authored_pose(target()).unwrap();
        assert_eq!(authored.slot_colors()[0], pending_color);
        assert_eq!(authored.slot_attachments()[0], None);
        assert_eq!(authored.deforms[0].1, [9.; 8]);
        assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pending);
        assert_eq!(session.document(), &doc);
        assert!(!session.is_dirty());
    }
    session.undo();
    assert_eq!(session.skin(), "awake");
    assert!(
        !session
            .pose_edits()
            .has_deform(0, &attachment("plain", "quad"))
    );
    assert_eq!(session.slot_pose(0, target()).unwrap().color, pending_color);
    session.redo();
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pending);
    session.begin_transaction();
    session
        .preview_edit_slot_pose(
            0,
            target(),
            &[SlotPoseEdit::Color { color: [1.; 4] }],
            false,
        )
        .unwrap();
    session.cancel_transaction();
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pending);
    assert_eq!(
        serde_json::to_value(session.document()).unwrap(),
        serde_json::to_value(&doc).unwrap()
    );
}

#[test]
fn skin_sampling_named_slot_noop_preserves_redo_and_keying_uses_same_skin_baseline() {
    let mut session = Session::new(document()).unwrap();
    session
        .dispatch(Command::RenameBone {
            bone: 0,
            name: "temporary".into(),
        })
        .unwrap();
    session.undo();
    assert!(session.can_redo());
    for skin in ["default", "awake"] {
        session.set_skin(skin);
        let current = session.slot_pose(0, target()).unwrap();
        session
            .edit_slot_pose(
                0,
                target(),
                &[SlotPoseEdit::Color {
                    color: current.color,
                }],
                false,
            )
            .unwrap();
        assert!(session.can_redo() && session.pose_edits().is_empty());
    }
    session
        .edit_slot_pose(
            0,
            target(),
            &[SlotPoseEdit::Color {
                color: [0.25, 0.5, 0.75, 1.],
            }],
            false,
        )
        .unwrap();
    let pending = serde_json::to_value(session.pose_edits()).unwrap();
    let document = session.document().clone();
    session.set_skin("default");
    assert!(
        session
            .key_slot_pose(0, 0., 0, &[SlotProperty::Color])
            .is_err()
    );
    assert_eq!(session.document(), &document);
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pending);
    session.set_skin("awake");
    session
        .key_slot_pose(0, 0., 0, &[SlotProperty::Color])
        .unwrap();
    assert!(session.pose_edits().is_empty());
    session.undo();
    assert_eq!(session.document(), &document);
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pending);
    assert_eq!(session.skin(), "awake");
}

#[test]
fn skin_sampling_exact_current_edits_keep_pending_equal_to_new_skin_base_and_redo() {
    let doc = document();
    let named = sample_authored_pose_with_skin(&doc, Some(0), 0., "awake").unwrap();
    let color = named.slot_colors()[0];
    let shown = named.slot_attachments()[0].clone();
    let offsets = named.deforms[0].1.clone();
    let mut session = Session::new(doc.clone()).unwrap();
    session
        .edit_slot_pose(
            0,
            target(),
            &[
                SlotPoseEdit::Color { color },
                SlotPoseEdit::Attachment {
                    attachment: shown.clone(),
                },
            ],
            false,
        )
        .unwrap();
    session
        .edit_deform_pose(
            attachment("plain", "quad"),
            target(),
            offsets.clone(),
            false,
        )
        .unwrap();
    session.set_skin("awake");
    session
        .dispatch(Command::RenameBone {
            bone: 0,
            name: "temporary".into(),
        })
        .unwrap();
    session.undo();
    let pending = serde_json::to_value(session.pose_edits()).unwrap();
    assert!(session.can_redo() && !session.pose_edits().is_empty());

    session
        .edit_slot_pose(
            0,
            target(),
            &[
                SlotPoseEdit::Color { color },
                SlotPoseEdit::Attachment { attachment: shown },
            ],
            true,
        )
        .unwrap();
    session
        .edit_deform_pose(attachment("plain", "quad"), target(), offsets, true)
        .unwrap();
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pending);
    assert!(session.can_redo());
    assert_eq!(session.document(), &doc);

    // The no-op fast path must not bypass duplicate edits or layout validation.
    assert!(
        session
            .edit_slot_pose(
                0,
                target(),
                &[SlotPoseEdit::Color { color }, SlotPoseEdit::Color { color },],
                false
            )
            .is_err()
    );
    assert!(
        session
            .edit_deform_pose(attachment("plain", "quad"), target(), vec![1., 2.], false)
            .is_err()
    );
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pending);
    assert!(session.can_redo());
    assert_eq!(session.document(), &doc);
}
