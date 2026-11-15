use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
use super::*;
use peachy_document::{TransformMapping, TransformTarget, TransformValues};

fn ready() -> (Editor, egui::Context, Document) {
    let mut editor = editor_fixture();
    let mut doc = editor.session.document().clone();
    doc.constraints
        .push(Constraint::Transform(TransformConstraint {
            name: "driver X to rotation".into(),
            bones: vec![5],
            source: 8,
            offsets: TransformValues::default(),
            mix: TransformValues {
                rotate: 1.,
                ..Default::default()
            },
            mappings: vec![TransformMapping {
                from: TransformProperty::X,
                offset: 0.,
                to: vec![TransformTarget {
                    property: TransformProperty::Rotate,
                    offset: 0.,
                    max: 100.,
                    scale: 1.,
                }],
            }],
            local_source: false,
            local_target: false,
            additive: false,
            clamp: false,
        }));
    editor.session = Session::new(doc.clone()).unwrap();
    editor.setup_mode = true;
    editor.select(TreeItem::Constraint(0));
    let ctx = editor_context();
    for _ in 0..4 {
        frame(&mut editor, &ctx, vec![]);
    }
    (editor, ctx, doc)
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
            editor.reconcile_selection();
            editor.reconcile_constraint_input(ui.ctx());
            editor.shortcuts(ui.ctx());
            egui::Panel::right("test-transform-properties")
                .exact_size(460.)
                .show(ui, |ui| editor.transform_constraint_properties(ui, 0));
            egui::CentralPanel::default().show(ui, |_| {});
            editor.finish_idle_transactions(ui.ctx());
        },
    );
    output.textures_delta.clear();
}
fn rect(editor: &Editor, name: &str) -> egui::Rect {
    let [l, t, r, b] = *editor
        .capture_targets
        .get(name)
        .unwrap_or_else(|| panic!("Missing {name}"));
    egui::Rect::from_min_max(pos2(l, t), pos2(r, b))
}
fn button(editor: &mut Editor, ctx: &egui::Context, point: Pos2, pressed: bool) {
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
    let point = rect(editor, name).center();
    button(editor, ctx, point, true);
    button(editor, ctx, point, false);
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
fn text(editor: &mut Editor, ctx: &egui::Context, target: &str, value: &str) {
    click(editor, ctx, target);
    assert!(ctx.text_edit_focused(), "{target}");
    key(editor, ctx, egui::Key::A, egui::Modifiers::COMMAND);
    frame(editor, ctx, vec![egui::Event::Text(value.into())]);
}
fn range(editor: &Editor) -> peachy_engine::TransformMappingRange {
    transform_mapping_range(
        editor.session.document(),
        0,
        TransformProperty::X,
        TransformProperty::Rotate,
    )
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
}

#[test]
fn transform_mapping_typed_range_and_mix_are_exact_separate_undo_steps() {
    let (mut editor, ctx, original) = ready();
    text(&mut editor, &ctx, "transformTargetMax", "150");
    assert_eq!(editor.session.document(), &original);
    key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    frame(&mut editor, &ctx, vec![]);
    assert_eq!(range(&editor).target_max, 150.);
    assert_eq!(range(&editor).source_max, Some(100.));
    assert_eq!(
        editor.transform_constraint(0).unwrap().mappings[0].to[0].scale,
        1.5
    );
    let mapped = editor.session.document().clone();
    text(&mut editor, &ctx, "transformMixrotate", "50");
    key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(editor.transform_constraint(0).unwrap().mix.rotate, 0.5);
    editor.undo();
    assert_eq!(editor.session.document(), &mapped);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn transform_mapping_cumulative_range_preview_retains_repaired_partner_one_undo() {
    let (mut editor, ctx, original) = ready();
    click(&mut editor, &ctx, "transformSourceMin");
    assert!(editor.constraint_edit_active());
    editor.preview_constraint_number(100.).unwrap();
    assert_eq!(range(&editor).source_max, Some(200.));
    editor.preview_constraint_number(150.).unwrap();
    assert_eq!(range(&editor).source_min, 150.);
    assert_eq!(range(&editor).source_max, Some(200.));
    editor.finish_constraint_edit();
    editor.undo();
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn transform_mapping_unchanged_precision_and_invalid_text_preserve_redo() {
    let (mut editor, ctx, _) = ready();
    let mut doc = editor.session.document().clone();
    let Constraint::Transform(value) = &mut doc.constraints[0] else {
        panic!()
    };
    value.mappings[0].to[0].scale = 0.7;
    editor.session = Session::new(doc.clone()).unwrap();
    redo_branch(&mut editor);
    frame(&mut editor, &ctx, vec![]);
    click(&mut editor, &ctx, "transformSourceMax");
    key(&mut editor, &ctx, egui::Key::Tab, egui::Modifiers::NONE);
    assert_eq!(editor.session.document(), &doc);
    assert!(editor.session.can_redo());
    text(&mut editor, &ctx, "transformTargetMax", "NaN");
    key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(editor.session.document(), &doc);
    assert!(editor.session.can_redo());
    assert!(
        editor
            .transform_constraint_view
            .error
            .as_deref()
            .is_some_and(|error| error.contains("finite"))
    );
}

#[test]
fn transform_mapping_undefined_source_max_stays_blank_until_finite_text_even_zero() {
    for typed in ["0", "100"] {
        let (mut editor, ctx, _) = ready();
        let mut original = editor.session.document().clone();
        let Constraint::Transform(value) = &mut original.constraints[0] else {
            panic!()
        };
        value.mappings[0].to[0].scale = 0.;
        editor.session = Session::new(original.clone()).unwrap();
        redo_branch(&mut editor);
        frame(&mut editor, &ctx, vec![]);
        assert_eq!(range(&editor).source_max, None);
        click(&mut editor, &ctx, "transformSourceMax");
        assert!(ctx.text_edit_focused());
        let diagnostic = editor.constraint_edit_diagnostic();
        assert_eq!(diagnostic["active"]["undefined"], true);
        assert!(diagnostic["active"]["value"].is_null());
        key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
        frame(&mut editor, &ctx, vec![]);
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
        text(&mut editor, &ctx, "transformSourceMax", typed);
        assert_eq!(editor.session.document(), &original);
        key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
        frame(&mut editor, &ctx, vec![]);
        // Zero is an explicit endpoint edit and invokes the same native +100
        // repair as the semantic MCP command. It is not a placeholder no-op.
        assert_eq!(range(&editor).source_max, Some(100.));
        assert_eq!(
            editor.transform_constraint(0).unwrap().mappings[0].to[0].scale,
            1.
        );
        assert!(!editor.constraint_edit_active());
        assert_eq!(
            editor.constraint_edit_diagnostic()["range"]["sourceMax"],
            100.
        );
        editor.undo();
        assert_eq!(editor.session.document(), &original);
    }
}

#[test]
fn transform_mapping_held_mix_escape_focus_and_history_cancel_without_reentry() {
    for cancel in ["escape", "focus", "undo"] {
        let (mut editor, ctx, original) = ready();
        redo_branch(&mut editor);
        frame(&mut editor, &ctx, vec![]);
        let slider = rect(&editor, "transformMixSliderrotate");
        let point = pos2(slider.left() + slider.width() * 0.25, slider.center().y);
        button(&mut editor, &ctx, point, true);
        frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(point + vec2(10., 0.))],
        );
        assert!(editor.constraint_edit_active());
        assert_ne!(editor.session.document(), &original);
        match cancel {
            "escape" => key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE),
            "undo" => key(&mut editor, &ctx, egui::Key::Z, egui::Modifiers::COMMAND),
            _ => {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        focused: false,
                        events: vec![egui::Event::WindowFocused(false)],
                        ..Default::default()
                    },
                    |ui| {
                        editor.reconcile_constraint_input(ui.ctx());
                        editor.finish_idle_transactions(ui.ctx());
                    },
                );
                output.textures_delta.clear();
            }
        }
        assert!(!editor.constraint_edit_active());
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
        frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(point + vec2(25., 0.))],
        );
        button(&mut editor, &ctx, point, false);
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
    }
}

