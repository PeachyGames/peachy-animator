//! Whole-workspace interactions for grouped animated IK authoring.
use super::test_support::{animated_ik_editor, editor_context, workspace_frame};
use super::*;

fn key(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
    [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        })
        .collect()
}
fn center(editor: &Editor, target: &str) -> Pos2 {
    let [left, top, right, bottom] = editor.capture_targets[target];
    pos2((left + right) / 2., (top + bottom) / 2.)
}
fn pointer(point: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}
fn click(editor: &mut Editor, ctx: &egui::Context, target: &str) {
    let point = center(editor, target);
    workspace_frame(
        editor,
        ctx,
        vec![egui::Event::PointerMoved(point), pointer(point, true)],
    );
    workspace_frame(editor, ctx, vec![pointer(point, false)]);
}
fn text(editor: &mut Editor, ctx: &egui::Context, value: &str) {
    click(editor, ctx, "ikMixValue");
    workspace_frame(editor, ctx, key(egui::Key::A, egui::Modifiers::COMMAND));
    workspace_frame(editor, ctx, vec![egui::Event::Text(value.into())]);
}
fn ready() -> (Editor, egui::Context) {
    let mut editor = animated_ik_editor();
    editor.time = 0.3;
    editor.tree.filters = tree_model::Filters {
        bones: false,
        slots: false,
        attachments: false,
        skins: false,
        animations: false,
        constraints: true,
    };
    let ctx = editor_context();
    workspace_frame(&mut editor, &ctx, vec![]);
    workspace_frame(&mut editor, &ctx, vec![]);
    (editor, ctx)
}

#[test]
fn valid_live_mix_key_uses_current_group_and_one_undo_without_an_intermediate_pose_step() {
    let (mut editor, ctx) = ready();
    let original = editor.session.document().clone();
    text(&mut editor, &ctx, "62.5");
    assert_eq!(editor.current_ik_pose(1).unwrap().mix, 0.625);
    assert!(editor.session.pose_edits().has_ik(0, 1));
    assert_eq!(editor.session.document(), &original);
    assert!(editor.ik_edit.is_some());
    click(&mut editor, &ctx, "ikKey");
    assert!(editor.ik_edit.is_none());
    assert!(editor.session.pose_edits().is_empty());
    let keys = &editor.session.document().animations[0].ik_tracks[0].keys;
    let frame = keys.iter().find(|key| key.time == 0.3).unwrap();
    assert_eq!(frame.mix, 0.625);
    assert!(frame.bend_positive);
    let keyed = editor.session.document().clone();
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.pose_edits().is_empty());
    assert!(!editor.session.can_undo());
    editor.redo();
    assert_eq!(editor.session.document(), &keyed);
    assert_eq!(editor.time, 0.3);
}

#[test]
fn invalid_mix_text_blocks_the_first_key_without_normalizing_or_keying_stale_values() {
    let (mut editor, ctx) = ready();
    editor
        .session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo".into(),
        })
        .unwrap();
    editor.undo();
    workspace_frame(&mut editor, &ctx, vec![]);
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    text(&mut editor, &ctx, "junk");
    assert_eq!(editor.ik_diagnostic()["edit"]["validText"], false);
    click(&mut editor, &ctx, "ikKey");
    assert!(editor.status.contains("finite Mix"), "{}", editor.status);
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(editor.session.can_redo());
    assert!(!editor.session.can_undo());
    assert_eq!(editor.ik_diagnostic()["edit"]["validText"], false);
}

