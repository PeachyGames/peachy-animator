use peachy_document::Document;
use peachy_engine::{Command, apply_command};

#[test]
fn renamed_bone_and_slot_references_survive_spine_json_roundtrip() {
    let before: Document =
        serde_json::from_str(include_str!("../../../examples/peach-bot/mesh-bot.peachy")).unwrap();
    let slot = before.slots.iter().position(|s| s.name == "head").unwrap();
    let bone = before.slots[slot].bone;
    let commands: Vec<Command> = serde_json::from_value(serde_json::json!([
        {"command":"renameBone","bone":bone,"name":"face-bone"},
        {"command":"renameSlot","slot":slot,"name":"face-slot"}
    ]))
    .unwrap();
    let after = apply_command(&before, Command::Batch { commands }).unwrap();
    let exported = peachy_spine::export_json(&after).unwrap();
    assert_eq!(exported["slots"][slot]["name"], "face-slot");
    assert_eq!(exported["slots"][slot]["bone"], "face-bone");
    for skin in exported["skins"].as_array().unwrap() {
        assert!(skin["attachments"].get("head").is_none());
    }
    let restored = peachy_spine::import_json(&exported, &after.name).unwrap();
    assert_eq!(restored.bones, after.bones);
    assert_eq!(restored.slots, after.slots);
    assert_eq!(restored.skins, after.skins);
    assert_eq!(peachy_spine::export_json(&restored).unwrap(), exported);
}
