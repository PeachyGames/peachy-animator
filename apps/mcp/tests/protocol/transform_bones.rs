use super::*;

fn write_fixture(root: &Path) -> Vec<u8> {
    let mut document = serde_json::to_value(
        peachy_document::Document::new("Original transform selection".into(), 60).unwrap(),
    )
    .unwrap();
    for (name, parent, x, y, rotation) in [
        ("parent", 0, 30., 20., 0.),
        ("first", 1, 5., 3., 0.),
        ("second", 1, -7., 11., 90.),
        ("child", 2, 9., -2., 15.),
    ] {
        document["bones"].as_array_mut().unwrap().push(json!({"name":name,"parent":parent,"length":20.,"inherit":"normal",
            "setup":{"x":x,"y":y,"rotation":rotation,"scale_x":1.,"scale_y":1.,"shear_x":0.,"shear_y":0.}}));
    }
    let mut document = peachy_io::decode_project(&document.to_string()).unwrap();
    document
        .animations
        .push(peachy_document::Animation::new("pose", vec![]));
    peachy_io::create_project(&root.join("translation.peachy"), &document).unwrap();
    fs::read(root.join("translation.peachy")).unwrap()
}

fn args(axes: &str, bones: Value, primary: usize, x: f32, y: f32) -> Value {
    json!({"target":{"mode":"animate","animation":0,"time":0.5},"skin":"default",
        "bones":bones,"primary":primary,"operation":{"type":"translate","axes":axes,"delta":{"x":x,"y":y}},"auto_key":false})
}

async fn mutate(client: &RunningService<RoleClient, ()>, name: &str, mut args: Value) -> Value {
    args["expected_revision"] = doc(client).await["state"]["revision"].clone();
    success(call(client, name, args).await)
}

async fn pose(client: &RunningService<RoleClient, ()>, animation: &str, skin: &str) -> Value {
    success(
        call(
            client,
            "evaluate_pose",
            json!({"animation":animation,"time":0.5,"skin":skin}),
        )
        .await,
    )
}

