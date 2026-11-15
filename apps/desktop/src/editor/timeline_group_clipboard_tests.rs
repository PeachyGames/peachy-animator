//! Native event routing exercises portable payloads separately from live bindings.
use super::super::test_support::{editor_context, editor_fixture, mesh_editor, workspace_frame};
use super::*;
use peachy_engine::{AnimationKey, KeyOwner};

fn key(bone: usize, property: Property, time: f32) -> AnimationKey {
    AnimationKey::Bone {
        bone,
        property,
        time,
    }
}
fn fixture() -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    let mut doc = editor.session.document().clone();
    let track = |bone, property, times: &[f32]| Track {
        bone,
        property,
        keys: times
            .iter()
            .enumerate()
            .map(|(index, time)| Keyframe {
                time: *time,
                value: index as f32 * 11.25,
                curve: Curve::BezierOffset([0.2, -0., 0.8, 7.25]),
            })
            .collect(),
    };
    doc.animations = vec![
        Animation::new(
            "original source",
            vec![
                track(8, Property::Rotation, &[0.125, 0.375, 0.75]),
                track(7, Property::Y, &[0.25, 0.5, 0.75]),
            ],
        ),
        Animation::new("original destination", vec![]),
    ];
    editor.session = Session::new(doc).unwrap();
    editor.session.set_skin(&editor.skin);
    editor.selection = None;
    editor.key_selection = Default::default();
    editor.time = 1.;
    editor.reconcile_selection();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    select_group(&mut editor);
    (editor, ctx)
}
fn select_group(editor: &mut Editor) {
    editor.key_selection.replace(vec![
        key(8, Property::Rotation, 0.375),
        key(7, Property::Y, 0.25),
        key(8, Property::Rotation, 0.125),
    ]);
}
fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..3 {
        workspace_frame(editor, ctx, vec![]);
    }
}
fn event(editor: &mut Editor, ctx: &egui::Context, event: egui::Event) {
    ctx.memory_mut(|memory| memory.request_focus(Editor::dopesheet_id()));
    workspace_frame(editor, ctx, vec![event]);
}
fn command(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::COMMAND,
    }
}
fn owned_text(editor: &Editor) -> String {
    editor.group_clipboard.as_ref().unwrap().text()
}
fn times(editor: &Editor, animation: usize, bone: usize, property: Property) -> Vec<f32> {
    editor.session.document().animations[animation]
        .tracks
        .iter()
        .find(|track| track.bone == bone && track.property == property)
        .unwrap()
        .keys
        .iter()
        .map(|key| key.time)
        .collect()
}
fn add_pending_with_redo(editor: &mut Editor) {
    let mut pose = editor.session.local_pose(Some(0), editor.time)[8];
    pose.x += 13.5;
    editor
        .session
        .edit_pose(
            8,
            EditTarget::Animate {
                animation: 0,
                time: editor.time,
            },
            pose,
            &[Property::X],
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
}

#[test]
fn group_clipboard_native_copy_paste_uses_one_origin_and_keeps_pending_values_curves_and_one_undo()
{
    let (mut editor, ctx) = fixture();
    add_pending_with_redo(&mut editor);
    let before = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let selected = editor.key_selection.keys().to_vec();
    let history = (editor.session.can_undo(), editor.session.can_redo());
    event(&mut editor, &ctx, egui::Event::Copy);
    assert_eq!(editor.session.document(), &before);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.key_selection.keys(), selected);
    assert_eq!(
        (editor.session.can_undo(), editor.session.can_redo()),
        history
    );
    assert!(editor.timeline_clipboard.is_none());
    let text = owned_text(&editor);
    let envelope: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(envelope["version"], 3);
    assert!(envelope.get("sourceOwners").is_none());
    assert!(envelope["payload"].get("sourceOwners").is_none());
    assert_eq!(
        envelope["payload"]["entries"][0]["keys"][0]["key"]["time"],
        0.
    );
    assert_eq!(
        envelope["payload"]["entries"][1]["keys"][0]["key"]["time"],
        0.125
    );
    assert!(
        envelope["payload"]["entries"][0]["keys"][0]
            .get("bone")
            .is_none()
    );
    assert_eq!(
        editor.timeline_clipboard_diagnostic()["bindings"][0]["owner"]["bone"],
        8
    );
    event(&mut editor, &ctx, egui::Event::Paste(text));
    assert!(editor.error.is_none(), "{:?}", editor.error);
    assert_eq!(editor.key_selection.keys(), selected);
    let receipt = editor.timeline_clipboard_diagnostic();
    assert_eq!(receipt["lastPaste"]["acceptedCount"], 3);
    assert_eq!(receipt["lastPaste"]["selectionPolicy"], "preserveExisting");
    assert_eq!(
        times(&editor, 0, 8, Property::Rotation),
        vec![0.125, 0.375, 0.75, 1., 1.25]
    );
    assert_eq!(
        times(&editor, 0, 7, Property::Y),
        vec![0.25, 0.5, 0.75, 1.125]
    );
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(!editor.session.can_redo());
    let after = editor.session.document().clone();
    assert_eq!(
        after.animations[0].tracks[0].keys[3].curve,
        before.animations[0].tracks[0].keys[0].curve
    );
    assert_eq!(
        after.animations[0].tracks[0].keys[4].value,
        before.animations[0].tracks[0].keys[1].value
    );
    event(&mut editor, &ctx, egui::Event::Cut);
    assert_eq!(
        times(&editor, 0, 8, Property::Rotation),
        vec![0.75, 1., 1.25]
    );
    assert_eq!(times(&editor, 0, 7, Property::Y), vec![0.5, 0.75, 1.125]);
    editor.undo();
    assert_eq!(editor.session.document(), &after);
    editor.undo();
    assert_eq!(editor.session.document(), &before);
    assert_eq!(editor.session.pose_edits(), &pending);
    editor.redo();
    assert_eq!(editor.session.document(), &after);
}

