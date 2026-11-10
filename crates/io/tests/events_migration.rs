//! Native v14 → v15 adds empty event lists; older formats cannot carry events.
use peachy_document::FORMAT_VERSION;
use peachy_io::decode_project;
use serde_json::{Value, json};

const V14: &str = include_str!("fixtures/native-v14-pre-events/peach-bot.peachy");

#[test]
fn v14_gains_empty_events_and_legacy_formats_refuse_them() {
    let old: Value = serde_json::from_str(V14).unwrap();
    assert_eq!(old["format"], 14);
    let migrated = decode_project(V14).unwrap();
    assert_eq!(migrated.format, FORMAT_VERSION);
    let current =
        decode_project(include_str!("../../../examples/peach-bot/peach-bot.peachy")).unwrap();
    assert_eq!(migrated, current);
    let mut smuggled = old.clone();
    smuggled["events"] = json!([]);
    assert!(
        decode_project(&smuggled.to_string())
            .unwrap_err()
            .contains("cannot contain events")
    );
    let mut keyed = old;
    keyed["animations"][0]["events"] = json!([]);
    assert!(
        decode_project(&keyed.to_string())
            .unwrap_err()
            .contains("cannot contain events")
    );
}
