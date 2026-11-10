//! Native8 → native9 adds empty IK animation tracks after physics data. Older
//! formats cannot carry IK tracks, and their static IK Mix stays within 0–1.
use peachy_document::FORMAT_VERSION;
use peachy_io::decode_project;
use serde_json::{Value, json};

fn v8() -> Value {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../examples/peach-bot/ik-chain-bot.peachy"
    ))
    .unwrap();
    value["format"] = json!(8);
    for animation in value["animations"].as_array_mut().unwrap() {
        for field in ["ik_tracks", "inherit_tracks"] {
            assert_eq!(
                animation.as_object_mut().unwrap().remove(field),
                Some(json!([]))
            );
        }
    }
    // Native10 inheritance fields did not exist yet.
    for bone in value["bones"].as_array_mut().unwrap() {
        assert_eq!(
            bone.as_object_mut().unwrap().remove("inherit"),
            Some(json!("normal"))
        );
    }
    value
}

#[test]
fn native8_gains_empty_ik_tracks_and_equals_the_current_example() {
    let migrated = decode_project(&v8().to_string()).unwrap();
    assert_eq!(migrated.format, FORMAT_VERSION);
    assert!(migrated.animations.iter().all(|a| a.ik_tracks.is_empty()));
    let current = decode_project(include_str!(
        "../../../examples/peach-bot/ik-chain-bot.peachy"
    ))
    .unwrap();
    assert_eq!(migrated, current);
}

#[test]
fn legacy_ik_tracks_and_out_of_range_static_mix_are_refused() {
    for version in [6, 7, 8] {
        let mut value = v8();
        value["format"] = json!(version);
        if version < 8 {
            for animation in value["animations"].as_array_mut().unwrap() {
                let object = animation.as_object_mut().unwrap();
                object.remove("physics");
                object.remove("physics_resets");
            }
        }
        if version < 7 {
            for slot in value["slots"].as_array_mut().unwrap() {
                slot.as_object_mut().unwrap().remove("color");
            }
        }
        assert!(decode_project(&value.to_string()).is_ok(), "v{version}");
        let mut with_tracks = value.clone();
        with_tracks["animations"][0]["ik_tracks"] = json!([]);
        assert!(
            decode_project(&with_tracks.to_string())
                .unwrap_err()
                .contains("cannot contain IK animation tracks"),
            "v{version}"
        );
        let mut overrange = value;
        overrange["constraints"][0]["setup"]["mix"] = json!(1.5);
        assert!(
            decode_project(&overrange.to_string())
                .unwrap_err()
                .contains("within 0–1"),
            "v{version}"
        );
    }
    // Native9 itself accepts signed or overshooting static Mix.
    let mut current: Value = serde_json::from_str(include_str!(
        "../../../examples/peach-bot/ik-chain-bot.peachy"
    ))
    .unwrap();
    current["constraints"][0]["setup"]["mix"] = json!(1.5);
    assert!(decode_project(&current.to_string()).is_ok());
}
