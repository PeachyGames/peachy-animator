//! Native v20 → v21 (IK stretch) changes no data; older formats cannot carry it.
use peachy_document::FORMAT_VERSION;
use peachy_io::decode_project;
use serde_json::{Value, json};

const V20: &str = include_str!("fixtures/native-v20-pre-ik-stretch/peach-bot.peachy");

#[test]
fn v20_migrates_unchanged_and_legacy_formats_refuse_stretch() {
    let old: Value = serde_json::from_str(V20).unwrap();
    assert_eq!(old["format"], 20);
    let migrated = decode_project(V20).unwrap();
    assert_eq!(migrated.format, FORMAT_VERSION);
    let current =
        decode_project(include_str!("../../../examples/peach-bot/peach-bot.peachy")).unwrap();
    assert_eq!(migrated, current);
    let mut smuggled = old;
    smuggled["constraints"] = json!([{"type":"ik","name":"x","bones":[1],"target":2,
        "setup":{"mix":1.0,"bend_positive":true},"stretch":true}]);
    assert!(
        decode_project(&smuggled.to_string())
            .unwrap_err()
            .contains("cannot contain IK stretch")
    );
}
