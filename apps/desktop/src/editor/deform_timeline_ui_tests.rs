use super::super::key_drag::{ChartGeometry, KeyDrag};
use super::super::test_support::{editor_context, mesh_editor, workspace_frame};
use super::*;

fn fixture(weighted: bool) -> (Editor, AttachmentRef, egui::Context) {
    let (mut editor, attachment) = mesh_editor(weighted);
    let mut doc = editor.session.document().clone();
    let length = doc
        .attachment(&attachment)
        .unwrap()
        .vertices()
        .unwrap()
        .deform_len();
    doc.animations[0].deform_tracks = vec![DeformTrack {
        attachment: attachment.clone(),
        keys: [0., 0.3, 0.300005, 1.]
            .into_iter()
            .enumerate()
            .map(|(index, time)| DeformKey {
                time,
                curve: Curve::Bezier([0.42, 0., 0.58, 1.]),
                offsets: vec![index as f32; length],
            })
            .collect(),
    }];
    editor.session = Session::new(doc).unwrap();
    editor.set_setup_mode(false);
    editor.time = 0.3;
    editor.reconcile_selection();
    let ctx = editor_context();
    (editor, attachment, ctx)
}

fn key(attachment: &AttachmentRef, time: f32) -> AnimationKey {
    AnimationKey::Deform {
        attachment: attachment.clone(),
        time,
    }
}

fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..3 {
        workspace_frame(editor, ctx, vec![]);
    }
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

fn target(editor: &Editor, name: &str) -> Pos2 {
    let [left, top, right, bottom] = editor.capture_targets[name];
    pos2((left + right) * 0.5, (top + bottom) * 0.5)
}

fn graph_fixture() -> (Editor, AttachmentRef, egui::Context) {
    let (mut editor, attachment, ctx) = fixture(true);
    editor.key_selection.replace(vec![key(&attachment, 0.3)]);
    editor.show_tree = false;
    editor.open_graph_view();
    settle(&mut editor, &ctx);
    (editor, attachment, ctx)
}

#[test]
fn deform_dopesheet_rows_clipboard_and_delete_use_mesh_identity() {
    for weighted in [false, true] {
        let (mut editor, attachment, ctx) = fixture(weighted);
        settle(&mut editor, &ctx);
        assert!(editor.capture_targets.contains_key("timelineRow:deform"));
        assert!(editor.capture_targets.contains_key("timelineKey:deform:2"));
        assert!(
            !editor
                .capture_targets
                .contains_key("timelineKey:Rotation:0")
        );
        assert_eq!(
            editor.timeline_owner(),
            Some(KeyOwner::Deform {
                attachment: attachment.clone()
            })
        );
        editor
            .key_selection
            .select(key(&attachment, 0.300005), false);
        assert!(editor.copy_dopesheet_keys(&ctx));
        let Some(peachy_engine::AnimationKeyClipboard::Deform { keys, .. }) =
            editor.timeline_clipboard.as_ref()
        else {
            panic!("Expected dense mesh clipboard");
        };
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].offsets[0], 2.);
        editor.set_time(1.5);
        assert!(editor.paste_dopesheet_keys());
        assert_eq!(editor.key_selection.keys(), &[key(&attachment, 1.5)]);
        let pasted = editor.session.document().clone();
        editor.delete_dopesheet_keys();
        assert!(
            !editor.session.document().animations[0].deform_tracks[0]
                .keys
                .iter()
                .any(|key| key.time == 1.5)
        );
        editor.undo();
        assert_eq!(editor.session.document(), &pasted);
        editor.select(TreeItem::Bone(8));
        assert!(editor.key_selection.is_empty());
        assert!(!editor.paste_dopesheet_keys());
    }
}

