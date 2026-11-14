//! Exact Deform timeline edits must preserve dense data, typed ownership,
//! linked sources and history; geometry checks distinguish curves from timing.
use crate::*;
use peachy_document::*;

fn lab() -> Document {
    serde_json::from_str(include_str!(
        "../../../examples/peach-bot/deform-bot.peachy"
    ))
    .unwrap()
}
fn linked_lab() -> Document {
    serde_json::from_str(include_str!(
        "../../../examples/peach-bot/linked-mesh-bot.peachy"
    ))
    .unwrap()
}
fn reference(skin: &str, slot: &str, placeholder: &str) -> AttachmentRef {
    AttachmentRef {
        skin: skin.into(),
        slot: slot.into(),
        placeholder: placeholder.into(),
    }
}
fn hand() -> AttachmentRef {
    reference("default", "hand.R", "hand.R")
}
fn body(skin: &str) -> AttachmentRef {
    reference(skin, "body", "body")
}
fn identity(attachment: &AttachmentRef, time: f32) -> AnimationKey {
    AnimationKey::Deform {
        attachment: attachment.clone(),
        time,
    }
}
fn track<'a>(doc: &'a Document, animation: usize, attachment: &AttachmentRef) -> &'a DeformTrack {
    doc.animations[animation]
        .deform_tracks
        .iter()
        .find(|t| t.attachment == *attachment)
        .unwrap()
}
fn owner(attachment: &AttachmentRef) -> KeyOwner {
    KeyOwner::Deform {
        attachment: attachment.clone(),
    }
}

#[test]
fn deform_timeline_exact_retiming_copy_and_mixed_keys_preserve_original_dense_values() {
    let mut document = lab();
    let original = track(&document, 0, &hand()).keys[1].clone();
    let close = f32::from_bits(0.25_f32.to_bits() + 2);
    document.animations[0].deform_tracks[0].keys = vec![
        DeformKey {
            time: 0.25,
            curve: Curve::Stepped,
            ..original.clone()
        },
        DeformKey {
            time: close,
            curve: Curve::Bezier([0.2, -0.5, 0.8, 1.5]),
            offsets: original.offsets.iter().map(|v| v + 1.).collect(),
        },
    ];
    document.validate().unwrap();
    let original = track(&document, 0, &hand()).keys.clone();
    let bone_track = document.animations[0].tracks[0].clone();
    let bone_key = bone_track.keys[0];
    let selected = vec![
        identity(&hand(), close),
        AnimationKey::Bone {
            bone: bone_track.bone,
            property: bone_track.property,
            time: bone_key.time,
        },
        identity(&hand(), 0.25),
    ];
    let moved = apply_command(
        &document,
        Command::MoveAnimationKeys {
            animation: 0,
            keys: selected.clone(),
            delta: 0.125,
            copy: false,
        },
    )
    .unwrap();
    let moved_keys = &track(&moved, 0, &hand()).keys;
    assert_eq!(moved_keys.len(), 2);
    for (key, previous) in moved_keys.iter().zip(&original) {
        assert_eq!(key.time, previous.time + 0.125);
        assert_eq!(key.offsets, previous.offsets);
        assert_eq!(key.curve, previous.curve);
    }
    assert!(moved_keys[0].time < moved_keys[1].time);
    assert_eq!(moved.skins, document.skins);
    assert_eq!(
        track(&moved, 0, &body("peach")),
        track(&document, 0, &body("peach"))
    );
    assert!(
        moved.animations[0]
            .tracks
            .iter()
            .find(|t| t.bone == bone_track.bone && t.property == bone_track.property)
            .unwrap()
            .keys
            .iter()
            .any(|key| key.time == bone_key.time + 0.125 && key.value == bone_key.value)
    );
    let copied = apply_command(
        &document,
        Command::MoveAnimationKeys {
            animation: 0,
            keys: vec![identity(&hand(), 0.25)],
            delta: 10.,
            copy: true,
        },
    )
    .unwrap();
    assert_eq!(track(&copied, 0, &hand()).keys[..2], original);
    let last = track(&copied, 0, &hand()).keys.last().unwrap();
    assert_eq!(last.time, 10.25);
    assert_eq!(last.offsets, original[0].offsets);
    assert_eq!(last.curve, original[0].curve);
    assert_eq!(copied.animations[0].duration(), 10.25);
    assert_eq!(document.animations[0].deform_tracks[0].keys, original);
}

