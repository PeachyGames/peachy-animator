use peachy_document::{Curve, Document, Keyframe, Property};
use peachy_io::{decode_project, load, save};
use serde_json::{Value, json};
use std::{fs, path::PathBuf, time::SystemTime};

const LEGACY: &str = include_str!("fixtures/legacy-v2-duration.peachy");
const GOLDEN: &str = include_str!("fixtures/legacy-v2-export.json");

fn legacy() -> Value {
    serde_json::from_str(LEGACY).unwrap()
}

fn decode(value: &Value) -> Result<Document, String> {
    decode_project(&value.to_string())
}

#[test]
fn migration_materializes_exactly_the_previous_export_without_reordering_tracks() {
    let doc = decode_project(LEGACY).unwrap();
    assert_eq!(doc.format, peachy_document::FORMAT_VERSION);
    assert_eq!(
        peachy_spine::export_json(&doc).unwrap(),
        serde_json::from_str::<Value>(GOLDEN).unwrap()
    );
    assert_eq!(
        doc.animations
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        [
            "reserved",
            "no_tracks",
            "empty_root",
            "empty_other",
            "at_end",
            "tiny"
        ]
    );
    let reserved = &doc.animations[0];
    assert_eq!(reserved.duration(), 4.0);
    assert_eq!(reserved.tracks.len(), 4);
    assert_eq!(
        reserved.tracks[0].keys[0].curve,
        Curve::Bezier([0.2, 0.1, 0.7, 0.9])
    );
    assert_eq!(reserved.tracks[0].keys[1].curve, Curve::Linear);
    assert_eq!(
        reserved.tracks[0].keys[2],
        Keyframe {
            time: 4.0,
            value: 8.0,
            curve: Curve::Linear
        }
    );
    assert_eq!(reserved.tracks[1].keys[1].curve, Curve::Stepped);
    assert!(reserved.tracks[3].keys.is_empty());
    // A final curve stays unused when there is no new hold to append.
    assert!(matches!(
        doc.animations[4].tracks[0].keys[1].curve,
        Curve::Bezier(_)
    ));
    assert_eq!(doc.animations[5].duration(), 0.001);

    // Existing empty root rotation is filled in place; other empty tracks survive.
    let empty_root = &doc.animations[2];
    assert_eq!(empty_root.tracks.len(), 3);
    assert_eq!(empty_root.tracks[0].property, Property::X);
    assert!(empty_root.tracks[0].keys.is_empty());
    assert_eq!(empty_root.tracks[1].property, Property::Rotation);
    assert_eq!(
        empty_root.tracks[1].keys,
        [Keyframe {
            time: 2.0,
            value: 0.0,
            curve: Curve::Linear
        }]
    );
    assert!(empty_root.tracks[2].keys.is_empty());
    assert_eq!(doc.animations[1].tracks.len(), 1);
    assert_eq!(doc.animations[3].tracks.len(), 2);
    assert_eq!(doc.animations[3].tracks[1].bone, 0);
    assert_eq!(doc.animations[3].tracks[1].property, Property::Rotation);

    let native = serde_json::to_value(&doc).unwrap();
    assert!(
        native["animations"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a.get("duration").is_none())
    );
    assert_eq!(decode(&native).unwrap(), doc);
}

#[test]
fn invalid_legacy_duration_or_keys_are_rejected_before_materialization() {
    for bad in [
        json!(0),
        json!(-1),
        json!(1e100),
        json!(1e-100),
        json!("4"),
        Value::Null,
    ] {
        let mut value = legacy();
        value["animations"][1]["duration"] = bad.clone();
        assert!(decode(&value).is_err(), "duration {bad}");
    }
    let mut missing = legacy();
    missing["animations"][0]
        .as_object_mut()
        .unwrap()
        .remove("duration");
    assert!(decode(&missing).is_err());
    for (path, bad) in [
        ("/animations/0/tracks/0/keys/1/time", json!(5)),
        ("/animations/0/tracks/0/keys/1/time", json!(-1)),
        ("/animations/0/tracks/0/keys/1/time", json!(0)),
        ("/animations/0/tracks/0/keys/0/time", json!(2)),
        ("/animations/0/tracks/0/keys/1/value", json!(1e100)),
        (
            "/animations/0/tracks/0/keys/1/curve",
            json!({"bezier":[1.2,0,0.2,1]}),
        ),
        (
            "/animations/0/tracks/0/keys/1/curve",
            json!({"bezier":[0.2,0,0.8,1e100]}),
        ),
        ("/animations/0/tracks/0/keys/1/curve", json!("unknown")),
        ("/animations/2/tracks/0/bone", json!(99)),
    ] {
        let mut value = legacy();
        *value.pointer_mut(path).unwrap() = bad;
        assert!(decode(&value).is_err(), "{path}");
    }
    let mut duplicate = legacy();
    let track = duplicate["animations"][0]["tracks"][0].clone();
    duplicate["animations"][0]["tracks"]
        .as_array_mut()
        .unwrap()
        .push(track);
    assert!(decode(&duplicate).is_err());
}

