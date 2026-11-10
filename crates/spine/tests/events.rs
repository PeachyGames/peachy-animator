//! Event definitions and keys round-trip exactly as written.
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}],"slots":[],"skins":[],
        "events":{"bare":{},"full":{"int":3,"float":1.5,"string":"hello","audio":"sfx/hit.wav","volume":0.5,"balance":-0.25}},
        "animations":{"anim":{"events":[{"time":0.25,"name":"bare"},{"time":0.5,"name":"full"},
            {"time":1,"name":"bare","int":1},{"time":1,"name":"full","int":7,"float":-2,"string":"over","volume":0.8,"balance":0.3}]}}})
}

#[test]
fn definitions_and_keys_roundtrip_with_overrides_only_where_written() {
    let input = fixture();
    assert!(
        inspect(&input).supported,
        "{:?}",
        inspect(&input).unsupported
    );
    let doc = import_json(&input, "Events").unwrap();
    assert_eq!(doc.events.len(), 2);
    let full = doc.events.iter().find(|e| e.name == "full").unwrap();
    assert_eq!(
        (full.int, full.audio.as_deref(), full.volume),
        (3, Some("sfx/hit.wav"), 0.5)
    );
    let keys = &doc.animations[0].events;
    assert_eq!(keys.len(), 4);
    assert_eq!((keys[0].int, keys[2].int), (None, Some(1)));
    // Events extend the animation like any other key.
    assert_eq!(doc.animations[0].duration(), 1.);
    let output = export_json(&doc).unwrap();
    assert_eq!(output["events"]["bare"], json!({}));
    assert_eq!(
        output["animations"]["anim"]["events"][0],
        json!({"time":0.25,"name":"bare"})
    );
    assert_eq!(import_json(&output, "Events").unwrap(), doc);
}

#[test]
fn malformed_events_are_refused() {
    for (patch, reason) in [
        (json!({"events":{"bad":{"int":1.5}}}), "32-bit integer"),
        (
            json!({"events":{"bad":{"color":"ff"}}}),
            "unsupported field color",
        ),
        (
            json!({"animations":{"anim":{"events":[{"time":0,"name":"ghost"}]}}}),
            "unknown event ghost",
        ),
        (
            json!({"animations":{"anim":{"events":[{"time":1,"name":"bare"},{"time":0.5,"name":"bare"}]}}}),
            "nondecreasing",
        ),
    ] {
        let mut input = fixture();
        for (key, value) in patch.as_object().unwrap() {
            input[key] = value.clone();
        }
        let error = import_json(&input, "Bad").unwrap_err();
        assert!(error.contains(reason), "expected {reason:?}, got {error}");
    }
}
