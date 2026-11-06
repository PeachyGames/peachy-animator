use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
use super::*;

fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..3 {
        workspace_frame(editor, ctx, vec![]);
    }
}
fn point(editor: &Editor, name: &str, x: f32, y: f32) -> Pos2 {
    let [left, top, right, bottom] = editor.capture_targets[name];
    pos2(left + (right - left) * x, top + (bottom - top) * y)
}
fn pointer(editor: &mut Editor, ctx: &egui::Context, position: Pos2, pressed: bool) {
    workspace_frame(
        editor,
        ctx,
        vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
}
fn click(editor: &mut Editor, ctx: &egui::Context, name: &str) {
    let p = point(editor, name, 0.5, 0.5);
    for pressed in [true, false] {
        pointer(editor, ctx, p, pressed);
    }
    settle(editor, ctx);
}
fn key(editor: &mut Editor, ctx: &egui::Context, key: egui::Key, modifiers: egui::Modifiers) {
    for pressed in [true, false] {
        workspace_frame(
            editor,
            ctx,
            vec![egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed,
                repeat: false,
                modifiers,
            }],
        );
    }
}
fn fixture(color: [u8; 4]) -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    let mut doc = editor.session.document().clone();
    doc.slots[0].color = color;
    editor.session = Session::new(doc).unwrap();
    editor.setup_mode = true;
    editor.select(TreeItem::Slot(0));
    editor.open_slot_color_view();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    (editor, ctx)
}
fn bytes(editor: &Editor, slot: usize) -> [u8; 4] {
    editor.session.document().slots[slot].color
}
fn enter_hex(editor: &mut Editor, ctx: &egui::Context, text: &str, commit: egui::Key) {
    click(editor, ctx, "slotColorHex");
    key(editor, ctx, egui::Key::A, egui::Modifiers::COMMAND);
    workspace_frame(editor, ctx, vec![egui::Event::Text(text.into())]);
    key(editor, ctx, commit, egui::Modifiers::NONE);
    settle(editor, ctx);
}
fn begin_picker(editor: &mut Editor, ctx: &egui::Context) -> Pos2 {
    let p = point(editor, "slotColorSV", 0.75, 0.25);
    pointer(editor, ctx, p, true);
    workspace_frame(
        editor,
        ctx,
        vec![egui::Event::PointerMoved(p + vec2(-10., 5.))],
    );
    assert!(editor.slot_color_edit_active());
    p
}
fn pending_redo(editor: &mut Editor) {
    editor.setup_mode = false;
    editor.selection = Some(TreeItem::Bone(8));
    let mut pose = editor.selected_transform().unwrap();
    pose.rotation += 13.;
    editor.edit_transform(pose, &[Property::Rotation]);
    editor
        .session
        .dispatch(Command::SetBoneLength {
            bone: 0,
            length: 11.,
        })
        .unwrap();
    editor.session.undo();
    editor.setup_mode = true;
    editor.selection = Some(TreeItem::Slot(0));
    assert!(editor.session.can_redo());
    assert!(!editor.session.pose_edits().is_empty());
}

#[test]
fn picker_drag_previews_exact_bytes_then_commits_once_and_undo_redo_restore_them() {
    let (mut editor, ctx) = fixture([201, 43, 77, 189]);
    let original = editor.session.document().clone();
    let p = begin_picker(&mut editor, &ctx);
    let preview = bytes(&editor, 0);
    assert_ne!(preview, [201, 43, 77, 189]);
    assert!(!editor.session.can_undo());
    let end = point(&editor, "slotColorSV", 0.3, 0.7);
    pointer(&mut editor, &ctx, end, false);
    assert!(!editor.slot_color_edit_active());
    let edited = editor.session.document().clone();
    assert_ne!(edited, original);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
    editor.redo();
    assert_eq!(editor.session.document(), &edited);
    assert_ne!(p, end);
    // Hue and alpha are real independent pointer controls.
    settle(&mut editor, &ctx);
    let alpha = point(&editor, "slotColorAlpha", 0.25, 0.5);
    for pressed in [true, false] {
        pointer(&mut editor, &ctx, alpha, pressed);
    }
    assert!((i16::from(bytes(&editor, 0)[3]) - 64).abs() <= 1);
    let hue = point(&editor, "slotColorHue", 0.6, 0.5);
    for pressed in [true, false] {
        pointer(&mut editor, &ctx, hue, pressed);
    }
    assert_ne!(bytes(&editor, 0), edited.slots[0].color);
}

#[test]
fn hue_selection_on_gray_is_cached_without_rewriting_rgba_or_creating_history() {
    let (mut editor, ctx) = fixture([255; 4]);
    let hue = point(&editor, "slotColorHue", 0.6, 0.5);
    for pressed in [true, false] {
        pointer(&mut editor, &ctx, hue, pressed);
    }
    settle(&mut editor, &ctx);
    assert_eq!(bytes(&editor, 0), [255; 4]);
    assert!(!editor.session.can_undo());
    assert!((editor.slot_color.hsv.h - 0.6).abs() < 0.001);
    let sv = point(&editor, "slotColorSV", 0.8, 0.2);
    for pressed in [true, false] {
        pointer(&mut editor, &ctx, sv, pressed);
    }
    let color = bytes(&editor, 0);
    assert!(color[2] > color[0]);
    assert!(color[1] > color[0]);
    editor.undo();
    assert_eq!(bytes(&editor, 0), [255; 4]);
    assert!(!editor.session.can_undo());
}

