use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn command_line_exports_imports_and_preserves_inputs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("examples/peach-bot/peach-bot.peachy");
    let original = fs::read(&source).unwrap();
    let folder = std::env::temp_dir().join(format!(
        "peachy-cli-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&folder).unwrap();
    let cli = env!("CARGO_BIN_EXE_peachy-cli");
    let output = folder.join("character.json");
    let result = Command::new(cli)
        .arg("export")
        .arg(&source)
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let duplicate = Command::new(cli)
        .arg("export")
        .arg(&source)
        .arg(&output)
        .output()
        .unwrap();
    assert!(!duplicate.status.success());
    let restored = folder.join("restored.peachy");
    assert!(
        Command::new(cli)
            .arg("import")
            .arg(&output)
            .arg(&restored)
            .output()
            .unwrap()
            .status
            .success()
    );
    let before: peachy_document::Document = serde_json::from_slice(&original).unwrap();
    let after: peachy_document::Document =
        serde_json::from_slice(&fs::read(restored).unwrap()).unwrap();
    assert_eq!(before.bones, after.bones);
    assert_eq!(before.skins, after.skins);
    assert_eq!(fs::read(source).unwrap(), original);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn serialized_pose_command_uses_the_shared_domain_transition() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("examples/peach-bot/peach-bot.peachy");
    let folder = std::env::temp_dir().join(format!(
        "peachy-pose-cli-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&folder).unwrap();
    let command = folder.join("commands.json");
    let output = folder.join("edited.peachy");
    let mut commands = serde_json::json!([{"command":"setPose","bone":0,"target":{"mode":"animate","animation":0,"time":0.301},"transform":{"x":0,"y":0,"rotation":15,"scale_x":1,"scale_y":1},"properties":["rotation","shearX","shearY"]}]);
    fs::write(&command, serde_json::to_vec(&commands).unwrap()).unwrap();
    let rejected = Command::new(env!("CARGO_BIN_EXE_peachy-cli"))
        .arg("apply")
        .arg(&source)
        .arg(&command)
        .arg(&output)
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("missing field `shear_x`"));
    assert!(!output.exists());
    commands[0]["transform"]["shear_x"] = serde_json::json!(17);
    commands[0]["transform"]["shear_y"] = serde_json::json!(-11);
    fs::write(&command, serde_json::to_vec(&commands).unwrap()).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_peachy-cli"))
        .arg("apply")
        .arg(&source)
        .arg(&command)
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let edited = peachy_io::load(&output).unwrap();
    let pose = peachy_engine::local_pose(&edited, Some(0), 0.3);
    assert_eq!(pose[0].rotation, 15.0);
    assert_eq!(pose[0].shear_x, 17.0);
    assert_eq!(pose[0].shear_y, -11.0);
    assert_eq!(edited.bones[0].setup.rotation, 0.0);
    assert_eq!(edited.bones[0].setup.shear_x, 0.0);
    assert_eq!(edited.bones[0].setup.shear_y, 0.0);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn serialized_mesh_edits_export_and_evaluate_the_shared_scene() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("examples/peach-bot/peach-bot.peachy");
    let folder = std::env::temp_dir().join(format!(
        "peachy-mesh-cli-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&folder).unwrap();
    let key = serde_json::json!({"skin":"default","slot":"hand.R","placeholder":"hand.R"});
    let commands = serde_json::json!([
        {"command":"convertRegionToMesh","attachment":key},
        {"command":"splitMeshTriangle","attachment":key,"triangle":0},
        {"command":"setMeshVertex","attachment":key,"vertex":4,"position":[16,4]}
    ]);
    let command = folder.join("commands.json");
    fs::write(&command, commands.to_string()).unwrap();
    let edited = folder.join("edited.peachy");
    let export = folder.join("mesh.json");
    let cli = env!("CARGO_BIN_EXE_peachy-cli");
    for args in [
        vec!["apply".into(), source, command, edited.clone()],
        vec!["export".into(), edited.clone(), export.clone()],
    ] {
        let output = Command::new(cli).args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let value: serde_json::Value = serde_json::from_slice(&fs::read(export).unwrap()).unwrap();
    assert_eq!(
        value["skins"][0]["attachments"]["hand.R"]["hand.R"]["vertices"][8].as_f64(),
        Some(16.0)
    );
    let output = Command::new(cli)
        .arg("pose")
        .arg(&edited)
        .args(["wave", "0.3", "mint"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let scene: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let hand = scene["attachments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["attachment"]["slot"] == "hand.R")
        .unwrap();
    assert_eq!(hand["vertices"].as_array().unwrap().len(), 5);
    assert_eq!(hand["triangles"].as_array().unwrap().len(), 12);
    assert_eq!(hand["attachment"]["skin"], "default");
    fs::remove_dir_all(folder).unwrap();
}
