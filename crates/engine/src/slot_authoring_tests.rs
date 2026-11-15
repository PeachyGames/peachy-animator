//! Slot authoring must preserve sampled precision and share pending/history,
//! while persistent keys retain their exact identities and existing RGBA curves.
use crate::*;
use peachy_document::*;

fn lab() -> Document {
    crate::native_fixture::decode(include_str!("../../../examples/peach-bot/slot-bot.peachy"))
        .unwrap()
}
fn slot(doc: &Document, name: &str) -> usize {
    doc.slots.iter().position(|slot| slot.name == name).unwrap()
}
fn target(animation: usize, time: f32) -> EditTarget {
    EditTarget::Animate { animation, time }
}
fn color_key(time: f32, color: [u8; 4], curves: [Curve; 4]) -> ColorKey {
    ColorKey {
        time,
        color,
        curves,
    }
}
fn color_track(doc: &Document, animation: usize, slot: usize) -> &ColorTrack {
    doc.animations[animation]
        .color_tracks
        .iter()
        .find(|track| track.slot == slot)
        .unwrap()
}
fn attachment_track(doc: &Document, animation: usize, slot: usize) -> &AttachmentTrack {
    doc.animations[animation]
        .attachment_tracks
        .iter()
        .find(|track| track.slot == slot)
        .unwrap()
}
fn different_curves() -> [Curve; 4] {
    [
        Curve::Bezier([0.2, 0.1, 0.8, 0.9]),
        Curve::Linear,
        Curve::Bezier([0.25, -0.2, 0.75, 1.2]),
        Curve::Bezier([0.4, 0.2, 0.6, 0.8]),
    ]
}

#[test]
fn slot_authoring_reading_sampled_float_and_hidden_state_never_quantizes_or_rewrites() {
    let mut document = lab();
    let body = slot(&document, "body");
    let hand = slot(&document, "hand.L");
    let attachment = AttachmentRef {
        skin: "peach".into(),
        slot: "body".into(),
        placeholder: "body".into(),
    };
    let Attachment::Region(region) = document.attachment_mut(&attachment).unwrap() else {
        panic!()
    };
    region.color = [128, 220, 180, 128];
    let mut session = Session::new(document.clone()).unwrap();
    session.set_skin("peach");
    session
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "temporary".into(),
        })
        .unwrap();
    session.undo();
    assert!(session.can_redo());
    let current = session.slot_pose(body, target(0, 0.9)).unwrap();
    let display = slot_color_bytes(current.color).unwrap();
    assert!(
        current
            .color
            .iter()
            .zip(display)
            .any(|(&raw, byte)| raw != f32::from(byte) / 255.)
    );
    assert_eq!(
        session.slot_pose(hand, target(0, 1.2)).unwrap().attachment,
        None
    );
    for _ in 0..3 {
        assert_eq!(session.slot_pose(body, target(0, 0.9)).unwrap(), current);
        assert_eq!(
            slot_key_time(session.document(), 0, 0.9, body, SlotProperty::Color).unwrap(),
            None
        );
    }
    let scene = checked_scene_from_authored(
        session.document(),
        "peach",
        &session.authored_pose(target(0, 0.9)).unwrap(),
    )
    .unwrap();
    assert_ne!(
        scene
            .attachments
            .iter()
            .find(|draw| draw.slot == body)
            .unwrap()
            .color,
        display
    );
    assert_eq!(session.document(), &document);
    assert!(session.pose_edits().is_empty());
    assert!(!session.is_dirty());
    assert!(session.can_redo());
    assert!(!session.can_undo());
}

