use peachy_document::FORMAT_VERSION;
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn source() -> Value {
    json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}],"slots":[{"name":"paint","bone":"root"}],"skins":[],"animations":{}})
}

#[test]
fn rgb_and_rgba_setup_colors_round_trip_as_exact_bytes_with_white_omitted() {
    for (input, expected) in [
        ("123456", [18, 52, 86, 255]),
        ("12345678", [18, 52, 86, 120]),
        ("FF000080", [255, 0, 0, 128]),
        ("fF80aA", [255, 128, 170, 255]),
        ("00000000", [0, 0, 0, 0]),
        ("FFFFFFFF", [255; 4]),
    ] {
        let mut value = source();
        value["slots"][0]["color"] = json!(input);
        let original = value.clone();
        assert!(inspect(&value).supported);
        let doc = import_json(&value, "Original slot tint").unwrap();
        assert_eq!(doc.format, FORMAT_VERSION);
        assert_eq!(doc.slots[0].color, expected);
        assert_eq!(value, original);
        let output = export_json(&doc).unwrap();
        if expected == [255; 4] {
            assert!(output["slots"][0].get("color").is_none());
        } else {
            let [r, g, b, a] = expected;
            assert_eq!(
                output["slots"][0]["color"],
                format!("{r:02x}{g:02x}{b:02x}{a:02x}")
            );
        }
        let imported = import_json(&output, "Original slot tint").unwrap();
        assert_eq!(imported, doc);
    }
    let doc = import_json(&source(), "White").unwrap();
    assert_eq!(doc.slots[0].color, [255; 4]);
    assert!(
        export_json(&doc).unwrap()["slots"][0]
            .get("color")
            .is_none()
    );
}

#[test]
fn malformed_setup_color_data_is_rejected_without_permissive_defaults() {
    for color in [
        Value::Null,
        json!(true),
        json!(7),
        json!([]),
        json!(""),
        json!("fff"),
        json!("12345"),
        json!("1234567"),
        json!("123456789"),
        json!("gg0000ff"),
        json!("ＦＦＦＦＦＦ"),
        json!("#123456"),
    ] {
        let mut value = source();
        value["slots"][0]["color"] = color;
        let original = value.clone();
        assert!(!inspect(&value).supported);
        assert!(import_json(&value, "Reject").is_err());
        assert_eq!(value, original);
    }
}

#[test]
fn dark_tint_and_slot_animation_continue_to_be_explicit_refusals() {
    for dark in [Value::Null, json!("000000"), json!(false)] {
        let mut value = source();
        value["slots"][0]["color"] = json!("12345678");
        value["slots"][0]["dark"] = dark;
        assert!(!inspect(&value).supported);
        assert!(import_json(&value, "Reject dark").is_err());
    }
    for timeline in [
        json!({"rgba":[{"time":0,"color":"ff000080"}]}),
        json!({"color":[]}),
        json!({"rgba2":[]}),
        json!({}),
    ] {
        let mut value = source();
        value["animations"] = json!({"clip":{"slots":{"paint":timeline}}});
        assert!(!inspect(&value).supported);
        assert!(import_json(&value, "Reject slot keys").is_err());
    }
}
