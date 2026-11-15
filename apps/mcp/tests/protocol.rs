//! Real stdio child-process client: lifecycle, schemas and editor/file workflows.
use rmcp::{
    RoleClient, ServiceExt,
    model::{CallToolRequestParams, CallToolResult},
    service::RunningService,
    transport::TokioChildProcess,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Sandbox(PathBuf);
impl Sandbox {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "peachy-mcp-test-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn fixture(&self) {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
        copy_tree(&source, &self.0.join("source"));
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &to.join(entry.file_name()));
        } else {
            fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

async fn start(root: &Path) -> RunningService<RoleClient, ()> {
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_peachy-mcp"));
    command.arg("--root").arg(root);
    ().serve(TokioChildProcess::new(command).unwrap())
        .await
        .unwrap()
}
async fn call(client: &RunningService<RoleClient, ()>, name: &str, args: Value) -> CallToolResult {
    client
        .call_tool(
            CallToolRequestParams::new(name.to_owned())
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap()
}
fn success(result: CallToolResult) -> Value {
    assert_ne!(result.is_error, Some(true), "{result:?}");
    let value = result.structured_content.unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&result.content[0].as_text().unwrap().text).unwrap(),
        value
    );
    value
}
fn error(result: CallToolResult) -> Value {
    assert_eq!(result.is_error, Some(true), "{result:?}");
    result.structured_content.unwrap()
}
async fn doc(client: &RunningService<RoleClient, ()>) -> Value {
    success(call(client, "get_document", json!({})).await)
}
async fn open(client: &RunningService<RoleClient, ()>) -> Value {
    success(
        call(
            client,
            "open_project",
            json!({"path":"source/peach-bot.peachy","expected_revision":0}),
        )
        .await,
    )
}

fn write_transform_mapping_fixture(root: &Path) -> Vec<u8> {
    let mut fixture = serde_json::to_value(
        peachy_document::Document::new("Transform mapping ranges".into(), 60).unwrap(),
    )
    .unwrap();
    for (name, parent, x, rotation) in [
        ("driver", 0, 20.0, 0.0),
        ("follower", 0, 0.0, 30.0),
        ("tip", 2, 120.0, 0.0),
    ] {
        fixture["bones"].as_array_mut().unwrap().push(json!({
            "name":name,"parent":parent,"length":120.0,"inherit":"normal",
            "setup":{"x":x,"y":0.0,"rotation":rotation,"scale_x":1.0,
                "scale_y":1.0,"shear_x":0.0,"shear_y":0.0}
        }));
    }
    fixture["constraints"] = json!([{
        "type":"transform","name":"driver X to follower Rotate","source":1,"bones":[2],
        "offsets":{"rotate":11.0,"x":7.0,"y":-3.0,"scale_x":0.0,"scale_y":0.0,"shear_y":0.0},
        "mix":{"rotate":0.5,"x":0.0,"y":0.0,"scale_x":0.0,"scale_y":0.0,"shear_y":0.0},
        "mappings":[
            {"from":"x","offset":10.0,"to":[{"property":"rotate","offset":-30.0,"max":150.0,"scale":2.0}]},
            {"from":"y","offset":7.0,"to":[{"property":"y","offset":9.0,"max":11.0,"scale":0.5}]}
        ],
        "local_source":false,"local_target":false,"additive":false,"clamp":false
    }]);
    let mut fixture: peachy_document::Document = serde_json::from_value(fixture).unwrap();
    fixture.animations.push(peachy_document::Animation::new(
        "calibration",
        vec![peachy_document::Track {
            bone: 1,
            property: peachy_document::Property::X,
            keys: vec![
                peachy_document::Keyframe {
                    time: 0.0,
                    value: 0.0,
                    curve: peachy_document::Curve::Linear,
                },
                peachy_document::Keyframe {
                    time: 1.0,
                    value: 80.0,
                    curve: peachy_document::Curve::Linear,
                },
            ],
        }],
    ));
    fixture.validate().unwrap();
    peachy_io::create_project(&root.join("mapping.peachy"), &fixture).unwrap();
    fs::read(root.join("mapping.peachy")).unwrap()
}

fn transform_range_edit(field: &str, value: f64) -> Value {
    json!({"command":"editTransformMappingRange","constraint":0,
        "from":"x","to":"rotate","field":field,"value":value})
}

async fn apply_transform_range_commands(
    client: &RunningService<RoleClient, ()>,
    commands: Vec<Value>,
) -> Value {
    let revision = doc(client).await["state"]["revision"].clone();
    success(
        call(
            client,
            "apply_commands",
            json!({"expected_revision":revision,"commands":commands}),
        )
        .await,
    )
}

async fn transform_range_query(client: &RunningService<RoleClient, ()>) -> Value {
    let revision = doc(client).await["state"]["revision"].clone();
    success(
        call(
            client,
            "get_transform_mapping",
            json!({"expected_revision":revision,"constraint":0,"from":"x","to":"rotate"}),
        )
        .await,
    )
}

async fn transform_range_history(client: &RunningService<RoleClient, ()>, action: &str) {
    let revision = doc(client).await["state"]["revision"].clone();
    success(
        call(
            client,
            "history",
            json!({"expected_revision":revision,"action":action}),
        )
        .await,
    );
}

#[tokio::test]
async fn stdio_transform_mapping_ranges_author_endpoints_history_and_persistence() {
    let root = Sandbox::new();
    let original = write_transform_mapping_fixture(&root.0);
    let client = start(&root.0).await;
    let tools = client.list_tools(Default::default()).await.unwrap().tools;
    let query_tool = tools
        .iter()
        .find(|tool| tool.name == "get_transform_mapping")
        .unwrap();
    assert_eq!(query_tool.input_schema["additionalProperties"], false);
    let required = query_tool.input_schema["required"].as_array().unwrap();
    assert_eq!(required.len(), 4);
    for field in ["expected_revision", "constraint", "from", "to"] {
        assert!(required.contains(&json!(field)));
    }
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":0,"path":"mapping.peachy"}),
        )
        .await,
    );
    let before = doc(&client).await;
    let query = transform_range_query(&client).await;
    assert_eq!(query["revision"], 1);
    assert_eq!(query["constraint"], 0);
    assert_eq!(query["from"], "x");
    assert_eq!(query["to"], "rotate");
    assert_eq!(
        query["range"],
        json!({"sourceMin":10.0,"sourceMax":100.0,"targetMin":-30.0,"targetMax":150.0})
    );
    assert_eq!(doc(&client).await, before);

    // One user batch crosses both reversed and equal endpoint states.
    apply_transform_range_commands(
        &client,
        vec![
            transform_range_edit("sourceMax", -30.0),
            transform_range_edit("targetMax", -30.0),
            transform_range_edit("sourceMin", -30.0),
            transform_range_edit("targetMin", 70.0),
        ],
    )
    .await;
    assert_eq!(
        transform_range_query(&client).await["range"],
        json!({"sourceMin":-30.0,"sourceMax":70.0,"targetMin":70.0,"targetMax":170.0})
    );
    let edited = doc(&client).await;
    let mut expected = before["document"].clone();
    expected["constraints"][0]["mappings"][0] = json!({"from":"x","offset":-30.0,
        "to":[{"property":"rotate","offset":70.0,"max":170.0,"scale":1.0}]});
    assert_eq!(edited["document"], expected);
    transform_range_history(&client, "undo").await;
    let undone = doc(&client).await;
    assert_eq!(undone["document"], before["document"]);
    assert_eq!(undone["state"]["canUndo"], false);
    assert_eq!(undone["state"]["canRedo"], true);
    assert_eq!(
        transform_range_query(&client).await["range"],
        query["range"]
    );
    assert_eq!(doc(&client).await, undone);

    // Exact readback is a raw-data/history no-op, even with redo waiting.
    apply_transform_range_commands(&client, vec![transform_range_edit("sourceMax", 100.0)]).await;
    let no_op = doc(&client).await;
    assert_eq!(no_op["document"], undone["document"]);
    assert_eq!(no_op["state"]["canUndo"], false);
    assert_eq!(no_op["state"]["canRedo"], true);
    assert_eq!(no_op["state"]["dirty"], false);
    transform_range_history(&client, "redo").await;
    assert_eq!(doc(&client).await["document"], edited["document"]);
    let revision = doc(&client).await["state"]["revision"].clone();
    let saved = success(
        call(
            &client,
            "save_project",
            json!({"expected_revision":revision,"directory":"saved"}),
        )
        .await,
    );
    assert_eq!(saved["state"]["dirty"], false);
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(root.0.join("saved/project.peachy")).unwrap())
            .unwrap(),
        edited["document"]
    );
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":saved["state"]["revision"],"path":"saved/project.peachy"}),
        )
        .await,
    );
    let reopened = doc(&client).await;
    assert_eq!(reopened["document"], edited["document"]);
    assert_eq!(reopened["state"]["canUndo"], false);
    assert_eq!(reopened["state"]["canRedo"], false);
    assert_eq!(fs::read(root.0.join("mapping.peachy")).unwrap(), original);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn stdio_transform_mapping_range_failures_and_raw_zero_preserve_session() {
    let root = Sandbox::new();
    let original = write_transform_mapping_fixture(&root.0);
    let client = start(&root.0).await;
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":0,"path":"mapping.peachy"}),
        )
        .await,
    );
    let mut raw = doc(&client).await["document"]["constraints"][0].clone();
    raw.as_object_mut().unwrap().remove("type");
    apply_transform_range_commands(&client, vec![transform_range_edit("targetMax", 160.0)]).await;
    let edited = doc(&client).await["document"].clone();
    transform_range_history(&client, "undo").await;
    let before = doc(&client).await;
    assert_eq!(before["state"]["canRedo"], true);
    let revision = before["state"]["revision"].as_u64().unwrap();
    let query = json!({"expected_revision":revision,"constraint":0,"from":"x","to":"rotate"});
    let mut invalid_queries = vec![];
    for (field, value) in [
        ("expected_revision", json!(revision - 1)),
        ("constraint", json!(99)),
        ("from", json!("y")),
        ("to", json!("x")),
        ("from", json!("rotation")),
        ("to", Value::Null),
        ("future", json!(true)),
    ] {
        let mut invalid = query.clone();
        invalid[field] = value;
        invalid_queries.push(invalid);
    }
    for args in invalid_queries {
        let result = client
            .call_tool(
                CallToolRequestParams::new("get_transform_mapping")
                    .with_arguments(args.as_object().unwrap().clone()),
            )
            .await;
        assert!(
            result.is_err()
                || result
                    .as_ref()
                    .is_ok_and(|value| value.is_error == Some(true))
        );
        assert_eq!(doc(&client).await, before);
    }
    for (field, value) in [
        ("constraint", json!(99)),
        ("from", json!("y")),
        ("to", json!("x")),
        ("field", json!("source_min")),
        ("value", Value::Null),
        ("value", json!(1e308_f64)),
        ("future", json!(true)),
    ] {
        let mut invalid = transform_range_edit("targetMax", 170.0);
        invalid[field] = value;
        let args = json!({"expected_revision":revision,"commands":[
            {"command":"renameBone","bone":0,"name":"must-rollback"},invalid]});
        let result = client
            .call_tool(
                CallToolRequestParams::new("apply_commands")
                    .with_arguments(args.as_object().unwrap().clone()),
            )
            .await;
        assert!(
            result.is_err()
                || result
                    .as_ref()
                    .is_ok_and(|value| value.is_error == Some(true))
        );
        assert_eq!(doc(&client).await, before);
    }
    error(
        call(
            &client,
            "apply_commands",
            json!({"expected_revision":revision-1,
        "commands":[transform_range_edit("targetMax",170.0)]}),
        )
        .await,
    );
    assert_eq!(doc(&client).await, before);
    transform_range_history(&client, "redo").await;
    assert_eq!(doc(&client).await["document"], edited);

    // The complete raw setter still accepts data without manufacturing endpoints.
    let mut ambiguous = raw.clone();
    ambiguous["mappings"]
        .as_array_mut()
        .unwrap()
        .push(json!({"from":"rotate","offset":0.0,
        "to":[{"property":"rotate","offset":0.0,"max":1.0,"scale":1.0}]}));
    apply_transform_range_commands(
        &client,
        vec![json!({"command":"setTransformConstraint",
        "constraint":0,"value":ambiguous})],
    )
    .await;
    let ambiguous_before = doc(&client).await;
    error(
        call(
            &client,
            "get_transform_mapping",
            json!({"expected_revision":ambiguous_before["state"]["revision"],
        "constraint":0,"from":"x","to":"rotate"}),
        )
        .await,
    );
    error(
        call(
            &client,
            "apply_commands",
            json!({"expected_revision":ambiguous_before["state"]["revision"],
        "commands":[{"command":"renameBone","bone":0,"name":"must-rollback"},
            transform_range_edit("targetMax",170.0)]}),
        )
        .await,
    );
    assert_eq!(doc(&client).await, ambiguous_before);

    raw["mappings"][0]["to"][0]["scale"] = json!(0.0);
    apply_transform_range_commands(
        &client,
        vec![json!({"command":"setTransformConstraint",
        "constraint":0,"value":raw})],
    )
    .await;
    let zero_before = doc(&client).await;
    let zero_query = transform_range_query(&client).await;
    assert_eq!(
        zero_query["range"],
        json!({"sourceMin":10.0,"sourceMax":null,"targetMin":-30.0,"targetMax":150.0})
    );
    assert_eq!(doc(&client).await, zero_before);
    error(
        call(
            &client,
            "apply_commands",
            json!({"expected_revision":zero_before["state"]["revision"],
        "commands":[{"command":"renameBone","bone":0,"name":"must-rollback"},
            transform_range_edit("sourceMin",11.0)]}),
        )
        .await,
    );
    assert_eq!(doc(&client).await, zero_before);
    apply_transform_range_commands(&client, vec![transform_range_edit("sourceMin", 10.0)]).await;
    assert_eq!(doc(&client).await["document"], zero_before["document"]);
    apply_transform_range_commands(&client, vec![transform_range_edit("sourceMax", 100.0)]).await;
    assert_eq!(
        transform_range_query(&client).await["range"],
        json!({"sourceMin":10.0,"sourceMax":100.0,"targetMin":-30.0,"targetMax":150.0})
    );
    let mut flat_raw = zero_before["document"]["constraints"][0].clone();
    flat_raw.as_object_mut().unwrap().remove("type");
    flat_raw["mappings"][0]["to"][0]["max"] = json!(-30.0);
    apply_transform_range_commands(
        &client,
        vec![json!({"command":"setTransformConstraint",
        "constraint":0,"value":flat_raw})],
    )
    .await;
    let flat_before = doc(&client).await;
    assert_eq!(
        transform_range_query(&client).await["range"],
        json!({"sourceMin":10.0,"sourceMax":null,"targetMin":-30.0,"targetMax":-30.0})
    );
    error(
        call(
            &client,
            "apply_commands",
            json!({"expected_revision":flat_before["state"]["revision"],
        "commands":[transform_range_edit("sourceMax",100.0)]}),
        )
        .await,
    );
    assert_eq!(doc(&client).await, flat_before);
    assert_eq!(fs::read(root.0.join("mapping.peachy")).unwrap(), original);
    client.cancel().await.unwrap();
}

