//! Native v7 → v8: physics timeline lists are added, and older formats can never
//! carry physics data. Fixtures are byte-for-byte v7 Peach Bot files.
use peachy_document::FORMAT_VERSION;
use peachy_io::decode_project;
use serde_json::{Value, json};

const V7: [(&str, &str); 2] = [
    (
        "peach-bot",
        include_str!("fixtures/native-v7-pre-physics/peach-bot.peachy"),
    ),
    (
        "ik-bot",
        include_str!("fixtures/native-v7-pre-physics/ik-bot.peachy"),
    ),
];

#[test]
fn v7_examples_migrate_to_the_current_examples() {
    let examples =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
    for (name, source) in V7 {
        let old: Value = serde_json::from_str(source).unwrap();
        assert_eq!(old["format"], 7);
        let migrated = decode_project(source).unwrap();
        assert_eq!(migrated.format, FORMAT_VERSION);
        assert!(
            migrated
                .animations
                .iter()
                .all(|a| a.physics.is_empty() && a.physics_resets.is_empty())
        );
        let current = decode_project(
            &std::fs::read_to_string(examples.join(format!("{name}.peachy"))).unwrap(),
        )
        .unwrap();
        assert_eq!(migrated, current, "{name}");
        // Spine export of migrated data is unchanged: no physics fields appear.
        let output = peachy_spine::export_json(&migrated).unwrap();
        assert!(
            output["animations"]
                .as_object()
                .unwrap()
                .values()
                .all(|a| a.get("physics").is_none())
        );
    }
}

#[test]
fn legacy_formats_cannot_smuggle_physics_data() {
    let source: Value = serde_json::from_str(V7[1].1).unwrap();
    for version in [6, 7] {
        let mut physics_constraint = source.clone();
        physics_constraint["format"] = json!(version);
        if version == 6 {
            for slot in physics_constraint["slots"].as_array_mut().unwrap() {
                slot.as_object_mut().unwrap().remove("color");
            }
        }
        let mut with_field = physics_constraint.clone();
        physics_constraint["constraints"]
            .as_array_mut()
            .unwrap()
            .push(json!({"type":"physics","name":"p","bone":1}));
        assert!(
            decode_project(&physics_constraint.to_string())
                .unwrap_err()
                .contains("only IK constraints"),
            "v{version}"
        );
        for field in ["physics", "physics_resets"] {
            with_field["animations"][0][field] = json!([]);
            assert!(
                decode_project(&with_field.to_string())
                    .unwrap_err()
                    .contains("cannot contain physics timelines"),
                "v{version} {field}"
            );
            with_field["animations"][0]
                .as_object_mut()
                .unwrap()
                .remove(field);
        }
    }
    // Current v8 requires both lists rather than defaulting them.
    let mut current = source;
    current["format"] = json!(FORMAT_VERSION);
    assert!(decode_project(&current.to_string()).is_err());
}