#[test]
fn hex_enter_and_tab_apply_real_rgba_with_hidden_rgb_at_zero_alpha() {
    for commit in [egui::Key::Enter, egui::Key::Tab] {
        let (mut editor, ctx) = fixture([255; 4]);
        enter_hex(&mut editor, &ctx, "#D13B7500", commit);
        assert_eq!(bytes(&editor, 0), [209, 59, 117, 0]);
        assert!(!editor.slot_color_edit_active());
        assert_eq!(editor.slot_color_diagnostic()["hex"], "#D13B7500");
        editor.undo();
        assert_eq!(bytes(&editor, 0), [255; 4]);
        editor.redo();
        assert_eq!(bytes(&editor, 0), [209, 59, 117, 0]);
    }
}

#[test]
fn focus_roundtrip_invalid_hex_and_cancel_never_rewrite_source_bytes_or_redo() {
    let original = [209, 59, 117, 0];
    let (mut editor, ctx) = fixture(original);
    pending_redo(&mut editor);
    settle(&mut editor, &ctx);
    let doc = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    for _ in 0..4 {
        settle(&mut editor, &ctx);
    }
    assert_eq!(bytes(&editor, 0), original);
    click(&mut editor, &ctx, "slotColorHex");
    key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    settle(&mut editor, &ctx);
    assert_eq!(editor.session.document(), &doc);
    assert!(editor.session.can_redo());
    assert_eq!(editor.session.pose_edits(), &pending);
    enter_hex(&mut editor, &ctx, "#GGGGGGGG", egui::Key::Enter);
    assert!(editor.slot_color.error.is_some());
    assert_eq!(editor.session.document(), &doc);
    assert!(editor.session.can_redo());
    key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    settle(&mut editor, &ctx);
    assert_eq!(editor.session.document(), &doc);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(editor.session.can_redo());
    // Valid text alone is not a command; clicking away cancels it.
    click(&mut editor, &ctx, "slotColorHex");
    key(&mut editor, &ctx, egui::Key::A, egui::Modifiers::COMMAND);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::Text("#01020304".into())],
    );
    let outside = point(&editor, "viewport", 0.95, 0.95);
    for pressed in [true, false] {
        pointer(&mut editor, &ctx, outside, pressed);
    }
    assert_eq!(editor.session.document(), &doc);
    assert!(editor.session.can_redo());
}

#[test]
fn typed_slot_and_attachment_own_slot_are_the_only_targets_and_animate_is_read_only() {
    let (mut editor, ctx) = fixture([255; 4]);
    let key = editor
        .session
        .document()
        .resolved_attachment(&editor.skin, 1)
        .unwrap()
        .0;
    editor.select(TreeItem::Attachment(key.clone()));
    settle(&mut editor, &ctx);
    assert_eq!(editor.slot_color_context().unwrap().slot, 1);
    enter_hex(&mut editor, &ctx, "#12345678", egui::Key::Enter);
    assert_eq!(bytes(&editor, 1), [18, 52, 86, 120]);
    assert_eq!(bytes(&editor, 0), [255; 4]);
    let doc = editor.session.document().clone();
    editor.select(TreeItem::Bone(8));
    settle(&mut editor, &ctx);
    assert!(editor.slot_color_context().is_none());
    assert!(!editor.capture_targets.contains_key("slotColorHex"));
    editor.select(TreeItem::Slot(0));
    editor.set_setup_mode(false);
    settle(&mut editor, &ctx);
    let p = point(&editor, "slotColorSV", 0.7, 0.3);
    for pressed in [true, false] {
        pointer(&mut editor, &ctx, p, pressed);
    }
    assert_eq!(editor.session.document(), &doc);
    assert!(!editor.slot_color_edit_active());
    assert!(!editor.slot_color_diagnostic()["setup"].as_bool().unwrap());
}

#[test]
fn pending_pose_survives_color_commit_undo_redo_and_cancel() {
    let (mut editor, ctx) = fixture([255; 4]);
    pending_redo(&mut editor);
    settle(&mut editor, &ctx);
    let pending = editor.session.pose_edits().clone();
    let doc = editor.session.document().clone();
    enter_hex(&mut editor, &ctx, "#11223344", egui::Key::Enter);
    assert_eq!(bytes(&editor, 0), [17, 34, 51, 68]);
    assert_eq!(editor.session.pose_edits(), &pending);
    editor.undo();
    assert_eq!(editor.session.document(), &doc);
    assert_eq!(editor.session.pose_edits(), &pending);
    editor.redo();
    assert_eq!(bytes(&editor, 0), [17, 34, 51, 68]);
    assert_eq!(editor.session.pose_edits(), &pending);
    settle(&mut editor, &ctx);
    let p = begin_picker(&mut editor, &ctx);
    key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert_eq!(bytes(&editor, 0), [17, 34, 51, 68]);
    assert_eq!(editor.session.pose_edits(), &pending);
    pointer(&mut editor, &ctx, p, false);
}

