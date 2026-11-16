use super::*;

fn input() -> Value {
    json!({"expected_revision":4,"target":{"mode":"animate","animation":0,"time":0.5},
        "skin":"default","bones":[1,2],"primary":1,
        "operation":{"type":"translate","axes":"local","delta":{"x":10.0,"y":-2.0}},
        "auto_key":false})
}

#[test]
fn transform_bones_strict_objects_tokens_and_discovery_schema() {
    assert!(serde_json::from_value::<TransformBonesInput>(input()).is_ok());
    for field in [
        "expected_revision",
        "target",
        "skin",
        "bones",
        "primary",
        "operation",
        "auto_key",
    ] {
        let mut missing = input();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<TransformBonesInput>(missing).is_err(),
            "Missing {field}"
        );
        let mut null = input();
        null[field] = Value::Null;
        assert!(
            serde_json::from_value::<TransformBonesInput>(null).is_err(),
            "Null {field}"
        );
    }
    for (field, value) in [
        ("target", json!(["animate", 0, 0.5])),
        ("target", json!({"mode":"setup","time":0})),
        ("bones", json!([1, "2"])),
        ("primary", json!(-1)),
        ("operation", json!(["translate","local",{"x":1,"y":2}])),
        (
            "operation",
            json!({"type":"rotate","axes":"local","delta":{"x":1,"y":2}}),
        ),
        (
            "operation",
            json!({"type":{"translate":null},"axes":"local","delta":{"x":1,"y":2}}),
        ),
        (
            "operation",
            json!({"type":"translate","axes":{"local":null},"delta":{"x":1,"y":2}}),
        ),
        (
            "operation",
            json!({"type":"translate","axes":"future","delta":{"x":1,"y":2}}),
        ),
        (
            "operation",
            json!({"type":"translate","axes":"local","delta":[1,2]}),
        ),
        (
            "operation",
            json!({"type":"translate","axes":"local","delta":{"x":1}}),
        ),
        (
            "operation",
            json!({"type":"translate","axes":"local","delta":{"x":null,"y":2}}),
        ),
        (
            "operation",
            json!({"type":"translate","axes":"local","delta":{"x":1,"y":2,"z":3}}),
        ),
        (
            "operation",
            json!({"type":"translate","axes":"local","delta":{"x":1,"y":2},"angle":3}),
        ),
        ("future", json!(true)),
    ] {
        let mut value_input = input();
        value_input[field] = value;
        assert!(
            serde_json::from_value::<TransformBonesInput>(value_input).is_err(),
            "Accepted {field}"
        );
    }
    assert!(
        serde_json::from_value::<TransformBonesInput>(json!([
            4,
            {},
            "default",
            [1, 2],
            1,
            {},
            false
        ]))
        .is_err()
    );
    let schema = schema::transform_bones_tool_schema();
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["required"].as_array().unwrap().len(), 7);
    assert_eq!(schema["properties"]["bones"]["uniqueItems"], true);
    assert_eq!(
        schema["properties"]["operation"]["properties"]["type"]["const"],
        "translate"
    );
    assert_eq!(
        schema["properties"]["operation"]["properties"]["delta"]["additionalProperties"],
        false
    );
}

#[test]
fn transform_bones_duplicate_wire_fields_reject_before_sdk_materialization() {
    let raw = input().to_string();
    for duplicate in [
        raw.replacen("\"primary\":1", "\"primary\":1,\"primary\":2", 1),
        raw.replacen(
            "\"axes\":\"local\"",
            "\"axes\":\"local\",\"axes\":\"world\"",
            1,
        ),
        raw.replacen("\"x\":10.0", "\"x\":10.0,\"x\":3.0", 1),
    ] {
        assert_ne!(duplicate, raw);
        assert!(serde_json::from_str::<TransformBonesInput>(&duplicate).is_err());
    }
}

fn fixture() -> AnimatorServer {
    let root = std::env::temp_dir();
    let server = AnimatorServer::new(&root).unwrap();
    let mut document =
        peachy_document::Document::new("Original semantic translation".into(), 60).unwrap();
    document
        .skins
        .insert("alt".into(), document.skins["default"].clone());
    for (name, rotation) in [("first", 0.), ("second", 90.)] {
        document.bones.push(peachy_document::Bone {
            name: name.into(),
            parent: Some(0),
            length: 30.,
            inherit: peachy_document::Inherit::Normal,
            setup: peachy_document::Transform {
                rotation,
                ..Default::default()
            },
        });
    }
    document
        .animations
        .push(peachy_document::Animation::new("pose", vec![]));
    let mut session = Session::new(document).unwrap();
    session.set_skin("alt");
    let mut unrelated = session.document().bones[0].setup;
    unrelated.x = 13.5;
    session
        .edit_poses(
            EditTarget::Animate {
                animation: 0,
                time: 0.5,
            },
            &[BonePoseEdit {
                bone: 0,
                transform: unrelated,
                properties: vec![peachy_document::Property::X],
            }],
            false,
        )
        .unwrap();
    session
        .dispatch(Command::RenameBone {
            bone: 0,
            name: "redo marker".into(),
        })
        .unwrap();
    session.undo();
    let mut state = server.state.lock().unwrap();
    state.revision = 4;
    state.project = Some(WorkingProject {
        session,
        source: None,
        asset_root: root,
    });
    drop(state);
    server
}

fn snapshot(server: &AnimatorServer) -> Value {
    let state = server.state.lock().unwrap();
    let session = &state.project().unwrap().session;
    json!({"summary":state.summary(),"document":session.document(),"skin":session.skin()})
}

#[test]
fn transform_bones_explicit_skin_and_failed_candidates_preserve_session_context() {
    let server = fixture();
    let before = snapshot(&server);
    for (field, value) in [
        ("bones", json!([])),
        ("bones", json!([1, 1])),
        ("bones", json!([1, 999])),
        ("primary", json!(0)),
        ("skin", json!("missing")),
        ("expected_revision", json!(0)),
    ] {
        let mut candidate = input();
        candidate[field] = value;
        let result = server.transform_bones(Parameters(serde_json::from_value(candidate).unwrap()));
        assert_eq!(result.is_error, Some(true), "{result:?}");
        assert_eq!(snapshot(&server), before);
    }
    let mut nonfinite: TransformBonesInput = serde_json::from_value(input()).unwrap();
    nonfinite.operation.delta.y = f32::INFINITY;
    assert_eq!(
        server.transform_bones(Parameters(nonfinite)).is_error,
        Some(true)
    );
    assert_eq!(snapshot(&server), before);
    let result = server.transform_bones(Parameters(serde_json::from_value(input()).unwrap()));
    assert_ne!(result.is_error, Some(true), "{result:?}");
    let state = server.state.lock().unwrap();
    let session = &state.project().unwrap().session;
    assert_eq!(session.skin(), "alt");
    assert_eq!(state.revision, 5);
    assert_eq!(
        serde_json::to_value(session.document()).unwrap(),
        before["document"]
    );
    assert!(!session.pose_edits().is_empty());
    assert!(!session.can_redo());
    let pose = session
        .authored_pose_with_skin(
            EditTarget::Animate {
                animation: 0,
                time: 0.5,
            },
            "default",
        )
        .unwrap();
    let evaluated = checked_pose_from_authored(session.document(), &pose).unwrap();
    assert!((evaluated.authored_locals()[1].x - 10.).abs() < 0.0001);
    assert!((evaluated.authored_locals()[2].y - 10.).abs() < 0.0001);
}
