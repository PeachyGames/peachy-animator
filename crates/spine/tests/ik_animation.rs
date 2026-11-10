use peachy_document::{Constraint, Document, IkCurve, IkKeyframe, IkTrack};
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture(frames: Value) -> Value {
    json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"},
        {"name":"upper","parent":"root","length":99},
        {"name":"lower","parent":"upper","x":7,"length":10},
        {"name":"target","parent":"root","x":7,"y":10}],
        "slots":[],"skins":[],"constraints":[{"type":"ik","name":"aim",
        "bones":["upper","lower"],"target":"target","mix":0.25,"bendPositive":false}],
        "animations":{"clip":{"ik":{"aim":frames}}}})
}
fn reject(value: &Value) {
    assert!(
        !inspect(value).supported,
        "Accepted unsupported input: {value}"
    );
    assert!(import_json(value, "Bad").is_err());
}

#[test]
fn grouped_defaults_are_absolute_and_empty_zero_fractional_ik_only_clips_derive_duration() {
    for (frames, end, count) in [
        (json!([]), 0.0, 0),
        (json!([{}]), 0.0, 1),
        (json!([{"time":0.001}]), 0.001, 1),
        (
            json!([{"time":0.25,"mix":0.2,"bendPositive":false},{"time":1}]),
            1.0,
            2,
        ),
    ] {
        let doc = import_json(&fixture(frames), "Defaults").unwrap();
        let animation = &doc.animations[0];
        assert!(animation.tracks.is_empty());
        assert_eq!(animation.duration(), end);
        assert_eq!(animation.ik_tracks[0].constraint, 0);
        assert_eq!(animation.ik_tracks[0].keys.len(), count);
        if count > 0 {
            let last = animation.ik_tracks[0].keys.last().unwrap();
            assert_eq!(last.mix, 1.0);
            assert!(last.bend_positive);
        }
        let out = export_json(&doc).unwrap();
        if count == 0 {
            assert!(out["animations"]["clip"].get("ik").is_none());
        } else {
            let reopened = import_json(&out, &doc.name).unwrap();
            assert_eq!(reopened, doc);
            assert_eq!(
                out["animations"]["clip"]["ik"]["aim"]
                    .as_array()
                    .unwrap()
                    .len(),
                count
            );
            assert!(
                out["animations"]["clip"]["bones"]
                    .as_object()
                    .unwrap()
                    .is_empty()
            );
        }
    }
}

#[test]
fn finite_signed_and_overshooting_mix_values_are_not_clamped_or_setup_relative() {
    for setup in [-0.625_f32, 1.625, f32::MAX] {
        let mut source = fixture(
            json!([{"mix":-0.625,"bendPositive":false,"curve":"stepped"},{"time":1,"mix":1.625}]),
        );
        source["constraints"][0]["mix"] = json!(setup);
        let doc = import_json(&source, "Finite Mix").unwrap();
        let Constraint::Ik(ik) = &doc.constraints[0] else {
            unreachable!("fixture constraint is IK")
        };
        assert_eq!(ik.setup.mix, setup);
        assert_eq!(doc.animations[0].ik_tracks[0].keys[0].mix, -0.625);
        assert_eq!(doc.animations[0].ik_tracks[0].keys[1].mix, 1.625);
        assert_eq!(
            import_json(&export_json(&doc).unwrap(), &doc.name).unwrap(),
            doc
        );
    }
    for field in ["mix", "time"] {
        let mut bad = fixture(json!([{}]));
        bad["animations"]["clip"]["ik"]["aim"][0][field] = json!(1e100);
        reject(&bad);
    }
}