#[test]
fn transform_mapping_secondary_press_cannot_restart_canceled_held_primary() {
    for target in ["transformMixSliderrotate", "transformOffsetSliderx"] {
        let (mut editor, ctx, original) = ready();
        if target == "transformOffsetSliderx" {
            click(&mut editor, &ctx, "transformOffsetGroup");
            for _ in 0..5 {
                frame(&mut editor, &ctx, vec![]);
            }
        }
        redo_branch(&mut editor);
        let slider = rect(&editor, target);
        let start = pos2(slider.left() + slider.width() * 0.25, slider.center().y);
        button(&mut editor, &ctx, start, true);
        let moved = start + vec2(12., 0.);
        frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(moved)]);
        assert!(editor.constraint_edit_active(), "{target}");
        assert_ne!(editor.session.document(), &original);
        key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
        assert!(!editor.constraint_edit_active());
        assert_eq!(editor.session.document(), &original);
        assert!(editor.transform_constraint_view.blocked.is_some());
        for pressed in [true, false] {
            frame(
                &mut editor,
                &ctx,
                vec![
                    egui::Event::PointerButton {
                        pos: moved,
                        button: egui::PointerButton::Secondary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                    egui::Event::PointerMoved(moved + vec2(18., 0.)),
                ],
            );
            assert!(!editor.constraint_edit_active(), "{target}");
            assert_eq!(editor.session.document(), &original);
            assert!(editor.session.can_redo());
            assert!(editor.transform_constraint_view.blocked.is_some());
        }
        button(&mut editor, &ctx, moved, false);
        assert!(!editor.constraint_edit_active());
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
        // A real new primary press is allowed once the canceled one is released.
        let point = rect(&editor, target).center() + vec2(10., 0.);
        button(&mut editor, &ctx, point, true);
        frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(point + vec2(12., 0.))],
        );
        assert!(editor.constraint_edit_active(), "{target}");
        assert_ne!(editor.session.document(), &original);
        button(&mut editor, &ctx, point + vec2(12., 0.), false);
        editor.undo();
        assert_eq!(editor.session.document(), &original);
    }
}

