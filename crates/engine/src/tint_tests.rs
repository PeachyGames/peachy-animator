//! Attachment tint and offset curves (checked end to end against the external
//! runtime by the Tint Lab fixture).
use crate::*;
use peachy_document::*;

fn lab() -> Document {
    serde_json::from_str(include_str!("../../../examples/peach-bot/tint-bot.peachy")).unwrap()
}

#[test]
fn draws_multiply_slot_color_by_attachment_color() {
    let doc = lab();
    let scene = checked_evaluate_scene(&doc, None, 0., "peach", None).unwrap();
    let head = doc.slots.iter().position(|s| s.name == "head").unwrap();
    let draw = scene.attachments.iter().find(|d| d.slot == head).unwrap();
    let slot = doc.slots[head].color;
    let tint = doc.attachment(&draw.attachment).unwrap().color();
    let expected: [u8; 4] = std::array::from_fn(|i| {
        ((f32::from(slot[i]) / 255. * f32::from(tint[i]) / 255.) * 255.).round() as u8
    });
    assert_eq!(draw.color, expected);
    assert_ne!(tint, WHITE);
}

#[test]
fn offset_curves_overshoot_between_equal_values_and_end_on_the_next_key() {
    let curve = Curve::BezierOffset([0.3, 30., 0.6, -20.]);
    assert_eq!(curve_value(curve, 0., -15., -15.), -15.);
    assert_eq!(curve_value(curve, 1., -15., -15.), -15.);
    let peak = (1..10)
        .map(|i| curve_value(curve, i as f32 / 10., -15., -15.))
        .fold(f32::MIN, f32::max);
    assert!(peak > -10., "overshoot {peak}");
    // With unequal values the offsets still define a well-formed curve.
    assert_eq!(curve_value(curve, 1., 0., 10.), 10.);
}