#[test]
fn equal_endpoint_excursions_crossed_time_controls_and_eight_channel_export_roundtrip() {
    let source = fixture(json!([{"mix":0.5,"curve":[0.8,0.9,0.2,0.9,0.1,0,0.9,0]},
        {"time":1,"mix":0.5,"bendPositive":false}]));
    let doc = import_json(&source, "Excursion").unwrap();
    let IkCurve::Bezier { time, mix } = doc.animations[0].ik_tracks[0].keys[0].curve else {
        panic!()
    };
    assert_eq!(time, [f64::from(0.8_f32), f64::from(0.2_f32)]);
    assert_eq!(mix, [0.9, 0.9]);
    let out = export_json(&doc).unwrap();
    let handles = out["animations"]["clip"]["ik"]["aim"][0]["curve"]
        .as_array()
        .unwrap();
    assert_eq!(handles.len(), 8);
    assert_eq!(handles[5], 0.0);
    assert_eq!(handles[7], 0.0);
    assert_eq!(import_json(&out, &doc.name).unwrap(), doc);
    assert_eq!(out["animations"]["clip"]["ik"]["aim"][0]["mix"], 0.5);
}

#[test]
fn ratio_time_controls_survive_native_json_and_reconstruct_canonical_absolute_times() {
    let source = fixture(
        json!([{"time":0.25,"mix":-2e38_f32,"curve":[0.4,-1e38_f32,0.775,1e38_f32,0.25,0,1,0]},
        {"time":1,"mix":2e38_f32}]),
    );
    let doc = import_json(&source, "Ratios").unwrap();
    let IkCurve::Bezier { time, .. } = doc.animations[0].ik_tracks[0].keys[0].curve else {
        panic!()
    };
    assert_eq!(
        time,
        [
            (f64::from(0.4_f32) - 0.25) / 0.75,
            (f64::from(0.775_f32) - 0.25) / 0.75
        ]
    );
    let reopened: Document = serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    assert_eq!(reopened, doc);
    let out = export_json(&reopened).unwrap();
    let h = &out["animations"]["clip"]["ik"]["aim"][0]["curve"];
    assert_eq!(h[0].as_f64().unwrap() as f32, 0.4);
    assert_eq!(h[2].as_f64().unwrap() as f32, 0.775);
    assert_eq!(import_json(&out, &doc.name).unwrap(), doc);
    // An exact decimal endpoint is compared in the same canonical space as its key.
    let endpoint = fixture(json!([{"time":0.1,"curve":[0.1,1,0.8,1,0.1,0,1,0]},{"time":1}]));
    assert!(import_json(&endpoint, "Endpoints").is_ok());
}

#[test]
fn tiny_nonzero_mix_spans_and_absolute_controls_are_not_flattened_by_bone_epsilon_rules() {
    let source = fixture(
        json!([{"mix":1e-30_f32,"curve":[0.2,2e-30_f32,0.8,-1e-30_f32,0.2,0,0.8,0]},
        {"time":1,"mix":1.1e-30_f32}]),
    );
    let doc = import_json(&source, "Tiny").unwrap();
    assert!(matches!(
        doc.animations[0].ik_tracks[0].keys[0].curve,
        IkCurve::Bezier { .. }
    ));
    assert_eq!(
        import_json(&export_json(&doc).unwrap(), &doc.name).unwrap(),
        doc
    );
}

#[test]
fn all_eight_controls_and_unsupported_softness_are_validated_before_eliding_neutral_metadata() {
    let valid = fixture(json!([{"mix":0,"curve":[0.2,0.1,0.8,1,0.9,0,0.1,0]},{"time":1,"mix":1}]));
    assert!(import_json(&valid, "Neutral softness").is_ok());
    for (index, value) in [
        (0, json!(-0.1)),
        (0, json!(-1e-50)),
        (4, json!(-1e-50)),
        (6, json!(1.00000000001)),
        (2, json!(1.1)),
        (4, json!(-0.1)),
        (6, json!(1.1)),
        (5, json!(1e-46)),
        (7, json!(0.0001)),
        (5, json!("0")),
        (4, Value::Null),
        (1, json!(1e100)),
    ] {
        let mut bad = valid.clone();
        bad["animations"]["clip"]["ik"]["aim"][0]["curve"][index] = value;
        reject(&bad);
    }
    for handles in [
        json!([0.2, 0, 0.8, 1]),
        json!([0, 0, 1, 1, 0, 0, 1, 0, 99]),
        json!([]),
    ] {
        let mut bad = valid.clone();
        bad["animations"]["clip"]["ik"]["aim"][0]["curve"] = handles;
        reject(&bad);
    }
    for mix in [0, 1] {
        for (field, badvalue) in [
            ("softness", json!(1e-46)),
            ("softness", json!("0")),
            ("compress", json!(true)),
            ("stretch", json!(true)),
            ("stretch", json!(0)),
            ("scaleY", json!("none")),
            ("skin", json!(false)),
            ("value", json!(1)),
            ("bendPositive", json!(0)),
        ] {
            let mut bad = fixture(json!([{"mix":mix}]));
            bad["animations"]["clip"]["ik"]["aim"][0][field] = badvalue;
            reject(&bad);
        }
    }
}

