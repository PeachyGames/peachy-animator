//! Draw order keys reorder scene draws (checked end to end against the
//! external runtime by the Draw Order Lab fixture).
use crate::*;
use peachy_document::*;

fn lab() -> Document {
    serde_json::from_str(include_str!(
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