#[test]
fn group_clipboard_native_cut_keeps_receipt_after_sources_disappear_and_each_action_is_one_step() {
    let (mut editor, ctx) = fixture();
    let before = editor.session.document().clone();
    event(&mut editor, &ctx, egui::Event::Cut);
    let cut = editor.session.document().clone();
    assert_eq!(times(&editor, 0, 8, Property::Rotation), vec![0.75]);
    assert_eq!(times(&editor, 0, 7, Property::Y), vec![0.5, 0.75]);
    assert_eq!(editor.timeline_clipboard_count(), 3);
    event(&mut editor, &ctx, command(egui::Key::V));
    let pasted = editor.session.document().clone();
    assert!(editor.key_selection.is_empty());
    assert_eq!(
        times(&editor, 0, 8, Property::Rotation),
        vec![0.75, 1., 1.25]
    );
    editor.undo();
    assert_eq!(editor.session.document(), &cut);
    editor.undo();
    assert_eq!(editor.session.document(), &before);
    editor.redo();
    editor.redo();
    assert_eq!(editor.session.document(), &pasted);
}

#[test]
fn group_clipboard_pastes_into_another_animation_on_original_owners_without_a_bone_target() {
    let (mut editor, ctx) = fixture();
    event(&mut editor, &ctx, command(egui::Key::C));
    let source = editor.session.document().animations[0].clone();
    editor.activate_animation(1);
    editor.time = 1.;
    settle(&mut editor, &ctx);
    assert!(editor.timeline_owner().is_none());
    let text = owned_text(&editor);
    event(&mut editor, &ctx, egui::Event::Paste(text));
    assert_eq!(editor.session.document().animations[0], source);
    assert_eq!(times(&editor, 1, 8, Property::Rotation), vec![1., 1.25]);
    assert_eq!(times(&editor, 1, 7, Property::Y), vec![1.125]);
    assert!(editor.key_selection.is_empty());
    assert_eq!(
        editor.timeline_clipboard_diagnostic()["sourceAnimation"]["index"],
        0
    );
    editor.undo();
    assert!(editor.session.document().animations[1].tracks.is_empty());
}

