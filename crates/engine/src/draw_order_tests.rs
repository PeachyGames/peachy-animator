//! Draw order keys reorder scene draws (checked end to end against the
//! external runtime by the Draw Order Lab fixture).
use crate::*;
use peachy_document::*;

fn lab() -> Document {
    crate::native_fixture::decode(include_str!(
        "../../../examples/peach-bot/draw-order-bot.peachy"
    ))
    .unwrap()
}

fn order(doc: &Document, animation: Option<usize>, time: f32) -> Vec<String> {
    checked_evaluate_scene(doc, animation, time, "peach", None)
        .unwrap()
        .attachments
        .iter()
        .map(|draw| doc.slots[draw.slot].name.clone())
        .collect()
}

#[test]
fn keys_hold_until_the_next_and_setup_applies_before_the_first_and_after_a_reset() {
    let doc = lab();
    let setup: Vec<String> = doc.slots.iter().map(|s| s.name.clone()).collect();
    assert_eq!(order(&doc, Some(0), 0.1), setup);
    let arm_in_front = order(&doc, Some(0), 0.5);
    assert_eq!(&arm_in_front[11..], ["upper-arm.L", "forearm.L", "hand.L"]);
    let head_behind = order(&doc, Some(0), 1.0);
    assert_eq!(head_behind[0], "head");
    assert_eq!(order(&doc, Some(0), 1.5), setup);
    // Setup mode ignores draw order keys.
    assert_eq!(order(&doc, None, 0.5), setup);
}

fn authoring_fixture(slot_count: usize) -> Document {
    let mut doc = Document::new("Original order authoring".into(), 60).unwrap();
    doc.slots = (0..slot_count)
        .map(|slot| Slot {
            name: format!("slot{slot}"),
            bone: 0,
            attachment: None,
            color: WHITE,
            blend: BlendMode::Normal,
        })
        .collect();
    doc.animations.push(Animation::new("order", vec![]));
    doc.validate().unwrap();
    doc
}

fn semantic(time: f32, order: Vec<usize>) -> Command {
    Command::SetDrawOrder {
        animation: 0,
        time,
        order,
    }
}

fn raw(time: f32, offsets: Vec<(usize, i32)>) -> Command {
    Command::SetDrawOrderKey {
        animation: 0,
        key: DrawOrderKey {
            time,
            offsets: offsets
                .into_iter()
                .map(|(slot, offset)| DrawOrderOffset { slot, offset })
                .collect(),
        },
    }
}

fn brute_minimum_offsets(order: &[usize]) -> usize {
    // Enumerate every possible listed-slot subset. A listed slot's only valid
    // destination is its position in the requested result. The existing decoder
    // decides whether the unlisted slots fill the remaining positions correctly.
    // This independent oracle does not compute a subsequence or choose its ties.
    (0usize..1 << order.len())
        .filter_map(|mask| {
            let offsets = (0..order.len())
                .filter(|slot| mask & (1 << slot) != 0)
                .map(|slot| DrawOrderOffset {
                    slot,
                    offset: order.iter().position(|&value| value == slot).unwrap() as i32
                        - slot as i32,
                })
                .collect::<Vec<_>>();
            let candidate = DrawOrderKey { time: 0., offsets };
            (candidate.order(order.len()).unwrap() == order).then_some(candidate.offsets.len())
        })
        .min()
        .unwrap()
}

#[test]
fn draw_order_encoder_round_trips_and_is_minimal_for_all_twenty_four_permutations() {
    let before = authoring_fixture(4);
    let mut count = 0;
    for a in 0..4 {
        for b in (0..4).filter(|&b| b != a) {
            for c in (0..4).filter(|&c| c != a && c != b) {
                let d = (0..4).find(|&d| d != a && d != b && d != c).unwrap();
                let requested = vec![a, b, c, d];
                let after = apply_command(&before, semantic(0.5, requested.clone())).unwrap();
                let key = &after.animations[0].draw_order[0];
                assert_eq!(key.order(4).unwrap(), requested);
                assert_eq!(key.offsets.len(), brute_minimum_offsets(&requested));
                assert_eq!(key.time.to_bits(), 0.5_f32.to_bits());
                assert_eq!(after.slots, before.slots);
                assert_eq!(after.bones, before.bones);
                assert_eq!(after.skins, before.skins);
                assert_eq!(
                    after,
                    apply_command(&before, semantic(0.5, requested.clone())).unwrap(),
                    "The independent encoder has a deterministic tie choice"
                );
                if requested == [0, 1, 2, 3] {
                    assert!(key.offsets.is_empty());
                }
                count += 1;
            }
        }
    }
    assert_eq!(count, 24);
}

