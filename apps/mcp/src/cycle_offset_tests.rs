use super::*;

fn input() -> Value {
    json!({"expected_revision":7,"animation":0,
        "keys":[{"type":"bone","bone":0,"property":"x","time":0.0},
            {"type":"bone","bone":0,"property":"x","time":0.5},
            {"type":"bone","bone":0,"property":"x","time":1.0}],
        "range":{"start":0.0,"end":1.0},"delta":0.125})
}

#[test]
fn cycle_offset_mcp_strict_map_boundaries_tokens_and_schemas() {
    assert!(serde_json::from_value::<OffsetAnimationKeysInput>(input()).is_ok());
    for field in ["expected_revision", "animation", "keys", "range", "delta"] {
        let mut missing = input();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<OffsetAnimationKeysInput>(missing).is_err());
        let mut null = input();
        null[field] = Value::Null;
        assert!(serde_json::from_value::<OffsetAnimationKeysInput>(null).is_err());
    }
    for (field, value) in [
        ("range", json!([0, 1])),
        ("range", json!({"start":0})),
        ("range", json!({"start":0,"end":1,"extra":false})),
        ("keys", json!([["bone", 0, "x", 0]])),
        (
            "keys",
            json!([{"type":"bone","bone":0,"property":{"x":null},"time":0}]),
        ),
        (
            "keys",
            json!([{"type":"bone","bone":0,"property":"future","time":0}]),
        ),
        (
            "keys",
            json!([{"type":"bone","bone":0,"property":"x","time":0,"extra":1}]),
        ),
        (
            "keys",
            json!([{"type":"deform","attachment":["default","slot","mesh"],"time":0}]),
        ),
        ("animation", json!("0")),
        ("delta", json!(true)),
        ("extra", json!(0)),
    ] {
        let mut bad = input();
        bad[field] = value;
        assert!(serde_json::from_value::<OffsetAnimationKeysInput>(bad).is_err());
    }
    assert!(
        serde_json::from_value::<OffsetAnimationKeysInput>(json!([7, 0, [], {}, 0.125])).is_err()
    );
    for value in [
        json!([]),
        json!([7]),
        json!(null),
        json!({}),
        json!({"expected_revision":null}),
        json!({"expected_revision":7,"animation":0}),
    ] {
        assert!(serde_json::from_value::<ResetCycleOffsetMemoryInput>(value).is_err());
    }
    let schema = schema::offset_animation_keys_schema();
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["range"]["additionalProperties"], false);
    assert_eq!(schema["properties"]["keys"]["uniqueItems"], true);
    assert_eq!(schema["required"].as_array().unwrap().len(), 5);
    assert_eq!(
        schema::reset_cycle_offset_memory_schema()["required"],
        json!(["expected_revision"])
    );
}

#[test]
fn cycle_offset_mcp_duplicate_raw_fields_reject_before_sdk_materialization() {
    let raw = input().to_string();
    for duplicate in [
        raw.replacen("\"animation\":0", "\"animation\":0,\"animation\":1", 1),
        raw.replacen("\"end\":1.0", "\"end\":1.0,\"end\":2.0", 1),
        raw.replacen(
            "\"property\":\"x\"",
            "\"property\":\"x\",\"property\":\"y\"",
            1,
        ),
    ] {
        assert_ne!(duplicate, raw);
        assert!(serde_json::from_str::<OffsetAnimationKeysInput>(&duplicate).is_err());
    }
    assert!(
        serde_json::from_str::<ResetCycleOffsetMemoryInput>(
            r#"{"expected_revision":7,"expected_revision":8}"#
        )
        .is_err()
    );
}

