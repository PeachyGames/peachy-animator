use super::super::test_support::{editor_context, editor_fixture, mesh_editor, workspace_frame};
use super::*;

fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..4 {
        workspace_frame(editor, ctx, vec![]);
    }
}
fn point(editor: &Editor, name: &str) -> Pos2 {
    let [l, t, r, b] = editor.capture_targets[name];
    pos2((l + r) * 0.5, (t + b) * 0.5)
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
    let p = point(editor, name);
    for pressed in [true, false] {
        pointer(editor, ctx, p, pressed);
    }
    settle(editor, ctx);
}
fn press_key(editor: &mut Editor, ctx: &egui::Context, key: egui::Key, modifiers: egui::Modifiers) {
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
fn fixture() -> (Editor, AttachmentRef, egui::Context) {
    let (mut editor, key) = mesh_editor(false);
    let mut doc = editor.session.document().clone();
    doc.slots
        .iter_mut()
        .find(|slot| slot.name == key.slot)
        .unwrap()
        .bone = 0;
    doc.bones[0].setup = Transform::default();
    let Attachment::Mesh(mesh) = doc.attachment_mut(&key).unwrap() else {
        unreachable!()
    };
    mesh.vertices = MeshVertices::SlotLocal {
        positions: vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]],
    };
    editor.session = Session::new(doc).unwrap();
    editor.open_mesh_tools_view();
    editor.enter_mesh_tool();
    editor.mesh_tools.settings.size = 20.;
    editor.mesh_tools.settings.feather = 1.;
    editor.zoom = 2.;
    editor.fit = false;
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    (editor, key, ctx)
}
fn positions(editor: &Editor, key: &AttachmentRef) -> Vec<[f32; 2]> {
    let Attachment::Mesh(mesh) = editor.session.document().attachment(key).unwrap() else {
        unreachable!()
    };
    mesh.slot_local_positions().unwrap().to_vec()
}
fn begin_drag(editor: &mut Editor, ctx: &egui::Context) -> Pos2 {
    let [x, y] = editor.mesh_handles[0];
    let p = pos2(x, y);
    pointer(editor, ctx, p, true);
    let moved = p + vec2(20., -10.);
    for _ in 0..2 {
        workspace_frame(editor, ctx, vec![egui::Event::PointerMoved(moved)]);
    }
    assert_eq!(editor.mesh_drag.as_ref().map(|drag| drag.vertex), Some(0));
    p
}

#[test]
fn mesh_tools_menu_and_settings_are_session_only_and_restore_with_the_workspace() {
    let mut editor = editor_fixture();
    let mut pose = editor.selected_transform().unwrap();
    pose.x += 3.;
    editor.edit_transform(pose, &[Property::X]);
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let selection = editor.selection.clone();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "viewsMenu");
    click(&mut editor, &ctx, "viewMesh Tools");
    assert!(editor.view_tabs.open.contains(&views::DockView::MeshTools));
    assert!(!editor.setup_mode);
    for name in ["meshSoftSize", "meshSoftFeather"] {
        let [x, y, _, b] = editor.capture_targets[name];
        let position = pos2(x + 45., (y + b) * 0.5);
        for pressed in [true, false] {
            pointer(&mut editor, &ctx, position, pressed);
        }
    }
    assert_eq!(editor.mesh_tools.settings.size, 0.);
    click(&mut editor, &ctx, "meshSoftHull");
    assert!(editor.mesh_tools.settings.hull_vertices);
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.selection, selection);
    editor.set_setup_mode(true);
    settle(&mut editor, &ctx);
    for name in ["meshSoftSize", "meshSoftFeather"] {
        let [x, y, _, b] = editor.capture_targets[name];
        let position = pos2(x + 45., (y + b) * 0.5);
        for pressed in [true, false] {
            pointer(&mut editor, &ctx, position, pressed);
        }
    }
    assert!(editor.mesh_tools.settings.size > 0.);
    assert!(editor.mesh_tools.settings.feather > 0. && editor.mesh_tools.settings.feather < 1.);
    click(&mut editor, &ctx, "meshSoftHull");
    assert!(!editor.mesh_tools.settings.hull_vertices);
    let settings = editor.mesh_tools.settings;
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.pose_edits().is_empty());
    assert_eq!(editor.selection, selection);
    click(&mut editor, &ctx, "closeView:Mesh Tools");
    assert!(!editor.view_tabs.open.contains(&views::DockView::MeshTools));
    editor.open_mesh_tools_view();
    settle(&mut editor, &ctx);
    press_key(&mut editor, &ctx, egui::Key::F9, egui::Modifiers::NONE);
    assert!(!editor.has_dock_views());
    press_key(&mut editor, &ctx, egui::Key::F9, egui::Modifiers::NONE);
    assert!(editor.view_tabs.open.contains(&views::DockView::MeshTools));
    assert_eq!(editor.mesh_tools.settings.size, settings.size);
    assert_eq!(editor.mesh_tools.settings.feather, settings.feather);
    assert!(!editor.mesh_tools.settings.hull_vertices);
    editor.set_setup_mode(true);
    settle(&mut editor, &ctx);
    assert!(editor.capture_targets.contains_key("meshSoftSize"));
    click(&mut editor, &ctx, "viewsMenu");
    click(&mut editor, &ctx, "viewRestore");
    assert_eq!(editor.mesh_tools.settings.size, 0.);
    assert!(!editor.view_tabs.open.contains(&views::DockView::MeshTools));
}

