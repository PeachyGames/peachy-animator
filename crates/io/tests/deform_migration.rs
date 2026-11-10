//! Native v12 → v13 adds empty deform key lists; older formats cannot carry them.
use peachy_document::FORMAT_VERSION;
use peachy_io::decode_project;
use serde_json::{Value, json};

const V12: &str = include_str!("fixtures/native-v12-pre-deform/peach-bot.peachy");

#[test]
fn v12_gains_empty_deform_lists_and_legacy_formats_refuse_them() {
    let old: Value = serde_json::from_str(V12).unwrap();
    assert_eq!(old["format"], 12);
    let migrated = decode_project(V12).unwrap();
    assert_eq!(migrated.format, FORMAT_VERSION);
    assert!(
        migrated
            .animations
            .iter()
            .all(|a| a.deform_tracks.is_empty())
    );
    let current =
        decode_project(include_str!("../../../examples/peach-bot/peach-bot.peachy")).unwrap();
    assert_eq!(migrated, current);
    let mut smuggled = old;
    smuggled["animations"][0]["deform_tracks"] = json!([]);
    assert!(
        decode_project(&smuggled.to_string())
            .unwrap_err()
            .contains("cannot contain deform keys")
    );
}
