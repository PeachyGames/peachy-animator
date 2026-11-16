use super::*;
use peachy_document::*;
use std::path::Path;

fn copy_input(keys: Value) -> Value {
    json!({"expected_revision":7,"animation":0,"keys":keys})
}

fn bone_key(bone: usize, time: f32) -> Value {
    json!({"type":"bone","bone":bone,"property":"x","time":time})
}

#[test]
fn group_copy_schema_and_decoder_require_exact_object_identities() {
    let shape = schema::copy_animation_key_group_schema();
    assert_eq!(
        shape["required"],
        json!(["expected_revision", "animation", "keys"])
    );
    assert_eq!(shape["additionalProperties"], false);
    assert_eq!(shape["properties"]["keys"]["minItems"], 1);
    assert_eq!(shape["properties"]["keys"]["uniqueItems"], true);
    let keys = json!([
        {"type":"bone","bone":0,"property":"shearY","time":0.125},
        {"type":"ik","constraint":1,"time":0.25},
        {"type":"slider","constraint":0,"property":"mix","time":0.375},
        {"type":"deform","attachment":{"skin":"default","slot":"cloth","placeholder":"mesh"},"time":0.5},
        {"type":"slotAttachment","slot":0,"time":0.625},
        {"type":"slotColor","slot":0,"time":0.75},
        {"type":"drawOrder","time":0.875}
    ]);
    let decoded: CopyAnimationKeyGroupInput =
        serde_json::from_value(copy_input(keys.clone())).unwrap();
    assert_eq!(
        decoded
            .keys
            .into_iter()
            .map(|key| serde_json::to_value(key.0).unwrap())
            .collect::<Vec<_>>(),
        *keys.as_array().unwrap()
    );
    for field in ["expected_revision", "animation", "keys"] {
        let mut missing = copy_input(keys.clone());
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<CopyAnimationKeyGroupInput>(missing).is_err());
        let mut null = copy_input(keys.clone());
        null[field] = Value::Null;
        assert!(serde_json::from_value::<CopyAnimationKeyGroupInput>(null).is_err());
    }
    for invalid in [json!([7, 0, []]), json!(null), json!("copy"), json!(true)] {
        assert!(serde_json::from_value::<CopyAnimationKeyGroupInput>(invalid).is_err());
    }
    for key in [
        json!([]),
        json!([0, "x", 0.0]),
        json!(null),
        json!(true),
        json!("bone"),
        json!({"type":"bone","bone":0,"property":{"x":null},"time":0.0}),
        json!({"type":"bone","bone":0,"property":"future","time":0.0}),
        json!({"type":"bone","bone":0,"property":"x"}),
        json!({"type":"bone","bone":-1,"property":"x","time":0.0}),
        json!({"type":"bone","bone":0,"property":"x","time":null}),
        json!({"type":"bone","bone":0,"property":"x","time":0.0,"future":0}),
        json!({"type":"slider","constraint":0,"property":{"time":null},"time":0.0}),
        json!({"type":"deform","attachment":["default","cloth","mesh"],"time":0.0}),
        json!({"type":"deform","attachment":{"skin":"default","slot":"cloth"},"time":0.0}),
        json!({"type":"deform","attachment":{"skin":"default","slot":"cloth","placeholder":null},"time":0.0}),
        json!({"type":"deform","attachment":{"skin":"default","slot":"cloth","placeholder":"mesh","future":0},"time":0.0}),
        json!({"type":"future","time":0.0}),
    ] {
        assert!(
            serde_json::from_value::<CopyAnimationKeyGroupInput>(copy_input(json!([key.clone()])))
                .is_err(),
            "Accepted {key}"
        );
    }
}

