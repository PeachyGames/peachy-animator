use peachy_document::{Constraint, Document, FORMAT_VERSION};
use peachy_io::decode_project;
use serde_json::{Value, json};
use std::{fs, path::Path};

fn decode(value: &Value) -> Result<Document, String> {
    decode_project(&value.to_string())
}

fn legacy(version: u32) -> Value {
    if version == 6 {
        return serde_json::from_str(include_str!(
            "fixtures/native-v6-zero-ik-animation/ik-bot.peachy"
        ))
        .unwrap();
    }
    let mut value: Value =
        serde_json::from_str(include_str!("fixtures/native-v5-zero-ik/peach-bot.peachy")).unwrap();
    value["format"] = json!(version);
    if version <= 4 {
        for bone in value["bones"].as_array_mut().unwrap() {
            for field in ["shear_x", "shear_y"] {
                assert_eq!(bone["setup"][field].as_f64(), Some(0.0));
                bone["setup"].as_object_mut().unwrap().remove(field);
            }
        }
    }
    if version <= 2 {
        for animation in value["animations"].as_array_mut().unwrap() {
            animation["duration"] = json!(
                animation["tracks"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .flat_map(|t| t["keys"].as_array().unwrap())
                    .map(|k| k["time"].as_f64().unwrap())
                    .fold(0.0_f64, f64::max)
            );
        }
    }
    if version == 1 {
        for skin in value["skins"].as_object_mut().unwrap().values_mut() {
            for slot in skin.as_object_mut().unwrap().values_mut() {
                for attachment in slot.as_object_mut().unwrap().values_mut() {
                    assert_eq!(
                        attachment.as_object_mut().unwrap().remove("type"),
                        Some(json!("region"))
                    );
                }
            }
        }
    }
    value
}

#[test]
fn every_actual_legacy_schema_rejects_even_empty_ik_track_presence_before_insertion() {
    for version in 1..=6 {
        let source = legacy(version);
        let doc = decode(&source).unwrap();
        assert_eq!(doc.format, FORMAT_VERSION);
        assert!(doc.animations.iter().all(|a| a.ik_tracks.is_empty()));
        for field in [
            json!([]),
            Value::Null,
            json!({}),
            json!([{"constraint":0,"keys":[]}]),
        ] {
            let mut bad = source.clone();
            bad["animations"][0]["ik_tracks"] = field;
            assert!(
                decode(&bad)
                    .unwrap_err()
                    .contains("Legacy project formats cannot contain IK animation tracks")
            );
        }
    }
}

#[test]
fn native6_keeps_its_typed_f32_range_before_native7_accepts_all_finite_mix() {
    for (value, expected) in [(json!(1.000000001), 1.0_f32), (json!(-1e-50), -0.0_f32)] {
        let mut old = legacy(6);
        old["constraints"][0]["setup"]["mix"] = value;
        let doc = decode(&old).unwrap();
        let Constraint::Ik(ik) = &doc.constraints[0];
        assert_eq!(ik.setup.mix.to_bits(), expected.to_bits());
    }
    for value in [
        json!(1.0000002),
        json!(-1e-40),
        json!(f32::MAX),
        Value::Null,
        json!("1"),
    ] {
        let mut old = legacy(6);
        old["constraints"][0]["setup"]["mix"] = value;
        assert!(decode(&old).is_err());
    }
    let mut latest = serde_json::to_value(decode(&legacy(6)).unwrap()).unwrap();
    for value in [-0.625_f32, 1.625, f32::MAX] {
        latest["constraints"][0]["setup"]["mix"] = json!(value);
        let doc = decode(&latest).unwrap();
        let Constraint::Ik(ik) = &doc.constraints[0];
        assert_eq!(ik.setup.mix, value);
    }
}

#[test]
fn nine_native6_migrations_and_latest_examples_preserve_full_export_bytes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let baseline = root.join("tests/fixtures/native-v6-zero-ik-animation");
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
        let original = fs::read_to_string(baseline.join(format!("{name}.peachy"))).unwrap();
        let golden = fs::read(baseline.join(format!("{name}.json"))).unwrap();
        let mut expected: Value = serde_json::from_str(&original).unwrap();
        expected["format"] = json!(FORMAT_VERSION);
        for animation in expected["animations"].as_array_mut().unwrap() {
            animation["ik_tracks"] = json!([]);
        }
        for text in [
            original,
            fs::read_to_string(root.join(format!("../../examples/peach-bot/{name}.peachy")))
                .unwrap(),
        ] {
            let doc = decode_project(&text).unwrap();
            assert_eq!(
                doc,
                serde_json::from_value::<Document>(expected.clone()).unwrap()
            );
            assert_eq!(
                serde_json::to_vec_pretty(&peachy_spine::export_json(&doc).unwrap()).unwrap(),
                golden,
                "Pre-IK-animation bytes changed: {name}"
            );
        }
    }
}

#[test]
fn latest_requires_typed_ik_tracks_and_legacy_malformed_animations_do_not_become_objects() {
    let latest = serde_json::to_value(decode(&legacy(6)).unwrap()).unwrap();
    let mut missing = latest.clone();
    missing["animations"][0]
        .as_object_mut()
        .unwrap()
        .remove("ik_tracks");
    assert!(
        decode(&missing)
            .unwrap_err()
            .contains("missing field `ik_tracks`")
    );
    for bad in [
        Value::Null,
        json!({}),
        json!([{"constraint":99,"keys":[]}]),
        json!([{"constraint":0,"keys":[],"future":0}]),
    ] {
        let mut value = latest.clone();
        value["animations"][0]["ik_tracks"] = bad;
        assert!(decode(&value).is_err());
    }
    for bad in [Value::Null, json!([]), json!(0)] {
        let mut old = legacy(6);
        old["animations"][0] = bad;
        assert!(decode(&old).is_err());
    }
}
