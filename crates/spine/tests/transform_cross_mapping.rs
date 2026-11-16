//! The newly evaluated world X-to-Rotate mapping is an adapter capability,
//! not permission to normalize raw mappings or expand the native data rules.
use peachy_document::{
    Constraint, Document, TransformConstraint, TransformMapping, TransformProperty, TransformTarget,
};
use peachy_spine::{ExportTarget, export_json_for, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({
        "skeleton":{"spine":"4.3.26"},
        "bones":[
            {"name":"root"},
            {"name":"driver","parent":"root","x":20,"y":120},
            {"name":"follower","parent":"root","y":-100,"rotation":30,"length":120},
            {"name":"tip","parent":"follower","x":120}
        ],
        "slots":[],"skins":[],
        "constraints":[{
            "type":"transform","name":"X drives Rotate","source":"driver","bones":["follower"],
            "rotation":17,"x":5,"y":-7,"mixRotate":-0.375,
            "mixX":0,"mixY":0,"mixScaleX":0,"mixScaleY":0,"mixShearY":0,
            "localSource":false,"localTarget":false,"additive":false,"clamp":true,
            "properties":{"x":{"offset":10.0,"to":{"rotate":{"offset":-30.0,"max":150.0,"scale":2.0}}}}
        }],
        "animations":{"calibration":{"bones":{"driver":{"translatex":[
            {"time":0,"value":0},{"time":0.5,"value":80},{"time":1,"value":-80}
        ]}}}}
    })
}

fn transform(doc: &mut Document) -> &mut TransformConstraint {
    let Constraint::Transform(value) = &mut doc.constraints[0] else {
        panic!("expected original transform constraint")
    };
    value
}

fn rejected(input: &Value, doc: &Document, reason: &str) {
    // Native documents retain complete finite data even when the current
    // adapter and evaluation domain cannot execute that data.
    doc.validate().unwrap();
    let original = input.clone();
    let report = inspect(input);
    assert!(!report.supported);
    assert!(
        report
            .unsupported
            .iter()
            .any(|issue| issue.contains(reason)),
        "expected {reason:?}, got {:?}",
        report.unsupported
    );
    assert!(import_json(input, "Rejected cross mapping").is_err());
    assert_eq!(*input, original);
    let Constraint::Transform(value) = &doc.constraints[0] else {
        panic!("expected original transform constraint")
    };
    assert!(value.supported_subset().is_err());
    for target in [ExportTarget::SpineEditor43, ExportTarget::Runtime43] {
        assert!(export_json_for(doc, target).is_err());
    }
}

#[test]
fn world_x_to_rotate_preserves_offsets_coefficients_mix_clamp_and_keys() {
    let input = fixture();
    let original = input.clone();
    let report = inspect(&input);
    assert!(report.supported, "{:?}", report.unsupported);
    let doc = import_json(&input, "Cross mapping").unwrap();
    assert_eq!(input, original);
    let Constraint::Transform(value) = &doc.constraints[0] else {
        panic!("expected transform")
    };
    assert_eq!((value.source, value.bones.as_slice()), (1, &[2][..]));
    assert_eq!(value.offsets.rotate, 17.0);
    assert_eq!((value.offsets.x, value.offsets.y), (5.0, -7.0));
    assert_eq!(value.mix.rotate, -0.375);
    assert_eq!(
        (value.mix.x, value.mix.y, value.mix.scale_x),
        (0.0, 0.0, 0.0)
    );
    assert!(value.clamp);
    assert!(!value.local_source && !value.local_target && !value.additive);
    assert_eq!(value.mappings[0].from, TransformProperty::X);
    assert_eq!(value.mappings[0].offset, 10.0);
    assert_eq!(value.mappings[0].to[0].property, TransformProperty::Rotate);
    assert_eq!(value.mappings[0].to[0].offset, -30.0);
    assert_eq!(value.mappings[0].to[0].max, 150.0);
    assert_eq!(value.mappings[0].to[0].scale, 2.0);
    for target in [ExportTarget::SpineEditor43, ExportTarget::Runtime43] {
        let output = export_json_for(&doc, target).unwrap();
        assert!(inspect(&output).supported);
        assert_eq!(
            output["constraints"][0]["properties"],
            input["constraints"][0]["properties"]
        );
        assert_eq!(output["constraints"][0]["mixRotate"], -0.375);
        assert_eq!(output["constraints"][0]["clamp"], true);
        assert_eq!(import_json(&output, "Cross mapping").unwrap(), doc);
    }
}