fn server() -> AnimatorServer {
    let root = std::env::temp_dir();
    let server = AnimatorServer::new(&root).unwrap();
    let mut document =
        peachy_document::Document::new("Original Offset protocol fixture".into(), 60).unwrap();
    document.animations.push(peachy_document::Animation::new(
        "closed",
        vec![peachy_document::Track {
            bone: 0,
            property: peachy_document::Property::X,
            keys: [(0., 0.), (0.5, 10.), (1., 0.)]
                .into_iter()
                .map(|(time, value)| peachy_document::Keyframe {
                    time,
                    value,
                    curve: peachy_document::Curve::Linear,
                })
                .collect(),
        }],
    ));
    document
        .animations
        .push(peachy_document::Animation::new("unrelated", vec![]));
    let mut session = Session::new(document).unwrap();
    let mut transform = session.document().bones[0].setup;
    transform.y = 17.;
    session
        .edit_pose(
            0,
            EditTarget::Animate {
                animation: 0,
                time: 0.5,
            },
            transform,
            &[peachy_document::Property::Y],
            false,
        )
        .unwrap();
    let keys = input()["keys"]
        .as_array()
        .unwrap()
        .iter()
        .map(|key| {
            serde_json::from_value::<GroupAnimationKeyInput>(key.clone())
                .unwrap()
                .0
        })
        .collect::<Vec<_>>();
    session
        .offset_animation_keys(
            0,
            &keys,
            peachy_engine::CycleOffsetRange { start: 0., end: 1. },
            0.125,
        )
        .unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "redo marker".into(),
        })
        .unwrap();
    session.undo();
    let mut state = server.state.lock().unwrap();
    state.revision = 7;
    state.project = Some(WorkingProject {
        session,
        source: None,
        asset_root: root,
    });
    drop(state);
    server
}

fn snapshot(server: &AnimatorServer) -> (Value, peachy_engine::CycleOffsetMemory) {
    let state = server.state.lock().unwrap();
    let session = &state.project().unwrap().session;
    (
        json!({"state":state.summary(),"document":session.document(),"skin":session.skin()}),
        session.cycle_offset_memory().clone(),
    )
}

#[test]
fn cycle_offset_mcp_single_seed_offsets_whole_track_without_clearing_memory() {
    let server = server();
    let before = snapshot(&server);
    let mut request = input();
    request["keys"] = json!([{"type":"bone","bone":0,"property":"x","time":0.625}]);
    let result = server.offset_animation_keys(Parameters(serde_json::from_value(request).unwrap()));
    assert_ne!(result.is_error, Some(true), "{result:?}");
    let receipt = result.structured_content.unwrap();
    assert_eq!(receipt["phases"][0]["phase"], 0.25);
    assert!(receipt["keys"].as_array().unwrap().len() > 1);
    assert_eq!(receipt["boundaryKeys"].as_array().unwrap().len(), 2);
    assert_eq!(receipt["mappings"].as_array().unwrap().len(), 1);
    assert_eq!(
        receipt["mappings"][0]["source"],
        json!({"type":"bone","bone":0,"property":"x","time":0.625})
    );
    assert_eq!(
        receipt["mappings"][0]["target"],
        json!({"type":"bone","bone":0,"property":"x","time":0.75})
    );
    let after = snapshot(&server);
    assert_ne!(
        after.0["document"]["animations"][0],
        before.0["document"]["animations"][0]
    );
    assert_eq!(
        after.0["document"]["animations"][1],
        before.0["document"]["animations"][1]
    );
    assert_eq!(
        after.0["state"]["pendingPose"],
        before.0["state"]["pendingPose"]
    );
    assert_eq!(after.0["state"]["hasCycleOffsetMemory"], true);
}

#[test]
fn cycle_offset_mcp_mapping_preserves_request_order_and_endpoint_collisions() {
    let server = server();
    // Restore the original closed track and empty memory, retaining the
    // unrelated pending pose. The earlier helper offset is no longer active.
    server
        .state
        .lock()
        .unwrap()
        .project_mut()
        .unwrap()
        .session
        .undo();
    let mut request = input();
    let keys = json!([
        {"type":"bone","bone":0,"property":"x","time":1.0},
        {"type":"bone","bone":0,"property":"x","time":0.5},
        {"type":"bone","bone":0,"property":"x","time":0.0}]);
    request["keys"] = keys.clone();
    let result = server.offset_animation_keys(Parameters(serde_json::from_value(request).unwrap()));
    assert_ne!(result.is_error, Some(true), "{result:?}");
    let receipt = result.structured_content.unwrap();
    let mappings = receipt["mappings"].as_array().unwrap();
    assert_eq!(mappings.len(), 3);
    for (mapping, source) in mappings.iter().zip(keys.as_array().unwrap()) {
        assert_eq!(&mapping["source"], source);
        assert!(
            receipt["keys"]
                .as_array()
                .unwrap()
                .contains(&mapping["target"])
        );
    }
    assert_eq!(
        mappings[0]["target"], mappings[2]["target"],
        "Original end aliases original start"
    );
    assert_eq!(mappings[1]["target"]["time"], 0.625);
    assert_eq!(mappings[0]["target"]["time"], 0.125);
    assert_ne!(mappings[0]["source"], mappings[2]["source"]);
}

