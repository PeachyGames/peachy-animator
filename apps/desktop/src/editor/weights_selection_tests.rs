use super::super::test_support::{editor_context, mesh_editor};
use super::*;

fn frame(
    editor: &mut Editor,
    ctx: &egui::Context,
    modifiers: egui::Modifiers,
    mut events: Vec<egui::Event>,
) {
    events.insert(0, egui::Event::ModifiersChanged(modifiers));
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
            focused: true,
            events,
            ..Default::default()
        },
        |ui| editor.workspace_ui(ui),
    );
    output.textures_delta.clear();
}
fn ready() -> (Editor, egui::Context, AttachmentRef) {
    let (mut editor, key) = mesh_editor(true);
    let mut doc = editor.session.document().clone();
    let Some(Attachment::Mesh(mesh)) = doc.attachment_mut(&key) else {
        panic!()
    };
    let MeshVertices::Weighted { influences } = &mut mesh.vertices else {
        panic!()
    };
    for (index, weight) in [(0, 0.2), (1, 0.65)] {
        influences[index][0].weight = 1. - weight;
        influences[index][1].weight = weight;
    }
    editor.session = Session::new(doc).unwrap();
    editor.set_tool(Tool::Weights);
    editor.weights.bone = Some(8);
    let ctx = editor_context();
    for _ in 0..4 {
        frame(&mut editor, &ctx, egui::Modifiers::NONE, vec![]);
    }
    (editor, ctx, key)
}
fn rect(editor: &Editor, name: &str) -> egui::Rect {
    let [l, t, r, b] = editor.capture_targets[name];
    egui::Rect::from_min_max(pos2(l, t), pos2(r, b))
}
fn button(
    editor: &mut Editor,
    ctx: &egui::Context,
    point: Pos2,
    pressed: bool,
    modifiers: egui::Modifiers,
) {
    frame(
        editor,
        ctx,
        modifiers,
        vec![
            egui::Event::PointerMoved(point),
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers,
            },
        ],
    );
}
fn click(editor: &mut Editor, ctx: &egui::Context, point: Pos2, modifiers: egui::Modifiers) {
    button(editor, ctx, point, true, modifiers);
    button(editor, ctx, point, false, modifiers);
}
fn moved(editor: &mut Editor, ctx: &egui::Context, point: Pos2, modifiers: egui::Modifiers) {
    frame(
        editor,
        ctx,
        modifiers,
        vec![egui::Event::PointerMoved(point)],
    );
}
fn key(editor: &mut Editor, ctx: &egui::Context, key: egui::Key, modifiers: egui::Modifiers) {
    frame(
        editor,
        ctx,
        modifiers,
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed,
                repeat: false,
                modifiers,
            })
            .collect(),
    );
}
fn weights(editor: &Editor, key: &AttachmentRef) -> Vec<Vec<BoneInfluence>> {
    let Some(Attachment::Mesh(mesh)) = editor.session.document().attachment(key) else {
        panic!()
    };
    let MeshVertices::Weighted { influences } = &mesh.vertices else {
        panic!()
    };
    influences.clone()
}
fn pair(editor: &mut Editor, ctx: &egui::Context) {
    let a = rect(editor, "weightsVertex:0").center();
    click(editor, ctx, a, egui::Modifiers::NONE);
    let b = rect(editor, "weightsVertex:1").center();
    click(editor, ctx, b, egui::Modifiers::COMMAND);
    assert_eq!(editor.weights.vertices, vec![0, 1]);
}
fn empty(editor: &Editor) -> Pos2 {
    let area = rect(editor, "viewport").shrink(20.);
    [
        area.left_top(),
        area.right_top(),
        area.left_bottom(),
        area.right_bottom(),
        area.center(),
    ]
    .into_iter()
    .find(|point| {
        editor
            .weights_handles
            .iter()
            .all(|p| point.distance(pos2(p[0], p[1])) > 30.)
    })
    .unwrap()
}
fn type_percent(editor: &mut Editor, ctx: &egui::Context, text: &str) {
    // Vertex selection changes after the pane has rendered. Draw its new
    // context ID before using last-frame hit testing for the numeric click.
    frame(editor, ctx, egui::Modifiers::NONE, vec![]);
    let point = rect(editor, "weightsWeightValue").center();
    click(editor, ctx, point, egui::Modifiers::NONE);
    // DragValue requests focus on release, then draws its TextEdit next frame.
    frame(editor, ctx, egui::Modifiers::NONE, vec![]);
    assert!(
        ctx.text_edit_focused(),
        "Weight input {text:?} did not focus; field={:?}; focused={:?}; state={}",
        rect(editor, "weightsWeightValue"),
        ctx.memory(|memory| memory.focused()),
        editor.weight_direct_diagnostic()
    );
    key(editor, ctx, egui::Key::A, egui::Modifiers::COMMAND);
    frame(
        editor,
        ctx,
        egui::Modifiers::NONE,
        vec![egui::Event::Text(text.into())],
    );
    key(editor, ctx, egui::Key::Enter, egui::Modifiers::NONE);
}