#[test]
fn group_copy_raw_duplicate_fields_reject_before_sdk_value_materialization() {
    // Raw duplicates are a decoder check; tools/call materializes a JSON object
    // before dispatch, so the real-stdio test does not claim to preserve them.
    for raw in [
        r#"{"expected_revision":7,"expected_revision":7,"animation":0,"keys":[]}"#,
        r#"{"expected_revision":7,"animation":0,"animation":0,"keys":[]}"#,
        r#"{"expected_revision":7,"animation":0,"keys":[],"keys":[]}"#,
        r#"{"expected_revision":7,"animation":0,"keys":[{"type":"bone","type":"bone","bone":0,"property":"x","time":0}]}"#,
        r#"{"expected_revision":7,"animation":0,"keys":[{"type":"bone","bone":0,"bone":1,"property":"x","time":0}]}"#,
        r#"{"expected_revision":7,"animation":0,"keys":[{"type":"bone","bone":0,"property":"x","property":"y","time":0}]}"#,
        r#"{"expected_revision":7,"animation":0,"keys":[{"type":"bone","bone":0,"property":"x","time":0,"time":1}]}"#,
        r#"{"expected_revision":7,"animation":0,"keys":[{"type":"deform","attachment":{"skin":"default","skin":"other","slot":"cloth","placeholder":"mesh"},"time":0}]}"#,
    ] {
        assert!(
            serde_json::from_str::<CopyAnimationKeyGroupInput>(raw).is_err(),
            "Accepted {raw}"
        );
    }
}

fn paste_command() -> Value {
    json!({"command":"pasteAnimationKeyGroup","animation":0,"time":1.0,
        "clipboard":{"entries":[
            {"type":"bone","keys":[{"property":"x","key":{"time":0.0,"value":3.0,"curve":"linear"}}]},
            {"type":"slot","attachment_keys":[{"time":0.25,"attachment":null}],"color_keys":[]}
        ]},"targets":[{"entry":0,"target":{"type":"bone","bone":0}},{"entry":1,"target":{"type":"slot","slot":0}}]})
}

#[test]
fn group_paste_schema_and_engine_decoder_keep_nested_objects_strict() {
    let schema = schema::command_schema();
    let shape = schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|shape| shape["properties"]["command"]["const"] == "pasteAnimationKeyGroup")
        .unwrap();
    assert_eq!(shape["additionalProperties"], false);
    assert_eq!(
        shape["properties"]["clipboard"]["properties"]["entries"]["minItems"],
        1
    );
    assert_eq!(
        shape["properties"]["targets"]["items"]["required"],
        json!(["entry", "target"])
    );
    assert_eq!(
        shape["properties"]["targets"]["items"]["additionalProperties"],
        false
    );
    let valid = paste_command();
    let decoded: schema::CommandInput = serde_json::from_value(valid.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded.0).unwrap(), valid);
    for (path, invalid) in [
        ("/clipboard", json!([[]])),
        ("/clipboard/entries/0", json!([[]])),
        ("/clipboard/entries/0/keys/0", json!(["x", {}])),
        ("/clipboard/entries/0/keys/0/property", json!({"x":null})),
        (
            "/clipboard/entries/0/keys/0/key",
            json!([0.0, 3.0, "linear"]),
        ),
        (
            "/clipboard/entries/0/keys/0/key/curve",
            json!({"linear":null}),
        ),
        (
            "/clipboard/entries/1/attachment_keys/0",
            json!([0.25, null]),
        ),
        ("/targets/0", json!([0,{"type":"bone","bone":0}])),
        ("/targets/0/target", json!(["bone", 0])),
        ("/targets/0/entry", Value::Null),
    ] {
        let mut bad = valid.clone();
        *bad.pointer_mut(path).unwrap() = invalid;
        assert!(
            serde_json::from_value::<schema::CommandInput>(bad).is_err(),
            "Accepted {path}"
        );
    }
    for field in ["animation", "time", "clipboard", "targets"] {
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<schema::CommandInput>(missing).is_err());
    }
    for raw in [
        r#"{"command":"pasteAnimationKeyGroup","animation":0,"time":0,"clipboard":{"entries":[],"entries":[]},"targets":[]}"#,
        r#"{"command":"pasteAnimationKeyGroup","animation":0,"time":0,"clipboard":{"entries":[]},"targets":[{"entry":0,"entry":1,"target":{"type":"bone","bone":0}}]}"#,
        r#"{"command":"pasteAnimationKeyGroup","animation":0,"time":0,"clipboard":{"entries":[]},"targets":[{"entry":0,"target":{"type":"deform","attachment":{"skin":"default","slot":"cloth","placeholder":"mesh","placeholder":"other"}}}]}"#,
    ] {
        assert!(
            serde_json::from_str::<schema::CommandInput>(raw).is_err(),
            "Accepted {raw}"
        );
    }
}

