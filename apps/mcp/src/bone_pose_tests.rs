use super::*;

fn transform() -> Value {
    json!({"x":1.0,"y":2.0,"rotation":3.0,"scale_x":1.0,"scale_y":1.0,
        "shear_x":0.0,"shear_y":0.0})
}

fn edit() -> Value {
    json!({"expected_revision":1,"target":{"mode":"animate","animation":0,"time":0.5},
        "edits":[{"bone":1,"transform":transform(),"properties":["x","rotation"]}],"auto_key":false})
}

#[test]
fn bone_pose_tools_strict_objects_properties_and_schema_match() {
    assert!(serde_json::from_value::<EditBonePosesInput>(edit()).is_ok());
    assert!(
        serde_json::from_value::<KeyBonePosesInput>(json!({"expected_revision":1,
        "animation":0,"time":0.5,"keys":[{"bone":1,"properties":["shearY"]}]}))
        .is_ok()
    );
    for field in ["expected_revision", "target", "edits", "auto_key"] {
        let mut missing = edit();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<EditBonePosesInput>(missing).is_err());
        let mut null = edit();
        null[field] = Value::Null;
        assert!(serde_json::from_value::<EditBonePosesInput>(null).is_err());
    }
    for (field, value) in [
        ("target", json!(["animate", 0, 0.5])),
        ("target", json!({"mode":"setup","animation":0})),
        (
            "target",
            json!({"mode":"animate","animation":0,"time":0.5,"extra":true}),
        ),
        ("edits", json!([[1, transform(), ["x"]]])),
        (
            "edits",
            json!([{"bone":1,"transform":[1,2,3,1,1,0,0],"properties":["x"]}]),
        ),
        (
            "edits",
            json!([{"bone":1,"transform":transform(),"properties":[{"x":null}]}]),
        ),
        (
            "edits",
            json!([{"bone":1,"transform":transform(),"properties":["future"]}]),
        ),
        (
            "edits",
            json!([{"bone":1,"transform":transform(),"properties":["x"],"extra":1}]),
        ),
        ("extra", json!(true)),
    ] {
        let mut candidate = edit();
        candidate[field] = value;
        assert!(
            serde_json::from_value::<EditBonePosesInput>(candidate).is_err(),
            "Accepted {field}"
        );
    }
    for field in [
        "x", "y", "rotation", "scale_x", "scale_y", "shear_x", "shear_y",
    ] {
        let mut candidate = edit();
        candidate["edits"][0]["transform"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(serde_json::from_value::<EditBonePosesInput>(candidate).is_err());
    }
    for value in [json!(null), json!([]), json!([1,{"mode":"setup"},[],false])] {
        assert!(serde_json::from_value::<EditBonePosesInput>(value).is_err());
    }
    assert!(serde_json::from_value::<KeyBonePosesInput>(json!([1, 0, 0.5, []])).is_err());
    assert!(
        serde_json::from_value::<KeyBonePosesInput>(json!({"expected_revision":1,
        "animation":0,"time":0.5,"keys":[[1,["x"]]]}))
        .is_err()
    );
    assert!(serde_json::from_value::<KeyEditedPoseInput>(json!([1, 0, 0.5])).is_err());
    assert!(
        serde_json::from_value::<KeyEditedPoseInput>(json!({"expected_revision":1,
        "animation":0,"time":0.5,"bone":1}))
        .is_err()
    );
    let mut candidate = edit();
    candidate["edits"][0]["transform"]["future"] = json!(0);
    assert!(serde_json::from_value::<EditBonePosesInput>(candidate).is_err());
    let schema = schema::bone_pose_tool_schema("edit");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["edits"]["items"]["properties"]["transform"]["required"]
            .as_array()
            .unwrap()
            .len(),
        7
    );
    assert_eq!(
        schema["properties"]["edits"]["items"]["properties"]["properties"]["uniqueItems"],
        true
    );
}

#[test]
fn bone_pose_raw_duplicate_fields_reject_before_sdk_materialization() {
    for raw in [
        r#"{"expected_revision":1,"expected_revision":1,"animation":0,"time":0.5}"#,
        r#"{"expected_revision":1,"animation":0,"animation":0,"time":0.5}"#,
    ] {
        assert!(serde_json::from_str::<KeyEditedPoseInput>(raw).is_err());
    }
    let raw = edit()
        .to_string()
        .replacen("\"bone\":1", "\"bone\":1,\"bone\":2", 1);
    assert!(serde_json::from_str::<EditBonePosesInput>(&raw).is_err());
}

#[test]
fn bone_pose_queries_keep_session_skin_pending_and_revision() {
    let root = std::env::temp_dir();
    let server = AnimatorServer::new(&root).unwrap();
    let mut document = peachy_document::Document::new("Original pending query".into(), 60).unwrap();
    document
        .skins
        .insert("alt".into(), document.skins["default"].clone());
    document
        .animations
        .push(peachy_document::Animation::new("pose", vec![]));
    let mut session = Session::new(document).unwrap();
    session.set_skin("alt");
    let mut transform = session.document().bones[0].setup;
    transform.x = 23.0;
    session
        .edit_poses(
            EditTarget::Animate {
                animation: 0,
                time: 0.5,
            },
            &[BonePoseEdit {
                bone: 0,
                transform,
                properties: vec![peachy_document::Property::X],
            }],
            false,
        )
        .unwrap();
    let before = session.pose_edits().clone();
    {
        let mut state = server.state.lock().unwrap();
        state.revision = 7;
        state.project = Some(WorkingProject {
            session,
            source: None,
            asset_root: root,
        });
    }
    let result = server.evaluate_pose(Parameters(EvaluateInput {
        animation: Some("pose".into()),
        time: 0.5,
        skin: "default".into(),
    }));
    assert_ne!(result.is_error, Some(true), "{result:?}");
    assert_eq!(result.structured_content.unwrap()["locals"][0]["x"], 23.0);
    let state = server.state.lock().unwrap();
    let session = &state.project().unwrap().session;
    assert_eq!(session.skin(), "alt");
    assert_eq!(session.pose_edits(), &before);
    assert!(session.can_undo());
    assert!(!session.can_redo());
    assert!(!session.is_dirty());
    assert_eq!(state.revision, 7);
}