#[test]
fn weights_selection_click_toggle_select_all_and_empty_click_preserve_document_history() {
    let (mut editor, ctx, key_ref) = ready();
    editor
        .session
        .set_mesh_bone_weight(key_ref.clone(), 0, 8, 0.4, EditTarget::Setup)
        .unwrap();
    editor.session.undo();
    let original = editor.session.document().clone();
    pair(&mut editor, &ctx);
    let a = rect(&editor, "weightsVertex:0").center();
    click(&mut editor, &ctx, a, egui::Modifiers::COMMAND);
    assert_eq!(editor.weights.vertices, vec![1]);
    key(&mut editor, &ctx, egui::Key::A, egui::Modifiers::COMMAND);
    assert_eq!(editor.weights.vertices, vec![0, 1, 2, 3]);
    let point = empty(&editor);
    click(&mut editor, &ctx, point, egui::Modifiers::NONE);
    assert!(editor.weights.vertices.is_empty());
    assert_eq!(editor.weights.mesh, Some(key_ref));
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.can_redo());
}

#[test]
fn weights_selection_cmd_press_survives_modifier_release_and_canceled_vertex_press_cannot_reenter()
{
    for release_before_up in [false, true] {
        let (mut editor, ctx, key_ref) = ready();
        editor
            .session
            .set_mesh_bone_weight(key_ref, 0, 8, 0.4, EditTarget::Setup)
            .unwrap();
        editor.session.undo();
        let original = editor.session.document().clone();
        let a = rect(&editor, "weightsVertex:0").center();
        click(&mut editor, &ctx, a, egui::Modifiers::NONE);
        let b = rect(&editor, "weightsVertex:1").center();
        button(&mut editor, &ctx, b, true, egui::Modifiers::COMMAND);
        if release_before_up {
            frame(&mut editor, &ctx, egui::Modifiers::NONE, vec![]);
        }
        frame(
            &mut editor,
            &ctx,
            egui::Modifiers::NONE,
            vec![
                egui::Event::PointerMoved(b),
                egui::Event::PointerButton {
                    pos: b,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: if release_before_up {
                        egui::Modifiers::NONE
                    } else {
                        egui::Modifiers::COMMAND
                    },
                },
                egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            ],
        );
        assert_eq!(editor.weights.vertices, vec![0, 1]);
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
        let a = rect(&editor, "weightsVertex:0").center();
        click(&mut editor, &ctx, a, egui::Modifiers::NONE);
        let b = rect(&editor, "weightsVertex:1").center();
        button(&mut editor, &ctx, b, true, egui::Modifiers::COMMAND);
        key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
        assert_eq!(editor.weights.vertices, vec![0]);
        button(&mut editor, &ctx, b, false, egui::Modifiers::NONE);
        assert_eq!(editor.weights.vertices, vec![0]);
        frame(&mut editor, &ctx, egui::Modifiers::NONE, vec![]);
        key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
        assert!(editor.weights.vertices.is_empty());
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
    }
}

