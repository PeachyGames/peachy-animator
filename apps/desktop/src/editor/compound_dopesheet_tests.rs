//! Real workspace input over an original compound family; presentation scope
//! must never become an implicit clipboard, Graph or pose target.
use super::super::dopesheet_selection::TimelineScope;
use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
use super::*;
use peachy_engine::{AnimationKey, KeyOwner};

fn scalar(time: f32, value: f32) -> Keyframe {
    Keyframe {
        time,
        value,
        curve: Curve::BezierOffset([0.2, -0., 0.8, 2.25]),
    }
}
fn bone_key(time: f32) -> AnimationKey {
    AnimationKey::Bone {
        bone: 1,
        property: Property::Rotation,
        time,
    }
}
fn region(slot: usize) -> AttachmentRef {
    AttachmentRef {
        skin: "default".into(),
        slot: slot_name(slot).into(),
        placeholder: slot_name(slot).into(),
    }
}
fn slot_name(slot: usize) -> &'static str {
    [
        "zeta sibling",
        "alpha sibling",
        "child slot",
        "unrelated slot",
        "quiet slot",
    ][slot]
}
fn document() -> Document {
    let mut doc = Document::new("Original compound timing family".into(), 60).unwrap();
    for (name, parent, x) in [
        ("family", 0, 50.),
        ("child", 1, 40.),
        ("unrelated", 0, -120.),
        ("quiet", 0, 160.),
    ] {
        doc.bones.push(Bone {
            name: name.into(),
            parent: Some(parent),
            length: 30.,
            setup: Transform {
                x,
                ..Default::default()
            },
            inherit: Inherit::Normal,
        });
    }
    for (slot, bone) in [1, 1, 2, 3, 4].into_iter().enumerate() {
        let name = slot_name(slot);
        doc.slots.push(Slot {
            name: name.into(),
            bone,
            attachment: Some(name.into()),
            color: WHITE,
            blend: BlendMode::Normal,
        });
        doc.skins.entry("default".into()).or_default().insert(
            name.into(),
            BTreeMap::from([(
                name.into(),
                Attachment::Region(Region {
                    name: None,
                    path: "blue_cutout".into(),
                    width: 128.,
                    height: 128.,
                    transform: RegionTransform {
                        scale_x: 0.125,
                        scale_y: 0.125,
                        ..Default::default()
                    },
                    color: WHITE,
                    sequence: None,
                }),
            )]),
        );
    }
    let mut clip = Animation::new(
        "family host",
        vec![
            Track {
                bone: 1,
                property: Property::Rotation,
                keys: vec![scalar(0.12345, 13.), scalar(0.7, -17.)],
            },
            Track {
                bone: 2,
                property: Property::X,
                keys: vec![scalar(0.3, 4.)],
            },
            Track {
                bone: 3,
                property: Property::Y,
                keys: vec![scalar(0.2, -8.)],
            },
        ],
    );
    clip.attachment_tracks = [0, 1]
        .into_iter()
        .map(|slot| AttachmentTrack {
            slot,
            keys: vec![AttachmentKey {
                time: if slot == 0 { 0.15 } else { 0.4 },
                attachment: Some(slot_name(slot).into()),
            }],
        })
        .collect();
    clip.color_tracks = [0, 2, 3]
        .into_iter()
        .map(|slot| ColorTrack {
            slot,
            keys: if slot == 0 {
                vec![
                    ColorKey {
                        time: 0.25,
                        color: [30, 100, 190, 255],
                        curves: [Curve::BezierOffset([0.2, 0.125, 0.8, -0.25]); 4],
                    },
                    ColorKey {
                        time: 0.75,
                        color: [200, 40, 90, 160],
                        curves: [Curve::Linear; 4],
                    },
                ]
            } else {
                vec![ColorKey {
                    time: 0.3,
                    color: [60, 80, 100, 200],
                    curves: [Curve::Linear; 4],
                }]
            },
        })
        .collect();
    clip.draw_order = vec![
        DrawOrderKey {
            time: 0.5,
            offsets: vec![],
        },
        DrawOrderKey {
            time: 1.,
            offsets: vec![],
        },
    ];
    doc.animations = vec![clip, Animation::new("other host", vec![])];
    doc.validate().unwrap();
    doc
}
fn fixture() -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    editor.session = Session::new(document()).unwrap();
    editor.skin = "default".into();
    editor.session.set_skin(&editor.skin);
    editor.tree = TreeState::for_document(editor.session.document(), &editor.skin);
    editor.animation = 0;
    editor.time = 0.4;
    editor.key_selection = Default::default();
    editor.restore_selection(None);
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
fn center(editor: &Editor, name: &str) -> Pos2 {
    let [l, t, r, b] = *editor.capture_targets.get(name).unwrap_or_else(|| {
        panic!(
            "missing workspace target {name}; selection={:?}, Graph={}, Dopesheet={}, nearby targets={:?}",
            editor.selection,
            editor.graph_visible(),
            editor.dopesheet_visible(),
            editor.capture_targets.keys().filter(|key| key.contains("slot") || key.contains("Slot") || key.starts_with("timeline")).take(24).collect::<Vec<_>>(),
        )
    });
    assert!(r > l && b > t, "empty input target {name}");
    pos2((l + r) * 0.5, (t + b) * 0.5)
}
fn pointer(
    editor: &mut Editor,
    ctx: &egui::Context,
    point: Pos2,
    pressed: bool,
    modifiers: egui::Modifiers,
) {
    workspace_frame(
        editor,
        ctx,
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
fn scroll(editor: &mut Editor, ctx: &egui::Context, delta: f32) {
    let point = center(editor, "timelineRowsClip");
    workspace_frame(
        editor,
        ctx,
        vec![
            egui::Event::PointerMoved(point),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0., delta),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    // Wait for egui's scroll animation before using captured row positions.
    for _ in 0..12 {
        workspace_frame(editor, ctx, vec![]);
    }
}
fn target(editor: &mut Editor, ctx: &egui::Context, name: &str) -> Pos2 {
    scroll(editor, ctx, 2000.);
    for _ in 0..20 {
        if editor.capture_targets.contains_key(name) {
            return center(editor, name);
        }
        scroll(editor, ctx, -48.);
    }
    panic!("workspace target {name} was never shown");
}
fn click(editor: &mut Editor, ctx: &egui::Context, name: &str, modifiers: egui::Modifiers) {
    let point = if name.starts_with("timelineKey:") || name.starts_with("timelineRow:") {
        target(editor, ctx, name)
    } else {
        center(editor, name)
    };
    workspace_frame(editor, ctx, vec![egui::Event::PointerMoved(point)]);
    pointer(editor, ctx, point, true, modifiers);
    pointer(editor, ctx, point, false, egui::Modifiers::NONE);
}
fn key_event(key: egui::Key, modifiers: egui::Modifiers, pressed: bool) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    }
}
fn command_key(editor: &mut Editor, ctx: &egui::Context, key: egui::Key) {
    ctx.memory_mut(|memory| memory.request_focus(Editor::dopesheet_id()));
    workspace_frame(
        editor,
        ctx,
        vec![key_event(key, egui::Modifiers::COMMAND, true)],
    );
    workspace_frame(
        editor,
        ctx,
        vec![key_event(key, egui::Modifiers::NONE, false)],
    );
}
fn event(editor: &mut Editor, ctx: &egui::Context, event: egui::Event) {
    let focus = if editor.graph_visible() {
        Editor::graph_id()
    } else {
        Editor::dopesheet_id()
    };
    ctx.memory_mut(|memory| memory.request_focus(focus));
    workspace_frame(editor, ctx, vec![event]);
}
fn family_scope(editor: &Editor) {
    let TimelineScope::Owners(owners) = editor.timeline_scope() else {
        panic!("family must be explicitly scoped")
    };
    assert_eq!(owners.first(), Some(&KeyOwner::Bone { bone: 1 }));
    assert_eq!(owners.last(), Some(&KeyOwner::DrawOrder {}));
    assert_eq!(
        owners.into_iter().collect::<BTreeSet<_>>(),
        BTreeSet::from([
            KeyOwner::Bone { bone: 1 },
            KeyOwner::Slot { slot: 0 },
            KeyOwner::Slot { slot: 1 },
            KeyOwner::DrawOrder {}
        ])
    );
    assert!(editor.key_selection.shown().keys.iter().all(|key| matches!(
        key.owner(),
        KeyOwner::Bone { bone: 1 } | KeyOwner::Slot { slot: 0 | 1 } | KeyOwner::DrawOrder {}
    )));
}
fn scan_rows(editor: &mut Editor, ctx: &egui::Context) -> BTreeSet<String> {
    scroll(editor, ctx, 2000.);
    let mut names = BTreeSet::new();
    let mut unchanged = 0;
    for _ in 0..20 {
        let before = names.len();
        names.extend(editor.capture_targets.keys().cloned());
        unchanged = if names.len() == before {
            unchanged + 1
        } else {
            0
        };
        if unchanged == 2 {
            break;
        }
        scroll(editor, ctx, -48.);
    }
    names
}

#[test]
fn compound_dopesheet_slot_draw_order_slot_and_region_show_only_direct_family_and_keyed_channels() {
    for selected in [
        TreeItem::Slot(0),
        TreeItem::DrawOrderSlot { slot: 1 },
        TreeItem::Attachment(region(0)),
    ] {
        let (mut editor, ctx) = fixture();
        editor.select(selected.clone());
        settle(&mut editor, &ctx);
        family_scope(&editor);
        assert_eq!(editor.selection, Some(selected));
        assert!(
            center(&editor, "timelineRow:slot:0:color").y
                < center(&editor, "timelineRow:slot:0:attachment").y
        );
        let rows = scan_rows(&mut editor, &ctx);
        for required in [
            "timelineOwner:bone:1",
            "timelineRow:bone:1:Rotation",
            "timelineRow:slot:0:color",
            "timelineRow:slot:0:attachment",
            "timelineRow:slot:1:attachment",
            "timelineRow:drawOrder:Default",
        ] {
            assert!(rows.contains(required), "missing {required}");
        }
        for excluded in [
            "timelineRow:bone:1:X",
            "timelineRow:bone:1:ScaleX",
            "timelineRow:slot:1:color",
            "timelineOwner:bone:2",
            "timelineOwner:bone:3",
            "timelineRow:slot:2:color",
            "timelineRow:slot:3:color",
        ] {
            assert!(
                !rows.contains(excluded),
                "out-of-scope or unkeyed row {excluded}"
            );
        }
        assert_eq!(editor.session.document(), &document());
    }
}

#[test]
fn compound_dopesheet_three_siblings_sort_all_rgba_before_all_attachment_rows_by_name() {
    let (mut editor, ctx) = fixture();
    let mut doc = editor.session.document().clone();
    let middle = doc.slots.len();
    let name = "middle sibling";
    let original_region = doc.attachment(&region(0)).unwrap().clone();
    doc.slots.push(Slot {
        name: name.into(),
        bone: 1,
        attachment: Some(name.into()),
        color: WHITE,
        blend: BlendMode::Normal,
    });
    doc.skins.get_mut("default").unwrap().insert(
        name.into(),
        BTreeMap::from([(name.into(), original_region)]),
    );
    for slot in [1, middle] {
        doc.animations[0].color_tracks.push(ColorTrack {
            slot,
            keys: vec![ColorKey {
                time: 0.35,
                color: [20, 40, 60, 200],
                curves: [Curve::Linear; 4],
            }],
        });
    }
    doc.animations[0].attachment_tracks.push(AttachmentTrack {
        slot: middle,
        keys: vec![AttachmentKey {
            time: 0.45,
            attachment: Some(name.into()),
        }],
    });
    assert_eq!(
        doc.slots
            .iter()
            .filter(|slot| slot.bone == 1)
            .map(|slot| slot.name.as_str())
            .collect::<Vec<_>>(),
        vec!["zeta sibling", "alpha sibling", "middle sibling"],
        "setup and reverse setup order must both differ from name order",
    );
    editor.session = Session::new(doc).unwrap();
    editor.session.set_skin(&editor.skin);
    editor.key_selection = Default::default();
    editor.select(TreeItem::Slot(middle));
    settle(&mut editor, &ctx);
    let expected = vec![
        "timelineRow:bone:1:Rotation".to_string(),
        "timelineRow:slot:1:color".to_string(),
        format!("timelineRow:slot:{middle}:color"),
        "timelineRow:slot:0:color".to_string(),
        "timelineRow:slot:1:attachment".to_string(),
        format!("timelineRow:slot:{middle}:attachment"),
        "timelineRow:slot:0:attachment".to_string(),
    ];
    scroll(&mut editor, &ctx, 2000.);
    let mut observed: Vec<String> = Vec::new();
    for _ in 0..20 {
        let mut visible: Vec<_> = editor
            .capture_targets
            .iter()
            .filter(|(name, _)| expected.contains(name) && !observed.contains(name))
            .map(|(name, rect)| (name.clone(), (rect[1] + rect[3]) * 0.5))
            .collect();
        visible.sort_by(|a, b| a.1.total_cmp(&b.1));
        observed.extend(visible.into_iter().map(|(name, _)| name));
        if observed.len() == expected.len() {
            break;
        }
        scroll(&mut editor, &ctx, -48.);
    }
    assert_eq!(
        observed, expected,
        "middle-name attachment must follow even the last sibling RGBA row"
    );
    assert!(!editor.session.can_undo());
}

#[test]
fn compound_dopesheet_unkeyed_parent_retains_header_without_empty_scalar_channels() {
    let (mut editor, ctx) = fixture();
    editor.select(TreeItem::Slot(4));
    settle(&mut editor, &ctx);
    assert_eq!(
        editor.timeline_scope(),
        TimelineScope::Owners(vec![
            KeyOwner::Bone { bone: 4 },
            KeyOwner::Slot { slot: 4 },
            KeyOwner::DrawOrder {}
        ])
    );
    let rows = scan_rows(&mut editor, &ctx);
    assert!(rows.contains("timelineOwner:bone:4"));
    assert!(rows.contains("timelineRow:drawOrder:Default"));
    assert!(
        !rows
            .iter()
            .any(|name| name.starts_with("timelineRow:bone:4:")
                || name.starts_with("timelineRow:slot:4:"))
    );
    assert!(editor.key_selection.is_empty());
}

#[test]
fn compound_dopesheet_real_mixed_drag_escape_restores_exact_primary_pending_redo_and_expansion() {
    let (mut editor, ctx) = fixture();
    editor.select(TreeItem::Slot(0));
    settle(&mut editor, &ctx);
    for name in [
        "timelineKey:bone:1:Rotation:0",
        "timelineKey:slot:0:color:0",
        "timelineKey:drawOrder:Default:0",
    ] {
        click(&mut editor, &ctx, name, egui::Modifiers::COMMAND);
    }
    command_key(&mut editor, &ctx, egui::Key::A);
    assert_eq!(editor.key_selection.len(), 6);
    let sources = editor.key_selection.keys().to_vec();
    let primary = editor.key_selection.primary().cloned();
    assert!(sources.contains(&bone_key(0.12345)));
    let mut pose = editor.session.local_pose(Some(0), editor.time)[1];
    pose.x += 9.25;
    editor
        .session
        .edit_pose(
            1,
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
            name: "redo witness".into(),
        })
        .unwrap();
    editor.session.undo();
    assert!(editor.session.can_redo());
    let before = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let origin = target(&mut editor, &ctx, "timelineKey:bone:1:Rotation:0");
    pointer(&mut editor, &ctx, origin, true, egui::Modifiers::NONE);
    for distance in [42., 68.] {
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(origin + vec2(distance, 0.))],
        );
    }
    assert!(editor.key_drag.as_ref().is_some_and(|drag| drag.active));
    let delta = editor.key_drag.as_ref().unwrap().offset;
    let expected = peachy_engine::apply_command(
        &before,
        Command::MoveAnimationKeys {
            animation: 0,
            keys: sources.clone(),
            delta,
            copy: false,
        },
    )
    .unwrap();
    assert_eq!(editor.session.document(), &expected);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![key_event(egui::Key::Escape, egui::Modifiers::NONE, true)],
    );
    assert_eq!(editor.session.document(), &before);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.key_selection.keys(), sources);
    assert_eq!(editor.key_selection.primary(), primary.as_ref());
    assert!(editor.session.can_redo());
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(origin + vec2(90., 0.))],
    );
    pointer(
        &mut editor,
        &ctx,
        origin + vec2(90., 0.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(editor.session.document(), &before);
    command_key(&mut editor, &ctx, egui::Key::A);
    assert_eq!(
        editor.key_selection.len(),
        8,
        "cancel must restore the completed row-expansion phase"
    );
}

#[test]
fn compound_dopesheet_sibling_tree_change_preserves_primary_graph_but_uses_explicit_slot_paste_target()
 {
    for selected in [TreeItem::Slot(1), TreeItem::DrawOrderSlot { slot: 1 }] {
        let (mut editor, ctx) = fixture();
        editor.select(TreeItem::Slot(0));
        settle(&mut editor, &ctx);
        click(
            &mut editor,
            &ctx,
            "timelineKey:slot:0:color:0",
            egui::Modifiers::COMMAND,
        );
        let primary = AnimationKey::SlotColor {
            slot: 0,
            time: 0.25,
        };
        assert_eq!(editor.key_selection.primary(), Some(&primary));
        event(&mut editor, &ctx, egui::Event::Copy);
        assert_eq!(editor.timeline_clipboard_diagnostic()["kind"], "single");
        let copied = editor.timeline_clipboard.clone().unwrap();
        let original_scope = editor.timeline_scope();
        if matches!(&selected, TreeItem::DrawOrderSlot { .. }) {
            click(&mut editor, &ctx, "filter:Slots", egui::Modifiers::NONE);
            settle(&mut editor, &ctx);
        }
        editor.tree.reveal(
            editor.session.document(),
            &editor.skin,
            editor.setup_mode,
            &selected,
        );
        settle(&mut editor, &ctx);
        click(&mut editor, &ctx, &selected.key(), egui::Modifiers::NONE);
        settle(&mut editor, &ctx);
        assert_eq!(editor.timeline_scope(), original_scope);
        assert_eq!(editor.selection, Some(selected.clone()));
        assert_eq!(
            editor.timeline_owner(),
            Some(if matches!(selected, TreeItem::Slot(_)) {
                KeyOwner::Slot { slot: 1 }
            } else {
                KeyOwner::DrawOrder {}
            })
        );
        assert_eq!(editor.graph_owner(), Some(KeyOwner::Slot { slot: 0 }));
        assert_eq!(editor.key_selection.primary(), Some(&primary));
        editor.open_graph_view();
        settle(&mut editor, &ctx);
        assert_eq!(editor.slot_curve_diagnostic()["context"]["slot"], 0);
        editor.set_time(0.8);
        let before = editor.session.document().clone();
        let portable =
            serde_json::json!({"peachyAnimatorClipboard":"keys","version":2,"payload":copied})
                .to_string();
        event(&mut editor, &ctx, egui::Event::Paste(portable));
        assert!(editor.error.is_none(), "{:?}", editor.error);
        assert_eq!(editor.selection, Some(selected));
        assert_eq!(editor.key_selection.primary(), Some(&primary));
        assert_eq!(editor.graph_owner(), Some(KeyOwner::Slot { slot: 0 }));
        assert_eq!(
            editor.session.document().animations[0]
                .color_tracks
                .iter()
                .find(|track| track.slot == 0),
            before.animations[0]
                .color_tracks
                .iter()
                .find(|track| track.slot == 0)
        );
        let incoming: Vec<AnimationKey> = serde_json::from_value(
            editor.timeline_clipboard_diagnostic()["lastPaste"]["acceptedKeys"].clone(),
        )
        .unwrap();
        assert_eq!(
            incoming,
            vec![AnimationKey::SlotColor { slot: 1, time: 0.8 }]
        );
        editor.undo();
        assert_eq!(editor.session.document(), &before);
    }
}

#[test]
fn compound_dopesheet_whole_family_group_copy_delete_preserves_unrelated_domains_and_one_undo() {
    let (mut editor, ctx) = fixture();
    editor.select(TreeItem::DrawOrderSlot { slot: 0 });
    settle(&mut editor, &ctx);
    command_key(&mut editor, &ctx, egui::Key::A);
    assert_eq!(editor.key_selection.len(), 8);
    let selected = editor.key_selection.keys().to_vec();
    let before = editor.session.document().clone();
    event(&mut editor, &ctx, egui::Event::Copy);
    let receipt = editor.timeline_clipboard_diagnostic();
    assert_eq!(receipt["kind"], "group");
    assert_eq!(receipt["entryCount"], 4);
    assert_eq!(receipt["keys"], 8);
    assert_eq!(editor.session.document(), &before);
    event(
        &mut editor,
        &ctx,
        key_event(egui::Key::Delete, egui::Modifiers::NONE, true),
    );
    assert!(editor.error.is_none(), "{:?}", editor.error);
    assert_eq!(
        editor.session.document().animations[0].tracks,
        before.animations[0]
            .tracks
            .iter()
            .filter(|track| track.bone != 1)
            .cloned()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        editor.session.document().animations[0].color_tracks,
        before.animations[0]
            .color_tracks
            .iter()
            .filter(|track| track.slot != 0)
            .cloned()
            .collect::<Vec<_>>()
    );
    assert!(
        editor.session.document().animations[0]
            .attachment_tracks
            .is_empty()
    );
    assert!(
        editor.session.document().animations[0]
            .draw_order
            .is_empty()
    );
    assert!(selected.iter().all(|key| matches!(
        key.owner(),
        KeyOwner::Bone { bone: 1 } | KeyOwner::Slot { slot: 0 | 1 } | KeyOwner::DrawOrder {}
    )));
    editor.undo();
    assert_eq!(editor.session.document(), &before);
    assert!(!editor.session.can_undo());
}

#[test]
fn compound_dopesheet_unsupported_and_inactive_region_scopes_refuse_implicit_partial_group_edits() {
    let (mut editor, ctx) = fixture();
    let mut doc = editor.session.document().clone();
    doc.animations[0].inherit_tracks.push(InheritTrack {
        bone: 1,
        keys: vec![InheritKey {
            time: 0.25,
            inherit: Inherit::Normal,
        }],
    });
    doc.skin_scope.required_bones.extend([1, 2]);
    doc.skins.insert("shown".into(), BTreeMap::new());
    doc.skins.insert("hidden".into(), BTreeMap::new());
    doc.skin_scope.skins.insert(
        "shown".into(),
        SkinMembers {
            bones: vec![1, 2],
            constraints: vec![],
        },
    );
    editor.session = Session::new(doc).unwrap();
    editor.skin = "shown".into();
    editor.session.set_skin(&editor.skin);
    editor.key_selection = Default::default();
    editor.select(TreeItem::Attachment(region(0)));
    settle(&mut editor, &ctx);
    family_scope(&editor);
    assert_eq!(editor.key_selection.shown().unsupported.len(), 1);
    let before = editor.session.document().clone();
    command_key(&mut editor, &ctx, egui::Key::A);
    assert!(editor.key_selection.is_empty());
    assert!(editor.status.contains("read-only"));
    assert_eq!(editor.session.document(), &before);
    assert!(!editor.session.can_undo());
    click(
        &mut editor,
        &ctx,
        "timelineKey:bone:1:Rotation:0",
        egui::Modifiers::COMMAND,
    );
    event(&mut editor, &ctx, egui::Event::Copy);
    assert_eq!(editor.timeline_clipboard_diagnostic()["kind"], "single");
    let copied = editor.timeline_clipboard.clone();
    let source = editor.leaf_source.clone();
    editor.activate_skin("hidden".into());
    settle(&mut editor, &ctx);
    family_scope(&editor);
    assert!(editor.key_selection.is_empty());
    assert!(editor.graph_owner().is_none());
    assert_eq!(editor.key_selection.shown().readonly_owners.len(), 4);
    command_key(&mut editor, &ctx, egui::Key::A);
    assert!(editor.key_selection.is_empty());
    assert!(!editor.paste_dopesheet_keys());
    assert_eq!(editor.session.document(), &before);
    assert_eq!(editor.timeline_clipboard, copied);
    assert_eq!(editor.leaf_source, source);
    assert!(!editor.session.can_undo());
}
