use super::super::test_support::{editor_context, editor_fixture};
use super::*;

fn ready() -> (Editor, egui::Context, usize) {
    let mut editor = editor_fixture();
    let mut doc = editor.session.document().clone();
    let constraint = doc.constraints.len();
    doc.constraints
        .push(Constraint::Slider(SliderConstraint::new(
            "calibration slider",
            0,
        )));
    doc.animations.push(Animation::new("host", vec![]));
    let host = doc.animations.len() - 1;
    for property in SliderProperty::ALL {
        doc.animations[host].slider_tracks.push(SliderTrack {
            constraint,
            property,
            keys: [0.12345, 0.123455]
                .into_iter()
                .enumerate()
                .map(|(i, time)| Keyframe {
                    time,
                    value: 0.25 + i as f32 * 0.5,
                    curve: Curve::Bezier([0.2, 0.3, 0.8, 0.9]),
                })
                .collect(),
        });
    }
    editor.session = Session::new(doc).unwrap();
    editor.setup_mode = false;
    editor.animation = host;
    editor.time = 0.12345;
    editor.select(TreeItem::Constraint(constraint));
    let ctx = editor_context();
    for _ in 0..3 {
        frame(&mut editor, &ctx, vec![]);
    }
    (editor, ctx, constraint)
}

fn frame(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>) {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
            focused: true,
            events,
            ..Default::default()
        },
        |ui| {
            editor.capture_targets.clear();
            editor.reconcile_selection();
            editor.reconcile_slider_input(ui.ctx());
            editor.shortcuts(ui.ctx());
            egui::Panel::right("slider-test-properties")
                .exact_size(460.)
                .show(ui, |ui| editor.inspector(ui));
            egui::CentralPanel::default().show(ui, |_| {});
            editor.finish_idle_transactions(ui.ctx());
        },
    );
    output.textures_delta.clear();
}

#[test]
fn slider_manual_empty_reference_frame_is_disabled_without_redirecting_selection() {
    let (mut editor, ctx, constraint) = ready();
    let mut doc = editor.session.document().clone();
    let empty = doc.animations.len();
    doc.animations
        .push(Animation::new("empty reference", vec![]));
    let Constraint::Slider(value) = &mut doc.constraints[constraint] else {
        unreachable!()
    };
    value.animation = empty;
    editor.session = Session::new(doc).unwrap();
    frame(&mut editor, &ctx, vec![]);
    assert_eq!(
        editor.timeline_owner(),
        Some(peachy_engine::KeyOwner::Slider { constraint })
    );
    assert!(editor.capture_targets.contains_key("sliderCapability"));
    assert!(!editor.capture_targets.contains_key("ikMixValue"));
    editor
        .session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo marker".into(),
        })
        .unwrap();
    editor.session.undo();
    let original = editor.session.document().clone();
    let [l, t, r, b] = editor.capture_targets["sliderTimeValue"];
    let point = pos2((l + r) * 0.5, (t + b) * 0.5);
    for pressed in [true, false] {
        frame(
            &mut editor,
            &ctx,
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
    frame(&mut editor, &ctx, vec![egui::Event::Text("75".into())]);
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.can_redo());
    assert_eq!(editor.selection, Some(TreeItem::Constraint(constraint)));
    assert!(!ctx.text_edit_focused());
}