#[test]
fn slot_authoring_pending_swaps_and_float_color_are_undoable_but_excluded_from_saved_document() {
    let document = lab();
    let hand = slot(&document, "hand.L");
    let mut session = Session::new(document.clone()).unwrap();
    session.set_skin("peach");
    let time = target(0, 0.6);
    let original = session.slot_pose(hand, time).unwrap();
    assert_eq!(original.attachment.as_deref(), Some("fist"));
    let color = [0.123456, 0.67, 0.42, 0.333333];
    session
        .edit_slot_pose(
            hand,
            time,
            &[
                SlotPoseEdit::Attachment {
                    attachment: Some("hand.L".into()),
                },
                SlotPoseEdit::Color { color },
            ],
            false,
        )
        .unwrap();
    let pending = session.slot_pose(hand, time).unwrap();
    assert_eq!(pending.color, color);
    assert_eq!(pending.attachment.as_deref(), Some("hand.L"));
    for property in [SlotProperty::Attachment, SlotProperty::Color] {
        assert_eq!(
            session
                .pose_edits()
                .slot_key_state(session.document(), 0, 0.6, hand, property)
                .unwrap(),
            KeyState::Edited
        );
    }
    assert_eq!(session.document(), &document);
    assert!(!session.is_dirty());
    assert!(session.can_undo());
    assert_eq!(
        serde_json::to_value(session.document()).unwrap(),
        serde_json::to_value(&document).unwrap()
    );
    let scene = checked_scene_from_authored(
        session.document(),
        "peach",
        &session.authored_pose(time).unwrap(),
    )
    .unwrap();
    assert_eq!(
        scene
            .attachments
            .iter()
            .find(|draw| draw.slot == hand)
            .unwrap()
            .attachment
            .placeholder,
        "hand.L"
    );
    session.undo();
    assert_eq!(session.slot_pose(hand, time).unwrap(), original);
    session.redo();
    assert_eq!(session.slot_pose(hand, time).unwrap(), pending);
    session
        .edit_slot_pose(
            hand,
            time,
            &[
                SlotPoseEdit::Attachment {
                    attachment: original.attachment,
                },
                SlotPoseEdit::Color {
                    color: original.color,
                },
            ],
            false,
        )
        .unwrap();
    assert!(session.pose_edits().is_empty());
    assert!(!session.is_dirty());
    assert_eq!(
        session.slot_pose(hand, EditTarget::Setup).unwrap().color,
        [1.; 4]
    );
}

#[test]
fn slot_authoring_explicit_keys_preserve_prefirst_setup_holds_and_null_visibility() {
    let mut document = lab();
    let hand = slot(&document, "hand.L");
    let body = slot(&document, "body");
    document.animations[0].attachment_tracks.clear();
    document.animations[0].color_tracks.clear();
    let mut commands = vec![];
    for (time, attachment) in [
        (0.4, Some("fist".into())),
        (1., None),
        (1.6, Some("hand.L".into())),
    ] {
        commands.push(Command::SetSlotAttachmentKey {
            animation: 0,
            slot: hand,
            key: AttachmentKey { time, attachment },
        });
    }
    commands.extend([
        Command::SetSlotColorKey {
            animation: 0,
            slot: body,
            key: color_key(0.3, [200, 100, 50, 255], [Curve::Stepped; 4]),
        },
        Command::SetSlotColorKey {
            animation: 0,
            slot: body,
            key: color_key(1.5, [100, 150, 200, 0], [Curve::Linear; 4]),
        },
    ]);
    let authored = apply_command(&document, Command::Batch { commands }).unwrap();
    let session = Session::new(authored.clone()).unwrap();
    assert_eq!(
        session.slot_pose(hand, target(0, 0.1)).unwrap().attachment,
        document.slots[hand].attachment
    );
    assert_eq!(
        session
            .slot_pose(hand, target(0, 0.4))
            .unwrap()
            .attachment
            .as_deref(),
        Some("fist")
    );
    assert_eq!(
        session.slot_pose(hand, target(0, 1.)).unwrap().attachment,
        None
    );
    assert_eq!(
        session
            .slot_pose(hand, target(0, 9.))
            .unwrap()
            .attachment
            .as_deref(),
        Some("hand.L")
    );
    assert_eq!(
        slot_color_bytes(session.slot_pose(body, target(0, 0.1)).unwrap().color).unwrap(),
        document.slots[body].color
    );
    assert_eq!(
        slot_color_bytes(session.slot_pose(body, target(0, 0.9)).unwrap().color).unwrap(),
        [200, 100, 50, 255]
    );
    assert_eq!(
        slot_color_bytes(session.slot_pose(body, target(0, 9.)).unwrap().color).unwrap(),
        [100, 150, 200, 0]
    );
    assert_eq!(authored.skins, document.skins);
    assert_eq!(authored.slots, document.slots);
}

