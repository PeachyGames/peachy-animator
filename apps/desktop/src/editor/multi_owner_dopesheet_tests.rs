//! Full-workspace input exercises mixed exact identities, not a second timeline model.
use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
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
    let track = |bone, property, frames: &[f32]| Track {
        bone,
        property,
        keys: frames
            .iter()
            .map(|frame| Keyframe {
                time: frame / 60.,
                value: *frame,
                curve: Curve::BezierOffset([0.2, -0., 0.8, 7.25]),
            })
            .collect(),
    };
    doc.animations[0].tracks = vec![
        track(8, Property::Rotation, &[5., 20., 45., 60.]),
        track(7, Property::Y, &[0., 40., 60.]),
    ];
    editor.session = Session::new(doc).unwrap();
    editor.key_selection = Default::default();
    editor.selection = Default::default();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    (editor, ctx)
}
fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..3 {
        workspace_frame(editor, ctx, vec![]);
    }
}
fn target(editor: &Editor, name: &str) -> Pos2 {
    let [left, top, right, bottom] = editor.capture_targets[name];
    assert!(right > left && bottom > top, "{name}");
    pos2((left + right) * 0.5, (top + bottom) * 0.5)
}
fn pointer(
    editor: &mut Editor,
    ctx: &egui::Context,
    position: Pos2,
    pressed: bool,
    modifiers: egui::Modifiers,
) {
    workspace_frame(
        editor,
        ctx,
        vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers,
            },
        ],
    );
}
fn click(editor: &mut Editor, ctx: &egui::Context, name: &str, modifiers: egui::Modifiers) {
    let point = target(editor, name);
    workspace_frame(editor, ctx, vec![egui::Event::PointerMoved(point)]);
    pointer(editor, ctx, point, true, modifiers);
    pointer(editor, ctx, point, false, modifiers);
}
fn keyboard(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}
fn select_group(editor: &mut Editor, ctx: &egui::Context) {
    click(
        editor,
        ctx,
        "timelineKey:bone:8:Rotation:0",
        egui::Modifiers::NONE,
    );
    click(
        editor,
        ctx,
        "timelineKey:bone:8:Rotation:1",
        egui::Modifiers::COMMAND,
    );
    click(
        editor,
        ctx,
        "timelineKey:bone:7:Y:1",
        egui::Modifiers::COMMAND,
    );
    assert_eq!(
        editor.key_selection.keys(),
        &[
            key(8, Property::Rotation, 5.),
            key(8, Property::Rotation, 20.),
            key(7, Property::Y, 40.),
        ]
    );
}

#[test]
fn multi_owner_dopesheet_lists_keyed_owners_and_owner_name_prunes_without_selecting_keys() {
    let (mut editor, ctx) = fixture();
    assert!(editor.capture_targets.contains_key("timelineOwner:bone:7"));
    assert!(editor.capture_targets.contains_key("timelineOwner:bone:8"));
    assert!(target(&editor, "timelineOwner:bone:7").y < target(&editor, "timelineOwner:bone:8").y);
    let before = editor.session.document().clone();
    click(
        &mut editor,
        &ctx,
        "timelineOwner:bone:7",
        egui::Modifiers::NONE,
    );
    settle(&mut editor, &ctx);
    assert_eq!(editor.selection, Some(TreeItem::Bone(7)));
    assert!(editor.key_selection.is_empty());
    assert!(
        editor
            .capture_targets
            .contains_key("timelineKey:bone:7:Y:1")
    );
    assert!(
        !editor
            .capture_targets
            .contains_key("timelineKey:bone:8:Rotation:0")
    );
    assert_eq!(editor.session.document(), &before);
    assert!(!editor.session.can_undo());
}

