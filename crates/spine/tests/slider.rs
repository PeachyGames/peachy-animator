//! Original bone-only Slider interchange probes. Tests exercise data identity,
//! ownership and malformed-source refusal, independently of engine evaluation.
use peachy_document::{Constraint, Curve, SliderProperty, SliderSetup, TransformProperty};
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({
        "skeleton":{"spine":"4.3.26","fps":60},
        "bones":[{"name":"root"},{"name":"follower","parent":"root","rotation":30}],
        "slots":[], "skins":[],
        "constraints":[{"type":"slider","name":"calibration","animation":"z-reference"}],
        "animations":{"a-host":{},"z-reference":{"bones":{"root":{
            "translatex":[{},{"time":1.5,"value":80}]
        }}}}
    })
}

fn roundtrip(source: &Value) -> peachy_document::Document {
    let report = inspect(source);
    assert!(report.supported, "{:?}", report.unsupported);
    let document = import_json(source, "Original Slider").unwrap();
    let exported = export_json(&document).unwrap();
    assert_eq!(import_json(&exported, &document.name).unwrap(), document);
    document
}

fn reject(source: &Value, reason: &str) {
    let original = source.clone();
    let report = inspect(source);
    assert!(!report.supported, "{source}");
    let error = import_json(source, "Rejected Slider").unwrap_err();
    assert!(error.contains(reason), "Expected {reason}: {error}");
    assert_eq!(*source, original);
}

#[test]
fn slider_native_defaults_forward_reference_and_independent_range_fields_roundtrip() {
    let source = fixture();
    let document = roundtrip(&source);
    let slider = document.constraints[0].as_slider().unwrap();
    assert_eq!(slider.animation, 1);
    assert_eq!(slider.setup, SliderSetup { time: 0., mix: 1. });
    assert_eq!((slider.bone, slider.property), (None, None));
    assert_eq!([slider.from, slider.to, slider.scale, slider.max], [0.; 4]);
    assert!(!slider.local && !slider.looped && !slider.additive);

    let mut mapped = fixture();
    mapped["constraints"][0] = json!({
        "type":"slider", "name":"calibration", "animation":"z-reference",
        "bone":"follower", "property":"rotate", "local":true,
        "from":10, "to":0.3333333, "max":1.5, "scale":0.012962963,
        "time":-0.25, "mix":1.5, "loop":true, "additive":true
    });
    let document = roundtrip(&mapped);
    let slider = document.constraints[0].as_slider().unwrap();
    assert_eq!(slider.bone, Some(1));
    assert_eq!(slider.property, Some(TransformProperty::Rotate));
    assert_eq!(
        slider.setup,
        SliderSetup {
            time: -0.25,
            mix: 1.5
        }
    );
    assert_eq!(slider.from, 10.);
    assert_eq!(slider.to.to_bits(), 0.3333333_f32.to_bits());
    assert_eq!(slider.scale.to_bits(), 0.012962963_f32.to_bits());
    assert_eq!(slider.max, 1.5);

    // Inactive ranges remain raw data; no adapter operation resets them merely
    // because the public consumer ignores them while bone is absent.
    mapped["constraints"][0]
        .as_object_mut()
        .unwrap()
        .remove("bone");
    let manual = roundtrip(&mapped);
    let inactive = manual.constraints[0].as_slider().unwrap();
    assert_eq!(inactive.from, slider.from);
    assert_eq!(inactive.to, slider.to);
    assert_eq!(inactive.property, slider.property);
    assert_eq!(inactive.setup, slider.setup);
}

#[test]
fn slider_time_keys_resolve_constraint_type_and_preserve_seconds_absolute_curves_and_flat_handles()
{
    let mut source = fixture();
    source["constraints"].as_array_mut().unwrap().insert(
        0,
        json!({"type":"physics","name":"calibration","bone":"follower"}),
    );
    source["animations"]["a-host"]["slider"] = json!({"calibration":{"time":[
        {"value":0.5,"curve":[0.125,0.75,0.375,1.25]},
        {"time":0.5,"value":1.5,"curve":"stepped"},
        {"time":1,"value":-0.25}
    ]}});
    let document = roundtrip(&source);
    let track = &document.animations[0].slider_tracks[0];
    assert_eq!(track.constraint, 1);
    assert_eq!(track.property, SliderProperty::Time);
    assert_eq!((track.keys[0].time, track.keys[0].value), (0., 0.5));
    assert_eq!(track.keys[0].curve, Curve::Bezier([0.25, 0.25, 0.75, 0.75]));
    assert_eq!(track.keys[1].curve, Curve::Stepped);
    let exported = export_json(&document).unwrap();
    assert_eq!(
        exported["animations"]["a-host"]["slider"]["calibration"]["time"][0]["curve"],
        json!([0.125, 0.75, 0.375, 1.25])
    );

    source["animations"]["a-host"]["slider"]["calibration"]["time"] = json!([
        {"value":0.5,"curve":[0.125,0.75,0.375,0.25]},
        {"time":0.5,"value":0.5}
    ]);
    let flat = roundtrip(&source);
    assert!(matches!(
        flat.animations[0].slider_tracks[0].keys[0].curve,
        Curve::BezierOffset(_)
    ));
    assert_eq!(
        export_json(&flat).unwrap()["animations"]["a-host"]["slider"]["calibration"]["time"][0]["curve"],
        json!([0.125, 0.75, 0.375, 0.25])
    );
}