#[test]
fn transform_mapping_full_workspace_held_mix_after_typing_cancels_without_scroll_or_reentry() {
    let (mut editor, ctx, _) = ready();
    for _ in 0..5 {
        workspace_frame(&mut editor, &ctx, vec![]);
    }
    let press = |editor: &mut Editor, point: Pos2, pressed| {
        workspace_frame(
            editor,
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
    };
    let full_key = |editor: &mut Editor, key, modifiers| {
        workspace_frame(
            editor,
            &ctx,
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
    };
    let numeric = rect(&editor, "transformMixrotate").center();
    press(&mut editor, numeric, true);
    press(&mut editor, numeric, false);
    workspace_frame(&mut editor, &ctx, vec![]);
    assert!(ctx.text_edit_focused());
    full_key(&mut editor, egui::Key::A, egui::Modifiers::COMMAND);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("50".into())]);
    full_key(&mut editor, egui::Key::Enter, egui::Modifiers::NONE);
    for _ in 0..5 {
        workspace_frame(&mut editor, &ctx, vec![]);
    }
    assert_eq!(editor.transform_constraint(0).unwrap().mix.rotate, 0.5);
    let original = editor.session.document().clone();
    redo_branch(&mut editor);
    let slider = rect(&editor, "transformMixSliderrotate");
    let point = slider.center();
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(point)]);
    press(&mut editor, point, true);
    for dx in [8., 20., 35.] {
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(point + vec2(dx, 0.))],
        );
        assert_eq!(
            rect(&editor, "transformMixSliderrotate"),
            slider,
            "{}",
            editor.constraint_edit_diagnostic()
        );
    }
    assert!(
        editor.constraint_edit_active(),
        "{}",
        editor.constraint_edit_diagnostic()
    );
    assert!(editor.transform_constraint(0).unwrap().mix.rotate > 0.5);
    full_key(&mut editor, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(!editor.constraint_edit_active());
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.can_redo());
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(point + vec2(40., 0.))],
    );
    assert!(!editor.constraint_edit_active());
    assert_eq!(editor.session.document(), &original);
    press(&mut editor, point + vec2(40., 0.), false);
    assert_eq!(editor.session.document(), &original);
    let point = rect(&editor, "transformMixSliderrotate").center() + vec2(12., 0.);
    press(&mut editor, point, true);
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(point + vec2(12., 0.))],
    );
    assert!(
        editor.constraint_edit_active(),
        "{}",
        editor.constraint_edit_diagnostic()
    );
    press(&mut editor, point + vec2(12., 0.), false);
    assert_ne!(editor.session.document(), &original);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn transform_mapping_full_workspace_coalesced_mix_press_near_scrollbar_keeps_control_ownership() {
    let (mut editor, ctx, _) = ready();
    let mut original = editor.session.document().clone();
    let Constraint::Transform(value) = &mut original.constraints[0] else {
        panic!()
    };
    value.mix.rotate = 0.5;
    editor.session = Session::new(original.clone()).unwrap();
    redo_branch(&mut editor);
    // A solid scrollbar reserves its width through egui's show animation.
    // Establish the settled workspace before freezing a gesture's geometry.
    let settle_frames = (ctx.global_style().animation_time / ctx.input(|input| input.predicted_dt))
        .ceil() as usize
        + 2;
    for _ in 0..settle_frames {
        workspace_frame(&mut editor, &ctx, vec![]);
    }
    let slider = rect(&editor, "transformMixSliderrotate");
    workspace_frame(&mut editor, &ctx, vec![]);
    assert_eq!(rect(&editor, "transformMixSliderrotate"), slider);
    let start = slider.center();
    let end = pos2(slider.right() - 5., slider.center().y);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(start)]);
    // Native backends may deliver Down and the first substantial horizontal
    // move in one frame. The endpoint still belongs to the visible slider.
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerMoved(end),
        ],
    );
    assert!(
        editor.constraint_edit_active(),
        "{}",
        editor.constraint_edit_diagnostic()
    );
    assert!(editor.transform_constraint(0).unwrap().mix.rotate > 0.5);
    assert_eq!(rect(&editor, "transformMixSliderrotate"), slider);
    workspace_frame(&mut editor, &ctx, vec![]);
    assert!(
        editor.constraint_edit_active(),
        "{}",
        editor.constraint_edit_diagnostic()
    );
    assert_eq!(rect(&editor, "transformMixSliderrotate"), slider);
    workspace_frame(
        &mut editor,
        &ctx,
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: Some(egui::Key::Escape),
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            })
            .collect(),
    );
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.constraint_edit_active());
    assert!(editor.session.can_redo());
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerButton {
            pos: end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert_eq!(editor.session.document(), &original);
    assert!(editor.session.can_redo());
}