#[test]
fn multi_owner_dopesheet_real_cmd_selection_and_two_previews_retime_three_keys_once() {
    let (mut editor, ctx) = fixture();
    select_group(&mut editor, &ctx);
    assert!(editor.selection.is_empty());
    assert_eq!(editor.graph_owner(), Some(KeyOwner::Bone { bone: 8 }));
    let before = editor.session.document().clone();
    let sources = editor.key_selection.keys().to_vec();
    let origin = target(&editor, "timelineKey:bone:7:Y:1");
    let [left, _, right, _] = editor.capture_targets["timelineTrack"];
    let five = (right - left) * 5. / 60.;
    pointer(&mut editor, &ctx, origin, true, egui::Modifiers::NONE);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(origin + vec2(five * 0.6, 0.))],
    );
    assert!(editor.key_drag.as_ref().is_some_and(|drag| drag.active));
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(origin + vec2(five, 0.))],
    );
    assert!(editor.error.is_none(), "{:?}", editor.error);
    // The anchor snaps to a frame; the exact shared f32 delta is applied to
    // every source. Other stored times are never independently canonicalized.
    let accepted_delta = editor.key_drag.as_ref().unwrap().offset;
    let raw_expected = peachy_engine::apply_command(
        &before,
        Command::MoveAnimationKeys {
            animation: 0,
            keys: sources,
            delta: accepted_delta,
            copy: false,
        },
    )
    .unwrap();
    pointer(
        &mut editor,
        &ctx,
        origin + vec2(five, 0.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(
        editor.key_selection.keys(),
        &[
            key(8, Property::Rotation, 5.).with_delta(accepted_delta),
            key(8, Property::Rotation, 20.).with_delta(accepted_delta),
            key(7, Property::Y, 40.).with_delta(accepted_delta),
        ]
    );
    let after = editor.session.document().clone();
    assert_eq!(after, raw_expected);
    assert_eq!(
        editor
            .key_selection
            .keys()
            .iter()
            .map(|key| (key.time() * 60.).round() as u32)
            .collect::<Vec<_>>(),
        vec![10, 25, 45]
    );
    for (old, new) in before.animations[0]
        .tracks
        .iter()
        .zip(&after.animations[0].tracks)
    {
        for (index, (a, b)) in old.keys.iter().zip(&new.keys).enumerate() {
            assert_eq!(a.curve, b.curve);
            assert_eq!(a.value.to_bits(), b.value.to_bits());
            let moved = if old.bone == 8 { index < 2 } else { index == 1 };
            assert_eq!(
                b.time,
                if moved {
                    a.time + accepted_delta
                } else {
                    a.time
                }
            );
        }
    }
    assert_eq!(editor.graph_owner(), Some(KeyOwner::Bone { bone: 7 }));
    editor.undo();
    assert_eq!(editor.session.document(), &before);
    editor.redo();
    assert_eq!(editor.session.document(), &after);
}

trait ShiftKey {
    fn with_delta(self, delta: f32) -> Self;
}
impl ShiftKey for AnimationKey {
    fn with_delta(self, delta: f32) -> Self {
        match self {
            Self::Bone {
                bone,
                property,
                time,
            } => Self::Bone {
                bone,
                property,
                time: time + delta,
            },
            _ => unreachable!(),
        }
    }
}

#[test]
fn multi_owner_dopesheet_escape_restores_primary_pending_redo_and_blocks_held_restart() {
    let (mut editor, ctx) = fixture();
    select_group(&mut editor, &ctx);
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
    let before = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let selected = editor.key_selection.keys().to_vec();
    let primary = editor.key_selection.primary().cloned();
    let origin = target(&editor, "timelineKey:bone:7:Y:1");
    pointer(&mut editor, &ctx, origin, true, egui::Modifiers::NONE);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(origin + vec2(70., 0.))],
    );
    assert_ne!(editor.session.document(), &before);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![keyboard(egui::Key::Escape, egui::Modifiers::NONE)],
    );
    assert_eq!(editor.session.document(), &before);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.key_selection.keys(), selected);
    assert_eq!(editor.key_selection.primary(), primary.as_ref());
    assert!(editor.session.can_redo());
    assert!(editor.key_drag.is_none());
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(origin + vec2(95., 0.))],
    );
    assert_eq!(editor.session.document(), &before);
    pointer(
        &mut editor,
        &ctx,
        origin + vec2(95., 0.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(editor.session.document(), &before);
}

#[test]
fn multi_owner_dopesheet_readonly_metadata_remains_visible_and_select_all_cannot_hide_partial_scope()
 {
    let (mut editor, ctx) = fixture();
    let mut doc = editor.session.document().clone();
    doc.animations[0].inherit_tracks.push(InheritTrack {
        bone: 8,
        keys: vec![InheritKey {
            time: 0.25,
            inherit: Inherit::Normal,
        }],
    });
    doc.events.push(EventDef::new("original cue"));
    doc.animations[0].events.push(EventKey {
        time: 0.5,
        event: 0,
        int: None,
        float: None,
        string: None,
        volume: None,
        balance: None,
    });
    editor.session = Session::new(doc).unwrap();
    editor.key_selection = Default::default();
    settle(&mut editor, &ctx);
    assert_eq!(editor.key_selection.shown().unsupported.len(), 2);
    assert!(
        editor
            .capture_targets
            .keys()
            .any(|name| name.starts_with("timelineReadOnly:"))
    );
    let before = editor.session.document().clone();
    ctx.memory_mut(|memory| memory.request_focus(Editor::dopesheet_id()));
    workspace_frame(
        &mut editor,
        &ctx,
        vec![keyboard(egui::Key::A, egui::Modifiers::COMMAND)],
    );
    assert!(editor.key_selection.is_empty());
    assert!(editor.status.contains("read-only"));
    click(
        &mut editor,
        &ctx,
        "timelineKey:bone:8:Rotation:0",
        egui::Modifiers::NONE,
    );
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }],
    );
    workspace_frame(
        &mut editor,
        &ctx,
        vec![keyboard(egui::Key::A, egui::Modifiers::COMMAND)],
    );
    assert_eq!(editor.key_selection.len(), 4);
    assert!(
        editor
            .key_selection
            .keys()
            .iter()
            .all(|key| key.owner() == KeyOwner::Bone { bone: 8 })
    );
    assert_eq!(editor.session.document(), &before);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }],
    );
    workspace_frame(
        &mut editor,
        &ctx,
        vec![keyboard(egui::Key::A, egui::Modifiers::COMMAND)],
    );
    assert_eq!(editor.key_selection.len(), 4);
    assert!(editor.status.contains("read-only"));
    editor.delete_dopesheet_keys();
    assert_eq!(
        editor.session.document().animations[0].inherit_tracks,
        before.animations[0].inherit_tracks
    );
    assert_eq!(
        editor.session.document().animations[0].events,
        before.animations[0].events
    );
    assert!(
        editor.session.document().animations[0]
            .tracks
            .iter()
            .all(|track| track.bone != 8)
    );
    editor.undo();
    assert_eq!(editor.session.document(), &before);
}

