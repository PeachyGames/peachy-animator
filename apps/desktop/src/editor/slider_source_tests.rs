use super::super::test_support::{editor_context, editor_fixture};
use super::*;

#[derive(Clone, Copy)]
enum Surface {
    Properties,
    Workspace(Vec2),
}

fn input(
    editor: &mut Editor,
    ctx: &egui::Context,
    surface: Surface,
    focused: bool,
    events: Vec<egui::Event>,
) {
    let size = match surface {
        Surface::Properties => vec2(1500., 960.),
        Surface::Workspace(size) => size,
    };
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, size)),
            focused,
            events,
            ..Default::default()
        },
        |ui| match surface {
            Surface::Workspace(_) => editor.workspace_ui(ui),
            Surface::Properties => {
                editor.session.set_skin(&editor.skin);
                editor.capture_targets.clear();
                editor.reconcile_selection();
                editor.reconcile_slider_input(ui.ctx());
                editor.shortcuts(ui.ctx());
                egui::Panel::right("slider-source-properties")
                    .exact_size(460.)
                    .show(ui, |ui| editor.inspector(ui));
                egui::CentralPanel::default().show(ui, |_| {});
                editor.finish_idle_transactions(ui.ctx());
            }
        },
    );
    output.textures_delta.clear();
}

fn frame(editor: &mut Editor, ctx: &egui::Context, surface: Surface, events: Vec<egui::Event>) {
    input(editor, ctx, surface, true, events);
}

fn settle(editor: &mut Editor, ctx: &egui::Context, surface: Surface) {
    for _ in 0..4 {
        frame(editor, ctx, surface, vec![]);
    }
}

fn rect(editor: &Editor, name: &str) -> egui::Rect {
    let [l, t, r, b] = *editor
        .capture_targets
        .get(name)
        .unwrap_or_else(|| panic!("Missing {name}: {}", editor.slider_edit_diagnostic()));
    egui::Rect::from_min_max(pos2(l, t), pos2(r, b))
}