#[test]
fn slider_rows_clipboard_and_curve_descriptors_preserve_exact_time_mix_identity() {
    use super::super::dopesheet_selection::{TimelineRow, row_keys, rows};
    use peachy_engine::{AnimationKey, AnimationKeyClipboard, KeyOwner};
    let (mut editor, ctx, constraint) = ready();
    let owner = KeyOwner::Slider { constraint };
    assert_eq!(
        rows(Some(owner.clone())),
        vec![
            TimelineRow::Slider(SliderProperty::Time),
            TimelineRow::Slider(SliderProperty::Mix)
        ]
    );
    let keys = row_keys(
        editor.session.document(),
        editor.animation,
        owner,
        TimelineRow::Slider(SliderProperty::Time),
    );
    assert_eq!(keys.len(), 2);
    assert_eq!(
        keys[0],
        AnimationKey::Slider {
            constraint,
            property: SliderProperty::Time,
            time: 0.12345
        }
    );
    assert_eq!(keys[1].time().to_bits(), 0.123455_f32.to_bits());
    let targets =
        peachy_engine::curve_targets(editor.session.document(), editor.animation, &keys[..1])
            .unwrap();
    let channels = peachy_engine::curve_channels_for_targets(
        editor.session.document(),
        editor.animation,
        &targets,
    )
    .unwrap();
    assert_eq!(channels[0].values, [0.25, 0.75]);
    assert_eq!(
        channels[0].curve,
        peachy_engine::CurveData::Standard(Curve::Bezier([0.2, 0.3, 0.8, 0.9]))
    );
    editor.reconcile_dopesheet_selection();
    editor.key_selection.replace(keys);
    assert!(editor.copy_dopesheet_keys(&ctx));
    assert!(
        matches!(editor.timeline_clipboard,Some(AnimationKeyClipboard::Slider {ref keys}) if keys.len()==2 && keys.iter().all(|key|key.property==SliderProperty::Time))
    );
    let original = editor.session.document().clone();
    editor.delete_dopesheet_keys();
    assert!(
        editor.session.document().animations[editor.animation]
            .slider_tracks
            .iter()
            .all(|track| track.property != SliderProperty::Time)
    );
    assert!(
        editor.session.document().animations[editor.animation]
            .slider_tracks
            .iter()
            .any(|track| track.property == SliderProperty::Mix)
    );
    editor.undo();
    assert_eq!(editor.session.document(), &original);
}

fn manual_ready(setup: bool) -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    let mut doc = editor.session.document().clone();
    doc.fps = 60;
    doc.constraints = vec![Constraint::Slider(SliderConstraint::new(
        "manual slider",
        0,
    ))];
    doc.animations = vec![
        Animation::new(
            "reference",
            vec![Track {
                bone: 0,
                property: Property::X,
                keys: vec![
                    Keyframe {
                        time: 0.,
                        value: 0.,
                        curve: Curve::Linear,
                    },
                    Keyframe {
                        time: 1.5,
                        value: 120.,
                        curve: Curve::Linear,
                    },
                ],
            }],
        ),
        Animation::new("host", vec![]),
    ];
    // This fixture needs no copied artwork for save/draft cancellation tests.
    for skin in doc.skins.values_mut() {
        skin.clear();
    }
    for slot in &mut doc.slots {
        slot.attachment = None;
    }
    editor.session = Session::new(doc).unwrap();
    editor.setup_mode = setup;
    editor.animation = 1;
    editor.time = 0.25;
    editor.auto_key = false;
    editor.select(TreeItem::Constraint(0));
    let ctx = editor_context();
    for _ in 0..4 {
        frame(&mut editor, &ctx, vec![]);
    }
    (editor, ctx)
}

