//! Transform constraint interchange: observed 4.3 defaults, the evaluated
//! subset, and explicit refusals (docs/slices/transform-constraints.md).
use peachy_document::{Constraint, TransformProperty};
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"skeleton":{"spine":"4.3.26"},
        "bones":[{"name":"root"},{"name":"src","parent":"root","x":10,"rotation":20},
            {"name":"a","parent":"root","x":-5},{"name":"b","parent":"root","y":5},
            {"name":"a child","parent":"a","x":3}],
        "slots":[],"skins":[],
        "constraints":[
            {"type":"transform","name":"follow","bones":["a","b"],"source":"src",
             "x":-20,"y":4,"rotation":30,"mixX":0.5,
             "properties":{"x":{"to":{"x":{}}},"y":{"to":{"y":{}}},"rotate":{"to":{"rotate":{}}}}},
            {"type":"transform","name":"zeros","bones":["b"],"source":"src","mixRotate":0,"mixScaleX":0,"mixShearY":0,
             "properties":{"x":{"to":{"x":{}}}}}
        ],
        "animations":{"idle":{}}})
}

#[test]
fn identity_mappings_roundtrip_with_observed_mix_defaults() {
    let input = fixture();
    let report = inspect(&input);
    assert!(report.supported, "{:?}", report.unsupported);
    let doc = import_json(&input, "Transform").unwrap();
    let Constraint::Transform(follow) = &doc.constraints[0] else {
        panic!("transform")
    };
    assert_eq!((follow.bones.clone(), follow.source), (vec![2, 3], 1));
    // Mapped mixes default to 1, mixY follows mixX; unmapped stay zero.
    assert_eq!(follow.mix.get(TransformProperty::X), 0.5);
    assert_eq!(follow.mix.get(TransformProperty::Y), 0.5);
    assert_eq!(follow.mix.get(TransformProperty::Rotate), 1.);
    assert_eq!(follow.mix.get(TransformProperty::ScaleX), 0.);
    assert_eq!(follow.offsets.get(TransformProperty::Rotate), 30.);
    assert_eq!(follow.offsets.get(TransformProperty::X), -20.);
    let output = export_json(&doc).unwrap();
    assert!(inspect(&output).supported);
    assert_eq!(import_json(&output, "Transform").unwrap(), doc);
    assert_eq!(output["constraints"][0]["type"], json!("transform"));
}

fn reject(value: &Value, reason: &str) {
    let report = inspect(value);
    assert!(!report.supported, "Accepted: {value}");
    assert!(
        report
            .unsupported
            .iter()
            .any(|issue| issue.contains(reason)),
        "Expected {reason:?}, got {:?}",
        report.unsupported
    );
    assert!(import_json(value, "Rejected").is_err());
}

#[test]
fn modes_remapping_and_unsafe_domains_are_refused_explicitly() {
    for flag in ["localSource", "localTarget", "additive", "clamp"] {
        let mut input = fixture();
        input["constraints"][0][flag] = json!(true);
        reject(&input, "local, additive and clamped");
    }
    for properties in [
        json!({"x":{"offset":131.2,"to":{"x":{"offset":284.05,"max":384.05}}}}),
        json!({"x":{"to":{"y":{}}}}),
        json!({"x":{"to":{"x":{"scale":-1}}}}),
        json!({"scaleX":{"to":{"scaleX":{}}}}),
    ] {
        let mut input = fixture();
        input["constraints"][0]["properties"] = properties;
        reject(&input, "only identity Rotate/X/Y mappings");
    }
    let mut unmapped = fixture();
    unmapped["constraints"][1]["mixShearY"] = json!(0.4);
    reject(
        &unmapped,
        "mixShearY has no effect without a shearY mapping",
    );
    let mut skin = fixture();
    skin["constraints"][0]["skin"] = json!("yes");
    reject(&skin, "skin must be boolean");
    let mut source_inside = fixture();
    source_inside["constraints"][0]["source"] = json!("a child");
    reject(&source_inside, "must not contain one another");
    let mut inherited = fixture();
    inherited["bones"][2]["inherit"] = json!("noScale");
    reject(&inherited, "must inherit normally");
    let mut timeline = fixture();
    timeline["animations"]["idle"]["transform"] = json!({"follow":[{"mixX":0.2}]});
    reject(&timeline, "transform timelines");
}
