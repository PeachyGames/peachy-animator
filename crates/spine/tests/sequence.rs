//! Region/mesh sequences, sequence keys (delay carries forward) and blend modes.
use peachy_document::{Attachment, BlendMode, SequenceMode};
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}],
        "slots":[{"name":"s","bone":"root","attachment":"f","blend":"screen"},{"name":"t","bone":"root","blend":"additive"}],
        "skins":[{"name":"default","attachments":{"s":{"f":{"width":8,"height":8,"sequence":{"count":12,"digits":2,"setup":3}}}}}],
        "animations":{"a":{"attachments":{"default":{"s":{"f":{"sequence":[
            {"mode":"loop","delay":0.25},{"time":1,"mode":"pingpong","index":4},{"time":2,"delay":0}]}}}}}}})
}

#[test]
fn sequences_keys_and_blend_modes_roundtrip_with_runtime_defaults() {
    let input = fixture();
    assert!(
        inspect(&input).supported,
        "{:?}",
        inspect(&input).unsupported
    );
    let doc = import_json(&input, "Sequence").unwrap();
    assert_eq!(
        (doc.slots[0].blend, doc.slots[1].blend),
        (BlendMode::Screen, BlendMode::Additive)
    );
    let Attachment::Region(region) = &doc.skins["default"]["s"]["f"] else {
        panic!("region")
    };
    let sequence = region.sequence.unwrap();
    assert_eq!(
        (
            sequence.count,
            sequence.start,
            sequence.digits,
            sequence.setup_index
        ),
        (12, 1, 2, 3)
    );
    let keys = &doc.animations[0].sequence_tracks[0].keys;
    // The second key carries the first key's delay; mode and index do not carry.
    assert_eq!(
        (keys[1].delay, keys[1].mode),
        (0.25, SequenceMode::Pingpong)
    );
    assert_eq!(
        (keys[2].delay, keys[2].mode, keys[2].index),
        (0., SequenceMode::Hold, 0)
    );
    assert!(doc.image_paths().contains("f04"));
    assert_eq!(
        import_json(&export_json(&doc).unwrap(), "Sequence").unwrap(),
        doc
    );
}

#[test]
fn malformed_sequences_and_blends_are_refused() {
    for (target, value, reason) in [
        ("blend", json!("overlay"), "blend must be"),
        ("sequence", json!({"count":2,"setup":2}), "setup index"),
        ("index", json!(12), "outside the sequence"),
    ] {
        let mut input = fixture();
        match target {
            "blend" => input["slots"][0]["blend"] = value,
            "sequence" => input["skins"][0]["attachments"]["s"]["f"]["sequence"] = value,
            _ => {
                input["animations"]["a"]["attachments"]["default"]["s"]["f"]["sequence"][0]["index"] =
                    value
            }
        }
        let error = import_json(&input, "Bad").unwrap_err();
        assert!(error.contains(reason), "{target}: {error}");
    }
}