fn target(editor: &Editor, name: &str) -> egui::Rect {
    let [l, t, r, b] = editor.capture_targets[name];
    egui::Rect::from_min_max(pos2(l, t), pos2(r, b))
}
fn pointer(editor: &mut Editor, ctx: &egui::Context, point: Pos2, pressed: bool) {
    frame(
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
fn click(editor: &mut Editor, ctx: &egui::Context, name: &str) {
    frame(editor, ctx, vec![]);
    let point = target(editor, name).center();
    pointer(editor, ctx, point, true);
    pointer(editor, ctx, point, false);
    frame(editor, ctx, vec![]);
}
fn key(editor: &mut Editor, ctx: &egui::Context, key: egui::Key, modifiers: egui::Modifiers) {
    frame(
        editor,
        ctx,
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
fn draft(editor: &mut Editor, ctx: &egui::Context, name: &str, text: &str) {
    click(editor, ctx, name);
    assert!(ctx.text_edit_focused(), "{name}");
    key(editor, ctx, egui::Key::A, egui::Modifiers::COMMAND);
    frame(editor, ctx, vec![egui::Event::Text(text.into())]);
}
fn typed(editor: &mut Editor, ctx: &egui::Context, name: &str, text: &str) {
    draft(editor, ctx, name, text);
    key(editor, ctx, egui::Key::Enter, egui::Modifiers::NONE);
    frame(editor, ctx, vec![]);
}
fn pose(editor: &Editor) -> peachy_engine::SliderPose {
    editor
        .session
        .slider_pose(0, editor.weights_edit_target())
        .unwrap()
}
fn redo_branch(editor: &mut Editor) {
    editor
        .session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo marker".into(),
        })
        .unwrap();
    editor.session.undo();
    assert!(editor.session.can_redo());
}

#[test]
fn slider_manual_frame_beyond_end_signed_mix_and_independent_key_buttons() {
    let (mut editor, ctx) = manual_ready(false);
    let baseline = editor.session.document().clone();
    typed(&mut editor, &ctx, "sliderTimeValue", "120");
    typed(&mut editor, &ctx, "sliderMixValue", "-25");
    assert_eq!(
        pose(&editor),
        peachy_engine::SliderPose {
            time: 2.,
            mix: -0.25
        }
    );
    assert_eq!(editor.time, 0.25);
    assert_eq!(editor.session.document(), &baseline);
    assert!(!editor.session.is_dirty());
    click(&mut editor, &ctx, "sliderTimeKey");
    let clip = &editor.session.document().animations[1];
    assert_eq!(clip.slider_tracks.len(), 1);
    assert_eq!(clip.slider_tracks[0].property, SliderProperty::Time);
    assert_eq!(
        clip.slider_tracks[0].keys[0].time.to_bits(),
        0.25_f32.to_bits()
    );
    assert_eq!(clip.slider_tracks[0].keys[0].value, 2.);
    assert!(
        editor
            .session
            .pose_edits()
            .has_slider(1, 0, SliderProperty::Mix)
    );
    let time_keyed = editor.session.document().clone();
    click(&mut editor, &ctx, "sliderMixKey");
    assert_eq!(
        editor.session.document().animations[1].slider_tracks.len(),
        2
    );
    assert!(
        !editor
            .session
            .pose_edits()
            .has_slider(1, 0, SliderProperty::Mix)
    );
    editor.undo();
    assert_eq!(editor.session.document(), &time_keyed);
    assert_eq!(pose(&editor).mix, -0.25);
}

#[test]
fn slider_manual_key_button_retains_fractional_neighbors_and_all_existing_curves() {
    let (mut editor, ctx, constraint) = ready();
    let original = editor.session.document().clone();
    typed(&mut editor, &ctx, "sliderTimeValue", "75");
    click(&mut editor, &ctx, "sliderTimeKey");
    let clip = &editor.session.document().animations[editor.animation];
    let track = clip
        .slider_tracks
        .iter()
        .find(|track| track.constraint == constraint && track.property == SliderProperty::Time)
        .unwrap();
    let old = &original.animations[editor.animation].slider_tracks[0];
    assert_eq!(track.keys.len(), 2);
    assert_eq!(track.keys[0].time.to_bits(), old.keys[0].time.to_bits());
    assert_eq!(track.keys[0].curve, old.keys[0].curve);
    assert_eq!(track.keys[1], old.keys[1]);
    assert_eq!(
        track.keys[0].value,
        75. / editor.session.document().fps as f32
    );
    assert_eq!(
        clip.slider_tracks[1],
        original.animations[editor.animation].slider_tracks[1]
    );
}

#[test]
fn slider_manual_setup_options_and_values_use_separate_data_history() {
    let (mut editor, ctx) = manual_ready(true);
    let baseline = editor.session.document().clone();
    click(&mut editor, &ctx, "sliderLoop");
    click(&mut editor, &ctx, "sliderAdditive");
    assert!(editor.slider_constraint(0).unwrap().looped);
    assert!(editor.slider_constraint(0).unwrap().additive);
    typed(&mut editor, &ctx, "sliderTimeValue", "120");
    let before_mix = editor.session.document().clone();
    typed(&mut editor, &ctx, "sliderMixValue", "150");
    assert_eq!(editor.slider_constraint(0).unwrap().setup.time, 2.);
    assert_eq!(editor.slider_constraint(0).unwrap().setup.mix, 1.5);
    editor.undo();
    assert_eq!(editor.session.document(), &before_mix);
    for _ in 0..3 {
        editor.undo();
    }
    assert_eq!(editor.session.document(), &baseline);
    editor.setup_mode = false;
    frame(&mut editor, &ctx, vec![]);
    click(&mut editor, &ctx, "sliderLoop");
    assert_eq!(editor.session.document(), &baseline);
}

#[test]
fn slider_manual_focus_blur_invalid_frame_and_nan_preserve_raw_values_and_redo() {
    let (mut editor, ctx) = manual_ready(true);
    let mut doc = editor.session.document().clone();
    let Constraint::Slider(value) = &mut doc.constraints[0] else {
        unreachable!()
    };
    value.setup.time = 0.12345679;
    value.setup.mix = -0.33333334;
    editor.session = Session::new(doc).unwrap();
    redo_branch(&mut editor);
    let baseline = editor.session.document().clone();
    click(&mut editor, &ctx, "sliderTimeValue");
    key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    typed(&mut editor, &ctx, "sliderTimeValue", "-1");
    typed(&mut editor, &ctx, "sliderMixValue", "NaN");
    assert_eq!(editor.session.document(), &baseline);
    assert!(editor.session.can_redo());
    assert!(!editor.slider_edit_active());
}

#[test]
fn slider_manual_held_escape_blocks_secondary_reentry_and_one_undo_restores_pending() {
    let (mut editor, ctx) = manual_ready(false);
    let mut local = editor.session.local_pose(Some(1), editor.time)[8];
    local.rotation = 13.5;
    editor
        .session
        .edit_pose(
            8,
            editor.weights_edit_target(),
            local,
            &[Property::Rotation],
            false,
        )
        .unwrap();
    redo_branch(&mut editor);
    let baseline = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let rail = target(&editor, "sliderMixSlider");
    let start = pos2(rail.left() + rail.width() * 0.3, rail.center().y);
    pointer(&mut editor, &ctx, start, true);
    let changed = pos2(rail.left() + rail.width() * 0.7, rail.center().y);
    frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(changed)]);
    assert!(editor.slider_edit_active());
    assert_ne!(pose(&editor).mix, 1.);
    key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(!editor.slider_edit_active());
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(editor.session.can_redo());
    frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::PointerButton {
                pos: changed,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerMoved(start),
        ],
    );
    assert!(!editor.slider_edit_active());
    assert_eq!(editor.session.pose_edits(), &pending);
    pointer(&mut editor, &ctx, start, false);
    frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerButton {
            pos: start,
            button: egui::PointerButton::Secondary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    pointer(&mut editor, &ctx, start, true);
    frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(changed)]);
    assert!(editor.slider_edit_active());
    pointer(&mut editor, &ctx, changed, false);
    assert!(!editor.slider_edit_active());
    assert_eq!(editor.session.document(), &baseline);
    assert_ne!(editor.session.pose_edits(), &pending);
    editor.undo();
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.session.document(), &baseline);
}

