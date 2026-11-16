//! Single-owner source receipts never travel in the unchanged portable v2 text.
use super::super::test_support::{editor_context, editor_fixture, mesh_editor, workspace_frame};
use super::*;
use peachy_engine::{AnimationKey, KeyOwner};

fn key(bone: usize, property: Property, frame: f32) -> AnimationKey {
    AnimationKey::Bone {
        bone,
        property,
        time: frame / 60.,
    }
}
fn fixture() -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    let mut doc = editor.session.document().clone();
    doc.fps = 60;
    doc.slots[0].bone = 7;
    doc.constraints = vec![Constraint::Slider({
        let mut slider = SliderConstraint::new("original slider", 1);
        slider.setup.mix = 0.;
        slider
    })];
    doc.animations = vec![
        Animation::new(
            "original source",
            vec![
                Track {
                    bone: 8,
                    property: Property::Y,
                    keys: [(5., 3.), (25., 14.)]
                        .into_iter()
                        .map(|(frame, value)| Keyframe {
                            time: frame / 60.,
                            value,
                            curve: Curve::BezierOffset([0.2, -0., 0.8, 7.25]),
                        })
                        .collect(),
                },
                Track {
                    bone: 7,
                    property: Property::Rotation,
                    keys: [(12., 9.), (35., -4.)]
                        .into_iter()
                        .map(|(frame, value)| Keyframe {
                            time: frame / 60.,
                            value,
                            curve: Curve::Stepped,
                        })
                        .collect(),
                },
            ],
        ),
        Animation::new("other host", vec![]),
    ];
    editor.session = Session::new(doc).unwrap();
    editor.session.set_skin(&editor.skin);
    editor.selection = None;
    editor.key_selection = Default::default();
    editor.reconcile_selection();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    (editor, ctx)
}
fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..3 {
        workspace_frame(editor, ctx, vec![]);
    }
}
fn point(editor: &Editor, name: &str) -> Pos2 {
    let [l, t, r, b] = editor.capture_targets[name];
    pos2((l + r) * 0.5, (t + b) * 0.5)
}
fn pointer(
    editor: &mut Editor,
    ctx: &egui::Context,
    p: Pos2,
    pressed: bool,
    modifiers: egui::Modifiers,
) {
    workspace_frame(
        editor,
        ctx,
        vec![
            egui::Event::PointerMoved(p),
            egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers,
            },
        ],
    );
}
fn click(editor: &mut Editor, ctx: &egui::Context, name: &str, modifiers: egui::Modifiers) {
    let p = point(editor, name);
    workspace_frame(editor, ctx, vec![egui::Event::PointerMoved(p)]);
    pointer(editor, ctx, p, true, modifiers);
    pointer(editor, ctx, p, false, egui::Modifiers::NONE);
}
fn event(editor: &mut Editor, ctx: &egui::Context, event: egui::Event) {
    ctx.memory_mut(|memory| memory.request_focus(Editor::dopesheet_id()));
    workspace_frame(editor, ctx, vec![event]);
}
fn ruler(editor: &mut Editor, ctx: &egui::Context, frame: f32) {
    let [l, _, r, _] = editor.capture_targets["timelineTrack"];
    let [_, t, _, b] = editor.capture_targets["timelineRuler"];
    let p = pos2(
        l + (r - l) * (frame / editor.session.document().fps as f32) / editor.display_extent(),
        (t + b) * 0.5,
    );
    workspace_frame(editor, ctx, vec![egui::Event::PointerMoved(p)]);
    pointer(editor, ctx, p, true, egui::Modifiers::NONE);
    pointer(editor, ctx, p, false, egui::Modifiers::NONE);
}
fn copy_driver(editor: &mut Editor, ctx: &egui::Context) {
    ruler(editor, ctx, 50.);
    for name in ["timelineKey:bone:8:Y:0", "timelineKey:bone:8:Y:1"] {
        click(editor, ctx, name, egui::Modifiers::COMMAND);
    }
    event(editor, ctx, egui::Event::Copy);
    assert_eq!(editor.timeline_clipboard_diagnostic()["kind"], "single");
}
fn text(editor: &Editor) -> String {
    clipboard_text(editor.timeline_clipboard.as_ref().unwrap())
}
fn track(editor: &Editor, animation: usize, bone: usize, property: Property) -> &Track {
    editor.session.document().animations[animation]
        .tracks
        .iter()
        .find(|track| track.bone == bone && track.property == property)
        .unwrap()
}
fn accepted(editor: &Editor) -> Vec<AnimationKey> {
    serde_json::from_value(
        editor.timeline_clipboard_diagnostic()["lastPaste"]["acceptedKeys"].clone(),
    )
    .unwrap()
}

