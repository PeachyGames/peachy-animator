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

#[tokio::test]
async fn stdio_discovery_atomic_edit_history_save_and_export() {
    let root = Sandbox::new();
    root.fixture();
    let source = root.0.join("source/peach-bot.peachy");
    let original = fs::read(&source).unwrap();
    let client = start(&root.0).await;
    let tools = client.list_tools(Default::default()).await.unwrap().tools;
    assert_eq!(tools.len(), 10);
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
