use super::super::test_support::{editor_context, mesh_editor, workspace_frame};
use super::*;

fn fixture(weighted: bool, auto_key: bool) -> (Editor, AttachmentRef, egui::Context) {
    let (mut editor, key) = mesh_editor(weighted);
    editor.set_setup_mode(false);
    editor.auto_key = auto_key;
    editor.show_bones = false;
    editor.enter_mesh_tool();
    let ctx = editor_context();
    for _ in 0..3 {
        workspace_frame(&mut editor, &ctx, vec![]);
    }
    assert!(!editor.setup_mode);
    assert_eq!(editor.mesh_handles.len(), 4);
    (editor, key, ctx)
}

fn pointer(editor: &mut Editor, ctx: &egui::Context, point: Pos2, pressed: bool) {
    workspace_frame(
        editor,
        ctx,
        vec![
            egui::Event::PointerMoved(point),
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
}

fn key(editor: &mut Editor, ctx: &egui::Context, key: egui::Key) {
    for pressed in [true, false] {
        workspace_frame(
            editor,
            ctx,
            vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
}

fn click(editor: &mut Editor, ctx: &egui::Context, target: &str) {
    // Key requests settle after this frame's properties were laid out. Render
    // their resulting enabled state before locating and pressing another button.
    workspace_frame(editor, ctx, vec![]);
    let [left, top, right, bottom] = editor.capture_targets[target];
    let p = pos2((left + right) * 0.5, (top + bottom) * 0.5);
    pointer(editor, ctx, p, true);
    pointer(editor, ctx, p, false);
}

fn vertices(editor: &Editor, key: &AttachmentRef) -> Vec<[f32; 2]> {
    editor
        .scene_for_view()
        .unwrap()
        .attachments
        .iter()
        .find(|draw| &draw.attachment == key)
        .unwrap()
        .vertices
        .clone()
}

fn near(actual: [f32; 2], expected: [f32; 2]) {
    for axis in 0..2 {
        assert!(
            (actual[axis] - expected[axis]).abs() < 0.001,
            "{actual:?} != {expected:?}"
        );
    }
}

fn begin_drag(editor: &mut Editor, ctx: &egui::Context) -> Pos2 {
    let p = pos2(editor.mesh_handles[0][0], editor.mesh_handles[0][1]);
    pointer(editor, ctx, p, true);
    workspace_frame(
        editor,
        ctx,
        vec![egui::Event::PointerMoved(p + vec2(16., -8.))],
    );
    assert!(editor.deform_drag.is_some(), "{:?}", editor.error);
    p
}

#[test]
fn animate_vertex_nudge_and_key_button_preserve_setup_and_pending_bone_pose() {
    for weighted in [false, true] {
        let (mut editor, attachment, ctx) = fixture(weighted, false);
        let mut bone = editor.session.local_pose(Some(0), 0.)[8];
        bone.rotation += 12.;
        editor
            .session
            .edit_pose(
                8,
                EditTarget::Animate {
                    animation: 0,
                    time: 0.,
                },
                bone,
                &[Property::Rotation],
                false,
            )
            .unwrap();
        let original = editor.session.document().clone();
        let before = vertices(&editor, &attachment);
        key(&mut editor, &ctx, egui::Key::ArrowRight);
        let after = vertices(&editor, &attachment);
        near(after[0], [before[0][0] + 1., before[0][1]]);
        assert_eq!(&after[1..], &before[1..]);
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.is_dirty());
        assert!(editor.session.pose_edits().has_bone(0, 8));
        assert!(editor.session.pose_edits().has_deform(0, &attachment));
        assert_eq!(
            editor
                .session
                .pose_edits()
                .deform_key_state(editor.session.document(), 0, 0., &attachment)
                .unwrap(),
            KeyState::Edited
        );
        click(&mut editor, &ctx, "meshDeformKey");
        assert!(editor.session.is_dirty());
        assert!(editor.session.pose_edits().has_bone(0, 8));
        assert!(!editor.session.pose_edits().has_deform(0, &attachment));
        assert_eq!(
            editor.session.document().attachment(&attachment),
            original.attachment(&attachment)
        );
        assert_eq!(
            editor
                .session
                .pose_edits()
                .deform_key_state(editor.session.document(), 0, 0., &attachment)
                .unwrap(),
            KeyState::Keyed
        );
        let keyed = vertices(&editor, &attachment);
        near(keyed[0], after[0]);
        click(&mut editor, &ctx, "meshDeformDelete");
        assert!(
            editor.session.document().animations[0]
                .deform_tracks
                .is_empty(),
            "Delete did not remove the keyed deform: weighted={weighted}, error={:?}, tracks={:?}",
            editor.error,
            editor.session.document().animations[0].deform_tracks
        );
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.pose_edits().has_bone(0, 8));
        assert!(editor.error.is_none(), "{:?}", editor.error);
    }
}

#[test]
fn animate_mesh_drag_uses_one_frozen_preview_and_one_undo_including_release() {
    for weighted in [false, true] {
        let (mut editor, attachment, ctx) = fixture(weighted, true);
        let original = editor.session.document().clone();
        let before = vertices(&editor, &attachment);
        let start = begin_drag(&mut editor, &ctx);
        let zoom = editor.zoom;
        let end = start + vec2(34., -18.);
        for _ in 0..2 {
            workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
        }
        let release = end + vec2(6., -2.);
        pointer(&mut editor, &ctx, release, false);
        assert!(editor.deform_drag.is_none());
        let after = vertices(&editor, &attachment);
        near(
            after[0],
            [before[0][0] + 40. / zoom, before[0][1] + 20. / zoom],
        );
        assert_eq!(&after[1..], &before[1..]);
        assert_eq!(
            editor.session.document().attachment(&attachment),
            original.attachment(&attachment)
        );
        let changed = editor.session.document().clone();
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
        editor.redo();
        assert_eq!(editor.session.document(), &changed);
        assert!(editor.session.pose_edits().is_empty());
        assert!(editor.error.is_none(), "{:?}", editor.error);
    }
}

#[test]
fn escape_cancels_mesh_deform_preview_and_blocks_held_pointer_without_losing_redo() {
    for weighted in [false, true] {
        for auto_key in [false, true] {
            let (mut editor, attachment, ctx) = fixture(weighted, auto_key);
            editor.nudge_deform_vertex(attachment.clone(), [1., 0.]);
            editor.undo();
            assert!(editor.session.can_redo());
            let original = editor.session.document().clone();
            let pending = editor.session.pose_edits().clone();
            let start = begin_drag(&mut editor, &ctx);
            key(&mut editor, &ctx, egui::Key::Escape);
            assert!(editor.deform_drag.is_none());
            assert!(editor.mesh_blocked);
            assert_eq!(editor.session.document(), &original);
            assert_eq!(editor.session.pose_edits(), &pending);
            workspace_frame(
                &mut editor,
                &ctx,
                vec![egui::Event::PointerMoved(start + vec2(80., -30.))],
            );
            assert_eq!(editor.session.document(), &original);
            assert_eq!(editor.session.pose_edits(), &pending);
            assert!(editor.session.can_redo());
            pointer(&mut editor, &ctx, start, false);
            workspace_frame(&mut editor, &ctx, vec![]);
            assert!(!editor.mesh_blocked);
        }
    }
}

#[test]
fn key_edited_seals_deform_preview_and_prevents_the_held_pointer_from_rewriting_it() {
    let (mut editor, attachment, ctx) = fixture(true, false);
    let start = begin_drag(&mut editor, &ctx);
    key(&mut editor, &ctx, egui::Key::K);
    assert!(editor.deform_drag.is_none());
    assert!(editor.mesh_blocked);
    assert!(editor.session.pose_edits().is_empty());
    assert_eq!(
        editor
            .session
            .pose_edits()
            .deform_key_state(editor.session.document(), 0, 0., &attachment)
            .unwrap(),
        KeyState::Keyed
    );
    let keyed = editor.session.document().clone();
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(start + vec2(100., -40.))],
    );
    assert_eq!(editor.session.document(), &keyed);
    pointer(&mut editor, &ctx, start, false);
    editor.undo();
    assert!(!editor.session.is_dirty());
    assert!(editor.session.pose_edits().is_empty());
}

#[test]
fn context_changes_and_focus_loss_cancel_deform_previews() {
    for action in ["time", "mode", "selection", "views", "focus", "undo"] {
        let (mut editor, _, ctx) = fixture(true, true);
        let original = editor.session.document().clone();
        begin_drag(&mut editor, &ctx);
        match action {
            "time" => editor.set_time(0.5),
            "mode" => editor.set_setup_mode(true),
            "selection" => editor.select(TreeItem::Bone(8)),
            "views" => editor.toggle_views(),
            "undo" => editor.undo(),
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
        assert!(editor.deform_drag.is_none(), "{action}");
        assert_eq!(editor.session.document(), &original, "{action}");
        assert!(!editor.session.can_undo(), "{action}");
    }
}