#[test]
fn group_clipboard_constraint_receipt_remaps_through_prepend_and_history_without_changing_payload()
{
    let (mut editor, ctx) = fixture();
    let mut doc = editor.session.document().clone();
    let mut slider = SliderConstraint::new("existing original slider", 1);
    slider.setup.mix = 0.;
    doc.constraints = vec![Constraint::Slider(slider)];
    doc.animations[0].slider_tracks = vec![SliderTrack {
        constraint: 0,
        property: SliderProperty::Mix,
        keys: vec![Keyframe {
            time: 0.25,
            value: 0.5,
            curve: Curve::Bezier([0.2, 0.1, 0.8, 0.9]),
        }],
    }];
    editor.session = Session::new(doc).unwrap();
    editor.session.set_skin(&editor.skin);
    editor.key_selection = Default::default();
    editor.reconcile_selection();
    editor.key_selection.replace(vec![
        key(8, Property::Rotation, 0.125),
        AnimationKey::Slider {
            constraint: 0,
            property: SliderProperty::Mix,
            time: 0.25,
        },
    ]);
    event(&mut editor, &ctx, egui::Event::Copy);
    let text = owned_text(&editor);
    assert!(editor.dispatch(Command::CreateSliderConstraint { animation: 1 }));
    assert_eq!(
        editor.group_clipboard.as_ref().unwrap().targets().unwrap()[1].target,
        KeyOwner::Slider { constraint: 1 }
    );
    assert_eq!(owned_text(&editor), text);
    editor.undo();
    assert_eq!(
        editor.group_clipboard.as_ref().unwrap().targets().unwrap()[1].target,
        KeyOwner::Slider { constraint: 0 }
    );
    editor.redo();
    assert_eq!(
        editor.group_clipboard.as_ref().unwrap().targets().unwrap()[1].target,
        KeyOwner::Slider { constraint: 1 }
    );
    assert_eq!(owned_text(&editor), text);
    event(&mut editor, &ctx, egui::Event::Paste(text));
    assert!(editor.key_selection.is_empty());
    let accepted: Vec<AnimationKey> = serde_json::from_value(
        editor.timeline_clipboard_diagnostic()["lastPaste"]["acceptedKeys"].clone(),
    )
    .unwrap();
    assert!(accepted.contains(&AnimationKey::Slider {
        constraint: 1,
        property: SliderProperty::Mix,
        time: 1.125
    }));
    assert!(
        !editor.session.document().animations[0]
            .slider_tracks
            .iter()
            .any(|track| track.constraint == 0)
    );
}

#[test]
fn group_clipboard_removed_constraint_binding_is_a_tombstone_and_never_retargets_an_old_index() {
    let (mut editor, ctx) = fixture();
    assert!(editor.dispatch(Command::Batch {
        commands: vec![
            Command::CreateSliderConstraint { animation: 1 },
            Command::SetSliderKey {
                animation: 0,
                constraint: 0,
                property: SliderProperty::Mix,
                key: Keyframe {
                    time: 0.25,
                    value: 0.5,
                    curve: Curve::Linear
                }
            },
        ]
    }));
    editor.key_selection.replace(vec![
        key(8, Property::Rotation, 0.125),
        AnimationKey::Slider {
            constraint: 0,
            property: SliderProperty::Mix,
            time: 0.25,
        },
    ]);
    event(&mut editor, &ctx, egui::Event::Copy);
    editor.undo();
    let before = editor.session.document().clone();
    let history = (editor.session.can_undo(), editor.session.can_redo());
    let text = owned_text(&editor);
    event(&mut editor, &ctx, egui::Event::Paste(text));
    assert_eq!(editor.session.document(), &before);
    assert_eq!(
        (editor.session.can_undo(), editor.session.can_redo()),
        history
    );
    assert_eq!(
        editor.timeline_clipboard_diagnostic()["bindingsComplete"],
        false
    );
    assert!(editor.status.contains("removed"));
    editor.redo();
    assert!(editor.group_clipboard.as_ref().unwrap().targets().is_err());
}

