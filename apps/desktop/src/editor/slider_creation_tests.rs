//! Native creation input and structural identity survive shared history transitions.
use super::super::test_support::{editor_context, editor_fixture};
use super::*;
use peachy_engine::{AnimationKeyClipboard, IkPoseEdit, SliderPoseEdit};

fn frame(editor: &mut Editor, ctx: &egui::Context, size: Vec2, events: Vec<egui::Event>) {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, size)),
            focused: true,
            events,
            ..Default::default()
        },
        |ui| editor.workspace_ui(ui),
    );
    output.textures_delta.clear();
}

fn settle(editor: &mut Editor, ctx: &egui::Context, size: Vec2) {
    for _ in 0..4 {
        frame(editor, ctx, size, vec![]);
    }
}

fn rect(editor: &Editor, target: &str) -> egui::Rect {
    let [left, top, right, bottom] = editor.capture_targets[target];
    egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom))
}

fn click(editor: &mut Editor, ctx: &egui::Context, size: Vec2, target: &str) {
    let point = rect(editor, target).center();
    for pressed in [true, false] {
        frame(
            editor,
            ctx,
            size,
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
    settle(editor, ctx, size);
}

fn key(editor: &mut Editor, ctx: &egui::Context, code: egui::Key, modifiers: egui::Modifiers) {
    for pressed in [true, false] {
        frame(
            editor,
            ctx,
            vec2(1500., 960.),
            vec![
                egui::Event::ModifiersChanged(modifiers),
                egui::Event::Key {
                    key: code,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers,
                },
            ],
        );
    }
    frame(
        editor,
        ctx,
        vec2(1500., 960.),
        vec![egui::Event::ModifiersChanged(egui::Modifiers::NONE)],
    );
}

fn ready() -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    let mut document = editor.session.document().clone();
    document.skin_scope = Default::default();
    document.constraints = vec![
        Constraint::Ik(IkConstraint {
            name: "existing IK".into(),
            bones: vec![8],
            target: 0,
            setup: IkSetup {
                mix: 0.75,
                bend_positive: true,
            },
            stretch: false,
        }),
        Constraint::Slider(SliderConstraint::new("reference", 0)),
    ];
    document.animations = vec![
        Animation::new(
            "reference",
            vec![Track {
                bone: 8,
                property: Property::X,
                keys: vec![Keyframe {
                    time: 1.5,
                    value: 80.,
                    curve: Curve::Linear,
                }],
            }],
        ),
        Animation::new("host", vec![]),
    ];
    document.animations[1].ik_tracks.push(IkTrack {
        constraint: 0,
        keys: vec![IkKeyframe {
            time: 0.123456,
            mix: 0.6,
            bend_positive: false,
            curve: IkCurve::Linear,
        }],
    });
    document.animations[1].slider_tracks.push(SliderTrack {
        constraint: 1,
        property: SliderProperty::Mix,
        keys: vec![Keyframe {
            time: 0.123456,
            value: 0.4,
            curve: Curve::Bezier([0.2, 0.3, 0.8, 0.9]),
        }],
    });
    editor.session = Session::new(document).unwrap();
    editor.session.set_skin(&editor.skin);
    let target = EditTarget::Animate {
        animation: 1,
        time: 0.42,
    };
    editor
        .session
        .edit_ik_pose(0, target, &[IkPoseEdit::Mix { value: 0.55 }], false)
        .unwrap();
    editor
        .session
        .edit_slider_pose(1, target, &[SliderPoseEdit::Mix { value: 0.37 }], false)
        .unwrap();
    let mut pose = editor.session.local_pose(Some(1), 0.42)[8];
    pose.rotation += 13.5;
    editor
        .session
        .edit_pose(8, target, pose, &[Property::Rotation], false)
        .unwrap();
    // Keep a mixed pending state to verify remapping; no mode-switch action is
    // invoked here, since that deliberately discards pending poses.
    editor.setup_mode = true;
    editor.animation = 1;
    editor.time = 0.42;
    editor.tree = TreeState::for_document(editor.session.document(), &editor.skin);
    editor.tree.query = "reference".into();
    editor.select(TreeItem::Bone(8));
    let ctx = editor_context();
    settle(&mut editor, &ctx, vec2(1500., 960.));
    (editor, ctx)
}

#[test]
fn slider_creation_setup_animation_selection_and_menu_cancel_preserve_context_and_redo() {
    let (mut editor, ctx) = ready();
    editor
        .session
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "redo marker".into(),
        })
        .unwrap();
    editor.session.undo();
    let document = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let history = (editor.session.can_undo(), editor.session.can_redo());
    assert!(history.1);
    click(&mut editor, &ctx, vec2(1500., 960.), "animation:0");
    assert_eq!(editor.selection, Some(TreeItem::Animation(0)));
    assert!(editor.setup_mode);
    assert_eq!((editor.animation, editor.time), (1, 0.42));
    click(&mut editor, &ctx, vec2(1500., 960.), "treeNew");
    assert!(editor.capture_targets.contains_key("newSlider"));
    key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(!egui::Popup::is_any_open(&ctx));
    assert_eq!(editor.session.document(), &document);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(
        (editor.session.can_undo(), editor.session.can_redo()),
        history
    );
    assert_eq!(editor.selection, Some(TreeItem::Animation(0)));
}