#[test]
fn leaf_clipboard_no_owner_and_animation_selection_use_original_owner_with_v2_bytes_and_source_cut()
{
    for selection in [None, Some(TreeItem::Animation(0))] {
        let (mut editor, ctx) = fixture();
        editor.restore_selection(selection.clone());
        settle(&mut editor, &ctx);
        ruler(&mut editor, &ctx, 50.);
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
        let pending = editor.session.pose_edits().clone();
        let before = editor.session.document().clone();
        copy_driver(&mut editor, &ctx);
        let selected = editor.key_selection.keys().to_vec();
        let portable = text(&editor);
        let expected = peachy_engine::copy_animation_keys(&before, 0, &selected).unwrap();
        assert_eq!(portable, clipboard_text(&expected));
        let wire: serde_json::Value = serde_json::from_str(&portable).unwrap();
        assert_eq!(wire["version"], 2);
        assert!(wire.get("sourceOwner").is_none());
        assert!(wire["payload"].get("bindings").is_none());
        assert_eq!(
            editor.timeline_clipboard_diagnostic()["bindings"][0]["owner"]["bone"],
            8
        );
        click(&mut editor, &ctx, "timelinePaste", egui::Modifiers::NONE);
        assert_eq!(editor.selection, selection);
        assert_eq!(editor.key_selection.keys(), selected);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(
            track(&editor, 0, 8, Property::Y)
                .keys
                .iter()
                .map(|key| (key.time * 60.).round() as u32)
                .collect::<Vec<_>>(),
            vec![5, 25, 50, 70]
        );
        assert_eq!(
            track(&editor, 0, 7, Property::Rotation),
            before.animations[0]
                .tracks
                .iter()
                .find(|track| track.bone == 7)
                .unwrap()
        );
        let incoming = accepted(&editor);
        assert_eq!(incoming.len(), 2);
        assert!(
            incoming
                .iter()
                .all(|key| key.owner() == KeyOwner::Bone { bone: 8 })
        );
        let pasted = editor.session.document().clone();
        click(&mut editor, &ctx, "timelineCut", egui::Modifiers::NONE);
        assert_eq!(
            track(&editor, 0, 8, Property::Y)
                .keys
                .iter()
                .map(|key| (key.time * 60.).round() as u32)
                .collect::<Vec<_>>(),
            vec![50, 70]
        );
        assert_eq!(editor.session.pose_edits(), &pending);
        editor.undo();
        assert_eq!(editor.session.document(), &pasted);
        editor.undo();
        assert_eq!(editor.session.document(), &before);
    }
}