#[test]
fn transform_mapping_clamp_offset_and_animate_readonly_use_explicit_constraint() {
    let (mut editor, ctx, original) = ready();
    click(&mut editor, &ctx, "transformClamp");
    assert!(editor.transform_constraint(0).unwrap().clamp);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    click(&mut editor, &ctx, "transformOffsetGroup");
    for _ in 0..5 {
        frame(&mut editor, &ctx, vec![]);
    }
    assert!(!editor.capture_targets.contains_key("transformOffsetrotate"));
    text(&mut editor, &ctx, "transformOffsetx", "13.5");
    key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(editor.transform_constraint(0).unwrap().offsets.x, 13.5);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    frame(&mut editor, &ctx, vec![]);
    let slider = rect(&editor, "transformOffsetSliderx");
    let start = slider.center();
    let end = pos2(slider.right() + 30., start.y);
    button(&mut editor, &ctx, start, true);
    frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
    assert!(editor.transform_constraint(0).unwrap().offsets.x > 50.);
    assert_eq!(editor.transform_constraint(0).unwrap().offsets.y, 0.);
    button(&mut editor, &ctx, end, false);
    editor.undo();
    assert_eq!(editor.session.document(), &original);
    editor.set_setup_mode(false);
    frame(&mut editor, &ctx, vec![]);
    click(&mut editor, &ctx, "transformSourceMin");
    frame(&mut editor, &ctx, vec![egui::Event::Text("75".into())]);
    key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(editor.session.document(), &original);
    assert!(!editor.constraint_edit_active());
}

