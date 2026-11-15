use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
use super::*;

fn fixture() -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    let mut document = editor.session.document().clone();
    document.slots.truncate(4);
    let names: BTreeSet<_> = document
        .slots
        .iter()
        .map(|slot| slot.name.clone())
        .collect();
    for skin in document.skins.values_mut() {
        skin.retain(|slot, _| names.contains(slot));
    }
    document.animations = vec![Animation::new("order-study", vec![])];
    document.animations[0].draw_order = vec![
        DrawOrderKey {
            time: 0.5,
            offsets: vec![DrawOrderOffset { slot: 0, offset: 3 }],
        },
        DrawOrderKey {
            time: 1.5,
            offsets: vec![],
        },
    ];
    editor.session = Session::new(document).unwrap();
    editor.auto_key = false;
    editor.time = 0.2;
    editor.tree = TreeState::for_document(editor.session.document(), &editor.skin);
    editor.tree.filters = tree_model::Filters {
        bones: false,
        slots: true,
        attachments: false,
        skins: false,
        animations: false,
        constraints: false,
    };
    editor.selection = Some(TreeItem::DrawOrderSlot { slot: 0 });
    editor.key_selection.clear();
    editor.reconcile_selection();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    (editor, ctx)
}
fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..4 {
        workspace_frame(editor, ctx, vec![]);
    }
}
fn rect(editor: &Editor, target: &str) -> egui::Rect {
    let [l, t, r, b] = editor.capture_targets[target];
    egui::Rect::from_min_max(pos2(l, t), pos2(r, b))
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
fn click(editor: &mut Editor, ctx: &egui::Context, target: &str) {
    let point = rect(editor, target).center();
    pointer(editor, ctx, point, true);
    pointer(editor, ctx, point, false);
    settle(editor, ctx);
}
fn chord(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
    [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers,
        })
        .collect()
}

#[test]
fn draw_order_tree_keys_global_pending_order_without_changing_slot_identity_or_setup() {
    let (mut editor, ctx) = fixture();
    let original = editor.session.document().clone();
    assert_eq!(
        editor.timeline_owner(),
        Some(peachy_engine::KeyOwner::DrawOrder {})
    );
    assert_eq!(editor.selected_bone(), None);
    assert!(rect(&editor, "drawOrderSlot:3").top() < rect(&editor, "drawOrderSlot:0").top());
    assert!(
        editor
            .capture_targets
            .contains_key("timelineGroup:drawOrder")
    );
    workspace_frame(
        &mut editor,
        &ctx,
        chord(egui::Key::Equals, egui::Modifiers::NONE),
    );
    assert_eq!(editor.current_draw_order().unwrap(), vec![1, 0, 2, 3]);
    assert_eq!(editor.selection, Some(TreeItem::DrawOrderSlot { slot: 0 }));
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.session.is_dirty());
    assert_eq!(editor.draw_order_state(), Ok(KeyState::Edited));
    click(&mut editor, &ctx, "drawOrderKey");
    assert_eq!(editor.draw_order_state(), Ok(KeyState::Keyed));
    assert!(!editor.session.pose_edits().has_draw_order(0));
    assert_eq!(editor.current_draw_order().unwrap(), vec![1, 0, 2, 3]);
    assert_eq!(editor.session.document().slots, original.slots);
    assert_eq!(
        editor.session.document().animations[0].draw_order[1..],
        original.animations[0].draw_order
    );
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert_eq!(
        editor.session.pose_edits().pending_draw_order(0),
        Some([1, 0, 2, 3].as_slice())
    );
}

#[test]
fn draw_order_shift_moves_clamp_and_boundary_reset_noops_preserve_redo() {
    let (mut editor, ctx) = fixture();
    workspace_frame(
        &mut editor,
        &ctx,
        chord(egui::Key::Equals, egui::Modifiers::SHIFT),
    );
    assert_eq!(editor.current_draw_order().unwrap(), vec![1, 2, 3, 0]);
    workspace_frame(
        &mut editor,
        &ctx,
        chord(egui::Key::Minus, egui::Modifiers::SHIFT),
    );
    assert_eq!(editor.current_draw_order().unwrap(), vec![0, 1, 2, 3]);
    editor
        .session
        .dispatch(Command::SetDrawOrder {
            animation: 0,
            time: 2.,
            order: vec![3, 0, 1, 2],
        })
        .unwrap();
    editor.session.undo();
    assert!(editor.session.can_redo());
    editor.select(TreeItem::DrawOrderSlot { slot: 3 });
    settle(&mut editor, &ctx);
    let original = editor.session.document().clone();
    workspace_frame(
        &mut editor,
        &ctx,
        chord(egui::Key::Equals, egui::Modifiers::NONE),
    );
    click(&mut editor, &ctx, "drawOrderReset");
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.can_redo());
    assert!(!editor.session.pose_edits().has_draw_order(0));
}

