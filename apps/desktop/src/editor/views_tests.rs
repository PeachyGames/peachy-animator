use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
use super::*;

fn point(editor: &Editor, name: &str) -> Pos2 {
    let [left, top, right, bottom] = editor.capture_targets[name];
    pos2((left + right) * 0.5, (top + bottom) * 0.5)
}

fn click(editor: &mut Editor, ctx: &egui::Context, name: &str, button: egui::PointerButton) {
    let pos = point(editor, name);
    for pressed in [true, false] {
        workspace_frame(
            editor,
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    workspace_frame(editor, ctx, vec![]);
}

fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..3 {
        workspace_frame(editor, ctx, vec![]);
    }
}

#[test]
fn full_view_inventory_is_discoverable_and_setup_rejects_animate_panels() {
    let mut editor = editor_fixture();
    editor.setup_mode = true;
    let original = editor.session.document().clone();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "viewsMenu", egui::PointerButton::Primary);
    for name in [
        "Animations",
        "Audio",
        "Dopesheet",
        "Ghosting",
        "Graph",
        "Mesh Tools",
        "Metrics",
        "Outline",
        "Playback",
        "Preview",
        "Skins",
        "Slot Color",
        "Timeline",
        "Tree",
        "Weights",
    ] {
        assert!(
            editor.capture_targets.contains_key(&format!("view{name}")),
            "{name}"
        );
    }
    click(
        &mut editor,
        &ctx,
        "viewAnimations",
        egui::PointerButton::Primary,
    );
    assert!(!editor.view_tabs.open.contains(&DockView::Animations));
    click(
        &mut editor,
        &ctx,
        "viewPlayback",
        egui::PointerButton::Primary,
    );
    assert!(!editor.view_tabs.open.contains(&DockView::Playback));
    click(
        &mut editor,
        &ctx,
        "viewGhosting",
        egui::PointerButton::Primary,
    );
    assert!(!editor.view_tabs.open.contains(&DockView::Ghosting));
    click(
        &mut editor,
        &ctx,
        "viewTimeline",
        egui::PointerButton::Primary,
    );
    assert!(!editor.show_timeline);
    click(&mut editor, &ctx, "viewGraph", egui::PointerButton::Primary);
    assert!(!editor.has_dock_views());
    click(
        &mut editor,
        &ctx,
        "viewMetrics",
        egui::PointerButton::Primary,
    );
    assert_eq!(editor.view_tabs.active, DockView::Metrics);
    assert!(editor.has_dock_views());
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
}

#[test]
fn animation_and_skin_panels_activate_without_redirecting_typed_selection_or_document_history() {
    let mut editor = editor_fixture();
    let original = editor.session.document().clone();
    let selection = editor.selection.clone();
    let ctx = editor_context();
    editor.open_dock_view(DockView::Animations);
    settle(&mut editor, &ctx);
    click(
        &mut editor,
        &ctx,
        "animationsRow:1",
        egui::PointerButton::Primary,
    );
    assert_eq!(editor.animation, 1);
    assert_eq!(editor.time, 0.);
    assert_eq!(editor.selection, selection);
    click(
        &mut editor,
        &ctx,
        "animationsRow:0",
        egui::PointerButton::Secondary,
    );
    assert_eq!(editor.animation, 1);
    assert_eq!(editor.selection, Some(TreeItem::Animation(0)));
    editor.open_dock_view(DockView::Skins);
    settle(&mut editor, &ctx);
    click(
        &mut editor,
        &ctx,
        "skinsRow:mint",
        egui::PointerButton::Primary,
    );
    assert_eq!(editor.skin, "mint");
    assert_eq!(editor.selection, Some(TreeItem::Animation(0)));
    click(
        &mut editor,
        &ctx,
        "skinsRow:peach",
        egui::PointerButton::Secondary,
    );
    assert_eq!(editor.skin, "mint");
    assert_eq!(editor.selection, Some(TreeItem::Skin("peach".into())));
    click(
        &mut editor,
        &ctx,
        "skinsClear",
        egui::PointerButton::Primary,
    );
    assert_eq!(editor.skin, "default");
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
    assert!(!editor.session.is_dirty());
}

