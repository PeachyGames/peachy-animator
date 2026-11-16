//! Original naming/reference cases, calibrated with installed Spine 4.3.26
//! import/export (November 14, 2026). Default explicit names can rename lookup
//! placeholders and clear Setup/swap keys; named-skin overrides preserve them.
//! Runtime data retains both identities. No proprietary implementation is used.
use peachy_document::{Attachment, AttachmentRef, Document};
use peachy_spine::{ExportTarget, export_json, export_json_for, import_json};
use serde_json::{Value, json};

fn region(skin: &str, name: Option<&str>) -> Value {
    let mut attachment = json!({"path":"body","width":20,"height":20});
    if let Some(name) = name {
        attachment["name"] = json!(name);
    }
    json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}],
    "slots":[{"name":"s","bone":"root","attachment":"ph"}],
    "skins":[{"name":skin,"attachments":{"s":{"ph":attachment}}}],
    "animations":{"swap":{"slots":{"s":{"attachment":[
        {"name":"ph"},{"time":0.25,"name":"pretty"},{"time":0.5},{"time":1,"name":"ph"}
    ]}}}}})
}

fn attachments<'a>(output: &'a Value, skin: &str, slot: &str) -> &'a Value {
    &output["skins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == skin)
        .unwrap()["attachments"][slot]
}

fn roundtrip(doc: &Document, target: ExportTarget) -> Value {
    let output = export_json_for(doc, target).unwrap();
    assert_eq!(import_json(&output, &doc.name).unwrap(), *doc);
    output
}

#[test]
fn export_targets_keep_region_names_placeholders_setup_and_unresolved_keys_distinct() {
    assert_eq!(ExportTarget::default(), ExportTarget::SpineEditor43);
    for (skin, name) in [
        ("default", None),
        ("default", Some("ph")),
        ("default", Some("pretty")),
        ("variant", Some("pretty")),
    ] {
        let doc = import_json(&region(skin, name), "Names").unwrap();
        let output = roundtrip(&doc, ExportTarget::Runtime43);
        let value = &attachments(&output, skin, "s")["ph"];
        assert_eq!(value.get("name").and_then(Value::as_str), name);
        assert_eq!(value["path"], "body");
        assert_eq!(output["slots"][0]["attachment"], "ph");
        assert_eq!(
            output["animations"]["swap"]["slots"]["s"]["attachment"][1]["name"],
            "pretty"
        );
        assert!(
            doc.resolve_attachment(skin, 0, "pretty").is_none(),
            "An explicit object name must not resolve an unknown placeholder"
        );
        if skin == "default" && name == Some("pretty") {
            let expected = "Spine Editor export: attachment default/s/ph has explicit name \"pretty\" different from placeholder \"ph\"; export as Runtime43 to preserve both identities";
            assert_eq!(export_json(&doc).unwrap_err(), expected);
            assert_eq!(
                export_json_for(&doc, ExportTarget::SpineEditor43).unwrap_err(),
                expected
            );
        } else {
            assert_eq!(roundtrip(&doc, ExportTarget::SpineEditor43), output);
            assert_eq!(export_json(&doc).unwrap(), output);
        }
    }
}

#[test]
fn export_targets_keep_runtime_skin_fallback_while_editor_retains_its_collision_gate() {
    let mut input = region("default", None);
    input["skins"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"variant","attachments":{
            "s":{"ph":{"name":"variant display","path":"variant","width":20,"height":20}}
        }}));
    let doc = import_json(&input, "Fallback").unwrap();
    assert_eq!(
        doc.resolve_attachment("default", 0, "ph").unwrap().1.path(),
        Some("body")
    );
    assert_eq!(
        doc.resolve_attachment("variant", 0, "ph").unwrap().1.path(),
        Some("variant")
    );
    assert_eq!(
        doc.resolve_attachment("missing-skin", 0, "ph")
            .unwrap()
            .1
            .path(),
        Some("body")
    );
    let output = roundtrip(&doc, ExportTarget::Runtime43);
    assert!(attachments(&output, "default", "s").get("ph").is_some());
    assert!(attachments(&output, "variant", "s").get("ph").is_some());
    assert_eq!(
        export_json(&doc).unwrap_err(),
        "Spine Editor export: skin placeholder s/ph collides with a direct default attachment"
    );
}

fn meshes(weighted: bool) -> Value {
    let vertices = if weighted {
        json!([1, 0, 0, 0, 1, 1, 0, 10, 0, 1, 1, 0, 10, 10, 1])
    } else {
        json!([0, 0, 10, 0, 10, 10])
    };
    json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}],
    "slots":[{"name":"s","bone":"root","attachment":"ph"}],
    "skins":[
        {"name":"default","attachments":{"s":{"ph":{
            "type":"mesh","name":"pretty source","path":"body","uvs":[0,0,1,0,1,1],
            "triangles":[0,1,2],"vertices":vertices,"hull":3,"width":10,"height":10
        }}}},
        {"name":"variant","attachments":{"s":{
            "linked":{"type":"linkedmesh","name":"pretty link","source":"ph","path":"other"},
            "independent":{"type":"linkedmesh","source":"ph","timelines":false,"path":"third"}
        }}}
    ],
    "animations":{"deform":{"slots":{"s":{"attachment":[
        {"name":"ph"},{"time":0.25,"name":"linked"},{"time":0.5,"name":"independent"}
    ]}},"attachments":{
        "default":{"s":{"ph":{"deform":[{},{"time":1,"offset":2,"vertices":[3,4]}]}}},
        "variant":{"s":{"independent":{"deform":[{},{"time":1,"vertices":[5,6]}]}}}
    }}}})
}