#[test]
fn slot_authoring_autokey_and_exact_curve_edits_preserve_four_channel_curves_and_fractional_identities()
 {
    let mut document = lab();
    let body = slot(&document, "body");
    let curves = different_curves();
    let first = 0.25_f32;
    let close = f32::from_bits(first.to_bits() + 1);
    document.animations[0]
        .color_tracks
        .iter_mut()
        .find(|track| track.slot == body)
        .unwrap()
        .keys = vec![
        color_key(first, [100, 110, 120, 130], [Curve::Linear; 4]),
        color_key(close, [200, 210, 220, 230], curves),
    ];
    let mut session = Session::new(document.clone()).unwrap();
    let bytes = [91, 122, 153, 184];
    let color = bytes.map(|v| f32::from(v) / 255.);
    session
        .edit_slot_pose(
            body,
            target(0, close),
            &[SlotPoseEdit::Color { color }],
            true,
        )
        .unwrap();
    let keys = &color_track(session.document(), 0, body).keys;
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0], color_track(&document, 0, body).keys[0]);
    assert_eq!(keys[1].time, close);
    assert_eq!(keys[1].color, bytes);
    assert_eq!(keys[1].curves, curves);
    assert!(session.pose_edits().is_empty());
    assert_eq!(
        slot_key_time(session.document(), 0, close, body, SlotProperty::Color).unwrap(),
        Some(close)
    );
    assert!(
        slot_key_time(
            session.document(),
            0,
            first - 0.000001,
            body,
            SlotProperty::Color
        )
        .is_err()
    );
    let before = session.document().clone();
    session
        .dispatch(Command::SetSlotColorCurves {
            animation: 0,
            slot: body,
            time: close,
            curves: [Curve::Stepped; 4],
        })
        .unwrap();
    assert_eq!(
        color_track(session.document(), 0, body).keys[1].color,
        bytes
    );
    assert_eq!(
        color_track(session.document(), 0, body).keys[1].curves,
        [Curve::Stepped; 4]
    );
    session.undo();
    assert_eq!(session.document(), &before);
    assert!(
        session
            .dispatch(Command::SetSlotColorCurves {
                animation: 0,
                slot: body,
                time: close.next_up(),
                curves
            })
            .is_err()
    );
    assert_eq!(session.document(), &before);
    assert!(session.can_redo());
    session.undo();
    assert_eq!(session.document(), &document);
}