#[test]
fn multi_owner_dopesheet_mixed_clipboard_and_delete_keep_exact_scope() {
    let (mut editor, ctx) = fixture();
    select_group(&mut editor, &ctx);
    let before = editor.session.document().clone();
    assert!(editor.copy_dopesheet_keys(&ctx));
    assert!(editor.timeline_clipboard.is_none());
    assert_eq!(editor.timeline_clipboard_diagnostic()["entryCount"], 2);
    assert_eq!(editor.session.document(), &before);
    assert!(!editor.session.can_undo());
    editor.delete_dopesheet_keys();
    assert!(editor.error.is_none(), "{:?}", editor.error);
    assert_eq!(
        editor.session.document().animations[0].tracks[0]
            .keys
            .iter()
            .map(|key| key.time)
            .collect::<Vec<_>>(),
        vec![45. / 60., 1.]
    );
    assert_eq!(
        editor.session.document().animations[0].tracks[1]
            .keys
            .iter()
            .map(|key| key.time)
            .collect::<Vec<_>>(),
        vec![0., 1.]
    );
    editor.undo();
    assert_eq!(editor.session.document(), &before);
}

#[test]
fn multi_owner_dopesheet_slider_primary_dispatches_graph_without_changing_tree_or_losing_bone_keys()
{
    let (mut editor, ctx) = fixture();
    let mut doc = editor.session.document().clone();
    let mut slider = SliderConstraint::new("Original timeline slider", 0);
    slider.setup.mix = 0.;
    let constraint = doc.constraints.len();
    doc.constraints.push(Constraint::Slider(slider));
    doc.animations[0].slider_tracks.push(SliderTrack {
        constraint,
        property: SliderProperty::Time,
        keys: vec![
            Keyframe {
                time: 0.,
                value: 0.,
                curve: Curve::Linear,
            },
            Keyframe {
                time: 0.5,
                value: 0.5,
                curve: Curve::Linear,
            },
        ],
    });
    doc.animations[0].slider_tracks.push(SliderTrack {
        constraint,
        property: SliderProperty::Mix,
        keys: vec![
            Keyframe {
                time: 0.,
                value: 0.,
                curve: Curve::Bezier([0.25, 0., 0.75, 1.]),
            },
            Keyframe {
                time: 0.5,
                value: 1.,
                curve: Curve::Linear,
            },
        ],
    });
    editor.session = Session::new(doc).unwrap();
    editor.key_selection = Default::default();
    settle(&mut editor, &ctx);
    click(
        &mut editor,
        &ctx,
        "timelineKey:bone:8:Rotation:0",
        egui::Modifiers::NONE,
    );
    editor.key_selection.select(
        AnimationKey::Slider {
            constraint,
            property: SliderProperty::Time,
            time: 0.,
        },
        true,
    );
    let selected = AnimationKey::Slider {
        constraint,
        property: SliderProperty::Mix,
        time: 0.,
    };
    editor.key_selection.select(selected.clone(), true);
    editor.key_selection.set_primary(Some(selected));
    let keys = editor.key_selection.keys().to_vec();
    editor.open_curves_view();
    editor.open_graph_view();
    settle(&mut editor, &ctx);
    assert!(editor.selection.is_empty());
    assert_eq!(editor.graph_owner(), Some(KeyOwner::Slider { constraint }));
    assert_eq!(editor.key_selection.keys(), keys);
    assert!(editor.capture_targets.contains_key("graphSliderCanvas"));
    assert_eq!(editor.curve_preset_selection().len(), 3);
    assert_eq!(editor.graph_diagnostic()["context"]["property"], "mix");
    assert_eq!(editor.curves_diagnostic()["primary"][0]["property"], "mix");
    click(
        &mut editor,
        &ctx,
        "graphSliderProperty:Time",
        egui::Modifiers::NONE,
    );
    settle(&mut editor, &ctx);
    assert_eq!(editor.graph_owner(), Some(KeyOwner::Slider { constraint }));
    assert_eq!(editor.graph_diagnostic()["context"]["property"], "time");
    assert!(editor.key_selection.is_empty());
    assert!(!editor.session.can_undo());
}