fn slot_fixture(all_skipped: bool) -> (Editor, egui::Context) {
    let (mut editor, ctx) = fixture();
    let mut doc = editor.session.document().clone();
    doc.animations[0].attachment_tracks = vec![AttachmentTrack {
        slot: 0,
        keys: vec![
            AttachmentKey {
                time: 0.125,
                attachment: doc.slots[0].attachment.clone(),
            },
            AttachmentKey {
                time: 1.,
                attachment: doc.slots[0].attachment.clone(),
            },
        ],
    }];
    let second = if all_skipped {
        doc.animations[0].attachment_tracks.push(AttachmentTrack {
            slot: 1,
            keys: vec![
                AttachmentKey {
                    time: 0.125,
                    attachment: doc.slots[1].attachment.clone(),
                },
                AttachmentKey {
                    time: 1.,
                    attachment: doc.slots[1].attachment.clone(),
                },
            ],
        });
        AnimationKey::SlotAttachment {
            slot: 1,
            time: 0.125,
        }
    } else {
        doc.animations[0].color_tracks = vec![ColorTrack {
            slot: 1,
            keys: vec![
                ColorKey {
                    time: 0.125,
                    color: [17, 81, 159, 111],
                    curves: [Curve::Bezier([0.2, -0.1, 0.8, 1.2]); 4],
                },
                ColorKey {
                    time: 1.,
                    color: [17, 81, 159, 111],
                    curves: [Curve::Linear; 4],
                },
            ],
        }];
        AnimationKey::SlotColor {
            slot: 1,
            time: 0.125,
        }
    };
    editor.session = Session::new(doc).unwrap();
    editor.session.set_skin(&editor.skin);
    editor.key_selection = Default::default();
    editor.reconcile_selection();
    editor.key_selection.replace(vec![
        AnimationKey::SlotAttachment {
            slot: 0,
            time: 0.125,
        },
        second,
    ]);
    event(&mut editor, &ctx, egui::Event::Copy);
    // A formerly copied attachment name can become unavailable. Keep the
    // owner receipt while exercising the shared missing-name acceptance rule.
    for entry in &mut editor.group_clipboard.as_mut().unwrap().clipboard.entries {
        if let AnimationKeyClipboard::Slot {
            attachment_keys, ..
        } = entry
        {
            for key in attachment_keys {
                key.attachment = Some("unavailable-original-placeholder".into());
            }
        }
    }
    (editor, ctx)
}

#[test]
fn group_clipboard_skipped_attachment_is_not_selected_and_delete_preserves_existing_same_time_key()
{
    let (mut editor, ctx) = slot_fixture(false);
    let existing = editor.session.document().animations[0].attachment_tracks[0].keys[1].clone();
    let selected = editor.key_selection.keys().to_vec();
    let text = owned_text(&editor);
    event(&mut editor, &ctx, egui::Event::Paste(text));
    assert_eq!(editor.key_selection.keys(), selected);
    assert_eq!(
        editor.timeline_clipboard_diagnostic()["lastPaste"]["acceptedKeys"],
        serde_json::json!([AnimationKey::SlotColor { slot: 1, time: 1. }])
    );
    event(
        &mut editor,
        &ctx,
        egui::Event::Key {
            key: egui::Key::Delete,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        },
    );
    assert_eq!(
        *editor.session.document().animations[0].attachment_tracks[0]
            .keys
            .iter()
            .find(|key| key.time == 1.)
            .unwrap(),
        existing
    );
    assert!(
        editor.session.document().animations[0].color_tracks[0]
            .keys
            .iter()
            .any(|key| key.time == 1.)
    );
    assert!(
        !editor.session.document().animations[0].color_tracks[0]
            .keys
            .iter()
            .any(|key| key.time == 0.125)
    );
}