#[test]
fn deform_timeline_fixed_base_overwrite_delete_and_undo_restore_geometry_and_pending_pose() {
    let document = lab();
    let baseline = document.clone();
    let source = track(&document, 0, &hand()).keys[1].clone();
    let mut session = Session::new(document).unwrap();
    let edit_target = EditTarget::Animate {
        animation: 0,
        time: 0.6,
    };
    let mut pending = session.local_pose(Some(0), 0.6)[0];
    pending.x += 3.;
    session
        .edit_pose(0, edit_target, pending, &[Property::X], false)
        .unwrap();
    let pose = session.pose_edits().clone();
    session
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "temporary".into(),
        })
        .unwrap();
    session.undo();
    assert!(session.can_redo());
    session.begin_transaction();
    session
        .preview_transaction(Command::MoveAnimationKeys {
            animation: 0,
            keys: vec![identity(&hand(), 0.6)],
            delta: 0.6,
            copy: false,
        })
        .unwrap();
    assert_eq!(track(session.document(), 0, &hand()).keys.len(), 4);
    assert_eq!(
        track(session.document(), 0, &hand())
            .keys
            .iter()
            .find(|k| k.time == 1.2)
            .unwrap()
            .offsets,
        source.offsets
    );
    session
        .preview_transaction(Command::MoveAnimationKeys {
            animation: 0,
            keys: vec![identity(&hand(), 0.6)],
            delta: 0.3,
            copy: false,
        })
        .unwrap();
    assert_eq!(track(session.document(), 0, &hand()).keys.len(), 5);
    assert_eq!(
        track(session.document(), 0, &hand())
            .keys
            .iter()
            .find(|k| k.time == 1.2)
            .unwrap(),
        &track(&baseline, 0, &hand()).keys[2]
    );
    session.cancel_transaction();
    assert_eq!(session.document(), &baseline);
    assert_eq!(session.pose_edits(), &pose);
    assert!(session.can_redo());
    session.begin_transaction();
    session
        .preview_transaction(Command::MoveAnimationKeys {
            animation: 0,
            keys: vec![identity(&hand(), 0.6)],
            delta: 0.6,
            copy: false,
        })
        .unwrap();
    session.commit_transaction();
    let changed = session.document().clone();
    assert!(!session.can_redo());
    session.undo();
    assert_eq!(session.document(), &baseline);
    assert_eq!(session.pose_edits(), &pose);
    session.redo();
    assert_eq!(session.document(), &changed);
    let delete = track(session.document(), 0, &hand())
        .keys
        .iter()
        .map(|k| identity(&hand(), k.time))
        .collect();
    session
        .dispatch(Command::DeleteAnimationKeys {
            animation: 0,
            keys: delete,
        })
        .unwrap();
    assert!(
        !session.document().animations[0]
            .deform_tracks
            .iter()
            .any(|t| t.attachment == hand())
    );
    assert_eq!(
        track(session.document(), 0, &body("peach")),
        track(&baseline, 0, &body("peach"))
    );
    let mut no_hand_deform = baseline.clone();
    no_hand_deform.animations[0]
        .deform_tracks
        .retain(|t| t.attachment != hand());
    let deleted =
        checked_evaluate_scene(session.document(), Some(0), 0.6, "default", None).unwrap();
    let expected = checked_evaluate_scene(&no_hand_deform, Some(0), 0.6, "default", None).unwrap();
    assert_eq!(
        serde_json::to_value(deleted.attachments).unwrap(),
        serde_json::to_value(expected.attachments).unwrap()
    );
    session.undo();
    assert_eq!(session.document(), &changed);
}

