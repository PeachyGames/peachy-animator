//! Native v21 → v22 adds empty sequence track lists; older formats cannot
//! carry blend modes or sequences.
use peachy_document::FORMAT_VERSION;
use peachy_io::decode_project;
use serde_json::{Value, json};

const V21: &str = include_str!("fixtures/native-v21-pre-sequence/peach-bot.peachy");

#[test]
fn v21_gains_empty_sequence_tracks_and_legacy_formats_refuse_new_data() {
    let old: Value = serde_json::from_str(V21).unwrap();
    assert_eq!(old["format"], 21);
    let migrated = decode_project(V21).unwrap();
    assert_eq!(migrated.format, FORMAT_VERSION);
    let current =
        decode_project(include_str!("../../../examples/peach-bot/peach-bot.peachy")).unwrap();
    assert_eq!(migrated, current);
    let mut blend = old.clone();
    blend["slots"][0]["blend"] = json!("additive");
    let mut sequence = old;
    sequence["skins"]["default"]["thigh.L"]["thigh.L"]["sequence"] =
        json!({"count":2,"start":1,"digits":0,"setup_index":0});
    for smuggled in [blend, sequence] {
        assert!(
            decode_project(&smuggled.to_string())
                .unwrap_err()
                .contains("cannot contain blend modes or image sequences")
        );
    }
}
