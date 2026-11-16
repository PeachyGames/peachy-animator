//! Target rejection must happen before any output is published or image decoded.
use peachy_io::{
    ExportTarget, export_bundle, export_bundle_for, export_json, export_json_for, load,
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn export_targets_preserve_names_and_reject_editor_outputs_before_writes() {
    let assets = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
    let source = assets.join("slot-bot.peachy");
    let original = fs::read(&source).unwrap();
    let doc = load(&source).unwrap();
    let before = doc.clone();
    let folder = std::env::temp_dir().join(format!(
        "peachy-export-targets-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&folder).unwrap();
    let output = folder.join("character.json");
    assert!(
        export_json(&output, &doc)
            .unwrap_err()
            .contains("hand.L fist")
    );
    assert!(!output.exists());
    // Missing images must not mask the earlier Editor identity restriction.
    assert!(
        export_bundle(&folder, &doc, &folder)
            .unwrap_err()
            .contains("hand.L fist")
    );
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 0);
    export_json_for(&output, &doc, ExportTarget::Runtime43).unwrap();
    let bytes = fs::read(&output).unwrap();
    let value = serde_json::from_slice(&bytes).unwrap();
    let restored = peachy_spine::import_json(&value, &doc.name).unwrap();
    assert_eq!(restored.slots, doc.slots);
    assert_eq!(restored.skins, doc.skins);
    for clip in &doc.animations {
        // JSON animation objects are name-ordered; curve coordinate conversion
        // is covered separately. Identity references must remain exact.
        let imported = restored
            .animations
            .iter()
            .find(|a| a.name == clip.name)
            .unwrap();
        assert_eq!(imported.attachment_tracks, clip.attachment_tracks);
        assert_eq!(imported.deform_tracks, clip.deform_tracks);
        assert_eq!(imported.sequence_tracks, clip.sequence_tracks);
    }
    assert!(export_json_for(&output, &doc, ExportTarget::Runtime43).is_err());
    assert_eq!(fs::read(&output).unwrap(), bytes);
    let bundle = export_bundle_for(&folder, &doc, &assets, ExportTarget::Runtime43).unwrap();
    let bundled: serde_json::Value =
        serde_json::from_slice(&fs::read(bundle.join("character.json")).unwrap()).unwrap();
    assert_eq!(value, bundled);
    assert!(bundle.join("character.atlas").is_file());
    assert!(bundle.join("character.png").is_file());
    assert_eq!(doc, before);
    assert_eq!(fs::read(source).unwrap(), original);
    fs::remove_dir_all(folder).unwrap();
}
