use peachy_document::{Attachment, MeshVertices};
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/weighted-triangle.json")).unwrap()
}

fn vertices(value: &mut Value) -> &mut Value {
    &mut value["skins"][0]["attachments"]["body"]["cloth"]["vertices"]
}

#[test]
fn weighted_mesh_roundtrip_keeps_all_influences_including_repeats_zeros_and_rounded_sums() {
    let source = fixture();
    assert!(inspect(&source).supported);
    let document = import_json(&source, "original weighted triangle").unwrap();
    let Attachment::Mesh(mesh) = &document.skins["default"]["body"]["cloth"] else {
        panic!("Expected mesh");
    };
    let MeshVertices::Weighted { influences } = &mesh.vertices else {
        panic!("Expected weighted vertices");
    };
    assert_eq!(mesh.vertex_count(), 3);
    assert_eq!(
        influences.iter().map(Vec::len).collect::<Vec<_>>(),
        [6, 1, 2]
    );
    assert_eq!(
        influences[0].iter().map(|i| i.bone).collect::<Vec<_>>(),
        [0, 1, 2, 3, 4, 1]
    );
    assert_eq!(influences[0][3].weight, 0.39999);
    assert_eq!(influences[0][4].weight, 0.0);
    assert_eq!(influences[0][5].weight, 0.0);
    assert_eq!(influences[0][5].position, [-10.0, 0.0]);
    assert_ne!(influences[0].iter().map(|i| i.weight).sum::<f32>(), 1.0);
    let output = export_json(&document).unwrap();
    assert!(inspect(&output).supported);
    let encoded = &output["skins"][0]["attachments"]["body"]["cloth"];
    assert_eq!(encoded["vertices"].as_array().unwrap().len(), 39);
    assert_eq!(encoded["vertices"][0], 6);
    assert_eq!(encoded["vertices"][21], 1); // Repeated bone index stays an integer.
    assert_eq!(encoded["vertices"][24], 0.0);
    assert_eq!(
        encoded["triangles"],
        source["skins"][0]["attachments"]["body"]["cloth"]["triangles"]
    );
    assert_eq!(
        encoded["edges"],
        source["skins"][0]["attachments"]["body"]["cloth"]["edges"]
    );
    assert_eq!(import_json(&output, &document.name).unwrap(), document);
}

#[test]
fn integral_json_float_counts_and_bone_indices_import_and_roundtrip_identically() {
    let expected = import_json(&fixture(), "integer representations").unwrap();
    let mut source = fixture();
    let raw = vertices(&mut source).as_array_mut().unwrap();
    let mut cursor = 0;
    while cursor < raw.len() {
        let count = raw[cursor].as_u64().unwrap() as usize;
        raw[cursor] = json!(count as f64);
        assert!(raw[cursor].as_u64().is_none());
        cursor += 1;
        for _ in 0..count {
            raw[cursor] = json!(raw[cursor].as_u64().unwrap() as f64);
            assert!(raw[cursor].as_u64().is_none());
            cursor += 4;
        }
    }
    // Exercise real JSON decimal spellings, including 0.0 and 1.0, at the boundary.
    let encoded = serde_json::to_string(&source).unwrap();
    let source: Value = serde_json::from_str(&encoded).unwrap();
    assert!(inspect(&source).supported);
    let actual = import_json(&source, &expected.name).unwrap();
    assert_eq!(actual, expected);
    let output = export_json(&actual).unwrap();
    assert_eq!(output, export_json(&expected).unwrap());
    assert_eq!(import_json(&output, &expected.name).unwrap(), expected);
}

#[test]
fn malformed_weighted_counts_groups_numbers_and_references_are_rejected_by_inspection_and_import() {
    for (index, bad) in [
        (0, json!(0)),
        (0, json!(-1)),
        (0, json!(1.5)),
        (0, json!(2.0_f64.powi(usize::BITS as i32))),
        (0, json!(1e100)),
        (0, json!("6")),
        (0, json!(u64::MAX)),
        (1, json!(99)),
        (1, json!(-1)),
        (1, json!(0.5)),
        (1, json!(1e100)),
        (2, json!(1e100)),
        (2, json!("0")),
        (4, json!(-0.1)),
        (4, json!(1.1)),
        (4, json!(1e100)),
    ] {
        let mut value = fixture();
        vertices(&mut value)[index] = bad.clone();
        assert!(!inspect(&value).supported, "index={index} value={bad}");
        assert!(
            import_json(&value, "invalid").is_err(),
            "index={index} value={bad}"
        );
    }
    for length in [0, 5, 7, 24, 25, 30, 38] {
        let mut value = fixture();
        vertices(&mut value)
            .as_array_mut()
            .unwrap()
            .truncate(length);
        assert!(!inspect(&value).supported, "truncated at {length}");
        assert!(import_json(&value, "truncated").is_err());
    }
    let mut trailing = fixture();
    vertices(&mut trailing)
        .as_array_mut()
        .unwrap()
        .push(json!(0));
    assert!(!inspect(&trailing).supported);
    assert!(import_json(&trailing, "trailing").is_err());
    let mut zero_total = fixture();
    for index in [4, 8, 12, 16] {
        vertices(&mut zero_total)[index] = json!(0);
    }
    assert!(!inspect(&zero_total).supported);
    assert!(import_json(&zero_total, "zero total").is_err());
}

#[test]
fn weighted_inspection_validates_inactive_skin_influences_and_keeps_other_features_blocked() {
    let mut value = fixture();
    let mut inactive = value["skins"][0].clone();
    inactive["name"] = json!("inactive");
    inactive["attachments"]["body"]["cloth"]["name"] = json!("inactive-cloth");
    value["skins"].as_array_mut().unwrap().push(inactive);
    assert!(inspect(&value).supported);
    value["skins"][1]["attachments"]["body"]["cloth"]["vertices"][1] = json!(999);
    assert!(!inspect(&value).supported);
    assert!(import_json(&value, "inactive invalid").is_err());
    for (path, bad) in [
        ("/skins/0/attachments/body/cloth/type", json!("linkedmesh")),
        ("/skins/0/attachments/body/cloth/future", json!(1)),
        (
            "/animations/bend/attachments",
            json!({"default":{"body":{"cloth":{}}}}),
        ),
        ("/constraints", json!([{"type":"ik"}])),
    ] {
        let mut value = fixture();
        let (parent, field) = path.rsplit_once('/').unwrap();
        value.pointer_mut(parent).unwrap()[field] = bad;
        assert!(!inspect(&value).supported, "{path}");
        assert!(import_json(&value, "unsupported").is_err());
    }
}
