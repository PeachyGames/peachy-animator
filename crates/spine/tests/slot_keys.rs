//! Slot attachment and color keys: observed omitted values, per-channel curves.
use peachy_document::Curve;
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}],
        "slots":[{"name":"s","bone":"root","attachment":"a","color":"ff8040c0"},{"name":"t","bone":"root","attachment":"a"}],
        "skins":[{"name":"default","attachments":{"s":{"a":{"width":10,"height":10},"b":{"width":10,"height":10}},"t":{"a":{"width":10,"height":10}}}}],
        "animations":{"anim":{"slots":{
            "s":{"attachment":[{"time":0.5,"name":"b"},{"time":1},{"time":1.5,"name":"a"}],
                 "rgba":[{"time":0.25,"color":"00000000","curve":[0.35,0,0.45,1, 0.35,0,0.45,0.5, 0.3,0,0.6,1, 0.3,0,0.6,1]},
                         {"time":0.75,"color":"ffffffff","curve":"stepped"},{"time":1.25,"color":"ff0000ff"}]},
            "t":{"attachment":[{"name":null}]}}}}})
}

#[test]
fn attachment_and_color_keys_roundtrip() {
    let input = fixture();
    let report = inspect(&input);
    assert!(report.supported, "{:?}", report.unsupported);
    let doc = import_json(&input, "Slots").unwrap();
    let animation = &doc.animations[0];
    let s = animation
        .attachment_tracks
        .iter()
        .find(|t| t.slot == 0)
        .unwrap();
    assert_eq!(s.keys[1].attachment, None);
    assert_eq!(s.keys[0].attachment.as_deref(), Some("b"));
    let t = animation
        .attachment_tracks
        .iter()
        .find(|t| t.slot == 1)
        .unwrap();
    assert_eq!((t.keys[0].time, t.keys[0].attachment.clone()), (0., None));
    let color = &animation.color_tracks[0];
    assert_eq!(color.keys[0].color, [0, 0, 0, 0]);
    // Per-channel Bezier: absolute time/value become normalized handles.
    let Curve::Bezier(handles) = color.keys[0].curves[1] else {
        panic!("bezier")
    };
    for (actual, expected) in handles.iter().zip([0.2, 0., 0.4, 0.5]) {
        assert!((actual - expected).abs() < 1e-6, "{handles:?}");
    }
    assert_eq!(color.keys[1].curves, [Curve::Stepped; 4]);
    let output = export_json(&doc).unwrap();
    assert_eq!(
        output["animations"]["anim"]["slots"]["s"]["attachment"][1],
        json!({"time":1.0})
    );
    assert_eq!(import_json(&output, "Slots").unwrap(), doc);
}

#[test]
fn unsupported_or_malformed_slot_keys_are_refused() {
    for (timeline, keys, reason) in [
        (
            "rgb",
            json!([{"color":"ffffff"}]),
            "rgb timelines are unsupported",
        ),
        (
            "alpha",
            json!([{"value":0.5}]),
            "alpha timelines are unsupported",
        ),
        ("rgba", json!([{"time":0}]), "rgba keys need a color"),
        ("rgba", json!([{"color":"zz0000ff"}]), "hex digits"),
        ("attachment", json!([{"time":0,"name":7}]), "string or null"),
        ("attachment", json!([{"time":1},{"time":0.5}]), "ordered"),
    ] {
        let mut input = fixture();
        input["animations"]["anim"]["slots"]["s"] = json!({timeline: keys});
        let error = import_json(&input, "Bad").unwrap_err();
        assert!(
            error.contains(reason),
            "{timeline}: expected {reason:?}, got {error}"
        );
    }
    let mut unknown = fixture();
    unknown["animations"]["anim"]["slots"]["ghost"] = json!({"attachment":[{"time":0}]});
    assert!(
        import_json(&unknown, "Bad")
            .unwrap_err()
            .contains("unknown slot ghost")
    );
}

#[test]
fn slot_flat_color_beziers_keep_time_handles_and_subbyte_value_offsets() {
    let mut input = fixture();
    let value = f32::from(128u8) / 255.;
    let mut handles = vec![];
    for channel in 0..4 {
        let offset = if channel == 1 { 0.000001f32 } else { 0.0 };
        handles.extend([0.1f32, value + offset, 0.4, value - offset]);
    }
    input["animations"]["anim"]["slots"]["s"]["rgba"] = json!([
        {"time":0.0,"color":"80808080","curve":handles},
        {"time":0.5,"color":"80808080"}
    ]);
    let doc = import_json(&input, "Flat slot curves").unwrap();
    let curves = doc.animations[0].color_tracks[0].keys[0].curves;
    for (channel, curve) in curves.iter().enumerate() {
        let Curve::BezierOffset([x1, y1, x2, y2]) = curve else {
            panic!("Flat channel {channel} lost its authored Bezier: {curve:?}")
        };
        assert!((*x1 - 0.2).abs() < 1e-6 && (*x2 - 0.8).abs() < 1e-6);
        if channel == 1 {
            assert!(*y1 > 0. && *y2 < 0.);
        } else {
            assert_eq!((*y1, *y2), (0., 0.));
        }
    }
    let output = export_json(&doc).unwrap();
    let actual = output["animations"]["anim"]["slots"]["s"]["rgba"][0]["curve"]
        .as_array()
        .unwrap();
    for (value, expected) in actual.iter().zip(handles) {
        assert!((value.as_f64().unwrap() - f64::from(expected)).abs() < 1e-7);
    }
    assert_eq!(import_json(&output, "Flat slot curves").unwrap(), doc);
}