#[test]
fn animate_positive_preserves_a_valid_owned_mix_and_autokey_writes_one_complete_group() {
    let (mut editor, ctx) = ready();
    let mut document = editor.session.document().clone();
    document.animations[0].ik_tracks[0].constraint = 0;
    editor.session = Session::new(document).unwrap();
    editor.select(TreeItem::Constraint(0));
    workspace_frame(&mut editor, &ctx, vec![]);
    let original = editor.session.document().clone();
    text(&mut editor, &ctx, "60");
    click(&mut editor, &ctx, "ikPositive");
    let pose = editor.current_ik_pose(0).unwrap();
    assert_eq!(pose.mix, f64::from(0.6_f32));
    assert!(!pose.bend_positive);
    assert_eq!(editor.session.document(), &original);
    editor.undo();
    assert!(editor.session.pose_edits().is_empty());
    assert!(!editor.session.can_undo());
    click(&mut editor, &ctx, "autoKey");
    text(&mut editor, &ctx, "70");
    workspace_frame(
        &mut editor,
        &ctx,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    let frame = editor.session.document().animations[0].ik_tracks[0]
        .keys
        .iter()
        .find(|key| key.time == 0.3)
        .unwrap();
    assert_eq!(frame.mix, 0.7);
    assert!(frame.bend_positive);
    assert!(editor.session.pose_edits().is_empty());
    assert_eq!(editor.ik_group_state(0).unwrap(), KeyState::Keyed);
}

#[test]
fn fractional_existing_group_indicator_and_navigation_use_actual_key_identity() {
    let (mut editor, ctx) = ready();
    let mut document = editor.session.document().clone();
    let later = f32::from_bits(0.25_f32.to_bits() + 1);
    document.animations[0].ik_tracks[0].keys = vec![
        IkKeyframe {
            time: 0.25,
            mix: 0.4,
            bend_positive: true,
            curve: IkCurve::Stepped,
        },
        IkKeyframe {
            time: later,
            mix: 0.7,
            bend_positive: false,
            curve: IkCurve::Linear,
        },
    ];
    document.animations[0].tracks = vec![Track {
        bone: 2,
        property: Property::Rotation,
        keys: vec![Keyframe {
            time: 0.25,
            value: 0.,
            curve: Curve::Linear,
        }],
    }];
    editor.session = Session::new(document).unwrap();
    editor.set_time(0.25);
    workspace_frame(&mut editor, &ctx, vec![]);
    assert_eq!(editor.ik_group_state(1).unwrap(), KeyState::Keyed);
    assert_eq!(editor.key_state(2, &[Property::Rotation]), KeyState::Keyed);
    editor.jump_key(PlayDirection::Forward);
    assert_eq!(editor.time, later);
    assert_eq!(editor.ik_group_state(1).unwrap(), KeyState::Keyed);
    editor.jump_key(PlayDirection::Backward);
    assert_eq!(editor.time, 0.25);
}

#[test]
fn global_key_combines_bone_pending_and_live_ik_preview_in_one_atomic_history_step() {
    let (mut editor, ctx) = ready();
    let original = editor.session.document().clone();
    editor.select(TreeItem::Bone(16));
    let mut pose = editor.selected_transform().unwrap();
    pose.x += 20.;
    editor.edit_transform(pose, &[Property::X]);
    let bone_pending = editor.session.pose_edits().clone();
    editor.select(TreeItem::Constraint(1));
    workspace_frame(&mut editor, &ctx, vec![]);
    text(&mut editor, &ctx, "60");
    assert!(editor.session.pose_edits().has_bone(0, 16));
    assert!(editor.session.pose_edits().has_ik(0, 1));
    click(&mut editor, &ctx, "keyEdited");
    assert!(editor.session.pose_edits().is_empty());
    let clip = &editor.session.document().animations[0];
    assert!(
        clip.tracks
            .iter()
            .any(|track| track.bone == 16 && track.property == Property::X)
    );
    assert!(
        clip.ik_tracks[0]
            .keys
            .iter()
            .any(|key| key.time == 0.3 && key.mix == 0.6)
    );
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &bone_pending);
    editor.undo();
    assert!(editor.session.pose_edits().is_empty());
    assert!(!editor.session.can_undo());
}

#[test]
fn key_during_a_held_mix_slider_seals_the_preview_and_blocks_stale_drag_responses() {
    let (mut editor, ctx) = ready();
    let original = editor.session.document().clone();
    let point = center(&editor, "ikMixSlider");
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(point), pointer(point, true)],
    );
    assert!(editor.ik_edit.is_some());
    let preview = editor.current_ik_pose(1).unwrap();
    workspace_frame(&mut editor, &ctx, key(egui::Key::K, egui::Modifiers::NONE));
    assert!(editor.ik_edit.is_none());
    assert!(editor.session.pose_edits().is_empty());
    let keyed = editor.session.document().clone();
    assert_eq!(editor.current_ik_pose(1).unwrap(), preview);
    let further = point + vec2(20., 0.);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(further)]);
    workspace_frame(&mut editor, &ctx, vec![pointer(further, false)]);
    assert_eq!(editor.session.document(), &keyed);
    assert!(editor.ik_edit.is_none());
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.pose_edits().is_empty());
    assert!(!editor.session.can_undo());
}

