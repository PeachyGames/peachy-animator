//! Native v17 → v18 (linked meshes) changes no data; older formats cannot
//! carry mesh links.
use peachy_document::FORMAT_VERSION;
use peachy_io::decode_project;
use serde_json::{Value, json};

const V17: &str = include_str!("fixtures/native-v17-pre-linked-mesh/peach-bot.peachy");

#[test]
fn v17_migrates_unchanged_and_legacy_formats_refuse_links() {
    let old: Value = serde_json::from_str(V17).unwrap();
    assert_eq!(old["format"], 17);
    let migrated = decode_project(V17).unwrap();
    assert_eq!(migrated.format, FORMAT_VERSION);
    let current =
        decode_project(include_str!("../../../examples/peach-bot/peach-bot.peachy")).unwrap();
    assert_eq!(migrated, current);
    let mut smuggled = old;
    smuggled["skins"]["default"]["thigh.L"]["thigh.L"]["link"] =
        json!({"skin":"default","source":"thigh.L","timelines":true});
    assert!(
        decode_project(&smuggled.to_string())
            .unwrap_err()
            .contains("cannot contain linked meshes")
    );
}
