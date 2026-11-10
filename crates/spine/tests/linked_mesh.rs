//! `linkedmesh` attachments: sources in any skin (omitted = default), own
//! path and color, `timelines` flag; malformed links are refused.
use peachy_document::{Attachment, MeshLink};
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}],
        "slots":[{"name":"s","bone":"root","attachment":"m"}],
        "skins":[
            {"name":"default","attachments":{"s":{
                "m":{"type":"mesh","name":"msrc","uvs":[0,0,1,0,1,1],"triangles":[0,1,2],"vertices":[0,0,10,0,10,10],"hull":3,"width":10,"height":10},
                "k":{"type":"linkedmesh","name":"same-skin","source":"m"}}}},
            {"name":"alt","attachments":{"s":{
                "p":{"type":"linkedmesh","name":"other","skin":"z-later","source":"own","color":"00ff0080"},
                "n":{"type":"linkedmesh","path":"third","source":"m","timelines":false,"width":10,"height":10}}}},
            {"name":"z-later","attachments":{"s":{
                "own":{"type":"mesh","uvs":[0,0,1,0,1,1],"triangles":[0,1,2],"vertices":[0,0,20,0,20,20],"hull":3}}}}],
        "animations":{"anim":{}}})
}

fn link(
    doc: &peachy_document::Document,
    skin: &str,
    placeholder: &str,
) -> (String, [u8; 4], MeshLink) {
    let Attachment::Mesh(mesh) = &doc.skins[skin]["s"][placeholder] else {
        panic!("mesh")
    };
    (mesh.path.clone(), mesh.color, mesh.link.clone().unwrap())
}

#[test]
fn links_resolve_across_skins_and_roundtrip() {
    let input = fixture();
    assert!(
        inspect(&input).supported,
        "{:?}",
        inspect(&input).unsupported
    );
    let doc = import_json(&input, "Linked").unwrap();
    let (path, color, other) = link(&doc, "alt", "p");
    assert_eq!((path.as_str(), color), ("other", [0, 255, 0, 128]));
    assert_eq!(
        other,
        MeshLink {
            skin: "z-later".into(),
            source: "own".into(),
            timelines: true
        }
    );
    let (_, _, third) = link(&doc, "alt", "n");
    assert_eq!((third.skin.as_str(), third.timelines), ("default", false));
    assert_eq!(link(&doc, "default", "k").2.source, "m");
    let output = export_json(&doc).unwrap();
    let alt = &output["skins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|skin| skin["name"] == "alt")
        .unwrap()["attachments"]["s"];
    assert_eq!(alt["p"]["skin"], json!("z-later"));
    assert!(
        alt["n"].get("skin").is_none(),
        "default source skin is omitted"
    );
    assert_eq!(alt["n"]["timelines"], json!(false));
    assert!(alt["p"].get("vertices").is_none());
    assert_eq!(import_json(&output, "Linked").unwrap(), doc);
}

#[test]
fn malformed_links_are_refused() {
    for (patch, reason) in [
        (
            json!({"type":"linkedmesh","source":"ghost"}),
            "must be a mesh",
        ),
        (
            json!({"type":"linkedmesh","source":"k"}),
            "not a linked mesh",
        ),
        (
            json!({"type":"linkedmesh","source":"m","width":3}),
            "width must match",
        ),
        (
            json!({"type":"linkedmesh","source":"m","parent":"m"}),
            "unsupported field parent",
        ),
        (
            json!({"type":"linkedmesh","source":"m","timelines":1}),
            "true or false",
        ),
    ] {
        let mut input = fixture();
        input["skins"][1]["attachments"]["s"]["n"] = patch;
        let error = import_json(&input, "Bad").unwrap_err();
        assert!(error.contains(reason), "expected {reason:?}, got {error}");
        assert!(!inspect(&input).supported);
    }
}