#[test]
fn soft_vertex_drag_uses_frozen_weights_and_commits_once_including_release_position() {
    let (mut editor, key, ctx) = fixture();
    let original = editor.session.document().clone();
    let start = begin_drag(&mut editor, &ctx);
    let end = start + vec2(40., -20.);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
    pointer(&mut editor, &ctx, end, false);
    assert!(editor.mesh_drag.is_none());
    let result = positions(&editor, &key);
    assert_eq!(result[0], [20., 10.]);
    assert_eq!(result[1], [20., 5.]);
    assert_eq!(result[3], [10., 15.]);
    assert!((result[2][0] - (10. + 20. * (1. - 2.0_f32.sqrt() / 2.))).abs() < 0.00001);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
    editor.redo();
    assert_eq!(positions(&editor, &key), result);
    editor.undo();
    settle(&mut editor, &ctx);
    // Releasing at a new position must include that final pointer update.
    let start = begin_drag(&mut editor, &ctx);
    pointer(&mut editor, &ctx, start + vec2(60., -30.), false);
    assert_eq!(positions(&editor, &key)[0], [30., 15.]);
}

#[test]
fn returning_a_soft_preview_to_its_press_position_restores_every_affected_vertex() {
    let (mut editor, key, ctx) = fixture();
    let original = editor.session.document().clone();
    let start = begin_drag(&mut editor, &ctx);
    assert_ne!(positions(&editor, &key)[0], [0., 0.]);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(start)]);
    assert_eq!(editor.session.document(), &original);
    pointer(&mut editor, &ctx, start, false);
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
}

#[test]
fn an_invalid_view_projection_cancels_the_soft_preview_without_leaking_a_transaction() {
    let (mut editor, _, ctx) = fixture();
    let original = editor.session.document().clone();
    let start = begin_drag(&mut editor, &ctx);
    editor.zoom = f32::MAX;
    workspace_frame(&mut editor, &ctx, vec![]);
    assert!(editor.evaluation_error.is_some());
    assert!(editor.mesh_drag.is_none() && editor.mesh_blocked);
    assert_eq!(editor.session.document(), &original);
    editor.zoom = 2.;
    pointer(&mut editor, &ctx, start, false);
    settle(&mut editor, &ctx);
    assert!(editor.mesh_drag.is_none());
    assert!(!editor.session.can_undo());
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn cancelled_soft_drags_restore_the_baseline_and_block_the_held_pointer() {
    for action in [
        "escape",
        "undo",
        "redo",
        "mode",
        "tool",
        "selection",
        "f9",
        "modal",
        "focus",
        "close",
    ] {
        let (mut editor, key, ctx) = fixture();
        let original = editor.session.document().clone();
        let start = begin_drag(&mut editor, &ctx);
        assert_ne!(editor.session.document(), &original, "{action}");
        match action {
            "escape" => key_event(&mut editor, &ctx, egui::Key::Escape),
            "undo" => editor.undo(),
            "redo" => editor.redo(),
            "mode" => editor.set_setup_mode(false),
            "tool" => editor.set_tool(Tool::Rotate),
            "selection" => editor.select(TreeItem::Bone(8)),
            "f9" => editor.toggle_views(),
            "modal" => {
                editor.error = Some("test modal".into());
                workspace_frame(&mut editor, &ctx, vec![]);
            }
            "focus" => {
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        focused: false,
                        screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
                        ..Default::default()
                    },
                    |ui| editor.workspace_ui(ui),
                );
                out.textures_delta.clear();
            }
            "close" => {
                editor.cancel_gesture();
                editor.view_tabs.open.clear();
            }
            _ => unreachable!(),
        }
        assert!(editor.mesh_drag.is_none(), "{action}");
        assert_eq!(editor.session.document(), &original, "{action}");
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(start + vec2(80., -40.))],
        );
        assert_eq!(editor.session.document(), &original, "{action}");
        assert!(editor.mesh_drag.is_none(), "{action}");
        pointer(&mut editor, &ctx, start, false);
        settle(&mut editor, &ctx);
        assert!(!editor.mesh_blocked, "{action}");
        assert_eq!(positions(&editor, &key)[0], [0., 0.]);
    }
}

