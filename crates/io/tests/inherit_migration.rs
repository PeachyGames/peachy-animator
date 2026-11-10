//! Native v9 → v10 makes every bone inherit normally and adds empty inherit key
//! lists. Formats 1–9 can never carry inheritance fields.
use peachy_document::{FORMAT_VERSION, Inherit};
use peachy_io::decode_project;
use serde_json::{Value, json};

#[test]
fn v8_and_v9_examples_migrate_to_the_current_examples() {
    let examples =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
    for (name, source, format) in [
        (
            "peach-bot",
            include_str!("fixtures/native-v8-pre-inherit/peach-bot.peachy"),
            8,
        ),
        (
            "physics-bot",
            include_str!("fixtures/native-v8-pre-inherit/physics-bot.peachy"),
            8,
        ),
        (
            "ik-animation-bot",
            include_str!("fixtures/native-v9-pre-inherit/ik-animation-bot.peachy"),
            9,
        ),
    ] {
        let old: Value = serde_json::from_str(source).unwrap();
        assert_eq!(old["format"], format);
        let migrated = decode_project(source).unwrap();
        assert_eq!(migrated.format, FORMAT_VERSION);
        assert!(migrated.bones.iter().all(|b| b.inherit == Inherit::Normal));
        assert!(
            migrated
                .animations
                .iter()
                .all(|a| a.inherit_tracks.is_empty())
        );
        let current = decode_project(
            &std::fs::read_to_string(examples.join(format!("{name}.peachy"))).unwrap(),
        )
        .unwrap();
        assert_eq!(migrated, current, "{name}");
    }
}

#[test]
fn legacy_formats_refuse_inherit_fields_and_current_requires_them() {
    let source: Value = serde_json::from_str(include_str!(
        "fixtures/native-v9-pre-inherit/ik-animation-bot.peachy"
    ))
    .unwrap();
    let mut bone_field = source.clone();
    bone_field["bones"][1]["inherit"] = json!("normal");
    let mut track_field = source.clone();
    track_field["animations"][0]["inherit_tracks"] = json!([]);
    for bad in [bone_field, track_field] {
        assert!(
            decode_project(&bad.to_string())
                .unwrap_err()
                .contains("cannot contain inherit fields")
        );
    }
    let mut current = source;
    current["format"] = json!(FORMAT_VERSION);
    assert!(decode_project(&current.to_string()).is_err());
}