#[test]
fn transform_mapping_owned_text_save_cancels_before_writing_and_stale_enter_cannot_apply() {
    let (mut editor, ctx, original) = ready();
    let folder = std::env::temp_dir().join(format!(
        "peachy-transform-save-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&folder).unwrap();
    editor.asset_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
    editor.project_path = Some(folder.join("project.peachy"));
    text(&mut editor, &ctx, "transformTargetMax", "175");
    assert!(editor.constraint_edit_active());
    key(&mut editor, &ctx, egui::Key::S, egui::Modifiers::COMMAND);
    assert!(!editor.constraint_edit_active());
    assert_eq!(editor.session.document(), &original);
    assert_eq!(
        files::load(editor.project_path.as_ref().unwrap()).unwrap(),
        original
    );
    key(&mut editor, &ctx, egui::Key::Enter, egui::Modifiers::NONE);
    assert_eq!(editor.session.document(), &original);
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn transform_mapping_minimum_workspace_horizontal_overflow_reaches_target_and_clamp() {
    let (mut editor, ctx, original) = ready();
    let full = |editor: &mut Editor, events: Vec<egui::Event>| {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1120., 740.))),
                focused: true,
                events,
                ..Default::default()
            },
            |ui| editor.workspace_ui(ui),
        );
        output.textures_delta.clear();
    };
    for _ in 0..4 {
        full(&mut editor, vec![]);
    }
    let mut reached = std::collections::BTreeSet::new();
    for _ in 0..8 {
        for name in [
            "transformSourceMin",
            "transformSourceMax",
            "transformTargetMin",
            "transformTargetMax",
            "transformClamp",
        ] {
            if editor.capture_targets.contains_key(name) {
                let r = rect(&editor, name);
                assert!(r.is_positive() && r.is_finite());
                reached.insert(name);
            }
        }
        let ranges_reached = [
            "transformSourceMin",
            "transformSourceMax",
            "transformTargetMin",
            "transformTargetMax",
        ]
        .into_iter()
        .all(|name| reached.contains(name));
        if !reached.contains("transformSourceMin")
            || !reached.contains("transformSourceMax")
            || (ranges_reached && !reached.contains("transformClamp"))
        {
            let clip = rect(&editor, "properties");
            // The capture includes the clip margin. Stay inside the vertical
            // scroll viewport and left of the indented horizontal table.
            let point = pos2(clip.left() + 12., clip.center().y);
            full(
                &mut editor,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: vec2(0., -120.),
                        phase: egui::TouchPhase::Move,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            for _ in 0..2 {
                full(&mut editor, vec![]);
            }
        } else if editor
            .capture_targets
            .contains_key("transformPropertiesScroll")
        {
            let point = rect(&editor, "transformPropertiesScroll").center();
            full(
                &mut editor,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: vec2(-120., 0.),
                        phase: egui::TouchPhase::Move,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            for _ in 0..2 {
                full(&mut editor, vec![]);
            }
        }
    }
    for name in [
        "transformSourceMin",
        "transformSourceMax",
        "transformTargetMin",
        "transformTargetMax",
        "transformClamp",
    ] {
        assert!(reached.contains(name), "Unreachable {name}; {reached:?}");
    }
    let clip = rect(&editor, "properties");
    let point = pos2(clip.left() + 12., clip.center().y);
    full(
        &mut editor,
        vec![
            egui::Event::PointerMoved(point),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0., 10000.),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    for _ in 0..4 {
        full(&mut editor, vec![]);
    }
    let point = rect(&editor, "transformOffsetGroup").center();
    for pressed in [true, false] {
        full(
            &mut editor,
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
    let mut offsets = std::collections::BTreeSet::new();
    for _ in 0..8 {
        for name in [
            "transformOffsetx",
            "transformOffsety",
            "transformOffsetSliderx",
            "transformOffsetSlidery",
        ] {
            if editor.capture_targets.contains_key(name) {
                let r = rect(&editor, name);
                assert!(r.is_positive() && r.is_finite());
                offsets.insert(name);
            }
        }
        if editor.capture_targets.contains_key("transformOffsetScroll") {
            let point = rect(&editor, "transformOffsetScroll").center();
            full(
                &mut editor,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: vec2(-100., 0.),
                        phase: egui::TouchPhase::Move,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        for _ in 0..2 {
            full(&mut editor, vec![]);
        }
    }
    for name in [
        "transformOffsetx",
        "transformOffsety",
        "transformOffsetSliderx",
        "transformOffsetSlidery",
    ] {
        assert!(
            offsets.contains(name),
            "Unreachable Offset control {name}; {offsets:?}"
        );
    }
    assert_eq!(editor.session.document(), &original);
}
