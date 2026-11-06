use peachy_document::{Constraint, Property};
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"skeleton":{"spine":"4.3.26","fps":30},"bones":[
        {"name":"root"}, {"name":"upper","parent":"root","length":99},
        {"name":"lower","parent":"upper","x":7,"length":10},
        {"name":"target","parent":"root","x":7,"y":10}],
        "slots":[],"skins":[],"constraints":[{"type":"ik","name":"limb",
        "bones":["upper","lower"],"target":"target"}],
        "animations":{"move":{"bones":{"target":{"translate":[
            {"time":0.25,"x":0,"y":0,"curve":"stepped"},
            {"time":1,"x":10,"y":-3}]}}}}})
}

fn reject(value: &Value, reason: &str) {
    let report = inspect(value);
    assert!(!report.supported, "Accepted unsupported source: {value}");
    let error = import_json(value, "Rejected").unwrap_err();
    assert!(error.contains(reason), "Expected {reason:?}, got {error}");
}

#[test]
fn static_one_and_two_bone_defaults_settings_and_target_keys_roundtrip_without_baking() {
    for bones in [json!(["upper"]), json!(["upper", "lower"])] {
        for mix in [0.0, 0.5, 1.0] {
            for bend in [false, true] {
                let mut input = fixture();
                input["constraints"][0]["bones"] = bones.clone();
                input["constraints"][0]["mix"] = json!(mix);
                input["constraints"][0]["bendPositive"] = json!(bend);
                let doc = import_json(&input, "Original IK").unwrap();
                let Constraint::Ik(ik) = &doc.constraints[0];
                assert_eq!(ik.bones.len(), bones.as_array().unwrap().len());
                assert_eq!(ik.setup.mix, mix);
                assert_eq!(ik.setup.bend_positive, bend);
                assert_eq!(ik.target, 3);
                assert_eq!(doc.bones[1].setup.rotation, 0.0);
                assert_eq!(doc.bones[1].length, 99.0); // Display length is not the link offset.
                assert_eq!(doc.animations[0].duration(), 1.0);
                assert_eq!(doc.animations[0].tracks.len(), 2);
                assert_eq!(doc.animations[0].tracks[0].bone, 3);
                let exported = export_json(&doc).unwrap();
                assert!(exported["animations"]["move"].get("ik").is_none());
                assert_eq!(import_json(&exported, &doc.name).unwrap(), doc);
            }
        }
    }
    let defaults = import_json(&fixture(), "Defaults").unwrap();
    let Constraint::Ik(ik) = &defaults.constraints[0];
    assert_eq!(ik.setup.mix, 1.0);
    assert!(ik.setup.bend_positive);
}

#[test]
fn disjoint_constraint_list_order_survives_conflicting_obsolete_metadata_without_sorting() {
    let mut input = fixture();
    input["bones"].as_array_mut().unwrap().extend([
        json!({"name":"second","parent":"root","length":10}),
        json!({"name":"second-target","parent":"root","x":-8,"y":10}),
    ]);
    input["constraints"][0]["order"] = json!(99.0);
    input["constraints"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"ik","name":"first-alphabetically",
        "bones":["second"],"target":"second-target","order":0}));
    for reverse in [false, true] {
        let mut ordered = input.clone();
        if reverse {
            ordered["constraints"].as_array_mut().unwrap().reverse();
        }
        let doc = import_json(&ordered, "Order").unwrap();
        let out = export_json(&doc).unwrap();
        let names = |value: &Value| {
            value["constraints"]
                .as_array()
                .unwrap()
                .iter()
                .map(|entry| entry["name"].as_str().unwrap().to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&out), names(&ordered));
        assert!(
            out["constraints"]
                .as_array()
                .unwrap()
                .iter()
                .all(|c| c.get("order").is_none())
        );
        assert_eq!(import_json(&out, &doc.name).unwrap(), doc);
    }
    for bad in [json!(-1), json!(0.5), json!("0"), Value::Null] {
        let mut invalid = input.clone();
        invalid["constraints"][0]["order"] = bad;
        reject(&invalid, "order must be");
    }
}