#[test]
fn leaf_clipboard_explicit_bone_slot_and_region_pose_targets_override_the_original_owner() {
    for kind in 0..3 {
        let (mut editor, ctx) = fixture();
        copy_driver(&mut editor, &ctx);
        assert_eq!(
            editor.key_selection.keys(),
            &[key(8, Property::Y, 5.), key(8, Property::Y, 25.)]
        );
        let before = editor.session.document().clone();
        let (selection, target) = match kind {
            0 => (TreeItem::Bone(0), 0),
            1 => (TreeItem::Slot(0), 7),
            _ => {
                let attachment = editor.shown_slot_attachment(0).unwrap();
                assert!(matches!(
                    editor.session.document().attachment(&attachment),
                    Some(Attachment::Region(_))
                ));
                (TreeItem::Attachment(attachment), 7)
            }
        };
        editor.select(selection.clone());
        settle(&mut editor, &ctx);
        ruler(&mut editor, &ctx, 50.);
        let selected_before = editor.key_selection.keys().to_vec();
        assert!(selected_before.is_empty());
        if kind == 2 {
            assert_eq!(
                editor.timeline_owner(),
                Some(KeyOwner::Bone { bone: target })
            );
            assert_eq!(
                editor.key_selection.shown().owners,
                vec![KeyOwner::Bone { bone: target }]
            );
            assert!(
                editor
                    .capture_targets
                    .contains_key("timelineKey:bone:7:Rotation:0")
            );
            assert!(
                !editor
                    .capture_targets
                    .contains_key("timelineKey:bone:8:Y:0")
            );
        }
        let portable = text(&editor);
        event(&mut editor, &ctx, egui::Event::Paste(portable));
        assert_eq!(editor.selection, Some(selection));
        assert_eq!(editor.key_selection.keys(), selected_before);
        let incoming = accepted(&editor);
        assert!(
            editor
                .key_selection
                .keys()
                .iter()
                .all(|key| !incoming.contains(key))
        );
        assert_eq!(
            track(&editor, 0, 8, Property::Y),
            before.animations[0]
                .tracks
                .iter()
                .find(|track| track.bone == 8)
                .unwrap()
        );
        assert_eq!(
            track(&editor, 0, target, Property::Y)
                .keys
                .iter()
                .map(|key| ((key.time * 60.).round() as u32, key.value))
                .collect::<Vec<_>>(),
            vec![(50, 3.), (70, 14.)]
        );
        assert!(
            accepted(&editor)
                .iter()
                .all(|key| key.owner() == KeyOwner::Bone { bone: target })
        );
        if kind == 2 {
            let pasted = editor.session.document().clone();
            click(&mut editor, &ctx, "timelineCut", egui::Modifiers::NONE);
            assert_eq!(editor.session.document(), &pasted);
            editor.open_graph_view();
            settle(&mut editor, &ctx);
            assert_eq!(editor.graph_owner(), Some(KeyOwner::Bone { bone: target }));
            assert_eq!(editor.graph_diagnostic()["context"]["bone"], target);
        }
        editor.undo();
        assert_eq!(editor.session.document(), &before);
    }
}

#[test]
fn leaf_clipboard_visible_animate_slot_row_pastes_to_its_parent_without_changing_global_order_scope()
 {
    let (mut editor, ctx) = fixture();
    let mut doc = editor.session.document().clone();
    let slot = doc.slots.len() - 1;
    doc.slots[slot].bone = 7;
    editor.session = Session::new(doc).unwrap();
    editor.session.set_skin(&editor.skin);
    editor.key_selection = Default::default();
    editor.reconcile_selection();
    settle(&mut editor, &ctx);
    copy_driver(&mut editor, &ctx);
    click(&mut editor, &ctx, "filter:Slots", egui::Modifiers::NONE);
    settle(&mut editor, &ctx);
    let row = format!("drawOrderSlot:{slot}");
    assert!(editor.capture_targets.contains_key(&row));
    assert!(!editor.capture_targets.contains_key(&format!("slot:{slot}")));
    click(&mut editor, &ctx, &row, egui::Modifiers::NONE);
    settle(&mut editor, &ctx);
    let selection = Some(TreeItem::DrawOrderSlot { slot });
    assert_eq!(editor.selection, selection);
    // Slot compound Bone/slot scoping is a separate gap; preserve the current
    // explicit global-order scope instead of revealing pasted Bone keys.
    assert_eq!(editor.timeline_owner(), Some(KeyOwner::DrawOrder {}));
    assert!(editor.key_selection.is_empty());
    let before = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let order = editor.current_draw_order().unwrap();
    let portable = text(&editor);
    event(&mut editor, &ctx, egui::Event::Paste(portable.clone()));
    assert_eq!(editor.selection, selection);
    assert_eq!(editor.timeline_owner(), Some(KeyOwner::DrawOrder {}));
    assert!(editor.key_selection.is_empty());
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.current_draw_order().unwrap(), order);
    assert_eq!(editor.session.document().slots, before.slots);
    assert_eq!(
        track(&editor, 0, 8, Property::Y),
        before.animations[0]
            .tracks
            .iter()
            .find(|track| track.bone == 8)
            .unwrap()
    );
    assert_eq!(
        track(&editor, 0, 7, Property::Y)
            .keys
            .iter()
            .map(|key| ((key.time * 60.).round() as u32, key.value))
            .collect::<Vec<_>>(),
        vec![(50, 3.), (70, 14.)]
    );
    assert!(
        accepted(&editor)
            .iter()
            .all(|key| key.owner() == KeyOwner::Bone { bone: 7 })
    );
    editor.undo();
    assert_eq!(editor.session.document(), &before);
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "drawOrder", egui::Modifiers::NONE);
    settle(&mut editor, &ctx);
    assert_eq!(editor.selection, Some(TreeItem::DrawOrder));
    let history = (editor.session.can_undo(), editor.session.can_redo());
    event(&mut editor, &ctx, egui::Event::Paste(portable));
    assert_eq!(editor.session.document(), &before);
    assert_eq!(
        (editor.session.can_undo(), editor.session.can_redo()),
        history
    );
    assert_eq!(editor.selection, Some(TreeItem::DrawOrder));
}