#[test]
fn slider_manual_global_k_and_auto_key_include_mixed_pending_pose_atomically() {
    for auto in [false, true] {
        let (mut editor, ctx) = manual_ready(false);
        let mut local = editor.session.local_pose(Some(1), editor.time)[8];
        local.rotation = 13.5;
        editor
            .session
            .edit_pose(
                8,
                editor.weights_edit_target(),
                local,
                &[Property::Rotation],
                false,
            )
            .unwrap();
        typed(&mut editor, &ctx, "sliderTimeValue", "45");
        typed(&mut editor, &ctx, "sliderMixValue", "25");
        let baseline = editor.session.document().clone();
        let pending = editor.session.pose_edits().clone();
        if auto {
            editor.set_auto_key_enabled(true);
        } else {
            key(&mut editor, &ctx, egui::Key::K, egui::Modifiers::NONE);
        }
        assert!(editor.session.pose_edits().is_empty());
        assert_eq!(
            editor.session.document().animations[1].slider_tracks.len(),
            2
        );
        assert_eq!(
            editor.session.document().animations[1].tracks[0].keys[0].value,
            13.5
        );
        editor.undo();
        assert_eq!(editor.session.document(), &baseline);
        assert_eq!(editor.session.pose_edits(), &pending);
    }
}