#[test]
fn slot_authoring_slot_clipboard_and_exact_timeline_keep_coincident_property_kinds_separate() {
    let mut document = lab();
    let hand = slot(&document, "hand.L");
    document.animations[0].attachment_tracks.clear();
    document.animations[0].color_tracks.clear();
    let curves = different_curves();
    document = apply_command(
        &document,
        Command::Batch {
            commands: vec![
                Command::SetSlotAttachmentKey {
                    animation: 0,
                    slot: hand,
                    key: AttachmentKey {
                        time: 0.5,
                        attachment: Some("fist".into()),
                    },
                },
                Command::SetSlotAttachmentKey {
                    animation: 0,
                    slot: hand,
                    key: AttachmentKey {
                        time: 1.,
                        attachment: None,
                    },
                },
                Command::SetSlotColorKey {
                    animation: 0,
                    slot: hand,
                    key: color_key(0.5, [10, 20, 30, 40], curves),
                },
                Command::SetSlotColorKey {
                    animation: 0,
                    slot: hand,
                    key: color_key(2., [50, 60, 70, 80], [Curve::Linear; 4]),
                },
            ],
        },
    )
    .unwrap();
    let selected = [
        AnimationKey::SlotColor {
            slot: hand,
            time: 2.,
        },
        AnimationKey::SlotAttachment {
            slot: hand,
            time: 0.5,
        },
        AnimationKey::SlotColor {
            slot: hand,
            time: 0.5,
        },
        AnimationKey::SlotAttachment {
            slot: hand,
            time: 1.,
        },
    ];
    let clipboard = copy_animation_keys(&document, 0, &selected).unwrap();
    let AnimationKeyClipboard::Slot {
        attachment_keys,
        color_keys,
    } = &clipboard
    else {
        panic!()
    };
    assert_eq!(
        attachment_keys.iter().map(|k| k.time).collect::<Vec<_>>(),
        vec![0., 0.5]
    );
    assert_eq!(
        color_keys.iter().map(|k| k.time).collect::<Vec<_>>(),
        vec![0., 1.5]
    );
    assert_eq!(color_keys[0].curves, curves);
    let pasted = apply_command(
        &document,
        Command::PasteAnimationKeys {
            animation: 1,
            target: KeyOwner::Slot { slot: hand },
            time: 3.,
            clipboard,
        },
    )
    .unwrap();
    assert_eq!(pasted.animations[1].duration(), 4.5);
    let both = vec![
        AnimationKey::SlotAttachment {
            slot: hand,
            time: 3.,
        },
        AnimationKey::SlotColor {
            slot: hand,
            time: 3.,
        },
    ];
    let moved = apply_command(
        &pasted,
        Command::MoveAnimationKeys {
            animation: 1,
            keys: both,
            delta: 0.25,
            copy: false,
        },
    )
    .unwrap();
    assert_eq!(attachment_track(&moved, 1, hand).keys[0].time, 3.25);
    assert_eq!(color_track(&moved, 1, hand).keys[0].time, 3.25);
    assert_eq!(color_track(&moved, 1, hand).keys[0].curves, curves);
    let deleted = apply_command(
        &moved,
        Command::DeleteAnimationKeys {
            animation: 1,
            keys: vec![AnimationKey::SlotAttachment {
                slot: hand,
                time: 3.25,
            }],
        },
    )
    .unwrap();
    assert_eq!(attachment_track(&deleted, 1, hand).keys.len(), 1);
    assert_eq!(
        color_track(&deleted, 1, hand).keys,
        color_track(&moved, 1, hand).keys
    );
    assert_eq!(deleted.skins, document.skins);
    let other_slot = slot(&document, "body");
    let with_other = apply_command(
        &document,
        Command::SetSlotColorKey {
            animation: 0,
            slot: other_slot,
            key: color_key(0.3, [255; 4], [Curve::Linear; 4]),
        },
    )
    .unwrap();
    assert!(
        copy_animation_keys(
            &with_other,
            0,
            &[
                selected[0].clone(),
                AnimationKey::SlotColor {
                    slot: other_slot,
                    time: 0.3
                }
            ]
        )
        .unwrap_err()
        .contains("one typed owner")
    );
}

#[test]
fn slot_authoring_color_only_clipboard_crosses_slots_without_touching_attachment_state() {
    let document = lab();
    let source = slot(&document, "body");
    let destination = slot(&document, "head");
    let clipboard = copy_animation_keys(
        &document,
        0,
        &[AnimationKey::SlotColor {
            slot: source,
            time: 0.3,
        }],
    )
    .unwrap();
    let pasted = apply_command(
        &document,
        Command::PasteAnimationKeys {
            animation: 1,
            target: KeyOwner::Slot { slot: destination },
            time: 0.,
            clipboard,
        },
    )
    .unwrap();
    assert_eq!(
        color_track(&pasted, 1, destination).keys[0].color,
        color_track(&document, 0, source).keys[0].color
    );
    assert_eq!(
        pasted.animations[1].attachment_tracks,
        document.animations[1].attachment_tracks
    );
    assert_eq!(pasted.slots, document.slots);
    assert_eq!(pasted.skins, document.skins);
}

#[test]
fn slot_authoring_paste_skips_unavailable_named_attachments_without_changing_history() {
    let document = lab();
    let source = slot(&document, "hand.L");
    let destination = slot(&document, "hand.R");
    let clipboard = copy_animation_keys(
        &document,
        0,
        &[AnimationKey::SlotAttachment {
            slot: source,
            time: 1.6,
        }],
    )
    .unwrap();
    assert_eq!(
        attachment_track(&document, 0, source)
            .keys
            .last()
            .unwrap()
            .attachment
            .as_deref(),
        Some("hand.L")
    );
    let mut session = Session::new(document.clone()).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "redo-must-survive".into(),
        })
        .unwrap();
    session.undo();
    session
        .dispatch(Command::PasteAnimationKeys {
            animation: 1,
            target: KeyOwner::Slot { slot: destination },
            time: 2.2,
            clipboard,
        })
        .unwrap();
    assert_eq!(session.document(), &document);
    assert!(!session.can_undo());
    assert!(session.can_redo());
    session.redo();
    assert_eq!(session.document().animations[1].name, "redo-must-survive");
}