#[test]
fn leaf_clipboard_explicit_incompatible_constraint_never_falls_back_to_original_bone() {
    let (mut editor, ctx) = fixture();
    copy_driver(&mut editor, &ctx);
    editor.select(TreeItem::Constraint(0));
    settle(&mut editor, &ctx);
    let before = editor.session.document().clone();
    let history = (editor.session.can_undo(), editor.session.can_redo());
    let source = editor.leaf_source.clone();
    let portable = text(&editor);
    event(&mut editor, &ctx, egui::Event::Paste(portable.clone()));
    assert_eq!(editor.session.document(), &before);
    assert_eq!(
        (editor.session.can_undo(), editor.session.can_redo()),
        history
    );
    assert_eq!(editor.leaf_source, source);
    assert_eq!(text(&editor), portable);
}

#[test]
fn leaf_clipboard_cut_then_cross_animation_without_owner_keeps_original_binding() {
    let (mut editor, ctx) = fixture();
    copy_driver(&mut editor, &ctx);
    event(&mut editor, &ctx, egui::Event::Cut);
    let cut = editor.session.document().clone();
    assert!(!cut.animations[0].tracks.iter().any(|track| track.bone == 8));
    editor.activate_animation(1);
    editor.restore_selection(Some(TreeItem::Animation(1)));
    settle(&mut editor, &ctx);
    ruler(&mut editor, &ctx, 50.);
    let portable = text(&editor);
    event(&mut editor, &ctx, egui::Event::Paste(portable));
    assert_eq!(editor.session.document().animations[0], cut.animations[0]);
    assert!(editor.key_selection.is_empty());
    assert_eq!(
        track(&editor, 1, 8, Property::Y)
            .keys
            .iter()
            .map(|key| ((key.time * 60.).round() as u32, key.value))
            .collect::<Vec<_>>(),
        vec![(50, 3.), (70, 14.)]
    );
    assert_eq!(
        editor.timeline_clipboard_diagnostic()["sourceAnimation"]["index"],
        0
    );
    editor.undo();
    assert_eq!(editor.session.document(), &cut);
}