#[test]
fn references_key_time_collisions_and_final_source_bezier_metadata_are_not_silently_repaired() {
    for frames in [
        json!([{"time":0.25},{"time":0.250000001}]),
        json!([{"time":1},{"time":0}]),
        json!([{"time":-1}]),
        json!([{"curve":[0.2,0,0.8,1,0.2,0,0.8,0]}]),
        json!([null]),
    ] {
        reject(&fixture(frames));
    }
    let mut unknown = fixture(json!([]));
    unknown["animations"]["clip"]["ik"] = json!({"missing":[]});
    reject(&unknown);
    let close = fixture(
        json!([{"time":0.25,"bendPositive":true},{"time":0.2500000298023224,"bendPositive":false}]),
    );
    let doc = import_json(&close, "Distinct").unwrap();
    assert!(
        doc.animations[0].ik_tracks[0].keys[0].time < doc.animations[0].ik_tracks[0].keys[1].time
    );
}

#[test]
fn activity_is_per_setup_and_clip_and_includes_nonzero_controls_negative_values_and_reachable_setup()
 {
    let mut off = fixture(json!([{"mix":0}]));
    off["animations"]["clip"]["bones"] = json!({"upper":{"scale":[{"x":2,"y":2}]}});
    assert!(import_json(&off, "Inactive clip").is_ok());
    off["animations"]["clip"]["ik"]["aim"][0]["time"] = json!(0.25);
    reject(&off);
    for controls in [[1.0, 1.0], [-1.0, -1.0]] {
        let mut active = fixture(
            json!([{"mix":0,"curve":[0.2,controls[0],0.8,controls[1],0.2,0,0.8,0]},{"time":1,"mix":0}]),
        );
        active["constraints"][0]["mix"] = json!(0);
        active["bones"][1]["scaleX"] = json!(2);
        reject(&active);
    }
    let mut negative = fixture(json!([{"mix":-0.5}]));
    negative["constraints"][0]["mix"] = json!(0);
    negative["bones"][1]["scaleX"] = json!(2);
    reject(&negative);
    let mut canceled = fixture(json!([{"mix":1}]));
    canceled["constraints"][0]["mix"] = json!(0);
    canceled["bones"][1]["scaleX"] = json!(2);
    canceled["animations"]["clip"]["bones"] = json!({"upper":{"scalex":[{"value":0.5}]}});
    reject(&canceled);
}

#[test]
fn source_dependency_preflight_does_not_union_separate_clips_or_claim_never_coactive_support() {
    let mut source = fixture(json!([]));
    source["constraints"][0]["mix"] = json!(0);
    let mut other = source["constraints"][0].clone();
    other["name"] = json!("other");
    source["constraints"].as_array_mut().unwrap().push(other);
    source["animations"] = json!({"first":{"ik":{"aim":[{"mix":1}],"other":[{"mix":0}]}},
        "second":{"ik":{"aim":[{"mix":0}],"other":[{"mix":-1}]}}});
    assert!(import_json(&source, "Separate clips").is_ok());
    source["animations"] = json!({"shared":{"ik":{"aim":[{"mix":1,"curve":"stepped"},{"time":0.5,"mix":0}],
        "other":[{"mix":0,"curve":"stepped"},{"time":0.5,"mix":1}]}}});
    reject(&source);
}

