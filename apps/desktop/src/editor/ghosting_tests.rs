use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
use super::*;

fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..12 {
        workspace_frame(editor, ctx, vec![]);
    }
}

fn click(editor: &mut Editor, ctx: &egui::Context, name: &str) {
    let [left, top, right, bottom] = editor.capture_targets[name];
    let pos = pos2((left + right) * 0.5, (top + bottom) * 0.5);
    for pressed in [true, false] {
        workspace_frame(
            editor,
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    settle(editor, ctx);
}

fn enter_number(editor: &mut Editor, ctx: &egui::Context, name: &str, value: &str) {
    click(editor, ctx, name);
    assert!(ctx.text_edit_focused(), "{name} did not receive text focus");
    workspace_frame(editor, ctx, vec![egui::Event::Text(value.into())]);
    for pressed in [true, false] {
        workspace_frame(
            editor,
            ctx,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: Some(egui::Key::Enter),
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    settle(editor, ctx);
}

fn frame(editor: &Editor) -> GhostFrame {
    editor
        .ghost_frame(
            egui::Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.)),
            pos2(600., 400.),
            1.,
        )
        .unwrap()
}

#[test]
fn current_ghost_uses_keyed_pose_and_preserves_pending_pose_and_document_history() {
    let mut editor = editor_fixture();
    let original = editor.session.document().clone();
    let saved = editor.selected_transform().unwrap();
    let mut pending = saved;
    pending.rotation += 27.;
    editor.edit_transform(pending, &[Property::Rotation]);
    let edits = editor.session.pose_edits().clone();
    let history = (editor.session.can_undo(), editor.session.can_redo());
    let time = editor.time;
    editor.ghosting.enabled = true;
    editor.ghosting.frames.before = false;
    editor.ghosting.frames.after = false;
    editor.ghosting.current = true;
    let ghosts = frame(&editor);
    assert_eq!(ghosts.sample_count(), 1);
    assert_eq!(ghosts.poses[0].scene.locals[8], saved);
    assert_eq!(editor.scene_for_view().unwrap().locals[8], pending);
    assert_eq!(editor.session.pose_edits(), &edits);
    assert_eq!(editor.session.document(), &original);
    assert_eq!(
        (editor.session.can_undo(), editor.session.can_redo()),
        history
    );
    assert_eq!(editor.time, time);
    assert!(!editor.session.is_dirty());
}

#[test]
fn ghost_geometry_reuses_checked_weighted_mesh_and_ik_evaluation_at_each_sample() {
    for fixture in ["weighted-bot.peachy", "ik-bot.peachy"] {
        let mut editor = editor_fixture();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/peach-bot")
            .join(fixture);
        editor.session = Session::new(files::load(&path).unwrap()).unwrap();
        editor.skin = preferred_skin(editor.session.document());
        editor.set_time(0.3);
        editor.ghosting.enabled = true;
        editor.ghosting.offset = [3., -2.];
        let original = editor.session.document().clone();
        let samples =
            peachy_engine::ghost_samples(&original, 0, editor.time, &editor.ghost_sampling())
                .unwrap();
        let ghosts = frame(&editor);
        assert!(!ghosts.poses.is_empty());
        assert_eq!(samples.len(), ghosts.sample_count());
        for (sample, ghost) in samples.iter().zip(&ghosts.poses) {
            let expected = peachy_engine::checked_evaluate_scene(
                &original,
                Some(0),
                sample.time,
                &editor.skin,
                None,
            )
            .unwrap();
            assert_eq!(ghost.scene.locals, expected.locals);
            assert_eq!(ghost.scene.applied_locals, expected.applied_locals);
            assert_eq!(
                serde_json::to_value(&ghost.scene.world).unwrap(),
                serde_json::to_value(&expected.world).unwrap()
            );
            assert_eq!(
                serde_json::to_value(&ghost.scene.attachments).unwrap(),
                serde_json::to_value(&expected.attachments).unwrap()
            );
            assert!((ghost.origin.x - (600. + sample.frame_offset * 3.)).abs() < 0.0001);
            assert!((ghost.origin.y - (400. + sample.frame_offset * 2.)).abs() < 0.0001);
        }
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
    }
}

#[test]
fn selection_filter_is_typed_and_hidden_slots_and_locked_bones_are_reconciled() {
    let mut editor = editor_fixture();
    editor.ghosting.enabled = true;
    editor.ghosting.selection_only = true;
    let all_count = editor.scene_for_view().unwrap().attachments.len();
    let assert_bone = |editor: &Editor, bone| {
        let ghosts = frame(editor);
        for pose in ghosts.poses {
            assert!(!pose.scene.attachments.is_empty());
            assert!(
                pose.scene
                    .attachments
                    .iter()
                    .all(|draw| editor.session.document().slots[draw.slot].bone == bone)
            );
        }
    };
    assert_bone(&editor, 8);
    editor.ghosting.locked = true;
    editor.ghosting.locked_bone = Some(8);
    editor.select(TreeItem::Bone(7));
    assert_bone(&editor, 8);
    editor.ghosting.locked = false;
    editor.select(TreeItem::Slot(0));
    assert!(
        frame(&editor)
            .poses
            .iter()
            .all(|pose| pose.scene.attachments.len() == all_count)
    );
    editor.hidden_slots.insert(0);
    assert!(
        frame(&editor).poses.iter().all(|pose| pose
            .scene
            .attachments
            .iter()
            .all(|draw| draw.slot != 0))
    );
    editor.ghosting.locked = true;
    editor.ghosting.locked_bone = Some(usize::MAX);
    editor.reconcile_selection();
    assert!(!editor.ghosting.locked);
    assert_eq!(editor.ghosting.locked_bone, None);
    editor.ghosting.locked = true;
    editor.ghosting.locked_bone = Some(8);
    editor.ghosting.reset_document();
    assert!(!editor.ghosting.locked && editor.ghosting.locked_bone.is_none());
}

#[test]
fn ghost_failures_do_not_hide_current_geometry_or_change_fit_history_or_selection() {
    let mut editor = editor_fixture();
    let ctx = editor_context();
    editor.fit = true;
    settle(&mut editor, &ctx);
    // Fit after the real panels have settled their initial measured dimensions.
    editor.fit = true;
    settle(&mut editor, &ctx);
    let baseline = (editor.zoom, editor.pan, editor.selection.clone());
    let original = editor.session.document().clone();
    let bounds = editor.scene_for_view().unwrap().bounds;
    editor.ghosting.enabled = true;
    editor.ghosting.frames.before_frames = 60.;
    editor.ghosting.frames.after_frames = 60.;
    editor.ghosting.frames.step = 0.25;
    editor.ghosting.looping = true;
    editor.fit = true;
    settle(&mut editor, &ctx);
    assert!(editor.ghosting.error.is_some());
    assert_eq!(editor.ghosting.rendered_samples, 0);
    assert!(editor.evaluation_error.is_none());
    assert_eq!(editor.scene_for_view().unwrap().bounds, bounds);
    assert_eq!(
        (editor.zoom, editor.pan, editor.selection.clone()),
        baseline
    );
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
    editor.ghosting.enabled = false;
    settle(&mut editor, &ctx);
    assert!(editor.ghosting.error.is_none());
}

#[test]
fn actual_ghost_controls_change_preview_and_keep_pose_selection_and_workspace_lifecycle() {
    let mut editor = editor_fixture();
    let ctx = editor_context();
    editor.show_tree = false;
    editor.show_dopesheet = false;
    let original = editor.session.document().clone();
    let selection = editor.selection.clone();
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "viewsMenu");
    click(&mut editor, &ctx, "viewGhosting");
    click(&mut editor, &ctx, "ghostEnabled");
    assert!(editor.ghosting.enabled);
    assert_eq!(editor.ghosting.rendered_samples, 4);
    enter_number(&mut editor, &ctx, "ghostFramesStepValue", "1.5");
    assert_eq!(editor.ghosting.frames.step, 1.5);
    assert_eq!(editor.ghosting.rendered_samples, 8);
    click(&mut editor, &ctx, "ghostCurrent");
    assert_eq!(editor.ghosting.rendered_samples, 9);
    click(&mut editor, &ctx, "ghostSection:Frames");
    click(&mut editor, &ctx, "ghostSection:Key Frames");
    click(&mut editor, &ctx, "ghostKeysBefore");
    assert!(editor.ghosting.keys.before);
    enter_number(&mut editor, &ctx, "ghostKeysStepValue", "2.4");
    assert_eq!(editor.ghosting.keys.step, 2.);
    assert!(editor.ghosting.error.is_none());
    click(&mut editor, &ctx, "ghostKeysBefore");
    click(&mut editor, &ctx, "ghostSection:Key Frames");
    click(&mut editor, &ctx, "ghostSection:Display");
    enter_number(&mut editor, &ctx, "ghostOpacityValue", "45");
    assert!((editor.ghosting.opacity - 0.45).abs() < 0.000001);
    click(&mut editor, &ctx, "ghostSection:Options");
    editor.set_time(0.6);
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "ghostAnchor");
    assert!(editor.ghosting.anchor);
    assert_eq!(editor.ghosting.rendered_samples, 9); // Eight lattice poses plus Current.
    click(&mut editor, &ctx, "ghostSelection");
    click(&mut editor, &ctx, "ghostLock");
    assert_eq!(editor.ghosting.locked_bone, Some(8));
    editor.select(TreeItem::Bone(7));
    settle(&mut editor, &ctx);
    assert_eq!(editor.ghost_bone(), Some(8));
    click(&mut editor, &ctx, "ghostRefresh");
    assert_eq!(editor.ghost_bone(), Some(7));
    click(&mut editor, &ctx, "ghostOn top");
    assert!(editor.ghosting.on_top);
    enter_number(&mut editor, &ctx, "ghostOffset0Value", "2.5");
    assert_eq!(editor.ghosting.offset[0], 2.5);
    editor.select(selection.clone().unwrap());
    editor.toggle_views();
    settle(&mut editor, &ctx);
    assert!(editor.ghosting.enabled);
    assert!(editor.ghosting.rendered_samples > 0);
    editor.toggle_views();
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "closeView:Ghosting");
    assert!(editor.ghosting.enabled);
    editor.set_setup_mode(true);
    settle(&mut editor, &ctx);
    assert_eq!(editor.ghosting.rendered_samples, 0);
    editor.set_setup_mode(false);
    settle(&mut editor, &ctx);
    assert!(editor.ghosting.rendered_samples > 0);
    click(&mut editor, &ctx, "ghostViewportEnabled");
    assert!(!editor.ghosting.enabled);
    assert_eq!(editor.ghosting.rendered_samples, 0);
    assert_eq!(editor.selection, selection);
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
    assert!(!editor.session.is_dirty());
}

#[test]
fn primary_ghost_controls_fit_the_minimum_default_dock() {
    let mut editor = editor_fixture();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "viewsMenu");
    click(&mut editor, &ctx, "viewGhosting");
    for _ in 0..12 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1120., 740.))),
                focused: true,
                ..Default::default()
            },
            |ui| editor.workspace_ui(ui),
        );
        output.textures_delta.clear();
    }
    let [left, top, right, bottom] = editor.capture_targets["ghostPanelClip"];
    let clip = egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom));
    for name in [
        "ghostEnabled",
        "ghostFramesBefore",
        "ghostFramesStepValue",
        "ghostCurrent",
    ] {
        let [left, top, right, bottom] = editor.capture_targets[name];
        assert!(
            clip.contains_rect(egui::Rect::from_min_max(
                pos2(left, top),
                pos2(right, bottom)
            )),
            "{name} is clipped"
        );
    }
}
