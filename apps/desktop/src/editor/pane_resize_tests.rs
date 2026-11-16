//! Actual workspace divider input, layout persistence and transient-pose safety.
use super::super::test_support::{editor_context, editor_fixture};
use super::*;

fn frame(editor: &mut Editor, ctx: &egui::Context, size: Vec2, events: Vec<egui::Event>) {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, size)),
            focused: true,
            events,
            ..Default::default()
        },
        |ui| editor.workspace_ui(ui),
    );
    output.textures_delta.clear();
}

fn target(editor: &Editor, name: &str) -> egui::Rect {
    let [left, top, right, bottom] = editor.capture_targets[name];
    egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom))
}

fn settle(editor: &mut Editor, ctx: &egui::Context, size: Vec2) {
    let count = (ctx.global_style().animation_time / ctx.input(|input| input.predicted_dt)).ceil()
        as usize
        + 2;
    for _ in 0..count {
        frame(editor, ctx, size, vec![]);
    }
}

fn pointer(point: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

fn ready(pane: &str, size: Vec2) -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    editor.show_dopesheet = false;
    editor.show_tree = pane == "tree";
    editor.show_weights = pane == "weights-pane";
    if pane == "curves-view" {
        editor.open_curves_view();
    }
    let mut pose = editor
        .session
        .local_pose(Some(editor.animation), editor.time)[8];
    pose.rotation += 13.5;
    editor
        .session
        .edit_pose(
            8,
            EditTarget::Animate {
                animation: editor.animation,
                time: editor.time,
            },
            pose,
            &[Property::Rotation],
            false,
        )
        .unwrap();
    editor
        .session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo marker".into(),
        })
        .unwrap();
    editor.session.undo();
    let ctx = editor_context();
    settle(&mut editor, &ctx, size);
    (editor, ctx)
}

fn drag_pane(pane: &str, offset: f32) {
    let size = vec2(1500., 960.);
    let (mut editor, ctx) = ready(pane, size);
    let document = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let selection = editor.selection.clone();
    let history = (editor.session.can_undo(), editor.session.can_redo());
    assert!(history.1);
    let panel_id = egui::Id::new(pane);
    let resize_id = panel_id.with("__resize");
    let before = egui::containers::panel::PanelState::load(&ctx, panel_id)
        .unwrap()
        .outer_rect;
    let viewport = target(&editor, "viewport");
    let handle = ctx.read_response(resize_id).unwrap();
    let start = pos2(handle.rect.center().x + offset, viewport.center().y);
    assert!(handle.interact_rect.contains(start));
    frame(
        &mut editor,
        &ctx,
        size,
        vec![egui::Event::PointerMoved(start)],
    );
    frame(&mut editor, &ctx, size, vec![pointer(start, true)]);
    frame(&mut editor, &ctx, size, vec![]);
    assert!(
        ctx.read_response(resize_id)
            .unwrap()
            .is_pointer_button_down_on(),
        "{pane} at {offset}: divider did not own the press"
    );
    for delta in [8., 40., 120.] {
        frame(
            &mut editor,
            &ctx,
            size,
            vec![egui::Event::PointerMoved(start - vec2(delta, 0.))],
        );
        assert_eq!(ctx.dragged_id(), Some(resize_id), "{pane} at {offset}");
        assert!(!editor.viewport_transform_active());
        assert_eq!(editor.session.document(), &document);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(editor.selection, selection);
        assert_eq!(
            (editor.session.can_undo(), editor.session.can_redo()),
            history
        );
    }
    assert!(target(&editor, "viewport").right() < viewport.right() - 20.);
    frame(
        &mut editor,
        &ctx,
        size,
        vec![pointer(start - vec2(120., 0.), false)],
    );
    let after = egui::containers::panel::PanelState::load(&ctx, panel_id)
        .unwrap()
        .outer_rect;
    assert!(after.width() > before.width() + 20., "{pane} at {offset}");
    let viewport_after = target(&editor, "viewport");
    for _ in 0..2 {
        frame(&mut editor, &ctx, size, vec![]);
    }
    assert_eq!(
        egui::containers::panel::PanelState::load(&ctx, panel_id)
            .unwrap()
            .outer_rect,
        after
    );
    assert_eq!(target(&editor, "viewport"), viewport_after);
    assert_eq!(editor.session.document(), &document);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.selection, selection);
    assert_eq!(
        (editor.session.can_undo(), editor.session.can_redo()),
        history
    );
}

#[test]
fn pane_resize_tree_grab_band_preserves_selection_pending_and_history() {
    for offset in [-2., 0., 2.] {
        drag_pane("tree", offset);
    }
}