#[test]
fn leaf_clipboard_receipt_remaps_slider_prepend_history_and_never_revives_removed_owner() {
    let (mut editor, ctx) = fixture();
    let mut doc = editor.session.document().clone();
    doc.animations[0].slider_tracks = vec![SliderTrack {
        constraint: 0,
        property: SliderProperty::Mix,
        keys: vec![Keyframe {
            time: 5. / 60.,
            value: 0.5,
            curve: Curve::Bezier([0.2, 0.1, 0.8, 0.9]),
        }],
    }];
    editor.session = Session::new(doc).unwrap();
    editor.session.set_skin(&editor.skin);
    editor.key_selection = Default::default();
    editor.reconcile_selection();
    settle(&mut editor, &ctx);
    click(
        &mut editor,
        &ctx,
        "timelineKey:slider:0:Mix:0",
        egui::Modifiers::COMMAND,
    );
    event(&mut editor, &ctx, egui::Event::Copy);
    let portable = text(&editor);
    assert!(editor.dispatch(Command::CreateSliderConstraint { animation: 1 }));
    assert_eq!(
        editor.timeline_clipboard_diagnostic()["bindings"][0]["owner"]["constraint"],
        1
    );
    assert_eq!(text(&editor), portable);
    editor.undo();
    assert_eq!(
        editor.timeline_clipboard_diagnostic()["bindings"][0]["owner"]["constraint"],
        0
    );
    editor.redo();
    assert_eq!(
        editor.timeline_clipboard_diagnostic()["bindings"][0]["owner"]["constraint"],
        1
    );
    settle(&mut editor, &ctx);
    ruler(&mut editor, &ctx, 50.);
    event(&mut editor, &ctx, egui::Event::Paste(portable));
    assert_eq!(
        accepted(&editor)[0].owner(),
        KeyOwner::Slider { constraint: 1 }
    );
    let (mut editor, ctx) = fixture();
    assert!(editor.dispatch(Command::Batch {
        commands: vec![
            Command::CreateSliderConstraint { animation: 1 },
            Command::SetSliderKey {
                animation: 0,
                constraint: 0,
                property: SliderProperty::Mix,
                key: Keyframe {
                    time: 5. / 60.,
                    value: 0.5,
                    curve: Curve::Linear
                }
            }
        ]
    }));
    settle(&mut editor, &ctx);
    click(
        &mut editor,
        &ctx,
        "timelineKey:slider:0:Mix:0",
        egui::Modifiers::COMMAND,
    );
    event(&mut editor, &ctx, egui::Event::Copy);
    let portable = text(&editor);
    editor.undo();
    let before = editor.session.document().clone();
    let history = (editor.session.can_undo(), editor.session.can_redo());
    event(&mut editor, &ctx, egui::Event::Paste(portable.clone()));
    assert_eq!(editor.session.document(), &before);
    assert_eq!(
        (editor.session.can_undo(), editor.session.can_redo()),
        history
    );
    assert!(editor.status.contains("removed"));
    assert_eq!(
        editor.timeline_clipboard_diagnostic()["bindingsComplete"],
        false
    );
    editor.redo();
    assert_eq!(
        editor.timeline_clipboard_diagnostic()["bindingsComplete"],
        false
    );
    assert_eq!(text(&editor), portable);
}

#[test]
fn leaf_clipboard_mismatched_paste_is_nondestructive_but_open_reset_cannot_recreate_binding_from_v2_text()
 {
    let (mut editor, ctx) = fixture();
    copy_driver(&mut editor, &ctx);
    let portable = text(&editor);
    let clipboard = editor.timeline_clipboard.clone();
    let source = editor.leaf_source.clone();
    let before = editor.session.document().clone();
    let selected = editor.key_selection.keys().to_vec();
    let history = (editor.session.can_undo(), editor.session.can_redo());
    let mut external: serde_json::Value = serde_json::from_str(&portable).unwrap();
    external["payload"]["keys"][0]["key"]["value"] = 999.into();
    event(&mut editor, &ctx, egui::Event::Paste(external.to_string()));
    assert_eq!(editor.session.document(), &before);
    assert_eq!(editor.timeline_clipboard, clipboard);
    assert_eq!(editor.leaf_source, source);
    assert_eq!(editor.key_selection.keys(), selected);
    assert_eq!(
        (editor.session.can_undo(), editor.session.can_redo()),
        history
    );
    // The original receipt remained owned throughout rejection, so its exact
    // original bytes are still valid; no external text creates a new binding.
    event(&mut editor, &ctx, egui::Event::Paste(portable.clone()));
    assert_eq!(accepted(&editor).len(), 2);
    editor.clear_timeline_clipboard();
    editor.timeline_clipboard = clipboard;
    let after = editor.session.document().clone();
    let history = (editor.session.can_undo(), editor.session.can_redo());
    event(&mut editor, &ctx, egui::Event::Paste(portable));
    assert_eq!(editor.session.document(), &after);
    assert_eq!(
        (editor.session.can_undo(), editor.session.can_redo()),
        history
    );
    assert!(editor.leaf_source.is_none());
}

