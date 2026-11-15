use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
use super::*;
use peachy_engine::{AnimationKey, AnimationKeyClipboard, KeyOwner};

fn fixture() -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    let mut doc = editor.session.document().clone();
    doc.animations[0].attachment_tracks = vec![AttachmentTrack {
        slot: 0,
        keys: vec![
            AttachmentKey {
                time: 0.2,
                attachment: doc.slots[0].attachment.clone(),
            },
            AttachmentKey {
                time: 0.3,
                attachment: None,
            },
            AttachmentKey {
                time: 0.300005,
                attachment: doc.slots[0].attachment.clone(),
            },
        ],
    }];
    doc.animations[0].color_tracks = vec![ColorTrack {
        slot: 0,
        keys: vec![
            ColorKey {
                time: 0.2,
                color: [17, 81, 159, 111],
                curves: [
                    Curve::Bezier([0.2, -0.1, 0.8, 1.2]),
                    Curve::Linear,
                    Curve::BezierOffset([0.1, 0.2, 0.9, -0.1]),
                    Curve::Linear,
                ],
            },
            ColorKey {
                time: 0.3,
                color: [160, 90, 24, 140],
                curves: [Curve::Linear; 4],
            },
            ColorKey {
                time: 0.300005,
                color: [160, 90, 25, 141],
                curves: [Curve::Linear; 4],
            },
            ColorKey {
                time: 1.,
                color: [220, 80, 120, 40],
                curves: [Curve::Linear; 4],
            },
        ],
    }];
    editor.session = Session::new(doc).unwrap();
    editor.select(TreeItem::Slot(0));
    editor.time = 0.6;
    let ctx = editor_context();
    (editor, ctx)
}

fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..3 {
        workspace_frame(editor, ctx, vec![]);
    }
}
fn pointer(editor: &mut Editor, ctx: &egui::Context, name: &str, pressed: bool) {
    let [l, t, r, b] = editor.capture_targets[name];
    let point = pos2((l + r) * 0.5, (t + b) * 0.5);
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

#[test]
fn slot_authoring_attachment_pending_global_k_and_temporary_dot_are_distinct() {
    let (mut editor, ctx) = fixture();
    let original = editor.session.document().clone();
    editor.edit_slot_attachment(0, None);
    let pending = editor.session.pose_edits().clone();
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.current_slot_pose(0).unwrap().attachment, None);
    assert_eq!(
        editor.slot_property_state(0, SlotProperty::Attachment),
        Ok(KeyState::Edited)
    );
    assert!(!editor.hidden_slots.contains(&0));
    editor.key_edited(None);
    let keyed = editor.session.document().clone();
    assert_ne!(keyed, original);
    assert_eq!(
        keyed.animations[0].attachment_tracks[0]
            .keys
            .last()
            .unwrap(),
        &AttachmentKey {
            time: 0.6,
            attachment: None
        }
    );
    assert_eq!(
        keyed.animations[0].color_tracks,
        original.animations[0].color_tracks
    );
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
    editor.redo();
    assert_eq!(editor.session.document(), &keyed);
    editor.tree.query = original.slots[0].name.clone();
    settle(&mut editor, &ctx);
    let before_dot = editor.session.pose_edits().clone();
    pointer(&mut editor, &ctx, "dot:slot:0", true);
    pointer(&mut editor, &ctx, "dot:slot:0", false);
    assert!(editor.hidden_slots.contains(&0));
    assert_eq!(editor.session.document(), &keyed);
    assert_eq!(editor.session.pose_edits(), &before_dot);
}

#[test]
fn slot_authoring_typed_rows_clipboard_and_retime_preserve_coincident_row_kinds() {
    let (mut editor, ctx) = fixture();
    settle(&mut editor, &ctx);
    assert_eq!(editor.timeline_owner(), Some(KeyOwner::Slot { slot: 0 }));
    assert!(
        editor
            .capture_targets
            .contains_key("timelineRow:attachment")
    );
    assert!(editor.capture_targets.contains_key("timelineRow:color"));
    let sources = vec![
        AnimationKey::SlotAttachment { slot: 0, time: 0.3 },
        AnimationKey::SlotColor { slot: 0, time: 0.3 },
    ];
    editor.key_selection.replace(sources.clone());
    assert!(editor.copy_dopesheet_keys(&ctx));
    let Some(AnimationKeyClipboard::Slot {
        attachment_keys,
        color_keys,
    }) = editor.timeline_clipboard.as_ref()
    else {
        panic!("Expected slot keys")
    };
    assert_eq!(attachment_keys[0].time, 0.);
    assert_eq!(color_keys[0].time, 0.);
    let original = editor.session.document().clone();
    editor.set_time(1.5);
    assert!(editor.paste_dopesheet_keys());
    assert_eq!(editor.key_selection.len(), 2);
    assert!(
        editor
            .key_selection
            .keys()
            .contains(&AnimationKey::SlotAttachment { slot: 0, time: 1.5 })
    );
    assert!(
        editor
            .key_selection
            .keys()
            .contains(&AnimationKey::SlotColor { slot: 0, time: 1.5 })
    );
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    editor
        .session
        .dispatch(Command::MoveAnimationKeys {
            animation: 0,
            keys: sources,
            delta: 0.1,
            copy: false,
        })
        .unwrap();
    assert!(
        editor.session.document().animations[0].attachment_tracks[0]
            .keys
            .iter()
            .any(|key| key.time == 0.4)
    );
    assert!(
        editor.session.document().animations[0].color_tracks[0]
            .keys
            .iter()
            .any(|key| key.time == 0.300005)
    );
}

#[test]
fn slot_authoring_exact_color_value_edit_preserves_curves_and_fractional_neighbor() {
    let (mut editor, _) = fixture();
    let original = editor.session.document().clone();
    editor.time = 0.2;
    editor.auto_key = true;
    editor
        .session
        .edit_slot_pose(
            0,
            editor.weights_edit_target(),
            &[SlotPoseEdit::Color {
                color: [0.4, 0.2, 0.8, 0.5],
            }],
            true,
        )
        .unwrap();
    let changed = &editor.session.document().animations[0].color_tracks[0];
    assert_eq!(changed.keys[0].time, 0.2);
    assert_eq!(
        changed.keys[0].curves,
        original.animations[0].color_tracks[0].keys[0].curves
    );
    assert_eq!(
        changed.keys[1..],
        original.animations[0].color_tracks[0].keys[1..]
    );
    assert_eq!(
        editor.session.document().animations[0].attachment_tracks,
        original.animations[0].attachment_tracks
    );
    editor.undo();
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn slot_authoring_color_property_button_and_key_are_adjacent_and_contextual() {
    let (mut editor, ctx) = fixture();
    settle(&mut editor, &ctx);
    let swatch = editor.capture_targets["slotColorButton"];
    let key = editor.capture_targets["slotColorKey"];
    assert!(key[0] >= swatch[2] && key[0] - swatch[2] <= 12.);
    pointer(&mut editor, &ctx, "slotColorButton", true);
    pointer(&mut editor, &ctx, "slotColorButton", false);
    settle(&mut editor, &ctx);
    assert!(editor.slot_color_dialog_open());
    assert_eq!(editor.slot_color_diagnostic()["slot"], 0);
    pointer(&mut editor, &ctx, "slotColorDialogCancel", true);
    pointer(&mut editor, &ctx, "slotColorDialogCancel", false);
    editor.select(TreeItem::Bone(8));
    settle(&mut editor, &ctx);
    assert!(editor.slot_color_diagnostic()["slot"].is_null());
    assert!(!editor.capture_targets.contains_key("slotColorHex"));
}
