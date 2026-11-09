//! Physics constraint data and timeline interchange. Field names and omitted
//! values were observed from original probes through installed Spine 4.3.26
//! and the external 4.3.13 runtime's public data API (docs/PROVENANCE.md).
use peachy_document::{
    Constraint, Curve, PHYSICS_DEFAULT_SETUP, PhysicsConstraint, PhysicsProperty, ScaleYMode,
};
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"skeleton":{"spine":"4.3.26","fps":30},
    "bones":[{"name":"root"},{"name":"tail","parent":"root","length":50,"x":10},
        {"name":"tip","parent":"tail","length":40,"x":50},
        {"name":"target","parent":"root","x":7,"y":10},
        {"name":"upper","parent":"root","length":99},{"name":"lower","parent":"upper","x":7,"length":10}],
    "slots":[],"skins":[],
    "constraints":[
        {"type":"physics","name":"sway","bone":"tail","x":0.5,"y":0.25,"rotate":0.75,
         "scaleX":0.2,"shearX":0.3,"scaleY":"volume","limit":1234,"fps":45,"inertia":0.6,
         "strength":77,"damping":0.9,"mass":2,"wind":3,"gravity":-4,"mix":0.8,
         "inertiaGlobal":true,"windGlobal":true,"mixGlobal":true},
        {"type":"ik","name":"limb","bones":["upper","lower"],"target":"target"},
        {"type":"physics","name":"tip","bone":"tip","rotate":1,"order":7}
    ],
    "animations":{"move":{
        "bones":{"tail":{"rotate":[{"time":0,"value":0},{"time":1,"value":30}]}},
        "physics":{
            "sway":{"inertia":[{"value":0.5},{"time":1,"value":0.7}],
                "strength":[{"value":50,"curve":"stepped"},{"time":1,"value":60}],
                "mix":[{},{"time":1.5,"value":0.5,"curve":[1.6,0.4,1.8,0.2]},{"time":2,"value":0.25}],
                "reset":[{"time":0.5}]},
            "tip":{"reset":[{"time":0.25},{"time":2.5}]},
            "":{"wind":[{"value":4}],"reset":[{"time":0.1}]}
        }}}})
}

fn physics(doc: &peachy_document::Document, index: usize) -> &PhysicsConstraint {
    doc.constraints[index].as_physics().unwrap()
}