#[test]
fn group_clipboard_all_skipped_paste_preserves_document_pending_redo_and_selects_no_untouched_key()
{
    let (mut editor, ctx) = slot_fixture(true);
    add_pending_with_redo(&mut editor);
    let before = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let history = (editor.session.can_undo(), editor.session.can_redo());
    let selected = editor.key_selection.keys().to_vec();
    let text = owned_text(&editor);
    event(&mut editor, &ctx, egui::Event::Paste(text));
    assert_eq!(editor.session.document(), &before);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(
        (editor.session.can_undo(), editor.session.can_redo()),
        history
    );
    assert_eq!(editor.key_selection.keys(), selected);
    assert_eq!(
        editor.timeline_clipboard_diagnostic()["lastPaste"]["acceptedCount"],
        0
    );
}

#[test]
fn group_clipboard_readonly_rows_are_explicit_and_only_successful_copy_replaces_the_other_kind() {
    let (mut editor, ctx) = fixture();
    let mut doc = editor.session.document().clone();
    doc.animations[0].inherit_tracks = vec![InheritTrack {
        bone: 8,
        keys: vec![InheritKey {
            time: 0.5,
            inherit: Inherit::Normal,
        }],
    }];
    editor.session = Session::new(doc).unwrap();
    editor.session.set_skin(&editor.skin);
    editor.key_selection = Default::default();
    editor.reconcile_selection();
    select_group(&mut editor);
    event(&mut editor, &ctx, egui::Event::Copy);
    let group = owned_text(&editor);
    let before = editor.session.document().clone();
    editor.key_selection.clear();
    event(&mut editor, &ctx, command(egui::Key::A));
    assert!(editor.key_selection.is_empty());
    assert!(editor.status.contains("read-only"));
    event(&mut editor, &ctx, egui::Event::Copy);
    assert_eq!(owned_text(&editor), group);
    editor
        .key_selection
        .select(key(8, Property::Rotation, 0.125), false);
    event(&mut editor, &ctx, egui::Event::Copy);
    assert!(editor.group_clipboard.is_none());
    assert_eq!(editor.timeline_clipboard_diagnostic()["version"], 2);
    let leaf = editor.timeline_clipboard.clone();
    editor.key_selection.clear();
    event(&mut editor, &ctx, egui::Event::Copy);
    assert_eq!(editor.timeline_clipboard, leaf);
    select_group(&mut editor);
    event(&mut editor, &ctx, egui::Event::Copy);
    assert!(editor.timeline_clipboard.is_none());
    assert!(editor.group_clipboard.is_some());
    assert_eq!(editor.session.document(), &before);
    assert!(!editor.session.can_undo());
}

#[test]
fn group_clipboard_external_text_and_new_document_clear_bindings_without_reusing_external_payload_owners()
 {
    let (mut editor, ctx) = fixture();
    event(&mut editor, &ctx, egui::Event::Copy);
    let owned = owned_text(&editor);
    let before = editor.session.document().clone();
    event(
        &mut editor,
        &ctx,
        egui::Event::Paste("external payload".into()),
    );
    assert!(editor.group_clipboard.is_none());
    event(&mut editor, &ctx, egui::Event::Paste(owned));
    assert_eq!(editor.session.document(), &before);
    assert!(!editor.session.can_undo());
    select_group(&mut editor);
    event(&mut editor, &ctx, egui::Event::Copy);
    let owned = owned_text(&editor);
    // Open uses this same reset after installing a newly decoded Session.
    editor.session = Session::new(before.clone()).unwrap();
    editor.session.set_skin(&editor.skin);
    editor.key_selection = Default::default();
    editor.clear_timeline_clipboard();
    editor.reconcile_selection();
    event(&mut editor, &ctx, egui::Event::Paste(owned));
    assert_eq!(editor.session.document(), &before);
    assert!(!editor.session.can_undo());
    assert_eq!(editor.timeline_clipboard_diagnostic()["kind"], "empty");
}

