//! Native v16 → v17 (attachment colors and offset curves) changes no data;
//! older formats cannot carry attachment colors.
use peachy_document::FORMAT_VERSION;
use peachy_io::decode_project;
use serde_json::{Value, json};

const V16: &str = include_str!("fixtures/native-v16-pre-attachment-color/peach-bot.peachy");

#[test]
fn v16_migrates_unchanged_and_legacy_formats_refuse_attachment_colors() {
    let old: Value = serde_json::from_str(V16).unwrap();
    assert_eq!(old["format"], 16);
    let migrated = decode_project(V16).unwrap();
    assert_eq!(migrated.format, FORMAT_VERSION);
    let current =
        decode_project(include_str!("../../../examples/peach-bot/peach-bot.peachy")).unwrap();
    assert_eq!(migrated, current);
    let mut smuggled = old;
    smuggled["skins"]["default"]["thigh.L"]["thigh.L"]["color"] = json!([1, 2, 3, 4]);
    assert!(
        decode_project(&smuggled.to_string())
            .unwrap_err()
            .contains("cannot contain attachment colors")
    );
}