fn key_event(editor: &mut Editor, ctx: &egui::Context, key: egui::Key) {
    workspace_frame(
        editor,
        ctx,
        vec![egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
}

#[test]
fn hull_filter_and_nudges_share_soft_selection_and_zero_size_moves_only_the_selected_vertex() {
    let (mut editor, key, ctx) = fixture();
    editor.set_tool(Tool::Rotate);
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "meshSoftVertices");
    assert!(editor.tool == Tool::Mesh);
    let [x, y] = editor.mesh_handles[0];
    for pressed in [true, false] {
        pointer(&mut editor, &ctx, pos2(x, y), pressed);
    }
    editor.mesh_tools.settings.hull_vertices = false;
    key_event(&mut editor, &ctx, egui::Key::ArrowRight);
    let result = positions(&editor, &key);
    assert_eq!(result, vec![[1., 0.], [10., 0.], [10., 10.], [0., 10.]]);
    editor.undo();
    editor.mesh_tools.settings.hull_vertices = true;
    press_key(
        &mut editor,
        &ctx,
        egui::Key::ArrowRight,
        egui::Modifiers::NONE,
    );
    assert_eq!(positions(&editor, &key)[1], [10.5, 0.]);
    editor.undo();
    editor.mesh_tools.settings.size = 0.;
    press_key(
        &mut editor,
        &ctx,
        egui::Key::ArrowRight,
        egui::Modifiers::NONE,
    );
    assert_eq!(positions(&editor, &key), result);
}

#[test]
fn weighted_and_inactive_meshes_do_not_acquire_soft_selection_edits() {
    let (mut editor, key) = mesh_editor(true);
    editor.set_setup_mode(false);
    editor.open_mesh_tools_view();
    editor.enter_mesh_tool();
    editor.mesh_tools.settings.size = 200.;
    let original = editor.session.document().clone();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    assert!(!editor.setup_mode);
    assert!(editor.mesh_translation(&key, 0).is_err());
    assert!(editor.mesh_tools_diagnostic()["weights"].is_null());
    key_event(&mut editor, &ctx, egui::Key::ArrowRight);
    assert_eq!(editor.session.document(), &original);
    let (mut editor, key, ctx) = fixture();
    let mut doc = editor.session.document().clone();
    let attachment = doc.attachment(&key).unwrap().clone();
    let key = AttachmentRef {
        skin: "peach".into(),
        ..key
    };
    doc.skins
        .get_mut("peach")
        .unwrap()
        .entry(key.slot.clone())
        .or_default()
        .insert(key.placeholder.clone(), attachment);
    editor.session = Session::new(doc).unwrap();
    editor.selection = Some(TreeItem::Attachment(key.clone()));
    editor.skin = "mint".into();
    let original = editor.session.document().clone();
    editor.begin_mesh_drag(key, 0, Pos2::ZERO);
    assert!(editor.mesh_drag.is_none());
    key_event(&mut editor, &ctx, egui::Key::ArrowRight);
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn radius_preview_and_controls_fit_the_minimum_dock_without_document_edits() {
    let (mut editor, _, ctx) = fixture();
    let original = editor.session.document().clone();
    for _ in 0..4 {
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
    for name in [
        "meshSoftSize",
        "meshSoftFeather",
        "meshSoftHull",
        "closeView:Mesh Tools",
    ] {
        let rect = editor.capture_targets[name];
        assert!(
            rect[0] >= 0. && rect[2] <= 1120. && rect[1] >= 0. && rect[3] <= 740.,
            "{name}"
        );
    }
    let [left, top, right, bottom] = editor.capture_targets["meshSoftPanelClip"];
    for name in ["meshSoftSize", "meshSoftFeather", "meshSoftHull"] {
        let [a, b, c, d] = editor.capture_targets[name];
        assert!(a >= left && c <= right && b >= top && d <= bottom, "{name}");
    }
    settle(&mut editor, &ctx);
    let hover = point(&editor, "meshSoftSize");
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(hover)]);
    assert!(editor.mesh_tools.show_radius);
    assert!(editor.capture_targets.contains_key("meshSoftRadius"));
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
}

#[test]
fn save_cancels_a_soft_preview_before_writing_and_preserves_the_previous_redo() {
    let (mut editor, key, ctx) = fixture();
    let original = editor.session.document().clone();
    key_event(&mut editor, &ctx, egui::Key::ArrowRight);
    editor.undo();
    assert!(editor.session.can_redo());
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("peachy-soft-save-{}-{stamp}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("rig.peachy");
    editor.project_path = Some(path.clone());
    editor.asset_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
    let start = begin_drag(&mut editor, &ctx);
    assert_ne!(positions(&editor, &key)[0], [0., 0.]);
    press_key(&mut editor, &ctx, egui::Key::S, egui::Modifiers::COMMAND);
    assert!(editor.error.is_none(), "{:?}", editor.error);
    assert!(editor.mesh_drag.is_none());
    assert_eq!(files::load(&path).unwrap(), original);
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.can_redo());
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(start + vec2(80., -40.))],
    );
    assert_eq!(editor.session.document(), &original);
    pointer(&mut editor, &ctx, start, false);
    std::fs::remove_dir_all(directory).unwrap();
}
