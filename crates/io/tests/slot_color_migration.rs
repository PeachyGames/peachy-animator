use peachy_document::{Document, FORMAT_VERSION};
use peachy_io::{decode_project, load, save};
use serde_json::{Value, json};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

fn legacy(version: u32) -> Value {
    let mut value: Value =
        serde_json::from_str(include_str!("fixtures/legacy-v2-duration.peachy")).unwrap();
    value["format"] = json!(version);
    value["slots"] = json!([{"name":"paint","bone":0,"attachment":null}]);
    if version >= 3 {
        for animation in value["animations"].as_array_mut().unwrap() {
            animation.as_object_mut().unwrap().remove("duration");
        }
    }
    if version >= 5 {
        for bone in value["bones"].as_array_mut().unwrap() {
            bone["setup"]["shear_x"] = json!(0);
            bone["setup"]["shear_y"] = json!(0);
        }
    }
    if version >= 6 {
        value["constraints"] = json!([]);
    }
    value
}

fn decode(value: &Value) -> Result<Document, String> {
    decode_project(&value.to_string())
}

#[test]
fn every_legacy_version_gains_white_once_and_cannot_smuggle_any_color_presence() {
    for version in 1..=6 {
        let value = legacy(version);
        let source = value.to_string();
        let doc = decode_project(&source).unwrap();
        assert_eq!(doc.format, FORMAT_VERSION);
        assert_eq!(doc.slots[0].color, [255; 4]);
        assert_eq!(source, value.to_string());
        assert_eq!(decode(&serde_json::to_value(&doc).unwrap()).unwrap(), doc);
        for color in [
            json!([255, 255, 255, 255]),
            Value::Null,
            json!("ffffffff"),
            json!([]),
            json!({}),
        ] {
            let mut invalid = value.clone();
            invalid["slots"][0]["color"] = color;
            assert!(
                decode(&invalid)
                    .unwrap_err()
                    .contains("Legacy project formats cannot contain slot color fields"),
                "v{version}"
            );
        }
    }
}

#[test]
fn current_color_requires_exactly_four_typed_rgba_bytes_and_unknown_fields_stay_strict() {
    let current = serde_json::to_value(decode(&legacy(6)).unwrap()).unwrap();
    let mut missing = current.clone();
    missing["slots"][0].as_object_mut().unwrap().remove("color");
    assert!(
        decode(&missing)
            .unwrap_err()
            .contains("missing field `color`")
    );
    for color in [
        Value::Null,
        json!([]),
        json!([255, 255, 255]),
        json!([255, 255, 255, 255, 255]),
        json!([-1, 0, 0, 0]),
        json!([256, 0, 0, 0]),
        json!([1.0, 0, 0, 0]),
        json!([1.5, 0, 0, 0]),
        json!([true, 0, 0, 0]),
        json!(["255", 0, 0, 0]),
        json!("ffffffff"),
        json!({"r":255}),
    ] {
        let mut invalid = current.clone();
        invalid["slots"][0]["color"] = color;
        assert!(decode(&invalid).is_err());
    }
    for color in [[0, 0, 0, 0], [255, 255, 255, 255], [1, 127, 128, 254]] {
        let mut valid = current.clone();
        valid["slots"][0]["color"] = json!(color);
        let doc = decode(&valid).unwrap();
        assert_eq!(doc.slots[0].color, color);
    }
    for field in ["dark", "color_timeline", "future"] {
        let mut invalid = current.clone();
        invalid["slots"][0][field] = json!([]);
        assert!(decode(&invalid).unwrap_err().contains("unknown field"));
    }
    let mut future = current;
    future["format"] = json!(FORMAT_VERSION + 1);
    assert!(decode(&future).is_err());
}

#[test]
fn slot_shape_is_validated_before_white_is_materialized() {
    for slot in [
        Value::Null,
        json!(7),
        json!([]),
        json!({}),
        json!({"name":false,"bone":0}),
        json!({"name":"paint","bone":0,"unexpected":true}),
        json!({"name":"paint","bone":-1}),
    ] {
        let mut invalid = legacy(6);
        invalid["slots"] = json!([slot]);
        assert!(decode(&invalid).is_err());
    }
    for slots in [Value::Null, json!({}), json!("slots")] {
        let mut invalid = legacy(6);
        invalid["slots"] = slots;
        assert!(decode(&invalid).is_err());
    }
}

#[test]
fn all_nine_current_examples_equal_their_white_v6_migration() {
    let examples =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
    for name in [
        "peach-bot",
        "contract-cases",
        "mesh-bot",
        "mesh-contract",
        "weighted-bot",
        "weighted-contract",
        "weighted-repeated",
        "shear-bot",
        "ik-bot",
    ] {
        let bytes = fs::read(examples.join(format!("{name}.peachy"))).unwrap();
        let current: Value = serde_json::from_slice(&bytes).unwrap();
        let doc = decode(&current).unwrap();
        assert_eq!(doc.format, FORMAT_VERSION);
        let mut old = current;
        old["format"] = json!(6);
        assert!(old.as_object_mut().unwrap().remove("skin_scope").is_some());
        assert!(old.as_object_mut().unwrap().remove("events").is_some());
        assert!(
            old.as_object_mut()
                .unwrap()
                .remove("reference_scale")
                .is_some()
        );
        for animation in old["animations"].as_array_mut().unwrap() {
            for field in [
                "ik_tracks",
                "physics",
                "physics_resets",
                "inherit_tracks",
                "deform_tracks",
                "attachment_tracks",
                "color_tracks",
                "events",
                "draw_order",
                "path_tracks",
            ] {
                assert_eq!(
                    animation.as_object_mut().unwrap().remove(field),
                    Some(json!([]))
                );
            }
        }
        for bone in old["bones"].as_array_mut().unwrap() {
            assert_eq!(
                bone.as_object_mut().unwrap().remove("inherit"),
                Some(json!("normal"))
            );
        }
        for slot in old["slots"].as_array_mut().unwrap() {
            assert_eq!(
                slot.as_object_mut().unwrap().remove("color"),
                Some(json!([255, 255, 255, 255]))
            );
        }
        assert_eq!(decode(&old).unwrap(), doc, "{name}");
        assert!(
            peachy_spine::export_json(&doc).unwrap()["slots"]
                .as_array()
                .unwrap()
                .iter()
                .all(|slot| slot.get("color").is_none())
        );
    }
}

#[test]
fn loading_v6_is_read_only_and_explicit_save_backs_up_exact_original_bytes() {
    let folder = std::env::temp_dir().join(format!(
        "peachy-color-migrate-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&folder).unwrap();
    let path = folder.join("original.peachy");
    let original = serde_json::to_vec_pretty(&legacy(6)).unwrap();
    fs::write(&path, &original).unwrap();
    let doc = load(&path).unwrap();
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
    save(&path, &doc, &folder).unwrap();
    let backup = fs::read_dir(&folder)
        .unwrap()
        .filter_map(Result::ok)
        .find(|entry| entry.file_name().to_string_lossy().contains("backup-"))
        .unwrap();
    assert_eq!(fs::read(backup.path()).unwrap(), original);
    let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["format"], FORMAT_VERSION);
    assert_eq!(saved["slots"][0]["color"], json!([255, 255, 255, 255]));
    assert_eq!(load(&path).unwrap(), doc);
    fs::remove_dir_all(folder).unwrap();
}
