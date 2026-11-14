use crate::*;
use peachy_document::*;

fn fixture() -> Document {
    let mut doc: Document = serde_json::from_str(include_str!(
        "../../../examples/peach-bot/deform-bot.peachy"
    ))
    .unwrap();
    for clip in &mut doc.animations {
        clip.tracks.clear();
        clip.deform_tracks.clear();
    }
    doc
}
fn key(weighted: bool) -> AttachmentRef {
    AttachmentRef {
        skin: if weighted { "default" } else { "peach" }.into(),
        slot: if weighted { "hand.R" } else { "body" }.into(),
        placeholder: if weighted { "hand.R" } else { "body" }.into(),
    }
}
fn animate(time: f32) -> EditTarget {
    EditTarget::Animate { animation: 0, time }
}
fn geometry(session: &Session, attachment: &AttachmentRef, time: f32) -> Vec<[f32; 2]> {
    crate::deform_authoring::geometry(
        session.document(),
        &session.authored_pose(animate(time)).unwrap(),
        attachment,
    )
    .unwrap()
}
fn close(actual: [f32; 2], expected: [f32; 2]) {
    for axis in 0..2 {
        assert!(
            (actual[axis] - expected[axis]).abs() <= 0.001,
            "{actual:?} != {expected:?}"
        );
    }
}

#[test]
fn deform_authoring_pending_world_drags_preserve_setup_bindings_and_other_vertices() {
    for weighted in [false, true] {
        let mut doc = fixture();
        if weighted {
            let Some(Attachment::Mesh(mesh)) = doc.attachment_mut(&key(true)) else {
                panic!()
            };
            let MeshVertices::Weighted { influences } = &mut mesh.vertices else {
                panic!()
            };
            // Nonunit totals and repeated bones are valid imported data. Retain them.
            influences[0][0].weight = 0.9;
            influences[0].push(BoneInfluence {
                bone: 7,
                position: [10., 20.],
                weight: 0.2,
            });
            doc.bones[7].setup.scale_x = -1.25;
            doc.bones[8].setup.scale_y = 0.7;
        }
        let original = doc.clone();
        let mut session = Session::new(doc).unwrap();
        session.set_skin("peach");
        let attachment = key(weighted);
        let before = geometry(&session, &attachment, 0.);
        session
            .translate_deform_vertex(attachment.clone(), 0, animate(0.), [7.5, -3.25], false)
            .unwrap();
        let after = geometry(&session, &attachment, 0.);
        close(after[0], [before[0][0] + 7.5, before[0][1] - 3.25]);
        assert_eq!(&after[1..], &before[1..]);
        assert_eq!(session.document(), &original);
        assert!(!session.is_dirty());
        assert_eq!(
            session
                .pose_edits()
                .deform_key_state(session.document(), 0, 0., &attachment)
                .unwrap(),
            KeyState::Edited
        );
        session.key_deform_pose(0, 0., attachment.clone()).unwrap();
        assert!(session.pose_edits().is_empty());
        assert!(session.is_dirty());
        assert_eq!(geometry(&session, &attachment, 0.), after);
        assert_eq!(session.document().skins, original.skins);
        assert_eq!(
            session
                .pose_edits()
                .deform_key_state(session.document(), 0, 0., &attachment)
                .unwrap(),
            KeyState::Keyed
        );
        session.undo();
        assert_eq!(session.document(), &original);
        assert_eq!(geometry(&session, &attachment, 0.), after);
        session.redo();
        assert!(session.pose_edits().is_empty());
    }
}