#[test]
fn slider_manual_owned_draft_save_excludes_pending_and_blocks_stale_enter() {
    let (mut editor, ctx) = manual_ready(false);
    typed(&mut editor, &ctx, "sliderTimeValue", "45");
    let pending = editor.session.pose_edits().clone();
    let baseline = editor.session.document().clone();
    let dir = std::env::temp_dir().join(format!(
        "peachy-slider-save-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    editor.project_path = Some(dir.join("manual.peachy"));
    editor.asset_root = dir.clone();
    draft(&mut editor, &ctx, "sliderMixValue", "25");
    assert!(editor.slider_edit_active());
    key(&mut editor, &ctx, egui::Key::S, egui::Modifiers::COMMAND);
    assert!(!editor.slider_edit_active());
    assert_eq!(editor.session.pose_edits(), &pending);
    assert!(editor.error.is_none(), "Save failed: {:?}", editor.error);
    assert_eq!(
        files::load(editor.project_path.as_ref().unwrap()).unwrap(),
        baseline
    );
    key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(pose(&editor).mix, 1.);
    assert_eq!(editor.session.pose_edits(), &pending);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn slider_manual_full_workspace_coalesced_rail_and_context_cancel_preserve_baseline() {
    use super::super::test_support::workspace_frame;
    let (mut editor, ctx) = manual_ready(false);
    let mut document = editor.session.document().clone();
    let Constraint::Slider(value) = &mut document.constraints[0] else {
        unreachable!()
    };
    value.setup.mix = 0.5;
    editor.session = Session::new(document).unwrap();
    for _ in 0..24 {
        workspace_frame(&mut editor, &ctx, vec![]);
    }
    let rail = target(&editor, "sliderMixSlider");
    let start = rail.center();
    let end = pos2(rail.right() - 1., rail.center().y);
    let baseline = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::PointerMoved(start),
            egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerMoved(end),
        ],
    );
    assert!(editor.slider_edit_active());
    assert_eq!(pose(&editor).mix, 1.);
    assert_ne!(pose(&editor).mix, 0.5);
    editor.show_tree = false;
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(start)]);
    assert!(!editor.slider_edit_active());
    assert_eq!(editor.session.document(), &baseline);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(pose(&editor).mix, 0.5);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerButton {
            pos: start,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
}

#[test]
fn slider_manual_full_workspace_focus_loss_cancels_preview_and_preserves_redo() {
    use super::super::test_support::workspace_frame;
    let (mut editor, ctx) = manual_ready(true);
    redo_branch(&mut editor);
    for _ in 0..24 {
        workspace_frame(&mut editor, &ctx, vec![]);
    }
    let baseline = editor.session.document().clone();
    let rail = target(&editor, "sliderMixSlider");
    let end = pos2(rail.left() + rail.width() * 0.8, rail.center().y);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::PointerMoved(rail.center()),
            egui::Event::PointerButton {
                pos: rail.center(),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
    assert!(editor.slider_edit_active());
    assert_ne!(editor.session.document(), &baseline);
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
            focused: false,
            ..Default::default()
        },
        |ui| editor.workspace_ui(ui),
    );
    output.textures_delta.clear();
    assert!(!editor.slider_edit_active());
    assert_eq!(editor.session.document(), &baseline);
    assert!(editor.session.can_redo());
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(rail.center())],
    );
    assert_eq!(editor.session.document(), &baseline);
}