#[test]
fn deform_timeline_clipboard_can_cross_compatible_attachments_and_animations_without_borrowing_source()
 {
    let mut document = lab();
    let selected = vec![identity(&body("peach"), 1.8), identity(&body("peach"), 0.6)];
    let clipboard = copy_animation_keys(&document, 0, &selected).unwrap();
    let AnimationKeyClipboard::Deform { layout, keys } = &clipboard else {
        panic!("Expected Deform clipboard")
    };
    assert_eq!(*layout, DeformLayout::Unweighted { vertices: 5 });
    assert_eq!(keys[0].time, 0.);
    assert_eq!(keys[1].time, 1.8_f32 - 0.6);
    let captured = keys.clone();
    document.animations[0].deform_tracks[1].keys[1]
        .offsets
        .fill(42.);
    let pasted = apply_command(
        &document,
        Command::PasteAnimationKeys {
            animation: 1,
            target: owner(&body("mint")),
            time: 3.,
            clipboard: clipboard.clone(),
        },
    )
    .unwrap();
    let inserted = &track(&pasted, 1, &body("mint")).keys;
    for (actual, expected) in inserted.iter().zip(&captured) {
        assert_eq!(actual.time, expected.time + 3.);
        assert_eq!(actual.offsets, expected.offsets);
        assert_eq!(actual.curve, expected.curve);
    }
    assert_eq!(pasted.animations[1].duration(), captured[1].time + 3.);
    assert_eq!(pasted.animations[0], document.animations[0]);
    assert_eq!(pasted.skins, document.skins);
    assert!(document.animations[1].deform_tracks.is_empty());
    let mut session = Session::new(document.clone()).unwrap();
    session
        .dispatch(Command::PasteAnimationKeys {
            animation: 1,
            target: owner(&body("mint")),
            time: 3.,
            clipboard,
        })
        .unwrap();
    session.undo();
    assert_eq!(session.document(), &document);
    session.redo();
    assert_eq!(session.document(), &pasted);
}

#[test]
fn deform_timeline_clipboard_checks_per_vertex_influence_layout_instead_of_flat_length() {
    let document = lab();
    let clipboard = copy_animation_keys(&document, 0, &[identity(&hand(), 0.6)]).unwrap();
    let AnimationKeyClipboard::Deform {
        layout: DeformLayout::Weighted { influence_counts },
        keys,
    } = &clipboard
    else {
        panic!("Expected weighted clipboard")
    };
    assert!(influence_counts.contains(&1));
    assert!(influence_counts.contains(&2));
    let mut wrong_boundaries = influence_counts.clone();
    let single = wrong_boundaries.iter().position(|&n| n == 1).unwrap();
    let double = wrong_boundaries.iter().position(|&n| n == 2).unwrap();
    wrong_boundaries.swap(single, double);
    assert_eq!(
        wrong_boundaries.iter().sum::<usize>(),
        influence_counts.iter().sum::<usize>()
    );
    let incompatible = [
        AnimationKeyClipboard::Deform {
            layout: DeformLayout::Weighted {
                influence_counts: wrong_boundaries,
            },
            keys: keys.clone(),
        },
        // Matching the flat value count is insufficient for a multi-influence
        // source: its logical vertex count/segmentation differs from this mesh.
        AnimationKeyClipboard::Deform {
            layout: DeformLayout::Unweighted {
                vertices: keys[0].offsets.len() / 2,
            },
            keys: keys.clone(),
        },
    ];
    for clipboard in incompatible {
        let before = document.clone();
        assert!(
            apply_command(
                &document,
                Command::PasteAnimationKeys {
                    animation: 0,
                    target: owner(&hand()),
                    time: 4.,
                    clipboard,
                }
            )
            .unwrap_err()
            .contains("layout")
        );
        assert_eq!(document, before);
    }
    let copied = copy_animation_keys(&document, 0, &[identity(&body("peach"), 0.6)]).unwrap();
    assert!(
        apply_command(
            &document,
            Command::PasteAnimationKeys {
                animation: 1,
                target: owner(&hand()),
                time: 0.,
                clipboard: copied,
            }
        )
        .is_err()
    );
}