#[test]
fn slot_authoring_pasted_identities_exclude_skipped_existing_keys_and_keep_valid_same_value_overwrites()
 {
    let mut document = lab();
    let destination = slot(&document, "hand.R");
    document = apply_command(
        &document,
        Command::Batch {
            commands: vec![
                Command::SetSlotAttachmentKey {
                    animation: 1,
                    slot: destination,
                    key: AttachmentKey {
                        time: 2.,
                        attachment: Some("hand.R".into()),
                    },
                },
                Command::SetSlotColorKey {
                    animation: 1,
                    slot: destination,
                    key: color_key(2., [1, 2, 3, 4], different_curves()),
                },
            ],
        },
    )
    .unwrap();
    let clipboard = AnimationKeyClipboard::Slot {
        attachment_keys: vec![AttachmentKey {
            time: 0.,
            attachment: Some("hand.L".into()),
        }],
        color_keys: vec![color_key(0., [1, 2, 3, 4], different_curves())],
    };
    let owner = KeyOwner::Slot { slot: destination };
    let accepted = pasted_animation_keys(&document, 1, owner.clone(), 2., &clipboard).unwrap();
    assert_eq!(
        accepted,
        vec![AnimationKey::SlotColor {
            slot: destination,
            time: 2.
        }]
    );
    let all_skipped = AnimationKeyClipboard::Slot {
        attachment_keys: vec![AttachmentKey {
            time: 0.,
            attachment: Some("hand.L".into()),
        }],
        color_keys: vec![],
    };
    assert!(
        pasted_animation_keys(&document, 1, owner.clone(), 2., &all_skipped)
            .unwrap()
            .is_empty()
    );
    let pasted = apply_command(
        &document,
        Command::PasteAnimationKeys {
            animation: 1,
            target: owner,
            time: 2.,
            clipboard,
        },
    )
    .unwrap();
    assert_eq!(
        pasted, document,
        "An accepted same-value overwrite still has an incoming identity"
    );
    let deleted = apply_command(
        &pasted,
        Command::DeleteAnimationKeys {
            animation: 1,
            keys: accepted,
        },
    )
    .unwrap();
    assert_eq!(
        deleted.animations[1].attachment_tracks,
        document.animations[1].attachment_tracks
    );
    assert!(
        deleted.animations[1]
            .color_tracks
            .iter()
            .all(|track| track.slot != destination)
    );
}

#[test]
fn slot_authoring_paste_keeps_shared_origin_color_hide_and_any_skin_known_names() {
    let mut document = lab();
    let destination = slot(&document, "hand.R");
    let attachment = document.skins["default"]["hand.R"]["hand.R"].clone();
    document
        .skins
        .get_mut("peach")
        .unwrap()
        .entry("hand.R".into())
        .or_default()
        .insert("only-peach".into(), attachment);
    let clipboard = AnimationKeyClipboard::Slot {
        attachment_keys: vec![
            AttachmentKey {
                time: 0.,
                attachment: Some("hand.L".into()),
            },
            AttachmentKey {
                time: 0.25,
                attachment: None,
            },
            AttachmentKey {
                time: 0.5,
                attachment: Some("only-peach".into()),
            },
        ],
        color_keys: vec![color_key(0.125, [1, 2, 3, 4], different_curves())],
    };
    let pasted = apply_command(
        &document,
        Command::PasteAnimationKeys {
            animation: 1,
            target: KeyOwner::Slot { slot: destination },
            time: 2.,
            clipboard,
        },
    )
    .unwrap();
    assert_eq!(
        attachment_track(&pasted, 1, destination).keys,
        vec![
            AttachmentKey {
                time: 2.25,
                attachment: None
            },
            AttachmentKey {
                time: 2.5,
                attachment: Some("only-peach".into())
            },
        ]
    );
    assert_eq!(
        color_track(&pasted, 1, destination).keys,
        vec![color_key(2.125, [1, 2, 3, 4], different_curves())]
    );
    assert_eq!(pasted.slots, document.slots);
    assert_eq!(pasted.skins, document.skins);

    // Explicit authoring remains a data operation: a currently unavailable name
    // is preserved so a later skin/attachment edit can resolve it.
    let authored = apply_command(
        &document,
        Command::SetSlotAttachmentKey {
            animation: 1,
            slot: destination,
            key: AttachmentKey {
                time: 2.,
                attachment: Some("unavailable".into()),
            },
        },
    )
    .unwrap();
    assert_eq!(
        attachment_track(&authored, 1, destination).keys[0]
            .attachment
            .as_deref(),
        Some("unavailable")
    );
}

