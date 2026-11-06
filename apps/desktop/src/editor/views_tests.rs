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
    assert_eq!(editor.view_tabs.open.len(), 3);
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
fn view_close_controls_fit_the_minimum_workspace() {
    let mut editor = editor_fixture();
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
