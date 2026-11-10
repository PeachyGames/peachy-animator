//! Path attachments, path constraints and position/spacing timelines.
use peachy_document::{Attachment, Constraint, PathProperty, PositionMode, SpacingMode};
use peachy_spine::{export_json, import_json, inspect};
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"skeleton":{"spine":"4.3.26"},
        "bones":[{"name":"root"},{"name":"a","parent":"root","length":10},{"name":"b","parent":"a","length":8,"x":10}],
        "slots":[{"name":"p","bone":"root","attachment":"rail"}],
        "skins":[{"name":"default","attachments":{"p":{"rail":{"type":"path","vertexCount":6,
            "vertices":[-10,0,0,0,30,80,70,-40,100,10,110,10],"lengths":[129,129]}}}}],
        "constraints":[{"type":"path","name":"ride","slot":"p","bones":["a","b"],"spacingMode":"fixed","position":0.2,"spacing":12,"rotation":15,"mixY":0.5}],
        "animations":{"anim":{"path":{"ride":{"position":[{"value":0},{"time":1,"value":1.2}],"spacing":[{"time":0.5}]}}}}})
}

#[test]
fn paths_constraints_and_timelines_roundtrip_with_observed_defaults() {
    let input = fixture();
    assert!(
        inspect(&input).supported,
        "{:?}",
        inspect(&input).unsupported
    );
    let doc = import_json(&input, "Path").unwrap();
    let Attachment::Path(path) = &doc.skins["default"]["p"]["rail"] else {
        panic!("path")
    };
    assert!(!path.closed && path.constant_speed);
    let Constraint::Path(c) = &doc.constraints[0] else {
        panic!("constraint")
    };
    assert_eq!(
        (c.position_mode, c.spacing_mode, c.offset_rotation),
        (PositionMode::Percent, SpacingMode::Fixed, 15.)
    );
    assert_eq!(
        (c.setup.mix_rotate, c.setup.mix_x, c.setup.mix_y),
        (1., 1., 0.5)
    );
    let tracks = &doc.animations[0].path_tracks;
    let spacing = tracks
        .iter()
        .find(|t| t.property == PathProperty::Spacing)
        .unwrap();
    assert_eq!(spacing.keys[0].value, 0., "omitted spacing value is 0");
    assert_eq!(
        import_json(&export_json(&doc).unwrap(), "Path").unwrap(),
        doc
    );
}

#[test]
fn unsupported_path_features_are_refused() {
    for (target, patch, reason) in [
        (
            "constraint",
            json!({"rotateMode":"chain"}),
            "chain rotation",
        ),
        (
            "constraint",
            json!({"spacingMode":"proportional"}),
            "proportional spacing",
        ),
        ("attachment", json!({"closed":true}), "closed paths"),
        (
            "attachment",
            json!({"constantSpeed":false}),
            "constant speed",
        ),
        (
            "timeline",
            json!({"mix":[{"time":0}]}),
            "path mix timelines",
        ),
    ] {
        let mut input = fixture();
        let node = match target {
            "constraint" => &mut input["constraints"][0],
            "attachment" => &mut input["skins"][0]["attachments"]["p"]["rail"],
            _ => &mut input["animations"]["anim"]["path"]["ride"],
        };
        for (key, value) in patch.as_object().unwrap() {
            node[key] = value.clone();
        }
        let report = inspect(&input);
        assert!(!report.supported, "{target} {patch}");
        assert!(
            report.unsupported.join(" ").contains(reason),
            "{reason}: {:?}",
            report.unsupported
        );
    }
}