#[test]
fn slider_native_independent_time_and_mix_keys_keep_host_times_and_reference_seconds_separate() {
    let mut source = fixture();
    source["animations"]["a-host"]["slider"] = json!({"calibration":{
        "time":[{"value":0.5},{"time":0.25,"value":0.75}],
        "mix":[{"time":0.25,"value":0.25}]
    }});
    let document = roundtrip(&source);
    let host = &document.animations[0];
    assert_eq!(host.duration(), 0.25);
    assert_eq!(document.animations[1].duration(), 1.5);
    let time = host
        .slider_tracks
        .iter()
        .find(|track| track.property == SliderProperty::Time)
        .unwrap();
    let mix = host
        .slider_tracks
        .iter()
        .find(|track| track.property == SliderProperty::Mix)
        .unwrap();
    assert_eq!((time.keys[1].time, time.keys[1].value), (0.25, 0.75));
    assert_eq!((mix.keys[0].time, mix.keys[0].value), (0.25, 0.25));
    assert_eq!(time.constraint, mix.constraint);
    assert_eq!(document.constraints[0].as_slider().unwrap().setup.mix, 1.);
}

#[test]
fn slider_native_time_and_mix_omitted_values_are_one_while_explicit_zero_survives() {
    let mut source = fixture();
    source["animations"]["a-host"]["slider"] = json!({"calibration":{
        "time":[{"value":0.5},{"time":0.25,"value":0.75},{"time":0.5},{"time":1,"value":0}],
        "mix":[{"time":0.25,"value":0.25},{"time":0.5},{"time":1,"value":0}]
    }});
    let document = roundtrip(&source);
    for track in &document.animations[0].slider_tracks {
        assert_eq!(
            track.keys.iter().find(|key| key.time == 0.5).unwrap().value,
            1.
        );
        assert_eq!(track.keys.last().unwrap().value, 0.);
    }
    let exported = export_json(&document).unwrap();
    for property in ["time", "mix"] {
        let keys = exported["animations"]["a-host"]["slider"]["calibration"][property]
            .as_array()
            .unwrap();
        assert_eq!(keys.last().unwrap()["value"], json!(0.));
    }
}

#[test]
fn slider_native_named_skin_membership_roundtrips_by_constraint_type() {
    let mut source = fixture();
    source["constraints"][0]["skin"] = json!(true);
    source["constraints"].as_array_mut().unwrap().insert(
        0,
        json!({
            "type":"physics","name":"calibration","bone":"follower","skin":true
        }),
    );
    source["skins"] = json!([
        {"name":"default"},
        {"name":"slider_skin","slider":["calibration"]},
        {"name":"physics_skin","physics":["calibration"]}
    ]);
    let document = roundtrip(&source);
    assert_eq!(document.skin_scope.skins["slider_skin"].constraints, [1]);
    assert_eq!(document.skin_scope.skins["physics_skin"].constraints, [0]);
    let output = export_json(&document).unwrap();
    let skins = output["skins"].as_array().unwrap();
    let slider_skin = skins
        .iter()
        .find(|skin| skin["name"] == "slider_skin")
        .unwrap();
    assert_eq!(slider_skin["slider"], json!(["calibration"]));
    assert!(slider_skin.get("physics").is_none());

    for bad in [json!(null), json!("calibration"), json!(["missing"])] {
        source["skins"][1]["slider"] = bad;
        reject(&source, "slider");
    }
}

#[test]
fn slider_redundant_named_skin_membership_preserves_metadata_without_requiring_activity_membership()
{
    // The original source-only public-API cases turn skinRequired off while
    // retaining named skin lists. Nonrequired Slider activity stays true.
    let mut source = fixture();
    source["skins"] = json!([
        {"name":"default"},
        {"name":"listed","slider":["calibration"]},
        {"name":"other"}
    ]);
    let document = roundtrip(&source);
    assert!(document.skin_scope.required_constraints.is_empty());
    assert!(document.skin_scope.required_bones.is_empty());
    assert!(!document.skin_scope.is_empty());
    assert_eq!(document.skin_scope.skins["listed"].constraints, [0]);
    for skin in ["default", "listed", "other"] {
        assert!(document.activity(skin).constraints[0], "{skin}");
    }
    // A scope containing only redundant membership must survive native
    // serialization too; omitting it would silently lose the raw list.
    let native = serde_json::to_value(&document).unwrap();
    assert_eq!(
        native["skin_scope"]["skins"]["listed"]["constraints"],
        json!([0])
    );
    assert_eq!(
        serde_json::from_value::<peachy_document::Document>(native).unwrap(),
        document
    );

    source["skins"][1]["slider"] = json!(["calibration", "calibration"]);
    reject(&source, "listed constraints");
    source["skins"][1]["slider"] = json!(["missing"]);
    reject(&source, "unknown slider constraint");
    source["skins"][1]["slider"] = json!(["calibration"]);
    source["skins"][1]["bones"] = json!(["follower"]);
    reject(&source, "listed bones");
    source["skins"][1].as_object_mut().unwrap().remove("bones");
    source["constraints"].as_array_mut().unwrap().push(json!({
        "type":"physics","name":"sway","bone":"follower"
    }));
    source["skins"][1]["physics"] = json!(["sway"]);
    reject(&source, "listed constraints");
    source["skins"][1]
        .as_object_mut()
        .unwrap()
        .remove("physics");
    source["skins"][0]["slider"] = json!(["calibration"]);
    reject(&source, "default skin cannot list");
}