#[test]
fn pane_resize_weights_only_grab_band_preserves_selection_pending_and_history() {
    for offset in [-2., 0., 2.] {
        drag_pane("weights-pane", offset);
    }
}

#[test]
fn pane_resize_curves_only_grab_band_preserves_selection_pending_and_history() {
    for offset in [-2., 0., 2.] {
        drag_pane("curves-view", offset);
    }
}

#[test]
fn pane_resize_reserved_band_does_not_interrupt_an_owned_canvas_drag() {
    let size = vec2(1500., 960.);
    let (mut editor, ctx) = ready("tree", size);
    editor.set_tool(Tool::Move);
    settle(&mut editor, &ctx, size);
    let document = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let selection = editor.selection.clone();
    assert!(editor.session.can_redo());
    let panel_id = egui::Id::new("tree");
    let panel = egui::containers::panel::PanelState::load(&ctx, panel_id)
        .unwrap()
        .outer_rect;
    let viewport = target(&editor, "viewport");
    // Empty viewport space starts a free Move of the already selected bone.
    let start = pos2(viewport.right() - 120., viewport.center().y);
    frame(
        &mut editor,
        &ctx,
        size,
        vec![egui::Event::PointerMoved(start)],
    );
    frame(&mut editor, &ctx, size, vec![pointer(start, true)]);
    frame(
        &mut editor,
        &ctx,
        size,
        vec![egui::Event::PointerMoved(start + vec2(12., 0.))],
    );
    let owner = egui::Id::new("viewport-input");
    assert_eq!(ctx.dragged_id(), Some(owner));
    assert!(editor.viewport_transform_active());
    assert_eq!(editor.selection, selection);
    assert_ne!(editor.session.pose_edits(), &pending);
    let mut previous = editor.session.pose_edits().clone();
    let end = pos2(viewport.right() + 20., start.y);
    for point in [pos2(viewport.right() - 1., start.y), end] {
        frame(
            &mut editor,
            &ctx,
            size,
            vec![egui::Event::PointerMoved(point)],
        );
        assert!(!target(&editor, "viewportInput").contains(point));
        assert_eq!(ctx.dragged_id(), Some(owner));
        assert!(editor.viewport_transform_active());
        assert_ne!(editor.session.pose_edits(), &previous);
        previous = editor.session.pose_edits().clone();
        assert_eq!(editor.session.document(), &document);
        assert_eq!(editor.selection, selection);
        // A changed transaction can be undone; the old redo branch is not
        // available during its preview and is discarded when it commits.
        assert!(editor.session.can_undo());
        assert!(!editor.session.can_redo());
        assert_eq!(
            egui::containers::panel::PanelState::load(&ctx, panel_id)
                .unwrap()
                .outer_rect,
            panel
        );
        assert_eq!(target(&editor, "viewport"), viewport);
    }
    frame(&mut editor, &ctx, size, vec![pointer(end, false)]);
    assert!(!editor.viewport_transform_active());
    assert_ne!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.session.document(), &document);
    let moved = editor.session.pose_edits().clone();
    assert!(!editor.session.can_redo());
    editor.undo();
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.session.document(), &document);
    assert_eq!(editor.selection, selection);
    assert!(editor.session.can_redo());
    editor.redo();
    assert_eq!(editor.session.pose_edits(), &moved);
    assert_eq!(editor.session.document(), &document);
    assert_eq!(editor.selection, selection);
    assert_eq!(
        egui::containers::panel::PanelState::load(&ctx, panel_id)
            .unwrap()
            .outer_rect,
        panel
    );
}

#[test]
fn pane_resize_input_reservation_keeps_projection_and_minimum_viewport_size() {
    let size = vec2(1120., 740.);
    let (mut editor, ctx) = ready("tree", size);
    editor.show_weights = true;
    editor.open_curves_view();
    settle(&mut editor, &ctx, size);
    let viewport = target(&editor, "viewport");
    let input = target(&editor, "viewportInput");
    assert!(viewport.width() >= 320.);
    assert!(input.is_positive() && input.is_finite());
    assert_eq!(input.min, viewport.min);
    assert_eq!(input.bottom(), viewport.bottom());
    assert!(input.right() < viewport.right());
    let id = egui::Id::new("viewport-input");
    assert_eq!(ctx.read_response(id).unwrap().rect, input);
    let zoom = editor.zoom;
    let pan = editor.pan;
    editor.show_weights = false;
    editor.show_tree = false;
    editor.view_tabs.open.clear();
    settle(&mut editor, &ctx, size);
    assert_eq!(
        target(&editor, "viewportInput"),
        target(&editor, "viewport")
    );
    assert_eq!(ctx.read_response(id).unwrap().id, id);
    assert_eq!(editor.zoom, zoom);
    assert_eq!(editor.pan, pan);
}