fn write_multi_vertex_weight_fixture(root: &Path) -> Vec<u8> {
    let mut fixture = serde_json::to_value(
        peachy_document::Document::new("Direct weight matrix".into(), 60).unwrap(),
    )
    .unwrap();
    let origins = [-75.0_f32, 0.0, 75.0];
    for (index, x) in origins.iter().enumerate() {
        fixture["bones"].as_array_mut().unwrap().push(json!({
            "name":format!("weight_{}", ['A', 'B', 'C'][index]),
            "parent":0,"length":50.0,"inherit":"normal",
            "setup":{"x":x,"y":-100.0,"rotation":0.0,"scale_x":1.0,
                "scale_y":1.0,"shear_x":0.0,"shear_y":0.0}
        }));
    }
    let points = [
        [-96.0_f32, 0.0],
        [-48.0, 72.0],
        [48.0, 72.0],
        [96.0, 0.0],
        [48.0, -72.0],
        [-48.0, -72.0],
    ];
    let weights = [
        [1.0_f32, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.2, 0.3, 0.5],
        [0.65, 0.25, 0.1],
        [0.4, 0.0, 0.6],
        [0.05, 0.15, 0.8],
    ];
    let influences: Vec<Vec<Value>> = points
        .iter()
        .zip(weights)
        .map(|(point, weights)| {
            origins
                .iter()
                .zip(weights)
                .enumerate()
                .map(|(bone, (x, weight))| {
                    json!({"bone":bone+1,"position":[point[0]-x,point[1]+100.0],"weight":weight})
                })
                .collect()
        })
        .collect();
    let uvs: Vec<_> = points
        .iter()
        .map(|point| [(point[0] + 96.0) / 192.0, (72.0 - point[1]) / 144.0])
        .collect();
    fixture["slots"] = json!([{"name":"weight_mesh","bone":0,
        "attachment":"weight_mesh","color":[255,255,255,255],"blend":"normal"}]);
    fixture["skins"] = json!({"default":{"weight_mesh":{"weight_mesh":{
        "type":"mesh","path":"blue_cutout","width":128.0,"height":128.0,
        "vertices":{"mode":"weighted","influences":influences},
        "uvs":uvs,"triangles":[0,1,2,0,2,3,0,3,4,0,4,5],"hull":6
    }}}});
    let fixture: peachy_document::Document = serde_json::from_value(fixture).unwrap();
    fixture.validate().unwrap();
    fs::create_dir(root.join("images")).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/peach-bot/images/blue_cutout.png"),
        root.join("images/blue_cutout.png"),
    )
    .unwrap();
    peachy_io::create_project(&root.join("weights.peachy"), &fixture).unwrap();
    fs::read(root.join("weights.peachy")).unwrap()
}

fn multi_vertex_weight_edit(pose: &Value, vertices: Value, edit: Value) -> Value {
    json!({"command":"editMeshBoneWeights",
        "attachment":{"skin":"default","slot":"weight_mesh","placeholder":"weight_mesh"},
        "vertices":vertices,"bone":1,"edit":edit,"pose":pose})
}

fn assert_multi_vertex_weight_result(before: &Value, after: &Value, changes: &[(usize, [f64; 3])]) {
    let mut expected = before.clone();
    for (vertex, weights) in changes {
        for (bone, weight) in weights.iter().enumerate() {
            let actual = &after["skins"]["default"]["weight_mesh"]["weight_mesh"]["vertices"]["influences"]
                [*vertex][bone]["weight"];
            assert!((actual.as_f64().unwrap() - weight).abs() < 0.000001);
            expected["skins"]["default"]["weight_mesh"]["weight_mesh"]["vertices"]["influences"]
                [*vertex][bone]["weight"] = actual.clone();
        }
    }
    // Only these weights may change: preserve every bind position, zero entry,
    // unselected vertex and unrelated document field through the actual protocol.
    assert_eq!(after, &expected);
}

async fn apply_multi_vertex_weight_edit(
    client: &RunningService<RoleClient, ()>,
    command: Value,
) -> Value {
    let revision = doc(client).await["state"]["revision"].clone();
    success(
        call(
            client,
            "apply_commands",
            json!({"expected_revision":revision,"commands":[command]}),
        )
        .await,
    )
}

async fn multi_vertex_weight_history(client: &RunningService<RoleClient, ()>, action: &str) {
    let revision = doc(client).await["state"]["revision"].clone();
    success(
        call(
            client,
            "history",
            json!({"expected_revision":revision,"action":action}),
        )
        .await,
    );
}