#[test]
fn multi_owner_dopesheet_cached_metadata_reuses_pose_only_frames_and_refreshes_on_undo() {
    let (mut editor, ctx) = fixture();
    let shown = editor.key_selection.shown();
    settle(&mut editor, &ctx);
    assert!(std::sync::Arc::ptr_eq(
        &shown,
        &editor.key_selection.shown()
    ));
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
    settle(&mut editor, &ctx);
    assert!(std::sync::Arc::ptr_eq(
        &shown,
        &editor.key_selection.shown()
    ));
    editor
        .session
        .dispatch(Command::DeleteAnimationKeys {
            animation: 0,
            keys: vec![key(7, Property::Y, 40.)],
        })
        .unwrap();
    settle(&mut editor, &ctx);
    assert!(!std::sync::Arc::ptr_eq(
        &shown,
        &editor.key_selection.shown()
    ));
    assert!(
        !editor
            .key_selection
            .shown()
            .keys
            .contains(&key(7, Property::Y, 40.))
    );
    editor.undo();
    settle(&mut editor, &ctx);
    assert!(
        editor
            .key_selection
            .shown()
            .keys
            .contains(&key(7, Property::Y, 40.))
    );
    assert_eq!(
        editor
            .session
            .pose_edits()
            .locals(editor.session.document(), Some(0), editor.time)[8]
            .x,
        pose.x
    );
}