#[test]
fn draw_order_raw_redundant_and_zero_offsets_survive_semantic_noops_and_redo() {
    let before = authoring_fixture(4);
    let encoded = apply_command(
        &before,
        Command::Batch {
            commands: vec![
                raw(0.5, vec![(0, 3), (1, -1), (2, -1), (3, -1)]),
                raw(1.5, vec![(0, 0), (1, 0), (2, 0), (3, 0)]),
            ],
        },
    )
    .unwrap();
    assert_eq!(encoded.animations[0].draw_order[0].offsets.len(), 4);
    assert_eq!(encoded.animations[0].draw_order[1].offsets.len(), 4);
    assert_eq!(
        encoded.animations[0].draw_order[0].order(4).unwrap(),
        [1, 2, 3, 0]
    );
    assert_eq!(
        encoded.animations[0].draw_order[1].order(4).unwrap(),
        [0, 1, 2, 3]
    );
    let mut session = Session::new(encoded.clone()).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    session.dispatch(semantic(0.5, vec![1, 2, 3, 0])).unwrap();
    session.dispatch(semantic(1.5, vec![0, 1, 2, 3])).unwrap();
    assert_eq!(session.document(), &encoded);
    assert!(session.can_redo());
    assert!(!session.can_undo());
    // Explicit raw authoring still replaces the representation when requested.
    session.dispatch(raw(1.5, vec![])).unwrap();
    assert!(
        session.document().animations[0].draw_order[1]
            .offsets
            .is_empty()
    );
    session.undo();
    assert_eq!(session.document(), &encoded);
}

#[test]
fn draw_order_setters_keep_adjacent_fractional_key_identities_and_siblings() {
    let first = 0.500_003_f32;
    let second = f32::from_bits(first.to_bits() + 1);
    let third = f32::from_bits(first.to_bits() + 2);
    let before = apply_command(
        &authoring_fixture(4),
        Command::Batch {
            commands: vec![raw(first, vec![(0, 3)]), raw(second, vec![])],
        },
    )
    .unwrap();
    let after = apply_command(&before, semantic(first, vec![3, 2, 1, 0])).unwrap();
    assert_eq!(after.animations[0].draw_order.len(), 2);
    assert_eq!(
        after.animations[0].draw_order[0].time.to_bits(),
        first.to_bits()
    );
    assert_eq!(
        after.animations[0].draw_order[0].order(4).unwrap(),
        [3, 2, 1, 0]
    );
    assert_eq!(
        after.animations[0].draw_order[1],
        before.animations[0].draw_order[1]
    );
    let next = apply_command(&after, raw(second, vec![(3, -3)])).unwrap();
    assert_eq!(
        next.animations[0].draw_order[0],
        after.animations[0].draw_order[0]
    );
    assert_eq!(
        next.animations[0].draw_order[1].time.to_bits(),
        second.to_bits()
    );
    assert_eq!(
        next.animations[0].draw_order[1].order(4).unwrap(),
        [3, 0, 1, 2]
    );
    let inserted = apply_command(&next, semantic(third, vec![0, 2, 1, 3])).unwrap();
    assert_eq!(inserted.animations[0].draw_order.len(), 3);
    assert_eq!(
        inserted.animations[0].draw_order[..2],
        next.animations[0].draw_order
    );
    assert_eq!(
        inserted.animations[0].draw_order[2].time.to_bits(),
        third.to_bits()
    );

    let negative_zero = apply_command(&authoring_fixture(4), raw(-0., vec![])).unwrap();
    let changed = apply_command(&negative_zero, semantic(0., vec![1, 2, 3, 0])).unwrap();
    assert_eq!(
        changed.animations[0].draw_order[0].time.to_bits(),
        (-0_f32).to_bits()
    );
}

