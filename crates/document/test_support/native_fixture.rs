//! Test-only builder for the frozen v22 original examples. Production file
//! compatibility is exercised through peachy_io::decode_project and its tests.
//! Never relax Document deserialization or rewrite historical fixture bytes.
pub fn decode(source: &str) -> Result<peachy_document::Document, serde_json::Error> {
    let mut value: serde_json::Value = serde_json::from_str(source)?;
    assert_eq!(
        value["format"], 22,
        "Only the frozen v22 examples belong here"
    );
    value["format"] = peachy_document::FORMAT_VERSION.into();
    for animation in value["animations"]
        .as_array_mut()
        .expect("Fixture animations")
    {
        assert!(animation.get("slider_tracks").is_none());
        animation
            .as_object_mut()
            .expect("Fixture animation")
            .insert("slider_tracks".into(), serde_json::json!([]));
    }
    serde_json::from_value(value)
}