fn pointer(
    editor: &mut Editor,
    ctx: &egui::Context,
    surface: Surface,
    point: Pos2,
    button: egui::PointerButton,
    pressed: bool,
) {
    frame(
        editor,
        ctx,
        surface,
        vec![
            egui::Event::PointerMoved(point),
            egui::Event::PointerButton {
                pos: point,
                button,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
}

fn click(editor: &mut Editor, ctx: &egui::Context, surface: Surface, name: &str) {
    settle(editor, ctx, surface);
    let point = rect(editor, name).center();
    pointer(
        editor,
        ctx,
        surface,
        point,
        egui::PointerButton::Primary,
        true,
    );
    pointer(
        editor,
        ctx,
        surface,
        point,
        egui::PointerButton::Primary,
        false,
    );
    settle(editor, ctx, surface);
}

fn key(
    editor: &mut Editor,
    ctx: &egui::Context,
    surface: Surface,
    key: egui::Key,
    modifiers: egui::Modifiers,
) {
    let mut events = vec![egui::Event::ModifiersChanged(modifiers)];
    events.extend([true, false].into_iter().map(|pressed| egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed,
        repeat: false,
        modifiers,
    }));
    frame(editor, ctx, surface, events);
    frame(
        editor,
        ctx,
        surface,
        vec![egui::Event::ModifiersChanged(egui::Modifiers::NONE)],
    );
}

fn draft(editor: &mut Editor, ctx: &egui::Context, surface: Surface, name: &str, text: &str) {
    click(editor, ctx, surface, name);
    assert!(ctx.text_edit_focused(), "{name}");
    key(editor, ctx, surface, egui::Key::A, egui::Modifiers::COMMAND);
    frame(editor, ctx, surface, vec![egui::Event::Text(text.into())]);
}

fn typed(editor: &mut Editor, ctx: &egui::Context, surface: Surface, name: &str, text: &str) {
    draft(editor, ctx, surface, name, text);
    key(
        editor,
        ctx,
        surface,
        egui::Key::Enter,
        egui::Modifiers::NONE,
    );
    settle(editor, ctx, surface);
    assert!(
        editor.slider_view.error.is_none(),
        "{}",
        editor.slider_edit_diagnostic()
    );
}

fn ready(bound: bool, surface: Surface) -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    let mut doc = editor.session.document().clone();
    doc.fps = 60;
    doc.skin_scope = Default::default();
    for skin in doc.skins.values_mut() {
        skin.clear();
    }
    for slot in &mut doc.slots {
        slot.attachment = None;
    }
    doc.bones[8].setup.x = 150.;
    doc.bones[8].setup.y = 80.;
    doc.bones[8].setup.rotation = 30.;
    doc.bones[8].length = 100.;
    let mut slider = SliderConstraint::new("source slider", 0);
    slider.setup.time = 0.5;
    doc.constraints = vec![Constraint::Slider(slider)];
    doc.animations = vec![
        Animation::new(
            "reference",
            vec![Track {
                bone: 8,
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
    for property in SliderProperty::ALL {
        doc.animations[1].slider_tracks.push(SliderTrack {
            constraint: 0,
            property,
            keys: vec![Keyframe {
                time: 0.123456,
                value: if property == SliderProperty::Time {
                    0.75
                } else {
                    0.4
                },
                curve: Curve::Bezier([0.2, 0.3, 0.8, 0.9]),
            }],
        });
    }
    editor.session = Session::new(doc).unwrap();
    editor.session.set_skin(&editor.skin);
    if bound {
        editor
            .session
            .dispatch(Command::SetSliderSource {
                constraint: 0,
                bone: Some(8),
            })
            .unwrap();
    }
    editor.setup_mode = true;
    editor.animation = 1;
    editor.time = 0.42;
    editor.show_dopesheet = false;
    editor.select(TreeItem::Constraint(0));
    editor.tree.query = "source slider".into();
    let ctx = editor_context();
    settle(&mut editor, &ctx, surface);
    (editor, ctx)
}

fn redo_branch(editor: &mut Editor) {
    editor
        .session
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "redo marker".into(),
        })
        .unwrap();
    editor.session.undo();
    assert!(editor.session.can_redo());
}

#[test]
fn slider_source_ui_tree_picker_removes_time_keys_only_and_one_undo_restores_pending() {
    let surface = Surface::Workspace(vec2(1500., 960.));
    let (mut editor, ctx) = ready(false, surface);
    editor
        .session
        .edit_slider_pose(
            0,
            EditTarget::Animate {
                animation: 1,
                time: 0.42,
            },
            &[
                peachy_engine::SliderPoseEdit::Time { value: 0.8 },
                peachy_engine::SliderPoseEdit::Mix { value: 0.37 },
            ],
            false,
        )
        .unwrap();
    let original = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    click(&mut editor, &ctx, surface, "sliderBoneChoose");
    assert!(editor.slider_source_picker_active());
    editor.tree.query = editor.session.document().bones[8].name.clone();
    settle(&mut editor, &ctx, surface);
    click(&mut editor, &ctx, surface, "bone:8");
    assert!(!editor.slider_source_picker_active());
    assert_eq!(editor.selection, Some(TreeItem::Constraint(0)));
    assert_eq!(editor.slider_constraint(0).unwrap().bone, Some(8));
    let clip = &editor.session.document().animations[1];
    assert!(
        clip.slider_tracks
            .iter()
            .all(|track| track.property != SliderProperty::Time)
    );
    assert_eq!(
        clip.slider_tracks[0],
        original.animations[1].slider_tracks[1]
    );
    assert!(
        !editor
            .session
            .pose_edits()
            .has_slider(1, 0, SliderProperty::Time)
    );
    assert!(
        editor
            .session
            .pose_edits()
            .has_slider(1, 0, SliderProperty::Mix)
    );
    assert_eq!(
        editor
            .session
            .slider_pose(
                0,
                EditTarget::Animate {
                    animation: 1,
                    time: 0.42
                }
            )
            .unwrap()
            .mix,
        f64::from(0.37_f32)
    );
    key(
        &mut editor,
        &ctx,
        surface,
        egui::Key::Z,
        egui::Modifiers::COMMAND,
    );
    assert_eq!(editor.session.document(), &original);
    assert_eq!(editor.session.pose_edits(), &pending);
}

#[test]
fn slider_source_ui_viewport_picker_uses_bone_identity_without_transforming_it() {
    let surface = Surface::Workspace(vec2(1500., 960.));
    let (mut editor, ctx) = ready(false, surface);
    // This articulated fixture's source is above the initial view. Center its
    // actual evaluated segment before testing a real viewport choice.
    let world = editor
        .session
        .evaluated_pose(EditTarget::Setup)
        .unwrap()
        .world()[8]
        .point(50., 0.);
    editor.pan = vec2(-world[0] * editor.zoom, world[1] * editor.zoom);
    settle(&mut editor, &ctx, surface);
    let bones = editor.session.document().bones.clone();
    click(&mut editor, &ctx, surface, "sliderBoneChoose");
    editor.tree.filters.bones = false;
    let pose = editor.session.evaluated_pose(EditTarget::Setup).unwrap();
    let world = pose.world()[8].point(50., 0.);
    let point = scene_view::screen_point(
        rect(&editor, "viewport").center() + editor.pan,
        editor.zoom,
        world,
    );
    assert!(rect(&editor, "viewportInput").contains(point));
    pointer(
        &mut editor,
        &ctx,
        surface,
        point,
        egui::PointerButton::Primary,
        true,
    );
    pointer(
        &mut editor,
        &ctx,
        surface,
        point,
        egui::PointerButton::Primary,
        false,
    );
    settle(&mut editor, &ctx, surface);
    assert_eq!(editor.slider_constraint(0).unwrap().bone, Some(8));
    assert_eq!(editor.session.document().bones, bones);
    assert_eq!(editor.selection, Some(TreeItem::Constraint(0)));
    assert!(!editor.viewport_transform_active());
}

#[test]
fn slider_source_ui_picker_escape_focus_and_context_cancel_preserve_redo() {
    let surface = Surface::Workspace(vec2(1500., 960.));
    for action in ["escape", "focus", "history", "time", "views"] {
        let (mut editor, ctx) = ready(false, surface);
        redo_branch(&mut editor);
        let original = editor.session.document().clone();
        let pending = editor.session.pose_edits().clone();
        click(&mut editor, &ctx, surface, "sliderBoneChoose");
        assert!(editor.slider_source_picker_active(), "{action}");
        match action {
            "escape" => key(
                &mut editor,
                &ctx,
                surface,
                egui::Key::Escape,
                egui::Modifiers::NONE,
            ),
            "focus" => input(&mut editor, &ctx, surface, false, vec![]),
            "history" => key(
                &mut editor,
                &ctx,
                surface,
                egui::Key::Z,
                egui::Modifiers::COMMAND,
            ),
            "time" => {
                editor.time += 0.25;
                frame(&mut editor, &ctx, surface, vec![]);
            }
            "views" => key(
                &mut editor,
                &ctx,
                surface,
                egui::Key::F9,
                egui::Modifiers::NONE,
            ),
            _ => unreachable!(),
        }
        assert!(!editor.slider_source_picker_active(), "{action}");
        assert_eq!(editor.session.document(), &original, "{action}");
        assert_eq!(editor.session.pose_edits(), &pending, "{action}");
        assert!(editor.session.can_redo(), "{action}");
    }
}

#[test]
fn slider_source_ui_range_property_local_and_clear_use_shared_history() {
    let surface = Surface::Properties;
    let (mut editor, ctx) = ready(true, surface);
    typed(&mut editor, &ctx, surface, "sliderRangeSourceMin", "10");
    typed(&mut editor, &ctx, surface, "sliderRangeTargetMin", "20");
    let before = slider_mapping_range(editor.session.document(), 0).unwrap();
    assert!((before.target_min - 20. / 60.).abs() < 1e-6);
    click(&mut editor, &ctx, surface, "sliderSourceProperty");
    click(&mut editor, &ctx, surface, "sliderPropertyChoice:x");
    let changed = slider_mapping_range(editor.session.document(), 0).unwrap();
    assert_eq!(
        (changed.target_min, changed.target_max),
        (before.target_min, before.target_max)
    );
    click(&mut editor, &ctx, surface, "sliderSourceProperty");
    click(&mut editor, &ctx, surface, "sliderPropertyChoice:rotate");
    let restored = slider_mapping_range(editor.session.document(), 0).unwrap();
    assert!((restored.source_min - before.source_min).abs() < 1e-6);
    assert!((restored.source_max.unwrap() - before.source_max.unwrap()).abs() < 1e-4);
    let baseline = editor.session.document().clone();
    click(&mut editor, &ctx, surface, "sliderLocal");
    assert_ne!(
        editor.slider_constraint(0).unwrap().local,
        baseline.constraints[0].as_slider().unwrap().local
    );
    editor.undo();
    assert_eq!(editor.session.document(), &baseline);
    frame(&mut editor, &ctx, surface, vec![]);
    click(&mut editor, &ctx, surface, "sliderBoneClear");
    assert_eq!(editor.slider_constraint(0).unwrap().bone, None);
    assert!(editor.capture_targets.contains_key("sliderTimeValue"));
    assert!(!editor.capture_targets.contains_key("sliderDerivedFrame"));
    editor.undo();
    assert_eq!(editor.session.document(), &baseline);
}

#[test]
fn slider_source_ui_range_held_cancel_blocks_secondary_press_until_primary_release() {
    let surface = Surface::Properties;
    let (mut editor, ctx) = ready(true, surface);
    redo_branch(&mut editor);
    let baseline = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let start = rect(&editor, "sliderRangeSourceMin").center();
    pointer(
        &mut editor,
        &ctx,
        surface,
        start,
        egui::PointerButton::Primary,
        true,
    );
    for dx in [8., 24., 40.] {
        frame(
            &mut editor,
            &ctx,
            surface,
            vec![egui::Event::PointerMoved(start + vec2(dx, 0.))],
        );
    }
    assert!(
        editor.slider_edit_active(),
        "{}",
        editor.slider_edit_diagnostic()
    );
    assert_ne!(editor.session.document(), &baseline);
    key(
        &mut editor,
        &ctx,
        surface,
        egui::Key::Escape,
        egui::Modifiers::NONE,
    );
    assert_eq!(editor.session.document(), &baseline);
    assert!(editor.session.can_redo());
    pointer(
        &mut editor,
        &ctx,
        surface,
        start + vec2(45., 0.),
        egui::PointerButton::Secondary,
        true,
    );
    frame(
        &mut editor,
        &ctx,
        surface,
        vec![egui::Event::PointerMoved(start + vec2(50., 0.))],
    );
    assert!(!editor.slider_edit_active());
    assert_eq!(editor.session.document(), &baseline);
    pointer(
        &mut editor,
        &ctx,
        surface,
        start + vec2(50., 0.),
        egui::PointerButton::Secondary,
        false,
    );
    pointer(
        &mut editor,
        &ctx,
        surface,
        start + vec2(50., 0.),
        egui::PointerButton::Primary,
        false,
    );
    settle(&mut editor, &ctx, surface);
    let start = rect(&editor, "sliderRangeSourceMin").center();
    pointer(
        &mut editor,
        &ctx,
        surface,
        start,
        egui::PointerButton::Primary,
        true,
    );
    frame(
        &mut editor,
        &ctx,
        surface,
        vec![egui::Event::PointerMoved(start + vec2(30., 0.))],
    );
    pointer(
        &mut editor,
        &ctx,
        surface,
        start + vec2(30., 0.),
        egui::PointerButton::Primary,
        false,
    );
    assert_ne!(editor.session.document(), &baseline);
    editor.undo();
    assert_eq!(editor.session.document(), &baseline);
    assert_eq!(editor.session.pose_edits(), &pending);
}

#[test]
fn slider_source_ui_derived_frame_and_animate_controls_never_author_driven_time() {
    let surface = Surface::Properties;
    let (mut editor, ctx) = ready(true, surface);
    assert!(
        editor
            .session
            .slider_evaluation(0, EditTarget::Setup)
            .unwrap()
            .sample_time
            .is_some()
    );
    assert!(editor.capture_targets.contains_key("sliderDerivedFrame"));
    assert!(!editor.capture_targets.contains_key("sliderTimeValue"));
    assert!(!editor.capture_targets.contains_key("sliderTimeKey"));
    let baseline = editor.session.document().clone();
    typed(&mut editor, &ctx, surface, "sliderMixValue", "0");
    assert!(!editor.capture_targets.contains_key("sliderDerivedFrame"));
    assert!(
        editor
            .session
            .slider_evaluation(0, EditTarget::Setup)
            .unwrap()
            .sample_time
            .is_none()
    );
    editor.undo();
    assert_eq!(editor.session.document(), &baseline);
    editor.setup_mode = false;
    settle(&mut editor, &ctx, surface);
    let baseline = editor.session.document().clone();
    click(&mut editor, &ctx, surface, "sliderBoneChoose");
    assert!(!editor.slider_source_picker_active());
    click(&mut editor, &ctx, surface, "sliderLocal");
    click(&mut editor, &ctx, surface, "sliderRangeSourceMin");
    assert!(!ctx.text_edit_focused());
    assert_eq!(editor.session.document(), &baseline);
    assert!(editor.capture_targets.contains_key("sliderMixKey"));
    assert!(!editor.capture_targets.contains_key("sliderTimeKey"));
}

#[test]
fn slider_source_ui_undefined_source_max_focus_is_noop_and_typed_zero_repairs_range() {
    let surface = Surface::Properties;
    let (mut editor, ctx) = ready(true, surface);
    let mut value = editor.slider_constraint(0).unwrap().clone();
    value.scale = 0.;
    editor
        .session
        .dispatch(Command::SetSliderConstraint {
            constraint: 0,
            value,
        })
        .unwrap();
    redo_branch(&mut editor);
    settle(&mut editor, &ctx, surface);
    let baseline = editor.session.document().clone();
    assert_eq!(slider_mapping_range(&baseline, 0).unwrap().source_max, None);
    click(&mut editor, &ctx, surface, "sliderRangeSourceMax");
    assert!(ctx.text_edit_focused());
    key(
        &mut editor,
        &ctx,
        surface,
        egui::Key::Enter,
        egui::Modifiers::NONE,
    );
    assert_eq!(editor.session.document(), &baseline);
    assert!(editor.session.can_redo());
    typed(&mut editor, &ctx, surface, "sliderRangeSourceMax", "0");
    let range = slider_mapping_range(editor.session.document(), 0).unwrap();
    assert!((range.source_max.unwrap() - 100.).abs() < 1e-4);
    editor.undo();
    assert_eq!(editor.session.document(), &baseline);
}

#[test]
fn slider_source_ui_minimum_workspace_ranges_are_reachable_and_captures_are_clipped() {
    let surface = Surface::Workspace(vec2(1120., 740.));
    let (mut editor, ctx) = ready(true, surface);
    let original = editor.session.document().clone();
    let mut reached = BTreeSet::new();
    for _ in 0..18 {
        let clip = rect(&editor, "properties");
        for name in [
            "sliderRangeSourceMin",
            "sliderRangeSourceMax",
            "sliderRangeTargetMin",
            "sliderRangeTargetMax",
        ] {
            if editor.capture_targets.contains_key(name) {
                let target = rect(&editor, name);
                assert!(target.is_positive() && target.is_finite());
                assert!(clip.contains_rect(target));
                reached.insert(name);
            }
        }
        if reached.len() == 4 {
            break;
        }
        let (point, delta) = if editor.capture_targets.contains_key("sliderRangeScroll") {
            (rect(&editor, "sliderRangeScroll").center(), vec2(-100., 0.))
        } else {
            (pos2(clip.left() + 12., clip.center().y), vec2(0., -100.))
        };
        frame(
            &mut editor,
            &ctx,
            surface,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta,
                    phase: egui::TouchPhase::Move,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        settle(&mut editor, &ctx, surface);
    }
    assert_eq!(reached.len(), 4, "{reached:?}");
    assert_eq!(editor.session.document(), &original);
    typed(&mut editor, &ctx, surface, "sliderRangeTargetMax", "120");
    assert_eq!(
        slider_mapping_range(editor.session.document(), 0)
            .unwrap()
            .target_max,
        2.
    );
    editor.undo();
    assert_eq!(editor.session.document(), &original);
}

#[test]
fn slider_source_ui_owned_range_draft_save_cancels_and_stale_enter_cannot_publish() {
    let surface = Surface::Properties;
    let (mut editor, ctx) = ready(true, surface);
    let mut pending_bone = editor.session.local_pose(Some(1), 0.42)[8];
    pending_bone.rotation += 13.5;
    editor
        .session
        .edit_pose(
            8,
            EditTarget::Animate {
                animation: 1,
                time: 0.42,
            },
            pending_bone,
            &[Property::Rotation],
            false,
        )
        .unwrap();
    let baseline = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let dir = std::env::temp_dir().join(format!(
        "peachy-slider-range-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    editor.project_path = Some(dir.join("source.peachy"));
    editor.asset_root = dir.clone();
    draft(&mut editor, &ctx, surface, "sliderRangeSourceMin", "17.5");
    key(
        &mut editor,
        &ctx,
        surface,
        egui::Key::S,
        egui::Modifiers::COMMAND,
    );
    assert!(!editor.slider_edit_active());
    assert!(editor.error.is_none(), "{:?}", editor.error);
    assert_eq!(
        files::load(editor.project_path.as_ref().unwrap()).unwrap(),
        baseline
    );
    key(
        &mut editor,
        &ctx,
        surface,
        egui::Key::Enter,
        egui::Modifiers::NONE,
    );
    assert_eq!(editor.session.document(), &baseline);
    assert_eq!(editor.session.pose_edits(), &pending);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn slider_source_ui_leaving_context_and_successful_open_keep_process_ranges_only() {
    let surface = Surface::Properties;
    let (mut editor, ctx) = ready(true, surface);
    let baseline = editor.session.document().clone();
    let preferences = editor.session.slider_source_preferences().clone();
    let focus = editor.session.slider_source_focus();
    let choice = editor
        .session
        .slider_source_property_range(0, TransformProperty::ScaleX)
        .unwrap();
    assert_eq!((choice.min, choice.max), (1., 2.));
    assert_eq!(editor.session.slider_source_preferences(), &preferences);
    assert_eq!(editor.session.slider_source_focus(), focus);
    click(&mut editor, &ctx, surface, "sliderSourceProperty");
    click(&mut editor, &ctx, surface, "sliderPropertyChoice:scaleX");
    typed(&mut editor, &ctx, surface, "sliderRangeSourceMin", "8");
    typed(&mut editor, &ctx, surface, "sliderRangeSourceMax", "1");
    let accepted = editor
        .session
        .slider_source_property_range(0, TransformProperty::ScaleX)
        .unwrap();
    let preferences = editor.session.slider_source_preferences().clone();
    let focus = editor.session.slider_source_focus();
    let mut incoming = Session::new(baseline.clone()).unwrap();
    editor
        .carry_slider_source_preferences(&mut incoming)
        .unwrap();
    assert_eq!(editor.session.slider_source_preferences(), &preferences);
    assert_eq!(editor.session.slider_source_focus(), focus);
    click(&mut editor, &ctx, surface, "sliderBone");
    assert_eq!(editor.selection, Some(TreeItem::Bone(8)));
    assert_eq!(editor.session.slider_source_focus(), None);
    assert_eq!(incoming.document(), &baseline);
    assert!(!incoming.can_undo() && !incoming.can_redo());
    assert_eq!(incoming.slider_source_focus(), None);
    let kept = incoming
        .slider_source_property_range(0, TransformProperty::ScaleX)
        .unwrap();
    assert_eq!(kept, accepted);
    let fresh = Session::new(baseline)
        .unwrap()
        .slider_source_property_range(0, TransformProperty::ScaleX)
        .unwrap();
    assert_eq!((fresh.min, fresh.max), (1., 2.));
    editor.session = incoming;
    editor.select(TreeItem::Constraint(0));
    settle(&mut editor, &ctx, surface);
    click(&mut editor, &ctx, surface, "sliderSourceProperty");
    click(&mut editor, &ctx, surface, "sliderPropertyChoice:scaleX");
    let range = slider_mapping_range(editor.session.document(), 0).unwrap();
    assert_eq!(range.source_min, 8.);
    assert_eq!(
        editor
            .session
            .slider_source_property_range(0, TransformProperty::ScaleX)
            .unwrap(),
        accepted
    );
}
