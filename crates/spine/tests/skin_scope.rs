//! Skin-required bones/constraints and named skin lists round-trip.
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"skeleton":{"spine":"4.3.26"},
        "bones":[{"name":"root"},{"name":"hat","parent":"root","skin":true,"y":20},
            {"name":"feather","parent":"hat","skin":true,"x":4},{"name":"aim","parent":"root","x":30}],
        "slots":[],"constraints":[
            {"type":"ik","name":"tilt","skin":true,"bones":["hat"],"target":"aim"},
            {"type":"physics","name":"flutter","skin":true,"bone":"feather","rotate":1}],
        "skins":[{"name":"default"},{"name":"party","bones":["hat","feather"],"ik":["tilt"],"physics":["flutter"]},
            {"name":"plain"}],
        "animations":{"idle":{}}})
}

#[test]
fn skin_scope_roundtrips_by_kind() {
    let input = fixture();
    let report = inspect(&input);
    assert!(report.supported, "{:?}", report.unsupported);
    let doc = import_json(&input, "Scope").unwrap();
    assert_eq!(
        doc.skin_scope
            .required_bones
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(
        doc.skin_scope
            .required_constraints
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [0, 1]
    );
    assert_eq!(doc.skin_scope.skins["party"].constraints, [0, 1]);
    let output = export_json(&doc).unwrap();
    assert_eq!(output["skins"][1]["ik"], json!(["tilt"]));
    assert_eq!(output["skins"][1]["physics"], json!(["flutter"]));
    assert_eq!(output["constraints"][0]["skin"], json!(true));
    assert_eq!(import_json(&output, "Scope").unwrap(), doc);
}

#[test]
fn malformed_or_unsupported_scopes_are_refused() {
    for (patch, reason) in [
        (
            json!({"skins":[{"name":"party","bones":["missing"]}]}),
            "unknown bone",
        ),
        (
            json!({"skins":[{"name":"party","transform":["tilt"]}]}),
            "unknown transform constraint",
        ),
        (json!({"skins":[{"name":"party","path":["rail"]}]}), "path"),
        (
            json!({"skins":[{"name":"party","bones":"hat"}]}),
            "must list names",
        ),
    ] {
        let mut input = fixture();
        for (key, value) in patch.as_object().unwrap() {
            input[key] = value.clone();
        }
        assert!(
            import_json(&input, "Bad").unwrap_err().contains(reason),
            "{reason}"
        );
    }
    let mut unflagged = fixture();
    unflagged["bones"][1]
        .as_object_mut()
        .unwrap()
        .remove("skin");
    assert!(import_json(&unflagged, "Bad").is_err());
}
