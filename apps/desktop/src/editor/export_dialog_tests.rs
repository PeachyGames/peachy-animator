use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
use super::*;

fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers,
    }
}
fn pointer_event(point: Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(point),
        egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}
fn point(editor: &Editor, name: &str) -> Pos2 {
    let [l, t, r, b] = editor.capture_targets[name];
    pos2((l + r) * 0.5, (t + b) * 0.5)
}
fn choice_frame(
    editor: &mut Editor,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
) -> Option<ExportTarget> {
    let mut target = None;
    editor.capture_targets.clear();
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1120., 740.))),
            focused: true,
            events,
            ..Default::default()
        },
        |ui| {
            target = editor.export_target_dialog(ui.ctx());
        },
    );
    output.textures_delta.clear();
    target
}
fn click_choice(editor: &mut Editor, ctx: &egui::Context, name: &str) -> Option<ExportTarget> {
    // Area centering uses the measured size from its previous frame. Click only
    // after that geometry settles, as the real visible-modal workflow does.
    for _ in 0..3 {
        assert_eq!(choice_frame(editor, ctx, vec![]), None);
    }
    let rect = editor.capture_targets[name];
    let p = point(editor, name);
    assert_eq!(choice_frame(editor, ctx, pointer_event(p, true)), None);
    assert_eq!(
        editor.capture_targets[name], rect,
        "Target moved during the press"
    );
    choice_frame(editor, ctx, pointer_event(p, false))
}
fn pending_redo(editor: &mut Editor) {
    let mut pose = editor.session.local_pose(Some(0), editor.time)[8];
    pose.rotation += 12.;
    editor
        .session
        .edit_pose(
            8,
            editor.weights_edit_target(),
            pose,
            &[Property::Rotation],
            false,
        )
        .unwrap();
    editor
        .session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo sentinel".into(),
        })
        .unwrap();
    editor.session.undo();
    assert!(editor.session.can_redo());
}

#[test]
fn export_target_chooser_defaults_to_editor_and_runtime_is_an_explicit_per_export_choice() {
    let mut editor = editor_fixture();
    pending_redo(&mut editor);
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let ctx = editor_context();
    editor.export();
    choice_frame(&mut editor, &ctx, vec![]);
    assert_eq!(
        editor.export_dialog.as_ref().unwrap().target,
        ExportTarget::SpineEditor43
    );
    assert_eq!(
        click_choice(&mut editor, &ctx, "exportTarget:runtime43"),
        None
    );
    assert_eq!(
        editor.export_dialog.as_ref().unwrap().target,
        ExportTarget::Runtime43
    );
    assert_eq!(
        click_choice(&mut editor, &ctx, "exportTargetContinue"),
        Some(ExportTarget::Runtime43)
    );
    assert!(editor.export_dialog.is_none());
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(editor.session.can_redo());
    editor.export();
    choice_frame(&mut editor, &ctx, vec![]);
    assert_eq!(
        click_choice(&mut editor, &ctx, "exportTargetContinue"),
        Some(ExportTarget::SpineEditor43)
    );
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn export_target_chooser_runtime_radio_stays_interactive_in_the_disabled_full_workspace() {
    let mut editor = editor_fixture();
    editor.set_setup_mode(true);
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let can_undo = editor.session.can_undo();
    let can_redo = editor.session.can_redo();
    let ctx = editor_context();
    workspace_frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::E, egui::Modifiers::COMMAND)],
    );
    for _ in 0..3 {
        workspace_frame(&mut editor, &ctx, vec![]);
    }
    for (name, target) in [
        ("exportTarget:runtime43", ExportTarget::Runtime43),
        ("exportTarget:spineEditor43", ExportTarget::SpineEditor43),
        ("exportTarget:runtime43", ExportTarget::Runtime43),
    ] {
        let rect = editor.capture_targets[name];
        let p = point(&editor, name);
        workspace_frame(&mut editor, &ctx, pointer_event(p, true));
        assert_eq!(
            editor.capture_targets[name], rect,
            "Radio moved during press"
        );
        workspace_frame(&mut editor, &ctx, pointer_event(p, false));
        assert_eq!(
            editor.capture_targets[name], rect,
            "Radio moved during release"
        );
        assert_eq!(editor.export_dialog.as_ref().unwrap().target, target);
    }
    assert_eq!(editor.export_dialog_diagnostic()["target"], "runtime43");
    let cancel = point(&editor, "exportTargetCancel");
    workspace_frame(&mut editor, &ctx, pointer_event(cancel, true));
    workspace_frame(&mut editor, &ctx, pointer_event(cancel, false));
    assert!(editor.export_dialog.is_none());
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.session.can_undo(), can_undo);
    assert_eq!(editor.session.can_redo(), can_redo);
}

