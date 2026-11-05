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
    fs::write(&command,serde_json::to_vec(&serde_json::json!([{"command":"setPose","bone":0,"target":{"mode":"animate","animation":0,"time":0.301},"transform":{"x":0,"y":0,"rotation":15,"scale_x":1,"scale_y":1},"properties":["rotation"]}])).unwrap()).unwrap();
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
    assert_eq!(edited.bones[0].setup.rotation, 0.0);
    fs::remove_dir_all(folder).unwrap();
}
