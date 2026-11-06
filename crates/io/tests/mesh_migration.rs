use peachy_document::{Attachment, MeshVertices};
use peachy_io::{decode_project, load, save};
use serde_json::{Value, json};
use std::{fs, time::SystemTime};

fn legacy_mesh() -> Value {
    let mut value: Value =
        serde_json::from_str(include_str!("fixtures/legacy-v2-duration.peachy")).unwrap();
    value["format"] = json!(3);
    value["animations"] = json!([]);
    value["slots"] = json!([{"name":"body","bone":0,"attachment":"cloth"}]);
    value["skins"] = json!({"default":{"body":{"cloth":{
        "type":"mesh", "path":"cloth", "vertices":[[0,0],[10,0],[0,10]],
        "uvs":[[0,0],[1,0],[0,1]], "triangles":[0,1,2], "hull":3,
        "width":16,"height":16,"edges":[0,2,2,4,4,0]
    }}}});
    value
}

fn decode(value: &Value) -> Result<peachy_document::Document, String> {
    decode_project(&value.to_string())
}

fn weighted_native() -> Value {
    let mut value = serde_json::to_value(decode(&legacy_mesh()).unwrap()).unwrap();
    value["skins"]["default"]["body"]["cloth"]["vertices"] = json!({
        "mode":"weighted", "influences":[
            [{"bone":0,"position":[0,0],"weight":0.4},{"bone":1,"position":[-10,0],"weight":0.59999}],
            [{"bone":1,"position":[0,0],"weight":1}],
            [{"bone":0,"position":[0,10],"weight":1}]
        ]
    });
    value
}

#[test]
fn v2_and_v3_mesh_arrays_migrate_to_slot_local_without_geometry_or_export_changes() {
    let v3 = legacy_mesh();
    let doc = decode(&v3).unwrap();
    assert_eq!(doc.format, peachy_document::FORMAT_VERSION);
    assert!(doc.animations.is_empty());
    let Attachment::Mesh(mesh) = &doc.skins["default"]["body"]["cloth"] else {
        panic!("Expected mesh");
    };
    assert_eq!(
        mesh.slot_local_positions().unwrap(),
        [[0.0, 0.0], [10.0, 0.0], [0.0, 10.0]]
    );
    let mut v2 = v3.clone();
    v2["format"] = json!(2);
    assert_eq!(decode(&v2).unwrap(), doc);
    let native = serde_json::to_value(&doc).unwrap();
    assert_eq!(
        native["skins"]["default"]["body"]["cloth"]["vertices"]["mode"],
        "slotLocal"
    );
    assert_eq!(decode(&native).unwrap(), doc);
    let export = peachy_spine::export_json(&doc).unwrap();
    assert_eq!(
        export["skins"][0]["attachments"]["body"]["cloth"]["vertices"],
        json!([0.0, 0.0, 10.0, 0.0, 0.0, 10.0])
    );

    // v2 duration materialization still runs after strict mesh validation.
    v2["animations"] = json!([{"name":"reserved","duration":4,"tracks":[]}]);
    let migrated = decode(&v2).unwrap();
    assert_eq!(migrated.animations[0].duration(), 4.0);
    assert_eq!(migrated.animations[0].tracks[0].keys[0].value, 0.0);
}