#[test]
fn native_final_curve_is_inert_and_export_validates_bad_references_before_indexing() {
    let mut doc = import_json(&fixture(json!([{"mix":0}])), "Native").unwrap();
    let Constraint::Ik(ik) = &mut doc.constraints[0] else {
        unreachable!("fixture constraint is IK")
    };
    ik.setup.mix = 0.0;
    doc.bones[1].setup.scale_x = 2.0;
    doc.animations[0].ik_tracks[0].keys[0].curve = IkCurve::Bezier {
        time: [0.2, 0.8],
        mix: [1.0, -1.0],
    };
    assert!(doc.validate().is_ok());
    let out = export_json(&doc).unwrap();
    assert!(
        out["animations"]["clip"]["ik"]["aim"][0]
            .get("curve")
            .is_none()
    );
    let mut invalid = doc.clone();
    let Constraint::Ik(ik) = &mut invalid.constraints[0] else {
        unreachable!("fixture constraint is IK")
    };
    ik.target = usize::MAX;
    assert!(export_json(&invalid).is_err());
    let mut invalid = doc.clone();
    invalid.animations[0].ik_tracks[0].constraint = usize::MAX;
    assert!(export_json(&invalid).is_err());
    let mut invalid = doc;
    invalid.animations[0].ik_tracks = vec![IkTrack {
        constraint: 0,
        keys: vec![IkKeyframe {
            time: 0.0,
            mix: 0.0,
            bend_positive: true,
            curve: IkCurve::Bezier {
                time: [f64::NAN, 0.8],
                mix: [0.0, 1.0],
            },
        }],
    }];
    assert!(export_json(&invalid).is_err());
}

#[test]
fn eight_observed_native_and_retimed_curves_export_complete_f64_control_times() {
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ik-time-fidelity");
    let mut cases = 0;
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|v| v.to_str()) != Some("peachy") {
            continue;
        }
        let doc = review_schema_document(&std::fs::read_to_string(&path).unwrap());
        doc.validate().unwrap();
        let track = &doc.animations[0].ik_tracks[0];
        let left = track.keys[0];
        let right = track.keys[1];
        let IkCurve::Bezier { time, .. } = left.curve else {
            panic!()
        };
        let expected = time.map(|ratio| {
            f64::from(left.time) + (f64::from(right.time) - f64::from(left.time)) * ratio
        });
        assert!(
            expected.iter().any(|&v| f64::from(v as f32) != v),
            "Fixture must expose the old rounding loss"
        );
        let output = export_json(&doc).unwrap();
        let Constraint::Ik(ik) = &doc.constraints[track.constraint] else {
            unreachable!("fixture constraint is IK")
        };
        let handles = &output["animations"][&doc.animations[0].name]["ik"][&ik.name][0]["curve"];
        assert_eq!(handles[0].as_f64(), Some(expected[0]), "{}", path.display());
        assert_eq!(handles[2].as_f64(), Some(expected[1]), "{}", path.display());
        assert_eq!(handles[4], handles[0]);
        assert_eq!(handles[6], handles[2]);
        cases += 1;
    }
    assert_eq!(cases, 8);
}

/// The time-fidelity fixtures are retained byte-for-byte (manifest SHA-256) in
/// the pre-renumbering IK review schema: `format: 7` with `ik_tracks`, before
/// setup slot colors, physics lists and native10 inheritance existed. Upgrade
/// them in memory only.
fn review_schema_document(text: &str) -> Document {
    let mut value: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(value["format"], 7);
    value["format"] = serde_json::json!(peachy_document::FORMAT_VERSION);
    for slot in value["slots"].as_array_mut().unwrap() {
        assert!(slot.get("color").is_none());
        slot["color"] = serde_json::json!([255, 255, 255, 255]);
    }
    for animation in value["animations"].as_array_mut().unwrap() {
        assert!(animation.get("ik_tracks").is_some());
        animation["physics"] = serde_json::json!([]);
        animation["physics_resets"] = serde_json::json!([]);
        animation["inherit_tracks"] = serde_json::json!([]);
    }
    for bone in value["bones"].as_array_mut().unwrap() {
        assert!(bone.get("inherit").is_none());
        bone["inherit"] = serde_json::json!("normal");
    }
    serde_json::from_value(value).unwrap()
}
