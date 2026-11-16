//! Global order poses and typed timelines use the same checked engine boundary.
use crate::*;
use peachy_document::*;

fn fixture() -> Document {
    let mut doc = Document::new("Original pending order".into(), 60).unwrap();
    doc.slots = (0..4)
        .map(|slot| Slot {
            name: format!("slot{slot}"),
            bone: 0,
            attachment: None,
            color: WHITE,
            blend: BlendMode::Normal,
        })
        .collect();
    doc.animations.push(Animation::new("order", vec![]));
    doc.animations.push(Animation::new("target", vec![]));
    doc.validate().unwrap();
    doc
}
fn target(time: f32) -> EditTarget {
    EditTarget::Animate { animation: 0, time }
}
fn raw(time: f32, offsets: &[(usize, i32)]) -> Command {
    Command::SetDrawOrderKey {
        animation: 0,
        key: DrawOrderKey {
            time,
            offsets: offsets
                .iter()
                .map(|&(slot, offset)| DrawOrderOffset { slot, offset })
                .collect(),
        },
    }
}
fn identity(time: f32) -> AnimationKey {
    AnimationKey::DrawOrder { time }
}
fn root_rotation(time: f32) -> AnimationKey {
    AnimationKey::Bone {
        bone: 0,
        property: Property::Rotation,
        time,
    }
}
fn edit_root(session: &mut Session, time: f32) {
    let mut transform = session.authored_pose(target(time)).unwrap().bone_locals()[0];
    transform.rotation = 15.;
    session
        .edit_pose(0, target(time), transform, &[Property::Rotation], false)
        .unwrap();
}

#[test]
fn draw_order_pending_at_absent_and_existing_keys_stays_outside_saved_data() {
    let before = apply_command(&fixture(), raw(0.5, &[(0, 3)])).unwrap();
    for time in [0.25, 0.5] {
        let mut session = Session::new(before.clone()).unwrap();
        assert_eq!(
            session
                .pose_edits()
                .draw_order_key_state(&before, 0, time)
                .unwrap(),
            if time == 0.5 {
                KeyState::Keyed
            } else {
                KeyState::Absent
            }
        );
        session
            .edit_draw_order(target(time), vec![3, 2, 1, 0], false)
            .unwrap();
        assert_eq!(session.document(), &before);
        assert!(!session.is_dirty());
        assert_eq!(
            session.pose_edits().pending_draw_order(0),
            Some(&[3, 2, 1, 0][..])
        );
        assert_eq!(session.pose_edits().pending_draw_order(1), None);
        assert_eq!(
            session
                .pose_edits()
                .draw_order_key_state(&before, 0, time)
                .unwrap(),
            KeyState::Edited
        );
        assert_eq!(session.draw_order(target(time)).unwrap(), [3, 2, 1, 0]);
        assert_eq!(
            session.evaluated_pose(target(time)).unwrap().draw_order(),
            &[3, 2, 1, 0]
        );
        assert_eq!(session.draw_order(EditTarget::Setup).unwrap(), [0, 1, 2, 3]);
        let pending = session.pose_edits().clone();
        assert!(
            session
                .edit_draw_order(EditTarget::Setup, vec![0, 1, 2, 3], false)
                .is_err()
        );
        assert_eq!(session.pose_edits(), &pending);
        let saved = serde_json::to_string(session.document()).unwrap();
        let reopened = Session::new(serde_json::from_str(&saved).unwrap()).unwrap();
        assert_eq!(reopened.document(), &before);
        assert!(reopened.pose_edits().is_empty());
        session.key_draw_order(0, time).unwrap();
        assert!(!session.pose_edits().has_draw_order(0));
        assert_eq!(
            session
                .pose_edits()
                .draw_order_key_state(session.document(), 0, time)
                .unwrap(),
            KeyState::Keyed
        );
        assert_eq!(
            sample_authored_pose(session.document(), Some(0), time)
                .unwrap()
                .draw_order(),
            &[3, 2, 1, 0]
        );
    }
    let mut explicit_reset = Session::new(fixture()).unwrap();
    explicit_reset.key_draw_order(0, 0.25).unwrap();
    assert_eq!(
        explicit_reset.document().animations[0].draw_order,
        [DrawOrderKey {
            time: 0.25,
            offsets: vec![]
        }]
    );
}