#[test]
fn draw_order_auto_key_checkbox_flushes_mixed_pending_pose_in_one_data_undo() {
    let (mut editor, ctx) = fixture();
    let mut pose = editor.session.local_pose(Some(0), editor.time)[8];
    pose.rotation += 13.5;
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
    workspace_frame(
        &mut editor,
        &ctx,
        chord(egui::Key::Equals, egui::Modifiers::NONE),
    );
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    assert!(pending.has_bone(0, 8) && pending.has_draw_order(0));
    click(&mut editor, &ctx, "autoKey");
    assert!(editor.auto_key);
    assert!(editor.session.pose_edits().is_empty());
    assert_ne!(editor.session.document(), &original);
    assert!(
        editor.session.document().animations[0]
            .tracks
            .iter()
            .any(|track| track.bone == 8 && track.property == Property::Rotation)
    );
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
}

#[test]
fn draw_order_front_to_back_drag_keys_one_change_and_preserves_selected_slot() {
    let (mut editor, ctx) = fixture();
    editor.auto_key = true;
    editor.select(TreeItem::DrawOrderSlot { slot: 3 });
    settle(&mut editor, &ctx);
    let original = editor.session.document().clone();
    let row_ids = editor.draw_order.row_ids.clone();
    assert_eq!(row_ids.len(), 4);
    for (slot, id) in &row_ids {
        for (other, other_id) in &row_ids {
            if slot != other {
                assert_ne!(id, other_id);
            }
        }
    }
    let from = rect(&editor, "drawOrderSlot:3").center();
    let to = rect(&editor, "drawOrderSlot:0").center() + vec2(0., 9.);
    pointer(&mut editor, &ctx, from, true);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(from + vec2(0., 12.))],
    );
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(to)]);
    assert!(editor.draw_order_edit_active());
    pointer(&mut editor, &ctx, to, false);
    settle(&mut editor, &ctx);
    assert!(!editor.draw_order_edit_active());
    assert_eq!(editor.current_draw_order().unwrap(), vec![3, 0, 1, 2]);
    assert_eq!(editor.selection, Some(TreeItem::DrawOrderSlot { slot: 3 }));
    assert_eq!(editor.session.document().slots, original.slots);
    assert_eq!(
        editor.draw_order.row_ids, row_ids,
        "Slot widgets must keep their IDs when display ranks change"
    );
    editor.undo();
    settle(&mut editor, &ctx);
    assert_eq!(editor.draw_order.row_ids, row_ids);
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn draw_order_drag_escape_restores_pending_pose_redo_and_blocks_held_reentry() {
    let (mut editor, ctx) = fixture();
    let mut pose = editor.session.local_pose(Some(0), editor.time)[8];
    pose.rotation += 13.5;
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
        .dispatch(Command::SetDrawOrder {
            animation: 0,
            time: 2.,
            order: vec![3, 0, 1, 2],
        })
        .unwrap();
    editor.session.undo();
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    assert!(editor.session.can_redo());
    let from = rect(&editor, "drawOrderSlot:0").center();
    let to = rect(&editor, "drawOrderSlot:3").center() - vec2(0., 9.);
    pointer(&mut editor, &ctx, from, true);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(to)]);
    assert!(editor.draw_order_edit_active());
    assert!(editor.session.pose_edits().has_draw_order(0));
    workspace_frame(
        &mut editor,
        &ctx,
        chord(egui::Key::Escape, egui::Modifiers::NONE),
    );
    assert!(!editor.draw_order_edit_active());
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(editor.session.can_redo());
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(to + vec2(0., 10.))],
    );
    assert!(!editor.draw_order_edit_active());
    assert_eq!(editor.session.pose_edits(), &pending);
    pointer(&mut editor, &ctx, to, false);
}

