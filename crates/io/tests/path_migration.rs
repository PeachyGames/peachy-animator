//! Native v19 → v20 adds empty path track lists; older formats cannot carry
//! paths or path constraints.
use peachy_document::FORMAT_VERSION;
use peachy_io::decode_project;
use serde_json::{Value, json};

const V19: &str = include_str!("fixtures/native-v19-pre-path/peach-bot.peachy");

#[test]
fn v19_gains_empty_path_tracks_and_legacy_formats_refuse_paths() {
    let old: Value = serde_json::from_str(V19).unwrap();
    assert_eq!(old["format"], 19);
    let migrated = decode_project(V19).unwrap();
    assert_eq!(migrated.format, FORMAT_VERSION);
    let current =
        decode_project(include_str!("../../../examples/peach-bot/peach-bot.peachy")).unwrap();
    assert_eq!(migrated, current);
    let mut tracks = old.clone();
    tracks["animations"][0]["path_tracks"] = json!([]);
    let mut constraint = old;
    constraint["constraints"] = json!([{"type":"path"}]);
    for smuggled in [tracks, constraint] {
        assert!(
            decode_project(&smuggled.to_string())
                .unwrap_err()
                .contains("cannot contain paths")
        );
    }
}
