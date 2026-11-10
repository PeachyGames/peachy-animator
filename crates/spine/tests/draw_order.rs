//! Draw order keys: offsets kept as written; invalid lists are refused.
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    let slots: Vec<Value> = (0..6)
        .map(|i| json!({"name":format!("s{i}"),"bone":"root"}))
        .collect();
    json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}],"slots":slots,"skins":[],
        "animations":{"anim":{"drawOrder":[
            {"offsets":[{"slot":"s0","offset":3}]},
            {"time":0.5,"offsets":[{"slot":"s1","offset":4},{"slot":"s2","offset":0},{"slot":"s4","offset":-4}]},
            {"time":1}]}}})
}

#[test]
fn offsets_roundtrip_and_evaluate_with_unlisted_slots_in_setup_order() {
    let input = fixture();
    assert!(
        inspect(&input).supported,
        "{:?}",
        inspect(&input).unsupported
    );
    let doc = import_json(&input, "Order").unwrap();
    let keys = &doc.animations[0].draw_order;
    assert_eq!(keys[0].order(6).unwrap(), [1, 2, 3, 0, 4, 5]);
    // Listed: s1→5, s2→2, s4→0; unlisted s0, s3, s5 fill 1, 3, 4.
    assert_eq!(keys[1].order(6).unwrap(), [4, 0, 2, 3, 5, 1]);
    assert_eq!(keys[2].order(6).unwrap(), [0, 1, 2, 3, 4, 5]);
    assert_eq!(doc.animations[0].duration(), 1.);
    let output = export_json(&doc).unwrap();
    assert_eq!(
        output["animations"]["anim"]["drawOrder"][2],
        json!({"time":1.0})
    );
    assert_eq!(
        output["animations"]["anim"]["drawOrder"][1]["offsets"][1],
        json!({"slot":"s2","offset":0})
    );
    assert_eq!(import_json(&output, "Order").unwrap(), doc);
}

#[test]
fn offsets_the_runtime_cannot_read_are_refused() {
    for (offsets, reason) in [
        (
            json!([{"slot":"s4","offset":-4},{"slot":"s1","offset":2}]),
            "in setup order",
        ),
        (
            json!([{"slot":"s1","offset":2},{"slot":"s1","offset":3}]),
            "in setup order",
        ),
        (
            json!([{"slot":"s0","offset":3},{"slot":"s1","offset":2}]),
            "distinct positions",
        ),
        (json!([{"slot":"s5","offset":1}]), "distinct positions"),
        (json!([{"slot":"s2","offset":-3}]), "distinct positions"),
        (json!([{"slot":"s2","offset":1.5}]), "integer"),
        (json!([{"slot":"ghost","offset":1}]), "known slot"),
    ] {
        let mut input = fixture();
        input["animations"]["anim"]["drawOrder"] = json!([{"time":0,"offsets":offsets}]);
        let error = import_json(&input, "Bad").unwrap_err();
        assert!(error.contains(reason), "expected {reason:?}, got {error}");
    }
    let mut folders = fixture();
    folders["animations"]["anim"]["drawOrder"] = json!({"folder":[]});
    assert!(
        import_json(&folders, "Bad")
            .unwrap_err()
            .contains("folders are unsupported")
    );
    let mut unordered = fixture();
    unordered["animations"]["anim"]["drawOrder"] = json!([{"time":1},{"time":0.5}]);
    assert!(
        import_json(&unordered, "Bad")
            .unwrap_err()
            .contains("increasing")
    );
}
