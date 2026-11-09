use peachy_document::{Document, FORMAT_VERSION};
use peachy_io::{decode_project, load, save};
use serde_json::{Value, json};
use std::{fs, path::Path, time::SystemTime};

fn decode(value: &Value) -> Result<Document, String> {
    decode_project(&value.to_string())
}

// Start with an actual valid old file, rather than relabeling a latest schema.
fn legacy(version: u32) -> Value {
    let mut value: Value =
        serde_json::from_str(include_str!("fixtures/legacy-v2-duration.peachy")).unwrap();
    value["format"] = json!(version);
    if version >= 3 {
        for animation in value["animations"].as_array_mut().unwrap() {
            animation.as_object_mut().unwrap().remove("duration");
        }
    }
    if version == 5 {
        for bone in value["bones"].as_array_mut().unwrap() {
            bone["setup"]["shear_x"] = json!(0);
            bone["setup"]["shear_y"] = json!(0);
        }
    }
    value
}

#[test]
fn all_legacy_versions_reject_constraint_presence_before_migration() {
    for version in 1..=5 {
        let source = legacy(version);
        let doc = decode(&source).unwrap();
        assert_eq!(doc.format, FORMAT_VERSION);
        assert!(doc.constraints.is_empty());
        for injected in [
            json!([]),
            Value::Null,
            json!({}),
            json!([{
                "type":"ik", "name":"future", "bones":[1], "target":0,
                "setup":{"mix":0,"bend_positive":true}
            }]),
        ] {
            let mut smuggled = source.clone();
            smuggled["constraints"] = injected;
            assert!(
                decode(&smuggled)
                    .unwrap_err()
                    .contains("Legacy project formats cannot contain constraints")
            );
        }
    }
}

#[test]
fn latest_requires_a_strict_typed_collection_and_keeps_animation_schema_unchanged() {
    let latest = serde_json::to_value(decode(&legacy(5)).unwrap()).unwrap();
    let mut missing = latest.clone();
    missing.as_object_mut().unwrap().remove("constraints");
    assert!(
        decode(&missing)
            .unwrap_err()
            .contains("missing field `constraints`")
    );
    for invalid in [Value::Null, json!({}), json!([{"type":"physics"}])] {
        let mut bad = latest.clone();
        bad["constraints"] = invalid;
        assert!(decode(&bad).is_err());
    }
    let mut valid = latest.clone();
    valid["constraints"] = json!([{"type":"ik","name":"aim","bones":[1],"target":0,
        "setup":{"mix":0.5,"bend_positive":false}}]);
    let doc = decode(&valid).unwrap();
    assert_eq!(decode(&serde_json::to_value(&doc).unwrap()).unwrap(), doc);
    for field in ["mix", "bend_positive"] {
        let mut missing = valid.clone();
        missing["constraints"][0]["setup"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(decode(&missing).unwrap_err().contains("missing field"));
    }
    for pointer in ["/constraints/0", "/constraints/0/setup", "/animations/0"] {
        let mut bad = valid.clone();
        bad.pointer_mut(pointer).unwrap()["future"] = json!([]);
        assert!(decode(&bad).unwrap_err().contains("unknown field"));
    }
    let mut bad = latest;
    bad["animations"][0]["ik_timelines"] = json!([]);
    assert!(decode(&bad).unwrap_err().contains("unknown field"));
}

#[test]
fn eight_original_v5_and_current_exports_keep_every_pre_ik_byte() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let old = root.join("tests/fixtures/native-v5-zero-ik");
    for name in [
        "peach-bot",
        "contract-cases",
        "mesh-bot",
        "mesh-contract",
        "weighted-bot",
        "weighted-contract",
        "weighted-repeated",
        "shear-bot",
    ] {
        let historical = fs::read_to_string(old.join(format!("{name}.peachy"))).unwrap();
        let original: Value = serde_json::from_str(&historical).unwrap();
        assert_eq!(original["format"], 5);
        assert!(original.get("constraints").is_none());
        let golden = fs::read(old.join(format!("{name}.json"))).unwrap();
        for input in [
            historical,
            fs::read_to_string(root.join(format!("../../examples/peach-bot/{name}.peachy")))
                .unwrap(),
        ] {
            let doc = decode_project(&input).unwrap();
            assert!(doc.constraints.is_empty());
            assert_eq!(
                serde_json::to_vec_pretty(&peachy_spine::export_json(&doc).unwrap()).unwrap(),
                golden,
                "Full pre-IK export changed: {name}"
            );
        }
    }
}

#[test]
fn v5_load_is_read_only_and_explicit_latest_save_backs_up_original_bytes() {
    let original = include_bytes!("fixtures/native-v5-zero-ik/weighted-repeated.peachy");
    let folder = std::env::temp_dir().join(format!(
        "peachy-ik-migrate-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&folder).unwrap();
    let path = folder.join("old.peachy");
    fs::write(&path, original).unwrap();
    let doc = load(&path).unwrap();
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
    let mut expected: Value = serde_json::from_slice(original).unwrap();
    expected["format"] = json!(FORMAT_VERSION);
    expected["constraints"] = json!([]);
    for slot in expected["slots"].as_array_mut().unwrap() {
        slot["color"] = json!([255, 255, 255, 255]);
    }
    for animation in expected["animations"].as_array_mut().unwrap() {
        animation["physics"] = json!([]);
        animation["physics_resets"] = json!([]);
        animation["inherit_tracks"] = json!([]);
    }
    for bone in expected["bones"].as_array_mut().unwrap() {
        bone["inherit"] = json!("normal");
    }
    assert_eq!(doc, serde_json::from_value::<Document>(expected).unwrap());
    save(&path, &doc, &folder).unwrap();
    let backup = fs::read_dir(&folder)
        .unwrap()
        .filter_map(Result::ok)
        .find(|entry| entry.file_name().to_string_lossy().contains("backup-"))
        .unwrap();
    assert_eq!(fs::read(backup.path()).unwrap(), original);
    assert_eq!(load(&path).unwrap(), doc);
    fs::remove_dir_all(folder).unwrap();
}