#[test]
fn group_copy_is_read_only_with_pending_redo_and_source_preferences() {
    let mut fixture = Document::new("Original group copy".into(), 60).unwrap();
    fixture.bones.push(Bone {
        name: "driver".into(),
        parent: Some(0),
        length: 10.,
        setup: Transform::default(),
        inherit: Inherit::Normal,
    });
    fixture.animations = vec![
        Animation::new(
            "host",
            vec![
                Track {
                    bone: 0,
                    property: Property::X,
                    keys: vec![Keyframe {
                        time: 0.125,
                        value: 3.,
                        curve: Curve::BezierOffset([0.2, -0., 0.8, 0.5]),
                    }],
                },
                Track {
                    bone: 1,
                    property: Property::X,
                    keys: vec![Keyframe {
                        time: 0.25,
                        value: 4.,
                        curve: Curve::Linear,
                    }],
                },
            ],
        ),
        Animation::new(
            "reference",
            vec![Track {
                bone: 1,
                property: Property::X,
                keys: vec![Keyframe {
                    time: 1.5,
                    value: 0.,
                    curve: Curve::Linear,
                }],
            }],
        ),
    ];
    fixture.constraints = vec![Constraint::Slider(SliderConstraint::new("slider", 1))];
    let mut session = Session::new(fixture).unwrap();
    session.set_slider_source(0, Some(1)).unwrap();
    session
        .set_slider_source_property(0, TransformProperty::ScaleX)
        .unwrap();
    session
        .edit_slider_pose(
            0,
            peachy_engine::EditTarget::Animate {
                animation: 0,
                time: 0.,
            },
            &[peachy_engine::SliderPoseEdit::Mix { value: 0.625 }],
            false,
        )
        .unwrap();
    session
        .dispatch(Command::RenameBone {
            bone: 0,
            name: "temporary".into(),
        })
        .unwrap();
    session.undo();
    assert!(!session.pose_edits().is_empty());
    let pending = serde_json::to_value(session.pose_edits()).unwrap();
    let preferences = session.slider_source_preferences().clone();
    let focus = session.slider_source_focus();
    let effects = session.last_edit_effects().clone();
    let server = AnimatorServer::new(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
    {
        let mut state = server.state.lock().unwrap();
        state.revision = 7;
        state.project = Some(WorkingProject {
            session,
            source: None,
            asset_root: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        });
    }
    let before = server.get_document().structured_content.unwrap();
    let input = copy_input(json!([bone_key(1, 0.25), bone_key(0, 0.125)]));
    let copied = server
        .copy_animation_key_group(Parameters(serde_json::from_value(input.clone()).unwrap()))
        .structured_content
        .unwrap();
    assert_eq!(copied["revision"], 7);
    assert_eq!(
        copied["sourceOwners"],
        json!([{"type":"bone","bone":1},{"type":"bone","bone":0}])
    );
    assert_eq!(
        copied["clipboard"]["entries"][0]["keys"][0]["key"]["time"],
        0.125
    );
    assert_eq!(
        copied["clipboard"]["entries"][1]["keys"][0]["key"]["time"],
        0.0
    );
    assert_eq!(server.get_document().structured_content.unwrap(), before);
    for invalid in [
        copy_input(json!([])),
        copy_input(json!([bone_key(0, 0.125), bone_key(0, 0.125)])),
        copy_input(json!([bone_key(999, 0.125)])),
        copy_input(json!([bone_key(0, 0.5)])),
        json!({"expected_revision":6,"animation":0,"keys":[bone_key(0,0.125)]}),
        json!({"expected_revision":7,"animation":99,"keys":[bone_key(0,0.125)]}),
    ] {
        let result =
            server.copy_animation_key_group(Parameters(serde_json::from_value(invalid).unwrap()));
        assert_eq!(result.is_error, Some(true));
        assert_eq!(server.get_document().structured_content.unwrap(), before);
    }
    let state = server.state.lock().unwrap();
    let session = &state.project().unwrap().session;
    assert_eq!(state.revision, 7);
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pending);
    assert_eq!(session.slider_source_preferences(), &preferences);
    assert_eq!(session.slider_source_focus(), focus);
    assert_eq!(session.last_edit_effects(), &effects);
    assert!(session.can_redo());
}