#[test]
fn draw_order_direct_query_matches_authored_order_and_checked_context_without_pose_mutation() {
    let before = apply_command(
        &fixture(),
        Command::Batch {
            commands: vec![raw(0.5, &[(0, 3)]), raw(1.5, &[])],
        },
    )
    .unwrap();
    let mut session = Session::new(before.clone()).unwrap();
    for pending in [false, true] {
        if pending {
            session
                .edit_draw_order(target(0.25), vec![3, 2, 1, 0], false)
                .unwrap();
            edit_root(&mut session, 0.25);
        }
        let pose = session.pose_edits().clone();
        let history = (session.can_undo(), session.can_redo());
        for context in [
            EditTarget::Setup,
            target(0.),
            target(0.5),
            target(1.5),
            target(2.),
            EditTarget::Animate {
                animation: 1,
                time: 0.5,
            },
        ] {
            assert_eq!(
                session.draw_order(context).unwrap(),
                session.authored_pose(context).unwrap().draw_order()
            );
        }
        for context in [
            target(f32::NAN),
            target(f32::INFINITY),
            target(-1.),
            EditTarget::Animate {
                animation: 99,
                time: 0.5,
            },
            EditTarget::Animate {
                animation: 99,
                time: f32::NAN,
            },
        ] {
            assert_eq!(
                session.draw_order(context).unwrap_err(),
                session.authored_pose(context).unwrap_err()
            );
        }
        assert_eq!(session.document(), &before);
        assert_eq!(session.pose_edits(), &pose);
        assert_eq!((session.can_undo(), session.can_redo()), history);
    }
}

#[test]
fn draw_order_same_pose_interactive_edits_are_noops_even_with_auto_key_and_redo() {
    let mut session = Session::new(fixture()).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    for auto_key in [false, true] {
        session
            .edit_draw_order(target(0.25), vec![0, 1, 2, 3], auto_key)
            .unwrap();
        assert!(session.document().animations[0].draw_order.is_empty());
        assert!(!session.can_undo());
        assert!(session.can_redo());
    }
    session
        .edit_draw_order(target(0.25), vec![1, 2, 3, 0], false)
        .unwrap();
    edit_root(&mut session, 0.25);
    session.undo();
    let before = session.document().clone();
    let pending = session.pose_edits().clone();
    assert!(session.can_redo());
    session
        .edit_draw_order(target(0.25), vec![1, 2, 3, 0], true)
        .unwrap();
    assert_eq!(session.document(), &before);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
    // Enabling Auto Key flushes through global key_edited, not a no-op gesture.
    session.key_edited(0, 0.25, None).unwrap();
    assert!(!session.pose_edits().has_draw_order(0));
    assert_eq!(session.document().animations[0].draw_order.len(), 1);
}

#[test]
fn draw_order_global_keying_is_one_mixed_step_and_scoped_keying_keeps_other_pending() {
    let before = fixture();
    let mut session = Session::new(before.clone()).unwrap();
    edit_root(&mut session, 0.25);
    session
        .edit_draw_order(target(0.25), vec![1, 2, 3, 0], false)
        .unwrap();
    let pending = session.pose_edits().clone();
    session.key_edited(0, 0.25, None).unwrap();
    let keyed = session.document().clone();
    assert_eq!(keyed.animations[0].tracks[0].keys[0].value, 15.);
    assert_eq!(
        keyed.animations[0].draw_order[0].order(4).unwrap(),
        [1, 2, 3, 0]
    );
    assert!(session.pose_edits().is_empty());
    session.undo();
    assert_eq!(session.document(), &before);
    // Established WorkingState history retains the pre-key pose. Native Spine
    // was observed to re-evaluate keys and omit this pending state on Undo.
    assert_eq!(session.pose_edits(), &pending);
    session.redo();
    assert_eq!(session.document(), &keyed);
    assert!(session.pose_edits().is_empty());

    let mut order_only = Session::new(before.clone()).unwrap();
    edit_root(&mut order_only, 0.25);
    order_only
        .edit_draw_order(target(0.25), vec![1, 2, 3, 0], false)
        .unwrap();
    order_only.key_draw_order(0, 0.25).unwrap();
    assert!(order_only.pose_edits().has_bone(0, 0));
    assert!(!order_only.pose_edits().has_draw_order(0));
    assert!(order_only.document().animations[0].tracks.is_empty());

    let mut bone_only = Session::new(before).unwrap();
    edit_root(&mut bone_only, 0.25);
    bone_only
        .edit_draw_order(target(0.25), vec![1, 2, 3, 0], false)
        .unwrap();
    bone_only.key_edited(0, 0.25, Some(0)).unwrap();
    assert!(!bone_only.pose_edits().has_bone(0, 0));
    assert!(bone_only.pose_edits().has_draw_order(0));
    assert!(bone_only.document().animations[0].draw_order.is_empty());
}