#[test]
fn leaf_clipboard_only_successful_copy_replaces_receipt_and_group_copy_clears_it() {
    let (mut editor, ctx) = fixture();
    copy_driver(&mut editor, &ctx);
    let source = editor.leaf_source.clone();
    let portable = text(&editor);
    editor.key_selection.clear();
    event(&mut editor, &ctx, egui::Event::Copy);
    assert_eq!(editor.leaf_source, source);
    assert_eq!(text(&editor), portable);
    for name in [
        "timelineKey:bone:7:Rotation:0",
        "timelineKey:bone:7:Rotation:1",
    ] {
        click(&mut editor, &ctx, name, egui::Modifiers::COMMAND);
    }
    event(&mut editor, &ctx, egui::Event::Copy);
    assert_eq!(
        editor.timeline_clipboard_diagnostic()["bindings"][0]["owner"]["bone"],
        7
    );
    assert_ne!(text(&editor), portable);
    click(
        &mut editor,
        &ctx,
        "timelineKey:bone:8:Y:0",
        egui::Modifiers::COMMAND,
    );
    event(&mut editor, &ctx, egui::Event::Copy);
    assert!(editor.timeline_clipboard.is_none());
    assert!(editor.leaf_source.is_none());
    assert!(editor.group_clipboard.is_some());
}

#[test]
fn leaf_clipboard_inactive_explicit_slot_and_region_refuse_before_original_source_fallback() {
    for kind in 0..4 {
        let (mut editor, ctx) = fixture();
        let shown = editor.shown_slot_attachment(0).unwrap();
        let appearance = AttachmentRef {
            skin: "peach".into(),
            ..shown.clone()
        };
        let mut doc = editor.session.document().clone();
        let default_region = AttachmentRef {
            skin: "default".into(),
            ..shown.clone()
        };
        let region = doc.attachment(&shown).unwrap().clone();
        assert!(matches!(region, Attachment::Region(_)));
        doc.skins
            .entry("default".into())
            .or_default()
            .entry(default_region.slot.clone())
            .or_default()
            .insert(default_region.placeholder.clone(), region.clone());
        doc.skins
            .entry(appearance.skin.clone())
            .or_default()
            .entry(appearance.slot.clone())
            .or_default()
            .insert(appearance.placeholder.clone(), region);
        doc.skin_scope.required_bones = (0..doc.bones.len()).collect();
        doc.skin_scope.skins.insert(
            "peach".into(),
            SkinMembers {
                bones: (0..doc.bones.len()).collect(),
                constraints: vec![],
            },
        );
        editor.session = Session::new(doc).unwrap();
        editor.session.set_skin(&editor.skin);
        editor.key_selection = Default::default();
        editor.reconcile_selection();
        settle(&mut editor, &ctx);
        copy_driver(&mut editor, &ctx);
        editor.select(match kind {
            0 => TreeItem::Slot(0),
            1 => TreeItem::Attachment(default_region),
            2 => TreeItem::Attachment(appearance),
            _ => TreeItem::DrawOrderSlot { slot: 0 },
        });
        editor.activate_skin("mint".into());
        settle(&mut editor, &ctx);
        if matches!(kind, 1 | 2) {
            assert_eq!(editor.timeline_owner(), Some(KeyOwner::Bone { bone: 7 }));
            assert_eq!(
                editor.key_selection.shown().readonly_owners,
                vec![KeyOwner::Bone { bone: 7 }]
            );
            assert!(editor.key_selection.is_empty());
            assert!(editor.graph_owner().is_none());
        }
        let before = editor.session.document().clone();
        let history = (editor.session.can_undo(), editor.session.can_redo());
        let pending = editor.session.pose_edits().clone();
        let source = editor.leaf_source.clone();
        let refusal = editor
            .leaf_paste_target(editor.timeline_clipboard.as_ref().unwrap())
            .unwrap_err();
        let portable = text(&editor);
        event(&mut editor, &ctx, egui::Event::Paste(portable));
        assert_eq!(editor.session.document(), &before);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(
            (editor.session.can_undo(), editor.session.can_redo()),
            history
        );
        assert_eq!(editor.leaf_source, source);
        assert_eq!(editor.status, refusal);
        assert!(
            editor
                .status
                .contains(if kind == 2 { "read-only" } else { "inactive" }),
            "case {kind}: {}",
            editor.status
        );
        assert!(editor.paste_receipt.is_none());
        if matches!(kind, 1 | 2) {
            editor.open_graph_view();
            settle(&mut editor, &ctx);
            assert!(editor.graph_owner().is_none());
            assert!(editor.graph_diagnostic()["context"].is_null());
            assert_eq!(editor.session.document(), &before);
            assert_eq!(
                (editor.session.can_undo(), editor.session.can_redo()),
                history
            );
        }
    }
}