#[test]
fn draw_order_dopesheet_copy_paste_delete_preserves_explicit_reset_and_source_keys() {
    let (mut editor, ctx) = fixture();
    let original = editor.session.document().clone();
    click(&mut editor, &ctx, "timelineKey:drawOrder:1");
    assert_eq!(
        editor.key_selection.keys(),
        &[peachy_engine::AnimationKey::DrawOrder { time: 1.5 }]
    );
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Copy]);
    assert_eq!(editor.session.document(), &original);
    editor.set_time(2.);
    settle(&mut editor, &ctx);
    workspace_frame(
        &mut editor,
        &ctx,
        chord(egui::Key::V, egui::Modifiers::COMMAND),
    );
    assert_eq!(
        editor.key_selection.keys(),
        &[peachy_engine::AnimationKey::DrawOrder { time: 2. }]
    );
    assert!(
        editor.session.document().animations[0]
            .draw_order
            .last()
            .unwrap()
            .offsets
            .is_empty()
    );
    workspace_frame(
        &mut editor,
        &ctx,
        chord(egui::Key::Delete, egui::Modifiers::NONE),
    );
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.document().slots, original.slots);
}

#[test]
fn draw_order_has_no_scalar_graph_or_curves_and_setup_reordering_is_disabled() {
    let (mut editor, ctx) = fixture();
    editor.open_graph_view();
    editor.open_curves_view();
    settle(&mut editor, &ctx);
    editor
        .key_selection
        .replace(vec![peachy_engine::AnimationKey::DrawOrder { time: 0.5 }]);
    settle(&mut editor, &ctx);
    assert!(editor.selected_curve_channels().unwrap().is_empty());
    assert!(!editor.capture_targets.contains_key("graphScalarHandle:0"));
    assert!(!editor.capture_targets.contains_key("curvesHandle:0"));
    let original = editor.session.document().clone();
    editor.set_setup_mode(true);
    settle(&mut editor, &ctx);
    workspace_frame(
        &mut editor,
        &ctx,
        chord(egui::Key::Equals, egui::Modifiers::NONE),
    );
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.capture_targets.contains_key("drawOrderKey"));
}

#[test]
fn draw_order_outside_drop_cancels_and_visible_tree_reentry_accepts_full_width() {
    for destination in ["viewport", "properties", "header", "empty", "reentry"] {
        let (mut editor, ctx) = fixture();
        editor.auto_key = true;
        let mut pose = editor.session.local_pose(Some(0), editor.time)[8];
        pose.rotation += 13.5;
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
            .dispatch(Command::SetDrawOrder {
                animation: 0,
                time: 2.,
                order: vec![1, 0, 2, 3],
            })
            .unwrap();
        editor.session.undo();
        editor.select(TreeItem::DrawOrderSlot { slot: 3 });
        settle(&mut editor, &ctx);
        let original = editor.session.document().clone();
        let pending = editor.session.pose_edits().clone();
        assert!(editor.session.can_redo());
        let drop = rect(&editor, "drawOrderDropArea");
        let from = rect(&editor, "drawOrderSlot:3").center();
        let inside = pos2(
            drop.left() + 3.,
            rect(&editor, "drawOrderSlot:0").center().y + 9.,
        );
        let outside = match destination {
            "properties" => rect(&editor, "properties").center(),
            "header" => rect(&editor, "drawOrder").center(),
            "empty" => {
                let upper = rect(&editor, "drawOrderSlot:3").bottom();
                let lower = rect(&editor, "drawOrderSlot:2").top();
                assert!(lower > upper);
                pos2(drop.left() + 3., (upper + lower) / 2.)
            }
            _ => pos2(drop.left() - 20., inside.y),
        };
        pointer(&mut editor, &ctx, from, true);
        workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(inside)]);
        assert!(editor.draw_order_edit_active(), "{destination}");
        assert_eq!(editor.current_draw_order().unwrap(), vec![3, 0, 1, 2]);
        let preview = editor.session.document().clone();
        workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(outside)]);
        assert!(editor.draw_order_edit_active());
        assert_eq!(editor.session.document(), &preview);
        if destination == "reentry" {
            workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(inside)]);
            pointer(&mut editor, &ctx, inside, false);
            settle(&mut editor, &ctx);
            assert_eq!(editor.current_draw_order().unwrap(), vec![3, 0, 1, 2]);
            assert_ne!(editor.session.document(), &original);
            editor.undo();
        } else {
            pointer(&mut editor, &ctx, outside, false);
            settle(&mut editor, &ctx);
            assert!(editor.session.can_redo());
        }
        assert!(!editor.draw_order_edit_active());
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
    }
}