#[test]
fn draw_order_previews_use_fixed_history_baselines_and_cancel_preserves_redo() {
    let before = apply_command(&fixture(), raw(0.5, &[(0, 3)])).unwrap();
    for auto_key in [false, true] {
        let mut session = Session::new(before.clone()).unwrap();
        edit_root(&mut session, 0.5);
        let pending = session.pose_edits().clone();
        session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "redo".into(),
            })
            .unwrap();
        session.undo();
        session.begin_transaction();
        session
            .preview_edit_draw_order(target(0.5), vec![3, 2, 1, 0], auto_key)
            .unwrap();
        session
            .preview_edit_draw_order(target(0.5), vec![1, 2, 3, 0], auto_key)
            .unwrap();
        assert_eq!(session.document(), &before);
        assert_eq!(session.pose_edits(), &pending);
        session
            .preview_edit_draw_order(target(0.5), vec![0, 2, 1, 3], auto_key)
            .unwrap();
        let valid_document = session.document().clone();
        let valid_pose = session.pose_edits().clone();
        assert!(
            session
                .preview_edit_draw_order(target(0.5), vec![0, 0, 1, 3], auto_key)
                .is_err()
        );
        assert_eq!(session.document(), &valid_document);
        assert_eq!(session.pose_edits(), &valid_pose);
        session.cancel_transaction();
        assert_eq!(session.document(), &before);
        assert_eq!(session.pose_edits(), &pending);
        assert!(session.can_redo());
        session.begin_transaction();
        session
            .preview_edit_draw_order(target(0.5), vec![3, 2, 1, 0], auto_key)
            .unwrap();
        session
            .preview_edit_draw_order(target(0.5), vec![0, 2, 1, 3], auto_key)
            .unwrap();
        session.commit_transaction();
        assert_eq!(session.draw_order(target(0.5)).unwrap(), [0, 2, 1, 3]);
        assert!(session.pose_edits().has_bone(0, 0));
        session.undo();
        assert_eq!(session.document(), &before);
        assert_eq!(session.pose_edits(), &pending);
    }
}