#[test]
fn leaf_clipboard_linked_deform_uses_canonical_source_and_refuses_stale_rename_without_redirecting()
{
    let (mut editor, base) = mesh_editor(false);
    let mut doc = editor.session.document().clone();
    let original = doc.attachment(&base).unwrap().clone();
    let source = AttachmentRef {
        skin: "mint".into(),
        slot: base.slot.clone(),
        placeholder: "original.source".into(),
    };
    doc.skins
        .get_mut("mint")
        .unwrap()
        .entry(source.slot.clone())
        .or_default()
        .insert(source.placeholder.clone(), original.clone());
    let Attachment::Mesh(mut alias) = original else {
        panic!("Expected original mesh")
    };
    alias.link = Some(MeshLink {
        skin: source.skin.clone(),
        source: source.placeholder.clone(),
        timelines: true,
    });
    let active = AttachmentRef {
        skin: "peach".into(),
        slot: base.slot.clone(),
        placeholder: "original.alias".into(),
    };
    doc.skins
        .get_mut("peach")
        .unwrap()
        .entry(active.slot.clone())
        .or_default()
        .insert(active.placeholder.clone(), Attachment::Mesh(alias));
    let length = doc
        .attachment(&source)
        .unwrap()
        .vertices()
        .unwrap()
        .deform_len();
    doc.animations[0].deform_tracks = vec![DeformTrack {
        attachment: source.clone(),
        keys: vec![DeformKey {
            time: 0.125,
            offsets: vec![1.; length],
            curve: Curve::Stepped,
        }],
    }];
    editor.session = Session::new(doc).unwrap();
    editor.session.set_skin(&editor.skin);
    editor.setup_mode = false;
    editor.selection = None;
    editor.key_selection = Default::default();
    editor.reconcile_selection();
    editor.key_selection.replace(vec![AnimationKey::Deform {
        attachment: source.clone(),
        time: 0.125,
    }]);
    let ctx = editor_context();
    event(&mut editor, &ctx, egui::Event::Copy);
    assert_eq!(
        editor.timeline_clipboard_diagnostic()["bindings"][0]["owner"],
        serde_json::json!(KeyOwner::Deform {
            attachment: source.clone()
        })
    );
    settle(&mut editor, &ctx);
    ruler(&mut editor, &ctx, 50.);
    let portable = text(&editor);
    event(&mut editor, &ctx, egui::Event::Paste(portable.clone()));
    assert_eq!(
        accepted(&editor)[0].owner(),
        KeyOwner::Deform {
            attachment: source.clone()
        }
    );
    let slot = editor
        .session
        .document()
        .slots
        .iter()
        .position(|slot| slot.name == source.slot)
        .unwrap();
    assert!(editor.dispatch(Command::RenameSlot {
        slot,
        name: "original renamed source slot".into()
    }));
    let before = editor.session.document().clone();
    let history = (editor.session.can_undo(), editor.session.can_redo());
    event(&mut editor, &ctx, egui::Event::Paste(portable));
    assert_eq!(editor.session.document(), &before);
    assert_eq!(
        (editor.session.can_undo(), editor.session.can_redo()),
        history
    );
}