#[test]
fn history_context_tool_hide_modal_and_focus_actions_cancel_and_block_the_held_pointer() {
    for action in [
        "undo", "redo", "slot", "skin", "mode", "tool", "close", "f9", "modal", "focus",
    ] {
        let (mut editor, ctx) = fixture([133, 77, 24, 211]);
        let doc = editor.session.document().clone();
        let p = begin_picker(&mut editor, &ctx);
        match action {
            "undo" => editor.undo(),
            "redo" => editor.redo(),
            "slot" => editor.select(TreeItem::Slot(1)),
            "skin" => editor.activate_skin("mint".into()),
            "mode" => editor.set_setup_mode(false),
            "tool" => editor.set_tool(Tool::Move),
            "close" => {
                editor.cancel_gesture();
                editor.view_tabs.open.clear();
            }
            "f9" => key(&mut editor, &ctx, egui::Key::F9, egui::Modifiers::NONE),
            "modal" => {
                editor.error = Some("test modal".into());
                workspace_frame(&mut editor, &ctx, vec![]);
            }
            "focus" => {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        focused: false,
                        screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
                        ..Default::default()
                    },
                    |ui| editor.workspace_ui(ui),
                );
                output.textures_delta.clear();
            }
            _ => unreachable!(),
        }
        assert!(!editor.slot_color_edit_active(), "{action}");
        assert_eq!(editor.session.document(), &doc, "{action}");
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(p + vec2(20., 20.))],
        );
        assert!(!editor.slot_color_edit_active(), "{action}");
        assert_eq!(editor.session.document(), &doc, "{action}");
        pointer(&mut editor, &ctx, p, false);
        settle(&mut editor, &ctx);
        assert!(!editor.slot_color.blocked, "{action}");
    }
}

#[test]
fn owned_text_keeps_undo_and_save_cancels_uncommitted_color_before_writing() {
    let (mut editor, ctx) = fixture([77, 88, 99, 0]);
    let doc = editor.session.document().clone();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "peachy-slot-color-save-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("rig.peachy");
    editor.project_path = Some(path.clone());
    editor.asset_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
    click(&mut editor, &ctx, "slotColorHex");
    key(&mut editor, &ctx, egui::Key::A, egui::Modifiers::COMMAND);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::Text("#11223344".into())],
    );
    key(&mut editor, &ctx, egui::Key::Z, egui::Modifiers::COMMAND);
    assert_eq!(editor.session.document(), &doc);
    assert!(!editor.session.can_undo());
    key(&mut editor, &ctx, egui::Key::S, egui::Modifiers::COMMAND);
    assert!(editor.error.is_none(), "{:?}", editor.error);
    assert!(!editor.slot_color_edit_active());
    assert_eq!(files::load(&path).unwrap(), doc);
    assert_eq!(bytes(&editor, 0), [77, 88, 99, 0]);
    let p = begin_picker(&mut editor, &ctx);
    assert_ne!(editor.session.document(), &doc);
    key(&mut editor, &ctx, egui::Key::S, egui::Modifiers::COMMAND);
    assert_eq!(files::load(&path).unwrap(), doc);
    assert_eq!(editor.session.document(), &doc);
    assert!(!editor.slot_color_edit_active());
    pointer(&mut editor, &ctx, p, false);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn chooser_and_exact_feedback_fit_the_minimum_window_and_opening_is_not_a_document_edit() {
    let (mut editor, ctx) = fixture([1, 2, 3, 0]);
    let doc = editor.session.document().clone();
    for _ in 0..3 {
        let mut out = ctx.run_ui(
            egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1120., 740.))),
                ..Default::default()
            },
            |ui| editor.workspace_ui(ui),
        );
        out.textures_delta.clear();
    }
    let [left, top, right, bottom] = editor.capture_targets["slotColorPanelClip"];
    for name in [
        "slotColorTarget",
        "slotColorHex",
        "slotColorRGBA",
        "slotColorSwatch",
        "slotColorSV",
        "slotColorHue",
        "slotColorAlpha",
    ] {
        let [l, t, r, b] = editor.capture_targets[name];
        assert!([l, t, r, b].into_iter().all(f32::is_finite), "{name}");
        assert!(
            l >= left && t >= top && r <= right + 1. && b <= bottom + 1.,
            "{name}: {:?} outside {:?}",
            [l, t, r, b],
            [left, top, right, bottom]
        );
    }
    assert_eq!(editor.slot_color.hex, "#01020300");
    assert_eq!(editor.session.document(), &doc);
    assert!(!editor.session.can_undo());
    assert!(!editor.session.is_dirty());
}
