//! Bone inheritance interchange. Field names and the omitted-key value were
//! observed through installed Spine 4.3.26 import/re-export and the external
//! 4.3.13 runtime (docs/slices/inherit-modes.md).
use peachy_document::Inherit;
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"skeleton":{"spine":"4.3.26"},
        "bones":[{"name":"root"},{"name":"p","parent":"root","scaleX":2,"rotation":30},
            {"name":"a","parent":"p","inherit":"noScale","x":5},
            {"name":"b","parent":"p","inherit":"onlyTranslation"},
            {"name":"c","parent":"p","inherit":"noRotationOrReflection"},
            {"name":"d","parent":"p","inherit":"noScaleOrReflection"},
            {"name":"e","parent":"p","inherit":"normal"}],
        "slots":[],"skins":[],
        "animations":{"anim":{"bones":{
            "a":{"inherit":[{"time":0.25,"inherit":"onlyTranslation"},{"time":0.5},{"time":1,"inherit":"noRotationOrReflection"}]},
            "e":{"inherit":[{"inherit":"noScale"}],"rotate":[{"time":0,"value":0},{"time":2,"value":45}]}}}}})
}

#[test]
fn modes_and_stepped_keys_roundtrip_with_observed_omitted_values() {
    let input = fixture();
    let report = inspect(&input);
    assert!(report.supported, "{:?}", report.unsupported);
    let doc = import_json(&input, "Inherit").unwrap();
    let modes: Vec<_> = doc.bones.iter().map(|b| b.inherit).collect();
    assert_eq!(
        modes,
        [
            Inherit::Normal,
            Inherit::Normal,
            Inherit::NoScale,
            Inherit::OnlyTranslation,
            Inherit::NoRotationOrReflection,
            Inherit::NoScaleOrReflection,
            Inherit::Normal
        ]
    );
    let tracks = &doc.animations[0].inherit_tracks;
    let a = tracks.iter().find(|t| t.bone == 2).unwrap();
    // A key without `inherit` is normal; a key without `time` is at zero.
    assert_eq!(
        a.keys
            .iter()
            .map(|k| (k.time, k.inherit))
            .collect::<Vec<_>>(),
        [
            (0.25, Inherit::OnlyTranslation),
            (0.5, Inherit::Normal),
            (1., Inherit::NoRotationOrReflection)
        ]
    );
    let e = tracks.iter().find(|t| t.bone == 6).unwrap();
    assert_eq!((e.keys[0].time, e.keys[0].inherit), (0., Inherit::NoScale));
    let output = export_json(&doc).unwrap();
    assert_eq!(output["bones"][2]["inherit"], json!("noScale"));
    assert!(output["bones"][6].get("inherit").is_none());
    assert_eq!(
        output["animations"]["anim"]["bones"]["a"]["inherit"][1],
        json!({"time":0.5,"inherit":"normal"})
    );
    assert!(inspect(&output).supported);
    assert_eq!(import_json(&output, "Inherit").unwrap(), doc);
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
fn malformed_or_ambiguous_inheritance_is_refused() {
    let mut legacy = fixture();
    legacy["bones"][6]["transform"] = json!("noScale");
    reject(&legacy, "legacy transform inheritance alias");
    let mut unknown = fixture();
    unknown["bones"][2]["inherit"] = json!("noShear");
    reject(&unknown, "unknown inherit mode");
    for (bad, reason) in [
        (json!([{"time":1},{"time":0.5}]), "increasing"),
        (
            json!([{"time":0,"inherit":"sideways"}]),
            "unknown inherit mode",
        ),
        (
            json!([{"time":0,"inherit":"normal","curve":"stepped"}]),
            "unsupported field curve",
        ),
        (json!([{"time":"0"}]), "time must be a finite f32 number"),
        (json!({}), "must be an array"),
    ] {
        let mut input = fixture();
        input["animations"]["anim"]["bones"]["a"]["inherit"] = bad;
        reject(&input, reason);
    }
}

#[test]
fn ik_constrained_bones_must_inherit_normally() {
    let mut input = json!({"skeleton":{"spine":"4.3.26"},
        "bones":[{"name":"root"},{"name":"arm","parent":"root","length":10},
            {"name":"target","parent":"root","x":10,"y":10}],
        "slots":[],"skins":[],
        "constraints":[{"type":"ik","name":"aim","bones":["arm"],"target":"target"}],
        "animations":{"a":{}}});
    assert!(import_json(&input, "IK").is_ok());
    input["bones"][1]["inherit"] = json!("noScale");
    reject(&input, "must inherit normally");
    input["bones"][1]["inherit"] = json!("normal");
    input["animations"]["a"] =
        json!({"bones":{"arm":{"inherit":[{"time":1,"inherit":"onlyTranslation"}]}}});
    reject(&input, "must inherit normally");
}