#[test]
fn shared_explicit_key_parses_bone_text_before_keying_and_blocks_invalid_bone_text() {
    let mut editor = super::test_support::editor_fixture();
    editor.time = 0.4;
    let original = editor.session.document().clone();
    let ctx = editor_context();
    workspace_frame(&mut editor, &ctx, vec![]);
    click(&mut editor, &ctx, "value:Rotation");
    workspace_frame(
        &mut editor,
        &ctx,
        key(egui::Key::A, egui::Modifiers::COMMAND),
    );
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("37.5".into())]);
    click(&mut editor, &ctx, "key:Rotation");
    assert!(editor.drag.is_none());
    let keyed = editor.session.document().animations[0]
        .tracks
        .iter()
        .find(|track| track.bone == 8 && track.property == Property::Rotation)
        .unwrap()
        .keys
        .iter()
        .find(|key| key.time == 0.4)
        .unwrap();
    assert_eq!(keyed.value, 37.5);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.pose_edits().is_empty());
    assert!(!editor.session.can_undo());
    workspace_frame(&mut editor, &ctx, vec![]);
    click(&mut editor, &ctx, "value:Rotation");
    workspace_frame(
        &mut editor,
        &ctx,
        key(egui::Key::A, egui::Modifiers::COMMAND),
    );
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("junk".into())]);
    click(&mut editor, &ctx, "key:Rotation");
    assert!(
        editor.status.contains("valid transform"),
        "{}",
        editor.status
    );
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.pose_edits().is_empty());
    assert!(editor.session.can_redo());
}

#[test]
fn weights_bind_and_direct_capture_full_animated_pending_ik_once() {
    let (mut editor, ctx) = ready();
    text(&mut editor, &ctx, "65");
    workspace_frame(
        &mut editor,
        &ctx,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    let pending = editor.session.pose_edits().clone();
    let mesh = AttachmentRef {
        skin: "peach".into(),
        slot: "head".into(),
        placeholder: "head".into(),
    };
    editor
        .session
        .dispatch(Command::ConvertRegionToMesh {
            attachment: mesh.clone(),
        })
        .unwrap();
    let vertices = |editor: &Editor| {
        editor
            .scene_for_view()
            .unwrap()
            .attachments
            .into_iter()
            .find(|draw| draw.attachment == mesh)
            .unwrap()
            .vertices
    };
    let before = vertices(&editor);
    editor.select(TreeItem::Attachment(mesh.clone()));
    editor.set_tool(Tool::Weights);
    editor.begin_weights_bind();
    editor.select(TreeItem::Bone(2));
    assert!(editor.error.is_none(), "{:?}", editor.error);
    let after = vertices(&editor);
    assert!(
        before
            .iter()
            .flatten()
            .zip(after.iter().flatten())
            .all(|(a, b)| (a - b).abs() < 0.001)
    );
    assert_eq!(editor.session.pose_edits(), &pending);
    editor.begin_weights_bind();
    editor.select(TreeItem::Bone(0));
    editor.weights.vertex = Some(0);
    editor.weights.bone = Some(0);
    workspace_frame(&mut editor, &ctx, vec![]);
    let applied = editor
        .session
        .evaluated_pose(editor.weights_edit_target())
        .unwrap();
    click(&mut editor, &ctx, "weightsWeightValue");
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("30".into())]);
    let direct = editor.weight_direct_diagnostic();
    assert_eq!(
        direct["active"]["pose"]["locals"][2],
        serde_json::to_value(applied.applied_locals()[2]).unwrap()
    );
    workspace_frame(
        &mut editor,
        &ctx,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.time, 0.3);
    assert!(
        vertices(&editor)
            .iter()
            .flatten()
            .all(|value| value.is_finite())
    );
}

#[test]
fn keying_a_different_owner_retains_the_valid_live_authored_preview() {
    let (mut editor, ctx) = ready();
    let target = editor.weights_edit_target();
    let mut bone = editor.session.local_pose(Some(0), editor.time)[16];
    bone.x += 10.;
    editor
        .session
        .edit_pose(16, target, bone, &[Property::X], false)
        .unwrap();
    let before_mix = editor.session.pose_edits().clone();
    let original = editor.session.document().clone();
    text(&mut editor, &ctx, "60");
    editor.key_edited(Some(16));
    workspace_frame(&mut editor, &ctx, vec![]);
    assert_eq!(editor.current_ik_pose(1).unwrap().mix, f64::from(0.6_f32));
    assert!(editor.session.pose_edits().has_ik(0, 1));
    assert!(!editor.session.pose_edits().has_bone(0, 16));
    assert!(
        !editor.session.document().animations[0].ik_tracks[0]
            .keys
            .iter()
            .any(|key| key.time == 0.3)
    );
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &before_mix);

    editor
        .session
        .edit_ik_pose(
            1,
            target,
            &[peachy_engine::IkPoseEdit::Mix { value: 0.6 }],
            false,
        )
        .unwrap();
    editor.select(TreeItem::Bone(16));
    workspace_frame(&mut editor, &ctx, vec![]);
    click(&mut editor, &ctx, "value:X");
    workspace_frame(
        &mut editor,
        &ctx,
        key(egui::Key::A, egui::Modifiers::COMMAND),
    );
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("150".into())]);
    click(&mut editor, &ctx, "keyConstraint:1");
    assert!(editor.drag.is_none());
    assert!(editor.session.pose_edits().has_bone(0, 16));
    assert!(!editor.session.pose_edits().has_ik(0, 1));
    assert_eq!(editor.session.local_pose(Some(0), editor.time)[16].x, 150.);
    assert!(
        editor.session.document().animations[0].ik_tracks[0]
            .keys
            .iter()
            .any(|key| key.time == 0.3 && key.mix == 0.6)
    );
}