#[test]
fn slot_authoring_fixed_baseline_preview_cancel_and_setup_edits_share_one_history() {
    let document = lab();
    let body = slot(&document, "body");
    let hand = slot(&document, "hand.L");
    let mut session = Session::new(document.clone()).unwrap();
    let edit_target = target(0, 0.6);
    session
        .edit_slot_pose(
            hand,
            edit_target,
            &[SlotPoseEdit::Attachment { attachment: None }],
            false,
        )
        .unwrap();
    let baseline = session.pose_edits().clone();
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
        .preview_edit_slot_pose(
            body,
            edit_target,
            &[SlotPoseEdit::Color {
                color: [0.1, 0.2, 0.3, 0.4],
            }],
            false,
        )
        .unwrap();
    let preview = session.pose_edits().clone();
    assert!(
        session
            .preview_edit_slot_pose(
                body,
                edit_target,
                &[SlotPoseEdit::Color {
                    color: [f32::NAN; 4]
                }],
                false
            )
            .is_err()
    );
    assert_eq!(session.pose_edits(), &preview);
    session
        .preview_edit_slot_pose(
            body,
            edit_target,
            &[SlotPoseEdit::Color {
                color: [0.7, 0.6, 0.5, 0.4],
            }],
            false,
        )
        .unwrap();
    assert_eq!(
        session.slot_pose(body, edit_target).unwrap().color,
        [0.7, 0.6, 0.5, 0.4]
    );
    session.cancel_transaction();
    assert_eq!(session.pose_edits(), &baseline);
    assert!(session.can_redo());
    session.begin_transaction();
    session
        .preview_edit_slot_pose(
            body,
            edit_target,
            &[SlotPoseEdit::Color {
                color: [0.7, 0.6, 0.5, 0.4],
            }],
            false,
        )
        .unwrap();
    session.commit_transaction();
    let committed = session.pose_edits().clone();
    session.undo();
    assert_eq!(session.pose_edits(), &baseline);
    session.redo();
    assert_eq!(session.pose_edits(), &committed);
    session
        .edit_slot_pose(
            body,
            EditTarget::Setup,
            &[SlotPoseEdit::Color {
                color: [0.5, 0.25, 0.75, 1.],
            }],
            false,
        )
        .unwrap();
    assert_eq!(session.document().slots[body].color, [128, 64, 191, 255]);
    assert_eq!(session.pose_edits(), &committed);
    session.undo();
    assert_eq!(session.document(), &document);
    assert_eq!(session.pose_edits(), &committed);
}