#[test]
fn weights_selection_mixed_numeric_absolute_edits_pair_and_zero_is_genuine_input() {
    let (mut editor, ctx, key_ref) = ready();
    pair(&mut editor, &ctx);
    let original = editor.session.document().clone();
    let before = weights(&editor, &key_ref);
    assert_eq!(editor.weight_direct_diagnostic()["mixed"], true);
    assert!(editor.weight_direct_diagnostic()["percent"].is_null());
    type_percent(&mut editor, &ctx, "50");
    let after = weights(&editor, &key_ref);
    for vertex in 0..2 {
        assert_eq!(after[vertex][1].weight, 0.5);
        for i in 0..2 {
            assert_eq!(before[vertex][i].position, after[vertex][i].position);
        }
    }
    assert_eq!(&after[2..], &before[2..]);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    frame(&mut editor, &ctx, egui::Modifiers::NONE, vec![]);
    type_percent(&mut editor, &ctx, "0");
    let zero = weights(&editor, &key_ref);
    assert_eq!(zero[0][1].weight, 0.);
    assert_eq!(zero[1][1].weight, 0.);
    assert_eq!(&zero[2..], &before[2..]);
}

#[test]
fn weights_selection_relative_threshold_incremental_clamp_reversal_is_one_undo() {
    let (mut editor, ctx, key_ref) = ready();
    pair(&mut editor, &ctx);
    let original = editor.session.document().clone();
    let start = empty(&editor);
    button(&mut editor, &ctx, start, true, egui::Modifiers::NONE);
    moved(
        &mut editor,
        &ctx,
        start + vec2(10., 0.),
        egui::Modifiers::NONE,
    );
    assert!(editor.weight_edit.is_none());
    assert_eq!(editor.session.document(), &original);
    moved(
        &mut editor,
        &ctx,
        start + vec2(12., 0.),
        egui::Modifiers::NONE,
    );
    assert!(editor.weight_edit.is_some());
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.weight_direct_diagnostic()["formattedPercent"], "");
    moved(
        &mut editor,
        &ctx,
        start + vec2(12., -35.),
        egui::Modifiers::NONE,
    );
    assert_eq!(editor.weight_direct_diagnostic()["mixed"], true);
    assert_eq!(editor.weight_direct_diagnostic()["formattedPercent"], "");
    moved(
        &mut editor,
        &ctx,
        start + vec2(12., -400.),
        egui::Modifiers::NONE,
    );
    assert_eq!(weights(&editor, &key_ref)[0][1].weight, 1.);
    assert_eq!(weights(&editor, &key_ref)[1][1].weight, 1.);
    let end = start + vec2(12., -350.);
    moved(&mut editor, &ctx, end, egui::Modifiers::NONE);
    for vertex in 0..2 {
        assert!((weights(&editor, &key_ref)[vertex][1].weight - 6. / 7.).abs() < 0.000001);
    }
    button(&mut editor, &ctx, end, false, egui::Modifiers::NONE);
    assert!(editor.weight_edit.is_none());
    editor.undo();
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn weights_selection_all_absolute_skips_sole_and_materializes_sparse_binding_through_engine() {
    let (mut editor, ctx, key_ref) = ready();
    let mut doc = editor.session.document().clone();
    let Some(Attachment::Mesh(mesh)) = doc.attachment_mut(&key_ref) else {
        panic!()
    };
    let MeshVertices::Weighted { influences } = &mut mesh.vertices else {
        panic!()
    };
    influences[2][0].weight = 0.;
    influences[2][1].weight = 1.;
    influences[3].retain(|entry| entry.bone != 8);
    influences[3][0].weight = 1.;
    let sole = influences[2].clone();
    editor.session = Session::new(doc).unwrap();
    frame(&mut editor, &ctx, egui::Modifiers::NONE, vec![]);
    key(&mut editor, &ctx, egui::Key::A, egui::Modifiers::COMMAND);
    assert_eq!(editor.weights.vertices, vec![0, 1, 2, 3]);
    let original = editor.session.document().clone();
    type_percent(&mut editor, &ctx, "50");
    let result = weights(&editor, &key_ref);
    assert_eq!(result[2], sole);
    assert_eq!(result[0][1].weight, 0.5);
    assert_eq!(result[1][1].weight, 0.5);
    let added = result[3].iter().find(|entry| entry.bone == 8).unwrap();
    assert_eq!(added.weight, 0.5);
    assert_eq!(added.position, [0., 0.]);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn weights_selection_marquee_add_cancel_and_held_release_preserve_redo() {
    let (mut editor, ctx, key_ref) = ready();
    let selected = rect(&editor, "weightsVertex:3").center();
    click(&mut editor, &ctx, selected, egui::Modifiers::NONE);
    editor
        .session
        .set_mesh_bone_weight(key_ref, 0, 8, 0.4, EditTarget::Setup)
        .unwrap();
    editor.session.undo();
    frame(&mut editor, &ctx, egui::Modifiers::NONE, vec![]);
    let original = editor.session.document().clone();
    let points = editor
        .weights_handles
        .iter()
        .map(|p| pos2(p[0], p[1]))
        .collect::<Vec<_>>();
    let start = pos2(
        points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min) - 20.,
        points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min) - 20.,
    );
    let end = pos2(
        points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max) + 20.,
        points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max) + 20.,
    );
    button(&mut editor, &ctx, start, true, egui::Modifiers::COMMAND);
    moved(&mut editor, &ctx, end, egui::Modifiers::COMMAND);
    assert_eq!(editor.weights.vertices, vec![0, 1, 2, 3]);
    key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert_eq!(editor.weights.vertices, vec![3]);
    assert!(editor.weights.pointer.is_none());
    moved(
        &mut editor,
        &ctx,
        end + vec2(-20., -20.),
        egui::Modifiers::NONE,
    );
    button(&mut editor, &ctx, end, false, egui::Modifiers::NONE);
    assert_eq!(editor.weights.vertices, vec![3]);
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.can_redo());
    let point = empty(&editor);
    click(&mut editor, &ctx, point, egui::Modifiers::NONE);
    button(&mut editor, &ctx, start, true, egui::Modifiers::NONE);
    moved(&mut editor, &ctx, end, egui::Modifiers::NONE);
    button(&mut editor, &ctx, end, false, egui::Modifiers::NONE);
    assert_eq!(editor.weights.vertices, vec![0, 1, 2, 3]);
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.can_redo());
}