#[test]
fn multi_owner_dopesheet_graph_property_switch_retains_the_explicit_bone_with_no_tree_owner() {
    let (mut editor, ctx) = fixture();
    click(
        &mut editor,
        &ctx,
        "timelineKey:bone:8:Rotation:0",
        egui::Modifiers::NONE,
    );
    let before = editor.session.document().clone();
    editor.open_graph_view();
    settle(&mut editor, &ctx);
    click(
        &mut editor,
        &ctx,
        "graphPropertyPicker",
        egui::Modifiers::NONE,
    );
    settle(&mut editor, &ctx);
    click(&mut editor, &ctx, "graphProperty:X", egui::Modifiers::NONE);
    settle(&mut editor, &ctx);
    assert_eq!(editor.graph_owner(), Some(KeyOwner::Bone { bone: 8 }));
    assert!(editor.selection.is_empty());
    assert!(editor.key_selection.is_empty());
    assert_eq!(editor.graph_diagnostic()["context"]["bone"], 8);
    assert_eq!(editor.graph_diagnostic()["context"]["property"], "x");
    editor.select(TreeItem::Bone(7));
    settle(&mut editor, &ctx);
    assert_eq!(editor.graph_owner(), Some(KeyOwner::Bone { bone: 7 }));
    editor.restore_selection(None);
    settle(&mut editor, &ctx);
    assert!(editor.graph_owner().is_none());
    assert_eq!(editor.session.document(), &before);
    assert!(!editor.session.can_undo());
}