#[test]
fn settings_timelines_and_constraint_order_roundtrip() {
    let input = fixture();
    let report = inspect(&input);
    assert!(report.supported, "{:?}", report.unsupported);
    let doc = import_json(&input, "Physics").unwrap();
    assert_eq!(doc.constraints.len(), 3);
    assert!(matches!(doc.constraints[1], Constraint::Ik(_)));
    let sway = physics(&doc, 0);
    assert_eq!(sway.bone, 1);
    assert_eq!(
        [
            sway.influence.x,
            sway.influence.y,
            sway.influence.rotate,
            sway.influence.scale_x,
            sway.influence.shear_x
        ],
        [0.5, 0.25, 0.75, 0.2, 0.3]
    );
    assert_eq!(sway.scale_y_mode, ScaleYMode::Volume);
    assert_eq!((sway.limit, sway.fps), (1234.0, 45.0));
    assert_eq!(
        PhysicsProperty::ALL.map(|p| p.read(sway.setup)),
        [0.6, 77.0, 0.9, 2.0, 3.0, -4.0, 0.8]
    );
    assert_eq!(
        PhysicsProperty::ALL.map(|p| p.global(sway.global)),
        [true, false, false, false, true, false, true]
    );
    let animation = &doc.animations[0];
    let track = |constraint, property| {
        animation
            .physics
            .iter()
            .find(|t| t.constraint == constraint && t.property == property)
            .unwrap()
    };
    // Omitted key values: 1 for mix, 0 otherwise; omitted time is zero.
    let mix = track(Some(0), PhysicsProperty::Mix);
    assert_eq!((mix.keys[0].time, mix.keys[0].value), (0.0, 1.0));
    assert!(matches!(mix.keys[1].curve, Curve::Bezier(_)));
    assert_eq!(
        track(Some(0), PhysicsProperty::Strength).keys[0].curve,
        Curve::Stepped
    );
    assert_eq!(track(None, PhysicsProperty::Wind).keys[0].value, 4.0);
    let resets: Vec<_> = animation
        .physics_resets
        .iter()
        .map(|r| (r.constraint, r.times.clone()))
        .collect();
    assert!(resets.contains(&(Some(0), vec![0.5])));
    assert!(resets.contains(&(Some(2), vec![0.25, 2.5])));
    assert!(resets.contains(&(None, vec![0.1])));
    // Reset keys extend the duration like any other timeline.
    assert_eq!(animation.duration(), 2.5);

    let output = export_json(&doc).unwrap();
    let constraints = output["constraints"].as_array().unwrap();
    assert_eq!(
        constraints
            .iter()
            .map(|c| c["type"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["physics", "ik", "physics"]
    );
    // Every value is explicit and obsolete order metadata is not exported.
    let f32_field = |value: &Value| value.as_f64().map(|v| v as f32);
    assert_eq!(f32_field(&constraints[2]["inertia"]), Some(0.5));
    assert_eq!(f32_field(&constraints[2]["damping"]), Some(0.85));
    assert_eq!(constraints[2]["scaleY"], json!("none"));
    assert!(constraints[2].get("order").is_none());
    assert_eq!(
        f32_field(&output["animations"]["move"]["physics"][""]["reset"][0]["time"]),
        Some(0.1)
    );
    assert!(inspect(&output).supported);
    assert_eq!(import_json(&output, "Physics").unwrap(), doc);
}

#[test]
fn omitted_fields_use_observed_spine_defaults() {
    let mut input = fixture();
    input["constraints"] = json!([{"type":"physics","name":"bare","bone":"tail"}]);
    input["animations"] = json!({});
    let doc = import_json(&input, "Defaults").unwrap();
    let bare = physics(&doc, 0);
    assert_eq!(bare.setup, PHYSICS_DEFAULT_SETUP);
    assert_eq!(
        PhysicsProperty::ALL.map(|p| p.read(bare.setup)),
        [0.5, 100.0, 0.85, 1.0, 0.0, 0.0, 1.0]
    );
    assert_eq!((bare.limit, bare.fps), (5000.0, 60.0));
    assert_eq!(bare.scale_y_mode, ScaleYMode::None);
    assert_eq!(*bare, PhysicsConstraint::new("bare", 1));
}

fn reject(value: &Value, reason: &str) {
    let report = inspect(value);
    assert!(!report.supported, "Accepted unsupported source: {value}");
    assert!(
        report
            .unsupported
            .iter()
            .any(|issue| issue.contains(reason)),
        "Inspection expected {reason:?}, got {:?}",
        report.unsupported
    );
    let error = import_json(value, "Rejected").unwrap_err();
    assert!(error.contains(reason), "Expected {reason:?}, got {error}");
}

#[test]
fn unsupported_or_malformed_physics_is_refused_atomically() {
    for (field, bad, reason) in [
        (
            "skin",
            json!(true),
            "skin-required physics constraints are unsupported",
        ),
        (
            "scaleY",
            json!("stretch"),
            "scaleY must be none, uniform or volume",
        ),
        ("mass", json!(0), "Mass must be positive"),
        ("mass", json!(-1), "Mass must be positive"),
        ("fps", json!(0), "FPS must be finite and positive"),
        ("limit", json!(-1), "limit must be finite and nonnegative"),
        ("mix", json!(-0.5), "Mix must be nonnegative"),
        (
            "inertia",
            json!(1e39),
            "inertia must be a finite f32 number",
        ),
        ("rotate", json!("1"), "rotate must be a finite f32 number"),
        ("windGlobal", json!(1), "windGlobal must be boolean"),
        ("bone", json!("missing"), "unknown bone missing"),
        ("bones", json!(["tail"]), "unsupported field bones"),
        (
            "order",
            json!(-1),
            "order must be a finite nonnegative integer",
        ),
    ] {
        let mut input = fixture();
        input["constraints"][0][field] = bad;
        reject(&input, reason);
    }
    for (path, bad, reason) in [
        (
            "missing",
            json!({"reset":[{"time":1}]}),
            "unknown constraint missing",
        ),
        (
            "limb",
            json!({"reset":[{"time":1}]}),
            "must reference a physics constraint",
        ),
        (
            "sway",
            json!({"limit":[{"time":1}]}),
            "unsupported physics timeline limit",
        ),
        (
            "sway",
            json!({"reset":[{"time":1,"value":2}]}),
            "unsupported field value",
        ),
        ("sway", json!({"wind":[{"time":2},{"time":1}]}), "ordered"),
        (
            "sway",
            json!({"wind":[{"time":0,"value":"2"}]}),
            "value must be a finite f32 number",
        ),
        (
            "sway",
            json!({"wind":[{"time":0,"curve":[1,2]}]}),
            "4 finite Bezier coordinates",
        ),
        ("sway", json!({"wind":{}}), "must be an array"),
    ] {
        let mut input = fixture();
        input["animations"]["move"]["physics"][path] = bad;
        reject(&input, reason);
    }
    let mut legacy_root = fixture();
    legacy_root["physics"] = json!([{"name":"old","bone":"tail"}]);
    reject(&legacy_root, "physics: not supported");
    let mut scaled = fixture();
    scaled["skeleton"]["referenceScale"] = json!(92);
    reject(&scaled, "referenceScale other than 100 is unsupported");
    scaled["skeleton"]["referenceScale"] = json!(100);
    assert!(inspect(&scaled).supported);
}

#[test]
fn inspection_names_each_constraint_kind_instead_of_calling_it_ik() {
    let mut input = fixture();
    input["constraints"].as_array_mut().unwrap().extend([
        json!({"type":"transform","name":"copy","bones":["tip"],"source":"root"}),
        json!({"type":"path","name":"rail","bones":["tip"],"slot":"x"}),
    ]);
    let report = inspect(&input);
    assert!(!report.supported);
    let joined = report.unsupported.join("\n");
    assert!(joined.contains("Transform constraint copy: transform constraints are unsupported"));
    assert!(joined.contains("Path constraint rail: path constraints are unsupported"));
    assert!(!joined.contains("IK constraint"), "{joined}");
}
