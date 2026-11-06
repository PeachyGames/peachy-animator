use peachy_document::Curve;
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture(timeline: &str, first: f32, last: f32, controls: [f32; 4]) -> Value {
    json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}],"slots":[],"skins":[],
    "animations":{"range":{"bones":{"root":{(timeline):[
        {"time":0,"value":first,"curve":controls},{"time":1,"value":last}
    ]}}}}})
}

#[test]
fn finite_control_conversion_uses_wider_intermediates_only_when_needed() {
    for timeline in ["shearx", "translatex", "rotate"] {
        for (first, last, handles, expected) in [
            (-2e38, 2e38, [0.2, -1e38, 0.8, 1e38], [0.2, 0.25, 0.8, 0.75]),
            (-3e38, -1e38, [0.2, 1e38, 0.8, 1e38], [0.2, 2.0, 0.8, 2.0]),
        ] {
            let source = fixture(timeline, first, last, handles);
            assert!(inspect(&source).supported);
            let doc = import_json(&source, "wide finite controls").unwrap();
            assert_eq!(
                doc.animations[0].tracks[0].keys[0].curve,
                Curve::Bezier(expected)
            );
            let output = export_json(&doc).unwrap();
            assert!(!serde_json::to_string(&output).unwrap().contains("null"));
            assert!(inspect(&output).supported);
            assert_eq!(import_json(&output, &doc.name).unwrap(), doc);
        }
    }
    let paired = json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}],"slots":[],"skins":[],
    "animations":{"range":{"bones":{"root":{"shear":[
        {"time":0,"x":-2e38_f32,"y":2e38_f32,"curve":[0.2,-1e38_f32,0.8,1e38_f32,0.2,1e38_f32,0.8,-1e38_f32]},
        {"time":1,"x":2e38_f32,"y":-2e38_f32}
    ]}}}}});
    let doc = import_json(&paired, "wide paired controls").unwrap();
    assert!(
        doc.animations[0]
            .tracks
            .iter()
            .all(|t| t.keys[0].curve == Curve::Bezier([0.2, 0.25, 0.8, 0.75]))
    );
    let output = export_json(&doc).unwrap();
    assert!(!serde_json::to_string(&output).unwrap().contains("null"));
    assert_eq!(import_json(&output, &doc.name).unwrap(), doc);
}

#[test]
fn genuinely_unrepresentable_controls_fail_without_null_or_clamping() {
    let source = fixture("shearx", 0.0, 1e-7, [0.2, f32::MAX, 0.8, f32::MAX]);
    let unchanged = source.clone();
    assert!(
        import_json(&source, "unrepresentable normalized controls")
            .unwrap_err()
            .contains("Bezier normalization exceeds")
    );
    assert_eq!(source, unchanged);

    let mut native = import_json(
        &fixture("shearx", 0.0, 2.0, [0.2, 0.5, 0.8, 1.5]),
        "unrepresentable target controls",
    )
    .unwrap();
    native.animations[0].tracks[0].keys[0].curve = Curve::Bezier([0.2, f32::MAX, 0.8, f32::MAX]);
    assert!(native.validate().is_ok());
    let unchanged = native.clone();
    assert!(
        export_json(&native)
            .unwrap_err()
            .contains("Bezier export handle exceeds")
    );
    assert_eq!(native, unchanged);
}
