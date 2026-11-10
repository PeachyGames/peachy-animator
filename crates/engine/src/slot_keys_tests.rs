//! Slot attachment swaps and color keys in evaluated scenes (checked end to end
//! against the external runtime by the Slot Lab fixture).
use crate::*;
use peachy_document::*;

fn lab() -> Document {
    serde_json::from_str(include_str!("../../../examples/peach-bot/slot-bot.peachy")).unwrap()
}

#[test]
fn keyed_attachments_swap_hide_and_return_with_setup_before_the_first_key() {
    let doc = lab();
    let hand = doc.slots.iter().position(|s| s.name == "hand.L").unwrap();
    let shown = |time| {
        checked_evaluate_scene(&doc, Some(0), time, "peach", None)
            .unwrap()
            .attachments
            .iter()
            .find(|d| d.slot == hand)
            .map(|d| d.attachment.placeholder.clone())
    };
    assert_eq!(shown(0.).as_deref(), Some("hand.L"));
    assert_eq!(shown(0.5).as_deref(), Some("fist"));
    assert_eq!(shown(1.2), None);
    assert_eq!(shown(2.).as_deref(), Some("hand.L"));
    // Setup mode ignores animation keys.
    let setup = checked_evaluate_scene(&doc, None, 0.5, "peach", None).unwrap();
    assert!(
        setup
            .attachments
            .iter()
            .any(|d| d.slot == hand && d.attachment.placeholder == "hand.L")
    );
}

#[test]
fn keyed_colors_interpolate_per_channel_hold_steps_and_default_to_setup() {
    let doc = lab();
    let body = doc.slots.iter().position(|s| s.name == "body").unwrap();
    let color = |time| {
        checked_evaluate_scene(&doc, Some(0), time, "peach", None)
            .unwrap()
            .attachments
            .iter()
            .find(|d| d.slot == body)
            .unwrap()
            .color
    };
    assert_eq!(color(0.1), doc.slots[body].color);
    assert_eq!(color(0.3), [255, 200, 200, 255]);
    assert_eq!(color(0.9), [218, 190, 228, 192]);
    assert_eq!(color(5.), [180, 180, 255, 128]);
    let head = doc.animations[0]
        .color_tracks
        .iter()
        .find(|t| t.keys.len() == 4)
        .unwrap();
    // The stepped key holds until the next key.
    assert_eq!(
        crate::ik::sample_color(head, 1.2),
        crate::ik::sample_color(head, 0.8)
    );
}