#[test]
fn draw_order_keying_guards_forward_snap_loss_and_keeps_exact_fractional_identities() {
    let mut session = Session::new(fixture()).unwrap();
    edit_root(&mut session, 0.026);
    session
        .edit_draw_order(target(0.026), vec![1, 2, 3, 0], false)
        .unwrap();
    let before = session.document().clone();
    let pending = session.pose_edits().clone();
    for global in [false, true] {
        let result = if global {
            session.key_edited(0, 0.026, None)
        } else {
            session.key_draw_order(0, 0.026)
        };
        assert!(result.unwrap_err().contains("lose the current pose"));
        assert_eq!(session.document(), &before);
        assert_eq!(session.pose_edits(), &pending);
    }
    let first = 0.500_003_f32;
    let second = f32::from_bits(first.to_bits() + 1);
    let doc = apply_command(
        &fixture(),
        Command::Batch {
            commands: vec![raw(first, &[(0, 3)]), raw(second, &[])],
        },
    )
    .unwrap();
    assert_eq!(
        draw_order_key_time(&doc, 0, first)
            .unwrap()
            .unwrap()
            .to_bits(),
        first.to_bits()
    );
    assert_eq!(
        draw_order_key_time(&doc, 0, second)
            .unwrap()
            .unwrap()
            .to_bits(),
        second.to_bits()
    );
    let mut exact = Session::new(doc.clone()).unwrap();
    exact
        .edit_draw_order(target(first), vec![3, 2, 1, 0], true)
        .unwrap();
    assert_eq!(
        exact.document().animations[0].draw_order[0].time.to_bits(),
        first.to_bits()
    );
    assert_eq!(
        exact.document().animations[0].draw_order[1],
        doc.animations[0].draw_order[1]
    );
    let midpoint = f32::from_bits(first.to_bits() + 1);
    let ambiguous = apply_command(
        &fixture(),
        Command::Batch {
            commands: vec![
                raw(first, &[]),
                raw(f32::from_bits(first.to_bits() + 2), &[]),
            ],
        },
    )
    .unwrap();
    assert!(draw_order_key_time(&ambiguous, 0, midpoint).is_err());
}

#[test]
fn draw_order_pending_rejects_slot_growth_but_survives_rename_and_recoverable_discard() {
    let mut session = Session::new(fixture()).unwrap();
    session
        .edit_draw_order(target(0.25), vec![1, 2, 3, 0], false)
        .unwrap();
    let before = session.document().clone();
    let pending = session.pose_edits().clone();
    assert!(
        session
            .dispatch(Command::AddSlot {
                slot: Slot {
                    name: "extra".into(),
                    bone: 0,
                    attachment: None,
                    color: WHITE,
                    blend: BlendMode::Normal,
                }
            })
            .unwrap_err()
            .contains("Slot count")
    );
    assert_eq!(session.document(), &before);
    assert_eq!(session.pose_edits(), &pending);
    session
        .dispatch(Command::RenameSlot {
            slot: 0,
            name: "renamed".into(),
        })
        .unwrap();
    assert_eq!(session.pose_edits(), &pending);
    assert_eq!(session.draw_order(target(0.25)).unwrap(), [1, 2, 3, 0]);
    session.discard_pose(true);
    assert!(session.pose_edits().is_empty());
    session.undo();
    assert_eq!(session.pose_edits(), &pending);
    assert_eq!(session.draw_order(target(0.75)).unwrap(), [1, 2, 3, 0]);
    session.discard_pose(false);
    session.undo();
    assert!(session.pose_edits().is_empty());
}

#[test]
fn draw_order_movement_uses_stable_slot_identity_and_back_to_front_indices() {
    let order = [2, 0, 3, 1];
    assert_eq!(move_draw_order_slot(&order, 0, 1).unwrap(), [2, 3, 0, 1]);
    assert_eq!(move_draw_order_slot(&order, 0, -1).unwrap(), [0, 2, 3, 1]);
    assert_eq!(move_draw_order_slot(&order, 0, 5).unwrap(), [2, 3, 1, 0]);
    assert_eq!(move_draw_order_slot(&order, 0, -5).unwrap(), [0, 2, 3, 1]);
    assert_eq!(move_draw_order_slot(&order, 1, i32::MAX).unwrap(), order);
    assert_eq!(move_draw_order_slot(&order, 2, i32::MIN).unwrap(), order);
    assert_eq!(
        place_draw_order_slot(&[0, 1, 2, 3], 3, 0, DrawOrderPlacement::Behind).unwrap(),
        [3, 0, 1, 2]
    );
    assert_eq!(
        place_draw_order_slot(&[0, 1, 2, 3], 0, 3, DrawOrderPlacement::InFront).unwrap(),
        [1, 2, 3, 0]
    );
    assert_eq!(
        place_draw_order_slot(&order, 0, 0, DrawOrderPlacement::Behind).unwrap(),
        order
    );
    assert!(move_draw_order_slot(&order, 4, 1).is_err());
    assert!(move_draw_order_slot(&[], 0, 1).is_err());
    assert!(move_draw_order_slot(&[0, 0], 0, 1).is_err());
    assert!(place_draw_order_slot(&order, 0, 4, DrawOrderPlacement::Behind).is_err());
}

