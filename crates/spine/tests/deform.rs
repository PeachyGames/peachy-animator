//! Mesh deform keys: sparse JSON, dense native, observed curve and value rules.
use peachy_document::Curve;
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"skeleton":{"spine":"4.3.26"},
        "bones":[{"name":"root"},{"name":"a","parent":"root","x":10},{"name":"b","parent":"a","x":20}],
        "slots":[{"name":"plain","bone":"a","attachment":"quad"},{"name":"bent","bone":"a","attachment":"limb"},
            {"name":"card","bone":"a","attachment":"card"}],
        "skins":[{"name":"default","attachments":{
            "plain":{"quad":{"type":"mesh","uvs":[0,1,1,1,1,0,0,0],"triangles":[0,1,2,2,3,0],"vertices":[-5,-5,5,-5,5,5,-5,5],"hull":4}},
            "bent":{"limb":{"type":"mesh","uvs":[0,1,1,1,1,0],"triangles":[0,1,2],"vertices":[1,1,0,0,1, 2,1,5,0,0.5,2,5,0,0.5, 1,2,0,6,1],"hull":3}},
            "card":{"card":{"width":10,"height":10}}}}],
        "animations":{"anim":{"attachments":{"default":{
            "plain":{"quad":{"deform":[{"offset":2,"vertices":[1,2,3,4],"curve":[0.3,0.2,0.6,0.9]},{"time":1,"vertices":[0,0,10,0,0,10,-10,0]},{"time":2}]}},
            "bent":{"limb":{"deform":[{},{"time":1,"offset":1,"vertices":[5,6,7],"curve":"stepped"},{"time":1.5}]}}}}}}})
}

#[test]
fn sparse_keys_become_dense_and_export_like_the_editor() {
    let input = fixture();
    let report = inspect(&input);
    assert!(report.supported, "{:?}", report.unsupported);
    let doc = import_json(&input, "Deform").unwrap();
    let tracks = &doc.animations[0].deform_tracks;
    let quad = tracks
        .iter()
        .find(|t| t.attachment.slot == "plain")
        .unwrap();
    assert_eq!(quad.keys[0].offsets, [0., 0., 1., 2., 3., 4., 0., 0.]);
    // Bezier x is absolute time in JSON, normalized natively; y stays a fraction.
    assert_eq!(quad.keys[0].curve, Curve::Bezier([0.3, 0.2, 0.6, 0.9]));
    assert_eq!(quad.keys[2].offsets, [0.; 8]);
    let limb = tracks.iter().find(|t| t.attachment.slot == "bent").unwrap();
    // Weighted: two values per influence (1 + 2 + 1 influences).
    assert_eq!(limb.keys[1].offsets, [0., 5., 6., 7., 0., 0., 0., 0.]);
    assert_eq!(limb.keys[1].curve, Curve::Stepped);
    assert_eq!(doc.animations[0].duration(), 2.);
    let output = export_json(&doc).unwrap();
    let written =
        &output["animations"]["anim"]["attachments"]["default"]["plain"]["quad"]["deform"];
    assert_eq!(written[1]["offset"], json!(2));
    assert_eq!(written[1]["vertices"].as_array().unwrap().len(), 5);
    assert!(written[2].get("vertices").is_none());
    assert_eq!(import_json(&output, "Deform").unwrap(), doc);
}

fn reject(value: &Value, reason: &str) {
    assert!(!inspect(value).supported, "Accepted: {value}");
    let error = import_json(value, "Rejected").unwrap_err();
    assert!(error.contains(reason), "Expected {reason:?}, got {error}");
}

#[test]
fn malformed_or_unsupported_deform_data_is_refused() {
    let base = fixture();
    let set = |path: &[&str], value: Value| {
        let mut v = base.clone();
        let mut node = &mut v["animations"]["anim"]["attachments"]["default"];
        for key in path {
            node = match key.parse::<usize>() {
                Ok(index) => &mut node[index],
                Err(_) => &mut node[*key],
            };
        }
        *node = value;
        v
    };
    reject(
        &set(&["plain", "quad", "deform", "0", "offset"], json!(7)),
        "exceeds",
    );
    reject(
        &set(&["card"], json!({"card":{"deform":[{"time":0}]}})),
        "non-mesh",
    );
    reject(
        &set(&["plain"], json!({"missing":{"deform":[{"time":0}]}})),
        "unknown attachment",
    );
    reject(
        &set(&["plain", "quad"], json!({"sequence":[{"time":0}]})),
        "sequence timelines are unsupported",
    );
    reject(
        &set(
            &["plain", "quad", "deform"],
            json!([{"time":0,"vertices":["x"]}]),
        ),
        "finite",
    );
    reject(
        &set(&["plain", "quad", "deform"], json!([{"time":0,"scale":1}])),
        "unsupported field scale",
    );
}
