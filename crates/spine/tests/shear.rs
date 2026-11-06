use peachy_document::{Curve, FORMAT_VERSION, Property};
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture(timelines: Value) -> Value {
    json!({
        "skeleton":{"spine":"4.3.26"},
        "bones":[{"name":"root","shearX":11,"shearY":-7}],
        "slots":[{"name":"body","bone":"root","attachment":"card"}],
        "skins":[{"name":"default","attachments":{"body":{"card":{"width":16,"height":16}}}}],
        "animations":{"test":{"bones":{"root":timelines}}}
    })
}

#[test]
fn setup_shear_is_preserved_but_zero_bones_and_region_transforms_keep_old_export_shape() {
    let mut source = fixture(json!({}));
    source["bones"][0]["shearX"] = json!(-1080.25);
    source["bones"][0]["shearY"] = json!(720.5);
    assert!(inspect(&source).supported);
    let mut doc = import_json(&source, "original shear setup").unwrap();
    assert_eq!(doc.format, FORMAT_VERSION);
    assert_eq!(doc.bones[0].setup.shear_x, -1080.25);
    assert_eq!(doc.bones[0].setup.shear_y, 720.5);
    let output = export_json(&doc).unwrap();
    assert_eq!(output["bones"][0]["shearX"], -1080.25);
    assert_eq!(output["bones"][0]["shearY"], 720.5);
    assert_eq!(import_json(&output, &doc.name).unwrap(), doc);
    let region = &output["skins"][0]["attachments"]["body"]["card"];
    assert!(region.get("shearX").is_none() && region.get("shearY").is_none());
    doc.bones[0].setup.shear_x = 0.0;
    doc.bones[0].setup.shear_y = -0.0;
    let zero = export_json(&doc).unwrap();
    assert!(zero["bones"][0].get("shearX").is_none());
    assert!(zero["bones"][0].get("shearY").is_none());
    assert!(
        zero["animations"]["test"]["bones"]
            .as_object()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn observed_scalar_and_paired_shear_offsets_defaults_and_neutral_keys_roundtrip() {
    let scalar = fixture(json!({
        "shearx":[{"time":0,"value":0},{"time":1,"value":40}],
        "sheary":[{"time":0,"value":0},{"time":1,"value":-30}]
    }));
    let paired = fixture(json!({"shear":[{"time":0},{"time":1,"x":40,"y":-30}]}));
    let a = import_json(&scalar, "offsets").unwrap();
    let b = import_json(&paired, "offsets").unwrap();
    assert_eq!(a, b);
    assert_eq!(a.bones[0].setup.shear_x, 11.0);
    assert_eq!(a.bones[0].setup.shear_y, -7.0);
    assert_eq!(a.animations[0].tracks[0].property, Property::ShearX);
    assert_eq!(a.animations[0].tracks[0].keys[1].value, 40.0);
    assert_eq!(a.animations[0].tracks[1].keys[1].value, -30.0);
    let output = export_json(&b).unwrap();
    assert_eq!(
        output["animations"]["test"]["bones"]["root"],
        json!({
            "shearx":[{"time":0.0,"value":0.0},{"time":1.0,"value":40.0}],
            "sheary":[{"time":0.0,"value":0.0},{"time":1.0,"value":-30.0}]
        })
    );
    assert_eq!(import_json(&output, &b.name).unwrap(), b);

    let omitted_y = import_json(
        &fixture(json!({"shear":[{}, {"time":1,"x":40}]})),
        "default y",
    )
    .unwrap();
    assert!(
        omitted_y.animations[0].tracks[1]
            .keys
            .iter()
            .all(|k| k.value == 0.0)
    );
    let neutral = import_json(&fixture(json!({"shearx":[{}]})), "real frame zero").unwrap();
    assert_eq!(neutral.animations[0].duration(), 0.0);
    assert_eq!(
        export_json(&neutral).unwrap()["animations"]["test"]["bones"]["root"]["shearx"],
        json!([{"time":0.0,"value":0.0}])
    );
    let mut turn = fixture(json!({"sheary":[{"value":-720},{"time":1,"value":1080}]}));
    turn["bones"][0]["shearY"] = json!(360);
    let turn = import_json(&turn, "whole turns").unwrap();
    assert_eq!(turn.bones[0].setup.shear_y, 360.0);
    assert_eq!(turn.animations[0].tracks[0].keys[0].value, -720.0);
    assert_eq!(turn.animations[0].tracks[0].keys[1].value, 1080.0);
}

#[test]
fn paired_bezier_uses_independent_four_handle_channels_and_exports_scalar_curves() {
    // Original black-box data-contract probe: X and Y have different control times/values.
    let source = fixture(json!({"shear":[
        {"time":0,"x":0,"y":0,"curve":[0.2,0,0.8,0, 0.3,-30,0.9,-30]},
        {"time":1,"x":40,"y":-30}
    ]}));
    assert!(inspect(&source).supported);
    let doc = import_json(&source, "independent channels").unwrap();
    assert_eq!(
        doc.animations[0].tracks[0].keys[0].curve,
        Curve::Bezier([0.2, 0.0, 0.8, 0.0])
    );
    assert_eq!(
        doc.animations[0].tracks[1].keys[0].curve,
        Curve::Bezier([0.3, 1.0, 0.9, 1.0])
    );
    let output = export_json(&doc).unwrap();
    let keys = &output["animations"]["test"]["bones"]["root"];
    assert_eq!(
        keys["shearx"][0]["curve"],
        json!([0.2_f32, 0.0, 0.8_f32, 0.0])
    );
    assert_eq!(
        keys["sheary"][0]["curve"],
        json!([0.3_f32, -30.0, 0.9_f32, -30.0])
    );
    assert_eq!(import_json(&output, &doc.name).unwrap(), doc);
    let stepped = import_json(
        &fixture(json!({"sheary":[{"value":-30,"curve":"stepped"},{"time":1,"value":10}]})),
        "stepped shear",
    )
    .unwrap();
    assert_eq!(
        stepped.animations[0].tracks[0].keys[0].curve,
        Curve::Stepped
    );
}

#[test]
fn malformed_or_conflicting_shear_channels_never_silently_default_or_discard_data() {
    for timelines in [
        json!({"shearX":[{"value":40}]}),
        json!({"shearY":[{"value":-30}]}),
        json!({"shearx":[{"x":40}]}),
        json!({"sheary":[{"y":-30}]}),
        json!({"shear":[{"value":40}]}),
        json!({"shear":[],"shearx":[]}),
        json!({"shear":[],"sheary":[]}),
        json!({"shear":[{}, {"time":1,"x":40}],"shearx":[{}]}),
        json!({"shearx":[{"value":"40"}]}),
        json!({"sheary":[{"value":1e100}]}),
        json!({"shear":[{"x":1e100}]}),
        json!({"shearx":{}}),
    ] {
        let source = fixture(timelines);
        assert!(!inspect(&source).supported, "{source}");
        assert!(import_json(&source, "invalid shear").is_err(), "{source}");
    }
    for (name, valid_count) in [("shear", 8), ("shearx", 4), ("sheary", 4)] {
        let end = if name == "shear" {
            json!({"time":1,"x":40,"y":-30})
        } else {
            json!({"time":1,"value":40})
        };
        let mut timelines = json!({});
        timelines[name] = json!([{"curve":vec![0.25;valid_count]},end]);
        let valid = fixture(timelines);
        assert!(inspect(&valid).supported);
        assert!(import_json(&valid, "valid curve shape control").is_ok());
        for count in [0, 3, 4, 5, 7, 8, 9] {
            if count == valid_count {
                continue;
            }
            let handles: Vec<_> = (0..count).map(|_| json!(0.25)).collect();
            let mut source = valid.clone();
            source["animations"]["test"]["bones"]["root"][name][0]["curve"] = json!(handles);
            assert!(!inspect(&source).supported, "{name} with {count} handles");
            assert!(
                inspect(&source)
                    .unsupported
                    .iter()
                    .any(|issue| issue.contains("curve"))
            );
            assert!(import_json(&source, "truncated/extra curve").is_err());
        }
    }
    let equal_overshoot =
        fixture(json!({"shearx":[{"value":0,"curve":[0.2,40,0.8,40]},{"time":1,"value":0}]}));
    assert!(import_json(&equal_overshoot, "unsupported equal-value overshoot").is_err());
    let mut unordered = fixture(json!({"shearx":[{"time":1,"value":0},{"time":0,"value":10}]}));
    assert!(import_json(&unordered, "unordered").is_err());
    unordered["animations"]["test"]["bones"]["root"]["shearx"] =
        json!([{"time":0,"value":0,"curve":[0.8,0,0.2,10]},{"time":1,"value":10}]);
    assert!(import_json(&unordered, "unordered handles").is_err());
}

#[test]
fn bone_finiteness_and_region_only_fields_keep_their_distinct_interchange_boundary() {
    for field in ["shearX", "shearY"] {
        for bad in [json!("0"), Value::Null, json!(1e100)] {
            let mut source = fixture(json!({}));
            source["bones"][0][field] = bad;
            assert!(!inspect(&source).supported);
            assert!(import_json(&source, "bad bone shear").is_err());
        }
        for value in [json!(0), json!(25)] {
            let mut region = fixture(json!({}));
            region["skins"][0]["attachments"]["body"]["card"][field] = value;
            assert!(!inspect(&region).supported);
            assert!(import_json(&region, "region shear unsupported").is_err());
        }
    }
    let mut inherited = fixture(json!({}));
    inherited["bones"][0]["inherit"] = json!("noScale");
    assert!(!inspect(&inherited).supported);
    assert!(import_json(&inherited, "shear does not add inheritance modes").is_err());
}