#[test]
fn version_boundaries_reject_wrong_vertex_shapes_and_stale_duration_fields() {
    let old = legacy_mesh();
    for version in [2, 3] {
        for invalid in [
            json!({"mode":"slotLocal","positions":[[0,0],[10,0],[0,10]]}),
            json!({"mode":"weighted","influences":[]}),
            json!("vertices"),
            Value::Null,
        ] {
            let mut value = old.clone();
            value["format"] = json!(version);
            value["skins"]["default"]["body"]["cloth"]["vertices"] = invalid;
            assert!(decode(&value).is_err(), "legacy version {version}");
        }
    }
    let mut latest = serde_json::to_value(decode(&old).unwrap()).unwrap();
    latest["skins"]["default"]["body"]["cloth"]["vertices"] =
        old["skins"]["default"]["body"]["cloth"]["vertices"].clone();
    assert!(decode(&latest).is_err());
    for version in [3, 4, peachy_document::FORMAT_VERSION] {
        let mut value = if version == 3 {
            old.clone()
        } else {
            serde_json::to_value(decode(&old).unwrap()).unwrap()
        };
        if version == 4 {
            value["format"] = json!(4);
            for bone in value["bones"].as_array_mut().unwrap() {
                for field in ["shear_x", "shear_y"] {
                    assert_eq!(bone["setup"][field].as_f64(), Some(0.0));
                    bone["setup"].as_object_mut().unwrap().remove(field);
                }
            }
        }
        value["animations"] = json!([{"name":"stale","duration":4,"tracks":[]}]);
        assert!(
            decode(&value)
                .unwrap_err()
                .contains("unknown field `duration`")
        );
    }
}

#[test]
fn legacy_mesh_unknown_fields_and_bad_geometry_are_rejected_before_holds_are_added() {
    for (field, bad) in [
        ("future", json!(true)),
        ("triangles", json!([0, 1, 99])),
        ("uvs", json!([[0, 0]])),
        ("vertices", json!([[0, 0], [10, 0], [0, 1e100]])),
    ] {
        let mut value = legacy_mesh();
        value["format"] = json!(2);
        value["animations"] = json!([{"name":"reserved","duration":4,"tracks":[]}]);
        value["skins"]["default"]["body"]["cloth"][field] = bad;
        assert!(decode(&value).is_err(), "{field}");
    }
    let mut v1 = legacy_mesh();
    v1["format"] = json!(1);
    assert!(decode(&v1).is_err()); // v1 could not contain tagged meshes.
}

#[test]
fn weighted_native_roundtrip_is_strict_and_validates_inactive_skin_references() {
    let source = weighted_native();
    let doc = decode(&source).unwrap();
    let Attachment::Mesh(mesh) = &doc.skins["default"]["body"]["cloth"] else {
        panic!("Expected mesh");
    };
    assert!(matches!(mesh.vertices, MeshVertices::Weighted { .. }));
    assert_eq!(decode(&serde_json::to_value(&doc).unwrap()).unwrap(), doc);
    for path in [
        "/skins/default/body/cloth",
        "/skins/default/body/cloth/vertices",
        "/skins/default/body/cloth/vertices/influences/0/0",
    ] {
        let mut value = source.clone();
        value.pointer_mut(path).unwrap()["future"] = json!(true);
        assert!(decode(&value).is_err(), "{path}");
    }
    let mut inactive = source;
    inactive["skins"]["inactive"] = inactive["skins"]["default"].clone();
    assert!(decode(&inactive).is_ok());
    inactive["skins"]["inactive"]["body"]["cloth"]["vertices"]["influences"][0][0]["bone"] =
        json!(99);
    assert!(decode(&inactive).is_err());
}

#[test]
fn v3_mesh_load_preserves_file_bytes_and_explicit_save_backs_up_before_writing_latest() {
    let folder = std::env::temp_dir().join(format!(
        "peachy-mesh-migrate-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&folder).unwrap();
    let path = folder.join("legacy.peachy");
    let original = serde_json::to_vec_pretty(&legacy_mesh()).unwrap();
    fs::write(&path, &original).unwrap();
    let document = load(&path).unwrap();
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
    save(&path, &document, &folder).unwrap();
    let backup = fs::read_dir(&folder)
        .unwrap()
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().contains("backup-"))
        .unwrap();
    assert_eq!(fs::read(backup.path()).unwrap(), original);
    assert_eq!(load(&path).unwrap(), document);
    let written: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(written["format"], peachy_document::FORMAT_VERSION);
    assert_eq!(
        written["skins"]["default"]["body"]["cloth"]["vertices"]["mode"],
        "slotLocal"
    );
    let weighted = decode(&weighted_native()).unwrap();
    save(&path, &weighted, &folder).unwrap();
    assert_eq!(load(&path).unwrap(), weighted);
    fs::remove_dir_all(folder).unwrap();
}