#[test]
fn weights_selection_relative_escape_and_focus_loss_preserve_pending_pose_and_context() {
    for lose_focus in [false, true] {
        let (mut editor, ctx, key_ref) = ready();
        editor.set_setup_mode(false);
        editor.set_time(0.37);
        let mut transform = editor.session.local_pose(Some(0), 0.37)[8];
        transform.rotation += 13.5;
        editor
            .session
            .edit_pose(
                8,
                editor.weights_edit_target(),
                transform,
                &[Property::Rotation],
                false,
            )
            .unwrap();
        for _ in 0..3 {
            frame(&mut editor, &ctx, egui::Modifiers::NONE, vec![]);
        }
        pair(&mut editor, &ctx);
        let original = editor.session.document().clone();
        let pending = editor.session.pose_edits().clone();
        let start = empty(&editor);
        button(&mut editor, &ctx, start, true, egui::Modifiers::NONE);
        moved(
            &mut editor,
            &ctx,
            start + vec2(12., 0.),
            egui::Modifiers::NONE,
        );
        moved(
            &mut editor,
            &ctx,
            start + vec2(12., -35.),
            egui::Modifiers::NONE,
        );
        assert_ne!(editor.session.document(), &original);
        if lose_focus {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
                    focused: false,
                    events: vec![egui::Event::WindowFocused(false)],
                    ..Default::default()
                },
                |ui| editor.workspace_ui(ui),
            );
            output.textures_delta.clear();
        } else {
            key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
        }
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(editor.weights.vertices, vec![0, 1]);
        assert_eq!(editor.weights.mesh, Some(key_ref));
        assert!(editor.weight_edit.is_none() && editor.weights.pointer.is_none());
        button(
            &mut editor,
            &ctx,
            start + vec2(12., -35.),
            false,
            egui::Modifiers::NONE,
        );
        assert_eq!(editor.session.document(), &original);
    }
}