#[test]
fn raw_zero_and_negative_mapping_scales_are_not_repaired_on_import_or_export() {
    for scale in [0.0, -2.5] {
        let mut input = fixture();
        input["constraints"][0]["properties"]["x"] =
            json!({"offset":23.5,"to":{"rotate":{"offset":17.0,"max":-91.0,"scale":scale}}});
        input["constraints"][0]["mixRotate"] = json!(1.5);
        input["constraints"][0]["clamp"] = json!(false);
        let doc = import_json(&input, "Raw coefficients").unwrap();
        for target in [ExportTarget::SpineEditor43, ExportTarget::Runtime43] {
            let output = export_json_for(&doc, target).unwrap();
            assert_eq!(
                output["constraints"][0]["properties"],
                input["constraints"][0]["properties"]
            );
            assert_eq!(output["constraints"][0]["mixRotate"], 1.5);
            assert_eq!(import_json(&output, "Raw coefficients").unwrap(), doc);
        }
    }
}

#[test]
fn new_cross_mapping_does_not_enable_other_properties_or_local_additive_modes() {
    let baseline = import_json(&fixture(), "Bounded mapping").unwrap();
    for flag in ["localSource", "localTarget", "additive"] {
        let mut input = fixture();
        input["constraints"][0][flag] = json!(true);
        let mut doc = baseline.clone();
        match flag {
            "localSource" => transform(&mut doc).local_source = true,
            "localTarget" => transform(&mut doc).local_target = true,
            "additive" => transform(&mut doc).additive = true,
            _ => unreachable!(),
        }
        rejected(&input, &doc, "local and additive");
    }
    for (from, to) in [("y", "rotate"), ("x", "scaleX"), ("x", "y")] {
        let mut properties = serde_json::Map::new();
        let mut targets = serde_json::Map::new();
        targets.insert(to.into(), json!({"offset":-30,"max":150,"scale":2}));
        properties.insert(from.into(), json!({"offset":10,"to":targets}));
        let mut input = fixture();
        input["constraints"][0]["properties"] = Value::Object(properties);
        let mut doc = baseline.clone();
        transform(&mut doc).mappings = vec![TransformMapping {
            from: TransformProperty::from_spine_name(from).unwrap(),
            offset: 10.0,
            to: vec![TransformTarget {
                property: TransformProperty::from_spine_name(to).unwrap(),
                offset: -30.0,
                max: 150.0,
                scale: 2.0,
            }],
        }];
        rejected(&input, &doc, "only X/Y mappings");
    }
}

#[test]
fn rotate_has_one_writer_and_x_mapping_has_one_destination() {
    let baseline = import_json(&fixture(), "Single writer").unwrap();
    let mut duplicate = fixture();
    duplicate["constraints"][0]["clamp"] = json!(false);
    duplicate["constraints"][0]["properties"]["rotate"] = json!({"to":{"rotate":{}}});
    let mut duplicate_doc = baseline.clone();
    let value = transform(&mut duplicate_doc);
    value.clamp = false;
    value.mappings.push(TransformMapping {
        from: TransformProperty::Rotate,
        offset: 0.0,
        to: vec![TransformTarget::identity(TransformProperty::Rotate)],
    });
    rejected(&duplicate, &duplicate_doc, "exactly one writer");

    let mut multiple = fixture();
    multiple["constraints"][0]["properties"]["x"]["to"]["x"] = json!({});
    let mut multiple_doc = baseline;
    transform(&mut multiple_doc).mappings[0]
        .to
        .push(TransformTarget::identity(TransformProperty::X));
    rejected(&multiple, &multiple_doc, "only X/Y mappings");
}