#[test]
fn neutral_advanced_fields_are_typed_and_nondefault_aliases_never_silently_disappear() {
    let mut neutral = fixture();
    for (field, value) in [
        ("softness", json!(0.0)),
        ("compress", json!(false)),
        ("stretch", json!(false)),
        ("scaleY", json!("none")),
        ("skin", json!(false)),
    ] {
        neutral["constraints"][0][field] = value;
    }
    let doc = import_json(&neutral, "Neutral").unwrap();
    assert_eq!(doc, import_json(&fixture(), "Neutral").unwrap());
    let out = export_json(&doc).unwrap();
    for field in ["softness", "compress", "stretch", "scaleY", "skin"] {
        assert!(out["constraints"][0].get(field).is_none());
    }
    for mix in [0, 1] {
        for (field, bad) in [
            ("softness", json!(1e-50)),
            ("softness", json!(-1)),
            ("softness", json!("0")),
            ("compress", json!(true)),
            ("compress", json!(0)),
            ("stretch", json!(true)),
            ("stretch", Value::Null),
            ("scaleY", json!("uniform")),
            ("scaleY", json!("volume")),
            ("scaleY", json!(false)),
            ("uniform", json!(false)),
            ("scaleYMode", json!("none")),
            ("enabled", json!(true)),
            ("skin", json!(true)),
            ("skin", json!(0)),
            ("skin", json!("false")),
            ("skin", Value::Null),
            ("future", json!([])),
        ] {
            let mut bad_input = fixture();
            bad_input["constraints"][0]["mix"] = json!(mix);
            bad_input["constraints"][0][field] = bad;
            reject(&bad_input, field);
        }
    }
}

#[test]
fn unknown_kinds_invalid_references_chains_targets_and_settings_are_atomic_import_errors() {
    for (field, bad) in [
        ("type", json!("physics")),
        ("type", Value::Null),
        ("name", json!(" ")),
        ("bones", json!([])),
        ("bones", json!(["upper", "lower", "target"])),
        ("bones", json!(["lower", "upper"])),
        ("bones", json!(["upper", "upper"])),
        ("bones", json!(["root", "lower"])),
        ("bones", json!([99])),
        ("target", json!("lower")),
        ("target", json!("missing")),
        ("target", json!(3)),
        ("mix", Value::Null),
        ("bendPositive", json!(1)),
    ] {
        let mut bad_input = fixture();
        bad_input["constraints"][0][field] = bad;
        assert!(!inspect(&bad_input).supported, "{field}: {bad_input}");
        assert!(import_json(&bad_input, "Bad").is_err());
    }
    let mut mixed = fixture();
    mixed["constraints"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"physics","name":"unsupported"}));
    reject(&mixed, "Only IK constraints");
    let mut duplicate = fixture();
    let mut second = duplicate["constraints"][0].clone();
    second["mix"] = json!(0);
    duplicate["constraints"]
        .as_array_mut()
        .unwrap()
        .push(second);
    reject(&duplicate, "names must be unique");
}

#[test]
fn legacy_aliases_and_malformed_collections_reject_while_known_empty_ik_tracks_are_retained() {
    for bad in [Value::Null, json!({}), json!(false), json!([null])] {
        let mut input = fixture();
        input["constraints"] = bad;
        assert!(!inspect(&input).supported);
        assert!(import_json(&input, "Bad").is_err());
    }
    for alias in ["ik", "transform", "path", "physics"] {
        let mut input = fixture();
        input[alias] = json!([{}]);
        assert!(import_json(&input, "Alias").is_err());
    }
    for bad in [json!([]), json!({}), Value::Null] {
        let mut input = fixture();
        input["ik"] = bad;
        reject(&input, "Legacy root ik");
    }
    for bad in [json!([]), Value::Null] {
        let mut input = fixture();
        input["animations"]["move"]["ik"] = bad;
        reject(&input, "ik");
    }
    let mut empty = fixture();
    empty["animations"]["move"]["ik"] = json!({"limb":[]});
    assert!(import_json(&empty, "Empty typed track").is_ok());
    empty["animations"]["move"]["ik"] = json!({});
    assert!(import_json(&empty, "Empty").is_ok());
    for bad in [json!({}), json!([]), Value::Null] {
        let mut input = fixture();
        input["animations"]["move"]["constraints"] = bad;
        reject(&input, "constraints");
    }
}

