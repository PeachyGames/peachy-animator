//! Clipping attachments: end slot, convex flag, plain or weighted polygons,
//! nonessential color; inverse clipping and malformed polygons are refused.
use peachy_document::{Attachment, MeshVertices};
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"},{"name":"b","parent":"root"}],
        "slots":[{"name":"c","bone":"root","attachment":"clip"},{"name":"s","bone":"root"},{"name":"w","bone":"root","attachment":"weighted"}],
        "skins":[{"name":"default","attachments":{
            "c":{"clip":{"type":"clipping","end":"s","vertexCount":4,"vertices":[-5,-5,5,-5,5,5,-5,5],"color":"ce3a3aff"}},
            "w":{"weighted":{"type":"clipping","convex":true,"vertexCount":3,"vertices":[1,0,0,0,1, 2,0,10,0,0.5,1,10,0,0.5, 1,1,0,10,1]}}}}],
        "animations":{"anim":{}}})
}

#[test]
fn clipping_attachments_roundtrip() {
    let input = fixture();
    assert!(
        inspect(&input).supported,
        "{:?}",
        inspect(&input).unsupported
    );
    let doc = import_json(&input, "Clip").unwrap();
    let Attachment::Clipping(clip) = &doc.skins["default"]["c"]["clip"] else {
        panic!("clip")
    };
    assert_eq!(
        (clip.end, clip.convex, clip.color),
        (Some(1), false, Some([0xce, 0x3a, 0x3a, 0xff]))
    );
    let Attachment::Clipping(weighted) = &doc.skins["default"]["w"]["weighted"] else {
        panic!("clip")
    };
    assert!(matches!(weighted.vertices, MeshVertices::Weighted { .. }));
    assert_eq!((weighted.end, weighted.convex), (None, true));
    let output = export_json(&doc).unwrap();
    let clip_json = &output["skins"][0]["attachments"]["c"]["clip"];
    assert_eq!(clip_json["end"], json!("s"));
    assert!(clip_json.get("path").is_none() && clip_json.get("convex").is_none());
    assert_eq!(import_json(&output, "Clip").unwrap(), doc);
}

#[test]
fn unsupported_or_malformed_clipping_is_refused() {
    for (patch, reason) in [
        (json!({"inverse":true}), "inverse clipping"),
        (json!({"end":"ghost"}), "end must name a slot"),
        (json!({"vertexCount":5}), "vertex count"),
        (json!({"vertexCount":2,"vertices":[0,0,1,1]}), "at least 3"),
        (json!({"path":"x"}), "unsupported field path"),
    ] {
        let mut input = fixture();
        for (key, value) in patch.as_object().unwrap() {
            input["skins"][0]["attachments"]["c"]["clip"][key] = value.clone();
        }
        let error = import_json(&input, "Bad").unwrap_err();
        assert!(
            error.to_lowercase().contains(reason),
            "expected {reason:?}, got {error}"
        );
    }
}