#[test]
fn slider_creation_native_menu_remaps_pending_and_groups_with_one_undo_redo() {
    let (mut editor, ctx) = ready();
    editor
        .selection_history
        .store(1, &TreeSelection::singleton(TreeItem::Constraint(0)));
    editor
        .selection_history
        .store(2, &TreeSelection::singleton(TreeItem::Constraint(1)));
    editor.timeline_clipboard = Some(AnimationKeyClipboard::Slider {
        keys: vec![peachy_engine::SliderClipboardKey {
            property: SliderProperty::Mix,
            key: Keyframe {
                time: 0.,
                value: 0.12345679,
                curve: Curve::Bezier([0.2, 0.3, 0.8, 0.9]),
            },
        }],
    });
    let document = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let clipboard = editor.timeline_clipboard.clone();
    click(&mut editor, &ctx, vec2(1500., 960.), "animation:0");
    click(&mut editor, &ctx, vec2(1500., 960.), "treeNew");
    click(&mut editor, &ctx, vec2(1500., 960.), "newSlider");
    let created = editor.session.document().clone();
    assert_eq!(created.constraints[1..], document.constraints);
    let slider = created.constraints[0].as_slider().unwrap();
    assert_eq!(slider.name, "reference2");
    assert_eq!(slider.animation, 0);
    assert_eq!((slider.bone, slider.property), (None, None));
    assert_eq!((slider.setup.time, slider.setup.mix), (0., 1.));
    assert!(!slider.looped && !slider.additive);
    assert_eq!(editor.selection, Some(TreeItem::Constraint(0)));
    assert!(editor.setup_mode);
    assert_eq!((editor.animation, editor.time), (1, 0.42));
    assert!(editor.session.pose_edits().has_ik(1, 1));
    assert!(
        editor
            .session
            .pose_edits()
            .has_slider(1, 2, SliderProperty::Mix)
    );
    assert!(editor.session.pose_edits().has_bone(1, 8));
    assert_eq!(created.animations[1].ik_tracks[0].constraint, 1);
    assert_eq!(
        created.animations[1].ik_tracks[0].keys,
        document.animations[1].ik_tracks[0].keys
    );
    assert_eq!(created.animations[1].slider_tracks[0].constraint, 2);
    assert_eq!(
        created.animations[1].slider_tracks[0].keys,
        document.animations[1].slider_tracks[0].keys
    );
    assert_eq!(
        editor
            .selection_history
            .recall(1)
            .and_then(|selection| selection.single().cloned()),
        Some(TreeItem::Constraint(1))
    );
    assert_eq!(
        editor
            .selection_history
            .recall(2)
            .and_then(|selection| selection.single().cloned()),
        Some(TreeItem::Constraint(2))
    );
    assert_eq!(editor.timeline_clipboard, clipboard);
    key(&mut editor, &ctx, egui::Key::Z, egui::Modifiers::COMMAND);
    assert_eq!(editor.session.document(), &document);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(
        editor.selection, None,
        "removed Slider must not select the old constraint at index zero"
    );
    assert_eq!(
        editor
            .selection_history
            .recall(1)
            .and_then(|selection| selection.single().cloned()),
        Some(TreeItem::Constraint(0))
    );
    assert_eq!(
        editor
            .selection_history
            .recall(2)
            .and_then(|selection| selection.single().cloned()),
        Some(TreeItem::Constraint(1))
    );
    key(
        &mut editor,
        &ctx,
        egui::Key::Z,
        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
    );
    assert_eq!(editor.session.document(), &created);
    assert!(
        editor
            .session
            .pose_edits()
            .has_slider(1, 2, SliderProperty::Mix)
    );
    assert_eq!(editor.timeline_clipboard, clipboard);
    key(&mut editor, &ctx, egui::Key::Num2, egui::Modifiers::NONE);
    assert_eq!(editor.selection, Some(TreeItem::Constraint(2)));
    assert_eq!(editor.slider_constraint(2).unwrap().name, "reference");
}

