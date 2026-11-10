//! Native v18 → v19 (clipping attachments) changes no data; older formats
//! cannot carry clipping attachments.
use peachy_document::FORMAT_VERSION;
use peachy_io::decode_project;
use serde_json::{Value, json};

const V18: &str = include_str!("fixtures/native-v18-pre-clipping/peach-bot.peachy");

#[test]
fn v18_migrates_unchanged_and_legacy_formats_refuse_clipping() {
    let old: Value = serde_json::from_str(V18).unwrap();
    assert_eq!(old["format"], 18);
    let migrated = decode_project(V18).unwrap();
    assert_eq!(migrated.format, FORMAT_VERSION);
    let current =
        decode_project(include_str!("../../../examples/peach-bot/peach-bot.peachy")).unwrap();
    assert_eq!(migrated, current);
    let mut smuggled = old;
    smuggled["skins"]["default"]["thigh.L"]["clip"] = json!({"type":"clipping","end":null,
        "vertices":{"mode":"slotLocal","positions":[[0,0],[1,0],[0,1]]},"convex":false});
    assert!(
        decode_project(&smuggled.to_string())
            .unwrap_err()
            .contains("cannot contain clipping attachments")
    );
}