#[test]
fn deform_dopesheet_drag_cancel_restores_offsets_selection_pending_pose_and_redo() {
    let (mut editor, attachment, _) = fixture(true);
    let mut pose = editor.session.local_pose(Some(0), editor.time)[8];
    pose.rotation += 7.;
    editor
        .session
        .edit_pose(
            8,
            EditTarget::Animate {
                animation: 0,
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
            name: "redo".into(),
        })
        .unwrap();
    editor.session.undo();
    assert!(editor.session.can_redo());
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let selected = key(&attachment, 0.300005);
    editor.key_selection.replace(vec![selected.clone()]);
    editor.key_drag = KeyDrag::new(
        0,
        selected.clone(),
        std::slice::from_ref(&selected),
        ChartGeometry {
            start: 0.,
            width: 300.,
            extent: 3.,
        },
        30.,
        egui::Modifiers::NONE,
    );
    editor.update_dopesheet_drag(80., true);
    assert!(
        editor.session.document() != &original,
        "Expected changed curve"
    );
    assert_eq!(editor.session.pose_edits(), &pending);
    editor.cancel_dopesheet_drag();
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.key_selection.keys(), &[selected]);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(editor.session.can_redo());
}

#[test]
fn deform_interpolation_uses_exact_key_and_preserves_offsets_and_pending_pose() {
    let (mut editor, attachment, ctx) = fixture(false);
    let mut pose = editor.session.local_pose(Some(0), editor.time)[8];
    pose.rotation += 7.;
    editor
        .session
        .edit_pose(
            8,
            EditTarget::Animate {
                animation: 0,
                time: editor.time,
            },
            pose,
            &[Property::Rotation],
            false,
        )
        .unwrap();
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    editor.set_deform_key_curve(attachment.clone(), 0.3, Curve::Stepped);
    let keys = &editor.session.document().animations[0].deform_tracks[0].keys;
    assert_eq!(keys[1].curve, Curve::Stepped);
    assert_eq!(keys[2], original.animations[0].deform_tracks[0].keys[2]);
    assert_eq!(
        keys[1].offsets,
        original.animations[0].deform_tracks[0].keys[1].offsets
    );
    assert_eq!(editor.session.pose_edits(), &pending);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    settle(&mut editor, &ctx);
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.can_redo());
}

#[test]
fn deform_graph_handle_drag_includes_release_and_is_one_undo_step() {
    let (mut editor, attachment, ctx) = graph_fixture();
    let original = editor.session.document().clone();
    let start = target(&editor, "graphDeformHandle:0");
    pointer(&mut editor, &ctx, start, true);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(start + vec2(18., -12.))],
    );
    assert!(editor.deform_curve_edit_active());
    pointer(&mut editor, &ctx, start + vec2(32., -24.), false);
    settle(&mut editor, &ctx);
    assert!(!editor.deform_curve_edit_active());
    let changed = editor.session.document().clone();
    assert_ne!(changed, original);
    let after = &changed.animations[0].deform_tracks[0].keys[1];
    assert_eq!(
        after.offsets,
        original.animations[0].deform_tracks[0].keys[1].offsets
    );
    assert_eq!(
        editor.timeline_owner(),
        Some(KeyOwner::Deform { attachment })
    );
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.can_undo());
    editor.redo();
    assert_eq!(editor.session.document(), &changed);
}

#[test]
fn deform_graph_escape_and_context_change_cancel_held_preview_without_rearming() {
    let (mut editor, _, ctx) = graph_fixture();
    let original = editor.session.document().clone();
    let start = target(&editor, "graphDeformHandle:0");
    pointer(&mut editor, &ctx, start, true);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(start + vec2(20., -15.))],
    );
    assert!(
        editor.session.document() != &original,
        "Expected changed curve"
    );
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: Some(egui::Key::Escape),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.deform_curve_edit_active());
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(start + vec2(50., -30.))],
    );
    assert_eq!(editor.session.document(), &original);
    pointer(&mut editor, &ctx, start + vec2(50., -30.), false);
    settle(&mut editor, &ctx);
    let start = target(&editor, "graphDeformHandle:0");
    pointer(&mut editor, &ctx, start, true);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(start + vec2(20., -15.))],
    );
    editor.select(TreeItem::Bone(8));
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.deform_curve_edit_active());
}