#[test]
fn export_target_chooser_cancel_and_escape_keep_pending_document_selection_and_redo() {
    for cancel in ["button", "escape"] {
        let mut editor = editor_fixture();
        pending_redo(&mut editor);
        let original = editor.session.document().clone();
        let pending = editor.session.pose_edits().clone();
        let selection = editor.selection.clone();
        let can_undo = editor.session.can_undo();
        let ctx = editor_context();
        workspace_frame(
            &mut editor,
            &ctx,
            vec![key(egui::Key::E, egui::Modifiers::COMMAND)],
        );
        assert!(editor.export_dialog.is_some());
        for _ in 0..3 {
            workspace_frame(&mut editor, &ctx, vec![]);
        }
        if cancel == "button" {
            let rect = editor.capture_targets["exportTargetCancel"];
            let p = point(&editor, "exportTargetCancel");
            workspace_frame(&mut editor, &ctx, pointer_event(p, true));
            assert_eq!(
                editor.capture_targets["exportTargetCancel"], rect,
                "Cancel moved during the press"
            );
            workspace_frame(&mut editor, &ctx, pointer_event(p, false));
        } else {
            workspace_frame(
                &mut editor,
                &ctx,
                vec![key(egui::Key::Escape, egui::Modifiers::NONE)],
            );
        }
        assert!(editor.export_dialog.is_none(), "{cancel}");
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(editor.selection, selection);
        assert_eq!(editor.session.can_undo(), can_undo);
        assert!(editor.session.can_redo());
    }
}

#[test]
fn export_target_chooser_blocks_authoring_shortcuts_and_history_first_dismisses_it() {
    let mut editor = editor_fixture();
    pending_redo(&mut editor);
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let time = editor.time;
    let ctx = editor_context();
    workspace_frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::E, egui::Modifiers::COMMAND)],
    );
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            key(egui::Key::K, egui::Modifiers::NONE),
            key(egui::Key::ArrowRight, egui::Modifiers::NONE),
        ],
    );
    assert!(editor.export_dialog.is_some());
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.time, time);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::Z, egui::Modifiers::COMMAND)],
    );
    assert!(editor.export_dialog.is_none());
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(editor.session.can_redo());
}

#[test]
fn export_target_chooser_focus_context_and_view_changes_cancel_without_writing() {
    for cancel in ["focus", "skin", "mode", "views"] {
        let mut editor = editor_fixture();
        pending_redo(&mut editor);
        let original = editor.session.document().clone();
        let pending = editor.session.pose_edits().clone();
        let ctx = editor_context();
        workspace_frame(
            &mut editor,
            &ctx,
            vec![key(egui::Key::E, egui::Modifiers::COMMAND)],
        );
        match cancel {
            "skin" => editor.activate_skin("mint".into()),
            "mode" => editor.set_setup_mode(true),
            "views" => workspace_frame(
                &mut editor,
                &ctx,
                vec![key(egui::Key::F9, egui::Modifiers::NONE)],
            ),
            _ => {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1120., 740.))),
                        focused: false,
                        ..Default::default()
                    },
                    |ui| editor.workspace_ui(ui),
                );
                output.textures_delta.clear();
            }
        }
        assert!(editor.export_dialog.is_none(), "{cancel}");
        assert_eq!(editor.session.document(), &original);
        if cancel != "mode" {
            assert_eq!(editor.session.pose_edits(), &pending);
        }
        assert!(editor.session.can_redo());
    }
}

#[test]
fn export_target_chooser_cancels_a_held_color_preview_and_prevents_reentry() {
    let mut editor = editor_fixture();
    editor.set_setup_mode(true);
    editor.select(TreeItem::Slot(0));
    editor.open_slot_color_view();
    editor
        .session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo sentinel".into(),
        })
        .unwrap();
    editor.session.undo();
    let original = editor.session.document().clone();
    let ctx = editor_context();
    for _ in 0..3 {
        workspace_frame(&mut editor, &ctx, vec![]);
    }
    let [l, t, r, b] = editor.capture_targets["slotColorSV"];
    let p = pos2(l + (r - l) * 0.8, t + (b - t) * 0.2);
    workspace_frame(&mut editor, &ctx, pointer_event(p, true));
    assert!(editor.slot_color_edit_active());
    assert_ne!(editor.session.document(), &original);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![key(egui::Key::E, egui::Modifiers::COMMAND)],
    );
    assert!(editor.export_dialog.is_some());
    assert!(!editor.slot_color_edit_active());
    assert_eq!(editor.session.document(), &original);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            key(egui::Key::Escape, egui::Modifiers::NONE),
            egui::Event::PointerMoved(p + vec2(20., 10.)),
        ],
    );
    assert!(editor.export_dialog.is_none());
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.can_redo());
}