#[test]
fn slider_all_observed_source_properties_and_raw_nested_references_roundtrip() {
    for property in TransformProperty::ALL {
        let mut source = fixture();
        source["constraints"][0]["bone"] = json!("follower");
        source["constraints"][0]["property"] = json!(property.spine_name());
        let document = roundtrip(&source);
        assert_eq!(
            document.constraints[0].as_slider().unwrap().property,
            Some(property)
        );
    }
    let mut source = fixture();
    source["constraints"].as_array_mut().unwrap().push(json!({
        "type":"slider","name":"second","animation":"a-host"
    }));
    source["animations"]["a-host"]["slider"] = json!({"calibration":{"time":[{"value":0.5}]}});
    source["animations"]["z-reference"]["slider"] = json!({"second":{"time":[{"value":0.25}]}});
    let document = roundtrip(&source);
    assert_eq!(document.animations[0].slider_tracks[0].constraint, 0);
    assert_eq!(document.animations[1].slider_tracks[0].constraint, 1);
    assert!(matches!(document.constraints[1], Constraint::Slider(_)));
}

#[test]
fn slider_malformed_constraint_fields_and_missing_references_are_refused_without_rewriting() {
    for (field, value, reason) in [
        ("animation", Value::Null, "animation must be a name"),
        (
            "animation",
            json!("missing"),
            "unknown referenced animation",
        ),
        ("bone", Value::Null, "bone must be a name"),
        ("bone", json!("missing"), "unknown source bone"),
        ("bone", json!("follower"), "source bone needs a property"),
        ("property", json!("shearX"), "unknown source property"),
        ("property", Value::Null, "unknown source property"),
        ("from", json!(1e40), "finite native number"),
        ("to", Value::Null, "finite native number"),
        ("scale", json!("0.1"), "finite native number"),
        ("max", Value::Null, "finite native number"),
        ("time", Value::Null, "finite native number"),
        ("mix", json!(true), "finite native number"),
        ("local", Value::Null, "must be boolean"),
        ("loop", json!(1), "must be boolean"),
        ("additive", json!(0), "must be boolean"),
        ("skin", Value::Null, "must be boolean"),
        ("order", json!(-1), "order must be"),
        ("offset", json!(0), "unsupported field"),
    ] {
        let mut source = fixture();
        source["constraints"][0][field] = value;
        reject(&source, reason);
    }
}

#[test]
fn slider_timeline_wrong_owner_unknown_fields_and_collapsed_times_are_refused() {
    for (timeline, reason) in [
        (Value::Null, "slider must be an object"),
        (json!({"missing":{"time":[]}}), "unknown Slider constraint"),
        (json!({"calibration":[]}), "timelines must be an object"),
        (
            json!({"calibration":{"frame":[]}}),
            "unknown timeline frame",
        ),
        (
            json!({"calibration":{"time":null}}),
            "keys must be an array",
        ),
        (
            json!({"calibration":{"time":[null]}}),
            "key must be an object",
        ),
        (
            json!({"calibration":{"time":[{"value":null}]}}),
            "finite native number",
        ),
        (
            json!({"calibration":{"time":[{"time":-1}]}}),
            "nonnegative and ordered",
        ),
        (
            json!({"calibration":{"time":[{"frame":1}]}}),
            "unsupported field",
        ),
        (
            json!({"calibration":{"time":[{"curve":[0,0,1,1,0]}]}}),
            "four finite",
        ),
        (
            json!({"calibration":{"time":[{"curve":[0,0,1,1]}]}}),
            "Bezier on last key",
        ),
        (
            json!({"calibration":{"time":[{"time":1},{"time":1.00000001}]}}),
            "nonnegative and ordered",
        ),
    ] {
        let mut source = fixture();
        source["animations"]["a-host"]["slider"] = timeline;
        reject(&source, reason);
    }
    let mut wrong_owner = fixture();
    wrong_owner["constraints"][0] =
        json!({"type":"physics","name":"calibration","bone":"follower"});
    wrong_owner["animations"]["a-host"]["slider"] = json!({"calibration":{"time":[{}]}});
    reject(&wrong_owner, "unknown Slider constraint");
}