#[test]
fn slot_authoring_unrepresentable_fractional_keying_and_bad_payloads_preserve_pending_and_redo() {
    let document = lab();
    let hand = slot(&document, "hand.L");
    let body = slot(&document, "body");
    let mut session = Session::new(document.clone()).unwrap();
    let edit_target = target(1, 0.02);
    session
        .edit_slot_pose(
            hand,
            edit_target,
            &[SlotPoseEdit::Attachment { attachment: None }],
            false,
        )
        .unwrap();
    let pending = session.pose_edits().clone();
    assert!(
        session
            .key_slot_pose(1, 0.02, hand, &[SlotProperty::Attachment])
            .unwrap_err()
            .contains("current pose")
    );
    assert_eq!(session.document(), &document);
    assert_eq!(session.pose_edits(), &pending);
    session
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "temporary".into(),
        })
        .unwrap();
    session.undo();
    assert!(session.can_redo());
    for edits in [
        vec![SlotPoseEdit::Color {
            color: [f32::INFINITY; 4],
        }],
        vec![SlotPoseEdit::Attachment {
            attachment: Some(" ".into()),
        }],
        vec![
            SlotPoseEdit::Color { color: [0.; 4] },
            SlotPoseEdit::Color { color: [1.; 4] },
        ],
    ] {
        assert!(
            session
                .edit_slot_pose(body, edit_target, &edits, false)
                .is_err()
        );
        assert_eq!(session.document(), &document);
        assert_eq!(session.pose_edits(), &pending);
        assert!(session.can_redo());
    }
    let invalid = Command::SetSlotColorKey {
        animation: 0,
        slot: body,
        key: color_key(
            0.5,
            [255; 4],
            [Curve::Stepped, Curve::Linear, Curve::Linear, Curve::Linear],
        ),
    };
    assert!(session.dispatch(invalid).is_err());
    assert_eq!(session.document(), &document);
    assert!(session.can_redo());
    assert!(session.slot_pose(usize::MAX, edit_target).is_err());
    assert!(slot_color_bytes([f32::NAN; 4]).is_err());
}

#[test]
fn slot_authoring_global_k_keys_bone_ik_deform_and_slots_in_one_undo_step() {
    let document: Document =
        crate::native_fixture::decode(include_str!("../../../examples/peach-bot/ik-bot.peachy"))
            .unwrap();
    let attachment = AttachmentRef {
        skin: "peach".into(),
        slot: "body".into(),
        placeholder: "body".into(),
    };
    assert!(matches!(
        document.attachment(&attachment),
        Some(Attachment::Mesh(_))
    ));
    let body = slot(&document, "body");
    let mut session = Session::new(document.clone()).unwrap();
    session.set_skin("peach");
    let edit_target = target(0, 0.);
    let mut transform = session.local_pose(Some(0), 0.)[0];
    transform.x += 3.;
    session
        .edit_pose(0, edit_target, transform, &[Property::X], false)
        .unwrap();
    session
        .edit_ik_pose(0, edit_target, &[IkPoseEdit::Mix { value: 0.4 }], false)
        .unwrap();
    let mut offsets = session.deform_offsets(&attachment, edit_target).unwrap();
    offsets[0] += 3.;
    session
        .edit_deform_pose(attachment.clone(), edit_target, offsets.clone(), false)
        .unwrap();
    session
        .edit_slot_pose(
            body,
            edit_target,
            &[
                SlotPoseEdit::Attachment { attachment: None },
                SlotPoseEdit::Color {
                    color: [0.25, 0.5, 0.75, 1.],
                },
            ],
            false,
        )
        .unwrap();
    let pose = session.pose_edits().clone();
    assert_eq!(session.document(), &document);
    assert!(!pose.is_empty());
    session.key_edited(0, 0., None).unwrap();
    let keyed = session.document().clone();
    assert!(session.pose_edits().is_empty());
    assert!(session.is_dirty());
    assert_eq!(attachment_track(&keyed, 0, body).keys[0].attachment, None);
    assert_eq!(
        color_track(&keyed, 0, body).keys[0].color,
        [64, 128, 191, 255]
    );
    assert!(
        keyed.animations[0]
            .ik_tracks
            .iter()
            .any(|track| track.constraint == 0)
    );
    assert_eq!(
        keyed.animations[0]
            .deform_tracks
            .iter()
            .find(|track| track.attachment == attachment)
            .unwrap()
            .keys[0]
            .offsets,
        offsets
    );
    session.undo();
    assert_eq!(session.document(), &document);
    assert_eq!(session.pose_edits(), &pose);
    session.redo();
    assert_eq!(session.document(), &keyed);
    assert!(session.pose_edits().is_empty());
}