#[test]
fn group_clipboard_modal_and_text_focus_leave_native_clipboard_input_owned_by_the_control() {
    let (mut editor, ctx) = fixture();
    event(&mut editor, &ctx, egui::Event::Copy);
    let owned = owned_text(&editor);
    let before = editor.session.document().clone();
    editor.begin_export();
    assert!(!editor.copy_dopesheet_keys(&ctx));
    assert!(!editor.paste_dopesheet_keys());
    assert!(editor.export_dialog.is_some());
    assert_eq!(owned_text(&editor), owned);
    editor.export_dialog = None;
    editor.select(TreeItem::Slot(0));
    settle(&mut editor, &ctx);
    let [left, top, right, bottom] = editor.capture_targets["slotColorButton"];
    let swatch = pos2((left + right) * 0.5, (top + bottom) * 0.5);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(swatch)]);
    for pressed in [true, false] {
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerButton {
                pos: swatch,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    assert!(editor.slot_color_dialog_open());
    let mut output = ctx.run_ui(egui::RawInput {
        focused:true, events:vec![egui::Event::Paste("external background payload".into())],
        ..Default::default()
    }, |ui| {
        let canvas = ui.interact(ui.max_rect(), Editor::dopesheet_id(), Sense::click_and_drag());
        canvas.request_focus();
        assert!(editor.dopesheet_has_focus(ui.ctx()));
        editor.timeline_clipboard_shortcuts(ui.ctx());
        assert!(ui.input(|input|input.events.iter().any(|event| matches!(event,egui::Event::Paste(text) if text=="external background payload"))));
    });
    output.textures_delta.clear();
    assert_eq!(owned_text(&editor), owned);
    assert!(editor.slot_color_dialog_open());
    assert_eq!(editor.session.document(), &before);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(!editor.slot_color_dialog_open());
    let mut text = "control".to_owned();
    let mut output = ctx.run_ui(
        egui::RawInput {
            focused: true,
            events: vec![egui::Event::Paste("typed input".into())],
            ..Default::default()
        },
        |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut text)
                    .id(egui::Id::new("group-clipboard-text-owner")),
            );
            response.request_focus();
            editor.timeline_clipboard_shortcuts(ui.ctx());
        },
    );
    output.textures_delta.clear();
    assert_eq!(owned_text(&editor), owned);
    assert_eq!(editor.session.document(), &before);
}

#[test]
fn group_clipboard_stale_deform_binding_after_slot_rename_refuses_atomically() {
    let (mut editor, attachment) = mesh_editor(false);
    let mut doc = editor.session.document().clone();
    let length = doc
        .attachment(&attachment)
        .unwrap()
        .vertices()
        .unwrap()
        .deform_len();
    doc.animations[0].deform_tracks = vec![DeformTrack {
        attachment: attachment.clone(),
        keys: vec![DeformKey {
            time: 0.125,
            offsets: vec![0.; length],
            curve: Curve::Linear,
        }],
    }];
    let slot = doc
        .slots
        .iter()
        .position(|slot| slot.name == attachment.slot)
        .unwrap();
    doc.animations[0].color_tracks = vec![ColorTrack {
        slot,
        keys: vec![ColorKey {
            time: 0.25,
            color: [17, 81, 159, 111],
            curves: [Curve::Linear; 4],
        }],
    }];
    editor.session = Session::new(doc).unwrap();
    editor.session.set_skin(&editor.skin);
    editor.setup_mode = false;
    editor.selection = None;
    editor.key_selection = Default::default();
    editor.time = 1.;
    editor.reconcile_selection();
    editor.key_selection.replace(vec![
        AnimationKey::Deform {
            attachment: attachment.clone(),
            time: 0.125,
        },
        AnimationKey::SlotColor { slot, time: 0.25 },
    ]);
    let ctx = editor_context();
    event(&mut editor, &ctx, egui::Event::Copy);
    assert!(editor.dispatch(Command::RenameSlot {
        slot,
        name: "original renamed slot".into()
    }));
    let before = editor.session.document().clone();
    let history = (editor.session.can_undo(), editor.session.can_redo());
    let selection = editor.key_selection.keys().to_vec();
    let text = owned_text(&editor);
    event(&mut editor, &ctx, egui::Event::Paste(text));
    assert_eq!(editor.session.document(), &before);
    assert_eq!(
        (editor.session.can_undo(), editor.session.can_redo()),
        history
    );
    assert_eq!(editor.key_selection.keys(), selection);
    assert!(
        editor.status.contains("deform") || editor.status.contains("attachment"),
        "{}",
        editor.status
    );
}