#[test]
fn draw_order_invalid_raw_and_semantic_batches_preserve_document_and_redo() {
    let before = authoring_fixture(4);
    let mut session = Session::new(before.clone()).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    let mut actions = vec![
        semantic(0.5, vec![0, 1, 2]),
        semantic(0.5, vec![0, 1, 2, 3, 4]),
        semantic(0.5, vec![0, 1, 1, 3]),
        semantic(0.5, vec![0, 1, 2, 4]),
        semantic(f32::NAN, vec![0, 1, 2, 3]),
        semantic(f32::INFINITY, vec![0, 1, 2, 3]),
        semantic(-0.5, vec![0, 1, 2, 3]),
        raw(f32::NAN, vec![]),
        raw(f32::INFINITY, vec![]),
        raw(-0.5, vec![]),
        raw(0.5, vec![(2, 0), (0, 0)]),
        raw(0.5, vec![(0, 0), (0, 1)]),
        raw(0.5, vec![(0, 1), (1, 0)]),
        raw(0.5, vec![(4, 0)]),
        raw(0.5, vec![(0, -1)]),
        raw(0.5, vec![(3, 1)]),
    ];
    actions.push(Command::SetDrawOrder {
        animation: 99,
        time: 0.5,
        order: vec![0, 1, 2, 3],
    });
    actions.push(Command::SetDrawOrderKey {
        animation: 99,
        key: DrawOrderKey {
            time: 0.5,
            offsets: vec![],
        },
    });
    for action in actions {
        let batch = Command::Batch {
            commands: vec![
                Command::RenameAnimation {
                    animation: 0,
                    name: "must rollback".into(),
                },
                raw(0.25, vec![(0, 3)]),
                action,
            ],
        };
        assert!(session.dispatch(batch).is_err());
        assert_eq!(session.document(), &before);
        assert!(session.can_redo());
        assert!(!session.can_undo());
    }
}

#[test]
fn draw_order_public_pose_accessors_include_all_hidden_slots_and_hold_reset_keys() {
    let doc = apply_command(
        &authoring_fixture(4),
        Command::Batch {
            commands: vec![semantic(0.5, vec![1, 2, 3, 0]), raw(1.5, vec![])],
        },
    )
    .unwrap();
    for (time, expected) in [
        (0., [0, 1, 2, 3]),
        (0.499, [0, 1, 2, 3]),
        (0.5, [1, 2, 3, 0]),
        (1.499, [1, 2, 3, 0]),
        (1.5, [0, 1, 2, 3]),
        (2., [0, 1, 2, 3]),
    ] {
        let authored = sample_authored_pose(&doc, Some(0), time).unwrap();
        let evaluated = checked_evaluate_pose(&doc, Some(0), time).unwrap();
        assert_eq!(authored.draw_order(), &expected);
        assert_eq!(evaluated.draw_order(), &expected);
        assert!(
            checked_evaluate_scene(&doc, Some(0), time, "default", None)
                .unwrap()
                .attachments
                .is_empty()
        );
    }
    assert_eq!(
        sample_authored_pose(&doc, None, 1.).unwrap().draw_order(),
        &[0, 1, 2, 3]
    );
    assert_eq!(
        checked_evaluate_pose(&doc, None, 1.).unwrap().draw_order(),
        &[0, 1, 2, 3]
    );
    assert_eq!(doc.animations[0].duration(), 1.5);
}

#[test]
fn draw_order_commands_are_strict_and_empty_skeleton_reset_is_valid() {
    for command in [
        semantic(0.500_003, vec![1, 2, 3, 0]),
        raw(0.500_003, vec![(0, 3)]),
    ] {
        let wire = serde_json::to_value(command).unwrap();
        assert_eq!(
            serde_json::to_value(serde_json::from_value::<Command>(wire.clone()).unwrap()).unwrap(),
            wire
        );
        let mut unknown = wire;
        unknown["slot"] = serde_json::json!(0);
        assert!(serde_json::from_value::<Command>(unknown).is_err());
    }
    for invalid in [
        serde_json::json!({"command":"setDrawOrderKey","animation":0,"key":{"time":0,"offsets":[],"curve":"linear"}}),
        serde_json::json!({"command":"setDrawOrderKey","animation":0,"key":{"time":0,"offsets":[{"slot":0,"offset":0.5}]}}),
        serde_json::json!({"command":"setDrawOrder","animation":0,"time":0,"order":[0.5]}),
    ] {
        assert!(serde_json::from_value::<Command>(invalid).is_err());
    }
    let empty = apply_command(&authoring_fixture(0), semantic(0.5, vec![])).unwrap();
    assert_eq!(
        empty.animations[0].draw_order,
        vec![DrawOrderKey {
            time: 0.5,
            offsets: vec![]
        }]
    );
    assert!(
        checked_evaluate_pose(&empty, Some(0), 0.5)
            .unwrap()
            .draw_order()
            .is_empty()
    );
}
