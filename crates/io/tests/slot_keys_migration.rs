//! Native v13 → v14 adds empty slot key lists; older formats cannot carry them.
use peachy_document::FORMAT_VERSION;
use peachy_io::decode_project;
use serde_json::{Value, json};

const V13: &str = include_str!("fixtures/native-v13-pre-slot-keys/peach-bot.peachy");

#[test]
fn v13_gains_empty_slot_key_lists_and_legacy_formats_refuse_them() {
    let old: Value = serde_json::from_str(V13).unwrap();
    assert_eq!(old["format"], 13);
    let migrated = decode_project(V13).unwrap();
    assert_eq!(migrated.format, FORMAT_VERSION);
    let current =
        decode_project(include_str!("../../../examples/peach-bot/peach-bot.peachy")).unwrap();
    assert_eq!(migrated, current);
    for field in ["attachment_tracks", "color_tracks"] {
        let mut smuggled = old.clone();
        smuggled["animations"][0][field] = json!([]);
        assert!(
            decode_project(&smuggled.to_string())
                .unwrap_err()
                .contains("cannot contain slot keys")
        );
    }
}