#[test]
fn slot_authoring_typed_wire_payloads_are_strict_and_malformed_clipboards_are_atomic() {
    let document = lab();
    let body = slot(&document, "body");
    let key = color_key(0., [100, 120, 140, 160], different_curves());
    let command = Command::SetSlotColorKey {
        animation: 1,
        slot: body,
        key: key.clone(),
    };
    let wire = serde_json::to_value(&command).unwrap();
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<Command>(wire.clone()).unwrap()).unwrap(),
        wire
    );
    let mut unknown = wire.clone();
    unknown["key"]["alpha"] = serde_json::json!(1.);
    assert!(serde_json::from_value::<Command>(unknown).is_err());
    let mut outside = wire;
    outside["key"]["color"] = serde_json::json!([0, 0, 0, 256]);
    assert!(serde_json::from_value::<Command>(outside).is_err());
    let clipboard = AnimationKeyClipboard::Slot {
        attachment_keys: vec![AttachmentKey {
            time: 0.,
            attachment: None,
        }],
        color_keys: vec![key.clone()],
    };
    let serialized = serde_json::to_value(&clipboard).unwrap();
    assert_eq!(
        serde_json::to_value(
            serde_json::from_value::<AnimationKeyClipboard>(serialized.clone()).unwrap()
        )
        .unwrap(),
        serialized
    );
    for clipboard in [
        AnimationKeyClipboard::Slot {
            attachment_keys: vec![AttachmentKey {
                time: 1.,
                attachment: None,
            }],
            color_keys: vec![],
        },
        AnimationKeyClipboard::Slot {
            attachment_keys: vec![],
            color_keys: vec![key.clone(), key.clone()],
        },
        AnimationKeyClipboard::Slot {
            attachment_keys: vec![AttachmentKey {
                time: 0.,
                attachment: Some("".into()),
            }],
            color_keys: vec![],
        },
    ] {
        assert!(
            apply_command(
                &document,
                Command::PasteAnimationKeys {
                    animation: 1,
                    target: KeyOwner::Slot { slot: body },
                    time: 0.,
                    clipboard
                }
            )
            .is_err()
        );
    }
    let invalid = serde_json::json!({"type":"slotColor","slot":body,"time":0.,"property":"alpha"});
    assert!(serde_json::from_value::<AnimationKey>(invalid).is_err());
    assert!(
        apply_command(
            &document,
            Command::PasteAnimationKeys {
                animation: 1,
                target: KeyOwner::Bone { bone: 0 },
                time: 0.,
                clipboard
            }
        )
        .is_err()
    );
}

#[test]
fn slot_authoring_pending_slot_index_survives_rename_and_skin_fallback_without_redirecting() {
    let document = lab();
    let hand = slot(&document, "hand.L");
    let mut session = Session::new(document.clone()).unwrap();
    let edit_target = target(0, 0.6);
    let pending = [
        SlotPoseEdit::Attachment {
            attachment: Some("hand.L".into()),
        },
        SlotPoseEdit::Color {
            color: [0.1, 0.2, 0.3, 0.4],
        },
    ];
    session
        .edit_slot_pose(hand, edit_target, &pending, false)
        .unwrap();
    let pose = session.slot_pose(hand, edit_target).unwrap();
    session
        .dispatch(Command::RenameSlot {
            slot: hand,
            name: "left-hand".into(),
        })
        .unwrap();
    assert_eq!(session.slot_pose(hand, edit_target).unwrap(), pose);
    session.set_skin("peach");
    let scene = checked_scene_from_authored(
        session.document(),
        "peach",
        &session.authored_pose(edit_target).unwrap(),
    )
    .unwrap();
    assert_eq!(
        scene
            .attachments
            .iter()
            .find(|draw| draw.slot == hand)
            .unwrap()
            .attachment
            .slot,
        "left-hand"
    );
    assert_eq!(
        session.document().slots[hand].attachment,
        document.slots[hand].attachment
    );
    session.set_skin("mint");
    assert_eq!(session.slot_pose(hand, edit_target).unwrap(), pose);
    session.undo();
    assert_eq!(session.document(), &document);
    assert_eq!(session.slot_pose(hand, edit_target).unwrap(), pose);
    session.discard_pose(false);
    assert!(session.pose_edits().is_empty());
    assert_eq!(
        session
            .slot_pose(hand, edit_target)
            .unwrap()
            .attachment
            .as_deref(),
        Some("fist")
    );
}
