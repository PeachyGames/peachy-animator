//! Native v10 → v11 adds the transform constraint kind; nothing else changes.
use peachy_document::FORMAT_VERSION;
use peachy_io::decode_project;
use serde_json::{Value, json};

const V10: &str = include_str!("fixtures/native-v10-pre-transform/peach-bot.peachy");

#[test]
fn v10_migrates_unchanged_and_legacy_formats_refuse_transform_constraints() {
    let old: Value = serde_json::from_str(V10).unwrap();
    assert_eq!(old["format"], 10);
    let migrated = decode_project(V10).unwrap();
    assert_eq!(migrated.format, FORMAT_VERSION);
    let current =
        decode_project(include_str!("../../../examples/peach-bot/peach-bot.peachy")).unwrap();
    assert_eq!(migrated, current);
    let mut smuggled = old;
    smuggled["constraints"] = json!([{"type":"transform","name":"t","bones":[1],"source":2,
        "offsets":{"rotate":0,"x":0,"y":0,"scale_x":0,"scale_y":0,"shear_y":0},
        "mix":{"rotate":0,"x":1,"y":0,"scale_x":0,"scale_y":0,"shear_y":0},
        "mappings":[],"local_source":false,"local_target":false,"additive":false,"clamp":false}]);
    assert!(
        decode_project(&smuggled.to_string())
            .unwrap_err()
            .contains("cannot contain transform constraints")
    );
}