async fn rejected(client: &RunningService<RoleClient, ()>, args: Value) {
    let before = doc(client).await;
    let result = client
        .call_tool(
            CallToolRequestParams::new("transform_bones".to_owned())
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await;
    assert!(
        result.is_err()
            || result
                .as_ref()
                .is_ok_and(|result| result.is_error == Some(true)),
        "Accepted {args}"
    );
    assert_eq!(
        doc(client).await,
        before,
        "Rejected transform changed state"
    );
}

#[tokio::test]
async fn stdio_transform_bones_tool_axes_descendant_filter_and_atomic_pending_history() {
    let root = Sandbox::new();
    let source = write_fixture(&root.0);
    let client = start(&root.0).await;
    let tools = client.list_tools(Default::default()).await.unwrap().tools;
    assert_eq!(tools.len(), 25);
    let tool = tools
        .iter()
        .find(|tool| tool.name == "transform_bones")
        .unwrap();
    assert_eq!(tool.input_schema["additionalProperties"], false);
    assert_eq!(
        tool.input_schema["properties"]["bones"]["uniqueItems"],
        true
    );
    assert_eq!(
        tool.input_schema["properties"]["operation"]["properties"]["type"]["const"],
        "translate"
    );
    success(
        call(
            &client,
            "open_project",
            json!({"path":"translation.peachy","expected_revision":0}),
        )
        .await,
    );
    let baseline = doc(&client).await;
    let original = pose(&client, "pose", "default").await;
    mutate(
        &client,
        "transform_bones",
        args("local", json!([2, 3]), 2, 10., 0.),
    )
    .await;
    let pending = doc(&client).await;
    assert_eq!(pending["document"], baseline["document"]);
    assert_eq!(pending["state"]["dirty"], false);
    assert_eq!(pending["state"]["hasPendingEdits"], true);
    let moved = pose(&client, "pose", "default").await;
    assert!((moved["locals"][2]["x"].as_f64().unwrap() - 15.).abs() < 0.0001);
    assert!((moved["locals"][3]["x"].as_f64().unwrap() + 7.).abs() < 0.0001);
    assert!((moved["locals"][3]["y"].as_f64().unwrap() - 21.).abs() < 0.0001);
    mutate(&client, "history", json!({"action":"undo"})).await;
    assert_eq!(
        doc(&client).await["state"]["pendingPose"],
        baseline["state"]["pendingPose"]
    );
    assert_eq!(doc(&client).await["state"]["canRedo"], true);
    let mut valid = args("parent", json!([2, 3]), 2, 10., -4.);
    valid["expected_revision"] = doc(&client).await["state"]["revision"].clone();
    for (field, value) in [
        ("bones", json!([])),
        ("bones", json!([2, 2])),
        ("bones", json!([2, 999])),
        ("primary", json!(1)),
        ("skin", json!("missing")),
        ("expected_revision", json!(0)),
        (
            "target",
            json!({"mode":"animate","animation":999,"time":0.5}),
        ),
        ("target", json!({"mode":"animate","animation":0,"time":-1})),
        ("target", json!(["animate", 0, 0.5])),
        ("future", json!(true)),
        ("auto_key", Value::Null),
        (
            "operation",
            json!({"type":"translate","axes":"world","delta":{"x":1e100,"y":0}}),
        ),
        (
            "operation",
            json!({"type":"translate","axes":{"parent":null},"delta":{"x":1,"y":0}}),
        ),
        (
            "operation",
            json!({"type":"translate","axes":"parent","delta":[1,0]}),
        ),
    ] {
        let mut candidate = valid.clone();
        candidate[field] = value;
        rejected(&client, candidate).await;
    }
    mutate(&client, "history", json!({"action":"redo"})).await;
    assert_eq!(
        doc(&client).await["state"]["pendingPose"],
        pending["state"]["pendingPose"]
    );
    mutate(&client, "history", json!({"action":"undo"})).await;
    mutate(
        &client,
        "transform_bones",
        args("parent", json!([2, 3]), 2, 10., 0.),
    )
    .await;
    let parent = pose(&client, "pose", "default").await;
    assert!((parent["locals"][2]["x"].as_f64().unwrap() - 15.).abs() < 0.0001);
    assert!((parent["locals"][3]["x"].as_f64().unwrap() - 3.).abs() < 0.0001);
    mutate(&client, "history", json!({"action":"undo"})).await;
    mutate(
        &client,
        "transform_bones",
        args("world", json!([4, 2]), 4, 10., -4.),
    )
    .await;
    let world = pose(&client, "pose", "default").await;
    assert_eq!(
        world["locals"][4], original["locals"][4],
        "Selected child received a second authored edit"
    );
    for bone in [2, 4] {
        for (field, delta) in [("x", 10.), ("y", -4.)] {
            assert!(
                (world["world"][bone][field].as_f64().unwrap()
                    - original["world"][bone][field].as_f64().unwrap()
                    - delta)
                    .abs()
                    < 0.001
            );
        }
    }
    assert_eq!(fs::read(root.0.join("translation.peachy")).unwrap(), source);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn stdio_transform_bones_requested_skin_uses_constraint_frame_and_autokeys_once() {
    let root = Sandbox::new();
    let source = write_transform_mapping_fixture(&root.0);
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
    peachy_io::create_project(&root.0.join("skin.peachy"), &document).unwrap();
    let client = start(&root.0).await;
    success(
        call(
            &client,
            "open_project",
            json!({"path":"skin.peachy","expected_revision":0}),
        )
        .await,
    );
    let baseline = doc(&client).await;
    mutate(
        &client,
        "transform_bones",
        args("local", json!([2]), 2, 10., 0.),
    )
    .await;
    let default = pose(&client, "calibration", "default").await;
    mutate(&client, "history", json!({"action":"undo"})).await;
    let mut alt = args("local", json!([2]), 2, 10., 0.);
    alt["skin"] = json!("alt");
    alt["auto_key"] = json!(true);
    mutate(&client, "transform_bones", alt).await;
    let keyed = doc(&client).await;
    let alt = pose(&client, "calibration", "alt").await;
    assert_ne!(
        alt["locals"][2], default["locals"][2],
        "Requested skin did not affect the captured constraint frame"
    );
    assert_eq!(keyed["state"]["hasPendingEdits"], false);
    let tracks = keyed["document"]["animations"][0]["tracks"]
        .as_array()
        .unwrap();
    for property in ["x", "y"] {
        assert!(tracks.iter().any(|track| track["bone"] == 2
            && track["property"] == property
            && track["keys"].as_array().unwrap().len() == 1));
    }
    mutate(&client, "history", json!({"action":"undo"})).await;
    assert_eq!(doc(&client).await["document"], baseline["document"]);
    mutate(&client, "history", json!({"action":"redo"})).await;
    assert_eq!(doc(&client).await["document"], keyed["document"]);
    client.cancel().await.unwrap();
}
