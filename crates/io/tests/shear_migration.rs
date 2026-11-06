use peachy_document::{Attachment, Document, FORMAT_VERSION, MeshVertices, RegionTransform};
use peachy_io::{decode_project, load, save};
use serde_json::{Value, json};
use std::{fs, time::SystemTime};

fn legacy(version: u32) -> Value {
    let mut value: Value =
        serde_json::from_str(include_str!("fixtures/legacy-v2-duration.peachy")).unwrap();
    value["format"] = json!(version);
    if version >= 3 {
        for animation in value["animations"].as_array_mut().unwrap() {
            animation.as_object_mut().unwrap().remove("duration");
        }
    }
    value["slots"] = json!([{"name":"body","bone":0,"attachment":"card"}]);
    let mut region = json!({"path":"card","width":10,"height":10,"transform":{
        "x":0,"y":0,"rotation":0,"scale_x":1,"scale_y":1
    }});
    if version != 1 {
        region["type"] = json!("region");
    }
    value["skins"] = json!({"default":{"body":{"card":region}}});
    value
}

fn decode(value: &Value) -> Result<Document, String> {
    decode_project(&value.to_string())
}

#[test]
fn original_v4_migrations_and_current_zero_shear_examples_keep_complete_export_goldens() {
    let historical = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/native-v4-zero-shear");
    let examples =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
    for name in [
        "peach-bot",
        "contract-cases",
        "mesh-bot",
        "mesh-contract",
        "weighted-bot",
        "weighted-contract",
        "weighted-repeated",
    ] {
        let original = fs::read_to_string(historical.join(format!("{name}.peachy"))).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&original).unwrap()["format"],
            4
        );
        let golden = fs::read(historical.join(format!("{name}.json"))).unwrap();
        for (label, input) in [
            ("historical v4", original),
            (
                "current example",
                fs::read_to_string(examples.join(format!("{name}.peachy"))).unwrap(),
            ),
        ] {
            let document = decode_project(&input).unwrap();
            assert_eq!(document.format, FORMAT_VERSION);
            // Compare actual exporter bytes. Parsing long golden decimals into
            // f64 can change a last bit with serde_json's default float parser.
            let exported =
                serde_json::to_vec_pretty(&peachy_spine::export_json(&document).unwrap()).unwrap();
            assert!(
                exported == golden,
                "{name}: {label} export differs from pre-change bytes"
            );
        }
    }
}

#[test]
fn every_valid_legacy_version_gains_only_bone_zeros_and_rejects_future_shear_data() {
    for version in 1..=4 {
        let source = legacy(version);
        let doc = decode(&source).unwrap(); // Prove the negative controls start from valid legacy data.
        assert_eq!(doc.format, FORMAT_VERSION);
        assert!(
            doc.bones
                .iter()
                .all(|b| b.setup.shear_x == 0.0 && b.setup.shear_y == 0.0)
        );
        let Attachment::Region(region) = &doc.skins["default"]["body"]["card"] else {
            panic!()
        };
        assert_eq!(
            region.transform,
            serde_json::from_value::<RegionTransform>(
                source["skins"]["default"]["body"]["card"]["transform"].clone()
            )
            .unwrap()
        );
        for field in ["shear_x", "shear_y"] {
            for bad in [json!(0), json!(-720), Value::Null] {
                let mut smuggled = source.clone();
                smuggled["bones"][0]["setup"][field] = bad;
                assert!(
                    decode(&smuggled)
                        .unwrap_err()
                        .contains("Legacy project formats cannot contain bone shear")
                );
            }
        }
        for property in ["shearX", "shearY"] {
            for keys in [json!([]), json!([{"time":0,"value":0,"curve":"linear"}])] {
                let mut smuggled = source.clone();
                smuggled["animations"][0]["tracks"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"bone":0,"property":property,"keys":keys}));
                assert!(
                    decode(&smuggled)
                        .unwrap_err()
                        .contains("Legacy project formats cannot contain shear tracks")
                );
            }
        }
        let mut missing_setup = source;
        missing_setup["bones"][0]
            .as_object_mut()
            .unwrap()
            .remove("setup");
        assert!(decode(&missing_setup).is_err());
    }
}