#[test]
fn deform_timeline_clipboard_pastes_single_influence_weighted_and_unweighted_both_directions() {
    let mut document = lab();
    let slot_bone = document
        .slots
        .iter()
        .find(|slot| slot.name == "body")
        .unwrap()
        .bone;
    let target = document.attachment_mut(&body("mint")).unwrap();
    let Attachment::Mesh(mesh) = target else {
        panic!("Expected mesh")
    };
    let MeshVertices::SlotLocal { positions } = &mesh.vertices else {
        panic!("Expected unweighted fixture")
    };
    mesh.vertices = MeshVertices::Weighted {
        influences: positions
            .iter()
            .map(|&position| {
                vec![BoneInfluence {
                    bone: slot_bone,
                    position,
                    weight: 1.,
                }]
            })
            .collect(),
    };
    document.validate().unwrap();
    let clipboard = copy_animation_keys(
        &document,
        0,
        &[identity(&body("peach"), 0.6), identity(&body("peach"), 1.2)],
    )
    .unwrap();
    let AnimationKeyClipboard::Deform {
        layout,
        keys: unweighted_keys,
    } = &clipboard
    else {
        panic!()
    };
    assert_eq!(*layout, DeformLayout::Unweighted { vertices: 5 });
    let unweighted_keys = unweighted_keys.clone();
    let weighted = apply_command(
        &document,
        Command::PasteAnimationKeys {
            animation: 1,
            target: owner(&body("mint")),
            time: 0.,
            clipboard,
        },
    )
    .unwrap();
    assert_eq!(track(&weighted, 1, &body("mint")).keys, unweighted_keys);
    assert!(matches!(
        weighted.attachment(&body("mint")),
        Some(Attachment::Mesh(Mesh {
            vertices: MeshVertices::Weighted { .. },
            ..
        }))
    ));
    let reverse = copy_animation_keys(
        &weighted,
        1,
        &track(&weighted, 1, &body("mint"))
            .keys
            .iter()
            .map(|key| identity(&body("mint"), key.time))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let AnimationKeyClipboard::Deform {
        layout,
        keys: weighted_keys,
    } = &reverse
    else {
        panic!()
    };
    assert_eq!(
        *layout,
        DeformLayout::Weighted {
            influence_counts: vec![1; 5]
        }
    );
    assert_eq!(*weighted_keys, unweighted_keys);
    let roundtrip = apply_command(
        &weighted,
        Command::PasteAnimationKeys {
            animation: 1,
            target: owner(&body("peach")),
            time: 0.,
            clipboard: reverse,
        },
    )
    .unwrap();
    assert_eq!(track(&roundtrip, 1, &body("peach")).keys, unweighted_keys);
    assert_eq!(
        roundtrip.skins, document.skins,
        "Pasting keys must not convert bindings"
    );
    let body_vertices = |skin| {
        checked_evaluate_scene(&roundtrip, Some(1), 0.3, skin, None)
            .unwrap()
            .attachments
            .into_iter()
            .find(|draw| draw.attachment == body(skin))
            .unwrap()
            .vertices
    };
    for (plain, weighted) in body_vertices("peach").iter().zip(body_vertices("mint")) {
        for axis in 0..2 {
            assert!((plain[axis] - weighted[axis]).abs() < 0.001);
        }
    }
    assert!(document.animations[1].deform_tracks.is_empty());
}

#[test]
fn deform_timeline_linked_owners_are_canonical_and_independent_links_keep_separate_tracks() {
    let document = linked_lab();
    let glove = reference("default", "hand.R", "glove");
    let mitt = reference("default", "hand.R", "mitt");
    assert_eq!(deform_key_owner(&document, &glove).unwrap(), owner(&hand()));
    assert_eq!(deform_key_owner(&document, &mitt).unwrap(), owner(&mitt));
    assert!(
        copy_animation_keys(
            &document,
            0,
            &[identity(&glove, 0.6), identity(&hand(), 0.6)]
        )
        .is_err()
    );
    let clipboard = copy_animation_keys(&document, 0, &[identity(&glove, 0.6)]).unwrap();
    let moved = apply_command(
        &document,
        Command::MoveAnimationKeys {
            animation: 0,
            keys: vec![identity(&glove, 0.6)],
            delta: 1.,
            copy: false,
        },
    )
    .unwrap();
    assert!(track(&moved, 0, &hand()).keys.iter().any(|k| k.time == 1.6));
    assert!(
        !moved.animations[0]
            .deform_tracks
            .iter()
            .any(|t| t.attachment == glove)
    );
    let pasted = apply_command(
        &document,
        Command::Batch {
            commands: vec![
                Command::PasteAnimationKeys {
                    animation: 1,
                    target: owner(&glove),
                    time: 0.,
                    clipboard: clipboard.clone(),
                },
                Command::PasteAnimationKeys {
                    animation: 1,
                    target: owner(&mitt),
                    time: 1.,
                    clipboard,
                },
            ],
        },
    )
    .unwrap();
    assert_eq!(
        track(&pasted, 1, &hand()).keys[0].offsets,
        track(&pasted, 1, &mitt).keys[0].offsets
    );
    assert_eq!(pasted.animations[1].deform_tracks.len(), 2);
    assert_eq!(pasted.skins, document.skins);
    let missing_fallback = reference("peach", "hand.R", "hand.R");
    assert!(deform_key_owner(&document, &missing_fallback).is_err());
}

#[test]
fn deform_timeline_curve_edits_are_exact_and_change_geometry_without_modifying_dense_data() {
    let document = lab();
    for attachment in [hand(), body("peach")] {
        let before_track = track(&document, 0, &attachment).clone();
        let time = before_track.keys[0].time;
        let edited = apply_command(
            &document,
            Command::SetDeformCurve {
                animation: 0,
                attachment: attachment.clone(),
                time,
                curve: Curve::Stepped,
            },
        )
        .unwrap();
        assert_eq!(edited.skins, document.skins);
        assert_eq!(edited.animations[0].tracks, document.animations[0].tracks);
        let mut expected = before_track;
        expected.keys[0].curve = Curve::Stepped;
        assert_eq!(track(&edited, 0, &attachment), &expected);
        let skin = &attachment.skin;
        let before_scene = checked_evaluate_scene(&document, Some(0), 0.3, skin, None).unwrap();
        let edited_scene = checked_evaluate_scene(&edited, Some(0), 0.3, skin, None).unwrap();
        assert_eq!(
            before_scene
                .world
                .iter()
                .map(|m| [m.a, m.b, m.c, m.d, m.x, m.y])
                .collect::<Vec<_>>(),
            edited_scene
                .world
                .iter()
                .map(|m| [m.a, m.b, m.c, m.d, m.x, m.y])
                .collect::<Vec<_>>(),
        );
        let vertices = |scene: &Scene| {
            scene
                .attachments
                .iter()
                .find(|a| a.attachment == attachment)
                .unwrap()
                .vertices
                .clone()
        };
        assert_ne!(vertices(&before_scene), vertices(&edited_scene));
        let close = f32::from_bits(0.6_f32.to_bits() + 1);
        assert!(
            apply_command(
                &document,
                Command::SetDeformCurve {
                    animation: 0,
                    attachment: attachment.clone(),
                    time: close,
                    curve: Curve::Linear,
                }
            )
            .unwrap_err()
            .contains("exact")
        );
        let mut session = Session::new(document.clone()).unwrap();
        session
            .dispatch(Command::SetDeformCurve {
                animation: 0,
                attachment: attachment.clone(),
                time,
                curve: Curve::Stepped,
            })
            .unwrap();
        session.undo();
        assert_eq!(session.document(), &document);
        session.redo();
        assert_eq!(session.document(), &edited);
    }
}

#[test]
fn deform_timeline_also_edits_imported_path_and_clipping_tracks_without_mesh_conversion() {
    for document in [
        serde_json::from_str::<Document>(include_str!(
            "../../../examples/peach-bot/path-bot.peachy"
        ))
        .unwrap(),
        serde_json::from_str::<Document>(include_str!(
            "../../../examples/peach-bot/clip-bot.peachy"
        ))
        .unwrap(),
    ] {
        let original = document.animations[0].deform_tracks[0].clone();
        let attachment = original.attachment.clone();
        assert_eq!(
            deform_key_owner(&document, &attachment).unwrap(),
            owner(&attachment)
        );
        let clipboard = copy_animation_keys(
            &document,
            0,
            &[identity(&attachment, original.keys[0].time)],
        )
        .unwrap();
        let edited = apply_command(
            &document,
            Command::Batch {
                commands: vec![
                    Command::SetDeformCurve {
                        animation: 0,
                        attachment: attachment.clone(),
                        time: original.keys[0].time,
                        curve: Curve::Stepped,
                    },
                    Command::PasteAnimationKeys {
                        animation: 1,
                        target: owner(&attachment),
                        time: 4.,
                        clipboard,
                    },
                ],
            },
        )
        .unwrap();
        assert_eq!(edited.skins, document.skins);
        assert_eq!(
            track(&edited, 1, &attachment).keys[0].offsets,
            original.keys[0].offsets
        );
        assert_eq!(track(&edited, 1, &attachment).keys[0].time, 4.);
        assert_eq!(track(&edited, 0, &attachment).keys[0].curve, Curve::Stepped);
    }
}

#[test]
fn deform_timeline_invalid_payloads_fail_atomically_and_preserve_redo() {
    let document = lab();
    let mut session = Session::new(document.clone()).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "changed".into(),
        })
        .unwrap();
    session.undo();
    assert!(session.can_redo());
    let wrong = reference("missing", "body", "body");
    for command in [
        Command::MoveAnimationKeys {
            animation: 0,
            keys: vec![identity(&hand(), 0.6)],
            delta: -1.,
            copy: false,
        },
        Command::MoveAnimationKeys {
            animation: 0,
            keys: vec![identity(&hand(), 0.6), identity(&hand(), 1.2)],
            delta: 1e20,
            copy: false,
        },
        Command::MoveAnimationKeys {
            animation: 0,
            keys: vec![identity(&hand(), 0.6)],
            delta: f32::INFINITY,
            copy: true,
        },
        Command::DeleteAnimationKeys {
            animation: 0,
            keys: vec![identity(&hand(), 0.6), identity(&hand(), 0.6)],
        },
        Command::DeleteAnimationKeys {
            animation: 0,
            keys: vec![identity(&wrong, 0.6)],
        },
        Command::DeleteAnimationKeys {
            animation: 0,
            keys: vec![identity(&hand(), 0.7)],
        },
        Command::SetDeformCurve {
            animation: 0,
            attachment: hand(),
            time: 0.,
            curve: Curve::Bezier([f32::NAN, 0., 1., 1.]),
        },
    ] {
        assert!(session.dispatch(command).is_err());
        assert_eq!(session.document(), &document);
        assert!(session.can_redo());
        assert!(!session.can_undo());
    }
    let good = copy_animation_keys(&document, 0, &[identity(&body("peach"), 0.6)]).unwrap();
    let AnimationKeyClipboard::Deform { layout, keys } = good else {
        panic!()
    };
    let mut malformed = keys.clone();
    malformed[0].offsets.pop();
    let mut nonfinite = keys.clone();
    nonfinite[0].offsets[0] = f32::NAN;
    for clipboard in [
        AnimationKeyClipboard::Deform {
            layout: layout.clone(),
            keys: malformed,
        },
        AnimationKeyClipboard::Deform {
            layout: layout.clone(),
            keys: nonfinite,
        },
        AnimationKeyClipboard::Deform {
            layout: layout.clone(),
            keys: vec![keys[0].clone(), keys[0].clone()],
        },
        AnimationKeyClipboard::Deform {
            layout: DeformLayout::Unweighted {
                vertices: usize::MAX,
            },
            keys,
        },
    ] {
        assert!(
            session
                .dispatch(Command::PasteAnimationKeys {
                    animation: 0,
                    target: owner(&body("peach")),
                    time: 0.,
                    clipboard,
                })
                .is_err()
        );
        assert_eq!(session.document(), &document);
        assert!(session.can_redo());
    }
}