#[test]
fn draw_order_typed_copy_paste_retime_delete_preserve_raw_offsets_and_reject_layout_changes() {
    let before = apply_command(
        &fixture(),
        Command::Batch {
            commands: vec![
                raw(0.25, &[(0, 0), (1, 0), (2, 0), (3, 0)]),
                raw(0.5, &[(0, 3), (1, -1), (2, -1), (3, -1)]),
                Command::SetKey {
                    animation: 0,
                    bone: 0,
                    property: Property::Rotation,
                    key: Keyframe {
                        time: 0.25,
                        value: 15.,
                        curve: Curve::Linear,
                    },
                },
            ],
        },
    )
    .unwrap();
    let clipboard = copy_animation_keys(&before, 0, &[identity(0.5), identity(0.25)]).unwrap();
    let AnimationKeyClipboard::DrawOrder { slots, keys } = &clipboard else {
        panic!()
    };
    assert_eq!(slots, &vec!["slot0", "slot1", "slot2", "slot3"]);
    assert_eq!(keys[0].time, 0.);
    assert_eq!(keys[1].time, 0.25);
    assert_eq!(keys[0].offsets, before.animations[0].draw_order[0].offsets);
    assert_eq!(keys[1].offsets, before.animations[0].draw_order[1].offsets);
    assert_eq!(
        pasted_animation_keys(&before, 1, KeyOwner::DrawOrder {}, 1., &clipboard).unwrap(),
        [identity(1.), identity(1.25)]
    );
    let paste = Command::PasteAnimationKeys {
        animation: 1,
        target: KeyOwner::DrawOrder {},
        time: 1.,
        clipboard: clipboard.clone(),
    };
    let pasted = apply_command(&before, paste.clone()).unwrap();
    assert_eq!(pasted.animations[1].draw_order[0].offsets, keys[0].offsets);
    assert_eq!(pasted.animations[1].draw_order[1].offsets, keys[1].offsets);
    let copied = apply_command(
        &before,
        Command::MoveAnimationKeys {
            animation: 0,
            keys: vec![identity(0.25)],
            delta: 1.,
            copy: true,
        },
    )
    .unwrap();
    assert_eq!(copied.animations[0].draw_order.len(), 3);
    assert_eq!(copied.animations[0].draw_order[2].offsets, keys[0].offsets);
    assert_eq!(
        copied.animations[0].draw_order[..2],
        before.animations[0].draw_order
    );
    let moved = apply_command(
        &before,
        Command::MoveAnimationKeys {
            animation: 0,
            keys: vec![identity(0.25), root_rotation(0.25)],
            delta: 0.25,
            copy: false,
        },
    )
    .unwrap();
    assert_eq!(moved.animations[0].draw_order.len(), 1);
    assert_eq!(moved.animations[0].draw_order[0].time, 0.5);
    assert_eq!(
        moved.animations[0].draw_order[0].offsets,
        before.animations[0].draw_order[0].offsets
    );
    assert_eq!(moved.animations[0].tracks[0].keys[0].time, 0.5);
    let deleted = apply_command(
        &before,
        Command::DeleteAnimationKeys {
            animation: 0,
            keys: vec![identity(0.5)],
        },
    )
    .unwrap();
    assert_eq!(
        deleted.animations[0].draw_order,
        before.animations[0].draw_order[..1]
    );
    assert_eq!(deleted.animations[0].tracks, before.animations[0].tracks);
    assert!(copy_animation_keys(&before, 0, &[identity(0.25), root_rotation(0.25)]).is_err());
    assert!(
        apply_command(
            &before,
            Command::DeleteAnimationKeys {
                animation: 0,
                keys: vec![identity(f32::from_bits(0.5_f32.to_bits() + 1))]
            }
        )
        .is_err()
    );
    let mut session = Session::new(before.clone()).unwrap();
    let mismatch = Command::Batch {
        commands: vec![
            Command::RenameSlot {
                slot: 0,
                name: "other".into(),
            },
            paste,
        ],
    };
    assert!(
        session
            .dispatch(mismatch)
            .unwrap_err()
            .contains("slot-name layout")
    );
    assert_eq!(session.document(), &before);
    assert!(!session.can_undo());
    let mut invalid_offsets = clipboard.clone();
    if let AnimationKeyClipboard::DrawOrder { keys, .. } = &mut invalid_offsets {
        keys[0].offsets = vec![DrawOrderOffset { slot: 4, offset: 0 }];
    }
    for (time, clipboard) in [(1., invalid_offsets), (f32::MAX, clipboard.clone())] {
        assert!(
            session
                .dispatch(Command::Batch {
                    commands: vec![
                        Command::RenameAnimation {
                            animation: 0,
                            name: "must rollback".into()
                        },
                        Command::PasteAnimationKeys {
                            animation: 1,
                            target: KeyOwner::DrawOrder {},
                            time,
                            clipboard
                        },
                    ]
                })
                .is_err()
        );
        assert_eq!(session.document(), &before);
        assert!(!session.can_undo());
    }
    assert!(
        pasted_animation_keys(&before, 1, KeyOwner::DrawOrder {}, f32::MAX, &clipboard).is_err()
    );
    let wire = serde_json::to_value(&clipboard).unwrap();
    assert_eq!(
        serde_json::to_value(
            serde_json::from_value::<AnimationKeyClipboard>(wire.clone()).unwrap()
        )
        .unwrap(),
        wire
    );
    let mut unknown = wire;
    unknown["bone"] = serde_json::json!(0);
    assert!(serde_json::from_value::<AnimationKeyClipboard>(unknown).is_err());
    assert_eq!(
        serde_json::to_value(KeyOwner::DrawOrder {}).unwrap(),
        serde_json::json!({"type":"drawOrder"})
    );
    assert!(serde_json::from_value::<KeyOwner>(serde_json::json!({"type":"drawOrder"})).is_ok());
    assert!(
        serde_json::from_value::<KeyOwner>(serde_json::json!({"type":"drawOrder","slot":0}))
            .is_err()
    );
}