#[test]
fn group_clipboard_paste_keeps_single_owner_scope_and_preserves_selected_source_keys() {
    let (mut editor, ctx) = fixture();
    event(&mut editor, &ctx, egui::Event::Copy);
    editor.select(TreeItem::Bone(8));
    settle(&mut editor, &ctx);
    let text = owned_text(&editor);
    event(&mut editor, &ctx, egui::Event::Paste(text));
    assert_eq!(editor.selection, Some(TreeItem::Bone(8)));
    assert_eq!(
        editor.key_selection.keys(),
        &[
            key(8, Property::Rotation, 0.375),
            key(8, Property::Rotation, 0.125)
        ]
    );
    assert_eq!(
        times(&editor, 0, 7, Property::Y),
        vec![0.25, 0.5, 0.75, 1.125]
    );
    let receipt = editor.timeline_clipboard_diagnostic();
    assert_eq!(receipt["lastPaste"]["acceptedCount"], 3);
    assert_eq!(receipt["lastPaste"]["visibleSelectedCount"], 2);
    assert_eq!(
        receipt["lastPaste"]["acceptedKeys"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert!(editor.status.starts_with("Pasted 3 keys"));
    event(
        &mut editor,
        &ctx,
        egui::Event::Key {
            key: egui::Key::Delete,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        },
    );
    assert_eq!(
        times(&editor, 0, 8, Property::Rotation),
        vec![0.75, 1., 1.25]
    );
    assert_eq!(
        times(&editor, 0, 7, Property::Y),
        vec![0.25, 0.5, 0.75, 1.125]
    );
}

#[test]
fn group_clipboard_paste_with_unrelated_owner_keeps_tree_empty_rows_and_cut_delete_cannot_edit_hidden_keys()
 {
    let (mut editor, ctx) = fixture();
    event(&mut editor, &ctx, egui::Event::Copy);
    editor.select(TreeItem::Bone(0));
    settle(&mut editor, &ctx);
    let text = owned_text(&editor);
    event(&mut editor, &ctx, egui::Event::Paste(text));
    assert_eq!(editor.selection, Some(TreeItem::Bone(0)));
    assert!(editor.key_selection.is_empty());
    assert_eq!(
        times(&editor, 0, 8, Property::Rotation),
        vec![0.125, 0.375, 0.75, 1., 1.25]
    );
    assert_eq!(
        times(&editor, 0, 7, Property::Y),
        vec![0.25, 0.5, 0.75, 1.125]
    );
    assert_eq!(
        editor.timeline_clipboard_diagnostic()["lastPaste"]["acceptedCount"],
        3
    );
    assert_eq!(
        editor.timeline_clipboard_diagnostic()["lastPaste"]["visibleSelectedCount"],
        0
    );
    let after = editor.session.document().clone();
    let clipboard = owned_text(&editor);
    let history = (editor.session.can_undo(), editor.session.can_redo());
    event(&mut editor, &ctx, egui::Event::Cut);
    event(
        &mut editor,
        &ctx,
        egui::Event::Key {
            key: egui::Key::Delete,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        },
    );
    assert_eq!(editor.session.document(), &after);
    assert_eq!(owned_text(&editor), clipboard);
    assert_eq!(
        (editor.session.can_undo(), editor.session.can_redo()),
        history
    );
    assert!(
        editor
            .capture_targets
            .keys()
            .all(|name| !name.starts_with("timelineKey:bone:8:")
                && !name.starts_with("timelineKey:bone:7:"))
    );
}