#[test]
fn multi_owner_dopesheet_pointer_group_copy_ruler_paste_cut_preserves_selection_and_toggle_pose() {
    for toggle in [egui::Modifiers::COMMAND, egui::Modifiers::CTRL] {
        let (mut editor, ctx) = fixture();
        let mut doc = editor.session.document().clone();
        for track in &mut doc.animations[0].tracks {
            let frames = if track.bone == 8 {
                [5., 25., 60.]
            } else {
                [12., 35., 60.]
            };
            track.keys = frames
                .into_iter()
                .map(|frame| Keyframe {
                    time: frame / 60.,
                    value: frame,
                    curve: Curve::BezierOffset([0.2, -0., 0.8, 7.25]),
                })
                .collect();
        }
        editor.session = Session::new(doc).unwrap();
        editor.session.set_skin(&editor.skin);
        editor.key_selection = Default::default();
        editor.reconcile_selection();
        settle(&mut editor, &ctx);
        let ruler_point = |editor: &Editor, frame: f32| {
            let [left, _, right, _] = editor.capture_targets["timelineTrack"];
            let [_, top, _, bottom] = editor.capture_targets["timelineRuler"];
            pos2(
                left + (right - left) * (frame / editor.session.document().fps as f32)
                    / editor.display_extent(),
                (top + bottom) * 0.5,
            )
        };
        let fifty = ruler_point(&editor, 50.);
        workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(fifty)]);
        pointer(&mut editor, &ctx, fifty, true, egui::Modifiers::NONE);
        pointer(&mut editor, &ctx, fifty, false, egui::Modifiers::NONE);
        assert_eq!((editor.time * 60.).round(), 50.);
        let cursor = editor.time;
        let mut pose = editor.session.local_pose(Some(0), cursor)[8];
        pose.x += 13.5;
        editor
            .session
            .edit_pose(
                8,
                EditTarget::Animate {
                    animation: 0,
                    time: cursor,
                },
                pose,
                &[Property::X],
                false,
            )
            .unwrap();
        let pending = editor.session.pose_edits().clone();
        let before = editor.session.document().clone();
        for (index, name) in [
            "timelineKey:bone:8:Rotation:0",
            "timelineKey:bone:8:Rotation:1",
            "timelineKey:bone:7:Y:0",
            "timelineKey:bone:7:Y:1",
        ]
        .into_iter()
        .enumerate()
        {
            let point = target(&editor, name);
            workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(point)]);
            pointer(&mut editor, &ctx, point, true, toggle);
            // Release modifiers can already be clear in the same OS input batch.
            pointer(&mut editor, &ctx, point, false, egui::Modifiers::NONE);
            assert_eq!(
                editor.time.to_bits(),
                cursor.to_bits(),
                "toggle moved cursor for {name}"
            );
            assert_eq!(
                editor.session.pose_edits(),
                &pending,
                "toggle discarded pending pose"
            );
            assert_eq!(editor.session.document(), &before);
            assert_eq!(editor.key_selection.len(), index + 1);
        }
        let selected = editor.key_selection.keys().to_vec();
        workspace_frame(&mut editor, &ctx, vec![egui::Event::Copy]);
        assert_eq!(editor.timeline_clipboard_diagnostic()["kind"], "group");
        assert_eq!(editor.session.pose_edits(), &pending);
        // A real ruler drag may change time and discard pending, but not keys.
        let forty = ruler_point(&editor, 40.);
        pointer(&mut editor, &ctx, fifty, true, egui::Modifiers::NONE);
        workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(forty)]);
        pointer(&mut editor, &ctx, forty, false, egui::Modifiers::NONE);
        assert_eq!((editor.time * 60.).round(), 40.);
        assert!(editor.session.pose_edits().is_empty());
        assert_eq!(editor.key_selection.keys(), selected);
        pointer(&mut editor, &ctx, fifty, true, egui::Modifiers::NONE);
        pointer(&mut editor, &ctx, fifty, false, egui::Modifiers::NONE);
        assert_eq!(editor.time.to_bits(), cursor.to_bits());
        assert_eq!(editor.key_selection.keys(), selected);
        click(&mut editor, &ctx, "timelinePaste", egui::Modifiers::NONE);
        assert_eq!(editor.key_selection.keys(), selected);
        assert_eq!(editor.time.to_bits(), cursor.to_bits());
        let accepted: Vec<AnimationKey> = serde_json::from_value(
            editor.timeline_clipboard_diagnostic()["lastPaste"]["acceptedKeys"].clone(),
        )
        .unwrap();
        assert_eq!(accepted.len(), 4);
        assert_eq!(
            accepted
                .iter()
                .map(|key| (key.time() * 60.).round() as u32)
                .collect::<Vec<_>>(),
            vec![50, 70, 57, 80]
        );
        let pasted = editor.session.document().clone();
        click(&mut editor, &ctx, "timelineCut", egui::Modifiers::NONE);
        let after = peachy_engine::animation_keys(
            editor.session.document(),
            0,
            &[KeyOwner::Bone { bone: 8 }, KeyOwner::Bone { bone: 7 }],
        )
        .unwrap()
        .keys;
        assert!(selected.iter().all(|key| !after.contains(key)));
        assert!(accepted.iter().all(|key| after.contains(key)));
        assert!(after.contains(&key(8, Property::Rotation, 60.)));
        assert!(after.contains(&key(7, Property::Y, 60.)));
        assert_eq!(after.len(), 6);
        editor.undo();
        assert_eq!(editor.session.document(), &pasted);
        editor.undo();
        assert_eq!(editor.session.document(), &before);
        // Undo restores the shorter clip; repaint before reading marker targets
        // recorded under the pasted clip's longer time extent.
        settle(&mut editor, &ctx);
        click(
            &mut editor,
            &ctx,
            "timelineKey:bone:8:Rotation:0",
            egui::Modifiers::NONE,
        );
        assert_eq!(editor.time.to_bits(), (5_f32 / 60.).to_bits());
    }
}
