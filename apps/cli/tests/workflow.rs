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
fn finite_extreme_curve_intermediates_roundtrip_and_pose_without_null_output() {
    use serde_json::{Value, json};
    let folder = std::env::temp_dir().join(format!(
        "peachy-wide-curve-cli-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&folder).unwrap();
    let cli = env!("CARGO_BIN_EXE_peachy-cli");
    for (name, timelines) in [
        (
            "scalar-shear",
            json!({"shearx":[{"time":0,"value":-2e38_f32,"curve":[0.2,-1e38_f32,0.8,1e38_f32]},{"time":1,"value":2e38_f32}]}),
        ),
        (
            "paired-shear",
            json!({"shear":[{"time":0,"x":-2e38_f32,"y":2e38_f32,"curve":[0.2,-1e38_f32,0.8,1e38_f32,0.2,1e38_f32,0.8,-1e38_f32]},{"time":1,"x":2e38_f32,"y":-2e38_f32}]}),
        ),
        (
            "translate",
            json!({"translatex":[{"time":0,"value":-2e38_f32,"curve":[0.2,-1e38_f32,0.8,1e38_f32]},{"time":1,"value":2e38_f32}]}),
        ),
    ] {
        let source = folder.join(format!("{name}.json"));
        let native = folder.join(format!("{name}.peachy"));
        let exported = folder.join(format!("{name}-out.json"));
        let reopened = folder.join(format!("{name}-reopened.peachy"));
        let json = json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}],"slots":[],"skins":[],"animations":{"range":{"bones":{"root":timelines}}}});
        let original = serde_json::to_vec_pretty(&json).unwrap();
        fs::write(&source, &original).unwrap();
        let inspect = Command::new(cli)
            .args(["inspect".as_ref(), source.as_os_str()])
            .output()
            .unwrap();
        assert!(inspect.status.success());
        assert_eq!(
            serde_json::from_slice::<Value>(&inspect.stdout).unwrap()["supported"],
            true
        );
        for args in [
            vec!["import".as_ref(), source.as_os_str(), native.as_os_str()],
            vec!["export".as_ref(), native.as_os_str(), exported.as_os_str()],
            vec![
                "import".as_ref(),
                exported.as_os_str(),
                reopened.as_os_str(),
            ],
        ] {
            let result = Command::new(cli).args(args).output().unwrap();
            assert!(
                result.status.success(),
                "{name}: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        let pose = Command::new(cli)
            .args([
                "pose".as_ref(),
                native.as_os_str(),
                "range".as_ref(),
                "0.5".as_ref(),
            ])
            .output()
            .unwrap();
        assert!(pose.status.success());
        let posed: Value = serde_json::from_slice(&pose.stdout).unwrap();
        assert!(
            posed["bones"][0]["world"]
                .as_object()
                .unwrap()
                .values()
                .all(|v| v.as_f64().is_some_and(f64::is_finite)),
            "{name}: {}",
            String::from_utf8_lossy(&pose.stdout)
        );
        assert!(!fs::read_to_string(&exported).unwrap().contains("null"));
        assert_eq!(
            peachy_io::load(&native).unwrap().animations,
            peachy_io::load(&reopened).unwrap().animations
        );
        assert_eq!(fs::read(&source).unwrap(), original);
    }
    let invalid_source = folder.join("unrepresentable-normalization.json");
    let unpublished_native = folder.join("unpublished.peachy");
    let invalid = json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"}],"slots":[],"skins":[],"animations":{"range":{"bones":{"root":{"shearx":[{"time":0,"value":0,"curve":[0.2,f32::MAX,0.8,f32::MAX]},{"time":1,"value":1e-7_f32}]}}}}});
    let original = serde_json::to_vec_pretty(&invalid).unwrap();
    fs::write(&invalid_source, &original).unwrap();
    let rejected = Command::new(cli)
        .args([
            "import".as_ref(),
            invalid_source.as_os_str(),
            unpublished_native.as_os_str(),
        ])
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("Bezier normalization exceeds"));
    assert!(!unpublished_native.exists());
    assert_eq!(fs::read(&invalid_source).unwrap(), original);

    let mut invalid = peachy_io::load(&folder.join("scalar-shear.peachy")).unwrap();
    invalid.animations[0].tracks[0].keys[0].value = 0.0;
    invalid.animations[0].tracks[0].keys[1].value = 2.0;
    invalid.animations[0].tracks[0].keys[0].curve =
        peachy_document::Curve::Bezier([0.2, f32::MAX, 0.8, f32::MAX]);
    assert!(invalid.validate().is_ok());
    let native = folder.join("unrepresentable-export.peachy");
    let unpublished_json = folder.join("unpublished.json");
    let original = serde_json::to_vec_pretty(&invalid).unwrap();
    fs::write(&native, &original).unwrap();
    let rejected = Command::new(cli)
        .args([
            "export".as_ref(),
            native.as_os_str(),
            unpublished_json.as_os_str(),
        ])
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("Bezier export handle exceeds"));
    assert!(!unpublished_json.exists());
    assert_eq!(fs::read(&native).unwrap(), original);
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