#[test]
fn draw_order_discrete_keys_count_for_curve_resolution_without_becoming_curves() {
    let before = apply_command(
        &fixture(),
        Command::Batch {
            commands: vec![
                raw(0.5, &[]),
                Command::SetKey {
                    animation: 0,
                    bone: 0,
                    property: Property::Rotation,
                    key: Keyframe {
                        time: 0.,
                        value: 0.,
                        curve: Curve::Linear,
                    },
                },
                Command::SetKey {
                    animation: 0,
                    bone: 0,
                    property: Property::Rotation,
                    key: Keyframe {
                        time: 1.,
                        value: 15.,
                        curve: Curve::Linear,
                    },
                },
            ],
        },
    )
    .unwrap();
    assert_eq!(
        curve_targets(&before, 0, &[root_rotation(1.)])
            .unwrap()
            .len(),
        1
    );
    assert!(
        curve_targets(&before, 0, &[root_rotation(1.), identity(0.5)])
            .unwrap()
            .is_empty()
    );
    assert!(
        curve_targets(&before, 0, &[identity(0.5)])
            .unwrap()
            .is_empty()
    );
    assert!(curve_targets(&before, 0, &[identity(0.5001)]).is_err());
    assert!(
        curve_channels(
            &before,
            0,
            &CurveTarget {
                key: identity(0.5),
                next_time: 1.
            }
        )
        .unwrap_err()
        .contains("no interpolated curve")
    );
    let mut session = Session::new(before.clone()).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    session
        .dispatch(Command::ApplyCurvePreset {
            animation: 0,
            selection: vec![
                CurveSelection {
                    key: root_rotation(1.),
                    channel: 0,
                },
                CurveSelection {
                    key: identity(0.5),
                    channel: 0,
                },
            ],
            preset: CurvePreset::Bezier,
            separate: false,
        })
        .unwrap();
    assert_eq!(session.document(), &before);
    assert!(session.can_redo());
}