#[test]
fn latest_bones_require_both_finite_shears_and_regions_stay_strictly_five_fields() {
    let mut latest = serde_json::to_value(decode(&legacy(4)).unwrap()).unwrap();
    latest["bones"][0]["setup"]["shear_x"] = json!(-1080.25);
    latest["bones"][0]["setup"]["shear_y"] = json!(720.5);
    let doc = decode(&latest).unwrap();
    assert_eq!(doc.bones[0].setup.shear_x, -1080.25);
    assert_eq!(doc.bones[0].setup.shear_y, 720.5);
    assert_eq!(decode(&serde_json::to_value(&doc).unwrap()).unwrap(), doc);
    assert_eq!(latest["bones"][0]["setup"].as_object().unwrap().len(), 7);
    assert_eq!(
        latest["skins"]["default"]["body"]["card"]["transform"]
            .as_object()
            .unwrap()
            .len(),
        5
    );
    for field in ["shear_x", "shear_y"] {
        let mut missing = latest.clone();
        missing["bones"][0]["setup"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(decode(&missing).unwrap_err().contains("missing field"));
        for bad in [json!("0"), Value::Null, json!(1e100)] {
            let mut malformed = latest.clone();
            malformed["bones"][0]["setup"][field] = bad;
            assert!(decode(&malformed).is_err());
        }
        for zero in [json!(0), json!(25)] {
            let mut region = latest.clone();
            region["skins"]["default"]["body"]["card"]["transform"][field] = zero;
            assert!(decode(&region).unwrap_err().contains("unknown field"));
        }
    }
    latest["animations"][0]["tracks"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "bone":0,"property":"shearX","keys":[{"time":0,"value":-360,"curve":"linear"}]
        }));
    assert!(decode(&latest).is_ok());
    latest["format"] = json!(FORMAT_VERSION + 1);
    assert!(decode(&latest).is_err());
}

#[test]
fn v4_weighted_raw_payload_and_original_bytes_survive_load_and_explicit_latest_save() {
    let mut old = legacy(4);
    old["skins"]["default"]["body"]["cloth"] = json!({
        "type":"mesh","path":"cloth","width":16,"height":16,"uvs":[[0,0],[1,0],[0,1]],
        "triangles":[0,1,2],"hull":3,"vertices":{"mode":"weighted","influences":[
            [{"bone":1,"position":[-10,-0.0],"weight":0.4_f32},
             {"bone":0,"position":[0,0],"weight":0.59999_f32},
             {"bone":1,"position":[2,3],"weight":-0.0_f32}],
            [{"bone":1,"position":[0,0],"weight":0.5001_f32},{"bone":0,"position":[10,0],"weight":0.5_f32}],
            [{"bone":0,"position":[0,10],"weight":1}]
        ]}
    });
    old["slots"][0]["attachment"] = json!("cloth");
    let original = serde_json::to_vec_pretty(&old).unwrap();
    let doc = decode(&old).unwrap();
    let mut expected = old.clone();
    expected["format"] = json!(FORMAT_VERSION);
    expected["constraints"] = json!([]);
    for bone in expected["bones"].as_array_mut().unwrap() {
        bone["setup"]["shear_x"] = json!(0);
        bone["setup"]["shear_y"] = json!(0);
    }
    assert_eq!(doc, serde_json::from_value::<Document>(expected).unwrap());
    let Attachment::Mesh(mesh) = &doc.skins["default"]["body"]["cloth"] else {
        panic!()
    };
    let MeshVertices::Weighted { influences } = &mesh.vertices else {
        panic!()
    };
    assert_eq!(
        influences[0].iter().map(|e| e.bone).collect::<Vec<_>>(),
        [1, 0, 1]
    );
    assert_eq!(influences[0][0].position[1].to_bits(), (-0.0_f32).to_bits());
    assert_eq!(influences[0][2].weight.to_bits(), (-0.0_f32).to_bits());
    assert_eq!(influences[0][1].weight.to_bits(), 0.59999_f32.to_bits());
    assert_eq!(influences[1][0].weight.to_bits(), 0.5001_f32.to_bits());
    let export = peachy_spine::export_json(&doc).unwrap();
    let imported = peachy_spine::import_json(&export, &doc.name).unwrap();
    assert_eq!(imported.bones, doc.bones);
    assert_eq!(imported.skins, doc.skins);
    assert_eq!(peachy_spine::export_json(&imported).unwrap(), export);

    let folder = std::env::temp_dir().join(format!(
        "peachy-shear-migrate-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&folder).unwrap();
    let path = folder.join("weighted.peachy");
    fs::write(&path, &original).unwrap();
    assert_eq!(load(&path).unwrap(), doc);
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
    save(&path, &doc, &folder).unwrap();
    let backup = fs::read_dir(&folder)
        .unwrap()
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().contains("backup-"))
        .unwrap();
    assert_eq!(fs::read(backup.path()).unwrap(), original);
    assert_eq!(load(&path).unwrap(), doc);
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&path).unwrap()).unwrap()["format"],
        FORMAT_VERSION
    );
    fs::remove_dir_all(folder).unwrap();
}