#[test]
fn cycle_offset_mcp_only_fake_keys_map_to_null_without_authoring_or_memory_changes() {
    let server = server();
    let before = snapshot(&server);
    let sources = json!([
        {"type":"bone","bone":0,"property":"x","time":1.0},
        {"type":"bone","bone":0,"property":"x","time":0.0}]);
    let mut request = input();
    request["keys"] = sources.clone();
    let result = server.offset_animation_keys(Parameters(serde_json::from_value(request).unwrap()));
    assert_ne!(result.is_error, Some(true), "{result:?}");
    let receipt = result.structured_content.unwrap();
    assert_eq!(receipt["keys"], json!([]));
    assert_eq!(receipt["boundaryKeys"], json!([]));
    assert_eq!(receipt["phases"], json!([]));
    let mappings = receipt["mappings"].as_array().unwrap();
    assert_eq!(mappings.len(), 2);
    for (mapping, source) in mappings.iter().zip(sources.as_array().unwrap()) {
        assert_eq!(&mapping["source"], source);
        assert!(mapping["target"].is_null());
    }
    let mut after = snapshot(&server);
    assert_eq!(
        after.0["state"]["revision"].as_u64().unwrap(),
        before.0["state"]["revision"].as_u64().unwrap() + 1
    );
    after.0["state"]["revision"] = before.0["state"]["revision"].clone();
    assert_eq!(
        after, before,
        "Only-fake input changed Document/pending/memory/history/dirty/skin"
    );
}

#[test]
fn cycle_offset_mcp_failure_and_explicit_reset_preserve_pending_history_and_revision() {
    let server = server();
    let before = snapshot(&server);
    assert_eq!(before.0["state"]["hasCycleOffsetMemory"], true);
    assert_eq!(before.0["state"]["hasPendingEdits"], true);
    assert_eq!(before.0["state"]["canRedo"], true);
    for (field, value) in [
        ("expected_revision", json!(0)),
        ("keys", json!([])),
        ("animation", json!(999)),
        ("range", json!({"start":0,"end":0})),
    ] {
        let mut bad = input();
        bad[field] = value;
        let result = server.offset_animation_keys(Parameters(serde_json::from_value(bad).unwrap()));
        assert_eq!(result.is_error, Some(true), "{result:?}");
        assert_eq!(snapshot(&server), before);
    }
    let mut nonfinite: OffsetAnimationKeysInput = serde_json::from_value(input()).unwrap();
    nonfinite.delta = f32::INFINITY;
    assert_eq!(
        server.offset_animation_keys(Parameters(nonfinite)).is_error,
        Some(true)
    );
    assert_eq!(snapshot(&server), before);
    assert_eq!(
        server
            .reset_cycle_offset_memory(Parameters(ResetCycleOffsetMemoryInput {
                expected_revision: 0
            }))
            .is_error,
        Some(true)
    );
    assert_eq!(snapshot(&server), before);
    server
        .state
        .lock()
        .unwrap()
        .project_mut()
        .unwrap()
        .session
        .begin_transaction();
    assert_eq!(
        server
            .reset_cycle_offset_memory(Parameters(ResetCycleOffsetMemoryInput {
                expected_revision: 7
            }))
            .is_error,
        Some(true)
    );
    assert_eq!(snapshot(&server), before);
    server
        .state
        .lock()
        .unwrap()
        .project_mut()
        .unwrap()
        .session
        .cancel_transaction();
    let result = server.reset_cycle_offset_memory(Parameters(ResetCycleOffsetMemoryInput {
        expected_revision: 7,
    }));
    assert_ne!(result.is_error, Some(true), "{result:?}");
    let mut after = snapshot(&server);
    assert!(after.1.is_empty());
    after.0["state"]["revision"] = before.0["state"]["revision"].clone();
    after.0["state"]["hasCycleOffsetMemory"] = json!(true);
    assert_eq!(
        after.0, before.0,
        "Reset changed authoring state/history/dirty/skin"
    );
}
