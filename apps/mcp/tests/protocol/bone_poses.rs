use super::*;

fn fixture(root: &Path) -> Vec<u8> {
    let source = write_transform_mapping_fixture(root);
    let mut document = peachy_io::decode_project(std::str::from_utf8(&source).unwrap()).unwrap();
    document
        .skins
        .insert("alt".into(), document.skins["default"].clone());
    document.skin_scope.required_constraints.insert(0);
    document.skin_scope.skins.insert(
        "alt".into(),
        peachy_document::SkinMembers {
            bones: vec![],
            constraints: vec![0],
        },
    );
    document.validate().unwrap();
    peachy_io::create_project(&root.join("poses.peachy"), &document).unwrap();
    fs::read(root.join("poses.peachy")).unwrap()
}

async fn mutate(client: &RunningService<RoleClient, ()>, name: &str, mut args: Value) -> Value {
    args["expected_revision"] = doc(client).await["state"]["revision"].clone();
    success(call(client, name, args).await)
}

async fn rejected_unchanged(client: &RunningService<RoleClient, ()>, name: &str, args: Value) {
    let before = doc(client).await;
    let result = call(client, name, args).await;
    assert_eq!(result.is_error, Some(true), "{result:?}");
    assert_eq!(doc(client).await, before, "Rejected {name} changed state");
}

async fn pose(client: &RunningService<RoleClient, ()>, skin: &str) -> Value {
    success(
        call(
            client,
            "evaluate_pose",
            json!({"animation":"calibration","time":0.5,"skin":skin}),
        )
        .await,
    )
}

fn edits(pose: &Value) -> Value {
    let mut driver = pose["locals"][1].clone();
    driver["x"] = json!(73.25);
    let mut follower = pose["locals"][2].clone();
    follower["y"] = json!(-13.5);
    json!([{"bone":1,"transform":driver,"properties":["x"]},
        {"bone":2,"transform":follower,"properties":["y"]}])
}