#[test]
fn f9_round_trip_preserves_all_views_and_each_view_can_close_and_reopen() {
    let mut editor = editor_fixture();
    let ctx = editor_context();
    for view in DockView::ALL {
        editor.open_dock_view(view);
    }
    let original = editor.view_tabs.open.clone();
    editor.toggle_views();
    assert!(!editor.show_tree && !editor.show_dopesheet && !editor.show_weights);
    assert!(!editor.has_dock_views());
    editor.toggle_views();
    assert!(editor.show_tree && editor.show_dopesheet && editor.show_weights);
    assert_eq!(editor.view_tabs.open, original);
    editor.open_dock_view(DockView::Metrics);
    settle(&mut editor, &ctx);
    click(
        &mut editor,
        &ctx,
        "closeView:Metrics",
        egui::PointerButton::Primary,
    );
    assert!(!editor.view_tabs.open.contains(&DockView::Metrics));
    click(
        &mut editor,
        &ctx,
        "closeView:Tree",
        egui::PointerButton::Primary,
    );
    assert!(!editor.show_tree);
    click(&mut editor, &ctx, "viewsMenu", egui::PointerButton::Primary);
    click(&mut editor, &ctx, "viewTree", egui::PointerButton::Primary);
    assert!(editor.show_tree);
    click(
        &mut editor,
        &ctx,
        "closeView:Dopesheet",
        egui::PointerButton::Primary,
    );
    assert!(!editor.show_dopesheet);
    click(&mut editor, &ctx, "viewsMenu", egui::PointerButton::Primary);
    click(
        &mut editor,
        &ctx,
        "viewDopesheet",
        egui::PointerButton::Primary,
    );
    assert!(editor.show_dopesheet);
    editor.toggle_views();
    editor.open_weights_view();
    assert!(editor.show_weights && editor.show_tree && editor.show_dopesheet);
    assert_eq!(editor.view_tabs.open.len(), 6);
    assert!(editor.hidden_views.is_none());
}

#[test]
fn outline_centers_main_view_without_editing_pose_and_animations_tab_survives_setup() {
    let mut editor = editor_fixture();
    let mut local = editor.session.local_pose(Some(0), editor.time)[8];
    local.rotation += 10.;
    editor
        .session
        .edit_pose(
            8,
            EditTarget::Animate {
                animation: 0,
                time: editor.time,
            },
            local,
            &[Property::Rotation],
            false,
        )
        .unwrap();
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let time = editor.time;
    editor.open_dock_view(DockView::Outline);
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    let before = editor.pan;
    click(
        &mut editor,
        &ctx,
        "outlineViewport",
        egui::PointerButton::Primary,
    );
    assert_ne!(editor.pan, before);
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.time, time);
    editor.open_dock_view(DockView::Animations);
    editor.set_setup_mode(true);
    settle(&mut editor, &ctx);
    assert!(editor.view_tabs.open.contains(&DockView::Animations));
    assert!(!editor.visible_dock_views().contains(&DockView::Animations));
    editor.set_setup_mode(false);
    assert!(editor.visible_dock_views().contains(&DockView::Animations));
}

#[test]
fn playback_controls_work_without_dopesheet_and_retain_workspace_and_preview_speed() {
    let mut editor = editor_fixture();
    let original = editor.session.document().clone();
    let selection = editor.selection.clone();
    let ctx = editor_context();
    editor.show_dopesheet = false;
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "viewsMenu", egui::PointerButton::Primary);
    click(
        &mut editor,
        &ctx,
        "viewPlayback",
        egui::PointerButton::Primary,
    );
    assert_eq!(editor.view_tabs.active, DockView::Playback);
    assert!(!editor.show_dopesheet);
    click(
        &mut editor,
        &ctx,
        "playbackSpeed:50",
        egui::PointerButton::Primary,
    );
    assert_eq!(editor.playback_speed, 0.5);
    click(
        &mut editor,
        &ctx,
        "playbackForward",
        egui::PointerButton::Primary,
    );
    assert!(editor.playing);
    assert_eq!(editor.play_direction, PlayDirection::Forward);
    click(
        &mut editor,
        &ctx,
        "playbackBackward",
        egui::PointerButton::Primary,
    );
    assert!(editor.playing);
    assert_eq!(editor.play_direction, PlayDirection::Backward);
    click(
        &mut editor,
        &ctx,
        "playbackBackward",
        egui::PointerButton::Primary,
    );
    assert!(!editor.playing);
    click(
        &mut editor,
        &ctx,
        "playbackLoop",
        egui::PointerButton::Primary,
    );
    assert!(!editor.looping);
    click(
        &mut editor,
        &ctx,
        "playbackStart",
        egui::PointerButton::Primary,
    );
    assert_eq!(editor.time, 0.);
    click(
        &mut editor,
        &ctx,
        "playbackNextFrame",
        egui::PointerButton::Primary,
    );
    assert!((editor.time - 1. / original.fps as f32).abs() < 0.000001);
    click(
        &mut editor,
        &ctx,
        "playbackPreviousFrame",
        egui::PointerButton::Primary,
    );
    assert_eq!(editor.time, 0.);
    click(
        &mut editor,
        &ctx,
        "playbackNextKey",
        egui::PointerButton::Primary,
    );
    assert_eq!(editor.time, 0.2);
    click(
        &mut editor,
        &ctx,
        "playbackPreviousKey",
        egui::PointerButton::Primary,
    );
    assert_eq!(editor.time, 0.);
    click(
        &mut editor,
        &ctx,
        "playbackEnd",
        egui::PointerButton::Primary,
    );
    assert_eq!(editor.time, editor.animation_end());
    editor.set_setup_mode(true);
    settle(&mut editor, &ctx);
    assert!(editor.view_tabs.open.contains(&DockView::Playback));
    assert!(!editor.visible_dock_views().contains(&DockView::Playback));
    editor.toggle_views();
    editor.toggle_views();
    editor.set_setup_mode(false);
    settle(&mut editor, &ctx);
    assert!(editor.visible_dock_views().contains(&DockView::Playback));
    click(
        &mut editor,
        &ctx,
        "closeView:Playback",
        egui::PointerButton::Primary,
    );
    assert!(!editor.view_tabs.open.contains(&DockView::Playback));
    click(&mut editor, &ctx, "viewsMenu", egui::PointerButton::Primary);
    click(
        &mut editor,
        &ctx,
        "viewPlayback",
        egui::PointerButton::Primary,
    );
    assert_eq!(editor.playback_speed, 0.5);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::F9,
            physical_key: Some(egui::Key::F9),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(editor.hidden_views.is_some());
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::F9,
            physical_key: Some(egui::Key::F9),
            pressed: false,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::F9,
            physical_key: Some(egui::Key::F9),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    settle(&mut editor, &ctx);
    assert!(editor.hidden_views.is_none());
    assert_eq!(editor.selection, selection);
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.is_dirty());
    assert!(!editor.session.can_undo());
}

