//! Frozen original v22 examples stay byte-for-byte intact; the file shell
//! upgrades them while current-schema serde remains strict.
use peachy_document::{Constraint, Document, FORMAT_VERSION, SliderConstraint};
use peachy_io::decode_project;
use serde_json::{Value, json};

const V22: &str = include_str!("../../../examples/peach-bot/transform-mapping-bot.peachy");

#[test]
fn slider_v22_migration_adds_only_required_empty_tracks_and_version() {
    let original: Value = serde_json::from_str(V22).unwrap();
    assert_eq!(original["format"], 22);
    let migrated = decode_project(V22).unwrap();
    assert_eq!(migrated.format, FORMAT_VERSION);
    assert!(
        migrated
            .animations
            .iter()
            .all(|clip| clip.slider_tracks.is_empty())
    );
    let mut written = serde_json::to_value(&migrated).unwrap();
    written["format"] = 22.into();
    for clip in written["animations"].as_array_mut().unwrap() {
        assert_eq!(
            clip.as_object_mut().unwrap().remove("slider_tracks"),
            Some(json!([]))
        );
    }
    assert_eq!(written, original);
    assert!(serde_json::from_str::<Document>(V22).is_err());
    assert_eq!(
        decode_project(&serde_json::to_string(&migrated).unwrap()).unwrap(),
        migrated
    );
}

#[test]
fn slider_legacy_guards_reject_even_empty_null_or_disguised_new_data() {
    let original: Value = serde_json::from_str(V22).unwrap();
    for version in 1..=22 {
        for data in [
            Value::Null,
            json!([]),
            json!([{"constraint":0,"property":"time","keys":[]}]),
        ] {
            let mut value = original.clone();
            value["format"] = version.into();
            value["animations"][0]["slider_tracks"] = data;
            assert!(
                decode_project(&value.to_string())
                    .unwrap_err()
                    .contains("cannot contain Slider")
            );
        }
        let mut value = original.clone();
        value["format"] = version.into();
        value["constraints"] = json!([{"type":"slider"}]);
        assert!(
            decode_project(&value.to_string())
                .unwrap_err()
                .contains("cannot contain Slider")
        );
    }
}

#[test]
fn slider_current_schema_requires_complete_tracks_and_nullable_fields() {
    let mut doc = decode_project(V22).unwrap();
    doc.constraints
        .push(Constraint::Slider(SliderConstraint::new("manual", 0)));
    let value = serde_json::to_value(&doc).unwrap();
    assert_eq!(decode_project(&value.to_string()).unwrap(), doc);
    let last = doc.constraints.len() - 1;
    for field in ["bone", "property", "from", "to", "scale", "max", "setup"] {
        let mut bad = value.clone();
        bad["constraints"][last]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(decode_project(&bad.to_string()).is_err(), "missing {field}");
    }
    for data in [None, Some(Value::Null)] {
        let mut bad = value.clone();
        if let Some(data) = data {
            bad["animations"][0]["slider_tracks"] = data;
        } else {
            bad["animations"][0]
                .as_object_mut()
                .unwrap()
                .remove("slider_tracks");
        }
        assert!(decode_project(&bad.to_string()).is_err());
    }
    let mut bad = value;
    bad["constraints"][last]["bone"] = 0.into();
    assert!(
        decode_project(&bad.to_string())
            .unwrap_err()
            .contains("needs a property")
    );
}