#[test]
fn animation_graph_and_dopesheet_share_full_width_bottom_tabs_and_restore_visibility() {
    let (mut editor, _, ctx) = fixture(false);
    editor.show_tree = true;
    editor.open_graph_view();
    settle(&mut editor, &ctx);
    assert!(editor.graph_visible() && !editor.dopesheet_visible());
    assert!(
        editor
            .capture_targets
            .contains_key("animationViewTab:Graph")
    );
    assert!(
        editor
            .capture_targets
            .contains_key("animationViewTab:Dopesheet")
    );
    assert!(!editor.capture_targets.contains_key("viewTab:Graph"));
    let [left, top, right, _] = editor.capture_targets["animationViews"];
    assert!(left < 12. && right > 1488.);
    assert!(editor.capture_targets["tree"][3] <= top);
    let pos = target(&editor, "animationViewTab:Dopesheet");
    pointer(&mut editor, &ctx, pos, true);
    pointer(&mut editor, &ctx, pos, false);
    settle(&mut editor, &ctx);
    assert!(editor.dopesheet_visible() && !editor.graph_visible());
    assert!(ctx.memory(|memory| memory.has_focus(Editor::dopesheet_id())));
    editor.open_graph_view();
    editor.toggle_views();
    assert!(!editor.graph_visible() && !editor.animation_views_open());
    editor.toggle_views();
    assert!(editor.graph_visible());
    editor.set_setup_mode(true);
    settle(&mut editor, &ctx);
    assert!(!editor.graph_visible() && !editor.animation_views_open());
    editor.set_setup_mode(false);
    settle(&mut editor, &ctx);
    assert!(editor.graph_visible());
}

#[test]
fn animation_bottom_dock_preserves_graph_plot_and_viewport_at_minimum_window() {
    let (mut editor, _, _) = fixture(true);
    editor.open_graph_view();
    let ctx = editor_context();
    for _ in 0..4 {
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
    let [left, top, right, bottom] = editor.capture_targets["viewport"];
    assert!(
        right - left > 400. && bottom - top >= 100.,
        "viewport {:?}",
        [left, top, right, bottom]
    );
    let plot = editor.capture_targets["graphPlot"];
    assert!(
        plot[2] - plot[0] > 700. && plot[3] - plot[1] >= 70.,
        "plot {plot:?}"
    );
    let panel = editor.capture_targets["animationViews"];
    assert!(
        plot[0] >= panel[0] && plot[2] <= panel[2] && plot[1] >= panel[1] && plot[3] <= panel[3]
    );
    assert!(bottom <= panel[1]);
}

#[test]
fn deform_graph_frame_positions_cover_every_interval_and_exact_key_points_select() {
    let (mut editor, attachment, ctx) = graph_fixture();
    let start = target(&editor, "graphDeformKeyPoint:0");
    let last = target(&editor, "graphDeformKeyPoint:3");
    let middle = target(&editor, "graphDeformKeyPoint:1");
    assert!(last.x > middle.x && middle.x > start.x);
    let ratio = (middle.x - start.x) / (last.x - start.x);
    assert!(
        (ratio - 0.3).abs() < 0.001,
        "Keys are positioned by clip time, not interval fraction: {ratio}"
    );
    assert!(editor.graph.deform.rendered_points >= 123);
    pointer(&mut editor, &ctx, last, true);
    pointer(&mut editor, &ctx, last, false);
    settle(&mut editor, &ctx);
    assert_eq!(editor.time, 1.);
    assert_eq!(editor.key_selection.keys(), &[key(&attachment, 1.)]);
}

#[test]
fn deform_graph_zoom_keys_makes_close_fractional_keys_editable_without_aliasing() {
    let (mut editor, attachment, ctx) = graph_fixture();
    let zoom = target(&editor, "graphZoomKeys");
    pointer(&mut editor, &ctx, zoom, true);
    pointer(&mut editor, &ctx, zoom, false);
    settle(&mut editor, &ctx);
    let from = target(&editor, "graphDeformKeyPoint:1");
    let to = target(&editor, "graphDeformKeyPoint:2");
    assert!(to.x - from.x > 100.);
    let before = editor.session.document().clone();
    let handle = target(&editor, "graphDeformHandle:0");
    pointer(&mut editor, &ctx, handle, true);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(handle + vec2(14., -10.))],
    );
    pointer(&mut editor, &ctx, handle + vec2(20., -18.), false);
    settle(&mut editor, &ctx);
    let keys = &editor.session.document().animations[0].deform_tracks[0].keys;
    assert_ne!(
        keys[1].curve,
        before.animations[0].deform_tracks[0].keys[1].curve
    );
    assert_eq!(keys[2], before.animations[0].deform_tracks[0].keys[2]);
    assert_eq!(
        editor.timeline_owner(),
        Some(KeyOwner::Deform { attachment })
    );
}