#[test]
fn deform_authoring_transaction_previews_recompute_from_the_frozen_pending_pose() {
    let mut session = Session::new(fixture()).unwrap();
    session.set_skin("peach");
    let attachment = key(true);
    let mut local = session.local_pose(Some(0), 0.)[0];
    local.x += 12.;
    session
        .edit_pose(0, animate(0.), local, &[Property::X], false)
        .unwrap();
    session
        .translate_deform_vertex(attachment.clone(), 1, animate(0.), [2., 4.], false)
        .unwrap();
    session
        .translate_deform_vertex(key(false), 0, animate(0.), [1., 2.], false)
        .unwrap();
    let baseline = geometry(&session, &attachment, 0.);
    let pending = session.pose_edits().clone();
    session.begin_transaction();
    for delta in [[1., 3.], [4., 6.], [0., 0.]] {
        session
            .preview_translate_deform_vertex(attachment.clone(), 0, animate(0.), delta, false)
            .unwrap();
        let actual = geometry(&session, &attachment, 0.);
        close(
            actual[0],
            [baseline[0][0] + delta[0], baseline[0][1] + delta[1]],
        );
        assert_eq!(&actual[1..], &baseline[1..]);
    }
    assert_eq!(session.pose_edits(), &pending);
    session
        .preview_translate_deform_vertex(attachment.clone(), 0, animate(0.), [5., -7.], false)
        .unwrap();
    let preview = geometry(&session, &attachment, 0.);
    session.commit_transaction();
    session.undo();
    assert_eq!(geometry(&session, &attachment, 0.), baseline);
    assert_eq!(session.pose_edits(), &pending);
    session.redo();
    assert_eq!(geometry(&session, &attachment, 0.), preview);
    session.undo();
    assert!(session.can_redo());
    session.begin_transaction();
    session
        .preview_translate_deform_vertex(attachment.clone(), 0, animate(0.), [3., 2.], false)
        .unwrap();
    session.cancel_transaction();
    assert!(session.can_redo());
    assert_eq!(geometry(&session, &attachment, 0.), baseline);
}

#[test]
fn deform_authoring_auto_key_delete_and_history_use_existing_dense_format() {
    let mut session = Session::new(fixture()).unwrap();
    let attachment = key(false);
    let original = session.document().clone();
    session
        .translate_deform_vertex(attachment.clone(), 0, animate(1.), [4., -1.], true)
        .unwrap();
    assert!(session.pose_edits().is_empty());
    assert_eq!(session.document().animations[0].duration(), 1.);
    assert_eq!(session.document().format, FORMAT_VERSION);
    let offsets = session.deform_offsets(&attachment, animate(1.)).unwrap();
    assert_eq!(
        session.document().animations[0].deform_tracks[0].keys[0].offsets,
        offsets
    );
    session
        .dispatch(Command::DeleteDeformKey {
            animation: 0,
            attachment,
            time: 1.,
        })
        .unwrap();
    assert_eq!(session.document(), &original);
    session.undo();
    assert_eq!(session.document().animations[0].duration(), 1.);
}

#[test]
fn deform_authoring_linked_meshes_key_the_canonical_source_and_keep_exact_identities() {
    let mut doc = fixture();
    let Attachment::Mesh(mut mesh) = doc.attachment(&key(true)).unwrap().clone() else {
        panic!()
    };
    mesh.name = Some("linked-hand".into());
    mesh.link = Some(MeshLink {
        skin: "default".into(),
        source: "hand.R".into(),
        timelines: true,
    });
    let link = AttachmentRef {
        skin: "peach".into(),
        slot: "hand.R".into(),
        placeholder: "linked-hand".into(),
    };
    doc.skins
        .get_mut("peach")
        .unwrap()
        .entry("hand.R".into())
        .or_default()
        .insert("linked-hand".into(), Attachment::Mesh(mesh));
    let len = doc
        .attachment(&key(true))
        .unwrap()
        .vertices()
        .unwrap()
        .deform_len();
    for (time, value) in [(0.4, 0.), (0.400005, 2.)] {
        doc = apply_command(
            &doc,
            Command::SetDeformKey {
                animation: 0,
                attachment: link.clone(),
                key: DeformKey {
                    time,
                    curve: Curve::Stepped,
                    offsets: vec![value; len],
                },
            },
        )
        .unwrap();
    }
    assert_eq!(doc.animations[0].deform_tracks.len(), 1);
    assert_eq!(doc.animations[0].deform_tracks[0].attachment, key(true));
    assert_eq!(doc.animations[0].deform_tracks[0].keys.len(), 2);
    let mut session = Session::new(doc).unwrap();
    assert!(
        session
            .pose_edits()
            .deform_key_state(session.document(), 0, 0.4000025, &link)
            .is_err()
    );
    session
        .translate_deform_vertex(link.clone(), 0, animate(0.400005), [1., 2.], false)
        .unwrap();
    session.key_deform_pose(0, 0.400005, link.clone()).unwrap();
    let keys = &session.document().animations[0].deform_tracks[0].keys;
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[1].time, 0.400005);
    assert_eq!(keys[1].curve, Curve::Stepped);
    session
        .dispatch(Command::DeleteDeformKey {
            animation: 0,
            attachment: link,
            time: 0.4,
        })
        .unwrap();
    assert_eq!(
        session.document().animations[0].deform_tracks[0].keys.len(),
        1
    );
    assert_eq!(
        session.document().animations[0].deform_tracks[0].keys[0].time,
        0.400005
    );
}