#[test]
fn migration_keeps_unknown_fields_strict_even_on_final_curves_and_empty_tracks() {
    for path in [
        "",
        "/animations/0",
        "/animations/2/tracks/0",
        "/animations/0/tracks/0/keys/1",
        "/bones/0",
        "/bones/0/setup",
    ] {
        let mut value = legacy();
        value.pointer_mut(path).unwrap()["future"] = json!(true);
        assert!(decode(&value).is_err(), "{path}");
    }
    let mut value = legacy();
    value["animations"][0]["tracks"][0]["keys"][1]["curve"]["future"] = json!(true);
    assert!(decode(&value).is_err());

    let doc = decode_project(LEGACY).unwrap();
    let mut native = serde_json::to_value(doc).unwrap();
    native["animations"][0]["duration"] = json!(4);
    assert!(
        decode(&native)
            .unwrap_err()
            .contains("unknown field `duration`")
    );
    native["format"] = json!(peachy_document::FORMAT_VERSION + 1);
    native["animations"][0]
        .as_object_mut()
        .unwrap()
        .remove("duration");
    assert!(decode(&native).is_err());
}

fn add_region(value: &mut Value, tagged: bool) {
    value["slots"] = json!([{"name":"body", "bone":0, "attachment":"shared"}]);
    let mut region = json!({"path":"body", "width":10, "height":10, "transform":{
        "x":0, "y":0, "rotation":0, "scale_x":1, "scale_y":1
    }});
    if tagged {
        region["type"] = json!("region");
    }
    value["skins"] = json!({"default":{"body":{"shared":region}}});
}

#[test]
fn v1_regions_follow_both_migrations_and_reject_mixed_or_unknown_attachment_fields() {
    let mut v2 = legacy();
    add_region(&mut v2, true);
    let expected = decode(&v2).unwrap();
    let mut v1 = legacy();
    v1["format"] = json!(1);
    add_region(&mut v1, false);
    assert_eq!(decode(&v1).unwrap(), expected);
    let mut mixed = v1.clone();
    mixed["skins"]["default"]["body"]["shared"]["type"] = json!("region");
    assert!(decode(&mixed).is_err());
    v1["skins"]["default"]["body"]["shared"]["vertices"] = json!([]);
    assert!(decode(&v1).is_err());
}

#[test]
fn native_migration_does_not_depend_on_export_target_restrictions() {
    let mut value = legacy();
    add_region(&mut value, true);
    value["skins"]["variant"] = value["skins"]["default"].clone();
    let doc = decode(&value).unwrap();
    assert_eq!(doc.skins.len(), 2);
    assert!(doc.validate().is_ok());
    assert!(peachy_spine::export_json(&doc).is_err());
}

#[test]
fn legacy_load_is_read_only_then_explicit_save_backs_up_original_and_keeps_the_reserved_hold() {
    let folder: PathBuf = std::env::temp_dir().join(format!(
        "peachy-migrate-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&folder).unwrap();
    let path = folder.join("legacy.peachy");
    fs::write(&path, LEGACY).unwrap();
    let mut doc = load(&path).unwrap();
    assert_eq!(fs::read(&path).unwrap(), LEGACY.as_bytes());
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
    doc.animations[0].tracks[0].keys.push(Keyframe {
        time: 6.0,
        value: 12.0,
        curve: Curve::Linear,
    });
    assert_eq!(doc.animations[0].duration(), 6.0);
    save(&path, &doc, &folder).unwrap();
    let backup = fs::read_dir(&folder)
        .unwrap()
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().contains("backup-"))
        .unwrap();
    assert_eq!(fs::read(backup.path()).unwrap(), LEGACY.as_bytes());
    let mut reopened = load(&path).unwrap();
    assert_eq!(reopened, doc);
    let written: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(written["format"], peachy_document::FORMAT_VERSION);
    assert!(written["animations"][0].get("duration").is_none());
    reopened.animations[0].tracks[0].keys.pop();
    assert_eq!(reopened.animations[0].duration(), 4.0);
    save(&path, &reopened, &folder).unwrap();
    assert_eq!(load(&path).unwrap(), reopened);
    assert_eq!(load(&path).unwrap().animations[0].duration(), 4.0);
    fs::remove_dir_all(folder).unwrap();
}