#[tokio::test]
async fn stdio_bone_poses_pending_skin_reads_key_history_and_save_exclusion() {
    let root = Sandbox::new();
    let original = fixture(&root.0);
    let client = start(&root.0).await;
    success(
        call(
            &client,
            "open_project",
            json!({"path":"poses.peachy","expected_revision":0}),
        )
        .await,
    );
    let baseline = doc(&client).await;
    let initial = pose(&client, "default").await;
    let changes = edits(&initial);
    let target = json!({"mode":"animate","animation":0,"time":0.5});
    mutate(
        &client,
        "edit_bone_poses",
        json!({"target":target,"edits":changes,"auto_key":false}),
    )
    .await;
    let pending = doc(&client).await;
    assert_eq!(pending["document"], baseline["document"]);
    assert_eq!(pending["state"]["dirty"], false);
    assert_eq!(pending["state"]["hasPendingEdits"], true);
    assert_eq!(
        pending["state"]["pendingPose"]["values"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let default = pose(&client, "default").await;
    assert_eq!(default["locals"][1]["x"], 73.25);
    assert_eq!(default["locals"][2]["y"], -13.5);
    let alt = pose(&client, "alt").await;
    assert_eq!(alt["locals"], default["locals"]);
    assert_ne!(alt["world"], default["world"]);
    assert_eq!(pose(&client, "default").await, default);
    assert_eq!(
        doc(&client).await,
        pending,
        "Read-only skin sampling changed Session"
    );
    rejected_unchanged(
        &client,
        "evaluate_pose",
        json!({"animation":"calibration","time":0.5,"skin":"missing"}),
    )
    .await;

    let pending_revision = pending["state"]["revision"].clone();
    rejected_unchanged(
        &client,
        "open_project",
        json!({"path":"poses.peachy","expected_revision":pending_revision}),
    )
    .await;
    rejected_unchanged(&client,"create_project",json!({"name":"discard attempt","fps":30,"asset_root":".","expected_revision":pending_revision})).await;

    mutate(
        &client,
        "save_project",
        json!({"directory":"pending-saved"}),
    )
    .await;
    let saved = peachy_io::decode_project(
        &fs::read_to_string(root.0.join("pending-saved/project.peachy")).unwrap(),
    )
    .unwrap();
    assert_eq!(serde_json::to_value(saved).unwrap(), baseline["document"]);
    assert_eq!(doc(&client).await["state"]["hasPendingEdits"], true);
    let saved_pending = doc(&client).await;
    rejected_unchanged(&client,"open_project",json!({"path":"pending-saved/project.peachy","expected_revision":saved_pending["state"]["revision"]})).await;

    let keys = json!([{"bone":1,"properties":["x"]},{"bone":2,"properties":["y"]}]);
    mutate(
        &client,
        "key_bone_poses",
        json!({"animation":0,"time":0.5,"keys":keys}),
    )
    .await;
    let keyed = doc(&client).await;
    assert_eq!(keyed["state"]["hasPendingEdits"], false);
    for (bone, property) in [(1, "x"), (2, "y")] {
        let tracks = keyed["document"]["animations"][0]["tracks"]
            .as_array()
            .unwrap();
        assert!(tracks.iter().any(|t| {
            t["bone"] == bone
                && t["property"] == property
                && t["keys"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|k| k["time"] == 0.5)
        }));
    }
    // Exact per-Bone requests must not manufacture companion Y/X tracks.
    assert!(
        !keyed["document"]["animations"][0]["tracks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| (t["bone"] == 1 && t["property"] == "y")
                || (t["bone"] == 2 && t["property"] == "x"))
    );
    mutate(&client, "history", json!({"action":"undo"})).await;
    let undone = doc(&client).await;
    assert_eq!(undone["document"], baseline["document"]);
    assert_eq!(
        undone["state"]["pendingPose"],
        pending["state"]["pendingPose"]
    );
    mutate(&client, "history", json!({"action":"redo"})).await;
    assert_eq!(doc(&client).await["document"], keyed["document"]);
    mutate(&client, "history", json!({"action":"undo"})).await;

    let revision = doc(&client).await["state"]["revision"].clone();
    let valid =
        json!({"expected_revision":revision,"target":target,"edits":changes,"auto_key":true});
    let mut bad = valid.clone();
    bad["edits"][1]["bone"] = json!(999);
    rejected_unchanged(&client, "edit_bone_poses", bad).await;
    let mut bad = valid.clone();
    bad["edits"][1]["bone"] = json!(1);
    rejected_unchanged(&client, "edit_bone_poses", bad).await;
    let mut bad = valid.clone();
    bad["edits"][1]["properties"] = json!(["y", "y"]);
    rejected_unchanged(&client, "edit_bone_poses", bad).await;
    let mut bad = valid.clone();
    bad["edits"][1]["properties"] = json!([]);
    rejected_unchanged(&client, "edit_bone_poses", bad).await;
    let mut bad = valid.clone();
    bad["edits"] = json!([]);
    rejected_unchanged(&client, "edit_bone_poses", bad).await;
    let mut bad = valid.clone();
    bad["edits"][1]["transform"]["shear_x"] = json!(1e100);
    rejected_unchanged(&client, "edit_bone_poses", bad).await;
    let mut bad = valid.clone();
    bad["expected_revision"] = json!(0);
    rejected_unchanged(&client, "edit_bone_poses", bad).await;
    let mut bad = valid.clone();
    bad["edits"][1]["transform"] = json!([0, 0, 0, 1, 1, 0, 0]);
    rejected_unchanged(&client, "edit_bone_poses", bad).await;
    let mut bad = valid.clone();
    bad["edits"][1]["properties"] = json!([{"y":null}]);
    rejected_unchanged(&client, "edit_bone_poses", bad).await;
    let mut bad = valid.clone();
    bad["target"] = json!(null);
    rejected_unchanged(&client, "edit_bone_poses", bad).await;
    let mut bad = valid;
    bad["future"] = json!(true);
    rejected_unchanged(&client, "edit_bone_poses", bad).await;
    for keys in [
        json!([]),
        json!([{"bone":1,"properties":["x"]},{"bone":999,"properties":["y"]}]),
        json!([{"bone":1,"properties":["x"]},{"bone":1,"properties":["y"]}]),
    ] {
        rejected_unchanged(
            &client,
            "key_bone_poses",
            json!({"expected_revision":revision,
            "animation":0,"time":0.5,"keys":keys}),
        )
        .await;
    }
    rejected_unchanged(
        &client,
        "key_edited_pose",
        json!({"expected_revision":revision,"animation":999,"time":0.5}),
    )
    .await;
    rejected_unchanged(
        &client,
        "key_edited_pose",
        json!({"expected_revision":revision,"animation":0,"time":-1.0}),
    )
    .await;

    mutate(
        &client,
        "key_edited_pose",
        json!({"animation":0,"time":0.5}),
    )
    .await;
    assert_eq!(doc(&client).await["state"]["hasPendingEdits"], false);
    assert_eq!(pose(&client, "default").await["locals"][1]["x"], 73.25);
    mutate(&client, "history", json!({"action":"undo"})).await;
    assert_eq!(
        doc(&client).await["state"]["pendingPose"],
        pending["state"]["pendingPose"]
    );
    mutate(
        &client,
        "open_project",
        json!({"path":"pending-saved/project.peachy","discard_changes":true}),
    )
    .await;
    let reopened = doc(&client).await;
    assert_eq!(reopened["document"], baseline["document"]);
    assert_eq!(reopened["state"]["hasPendingEdits"], false);
    assert_eq!(reopened["state"]["canUndo"], false);
    assert_eq!(reopened["state"]["canRedo"], false);
    assert_eq!(fs::read(root.0.join("poses.peachy")).unwrap(), original);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn stdio_bone_poses_setup_full_transform_and_autokey_groups_are_atomic() {
    let root = Sandbox::new();
    fixture(&root.0);
    let client = start(&root.0).await;
    let tools = client.list_tools(Default::default()).await.unwrap().tools;
    assert_eq!(tools.len(), 25);
    for name in ["edit_bone_poses", "key_bone_poses", "key_edited_pose"] {
        let tool = tools.iter().find(|t| t.name == name).unwrap();
        assert_eq!(
            serde_json::to_value(&tool.input_schema).unwrap()["additionalProperties"],
            false
        );
    }
    success(
        call(
            &client,
            "open_project",
            json!({"path":"poses.peachy","expected_revision":0}),
        )
        .await,
    );
    let baseline = doc(&client).await;
    let mut changes = edits(&pose(&client, "default").await);
    changes[0]["transform"]["rotation"] = json!(12.0);
    changes[1]["transform"]["scale_x"] = json!(2.0);
    mutate(
        &client,
        "edit_bone_poses",
        json!({"target":{"mode":"setup"},"edits":changes,"auto_key":false}),
    )
    .await;
    let setup = doc(&client).await;
    assert_eq!(setup["document"]["bones"][1]["setup"]["x"], 73.25);
    assert_eq!(setup["document"]["bones"][2]["setup"]["y"], -13.5);
    assert_eq!(setup["document"]["bones"][1]["setup"]["rotation"], 12.0);
    assert_eq!(setup["document"]["bones"][2]["setup"]["scale_x"], 2.0);
    assert_eq!(
        setup["document"]["animations"],
        baseline["document"]["animations"]
    );
    assert_eq!(setup["state"]["hasPendingEdits"], false);
    mutate(&client, "history", json!({"action":"undo"})).await;
    assert_eq!(doc(&client).await["document"], baseline["document"]);
    mutate(
        &client,
        "edit_bone_poses",
        json!({"target":{"mode":"animate","animation":0,"time":0.5},
        "edits":changes,"auto_key":true}),
    )
    .await;
    let keyed = doc(&client).await;
    assert_eq!(keyed["state"]["hasPendingEdits"], false);
    assert_eq!(keyed["document"]["bones"], baseline["document"]["bones"]);
    let tracks = keyed["document"]["animations"][0]["tracks"]
        .as_array()
        .unwrap();
    for bone in [1, 2] {
        for property in ["x", "y"] {
            assert!(tracks.iter().any(|track| {
                track["bone"] == bone
                    && track["property"] == property
                    && track["keys"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|key| key["time"] == 0.5)
            }));
        }
    }
    assert_eq!(pose(&client, "default").await["locals"][2]["y"], -13.5);
    assert_eq!(pose(&client, "default").await["locals"][1]["rotation"], 0.0);
    assert_eq!(pose(&client, "default").await["locals"][2]["scale_x"], 1.0);
    mutate(&client, "history", json!({"action":"undo"})).await;
    assert_eq!(doc(&client).await["document"], baseline["document"]);
    mutate(&client, "history", json!({"action":"redo"})).await;
    assert_eq!(doc(&client).await["document"], keyed["document"]);
    client.cancel().await.unwrap();
}