#[test]
fn failed_mix_autokey_blocks_positive_without_keying_the_last_valid_value() {
    let (mut editor, ctx) = ready();
    let mut document = editor.session.document().clone();
    document.animations[0].ik_tracks[0].constraint = 0;
    editor.session = Session::new(document).unwrap();
    editor.select(TreeItem::Constraint(0));
    editor.set_time(0.25);
    editor.auto_key = true;
    workspace_frame(&mut editor, &ctx, vec![]);
    let original = editor.session.document().clone();
    text(&mut editor, &ctx, "60");
    assert!(editor.ik_diagnostic()["edit"]["error"].is_string());
    assert_eq!(editor.session.document(), &original);
    click(&mut editor, &ctx, "ikPositive");
    assert!(editor.status.starts_with("Cannot change Positive:"));
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.pose_edits().is_empty());
    assert!(!editor.session.can_undo());
}

#[test]
fn passive_animated_overrange_mix_focus_blur_preserves_bits_and_redo() {
    for mix in [-0.25, 1.234567] {
        let (mut editor, ctx) = ready();
        let mut document = editor.session.document().clone();
        for key in &mut document.animations[0].ik_tracks[0].keys {
            key.mix = mix;
        }
        editor.session = Session::new(document).unwrap();
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "redo".into(),
            })
            .unwrap();
        editor.undo();
        workspace_frame(&mut editor, &ctx, vec![]);
        let original = editor.session.document().clone();
        click(&mut editor, &ctx, "ikMixValue");
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::Tab, egui::Modifiers::NONE),
        );
        workspace_frame(&mut editor, &ctx, vec![]);
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.current_ik_pose(1).unwrap().mix, f64::from(mix));
        assert!(editor.session.pose_edits().is_empty());
        assert!(editor.session.can_redo());
        assert!(!editor.session.can_undo());
    }
}

#[test]
fn owned_mix_save_cancels_live_preview_and_excludes_existing_pending_ik_from_file() {
    let (mut editor, ctx) = ready();
    text(&mut editor, &ctx, "60");
    workspace_frame(
        &mut editor,
        &ctx,
        key(egui::Key::Enter, egui::Modifiers::NONE),
    );
    let pending = editor.session.pose_edits().clone();
    let original = editor.session.document().clone();
    let folder = std::env::temp_dir().join(format!(
        "peachy-ik-animation-save-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&folder).unwrap();
    editor.project_path = Some(folder.join("rig.peachy"));
    editor.asset_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
    text(&mut editor, &ctx, "70");
    assert_ne!(editor.session.pose_edits(), &pending);
    workspace_frame(
        &mut editor,
        &ctx,
        key(egui::Key::S, egui::Modifiers::COMMAND),
    );
    assert!(editor.ik_edit.is_none());
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.session.document(), &original);
    assert_eq!(files::load(&folder.join("rig.peachy")).unwrap(), original);
    assert!(!editor.session.is_dirty());
    assert!(editor.status.contains("unkeyed pose is not saved"));
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn a_popup_opened_after_routing_cancels_a_queued_key_before_transaction_settlement() {
    let (mut editor, ctx) = ready();
    let original = editor.session.document().clone();
    text(&mut editor, &ctx, "60");
    editor.key_ik(1);
    assert!(editor.pose_key_request.is_some());
    let mut output = ctx.run_ui(
        egui::RawInput {
            focused: true,
            ..Default::default()
        },
        |ui| {
            egui::Popup::open_id(ui.ctx(), egui::Id::new("same-frame-key-menu"));
            editor.finish_idle_transactions(ui.ctx());
        },
    );
    output.textures_delta.clear();
    assert!(editor.pose_key_request.is_none());
    assert!(editor.ik_edit.is_none());
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.pose_edits().is_empty());
    assert!(!editor.session.can_undo());
}
