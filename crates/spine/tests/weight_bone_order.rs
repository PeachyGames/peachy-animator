//! Original native-observed Editor consumer controls; no array normalization.
use peachy_document::{Attachment, BoneInfluence, Document, MeshVertices};
use peachy_spine::{ExportTarget, export_json, export_json_for, import_json};
use serde_json::{Value, json};

fn source(rows: &[&[usize]]) -> Value {
    assert_eq!(rows.len(), 3);
    let mut vertices = Vec::new();
    for (vertex, row) in rows.iter().enumerate() {
        vertices.push(json!(row.len()));
        for (entry, &bone) in row.iter().enumerate() {
            vertices.extend([
                json!(bone),
                json!(vertex as f32 * 10. + entry as f32 * 0.5),
                json!(vertex as f32 * -5.),
                json!(0.25),
            ]);
        }
    }
    json!({"skeleton":{"spine":"4.3.26"},
        "bones":[{"name":"root"},{"name":"A","parent":"root"},
            {"name":"B","parent":"root"},{"name":"C","parent":"root"}],
        "slots":[{"name":"body","bone":"root","attachment":"mesh"}],
        "skins":[{"name":"default","attachments":{"body":{"mesh":{
            "type":"mesh","path":"original-cutout","vertices":vertices,
            "uvs":[0,0,1,0,0,1],"triangles":[0,1,2],"hull":3,"width":20,"height":20
        }}}}],"animations":{}})
}

fn encoded_vertices(value: &Value) -> &Value {
    &value["skins"][0]["attachments"]["body"]["mesh"]["vertices"]
}

fn influences(doc: &Document, placeholder: &str) -> Vec<Vec<BoneInfluence>> {
    let Attachment::Mesh(mesh) = &doc.skins["default"]["body"][placeholder] else {
        panic!()
    };
    let MeshVertices::Weighted { influences } = &mesh.vertices else {
        panic!()
    };
    influences.clone()
}

#[test]
fn weight_bone_order_editor_rejects_ab_ba_but_runtime_preserves_exact_raw_arrays() {
    let input = source(&[&[1, 2], &[2, 1], &[1, 2]]);
    let document = import_json(&input, "AB BA order control").unwrap();
    document.validate().unwrap();
    let before = document.clone();
    let error = export_json_for(&document, ExportTarget::SpineEditor43).unwrap_err();
    assert!(error.contains("attachment default/body/mesh"), "{error}");
    assert!(
        error.contains("precedence cycle") && error.contains("Runtime43"),
        "{error}"
    );
    assert_eq!(export_json(&document).unwrap_err(), error);
    assert_eq!(document, before);
    let output = export_json_for(&document, ExportTarget::Runtime43).unwrap();
    assert_eq!(encoded_vertices(&output), encoded_vertices(&input));
    assert_eq!(import_json(&output, &document.name).unwrap(), document);
}

#[test]
fn weight_bone_order_consistent_cab_is_accepted_without_numeric_sorting() {
    let input = source(&[&[3, 1, 2], &[3, 1, 2], &[3, 1, 2]]);
    let document = import_json(&input, "Non numeric CAB order").unwrap();
    let before = document.clone();
    let editor = export_json_for(&document, ExportTarget::SpineEditor43).unwrap();
    let runtime = export_json_for(&document, ExportTarget::Runtime43).unwrap();
    assert_eq!(encoded_vertices(&editor), encoded_vertices(&input));
    assert_eq!(editor, runtime);
    assert_eq!(import_json(&editor, &document.name).unwrap(), document);
    assert_eq!(document, before);
}

#[test]
fn weight_bone_order_detects_cross_vertex_cycle_without_requiring_matching_bone_sets() {
    let input = source(&[&[1, 2], &[2, 3], &[3, 1]]);
    let document = import_json(&input, "Sparse three bone cycle").unwrap();
    let error = export_json_for(&document, ExportTarget::SpineEditor43).unwrap_err();
    assert!(error.contains("precedence cycle"), "{error}");
    let output = export_json_for(&document, ExportTarget::Runtime43).unwrap();
    assert_eq!(encoded_vertices(&output), encoded_vertices(&input));
    let ordered = import_json(
        &source(&[&[3, 1], &[1, 2], &[3, 2]]),
        "Sparse acyclic order",
    )
    .unwrap();
    let output = export_json_for(&ordered, ExportTarget::SpineEditor43).unwrap();
    assert_eq!(import_json(&output, &ordered.name).unwrap(), ordered);
}

#[test]
fn weight_bone_order_first_occurrence_analysis_preserves_repeated_zero_and_nonunit_payload() {
    let mut input: Value =
        serde_json::from_str(include_str!("fixtures/weighted-triangle.json")).unwrap();
    let mesh = &mut input["skins"][0]["attachments"]["body"]["cloth"];
    // Isolate this consumer rule from the independently tested name override gate.
    mesh["name"] = json!("cloth");
    mesh["vertices"][24] = json!(-0.0);
    let document = import_json(&input, "Repeated and zero order control").unwrap();
    let before = influences(&document, "cloth");
    assert_eq!(
        before[0].iter().map(|entry| entry.bone).collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4, 1]
    );
    assert_ne!(before[0].iter().map(|entry| entry.weight).sum::<f32>(), 1.);
    assert_eq!(before[0][5].weight.to_bits(), (-0.0_f32).to_bits());
    for target in [ExportTarget::SpineEditor43, ExportTarget::Runtime43] {
        let output = export_json_for(&document, target).unwrap();
        let decoded = import_json(&output, &document.name).unwrap();
        let after = influences(&decoded, "cloth");
        for (old, new) in before.iter().zip(&after) {
            assert_eq!(old.len(), new.len());
            for (old, new) in old.iter().zip(new) {
                assert_eq!(old.bone, new.bone);
                assert_eq!(
                    old.position.map(f32::to_bits),
                    new.position.map(f32::to_bits)
                );
                assert_eq!(old.weight.to_bits(), new.weight.to_bits());
            }
        }
    }
    assert_eq!(influences(&document, "cloth"), before);
}
