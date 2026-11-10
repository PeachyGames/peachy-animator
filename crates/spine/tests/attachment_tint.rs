//! Attachment colors (RGB or RGBA hex; omitted is white) round-trip on
//! regions and meshes, and tint draws together with the slot color.
use peachy_document::{Attachment, WHITE};
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}],
        "slots":[{"name":"a","bone":"root","attachment":"r"},{"name":"b","bone":"root","attachment":"m","color":"80ff40c0"}],
        "skins":[{"name":"default","attachments":{
            "a":{"r":{"width":10,"height":10,"color":"3835f5"},"plain":{"width":4,"height":4}},
            "b":{"m":{"type":"mesh","uvs":[0,0,1,0,1,1],"triangles":[0,1,2],"vertices":[0,0,10,0,10,10],"hull":3,"color":"3835F580"}}}}],
        "animations":{"anim":{}}})
}

fn attachment<'a>(doc: &'a peachy_document::Document, slot: &str, name: &str) -> &'a Attachment {
    let slot = doc.slots.iter().position(|s| s.name == slot).unwrap();
    doc.resolve_attachment("default", slot, name).unwrap().1
}

#[test]
fn colors_import_as_bytes_and_export_as_lowercase_rgba() {
    let input = fixture();
    assert!(
        inspect(&input).supported,
        "{:?}",
        inspect(&input).unsupported
    );
    let doc = import_json(&input, "Tint").unwrap();
    assert_eq!(attachment(&doc, "a", "r").color(), [0x38, 0x35, 0xf5, 0xff]);
    assert_eq!(attachment(&doc, "a", "plain").color(), WHITE);
    assert_eq!(attachment(&doc, "b", "m").color(), [0x38, 0x35, 0xf5, 0x80]);
    let output = export_json(&doc).unwrap();
    let skin = &output["skins"][0]["attachments"];
    assert_eq!(skin["a"]["r"]["color"], json!("3835f5ff"));
    assert_eq!(skin["b"]["m"]["color"], json!("3835f580"));
    assert!(skin["a"]["plain"].get("color").is_none());
    assert_eq!(import_json(&output, "Tint").unwrap(), doc);
}

#[test]
fn malformed_attachment_colors_are_refused() {
    for bad in [json!("3835f"), json!("zz35f5ff"), json!(7)] {
        let mut input = fixture();
        input["skins"][0]["attachments"]["a"]["r"]["color"] = bad.clone();
        assert!(!inspect(&input).supported, "{bad}");
        assert!(import_json(&input, "Bad").is_err(), "{bad}");
    }
}