#[tokio::test]
async fn stdio_multi_vertex_direct_weights_clamp_history_and_persistence() {
    let root = Sandbox::new();
    let original = write_multi_vertex_weight_fixture(&root.0);
    let client = start(&root.0).await;
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":0,"path":"weights.peachy"}),
        )
        .await,
    );
    let baseline = doc(&client).await["document"].clone();
    let evaluated = success(call(&client, "evaluate_pose", json!({})).await);
    let pose = json!({"locals":evaluated["appliedLocals"],"inherits":evaluated["inherits"]});
    let read_before = doc(&client).await;
    let mixed = success(
        call(
            &client,
            "get_mesh_weights",
            json!({
                "expected_revision":read_before["state"]["revision"],
                "skin":"default","slot":"weight_mesh","placeholder":"weight_mesh",
                "vertices":[3,2],"bone":1
            }),
        )
        .await,
    );
    assert_eq!(mixed["vertices"], json!([2, 3]));
    assert_eq!(mixed["value"], json!({"kind":"mixed"}));
    assert_eq!(mixed["weights"][0]["vertex"], 2);
    assert_eq!(mixed["weights"][1]["vertex"], 3);
    assert!((mixed["weights"][0]["weight"].as_f64().unwrap() - 0.2).abs() < 0.000001);
    assert!((mixed["weights"][1]["weight"].as_f64().unwrap() - 0.65).abs() < 0.000001);
    assert_eq!(doc(&client).await, read_before);
    apply_multi_vertex_weight_edit(
        &client,
        multi_vertex_weight_edit(
            &pose,
            json!([0, 1, 2, 3, 4, 5]),
            json!({"mode":"absolute","weight":0.5}),
        ),
    )
    .await;
    let absolute = doc(&client).await;
    assert_multi_vertex_weight_result(
        &baseline,
        &absolute["document"],
        &[
            (0, [1.0, 0.0, 0.0]), // No positive recipients: skip only this vertex.
            (1, [0.5, 0.5, 0.0]),
            (2, [0.5, 0.1875, 0.3125]),
            (3, [0.5, 5.0 / 14.0, 1.0 / 7.0]),
            (4, [0.5, 0.0, 0.5]),
            (5, [0.5, 3.0 / 38.0, 8.0 / 19.0]),
        ],
    );
    let uniform = success(
        call(
            &client,
            "get_mesh_weights",
            json!({
                "expected_revision":absolute["state"]["revision"],
                "skin":"default","slot":"weight_mesh","placeholder":"weight_mesh",
                "vertices":[5,4,3,2,1],"bone":1
            }),
        )
        .await,
    );
    assert_eq!(uniform["vertices"], json!([1, 2, 3, 4, 5]));
    assert_eq!(uniform["value"], json!({"kind":"uniform","weight":0.5}));
    assert_eq!(doc(&client).await, absolute);
    multi_vertex_weight_history(&client, "undo").await;
    assert_eq!(doc(&client).await["document"], baseline);

    apply_multi_vertex_weight_edit(
        &client,
        multi_vertex_weight_edit(
            &pose,
            json!([2, 3]),
            json!({"mode":"relative","deltas":[2.0,-1.0/7.0]}),
        ),
    )
    .await;
    let reversed = doc(&client).await["document"].clone();
    assert_multi_vertex_weight_result(
        &baseline,
        &reversed,
        &[
            (2, [6.0 / 7.0, 3.0 / 56.0, 5.0 / 56.0]),
            (3, [6.0 / 7.0, 5.0 / 49.0, 2.0 / 49.0]),
        ],
    );
    // The held clamp and reversal are one command, so one Undo restores raw input.
    multi_vertex_weight_history(&client, "undo").await;
    assert_eq!(doc(&client).await["document"], baseline);
    multi_vertex_weight_history(&client, "redo").await;
    assert_eq!(doc(&client).await["document"], reversed);
    multi_vertex_weight_history(&client, "undo").await;

    apply_multi_vertex_weight_edit(
        &client,
        multi_vertex_weight_edit(
            &pose,
            json!([2, 3]),
            json!({"mode":"absolute","weight":1.0}),
        ),
    )
    .await;
    let committed = doc(&client).await["document"].clone();
    assert_multi_vertex_weight_result(
        &baseline,
        &committed,
        &[(2, [1.0, 0.0, 0.0]), (3, [1.0, 0.0, 0.0])],
    );
    apply_multi_vertex_weight_edit(
        &client,
        multi_vertex_weight_edit(
            &pose,
            json!([2, 3]),
            json!({"mode":"absolute","weight":0.5}),
        ),
    )
    .await;
    assert_eq!(doc(&client).await["document"], committed);
    let revision = doc(&client).await["state"]["revision"].clone();
    let saved = success(
        call(
            &client,
            "save_project",
            json!({
                "expected_revision":revision,"directory":"saved-weights"
            }),
        )
        .await,
    );
    success(
        call(
            &client,
            "open_project",
            json!({
                "expected_revision":saved["state"]["revision"],"path":saved["path"]
            }),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], committed);
    assert_eq!(fs::read(root.0.join("weights.peachy")).unwrap(), original);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn stdio_multi_vertex_direct_weights_rejections_preserve_document_and_redo() {
    let root = Sandbox::new();
    write_multi_vertex_weight_fixture(&root.0);
    let client = start(&root.0).await;
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":0,"path":"weights.peachy"}),
        )
        .await,
    );
    let evaluated = success(call(&client, "evaluate_pose", json!({})).await);
    let pose = json!({"locals":evaluated["appliedLocals"],"inherits":evaluated["inherits"]});
    let valid = multi_vertex_weight_edit(
        &pose,
        json!([2, 3]),
        json!({"mode":"absolute","weight":0.5}),
    );
    apply_multi_vertex_weight_edit(&client, valid.clone()).await;
    let edited = doc(&client).await["document"].clone();
    multi_vertex_weight_history(&client, "undo").await;
    let before = doc(&client).await;
    assert_eq!(before["state"]["canRedo"], true);
    for (revision, vertices) in [
        (before["state"]["revision"].as_u64().unwrap(), json!([])),
        (
            before["state"]["revision"].as_u64().unwrap() - 1,
            json!([2, 3]),
        ),
    ] {
        let rejected = call(
            &client,
            "get_mesh_weights",
            json!({
                "expected_revision":revision,"skin":"default","slot":"weight_mesh",
                "placeholder":"weight_mesh","vertices":vertices,"bone":1
            }),
        )
        .await;
        assert_eq!(rejected.is_error, Some(true), "{rejected:?}");
        assert_eq!(doc(&client).await, before);
    }
    let mut invalid = vec![];
    for vertices in [json!([]), json!([2, 2]), json!([2, 6]), Value::Null] {
        let mut command = valid.clone();
        command["vertices"] = vertices;
        invalid.push(command);
    }
    let mut null_edit = valid.clone();
    null_edit["edit"] = Value::Null;
    invalid.push(null_edit);
    let mut unknown_command = valid.clone();
    unknown_command["future"] = json!(true);
    invalid.push(unknown_command);
    let mut unknown_edit = valid.clone();
    unknown_edit["edit"]["future"] = json!(true);
    invalid.push(unknown_edit);
    for command in invalid {
        let rejected = call(
            &client,
            "apply_commands",
            json!({
                "expected_revision":before["state"]["revision"],"commands":[
                    {"command":"renameBone","bone":0,"name":"must-rollback"},command
                ]
            }),
        )
        .await;
        // Serde failures may be plain MCP error text; domain failures are structured.
        assert_eq!(rejected.is_error, Some(true), "{rejected:?}");
        assert_eq!(doc(&client).await, before);
    }
    let stale = before["state"]["revision"].as_u64().unwrap() - 1;
    let rejected = error(
        call(
            &client,
            "apply_commands",
            json!({
                "expected_revision":stale,"commands":[valid]
            }),
        )
        .await,
    );
    assert!(
        rejected["error"]
            .as_str()
            .unwrap()
            .contains("Stale revision")
    );
    assert_eq!(doc(&client).await, before);
    multi_vertex_weight_history(&client, "redo").await;
    assert_eq!(doc(&client).await["document"], edited);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn stdio_discovery_atomic_edit_history_save_and_export() {
    let root = Sandbox::new();
    root.fixture();
    let source = root.0.join("source/peach-bot.peachy");
    let original = fs::read(&source).unwrap();
    let client = start(&root.0).await;
    let tools = client.list_tools(Default::default()).await.unwrap().tools;
    assert_eq!(tools.len(), 13);
    assert!(tools.iter().all(|tool| !tool.input_schema.is_empty()));
    let apply = tools
        .iter()
        .find(|tool| tool.name == "apply_commands")
        .unwrap();
    let schema = serde_json::to_value(&apply.input_schema).unwrap();
    assert!(schema.to_string().contains("setSetup"));
    assert!(schema.to_string().contains("shear_x"));
    assert_eq!(doc(&client).await["state"]["revision"], 0);
    assert_eq!(
        success(
            call(
                &client,
                "inspect_file",
                json!({"path":"source/peach-bot.peachy"})
            )
            .await
        )["valid"],
        true
    );
    assert_eq!(open(&client).await["revision"], 1);
    let before = doc(&client).await;
    let failure=error(call(&client,"apply_commands",json!({"expected_revision":1,"commands":[{"command":"renameBone","bone":0,"name":"edited-root"},{"command":"setBoneLength","bone":0,"length":-1}]})).await);
    assert_eq!(failure["revision"], 1);
    assert_eq!(doc(&client).await, before);
    let edited=success(call(&client,"apply_commands",json!({"expected_revision":1,"commands":[{"command":"renameBone","bone":0,"name":"edited-root"},{"command":"setBoneLength","bone":0,"length":42}]})).await);
    assert_eq!(edited["revision"], 2);
    assert_eq!(edited["dirty"], true);
    assert!(error(call(&client,"apply_commands",json!({"expected_revision":1,"commands":[{"command":"setBoneLength","bone":0,"length":5}]})).await)["error"].as_str().unwrap().contains("Stale revision"));
    assert!(
        error(
            call(
                &client,
                "open_project",
                json!({"expected_revision":2,"path":"source/peach-bot.peachy"})
            )
            .await
        )["error"]
            .as_str()
            .unwrap()
            .contains("Unsaved")
    );
    assert_eq!(
        success(
            call(
                &client,
                "history",
                json!({"expected_revision":2,"action":"undo"})
            )
            .await
        )["revision"],
        3
    );
    assert_eq!(doc(&client).await["document"], before["document"]);
    assert_eq!(
        success(
            call(
                &client,
                "history",
                json!({"expected_revision":3,"action":"redo"})
            )
            .await
        )["revision"],
        4
    );
    let evaluated = success(
        call(
            &client,
            "evaluate_pose",
            json!({"animation":"wave","time":0.3,"skin":"peach"}),
        )
        .await,
    );
    assert_eq!(evaluated["locals"].as_array().unwrap().len(), 15);
    assert!(!evaluated["attachments"].as_array().unwrap().is_empty());
    assert!(evaluated["bounds"].is_object());
    let saved = success(
        call(
            &client,
            "save_project",
            json!({"expected_revision":4,"directory":"edited"}),
        )
        .await,
    );
    assert_eq!(saved["path"], "edited/project.peachy");
    assert_eq!(saved["state"]["dirty"], false);
    assert_eq!(saved["state"]["revision"], 5);
    let saved_doc = peachy_io::load(&root.0.join("edited/project.peachy")).unwrap();
    assert_eq!(saved_doc.bones[0].name, "edited-root");
    assert_eq!(saved_doc.bones[0].length, 42.0);
    for image in saved_doc.image_paths() {
        assert_eq!(
            fs::read(root.0.join("source/images").join(format!("{image}.png"))).unwrap(),
            fs::read(root.0.join("edited/images").join(format!("{image}.png"))).unwrap()
        );
    }
    assert!(
        error(
            call(
                &client,
                "save_project",
                json!({"expected_revision":5,"directory":"edited"})
            )
            .await
        )["error"]
            .as_str()
            .unwrap()
            .contains("exists")
    );
    success(
        call(
            &client,
            "export_project",
            json!({"expected_revision":5,"kind":"json","path":"export.json"}),
        )
        .await,
    );
    assert!(
        peachy_spine::import_json(
            &serde_json::from_slice(&fs::read(root.0.join("export.json")).unwrap()).unwrap(),
            "roundtrip"
        )
        .is_ok()
    );
    error(
        call(
            &client,
            "export_project",
            json!({"expected_revision":5,"kind":"json","path":"export.json"}),
        )
        .await,
    );
    let bundle = success(
        call(
            &client,
            "export_project",
            json!({"expected_revision":5,"kind":"bundle","path":"."}),
        )
        .await,
    );
    let folder = root.0.join(bundle["path"].as_str().unwrap());
    for file in ["character.png", "character.atlas", "character.json"] {
        assert!(folder.join(file).is_file());
    }
    assert_eq!(fs::read(&source).unwrap(), original);
    // Export doesn't alter history, dirty state or revision.
    assert_eq!(doc(&client).await["state"], saved["state"]);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn stdio_bind_pose_inherits_capture_keyed_modes_and_preserve_geometry() {
    use peachy_document::{Inherit, InheritKey, InheritTrack};

    let root = Sandbox::new();
    root.fixture();
    let original_path = root.0.join("source/peach-bot.peachy");
    let original_bytes = fs::read(&original_path).unwrap();
    let mut fixture: peachy_document::Document = serde_json::from_slice(&original_bytes).unwrap();
    assert_eq!(fixture.bones[8].name, "hand.R");
    fixture.bones[0].setup.rotation = 35.;
    fixture.animations[0].inherit_tracks = vec![InheritTrack {
        bone: 8,
        keys: vec![InheritKey {
            time: 0.,
            inherit: Inherit::OnlyTranslation,
        }],
    }];
    fixture.validate().unwrap();
    peachy_io::create_project(&root.0.join("source/inherited-bind.peachy"), &fixture).unwrap();
    let client = start(&root.0).await;
    success(
        call(
            &client,
            "open_project",
            json!({
                "expected_revision":0,"path":"source/inherited-bind.peachy"
            }),
        )
        .await,
    );
    let attachment = json!({"skin":"default","slot":"hand.R","placeholder":"hand.R"});
    success(
        call(
            &client,
            "apply_commands",
            json!({
                "expected_revision":1,"commands":[{
                    "command":"convertRegionToMesh","attachment":attachment
                }]
            }),
        )
        .await,
    );
    let arguments = json!({"animation":"wave","time":0.3,"skin":"peach"});
    let before = success(call(&client, "evaluate_pose", arguments.clone()).await);
    assert_eq!(
        before["inherits"].as_array().unwrap().len(),
        fixture.bones.len()
    );
    assert_eq!(before["inherits"][8], "onlyTranslation");
    let setup = success(call(&client, "evaluate_pose", json!({})).await);
    assert_eq!(setup["inherits"][8], "normal");
    let pose = json!({"locals":before["appliedLocals"],"inherits":before["inherits"]});
    let vertices = |sample: &Value| -> Vec<[f32; 2]> {
        let draw = sample["attachments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|draw| draw["slot"] == 13)
            .unwrap();
        serde_json::from_value(draw["vertices"].clone()).unwrap()
    };
    let difference = |left: &Value, right: &Value| -> f32 {
        let left = vertices(left);
        let right = vertices(right);
        assert_eq!(left.len(), right.len());
        left.iter()
            .flatten()
            .zip(right.iter().flatten())
            .map(|(a, b)| (a - b).abs())
            .fold(0., f32::max)
    };
    let pre_bind = doc(&client).await;
    // Missing modes deliberately retains legacy Setup reconstruction. The
    // changed geometry proves this fixture actually exercises the new field.
    success(
        call(
            &client,
            "apply_commands",
            json!({
                "expected_revision":2,"commands":[{
                    "command":"bindMeshToBone","attachment":attachment,"bone":0,
                    "pose":{"locals":before["appliedLocals"]}
                }]
            }),
        )
        .await,
    );
    let legacy = success(call(&client, "evaluate_pose", arguments.clone()).await);
    assert!(difference(&before, &legacy) > 1.);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":3,"action":"undo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], pre_bind["document"]);
    success(
        call(
            &client,
            "apply_commands",
            json!({
                "expected_revision":4,"commands":[{
                    "command":"bindMeshToBone","attachment":attachment,"bone":0,"pose":pose
                }]
            }),
        )
        .await,
    );
    let bound = success(call(&client, "evaluate_pose", arguments.clone()).await);
    assert!(difference(&before, &bound) <= 0.001);
    let stable = doc(&client).await;
    let bad_length = error(
        call(
            &client,
            "apply_commands",
            json!({
                "expected_revision":5,"commands":[
                    {"command":"renameBone","bone":0,"name":"must-rollback"},
                    {"command":"addMeshBoneBinding","attachment":attachment,"bone":8,
                     "pose":{"locals":bound["appliedLocals"],"inherits":["normal"]}}
                ]
            }),
        )
        .await,
    );
    assert!(bad_length["error"].as_str().unwrap().contains("inherit"));
    assert_eq!(doc(&client).await, stable);
    for invalid in [Value::Null, json!(["futureMode"])] {
        let rejected = call(
            &client,
            "apply_commands",
            json!({
                "expected_revision":5,"commands":[{
                    "command":"addMeshBoneBinding","attachment":attachment,"bone":8,
                    "pose":{"locals":bound["appliedLocals"],"inherits":invalid}
                }]
            }),
        )
        .await;
        assert_eq!(rejected.is_error, Some(true));
        assert_eq!(doc(&client).await, stable);
    }
    let pose = json!({"locals":bound["appliedLocals"],"inherits":bound["inherits"]});
    success(
        call(
            &client,
            "apply_commands",
            json!({
                "expected_revision":5,"commands":[{
                    "command":"addMeshBoneBinding","attachment":attachment,"bone":8,"pose":pose
                }]
            }),
        )
        .await,
    );
    success(
        call(
            &client,
            "apply_commands",
            json!({
                "expected_revision":6,"commands":[
                    {"command":"setMeshBoneWeight","attachment":attachment,"vertex":0,
                     "bone":8,"weight":0.35,"pose":pose},
                    {"command":"setMeshBoneWeight","attachment":attachment,"vertex":1,
                     "bone":8,"weight":0.35,"pose":pose}
                ]
            }),
        )
        .await,
    );
    let edited = success(call(&client, "evaluate_pose", arguments.clone()).await);
    assert!(difference(&before, &edited) <= 0.001);
    let final_document = doc(&client).await["document"].clone();
    success(
        call(
            &client,
            "save_project",
            json!({
                "expected_revision":7,"directory":"inherited-output"
            }),
        )
        .await,
    );
    success(
        call(
            &client,
            "open_project",
            json!({
                "expected_revision":8,"path":"inherited-output/project.peachy"
            }),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], final_document);
    let reopened = success(call(&client, "evaluate_pose", arguments).await);
    assert!(difference(&before, &reopened) <= 0.001);
    assert_eq!(fs::read(original_path).unwrap(), original_bytes);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn stdio_create_project_authors_region_and_weighted_mesh_without_importing() {
    let root = Sandbox::new();
    root.fixture();
    let image = root.0.join("source/images/hand.png");
    let source_image = fs::read(&image).unwrap();
    let client = start(&root.0).await;
    let created = success(
        call(
            &client,
            "create_project",
            json!({
                "expected_revision":0,"name":"Fresh tutorial rig","asset_root":"source"
            }),
        )
        .await,
    );
    assert_eq!(created["revision"], 1);
    assert_eq!(created["fps"], 30);
    assert_eq!(created["bones"], 1);
    assert_eq!(created["slots"], 0);
    assert!(created["source"].is_null());
    let blank = doc(&client).await;
    assert_eq!(blank["document"]["bones"][0]["name"], "root");
    assert_eq!(blank["document"]["skins"], json!({"default":{}}));
    assert_eq!(blank["document"]["animations"], json!([]));
    let transform = json!({
        "x":12.0,"y":20.0,"rotation":0.0,"scale_x":1.0,"scale_y":1.0,
        "shear_x":0.0,"shear_y":0.0
    });
    let mesh = json!({
        "type":"mesh","path":"hand","width":54.0,"height":56.0,
        "vertices":{"mode":"weighted","influences":[
            [{"bone":1,"position":[0.0,0.0],"weight":1.0}],
            [{"bone":1,"position":[12.0,0.0],"weight":1.0}],
            [{"bone":1,"position":[0.0,12.0],"weight":1.0}]
        ]},
        "uvs":[[0.0,0.0],[1.0,0.0],[0.0,1.0]],"triangles":[0,1,2],"hull":3
    });
    let commands = json!([
        {"command":"addBone","bone":{
            "name":"hair","parent":0,"length":30.0,"setup":transform,"inherit":"normal"
        }},
        {"command":"addSlot","slot":{"name":"body","bone":0,"attachment":null,"color":[255,255,255,255]}},
        {"command":"addAttachment","attachment":{"skin":"default","slot":"body","placeholder":"body"},
         "value":{"type":"region","path":"torso","width":140.0,"height":148.0,
          "transform":{"x":0.0,"y":0.0,"rotation":0.0,"scale_x":1.0,"scale_y":1.0}}},
        {"command":"setSlotAttachment","slot":0,"attachment":"body"},
        {"command":"addSlot","slot":{"name":"hair","bone":1,"attachment":null,"color":[255,255,255,255]}},
        {"command":"addAttachment","attachment":{"skin":"default","slot":"hair","placeholder":"hair"},"value":mesh},
        {"command":"setSlotAttachment","slot":1,"attachment":"hair"},
        {"command":"addAnimation","name":"wind","duration":0.0},
        {"command":"setKey","animation":0,"bone":1,"property":"rotation","key":{"time":0.0,"value":0.0,"curve":"linear"}},
        {"command":"setKey","animation":0,"bone":1,"property":"rotation","key":{"time":0.5,"value":18.0,"curve":"linear"}},
        {"command":"setKey","animation":0,"bone":1,"property":"rotation","key":{"time":1.0,"value":0.0,"curve":"linear"}},
        {"command":"setDeformKey","animation":0,"attachment":{"skin":"default","slot":"hair","placeholder":"hair"},
         "key":{"time":0.5,"curve":"linear","offsets":[0.0,0.0,2.0,0.0,0.0,0.0]}}
    ]);
    let authored = success(
        call(
            &client,
            "apply_commands",
            json!({"expected_revision":1,"commands":commands}),
        )
        .await,
    );
    assert_eq!(authored["revision"], 2);
    assert_eq!(authored["dirty"], true);
    assert_eq!(authored["bones"], 2);
    assert_eq!(authored["slots"], 2);
    let authored_doc = doc(&client).await;
    assert!(
        error(
            call(
                &client,
                "create_project",
                json!({
                    "expected_revision":2,"name":"Replacement","asset_root":"source"
                })
            )
            .await
        )["error"]
            .as_str()
            .unwrap()
            .contains("Unsaved")
    );
    assert_eq!(doc(&client).await, authored_doc);
    for invalid in [
        json!({"expected_revision":2,"name":"","asset_root":"source","discard_changes":true}),
        json!({"expected_revision":2,"name":"Replacement","fps":0,"asset_root":"source","discard_changes":true}),
        json!({"expected_revision":2,"name":"Replacement","asset_root":"source/peach-bot.peachy","discard_changes":true}),
        json!({"expected_revision":1,"name":"Replacement","asset_root":"source","discard_changes":true}),
    ] {
        error(call(&client, "create_project", invalid).await);
        assert_eq!(doc(&client).await, authored_doc);
    }
    error(call(&client,"apply_commands",json!({"expected_revision":2,"commands":[
        {"command":"renameBone","bone":0,"name":"must-rollback"},
        {"command":"addAttachment","attachment":{"skin":"default","slot":"hair","placeholder":"hair"},"value":mesh}
    ]})).await);
    assert_eq!(doc(&client).await, authored_doc);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":2,"action":"undo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], blank["document"]);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":3,"action":"redo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], authored_doc["document"]);
    let pose = success(
        call(
            &client,
            "evaluate_pose",
            json!({"animation":"wind","time":0.5}),
        )
        .await,
    );
    assert_eq!(pose["world"].as_array().unwrap().len(), 2);
    assert_eq!(pose["attachments"].as_array().unwrap().len(), 2);
    let saved = success(
        call(
            &client,
            "save_project",
            json!({"expected_revision":4,"directory":"created"}),
        )
        .await,
    );
    assert_eq!(saved["state"]["source"], "created/project.peachy");
    assert_eq!(saved["state"]["dirty"], false);
    let saved_document = peachy_io::load(&root.0.join("created/project.peachy")).unwrap();
    assert_eq!(saved_document.bones.len(), 2);
    assert_eq!(saved_document.animations[0].deform_tracks.len(), 1);
    assert_eq!(
        fs::read(root.0.join("created/images/hand.png")).unwrap(),
        source_image
    );
    success(
        call(
            &client,
            "export_project",
            json!({"expected_revision":5,"kind":"json","path":"created.json"}),
        )
        .await,
    );
    let bundle = success(
        call(
            &client,
            "export_project",
            json!({"expected_revision":5,"kind":"bundle","path":"."}),
        )
        .await,
    );
    assert!(
        root.0
            .join(bundle["path"].as_str().unwrap())
            .join("character.atlas")
            .is_file()
    );
    let reopened = success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":5,"path":"created/project.peachy"}),
        )
        .await,
    );
    assert_eq!(reopened["revision"], 6);
    assert_eq!(doc(&client).await["document"], authored_doc["document"]);
    let reset = success(
        call(
            &client,
            "create_project",
            json!({"expected_revision":6,"name":"Next","fps":24,"asset_root":"source"}),
        )
        .await,
    );
    assert_eq!(reset["fps"], 24);
    assert_eq!(reset["bones"], 1);
    assert_eq!(reset["canUndo"], false);
    assert!(reset["source"].is_null());
    assert_eq!(fs::read(&image).unwrap(), source_image);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn invalid_protocol_arguments_and_failed_opens_preserve_session() {
    let root = Sandbox::new();
    root.fixture();
    let client = start(&root.0).await;
    open(&client).await;
    let before = doc(&client).await;
    for args in [
        json!({"expected_revision":1,"commands":[{"command":"renameBone","bone":0,"name":"x","surprise":true}]}),
        json!({"expected_revision":1,"commands":[{"command":"setSetup","bone":0,"transform":{"x":0}}]}),
        json!({"expected_revision":1,"commands":[{"command":"batch","commands":[]}]}),
        json!({"expected_revision":1,"commands":[],"extra":true}),
    ] {
        let result = client
            .call_tool(
                CallToolRequestParams::new("apply_commands")
                    .with_arguments(args.as_object().unwrap().clone()),
            )
            .await;
        assert!(
            result.is_err()
                || result
                    .as_ref()
                    .is_ok_and(|value| value.is_error == Some(true))
        );
        assert_eq!(doc(&client).await, before);
    }
    error(
        call(
            &client,
            "apply_commands",
            json!({"expected_revision":1,"commands":[]}),
        )
        .await,
    );
    error(call(&client, "evaluate_pose", json!({"time":-1})).await);
    error(call(&client, "evaluate_pose", json!({"skin":"missing"})).await);
    error(call(&client, "evaluate_pose", json!({"animation":"missing"})).await);
    fs::write(root.0.join("invalid.peachy"), b"{}").unwrap();
    error(
        call(
            &client,
            "open_project",
            json!({"expected_revision":1,"path":"invalid.peachy","discard_changes":true}),
        )
        .await,
    );
    assert_eq!(doc(&client).await, before);
    fs::write(
        root.0.join("unsupported.json"),
        br#"{"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}],"constraints":[{"type":"slider","name":"rail","bone":"root"}]}"#,
    )
    .unwrap();
    assert_eq!(
        success(call(&client, "inspect_file", json!({"path":"unsupported.json"})).await)["report"]
            ["supported"],
        false
    );
    error(
        call(
            &client,
            "open_project",
            json!({"expected_revision":1,"path":"unsupported.json"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await, before);
    fs::write(
        root.0.join("bones-only.json"),
        br#"{"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}]}"#,
    )
    .unwrap();
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":1,"path":"bones-only.json"}),
        )
        .await,
    );
    let pose = success(call(&client, "evaluate_pose", json!({})).await);
    assert_eq!(pose["locals"].as_array().unwrap().len(), 1);
    assert_eq!(pose["attachments"], json!([]));
    client.cancel().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn confines_primary_paths_output_parents_and_referenced_images() {
    use std::os::unix::fs::symlink;
    let root = Sandbox::new();
    root.fixture();
    let outside = Sandbox::new();
    fs::write(outside.0.join("protected.json"), b"private").unwrap();
    symlink(&outside.0, root.0.join("escape")).unwrap();
    symlink(outside.0.join("absent"), root.0.join("dangling.json")).unwrap();
    let client = start(&root.0).await;
    open(&client).await;
    let original = doc(&client).await;
    for asset_root in [
        "../outside".to_owned(),
        "escape".to_owned(),
        outside.0.to_string_lossy().into_owned(),
    ] {
        error(
            call(
                &client,
                "create_project",
                json!({"expected_revision":1,"name":"Escape","asset_root":asset_root}),
            )
            .await,
        );
        assert_eq!(doc(&client).await, original);
    }
    for path in ["../outside.json", "escape/protected.json"] {
        error(call(&client, "inspect_file", json!({"path":path})).await);
    }
    error(
        call(
            &client,
            "inspect_file",
            json!({"path":outside.0.join("protected.json")}),
        )
        .await,
    );
    for path in ["escape/new.json", "dangling.json", "../escape.json"] {
        error(
            call(
                &client,
                "export_project",
                json!({"expected_revision":1,"kind":"json","path":path}),
            )
            .await,
        );
    }
    error(
        call(
            &client,
            "save_project",
            json!({"expected_revision":1,"directory":"escape/saved"}),
        )
        .await,
    );
    let document = peachy_io::load(&root.0.join("source/peach-bot.peachy")).unwrap();
    let image = document
        .image_paths()
        .into_iter()
        .next()
        .unwrap()
        .to_owned();
    let image_path = root.0.join("source/images").join(format!("{image}.png"));
    fs::rename(&image_path, outside.0.join("art.png")).unwrap();
    symlink(outside.0.join("art.png"), &image_path).unwrap();
    let before = doc(&client).await;
    let failed = error(
        call(
            &client,
            "save_project",
            json!({"expected_revision":1,"directory":"saved"}),
        )
        .await,
    );
    assert!(failed["error"].as_str().unwrap().contains("outside"));
    assert!(!root.0.join("saved").exists());
    error(
        call(
            &client,
            "export_project",
            json!({"expected_revision":1,"kind":"bundle","path":"."}),
        )
        .await,
    );
    assert_eq!(doc(&client).await, before);
    assert_eq!(
        fs::read(outside.0.join("protected.json")).unwrap(),
        b"private"
    );
    assert!(!outside.0.join("saved").exists());
    assert!(!outside.0.join("new.json").exists());
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn stdio_deform_timeline_copy_curve_retime_paste_and_persistence() {
    let root = Sandbox::new();
    root.fixture();
    let source = root.0.join("source/deform-bot.peachy");
    let original = fs::read(&source).unwrap();
    let client = start(&root.0).await;
    let tools = client.list_tools(Default::default()).await.unwrap().tools;
    let copy_tool = tools
        .iter()
        .find(|t| t.name == "copy_animation_keys")
        .unwrap();
    assert_eq!(
        copy_tool.annotations.as_ref().unwrap().read_only_hint,
        Some(true)
    );
    assert!(
        serde_json::to_value(&copy_tool.input_schema)
            .unwrap()
            .to_string()
            .contains("deform")
    );
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":0,"path":"source/deform-bot.peachy"}),
        )
        .await,
    );
    let baseline = doc(&client).await;
    let track = &baseline["document"]["animations"][0]["deform_tracks"][0];
    let reference = track["attachment"].clone();
    let first = track["keys"][0]["time"].clone();
    let second = track["keys"][1]["time"].clone();
    let identities = json!([
        {"type":"deform","attachment":reference,"time":first},
        {"type":"deform","attachment":reference,"time":second}
    ]);
    let copied = success(
        call(
            &client,
            "copy_animation_keys",
            json!({"expected_revision":1,"animation":0,"keys":identities}),
        )
        .await,
    );
    assert_eq!(copied["revision"], 1);
    let clipboard = copied["clipboard"].clone();
    assert_eq!(clipboard["type"], "deform");
    assert_eq!(clipboard["layout"]["type"], "weighted");
    assert_eq!(clipboard["keys"][0]["time"], 0.0);
    assert_eq!(clipboard["keys"][0]["offsets"], track["keys"][0]["offsets"]);
    assert_eq!(clipboard["keys"][1]["curve"], track["keys"][1]["curve"]);
    assert_eq!(doc(&client).await, baseline);
    let changed=success(call(&client,"apply_commands",json!({"expected_revision":1,"commands":[
        {"command":"setDeformCurve","animation":0,"attachment":reference,"time":second,"curve":"stepped"},
        {"command":"moveAnimationKeys","animation":0,"keys":[{"type":"deform","attachment":reference,"time":first}],"delta":0.25,"copy":true},
        {"command":"pasteAnimationKeys","animation":1,"target":{"type":"deform","attachment":reference},"time":0.5,"clipboard":clipboard}
    ]})).await);
    assert_eq!(changed["revision"], 2);
    let edited = doc(&client).await;
    let keys = edited["document"]["animations"][0]["deform_tracks"][0]["keys"]
        .as_array()
        .unwrap();
    let curve_key = keys.iter().find(|k| k["time"] == second).unwrap();
    assert_eq!(curve_key["curve"], "stepped");
    assert_eq!(curve_key["offsets"], track["keys"][1]["offsets"]);
    assert!(
        keys.iter()
            .any(|k| k["time"] == 0.25 && k["offsets"] == track["keys"][0]["offsets"])
    );
    let pasted = &edited["document"]["animations"][1]["deform_tracks"][0]["keys"];
    assert_eq!(pasted[0]["time"], 0.5);
    assert_eq!(pasted[0]["offsets"], clipboard["keys"][0]["offsets"]);
    let stale = error(
        call(
            &client,
            "copy_animation_keys",
            json!({"expected_revision":1,"animation":0,"keys":identities}),
        )
        .await,
    );
    assert!(stale["error"].as_str().unwrap().contains("Stale revision"));
    let mut incompatible = clipboard.clone();
    incompatible["layout"]["influence_counts"][0] = json!(999);
    error(call(&client,"apply_commands",json!({"expected_revision":2,"commands":[
        {"command":"renameBone","bone":0,"name":"must-rollback"},
        {"command":"pasteAnimationKeys","animation":0,"target":{"type":"deform","attachment":reference},"time":3.0,"clipboard":incompatible}
    ]})).await);
    assert_eq!(doc(&client).await, edited);
    error(call(&client,"apply_commands",json!({"expected_revision":2,"commands":[
        {"command":"setDeformCurve","animation":0,"attachment":reference,"time":9.0,"curve":"linear"}
    ]})).await);
    assert_eq!(doc(&client).await, edited);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":2,"action":"undo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], baseline["document"]);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":3,"action":"redo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], edited["document"]);
    success(call(&client,"apply_commands",json!({"expected_revision":4,"commands":[
        {"command":"deleteAnimationKeys","animation":0,"keys":[{"type":"deform","attachment":reference,"time":0.25}]}
    ]})).await);
    assert!(
        !doc(&client).await["document"]["animations"][0]["deform_tracks"][0]["keys"]
            .as_array()
            .unwrap()
            .iter()
            .any(|k| k["time"] == 0.25)
    );
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":5,"action":"undo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], edited["document"]);
    success(
        call(
            &client,
            "save_project",
            json!({"expected_revision":6,"directory":"deform-edited"}),
        )
        .await,
    );
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":7,"path":"deform-edited/project.peachy"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], edited["document"]);
    let pose = success(
        call(
            &client,
            "evaluate_pose",
            json!({"animation":"idle","time":0.75,"skin":"peach"}),
        )
        .await,
    );
    assert!(!pose["attachments"].as_array().unwrap().is_empty());
    assert_eq!(fs::read(source).unwrap(), original);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn stdio_slot_authoring_mixed_clipboard_curves_and_persistence() {
    let root = Sandbox::new();
    root.fixture();
    let source = root.0.join("source/slot-bot.peachy");
    let original = fs::read(&source).unwrap();
    let client = start(&root.0).await;
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":0,"path":"source/slot-bot.peachy"}),
        )
        .await,
    );
    let baseline = doc(&client).await;
    let curves = json!([{"bezier":[0.25,0.0,0.75,1.0]},"linear",{"bezierOffset":[0.25,0.125,0.75,-0.125]},"linear"]);
    success(call(&client,"apply_commands",json!({"expected_revision":1,"commands":[
        {"command":"setSlotAttachmentKey","animation":1,"slot":8,"key":{"time":0.25,"attachment":"fist"}},
        {"command":"setSlotAttachmentKey","animation":1,"slot":8,"key":{"time":0.75,"attachment":null}},
        {"command":"setSlotAttachmentKey","animation":1,"slot":8,"key":{"time":1.25,"attachment":"hand.L"}},
        {"command":"setSlotColorKey","animation":1,"slot":8,"key":{"time":0.25,"color":[255,128,64,255],"curves":curves}},
        {"command":"setSlotColorKey","animation":1,"slot":8,"key":{"time":0.75,"color":[0,40,200,0],"curves":["stepped","stepped","stepped","stepped"]}},
        {"command":"setSlotColorKey","animation":1,"slot":8,"key":{"time":1.25,"color":[255,255,255,255],"curves":["linear","linear","linear","linear"]}}
    ]})).await);
    let authored = doc(&client).await;
    assert_eq!(authored["document"]["slots"], baseline["document"]["slots"]);
    let identities = json!([
        {"type":"slotAttachment","slot":8,"time":0.25},{"type":"slotColor","slot":8,"time":0.25},
        {"type":"slotAttachment","slot":8,"time":0.75},{"type":"slotColor","slot":8,"time":0.75}
    ]);
    let copied = success(
        call(
            &client,
            "copy_animation_keys",
            json!({"expected_revision":2,"animation":1,"keys":identities}),
        )
        .await,
    );
    assert_eq!(copied["revision"], 2);
    assert_eq!(doc(&client).await, authored);
    let clipboard = &copied["clipboard"];
    assert_eq!(clipboard["type"], "slot");
    assert_eq!(
        clipboard["attachment_keys"],
        json!([{"time":0.0,"attachment":"fist"},{"time":0.5,"attachment":null}])
    );
    assert_eq!(clipboard["color_keys"][0]["time"], 0.0);
    assert_eq!(clipboard["color_keys"][0]["curves"], curves);
    assert_eq!(clipboard["color_keys"][1]["color"], json!([0, 40, 200, 0]));
    success(call(&client,"apply_commands",json!({"expected_revision":2,"commands":[
        {"command":"setSlotColorCurves","animation":1,"slot":8,"time":0.25,"curves":["linear","linear","linear","linear"]},
        {"command":"moveAnimationKeys","animation":1,"keys":[{"type":"slotAttachment","slot":8,"time":1.25},{"type":"slotColor","slot":8,"time":1.25}],"delta":0.25,"copy":false},
        {"command":"pasteAnimationKeys","animation":0,"target":{"type":"slot","slot":8},"time":2.75,"clipboard":clipboard}
    ]})).await);
    let edited = doc(&client).await;
    let attachment = &edited["document"]["animations"][1]["attachment_tracks"][0]["keys"];
    let color = &edited["document"]["animations"][1]["color_tracks"][0]["keys"];
    assert_eq!(attachment[2]["time"], 1.5);
    assert_eq!(color[2]["time"], 1.5);
    assert_eq!(color[0]["color"], json!([255, 128, 64, 255]));
    assert_eq!(
        color[0]["curves"],
        json!(["linear", "linear", "linear", "linear"])
    );
    let pasted = &edited["document"]["animations"][0]["color_tracks"];
    assert!(pasted.as_array().unwrap().iter().any(|t| t["slot"] == 8
        && t["keys"][0]["time"] == 2.75
        && t["keys"][0]["curves"] == curves));
    for command in [
        json!({"command":"setSlotColorCurves","animation":1,"slot":8,"time":0.25,"curves":["stepped","linear","linear","linear"]}),
        json!({"command":"setSlotColorCurves","animation":1,"slot":8,"time":9.0,"curves":["linear","linear","linear","linear"]}),
        json!({"command":"setSlotAttachmentKey","animation":1,"slot":8,"key":{"time":0.0,"attachment":" "}}),
        json!({"command":"setSlotColorKey","animation":1,"slot":999,"key":{"time":0.25,"color":[255,255,255,255],"curves":curves}}),
    ] {
        error(call(&client,"apply_commands",json!({"expected_revision":3,"commands":[{"command":"renameBone","bone":0,"name":"must-rollback"},command]})).await);
        assert_eq!(doc(&client).await, edited);
    }
    error(
        call(
            &client,
            "copy_animation_keys",
            json!({"expected_revision":2,"animation":1,"keys":identities}),
        )
        .await,
    );
    assert_eq!(doc(&client).await, edited);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":3,"action":"undo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], authored["document"]);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":4,"action":"redo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], edited["document"]);
    success(call(&client,"apply_commands",json!({"expected_revision":5,"commands":[
        {"command":"deleteAnimationKeys","animation":1,"keys":[{"type":"slotAttachment","slot":8,"time":0.25}]}
    ]})).await);
    let deleted = doc(&client).await;
    assert_eq!(
        deleted["document"]["animations"][1]["color_tracks"],
        edited["document"]["animations"][1]["color_tracks"]
    );
    assert_eq!(
        deleted["document"]["animations"][1]["attachment_tracks"][0]["keys"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":6,"action":"undo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], edited["document"]);
    success(
        call(
            &client,
            "save_project",
            json!({"expected_revision":7,"directory":"slot-edited"}),
        )
        .await,
    );
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":8,"path":"slot-edited/project.peachy"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], edited["document"]);
    for time in [0.0, 0.25, 0.5, 0.75, 1.5, 0.125] {
        let pose = success(
            call(
                &client,
                "evaluate_pose",
                json!({"animation":"idle","time":time,"skin":"peach"}),
            )
            .await,
        );
        assert!(!pose["attachments"].as_array().unwrap().is_empty());
        assert_eq!(pose["slotColors"].as_array().unwrap().len(), 14);
        if time < 0.25 {
            assert_eq!(pose["slotAttachments"][8], "hand.L");
            assert_eq!(pose["slotColors"][8], json!([1.0, 1.0, 1.0, 1.0]));
        } else if time == 0.5 {
            assert_eq!(pose["slotAttachments"][8], "fist");
            assert!((pose["slotColors"][8][3].as_f64().unwrap() * 255.0 - 127.5).abs() < 0.0001);
        } else if time == 0.75 {
            assert!(pose["slotAttachments"][8].is_null());
            assert_eq!(pose["slotColors"][8][3], 0.0);
            assert!(
                pose["attachments"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|draw| draw["slot"] != 8)
            );
        } else if time == 1.5 {
            assert_eq!(pose["slotAttachments"][8], "hand.L");
            assert_eq!(pose["slotColors"][8], json!([1.0, 1.0, 1.0, 1.0]));
        }
    }
    error(
        call(
            &client,
            "evaluate_pose",
            json!({"animation":"idle","time":-0.1,"skin":"peach"}),
        )
        .await,
    );
    let sampled = success(
        call(
            &client,
            "evaluate_pose",
            json!({"animation":"idle","time":0.5,"skin":"peach"}),
        )
        .await,
    );
    success(call(&client,"apply_commands",json!({"expected_revision":9,"commands":[
        {"command":"insertSlotColorKey","animation":1,"slot":8,"time":0.5,"color":sampled["slotColors"][8]}
    ]})).await);
    let split = doc(&client).await;
    let split_keys = &split["document"]["animations"][1]["color_tracks"][0]["keys"];
    assert_eq!(split_keys.as_array().unwrap().len(), 4);
    assert_eq!(split_keys[1]["time"], 0.5);
    assert_eq!(split_keys[0]["color"], color[0]["color"]);
    assert_eq!(split_keys[2]["color"], color[1]["color"]);
    assert_eq!(
        split["document"]["animations"][1]["attachment_tracks"],
        edited["document"]["animations"][1]["attachment_tracks"]
    );
    let after = success(
        call(
            &client,
            "evaluate_pose",
            json!({"animation":"idle","time":0.5,"skin":"peach"}),
        )
        .await,
    );
    for (before, after) in sampled["slotColors"][8]
        .as_array()
        .unwrap()
        .iter()
        .zip(after["slotColors"][8].as_array().unwrap())
    {
        assert!((before.as_f64().unwrap() - after.as_f64().unwrap()).abs() <= 0.5 / 255.0 + 1e-6);
    }
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":10,"action":"undo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], edited["document"]);
    success(
        call(
            &client,
            "export_project",
            json!({"expected_revision":11,"kind":"json","path":"slot-edited.json","target":"runtime43"}),
        )
        .await,
    );
    let exported: Value =
        serde_json::from_slice(&fs::read(root.0.join("slot-edited.json")).unwrap()).unwrap();
    assert!(exported["animations"]["idle"]["slots"]["hand.L"]["attachment"].is_array());
    assert!(exported["animations"]["idle"]["slots"]["hand.L"]["rgba"].is_array());
    assert_eq!(fs::read(source).unwrap(), original);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn stdio_export_targets_protect_editor_refs_and_preserve_runtime_identity() {
    let root = Sandbox::new();
    root.fixture();
    let source = root.0.join("source/slot-bot.peachy");
    let original = fs::read(&source).unwrap();
    let client = start(&root.0).await;
    let tools = client.list_tools(Default::default()).await.unwrap().tools;
    let schema = serde_json::to_value(
        &tools
            .iter()
            .find(|tool| tool.name == "export_project")
            .unwrap()
            .input_schema,
    )
    .unwrap();
    assert!(schema.to_string().contains("spineEditor43"));
    assert!(schema.to_string().contains("runtime43"));
    assert!(
        !schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("target"))
    );
    success(
        call(
            &client,
            "open_project",
            json!({
                "path":"source/slot-bot.peachy","expected_revision":0
            }),
        )
        .await,
    );
    let before = doc(&client).await;
    for args in [
        json!({"expected_revision":1,"kind":"json","path":"unsafe.json"}),
        json!({"expected_revision":1,"kind":"json","path":"unsafe-explicit.json","target":"spineEditor43"}),
        json!({"expected_revision":1,"kind":"bundle","path":".","target":"spineEditor43"}),
    ] {
        let failed = error(call(&client, "export_project", args).await);
        assert!(failed["error"].as_str().unwrap().contains("hand.L fist"));
        assert_eq!(failed["revision"], 1);
        assert_eq!(doc(&client).await, before);
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), 1);
    }
    let exported = success(
        call(
            &client,
            "export_project",
            json!({
                "expected_revision":1,"kind":"json","path":"runtime.json","target":"runtime43"
            }),
        )
        .await,
    );
    assert_eq!(exported["exportTarget"], "runtime43");
    let value: Value =
        serde_json::from_slice(&fs::read(root.0.join("runtime.json")).unwrap()).unwrap();
    let native: peachy_document::Document =
        serde_json::from_value(before["document"].clone()).unwrap();
    let restored = peachy_spine::import_json(&value, &native.name).unwrap();
    assert_eq!(restored.slots, native.slots);
    assert_eq!(restored.skins, native.skins);
    for clip in &native.animations {
        let imported = restored
            .animations
            .iter()
            .find(|a| a.name == clip.name)
            .unwrap();
        assert_eq!(imported.attachment_tracks, clip.attachment_tracks);
    }
    let bundle = success(
        call(
            &client,
            "export_project",
            json!({
                "expected_revision":1,"kind":"bundle","path":".","target":"runtime43"
            }),
        )
        .await,
    );
    assert_eq!(bundle["exportTarget"], "runtime43");
    let bundle_path = root.0.join(bundle["path"].as_str().unwrap());
    let bundled: Value =
        serde_json::from_slice(&fs::read(bundle_path.join("character.json")).unwrap()).unwrap();
    assert_eq!(bundled, value);
    assert!(bundle_path.join("character.atlas").exists());
    assert!(bundle_path.join("character.png").exists());
    assert_eq!(doc(&client).await, before);
    let failed = error(
        call(
            &client,
            "export_project",
            json!({
                "expected_revision":0,"kind":"json","path":"stale.json","target":"runtime43"
            }),
        )
        .await,
    );
    assert!(failed["error"].as_str().unwrap().contains("Stale revision"));
    assert!(!root.0.join("stale.json").exists());
    assert_eq!(doc(&client).await, before);
    assert_eq!(fs::read(source).unwrap(), original);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn stdio_scalar_curve_only_exact_keys_history_and_persistence() {
    let root = Sandbox::new();
    root.fixture();
    let source = root.0.join("source/peach-bot.peachy");
    let original = fs::read(&source).unwrap();
    let mut fixture: Value = serde_json::from_slice(&original).unwrap();
    let first = 0.5_f32;
    let middle = f32::from_bits(first.to_bits() + 1);
    let last = f32::from_bits(first.to_bits() + 2);
    fixture["animations"][0]["tracks"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "bone":0,"property":"rotation","keys":[
                {"time":first,"value":-0.0,"curve":"linear"},
                {"time":middle,"value":720.0,"curve":"linear"},
                {"time":last,"value":-540.0,"curve":"linear"}
            ]
        }));
    let exact_source = root.0.join("source/scalar-exact.peachy");
    fs::write(&exact_source, serde_json::to_vec(&fixture).unwrap()).unwrap();
    let client = start(&root.0).await;
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":0,"path":"source/scalar-exact.peachy"}),
        )
        .await,
    );
    let before = doc(&client).await;
    let count = before["document"]["animations"][0]["tracks"]
        .as_array()
        .unwrap()
        .len();
    let curve = json!({"bezierOffset":[0.25,2.0,0.75,-1.0]});
    success(call(&client,"apply_commands",json!({"expected_revision":1,"commands":[{
        "command":"setKeyCurve","animation":0,"bone":0,"property":"rotation","time":middle,"curve":curve
    }]})).await);
    let mut expected = before["document"].clone();
    expected["animations"][0]["tracks"][count - 1]["keys"][1]["curve"] = curve;
    assert_eq!(doc(&client).await["document"], expected);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":2,"action":"undo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], before["document"]);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":3,"action":"redo"}),
        )
        .await,
    );
    let edited = doc(&client).await;
    for time in [-0.1_f32, f32::from_bits(first.to_bits() + 3)] {
        error(call(&client,"apply_commands",json!({"expected_revision":4,"commands":[
            {"command":"setKeyCurve","animation":0,"bone":0,"property":"rotation","time":first,"curve":"stepped"},
            {"command":"setKeyCurve","animation":0,"bone":0,"property":"rotation","time":time,"curve":"linear"}
        ]})).await);
        assert_eq!(doc(&client).await, edited);
    }
    // Raw terminal metadata is addressed explicitly; presentation redirect is separate.
    success(call(&client,"apply_commands",json!({"expected_revision":4,"commands":[{
        "command":"setKeyCurve","animation":0,"bone":0,"property":"rotation","time":last,"curve":"stepped"
    }]})).await);
    expected["animations"][0]["tracks"][count - 1]["keys"][2]["curve"] = json!("stepped");
    assert_eq!(doc(&client).await["document"], expected);
    success(
        call(
            &client,
            "save_project",
            json!({"expected_revision":5,"directory":"curve-saved"}),
        )
        .await,
    );
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":6,"path":"curve-saved/project.peachy"}),
        )
        .await,
    );
    let reopened = doc(&client).await;
    assert_eq!(reopened["document"], expected);
    success(call(&client,"export_project",json!({"expected_revision":7,"kind":"json","path":"curve-runtime.json","target":"runtime43"})).await);
    assert_eq!(doc(&client).await, reopened);
    let exported: Value =
        serde_json::from_slice(&fs::read(root.0.join("curve-runtime.json")).unwrap()).unwrap();
    let keys = exported["animations"]["wave"]["bones"]["root"]["rotate"]
        .as_array()
        .unwrap();
    assert_eq!(keys.len(), 3);
    assert_eq!(keys[1]["time"].as_f64().unwrap() as f32, middle);
    assert_eq!(keys[1]["value"], 720.0);
    assert_eq!(keys[1]["curve"].as_array().unwrap().len(), 4);
    assert_eq!(fs::read(source).unwrap(), original);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn stdio_ik_curve_only_preserves_precision_values_flags_and_persistence() {
    let root = Sandbox::new();
    root.fixture();
    let source = root.0.join("source/ik-animation-bot.peachy");
    let original = fs::read(&source).unwrap();
    let client = start(&root.0).await;
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":0,"path":"source/ik-animation-bot.peachy"}),
        )
        .await,
    );
    let before = doc(&client).await;
    let curve =
        json!({"type":"bezier","time":[0.16666666666666669,0.8333333333333333],"mix":[-0.5,1.25]});
    success(
        call(
            &client,
            "apply_commands",
            json!({"expected_revision":1,"commands":[{
                "command":"setIkCurve","animation":4,"constraint":0,"time":0.25,"curve":curve
            }]}),
        )
        .await,
    );
    let mut expected = before["document"].clone();
    expected["animations"][4]["ik_tracks"][0]["keys"][0]["curve"] = curve;
    assert_eq!(doc(&client).await["document"], expected);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":2,"action":"undo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], before["document"]);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":3,"action":"redo"}),
        )
        .await,
    );
    let edited = doc(&client).await;
    let absent = f32::from_bits(0.25_f32.to_bits() + 1);
    for (time, bad_curve) in [
        (absent, json!({"type":"linear"})),
        (0.25, json!({"type":"bezier","time":[-0.1,0.8],"mix":[0,1]})),
    ] {
        error(call(&client,"apply_commands",json!({"expected_revision":4,"commands":[
            {"command":"setIkCurve","animation":4,"constraint":0,"time":0.25,"curve":{"type":"stepped"}},
            {"command":"setIkCurve","animation":4,"constraint":0,"time":time,"curve":bad_curve}
        ]})).await);
        assert_eq!(doc(&client).await, edited);
    }
    success(
        call(
            &client,
            "evaluate_pose",
            json!({"animation":"ik ratio time","time":0.625,"skin":"peach"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await, edited);
    success(
        call(
            &client,
            "save_project",
            json!({"expected_revision":4,"directory":"ik-curve-saved"}),
        )
        .await,
    );
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":5,"path":"ik-curve-saved/project.peachy"}),
        )
        .await,
    );
    let reopened = doc(&client).await;
    assert_eq!(reopened["document"], expected);
    success(call(&client,"export_project",json!({"expected_revision":6,"kind":"json","path":"ik-curve-runtime.json","target":"runtime43"})).await);
    assert_eq!(doc(&client).await, reopened);
    assert_eq!(fs::read(source).unwrap(), original);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn stdio_curve_actions_author_from_cutout_match_handles_and_resolve_terminal_selection() {
    let root = Sandbox::new();
    fs::create_dir_all(root.0.join("art/images")).unwrap();
    let image =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot/images/torso.png");
    let original = fs::read(&image).unwrap();
    fs::write(root.0.join("art/images/torso.png"), &original).unwrap();
    let client = start(&root.0).await;
    success(
        call(
            &client,
            "create_project",
            json!({"expected_revision":0,"name":"Curve study","fps":30,"asset_root":"art"}),
        )
        .await,
    );
    let mut commands = vec![
        json!({"command":"addSlot","slot":{"name":"body","bone":0,"attachment":null,"color":[255,255,255,255],"blend":"normal"}}),
        json!({"command":"addAttachment","attachment":{"skin":"default","slot":"body","placeholder":"body"},"value":{
            "type":"region","path":"torso","width":140,"height":148,
            "transform":{"x":0,"y":0,"rotation":0,"scale_x":1,"scale_y":1}}}),
        json!({"command":"setSlotAttachment","slot":0,"attachment":"body"}),
        json!({"command":"addAnimation","name":"curves","duration":0}),
    ];
    for (property, values) in [("rotation", [0., 30., 10.]), ("y", [0., 15., -5.])] {
        for (time, value) in [0., 0.5, 1.].into_iter().zip(values) {
            commands.push(
                json!({"command":"setKey","animation":0,"bone":0,"property":property,
                "key":{"time":time,"value":value,"curve":{"bezier":[0.25,0.75,0.75,0.25]}}}),
            );
        }
    }
    success(
        call(
            &client,
            "apply_commands",
            json!({"expected_revision":1,"commands":commands}),
        )
        .await,
    );
    let authored = doc(&client).await;
    let selected = |property: &str, time: f32| json!({"key":{"type":"bone","bone":0,"property":property,"time":time},"channel":0});
    let selection = json!([selected("rotation", 0.), selected("y", 0.)]);
    success(call(&client,"apply_commands",json!({"expected_revision":2,"commands":[{
        "command":"applyCurvePreset","animation":0,"selection":selection,"preset":"bezier","separate":true
    }]})).await);
    let preset = doc(&client).await;
    success(call(&client,"apply_commands",json!({"expected_revision":3,"commands":[{
        "command":"editCurveHandle","animation":0,"selection":selection,"primary":selected("rotation",0.),
        "handle":0,"point":[0.1,6.0],"match_curves":true,"separate":true
    }]})).await);
    let matched = doc(&client).await;
    let mut expected = authored["document"].clone();
    for index in 0..2 {
        let actual = &matched["document"]["animations"][0]["tracks"][index]["keys"][0]["curve"];
        let handles = actual["bezier"].as_array().unwrap();
        assert!((handles[0].as_f64().unwrap() - 0.2).abs() < 1e-6);
        assert!((handles[1].as_f64().unwrap() - 0.2).abs() < 1e-6);
        let before =
            &preset["document"]["animations"][0]["tracks"][index]["keys"][0]["curve"]["bezier"];
        assert_eq!(handles[2], before[2]);
        assert_eq!(handles[3], before[3]);
        expected["animations"][0]["tracks"][index]["keys"][0]["curve"] = actual.clone();
    }
    assert_eq!(matched["document"], expected);
    let pose = success(
        call(
            &client,
            "evaluate_pose",
            json!({"animation":"curves","time":0.2}),
        )
        .await,
    );
    let local = &pose["locals"][0];
    assert!(
        (local["rotation"].as_f64().unwrap() / 30. - local["y"].as_f64().unwrap() / 15.).abs()
            < 1e-6
    );
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":4,"action":"undo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], preset["document"]);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":5,"action":"redo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], matched["document"]);
    success(call(&client,"apply_commands",json!({"expected_revision":6,"commands":[{
        "command":"applyCurvePreset","animation":0,"selection":[selected("rotation",1.)],"preset":"stepped","separate":true
    }]})).await);
    let terminal = doc(&client).await;
    assert_eq!(
        terminal["document"]["animations"][0]["tracks"][0]["keys"][1]["curve"],
        "stepped"
    );
    assert_eq!(
        terminal["document"]["animations"][0]["tracks"][0]["keys"][2],
        matched["document"]["animations"][0]["tracks"][0]["keys"][2]
    );
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":7,"action":"undo"}),
        )
        .await,
    );
    success(call(&client,"apply_commands",json!({"expected_revision":8,"commands":[{
        "command":"applyCurvePreset","animation":0,"selection":[selected("rotation",1.),selected("y",1.)],"preset":"bezier","separate":true
    }]})).await);
    assert_eq!(doc(&client).await["document"], matched["document"]);
    // A no-op two-terminal preset must retain the existing redo branch.
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":9,"action":"redo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], terminal["document"]);
    success(
        call(
            &client,
            "save_project",
            json!({"expected_revision":10,"directory":"authored"}),
        )
        .await,
    );
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":11,"path":"authored/project.peachy"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], terminal["document"]);
    success(
        call(
            &client,
            "export_project",
            json!({"expected_revision":12,"kind":"bundle","path":".","target":"runtime43"}),
        )
        .await,
    );
    assert_eq!(
        fs::read(root.0.join("art/images/torso.png")).unwrap(),
        original
    );
    assert_eq!(fs::read(image).unwrap(), original);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn stdio_curve_live_separate_off_updates_neighbor_atomically() {
    let root = Sandbox::new();
    let client = start(&root.0).await;
    success(
        call(
            &client,
            "create_project",
            json!({
                "expected_revision":0,"name":"Original linked curve probe","fps":60,"asset_root":"."
            }),
        )
        .await,
    );
    let mut commands = vec![json!({"command":"addAnimation","name":"curves","duration":0})];
    for (time, value, curve) in [
        (0., 0., json!({"bezier":[0.2,0.,0.8,29./30.]})),
        (0.5, 30., json!({"bezier":[1./3.,-1./12.,2./3.,13./12.]})),
        (1., 10., json!({"bezier":[0.2,1./30.,0.8,1.]})),
        (1.5, 40., json!("linear")),
    ] {
        commands.push(
            json!({"command":"setKey","animation":0,"bone":0,"property":"rotation",
            "key":{"time":time,"value":value,"curve":curve}}),
        );
    }
    success(
        call(
            &client,
            "apply_commands",
            json!({"expected_revision":1,"commands":commands}),
        )
        .await,
    );
    let before = doc(&client).await;
    let primary =
        json!({"key":{"type":"bone","bone":0,"property":"rotation","time":0.5},"channel":0});
    let action = json!({"command":"editCurveHandle","animation":0,"selection":[primary.clone()],
        "primary":primary,"handle":0,"point":[0.6666667,28.409304],"match_curves":false,"separate":false});
    success(
        call(
            &client,
            "apply_commands",
            json!({"expected_revision":2,"commands":[action.clone()]}),
        )
        .await,
    );
    let after = doc(&client).await;
    let keys = &after["document"]["animations"][0]["tracks"][0]["keys"];
    let old = &before["document"]["animations"][0]["tracks"][0]["keys"];
    let previous = &keys[0]["curve"]["bezier"];
    assert_eq!(previous[0], old[0]["curve"]["bezier"][0]);
    assert_eq!(previous[1], old[0]["curve"]["bezier"][1]);
    assert_eq!(previous[2], old[0]["curve"]["bezier"][2]);
    assert!((previous[3].as_f64().unwrap() * 30. - 30.95443).abs() < 0.0002);
    let mut expected = before["document"].clone();
    for index in 0..2 {
        expected["animations"][0]["tracks"][0]["keys"][index]["curve"] =
            keys[index]["curve"].clone();
    }
    assert_eq!(after["document"], expected);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":3,"action":"undo"}),
        )
        .await,
    );
    let undone = doc(&client).await;
    assert_eq!(undone["document"], before["document"]);
    let mut extreme = action;
    extreme["point"] = json!([0.5183616, 10.004745]);
    error(
        call(
            &client,
            "apply_commands",
            json!({"expected_revision":4,"commands":[
                {"command":"renameBone","bone":0,"name":"must-rollback"},extreme
            ]}),
        )
        .await,
    );
    assert_eq!(doc(&client).await, undone);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":4,"action":"redo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], after["document"]);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn stdio_draw_order_preserves_raw_keys_full_hidden_order_and_persistence() {
    let root = Sandbox::new();
    let client = start(&root.0).await;
    success(
        call(
            &client,
            "create_project",
            json!({
                "expected_revision":0,"name":"Original order study","fps":60,"asset_root":"."
            }),
        )
        .await,
    );
    let mut commands = vec![json!({"command":"addAnimation","name":"orders","duration":0})];
    for slot in 0..4 {
        commands.push(json!({"command":"addSlot","slot":{
            "name":format!("s{slot}"),"bone":0,"attachment":null,"color":[255,255,255,255],"blend":"normal"}}));
    }
    commands.extend([
        json!({"command":"setDrawOrderKey","animation":0,"key":{"time":0.5,"offsets":[
            {"slot":0,"offset":1},{"slot":1,"offset":-1},{"slot":2,"offset":0},{"slot":3,"offset":0}]}}),
        json!({"command":"setDrawOrderKey","animation":0,"key":{"time":1.5,"offsets":[]}}),
    ]);
    success(
        call(
            &client,
            "apply_commands",
            json!({"expected_revision":1,"commands":commands}),
        )
        .await,
    );
    let raw = doc(&client).await;
    for (time, order) in [
        (0.49, json!([0, 1, 2, 3])),
        (0.5, json!([1, 0, 2, 3])),
        (1.5, json!([0, 1, 2, 3])),
    ] {
        let pose = success(
            call(
                &client,
                "evaluate_pose",
                json!({"animation":"orders","time":time}),
            )
            .await,
        );
        assert_eq!(pose["drawOrder"], order);
        assert_eq!(pose["slotAttachments"], json!([null, null, null, null]));
        assert_eq!(pose["attachments"], json!([]));
        assert_eq!(doc(&client).await, raw);
    }
    success(
        call(
            &client,
            "apply_commands",
            json!({"expected_revision":2,"commands":[{
                "command":"setDrawOrder","animation":0,"time":0.5,"order":[1,2,3,0]
            }]}),
        )
        .await,
    );
    let changed = doc(&client).await;
    assert_eq!(
        changed["document"]["animations"][0]["draw_order"][0]["offsets"],
        json!([{"slot":0,"offset":3}])
    );
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":3,"action":"undo"}),
        )
        .await,
    );
    success(
        call(
            &client,
            "apply_commands",
            json!({"expected_revision":4,"commands":[{
                "command":"setDrawOrder","animation":0,"time":0.5,"order":[1,0,2,3]
            }]}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], raw["document"]);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":5,"action":"redo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], changed["document"]);
    let before_bad = doc(&client).await;
    error(
        call(
            &client,
            "apply_commands",
            json!({"expected_revision":6,"commands":[
                {"command":"renameBone","bone":0,"name":"must-rollback"},
                {"command":"setDrawOrder","animation":0,"time":1,"order":[0,1,1,3]}
            ]}),
        )
        .await,
    );
    assert_eq!(doc(&client).await, before_bad);
    success(
        call(
            &client,
            "apply_commands",
            json!({"expected_revision":6,"commands":[{
                "command":"setDrawOrder","animation":0,"time":2,"order":[0,1,2,3]
            }]}),
        )
        .await,
    );
    let final_document = doc(&client).await["document"].clone();
    assert_eq!(
        final_document["animations"][0]["draw_order"][2],
        json!({"time":2.0,"offsets":[]})
    );
    success(
        call(
            &client,
            "save_project",
            json!({"expected_revision":7,"directory":"authored"}),
        )
        .await,
    );
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":8,"path":"authored/project.peachy"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], final_document);
    success(
        call(
            &client,
            "export_project",
            json!({"expected_revision":9,"kind":"json","path":"orders.json","target":"runtime43"}),
        )
        .await,
    );
    let exported: Value =
        serde_json::from_slice(&fs::read(root.0.join("orders.json")).unwrap()).unwrap();
    let keys = exported["animations"]["orders"]["drawOrder"]
        .as_array()
        .unwrap();
    assert_eq!(keys.len(), 3);
    assert_eq!(keys[2]["time"], 2.0);
    assert!(keys[2].get("offsets").is_none());
    let clipboard = success(
        call(
            &client,
            "copy_animation_keys",
            json!({
                "expected_revision":9,"animation":0,"keys":[
                    {"type":"drawOrder","time":0.5},{"type":"drawOrder","time":2.0}]
            }),
        )
        .await,
    )["clipboard"]
        .clone();
    assert_eq!(clipboard["slots"], json!(["s0", "s1", "s2", "s3"]));
    assert_eq!(clipboard["keys"][0]["time"], 0.0);
    assert_eq!(clipboard["keys"][1]["time"], 1.5);
    success(call(&client,"apply_commands",json!({"expected_revision":9,"commands":[{
        "command":"deleteAnimationKeys","animation":0,"keys":[{"type":"drawOrder","time":0.5}]
    }]})).await);
    success(call(&client,"apply_commands",json!({"expected_revision":10,"commands":[{
        "command":"pasteAnimationKeys","animation":0,"time":0.25,"target":{"type":"drawOrder"},"clipboard":clipboard.clone()
    }]})).await);
    let pasted = doc(&client).await;
    assert_eq!(
        pasted["document"]["animations"][0]["draw_order"][0]["time"],
        0.25
    );
    let mut wrong_layout = clipboard;
    wrong_layout["slots"] = json!(["s3", "s2", "s1", "s0"]);
    error(call(&client,"apply_commands",json!({"expected_revision":11,"commands":[
        {"command":"renameBone","bone":0,"name":"must-rollback"},
        {"command":"pasteAnimationKeys","animation":0,"time":0.25,"target":{"type":"drawOrder"},"clipboard":wrong_layout}
    ]})).await);
    assert_eq!(doc(&client).await, pasted);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":11,"action":"undo"}),
        )
        .await,
    );
    success(call(&client,"apply_commands",json!({"expected_revision":12,"commands":[{
        "command":"applyCurvePreset","animation":0,"selection":[{"key":{"type":"drawOrder","time":1.5},"channel":0}],
        "preset":"bezier","separate":false
    }]})).await);
    success(
        call(
            &client,
            "history",
            json!({"expected_revision":13,"action":"redo"}),
        )
        .await,
    );
    assert_eq!(doc(&client).await["document"], pasted["document"]);
    client.cancel().await.unwrap();
}