#[test]
fn slider_creation_menu_rejects_animate_and_nonanimation_selection_without_cached_fallback() {
    let (mut editor, ctx) = ready();
    let document = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    editor.setup_mode = false;
    editor.select(TreeItem::Animation(0));
    settle(&mut editor, &ctx, vec2(1500., 960.));
    click(&mut editor, &ctx, vec2(1500., 960.), "treeNew");
    click(&mut editor, &ctx, vec2(1500., 960.), "newSlider");
    key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    editor.setup_mode = true;
    editor.select(TreeItem::Bone(8));
    settle(&mut editor, &ctx, vec2(1500., 960.));
    click(&mut editor, &ctx, vec2(1500., 960.), "treeNew");
    click(&mut editor, &ctx, vec2(1500., 960.), "newSlider");
    key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert_eq!(editor.session.document(), &document);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.selection, Some(TreeItem::Bone(8)));
    assert_eq!(editor.animation, 1);
}

#[test]
fn slider_creation_failed_command_never_replays_previous_structural_effects() {
    let (mut editor, _) = ready();
    editor
        .selection_history
        .store(2, &TreeSelection::singleton(TreeItem::Constraint(1)));
    assert!(editor.dispatch(Command::CreateSliderConstraint { animation: 0 }));
    editor.select(TreeItem::Constraint(2));
    let document = editor.session.document().clone();
    let pending = editor.session.pose_edits().clone();
    let generation = editor.pose_view_generation;
    assert!(!editor.dispatch(Command::CreateSliderConstraint {
        animation: usize::MAX
    }));
    assert_eq!(editor.session.document(), &document);
    assert_eq!(editor.session.pose_edits(), &pending);
    assert_eq!(editor.pose_view_generation, generation);
    assert_eq!(editor.selection, Some(TreeItem::Constraint(2)));
    assert_eq!(
        editor
            .selection_history
            .recall(2)
            .and_then(|selection| selection.single().cloned()),
        Some(TreeItem::Constraint(2))
    );
}

#[test]
fn slider_creation_minimum_workspace_footer_and_popup_remain_visible() {
    let (mut editor, ctx) = ready();
    let size = vec2(1120., 740.);
    editor.show_weights = true;
    editor.select(TreeItem::Animation(0));
    settle(&mut editor, &ctx, size);
    let screen = egui::Rect::from_min_size(Pos2::ZERO, size);
    let footer = rect(&editor, "treeNew");
    assert!(footer.is_positive() && screen.contains_rect(footer));
    assert!(rect(&editor, "viewport").width() >= 320.);
    click(&mut editor, &ctx, size, "treeNew");
    let entry = rect(&editor, "newSlider");
    assert!(entry.is_positive() && screen.contains_rect(entry));
    click(&mut editor, &ctx, size, "newSlider");
    assert_eq!(editor.selection, Some(TreeItem::Constraint(0)));
    assert_eq!(editor.slider_constraint(0).unwrap().animation, 0);
}