#[test]
fn playback_numeric_speed_commits_real_text_without_editing_animation_or_pose() {
    let mut editor = editor_fixture();
    let ctx = editor_context();
    editor.open_dock_view(DockView::Playback);
    let original = editor.session.document().clone();
    let mut pose = editor.selected_transform().unwrap();
    pose.rotation += 11.;
    editor.edit_transform(pose, &[Property::Rotation]);
    let pending = editor.session.pose_edits().clone();
    settle(&mut editor, &ctx);
    click(
        &mut editor,
        &ctx,
        "playbackSpeedValue",
        egui::PointerButton::Primary,
    );
    assert!(ctx.text_edit_focused());
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("37".into())]);
    assert_eq!(editor.playback_speed, 1.);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: Some(egui::Key::Enter),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert_eq!(editor.playback_speed, 0.37);
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(!editor.session.is_dirty());
    click(
        &mut editor,
        &ctx,
        "playbackSpeed:50",
        egui::PointerButton::Primary,
    );
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::D,
            physical_key: Some(egui::Key::D),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(editor.playing);
    assert!(editor.session.pose_edits().is_empty());
}

#[test]
fn view_close_controls_fit_the_minimum_workspace() {
    let mut editor = editor_fixture();
    editor.open_dock_view(DockView::Playback);
    let ctx = editor_context();
    for _ in 0..3 {
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
    for (view, clip) in [("Tree", "tree"), ("Dopesheet", "dopesheetClip")] {
        let [left, top, right, bottom] = editor.capture_targets[clip];
        let clip = egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom));
        let [left, top, right, bottom] = editor.capture_targets[&format!("closeView:{view}")];
        let close = egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom));
        assert!(
            clip.contains_rect(close),
            "{view}: {close:?} outside {clip:?}"
        );
    }
    let [left, top, right, bottom] = editor.capture_targets["playbackPanelClip"];
    let clip = egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom));
    for name in [
        "playbackSpeedValue",
        "playbackForward",
        "playbackEnd",
        "playbackPreviousKey",
        "playbackNextKey",
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

#[test]
fn opening_another_document_retains_tabs_but_clears_searches_and_fits_its_outline() {
    let mut editor = editor_fixture();
    editor.open_dock_view(DockView::Skins);
    editor.open_dock_view(DockView::Outline);
    let open = editor.view_tabs.open.clone();
    editor.view_tabs.animation_query = "old animation".into();
    editor.view_tabs.skin_query = "old skin".into();
    editor.view_tabs.outline_zoom = 3.;
    editor.view_tabs.outline_pan = vec2(500., 100.);
    editor.view_tabs.outline_fit = false;
    let mut document = editor.session.document().clone();
    document.bones[0].setup.x += 1000.;
    editor.session = Session::new(document).unwrap();
    editor.view_tabs.reset_document();
    assert_eq!(editor.view_tabs.open, open);
    assert_eq!(editor.view_tabs.active, DockView::Outline);
    assert!(editor.view_tabs.animation_query.is_empty());
    assert!(editor.view_tabs.skin_query.is_empty());
    assert!(editor.view_tabs.outline_fit);
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    assert!(!editor.view_tabs.outline_fit);
    let bounds = editor.scene_for_view().unwrap().bounds.unwrap();
    let expected_x = -bounds.center()[0] * editor.view_tabs.outline_zoom;
    assert!((editor.view_tabs.outline_pan.x - expected_x).abs() < 0.001);
}