#[test]
fn deform_timeline_wire_payloads_reject_unknown_fields_and_keep_exact_references() {
    let key = identity(&hand(), 0.6);
    let command = Command::SetDeformCurve {
        animation: 0,
        attachment: hand(),
        time: 0.6,
        curve: Curve::Stepped,
    };
    let wire = serde_json::to_value(&command).unwrap();
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<Command>(wire.clone()).unwrap()).unwrap(),
        wire
    );
    for mut value in [
        serde_json::json!({"type":"deform","attachment":{"skin":"default","slot":"hand.R","placeholder":"hand.R","extra":true},"time":0.6}),
        serde_json::json!({"type":"deform","attachment":{"skin":"default","slot":"hand.R","placeholder":"hand.R"},"time":0.6,"bone":0}),
    ] {
        assert!(serde_json::from_value::<AnimationKey>(value.clone()).is_err());
        value.as_object_mut().unwrap().remove("time");
        assert!(serde_json::from_value::<AnimationKey>(value).is_err());
    }
    let mut unknown = wire;
    unknown["offsets"] = serde_json::json!([0., 0.]);
    assert!(serde_json::from_value::<Command>(unknown).is_err());
    let mut bad_layout = serde_json::to_value(AnimationKeyClipboard::Deform {
        layout: DeformLayout::Unweighted { vertices: 5 },
        keys: vec![DeformKey {
            time: 0.,
            curve: Curve::Linear,
            offsets: vec![0.; 10],
        }],
    })
    .unwrap();
    bad_layout["layout"]["influence_counts"] = serde_json::json!([1, 1, 1, 1, 1]);
    assert!(serde_json::from_value::<AnimationKeyClipboard>(bad_layout).is_err());
    assert_eq!(
        serde_json::from_value::<AnimationKey>(serde_json::to_value(&key).unwrap()).unwrap(),
        key
    );
}