#[test]
fn deform_authoring_key_edited_groups_pending_bone_ik_and_deform_without_saving_unkeyed_data() {
    let doc: Document = serde_json::from_str(include_str!(
        "../../../examples/peach-bot/ik-animation-bot.peachy"
    ))
    .unwrap();
    let original = doc.clone();
    let mut session = Session::new(doc).unwrap();
    let mut root = session.local_pose(Some(0), 0.)[0];
    root.x += 5.;
    session
        .edit_pose(0, animate(0.), root, &[Property::X], false)
        .unwrap();
    session
        .edit_ik_pose(0, animate(0.), &[IkPoseEdit::Mix { value: 0.4 }], false)
        .unwrap();
    session
        .translate_deform_vertex(key(true), 0, animate(0.), [3., 1.], false)
        .unwrap();
    assert!(!session.is_dirty());
    assert_eq!(session.document(), &original);
    let posed = geometry(&session, &key(true), 0.);
    session.key_edited(0, 0., Some(0)).unwrap();
    assert!(session.pose_edits().has_deform(0, &key(true)));
    assert!(session.pose_edits().has_ik(0, 0));
    session.key_edited(0, 0., None).unwrap();
    assert!(session.pose_edits().is_empty());
    close(geometry(&session, &key(true), 0.)[0], posed[0]);
    session.undo();
    assert!(session.pose_edits().has_deform(0, &key(true)));
    session.discard_pose(true);
    assert!(session.pose_edits().is_empty());
    session.undo();
    // An existing redo branch intentionally removes stale poses from history.
    assert!(!session.pose_edits().has_deform(0, &key(true)));
    let mut recoverable = Session::new(fixture()).unwrap();
    recoverable
        .translate_deform_vertex(key(true), 0, animate(0.), [1., 2.], false)
        .unwrap();
    recoverable.discard_pose(true);
    assert!(recoverable.pose_edits().is_empty());
    recoverable.undo();
    assert!(recoverable.pose_edits().has_deform(0, &key(true)));
}

#[test]
fn deform_authoring_failures_are_atomic_and_preserve_the_last_valid_preview() {
    let mut session = Session::new(fixture()).unwrap();
    let attachment = key(true);
    session.begin_transaction();
    session
        .preview_translate_deform_vertex(attachment.clone(), 0, animate(0.), [1., 2.], false)
        .unwrap();
    let valid = session.pose_edits().clone();
    for delta in [[f32::NAN, 0.], [f32::MAX, f32::MAX]] {
        assert!(
            session
                .preview_translate_deform_vertex(attachment.clone(), 0, animate(0.), delta, false)
                .is_err()
        );
        assert_eq!(session.pose_edits(), &valid);
    }
    assert!(
        session
            .preview_translate_deform_vertex(
                attachment.clone(),
                usize::MAX,
                animate(0.),
                [1., 2.],
                false
            )
            .is_err()
    );
    assert!(
        session
            .edit_deform_pose(attachment.clone(), animate(0.), vec![0.], false)
            .is_err()
    );
    assert!(
        session
            .translate_deform_vertex(attachment, 0, EditTarget::Setup, [1., 2.], false)
            .is_err()
    );
    assert_eq!(session.pose_edits(), &valid);
    assert!(!session.is_dirty());
    session.cancel_transaction();
    assert!(!session.can_undo());
}

#[test]
fn deform_authoring_singular_contributions_are_guarded_and_zero_weight_entries_preserved() {
    let mut doc = fixture();
    doc.bones[7].setup.scale_x = 0.;
    let mut session = Session::new(doc).unwrap();
    assert!(
        session
            .translate_deform_vertex(key(true), 0, animate(0.), [1., 0.], false)
            .is_err()
    );
    assert!(session.pose_edits().is_empty());
    assert!(!session.can_undo());
    let mut doc = fixture();
    doc.bones[3].setup.scale_x = 0.;
    let mut session = Session::new(doc).unwrap();
    let attachment = key(true);
    session
        .translate_deform_vertex(attachment.clone(), 4, animate(0.), [1., 2.], false)
        .unwrap();
    let values = session.deform_offsets(&attachment, animate(0.)).unwrap();
    assert_eq!(&values[values.len() - 2..], &[0., 0.]);
}