#[test]
fn deform_graph_numeric_percentage_entry_changes_curve_without_rewriting_offsets() {
    let (mut editor, _, ctx) = graph_fixture();
    let open = target(&editor, "graphDeformHandles");
    pointer(&mut editor, &ctx, open, true);
    pointer(&mut editor, &ctx, open, false);
    // The disclosure animates its height before it becomes a stable edit surface.
    for _ in 0..20 {
        workspace_frame(&mut editor, &ctx, vec![]);
    }
    let before = editor.session.document().clone();
    let value = target(&editor, "graphDeformValue:1");
    pointer(&mut editor, &ctx, value, true);
    pointer(&mut editor, &ctx, value, false);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::Key {
                key: egui::Key::A,
                physical_key: Some(egui::Key::A),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            },
            egui::Event::Key {
                key: egui::Key::A,
                physical_key: Some(egui::Key::A),
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            },
            egui::Event::Paste("25".into()),
        ],
    );
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
    settle(&mut editor, &ctx);
    let after = &editor.session.document().animations[0].deform_tracks[0].keys[1];
    assert_eq!(after.curve, Curve::Bezier([0.42, 0.25, 0.58, 1.]));
    assert_eq!(
        after.offsets,
        before.animations[0].deform_tracks[0].keys[1].offsets
    );
    editor.undo();
    assert_eq!(editor.session.document(), &before);
}

#[test]
fn deform_graph_canvas_routes_native_clipboard_and_delete_to_exact_mesh_keys() {
    let (mut editor, attachment, ctx) = graph_fixture();
    let original = editor.session.document().clone();
    let row = target(&editor, "graphDeformRow");
    pointer(&mut editor, &ctx, row, true);
    pointer(&mut editor, &ctx, row, false);
    settle(&mut editor, &ctx);
    assert!(editor.deform_graph_has_focus(&ctx));
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Copy]);
    let clipboard = editor
        .timeline_clipboard
        .clone()
        .expect("Graph copy should own dense key data");
    let peachy_engine::AnimationKeyClipboard::Deform { ref keys, .. } = clipboard else {
        panic!("Expected mesh keys");
    };
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].offsets[0], 1.);
    let text =
        serde_json::json!({"peachyAnimatorClipboard":"keys","version":2,"payload":clipboard})
            .to_string();
    editor.set_time(1.5);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Paste(text)]);
    assert_eq!(editor.key_selection.keys(), &[key(&attachment, 1.5)]);
    let pasted = editor.session.document().clone();
    assert_eq!(pasted.animations[0].tracks, original.animations[0].tracks);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Cut]);
    assert_eq!(editor.session.document(), &original);
    editor.undo();
    assert_eq!(editor.session.document(), &pasted);
    editor.key_selection.replace(vec![key(&attachment, 1.5)]);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::Delete,
            physical_key: Some(egui::Key::Delete),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert_eq!(editor.session.document(), &original);
    assert!(editor.key_selection.is_empty());
}

