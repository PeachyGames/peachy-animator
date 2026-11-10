//! Native v15 → v16 adds empty draw order lists; older formats cannot carry them.
use peachy_document::FORMAT_VERSION;
use peachy_io::decode_project;
use serde_json::{Value, json};

const V15: &str = include_str!("fixtures/native-v15-pre-draw-order/peach-bot.peachy");

#[test]
fn v15_gains_empty_draw_order_and_legacy_formats_refuse_it() {
    let old: Value = serde_json::from_str(V15).unwrap();
    assert_eq!(old["format"], 15);
    let migrated = decode_project(V15).unwrap();
    assert_eq!(migrated.format, FORMAT_VERSION);
    let current =
        decode_project(include_str!("../../../examples/peach-bot/peach-bot.peachy")).unwrap();
    assert_eq!(migrated, current);
    let mut smuggled = old;
    smuggled["animations"][0]["draw_order"] = json!([]);
    assert!(
        decode_project(&smuggled.to_string())
            .unwrap_err()
            .contains("cannot contain draw order keys")
    );
}