#[test]
fn deform_authoring_pending_slot_renames_remap_and_layout_edits_fail_atomically() {
    let mut session = Session::new(fixture()).unwrap();
    let attachment = key(false);
    session
        .translate_deform_vertex(attachment.clone(), 0, animate(0.), [2., 3.], false)
        .unwrap();
    let old = session.deform_offsets(&attachment, animate(0.)).unwrap();
    let before = session.document().clone();
    assert!(
        session
            .dispatch(Command::SplitMeshTriangle {
                attachment: attachment.clone(),
                triangle: 0
            })
            .is_err()
    );
    assert_eq!(session.document(), &before);
    assert_eq!(
        session.deform_offsets(&attachment, animate(0.)).unwrap(),
        old
    );
    let slot = session
        .document()
        .slots
        .iter()
        .position(|slot| slot.name == attachment.slot)
        .unwrap();
    session
        .dispatch(Command::RenameSlot {
            slot,
            name: "renamed-body".into(),
        })
        .unwrap();
    let renamed = AttachmentRef {
        slot: "renamed-body".into(),
        ..attachment.clone()
    };
    assert_eq!(session.deform_offsets(&renamed, animate(0.)).unwrap(), old);
    session.key_deform_pose(0, 0., renamed.clone()).unwrap();
    assert_eq!(
        session.document().animations[0].deform_tracks[0].attachment,
        renamed
    );
    session.undo();
    session.undo();
    assert_eq!(
        session.deform_offsets(&attachment, animate(0.)).unwrap(),
        old
    );

    // One influence per vertex still has the same number of dense values as
    // an unweighted mesh. A representation change must not reinterpret them.
    let doc = fixture();
    let mut session = Session::new(doc).unwrap();
    session
        .bind_mesh_to_bone(attachment.clone(), 0, EditTarget::Setup)
        .unwrap();
    session
        .translate_deform_vertex(attachment.clone(), 0, animate(0.), [1., 2.], true)
        .unwrap();
    let before = session.document().clone();
    assert!(session.unbind_mesh(attachment, EditTarget::Setup).is_err());
    assert_eq!(session.document(), &before);
}

#[test]
fn deform_authoring_new_fractional_key_refuses_a_snap_that_would_move_the_pose() {
    let mut doc = fixture();
    let attachment = key(false);
    let len = doc
        .attachment(&attachment)
        .unwrap()
        .vertices()
        .unwrap()
        .deform_len();
    for (time, value) in [(0., 0.), (1., 20.)] {
        doc = apply_command(
            &doc,
            Command::SetDeformKey {
                animation: 0,
                attachment: attachment.clone(),
                key: DeformKey {
                    time,
                    curve: Curve::Linear,
                    offsets: vec![value; len],
                },
            },
        )
        .unwrap();
    }
    let mut session = Session::new(doc).unwrap();
    session
        .translate_deform_vertex(attachment.clone(), 0, animate(0.51), [1., 2.], false)
        .unwrap();
    let pending = session.pose_edits().clone();
    let document = session.document().clone();
    assert!(session.key_deform_pose(0, 0.51, attachment).is_err());
    assert_eq!(session.document(), &document);
    assert_eq!(session.pose_edits(), &pending);
    assert!(!session.is_dirty());
}

#[test]
fn deform_authoring_delete_resolves_the_final_fractional_key_from_a_nearby_cursor_past_end() {
    let doc = fixture();
    let attachment = key(false);
    let len = doc
        .attachment(&attachment)
        .unwrap()
        .vertices()
        .unwrap()
        .deform_len();
    let final_time = 0.407123;
    let cursor = final_time + 0.000005;
    let keyed = apply_command(
        &doc,
        Command::SetDeformKey {
            animation: 0,
            attachment: attachment.clone(),
            key: DeformKey {
                time: final_time,
                curve: Curve::Stepped,
                offsets: vec![2.; len],
            },
        },
    )
    .unwrap();
    let mut session = Session::new(keyed.clone()).unwrap();
    assert!(cursor > session.document().animations[0].duration());
    assert_eq!(
        session
            .pose_edits()
            .deform_key_state(session.document(), 0, cursor, &attachment)
            .unwrap(),
        KeyState::Keyed
    );
    session
        .dispatch(Command::DeleteDeformKey {
            animation: 0,
            attachment: attachment.clone(),
            time: cursor,
        })
        .unwrap();
    assert_eq!(session.document(), &doc);
    session.undo();
    assert_eq!(session.document(), &keyed);
    session
        .dispatch(Command::DeleteDeformKey {
            animation: 0,
            attachment,
            time: final_time + 1.,
        })
        .unwrap();
    assert_eq!(session.document(), &keyed);
    assert!(session.can_redo(), "No-match delete must preserve redo");
}