fn skin_mesh_fixture(linked: bool) -> (Editor, AttachmentRef, AttachmentRef, egui::Context) {
    let (mut editor, base, ctx) = fixture(false);
    let mut doc = editor.session.document().clone();
    let value = doc.attachment(&base).unwrap().clone();
    let source = AttachmentRef {
        skin: "mint".into(),
        slot: base.slot.clone(),
        placeholder: "inactive.mesh".into(),
    };
    doc.skins
        .get_mut("mint")
        .unwrap()
        .entry(source.slot.clone())
        .or_default()
        .insert(source.placeholder.clone(), value.clone());
    doc.animations[0].deform_tracks[0].attachment = source.clone();
    let selected = if linked {
        let alias = AttachmentRef {
            skin: "peach".into(),
            slot: base.slot.clone(),
            placeholder: "active.link".into(),
        };
        let Attachment::Mesh(mut mesh) = value else {
            unreachable!()
        };
        mesh.link = Some(MeshLink {
            skin: source.skin.clone(),
            source: source.placeholder.clone(),
            timelines: true,
        });
        doc.skins
            .get_mut("peach")
            .unwrap()
            .entry(alias.slot.clone())
            .or_default()
            .insert(alias.placeholder.clone(), Attachment::Mesh(mesh));
        doc.slots
            .iter_mut()
            .find(|slot| slot.name == alias.slot)
            .unwrap()
            .attachment = Some(alias.placeholder.clone());
        alias
    } else {
        source.clone()
    };
    editor.session = Session::new(doc).unwrap();
    editor.skin = "peach".into();
    editor.tree.all_skin_attachments = true;
    editor.selection = Some(TreeItem::Attachment(selected.clone()));
    editor.reconcile_selection();
    (editor, selected, source, ctx)
}

#[test]
fn deform_inactive_skin_selection_stays_read_only_until_skin_activation() {
    let (mut editor, selected, source, ctx) = skin_mesh_fixture(false);
    let before = editor.session.document().clone();
    assert!(editor.timeline_owner().is_none());
    editor.key_selection.replace(vec![key(&source, 0.3)]);
    assert!(!editor.copy_dopesheet_keys(&ctx));
    editor.set_deform_key_curve(source.clone(), 0.3, Curve::Stepped);
    editor.delete_dopesheet_keys();
    editor.open_graph_view();
    settle(&mut editor, &ctx);
    assert!(!editor.capture_targets.contains_key("timelineRow:deform"));
    assert!(!editor.capture_targets.contains_key("graphDeformHandle:0"));
    assert!(
        !editor
            .capture_targets
            .contains_key("meshDeformCurve:Stepped")
    );
    assert_eq!(editor.session.document(), &before);
    editor.activate_skin("mint".into());
    assert_eq!(editor.selected_attachment(), Some(&selected));
    assert_eq!(
        editor.timeline_owner(),
        Some(KeyOwner::Deform {
            attachment: source.clone()
        })
    );
    editor.set_deform_key_curve(source, 0.3, Curve::Stepped);
    assert_eq!(
        editor.session.document().animations[0].deform_tracks[0].keys[1].curve,
        Curve::Stepped
    );
}

#[test]
fn deform_active_linked_appearance_can_edit_canonical_keys_in_another_skin() {
    let (mut editor, selected, source, ctx) = skin_mesh_fixture(true);
    assert!(editor.attachment_editable(&selected));
    assert!(!editor.attachment_editable(&source));
    assert_eq!(
        editor.timeline_owner(),
        Some(KeyOwner::Deform {
            attachment: source.clone()
        })
    );
    editor.open_graph_view();
    settle(&mut editor, &ctx);
    assert!(editor.capture_targets.contains_key("graphDeformHandle:0"));
    let before = editor.session.document().clone();
    editor.set_deform_key_curve(source.clone(), 0.3, Curve::Stepped);
    assert_eq!(
        editor.session.document().animations[0].deform_tracks[0].keys[1].curve,
        Curve::Stepped
    );
    assert_eq!(editor.session.document().skins, before.skins);
    editor.undo();
    assert_eq!(editor.session.document(), &before);
    editor.select(TreeItem::Attachment(source.clone()));
    assert!(editor.timeline_owner().is_none());
    editor.set_deform_key_curve(source, 0.3, Curve::Stepped);
    assert_eq!(editor.session.document(), &before);
}