#[test]
fn export_targets_preserve_mesh_names_link_sources_and_shared_or_independent_deforms() {
    for weighted in [false, true] {
        let doc = import_json(&meshes(weighted), "Mesh names").unwrap();
        let source = AttachmentRef {
            skin: "default".into(),
            slot: "s".into(),
            placeholder: "ph".into(),
        };
        let linked = AttachmentRef {
            skin: "variant".into(),
            slot: "s".into(),
            placeholder: "linked".into(),
        };
        let independent = AttachmentRef {
            placeholder: "independent".into(),
            ..linked.clone()
        };
        assert_eq!(doc.deform_target(&linked), source);
        assert_eq!(doc.deform_target(&independent), independent);
        let output = roundtrip(&doc, ExportTarget::Runtime43);
        assert_eq!(
            attachments(&output, "default", "s")["ph"]["name"],
            "pretty source"
        );
        assert_eq!(
            attachments(&output, "variant", "s")["linked"]["source"],
            "ph"
        );
        assert_eq!(
            attachments(&output, "variant", "s")["independent"]["timelines"],
            false
        );
        let tracks = &output["animations"]["deform"]["attachments"];
        assert!(tracks["default"]["s"].get("ph").is_some());
        assert!(tracks["default"]["s"].get("pretty source").is_none());
        assert!(tracks["variant"]["s"].get("independent").is_some());
        assert_eq!(
            export_json(&doc).unwrap_err(),
            "Spine Editor export: attachment default/s/ph has explicit name \"pretty source\" different from placeholder \"ph\"; export as Runtime43 to preserve both identities"
        );
    }
}

fn sequence_path_clip() -> Value {
    json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}],
    "slots":[{"name":"c","bone":"root","attachment":"clip"},
             {"name":"s","bone":"root","attachment":"seq"},
             {"name":"p","bone":"root","attachment":"rail"}],
    "skins":[{"name":"default","attachments":{
        "c":{"clip":{"type":"clipping","name":"pretty clip","end":"s","vertexCount":4,"vertices":[-5,-5,5,-5,5,5,-5,5]}},
        "s":{"seq":{"name":"pretty sequence","path":"body","width":10,"height":10,"sequence":{"count":2,"digits":1}}},
        "p":{"rail":{"type":"path","name":"pretty path","vertexCount":6,"vertices":[-10,0,0,0,30,80,70,-40,100,10,110,10],"lengths":[1,2]}}
    }}],
    "animations":{"anim":{"slots":{"c":{"attachment":[{"time":1}]}},"attachments":{"default":{
        "c":{"clip":{"deform":[{},{"time":1,"offset":2,"vertices":[3,4]}]}},
        "s":{"seq":{"sequence":[{"mode":"loop","delay":0.25},{"time":1,"index":1}]}},
        "p":{"rail":{"deform":[{},{"time":1,"vertices":[1,2]}]}}
    }}}}})
}

#[test]
fn export_targets_preserve_sequence_path_clip_names_and_all_attachment_reference_paths() {
    let mut input = sequence_path_clip();
    let doc = import_json(&input, "Other vertex names").unwrap();
    let output = roundtrip(&doc, ExportTarget::Runtime43);
    let tracks = &output["animations"]["anim"]["attachments"]["default"];
    assert!(tracks["s"]["seq"].get("sequence").is_some());
    assert!(tracks["p"]["rail"].get("deform").is_some());
    assert!(tracks["c"]["clip"].get("deform").is_some());
    assert_eq!(attachments(&output, "default", "c")["clip"]["end"], "s");
    assert_eq!(
        doc.slots
            .iter()
            .map(|s| s.attachment.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("clip"), Some("seq"), Some("rail")]
    );
    assert!(export_json(&doc).unwrap_err().contains("default/c/clip"));
    for (slot, placeholder) in [("c", "clip"), ("s", "seq"), ("p", "rail")] {
        input["skins"][0]["attachments"][slot][placeholder]["name"] = json!(placeholder);
    }
    let matching = import_json(&input, "Matching vertex names").unwrap();
    roundtrip(&matching, ExportTarget::SpineEditor43);
}

#[test]
fn export_targets_share_document_and_supported_source_validation() {
    let good = import_json(&sequence_path_clip(), "Validation").unwrap();
    let mut bad_document = good.clone();
    bad_document.slots[0].bone = usize::MAX;
    let mut unsupported = good;
    let Attachment::Path(path) = unsupported
        .skins
        .get_mut("default")
        .unwrap()
        .get_mut("p")
        .unwrap()
        .get_mut("rail")
        .unwrap()
    else {
        panic!("path")
    };
    path.closed = true;
    unsupported.validate().unwrap();
    for target in [ExportTarget::SpineEditor43, ExportTarget::Runtime43] {
        assert!(
            export_json_for(&bad_document, target)
                .unwrap_err()
                .contains("Invalid slot")
        );
        assert!(
            export_json_for(&unsupported, target)
                .unwrap_err()
                .contains("closed paths")
        );
    }
}