#[test]
fn source_domain_preflight_rejects_transformed_chains_dependent_lists_and_child_offset_tracks() {
    for (bone, field, value) in [
        (0, "scaleX", 1.01),
        (1, "scaleY", -1.0),
        (2, "shearX", 0.1),
        (0, "shearY", 1.0),
        (2, "x", 0.0),
        (2, "y", 0.1),
        (2, "length", 0.0),
    ] {
        let mut input = fixture();
        input["bones"][bone][field] = json!(value);
        assert!(!inspect(&input).supported, "{bone}/{field}");
        assert!(import_json(&input, "Domain").is_err());
    }
    for (bone, timeline) in [
        ("root", json!({"scalex":[{"value":1.01}]})),
        ("upper", json!({"sheary":[{"value":1}]})),
        ("lower", json!({"translatex":[{"value":1}]})),
        ("lower", json!({"translate":[{"x":0,"y":1}]})),
    ] {
        let mut input = fixture();
        input["animations"]["move"]["bones"][bone] = timeline;
        assert!(!inspect(&input).supported);
        assert!(import_json(&input, "Domain").is_err());
    }
    let mut overlap = fixture();
    overlap["constraints"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"ik","name":"again","bones":["lower"],"target":"target","mix":0.5}));
    reject(&overlap, "dependent constraint");
    let mut cross = fixture();
    cross["bones"].as_array_mut().unwrap().extend([
        json!({"name":"other","parent":"root","length":2}),
        json!({"name":"target2","parent":"root","x":-7,"y":10}),
    ]);
    cross["constraints"][0]["target"] = json!("other");
    cross["constraints"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"ik","name":"other-aim",
        "bones":["other"],"target":"target2"}));
    reject(&cross, "dependent constraint");
}

#[test]
fn native_data_stays_serializable_when_adapter_domain_refuses_and_mix_zero_is_exactly_inactive() {
    let mut doc = import_json(&fixture(), "Raw").unwrap();
    doc.bones[0].setup.scale_x = -1.3;
    assert!(doc.validate().is_ok());
    assert!(serde_json::to_vec(&doc).is_ok());
    assert!(
        export_json(&doc)
            .unwrap_err()
            .contains("unit positive axes")
    );
    let Constraint::Ik(ik) = &mut doc.constraints[0];
    ik.setup.mix = 0.0;
    let export = export_json(&doc).unwrap();
    assert_eq!(import_json(&export, &doc.name).unwrap(), doc);
}

#[test]
fn neutral_tracks_unrelated_transforms_and_dynamic_target_coincidence_are_not_blanket_rejected() {
    let mut input = fixture();
    input["bones"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"unrelated","parent":"root",
        "scaleX":0,"scaleY":-3,"shearX":17}));
    input["bones"][3]["x"] = json!(0);
    input["bones"][3]["y"] = json!(0);
    input["animations"]["move"]["bones"]["root"] = json!({"scale":[{"x":1,"y":1}],"shear":[{}]});
    input["animations"]["move"]["bones"]["lower"] = json!({"translate":[{}]});
    let doc = import_json(&input, "Dynamic boundary").unwrap();
    assert!(
        doc.animations[0]
            .tracks
            .iter()
            .any(|t| t.property == Property::ScaleX)
    );
    assert!(export_json(&doc).is_ok()); // Dynamic query errors belong to engine, not an adapter solver.
    let mut root = fixture();
    root["constraints"][0]["bones"] = json!(["root"]);
    root["constraints"][0]["target"] = json!("target");
    reject(&root, "target cannot"); // Structural descendant target is invalid independently of evaluator domain.
}