#[test]
fn deform_zoom_on_large_final_keys_has_finite_distinct_bounds_without_data_changes() {
    for time in [2_097_152., f32::MAX] {
        let (mut editor, attachment, ctx) = fixture(false);
        let mut doc = editor.session.document().clone();
        let track = &mut doc.animations[0].deform_tracks[0];
        track.keys.truncate(2);
        track.keys[1].time = time;
        editor.session = Session::new(doc).unwrap();
        let before = editor.session.document().clone();
        editor.key_selection.replace(vec![key(&attachment, time)]);
        editor.open_graph_view();
        settle(&mut editor, &ctx);
        let zoom = target(&editor, "graphZoomKeys");
        pointer(&mut editor, &ctx, zoom, true);
        pointer(&mut editor, &ctx, zoom, false);
        settle(&mut editor, &ctx);
        let bounds = editor.graph.deform.bounds.unwrap();
        let [left, top, right, bottom] = editor.capture_targets["graphPlot"];
        let rect = egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom));
        assert!(
            valid_curve_geometry(rect, bounds),
            "time={time}, bounds={bounds:?}"
        );
        assert!(curve_point(rect, bounds, [f64::from(time), 0.]).is_finite());
        assert!(
            editor.capture_targets["graphDeformKeyPoint:1"]
                .into_iter()
                .all(f32::is_finite)
        );
        assert_eq!(editor.session.document(), &before);
        assert!(!editor.session.can_undo());
    }
}

#[test]
fn deform_clipped_handles_cannot_begin_edits_from_the_label_rail() {
    let (mut editor, attachment, ctx) = fixture(false);
    let mut doc = editor.session.document().clone();
    doc.animations[0].deform_tracks[0].keys[2].curve = Curve::Bezier([-0.25, 1., 0.58, 1.]);
    editor.session = Session::new(doc).unwrap();
    editor
        .key_selection
        .replace(vec![key(&attachment, 0.300005)]);
    editor.open_graph_view();
    settle(&mut editor, &ctx);
    let zoom = target(&editor, "graphZoomKeys");
    pointer(&mut editor, &ctx, zoom, true);
    pointer(&mut editor, &ctx, zoom, false);
    settle(&mut editor, &ctx);
    let [left, top, right, bottom] = editor.capture_targets["graphPlot"];
    let rect = egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom));
    let start = f64::from(0.300005f32);
    let hidden = curve_point(
        rect,
        editor.graph.deform.bounds.unwrap(),
        [start - 0.25 * (1. - start), 1.],
    );
    assert!(!rect.contains(hidden));
    let [left, top, right, bottom] = editor.capture_targets["graphCanvas"];
    assert!(egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom)).contains(hidden));
    assert!(!editor.capture_targets.contains_key("graphDeformHandle:0"));
    let before = editor.session.document().clone();
    pointer(&mut editor, &ctx, hidden, true);
    assert!(!editor.deform_curve_edit_active());
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(rect.center())],
    );
    pointer(&mut editor, &ctx, rect.center(), false);
    assert!(!editor.deform_curve_edit_active());
    assert_eq!(editor.session.document(), &before);
    assert!(!editor.session.can_undo());
}

#[test]
fn deform_graph_rejects_degenerate_bounds_and_rectangles() {
    let rect = egui::Rect::from_min_size(Pos2::ZERO, vec2(100., 100.));
    assert!(!valid_curve_geometry(rect, [1., 1., 0., 1.]));
    assert!(!valid_curve_geometry(rect, [0., f64::INFINITY, 0., 1.]));
    assert!(!valid_curve_geometry(rect, [0., 1., 1., 1.]));
    assert!(!valid_curve_geometry(egui::Rect::NOTHING, [0., 1., 0., 1.]));
}

#[test]
fn deform_graph_blocks_fresh_background_drags_and_presets_for_modal_popup_and_focus_loss() {
    for blocked in ["error", "rename", "duplicate", "popup", "unfocused"] {
        let (mut editor, _, ctx) = graph_fixture();
        let original = editor.session.document().clone();
        let start = target(&editor, "graphDeformHandle:0");
        let block_input = |editor: &mut Editor| match blocked {
            "error" => editor.error = Some("Another window owns input".into()),
            "rename" => editor.begin_rename(TreeItem::Animation(0)),
            "duplicate" => editor.begin_duplicate_animation(0),
            "popup" => egui::Popup::open_id(&ctx, egui::Id::new("deform-background-menu")),
            _ => {}
        };
        block_input(&mut editor);
        let frame = |editor: &mut Editor, events| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
                    focused: blocked != "unfocused",
                    events,
                    ..Default::default()
                },
                |ui| editor.workspace_ui(ui),
            );
            output.textures_delta.clear();
        };
        frame(
            &mut editor,
            vec![
                egui::Event::PointerMoved(start),
                egui::Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert!(
            !editor.deform_curve_edit_active(),
            "{blocked} began a background drag"
        );
        frame(
            &mut editor,
            vec![egui::Event::PointerMoved(start + vec2(32., -24.))],
        );
        assert_eq!(
            editor.session.document(),
            &original,
            "{blocked} changed the document"
        );
        frame(
            &mut editor,
            vec![egui::Event::PointerButton {
                pos: start + vec2(32., -24.),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        let preset = target(&editor, "graphDeformCurve:Stepped");
        // An outside click may have dismissed the dialog or popup on release.
        // Start a fresh blocked state before testing a new background action.
        block_input(&mut editor);
        frame(
            &mut editor,
            vec![
                egui::Event::PointerMoved(preset),
                egui::Event::PointerButton {
                    pos: preset,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        frame(
            &mut editor,
            vec![egui::Event::PointerButton {
                pos: preset,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert_eq!(
            editor.session.document(),
            &original,
            "{blocked} committed a background preset"
        );
        assert!(!editor.deform_curve_edit_active());
        assert!(!editor.session.can_undo());
    }
}

#[test]
fn deform_graph_discards_numeric_draft_when_a_modal_takes_focus() {
    let (mut editor, _, ctx) = graph_fixture();
    let original = editor.session.document().clone();
    let disclosure = target(&editor, "graphDeformHandles");
    pointer(&mut editor, &ctx, disclosure, true);
    pointer(&mut editor, &ctx, disclosure, false);
    settle(&mut editor, &ctx);
    let field = target(&editor, "graphDeformValue:1");
    pointer(&mut editor, &ctx, field, true);
    pointer(&mut editor, &ctx, field, false);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::Key {
                key: egui::Key::A,
                physical_key: Some(egui::Key::A),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            },
            egui::Event::Paste("25".into()),
        ],
    );
    assert!(
        editor
            .graph
            .deform
            .draft
            .as_ref()
            .is_some_and(|draft| draft.changed)
    );
    assert_eq!(editor.session.document(), &original);
    editor.error = Some("A modal interrupts the numeric draft".into());
    settle(&mut editor, &ctx);
    assert_eq!(editor.session.document(), &original);
    assert!(editor.graph.deform.draft.is_none());
    assert!(!editor.session.can_undo());
}

#[test]
fn deform_graph_toolbar_deleting_or_cutting_all_keys_handles_removed_track_in_same_frame() {
    for action in ["timelineDelete", "timelineCut"] {
        let (mut editor, attachment, ctx) = graph_fixture();
        let original = editor.session.document().clone();
        let selected = original.animations[0].deform_tracks[0]
            .keys
            .iter()
            .map(|entry| key(&attachment, entry.time))
            .collect();
        editor.key_selection.replace(selected);
        settle(&mut editor, &ctx);
        let button = target(&editor, action);
        pointer(&mut editor, &ctx, button, true);
        pointer(&mut editor, &ctx, button, false);
        assert!(
            editor.session.document().animations[0]
                .deform_tracks
                .is_empty()
        );
        assert!(editor.key_selection.is_empty());
        assert!(!editor.deform_curve_edit_active());
        assert!(editor.graph.deform.context.is_none());
        assert!(!editor.capture_targets.contains_key("graphDeformHandle:0"));
        let deleted = editor.session.document().clone();
        assert_eq!(deleted.animations[0].tracks, original.animations[0].tracks);
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
        editor.redo();
        assert_eq!(editor.session.document(), &deleted);
    }
}
