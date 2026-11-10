use super::super::test_support::{editor_context, editor_fixture, mesh_editor, workspace_frame};
use super::*;

fn settle(editor: &mut Editor, ctx: &egui::Context) {
    for _ in 0..3 {
        workspace_frame(editor, ctx, vec![]);
    }
}

fn point(editor: &Editor, name: &str) -> Pos2 {
    let [left, top, right, bottom] = editor.capture_targets[name];
    pos2((left + right) * 0.5, (top + bottom) * 0.5)
}

fn pointer(
    editor: &mut Editor,
    ctx: &egui::Context,
    position: Pos2,
    pressed: bool,
    button: egui::PointerButton,
) {
    workspace_frame(
        editor,
        ctx,
        vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
}

fn click(editor: &mut Editor, ctx: &egui::Context, name: &str) {
    let position = point(editor, name);
    for pressed in [true, false] {
        pointer(editor, ctx, position, pressed, egui::PointerButton::Primary);
    }
    settle(editor, ctx);
}

fn key(editor: &mut Editor, ctx: &egui::Context, key: egui::Key, modifiers: egui::Modifiers) {
    for pressed in [true, false] {
        workspace_frame(
            editor,
            ctx,
            vec![egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed,
                repeat: false,
                modifiers,
            }],
        );
    }
}

fn main_state(editor: &Editor) -> serde_json::Value {
    serde_json::json!({
        "document":editor.session.document(), "pending":editor.session.pose_edits(),
        "selection":editor.selection.as_ref().map(TreeItem::key),
        "animation":editor.animation,"skin":editor.skin,"time":editor.time,
        "playing":editor.playing,"direction":editor.play_direction,"speed":editor.playback_speed,
        "looping":editor.looping,"tool":editor.tool as u8,
        "pan":[editor.pan.x,editor.pan.y],"zoom":editor.zoom,"fit":editor.fit,
        "undo":editor.session.can_undo(),"redo":editor.session.can_redo(),"dirty":editor.session.is_dirty()
    })
}

fn pending(editor: &mut Editor) {
    let mut pose = editor.selected_transform().unwrap();
    pose.rotation += 13.;
    editor.edit_transform(pose, &[Property::Rotation]);
    assert!(!editor.session.pose_edits().is_empty());
}

fn fixture() -> (Editor, egui::Context) {
    let mut editor = editor_fixture();
    let mut doc = editor.session.document().clone();
    doc.animations.push(Animation {
        name: "Preview alternate".into(),
        tracks: vec![Track {
            bone: 8,
            property: Property::Rotation,
            keys: vec![
                Keyframe {
                    time: 0.,
                    value: 0.,
                    curve: Curve::Linear,
                },
                Keyframe {
                    time: 1.,
                    value: 45.,
                    curve: Curve::Linear,
                },
            ],
        }],
        physics: vec![],
        physics_resets: vec![],
        ik_tracks: vec![],
        inherit_tracks: vec![],
        deform_tracks: vec![],
        attachment_tracks: vec![],
        color_tracks: vec![],
        events: vec![],
        draw_order: vec![],
    });
    editor.session = Session::new(doc).unwrap();
    pending(&mut editor);
    editor.open_preview_view();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    (editor, ctx)
}

#[test]
fn animation_skin_seek_and_transport_controls_preserve_the_main_pending_editor_and_history() {
    let (mut editor, ctx) = fixture();
    let main = main_state(&editor);
    let alternate = editor.session.document().animations.len() - 1;
    click(&mut editor, &ctx, "previewAnimation");
    click(&mut editor, &ctx, &format!("previewAnimation:{alternate}"));
    assert_eq!(editor.preview.animation, alternate);
    click(&mut editor, &ctx, "previewSkin");
    click(&mut editor, &ctx, "previewSkin:mint");
    assert_eq!(editor.preview.skin, "mint");
    let [left, top, right, bottom] = editor.capture_targets["previewScrub"];
    let seek = pos2(left + (right - left) * 0.4, (top + bottom) * 0.5);
    for pressed in [true, false] {
        pointer(
            &mut editor,
            &ctx,
            seek,
            pressed,
            egui::PointerButton::Primary,
        );
    }
    assert!(editor.preview.time > 0.2 && editor.preview.time < 0.6);
    assert!(!editor.preview.playing);
    click(&mut editor, &ctx, "previewForward");
    assert!(editor.preview.playing);
    assert!(editor.preview.time > 0.2);
    click(&mut editor, &ctx, "previewForward");
    assert!(!editor.preview.playing);
    click(&mut editor, &ctx, "previewEnd");
    assert_eq!(editor.preview.time, 1.);
    click(&mut editor, &ctx, "previewStart");
    assert_eq!(editor.preview.time, 0.);
    click(&mut editor, &ctx, "previewLoop");
    assert!(!editor.preview.looping);
    let speed = point(&editor, "previewSpeed");
    pointer(&mut editor, &ctx, speed, true, egui::PointerButton::Primary);
    let moved = speed + vec2(20., 0.);
    workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(moved)]);
    pointer(
        &mut editor,
        &ctx,
        moved,
        false,
        egui::PointerButton::Primary,
    );
    assert!(editor.preview.speed > 1. && editor.preview.speed <= 2.);
    assert_eq!(main_state(&editor), main);
}

#[test]
fn independent_transport_uses_engine_loop_boundaries_both_directions_and_safe_elapsed_time() {
    let mut preview = PreviewView {
        speed: 0.5,
        time: 0.98,
        ..Default::default()
    };
    preview.toggle(PlayDirection::Forward, 1.);
    preview.advance(0.08, 1.);
    assert!((preview.time - 0.02).abs() < 0.000001);
    preview.toggle(PlayDirection::Backward, 1.);
    preview.advance(0.08, 1.);
    assert!((preview.time - 0.98).abs() < 0.000001);
    preview.looping = false;
    preview.seek(0.99, 1.);
    preview.toggle(PlayDirection::Forward, 1.);
    preview.advance(0.08, 1.);
    assert_eq!(preview.time, 1.);
    assert!(!preview.playing);
    preview.toggle(PlayDirection::Backward, 1.);
    preview.seek(0.01, 1.);
    preview.playing = true;
    preview.advance(0.08, 1.);
    assert_eq!(preview.time, 0.);
    assert!(!preview.playing);
    preview.toggle(PlayDirection::Backward, 1.);
    assert_eq!(preview.time, 1.);
    preview.advance(f32::NAN, 1.);
    assert_eq!(preview.time, 1.);
    preview.advance(1000., 1.);
    assert!((preview.time - 0.95).abs() < 0.000001);
    preview.toggle(PlayDirection::Forward, 0.);
    assert_eq!(preview.time, 0.);
    assert!(!preview.playing);
}

#[test]
fn preview_reads_checked_keyed_weighted_mesh_and_ik_geometry_without_pending_pose_overrides() {
    let (mut editor, _) = mesh_editor(true);
    let mut doc = editor.session.document().clone();
    let target = doc.bones.len();
    doc.bones.push(Bone {
        name: "preview-target".into(),
        parent: Some(0),
        length: 0.,
        setup: Transform {
            x: 130.,
            y: 200.,
            ..Default::default()
        },
        inherit: peachy_document::Inherit::Normal,
    });
    doc.constraints.push(Constraint::Ik(IkConstraint {
        name: "preview IK".into(),
        bones: vec![8],
        target,
        setup: IkSetup {
            mix: 0.5,
            bend_positive: true,
        },
    }));
    editor.session = Session::new(doc).unwrap();
    editor.setup_mode = false;
    editor.selection = Some(TreeItem::Bone(8));
    pending(&mut editor);
    editor.open_preview_view();
    let ctx = editor_context();
    settle(&mut editor, &ctx);
    let main = main_state(&editor);
    let mut ik_applied = false;
    for time in [0., 0.4, 0.8] {
        editor.preview.time = time;
        let actual = editor.preview_scene().unwrap();
        let expected =
            checked_evaluate_scene(editor.session.document(), Some(0), time, "peach", None)
                .unwrap();
        assert_eq!(
            serde_json::to_value(&actual.attachments).unwrap(),
            serde_json::to_value(&expected.attachments).unwrap()
        );
        assert_eq!(actual.applied_locals, expected.applied_locals);
        ik_applied |= actual.applied_locals[8].rotation != actual.locals[8].rotation;
        assert!(actual.attachments.iter().any(|draw| matches!(editor.session.document().attachment(&draw.attachment),Some(Attachment::Mesh(mesh)) if mesh.is_weighted())));
        assert!(actual.attachments.iter().any(|draw| matches!(
            editor.session.document().attachment(&draw.attachment),
            Some(Attachment::Region(_))
        )));
        assert_ne!(
            editor.session.local_pose(Some(0), editor.time)[8].rotation,
            actual.locals[8].rotation
        );
    }
    assert!(ik_applied);
    click(&mut editor, &ctx, "previewBones");
    assert!(editor.preview.show_bones);
    assert!(editor.preview.error.is_none());
    assert_eq!(main_state(&editor), main);
}

#[test]
fn a_checked_preview_error_stops_only_preview_and_switching_to_a_valid_clip_recovers() {
    let (mut editor, ctx) = fixture();
    let mut doc = editor.session.document().clone();
    doc.animations[0].tracks = vec![Track {
        bone: 0,
        property: Property::ScaleX,
        keys: vec![
            Keyframe {
                time: 0.,
                value: 0.,
                curve: Curve::Bezier([0.25, f32::MAX, 0.75, f32::MAX]),
            },
            Keyframe {
                time: 1.,
                value: 2.,
                curve: Curve::Linear,
            },
        ],
    }];
    editor.session = Session::new(doc).unwrap();
    editor.animation = 1;
    editor.time = 0.2;
    pending(&mut editor);
    editor.preview.time = 0.5;
    editor.preview.playing = true;
    let main = main_state(&editor);
    settle(&mut editor, &ctx);
    assert!(!editor.preview.playing);
    assert!(
        editor
            .preview
            .error
            .as_ref()
            .is_some_and(|error| error.contains("nonfinite"))
    );
    assert!(editor.error.is_none());
    assert!(editor.evaluation_error.is_none());
    assert!(editor.preview_diagnostic()["attachments"].is_null());
    assert_eq!(main_state(&editor), main);
    click(&mut editor, &ctx, "previewAnimation");
    click(&mut editor, &ctx, "previewAnimation:1");
    assert!(editor.preview.error.is_none());
    assert_eq!(editor.preview.time, 0.);
    assert_eq!(main_state(&editor), main);
}

#[test]
fn close_minimize_focus_loss_and_document_reset_pause_while_setup_keeps_preview_available() {
    let (mut editor, ctx) = fixture();
    editor.preview.playing = true;
    editor.set_setup_mode(true);
    settle(&mut editor, &ctx);
    assert!(editor.preview_visible());
    assert!(editor.preview.playing);
    assert!(editor.preview.time > 0.);
    click(&mut editor, &ctx, "closeView:Preview");
    assert!(!editor.preview_visible());
    assert!(!editor.preview.playing);
    let time = editor.preview.time;
    settle(&mut editor, &ctx);
    assert_eq!(editor.preview.time, time);
    editor.open_preview_view();
    settle(&mut editor, &ctx);
    assert!(!editor.preview.playing);
    click(&mut editor, &ctx, "previewViewport");
    editor.preview.playing = true;
    key(&mut editor, &ctx, egui::Key::F9, egui::Modifiers::NONE);
    assert!(!editor.preview_visible());
    assert!(!editor.preview.playing);
    key(&mut editor, &ctx, egui::Key::F9, egui::Modifiers::NONE);
    settle(&mut editor, &ctx);
    assert!(editor.preview_visible());
    editor.preview.playing = true;
    let mut out = ctx.run_ui(
        egui::RawInput {
            focused: false,
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
            ..Default::default()
        },
        |ui| editor.workspace_ui(ui),
    );
    out.textures_delta.clear();
    assert!(!editor.preview.playing);
    editor.preview.animation = usize::MAX;
    editor.preview.skin = "old-document-skin".into();
    editor.preview.time = 8.;
    editor.preview.speed = 0.25;
    editor.preview.show_bones = true;
    editor.preview.reset_document();
    assert_eq!(editor.preview.animation, 0);
    assert!(editor.preview.skin.is_empty());
    assert_eq!(editor.preview.time, 0.);
    assert!(!editor.preview.playing);
    assert!(editor.preview.fit);
    assert_eq!(editor.preview.speed, 0.25);
    assert!(editor.preview.show_bones);
}

#[test]
fn preview_canvas_pan_zoom_fit_and_keyboard_focus_do_not_edit_or_navigate_the_main_editor() {
    let (mut editor, ctx) = fixture();
    let main = main_state(&editor);
    let center = point(&editor, "previewViewport");
    let zoom = editor.preview.zoom;
    workspace_frame(
        &mut editor,
        &ctx,
        vec![
            egui::Event::PointerMoved(center),
            egui::Event::MouseWheel {
                phase: egui::TouchPhase::Move,
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0., 120.),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    assert!(editor.preview.zoom > zoom);
    pointer(
        &mut editor,
        &ctx,
        center,
        true,
        egui::PointerButton::Secondary,
    );
    workspace_frame(
        &mut editor,
        &ctx,
        vec![egui::Event::PointerMoved(center + vec2(25., 15.))],
    );
    pointer(
        &mut editor,
        &ctx,
        center + vec2(25., 15.),
        false,
        egui::PointerButton::Secondary,
    );
    assert!(editor.preview.pan.length() > 0.);
    click(&mut editor, &ctx, "previewFit");
    assert_eq!(editor.preview.pan, Vec2::ZERO);
    click(&mut editor, &ctx, "previewViewport");
    for name in [
        egui::Key::C,
        egui::Key::V,
        egui::Key::X,
        egui::Key::Z,
        egui::Key::G,
        egui::Key::ArrowRight,
    ] {
        key(&mut editor, &ctx, name, egui::Modifiers::NONE);
    }
    assert_eq!(main_state(&editor), main);
    let time = editor.preview.time;
    key(&mut editor, &ctx, egui::Key::R, egui::Modifiers::NONE);
    assert!(editor.preview.time > time);
    editor.preview.playing = true;
    key(&mut editor, &ctx, egui::Key::Escape, egui::Modifiers::NONE);
    assert!(!editor.preview.playing);
    assert!(!ctx.memory(|memory| memory.has_focus(Editor::preview_canvas_id())));
    assert_eq!(main_state(&editor), main);
}

#[test]
fn minimum_dock_controls_and_canvas_remain_finite_including_empty_and_extreme_native_clips() {
    let (mut editor, ctx) = fixture();
    let doc = editor.session.document().clone();
    for _ in 0..3 {
        let mut out = ctx.run_ui(
            egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1120., 740.))),
                ..Default::default()
            },
            |ui| editor.workspace_ui(ui),
        );
        out.textures_delta.clear();
    }
    let [left, top, right, bottom] = editor.capture_targets["previewPanelClip"];
    for name in [
        "previewAnimation",
        "previewSkin",
        "previewStart",
        "previewEnd",
        "previewForward",
        "previewBackward",
        "previewLoop",
        "previewTime",
        "previewScrub",
        "previewSpeed",
        "previewFit",
        "previewBones",
        "previewViewport",
    ] {
        let [l, t, r, b] = editor.capture_targets[name];
        assert!([l, t, r, b].into_iter().all(f32::is_finite), "{name}");
        assert!(
            l >= left && r <= right + 1. && t >= top && b <= bottom + 1.,
            "{name}: {:?} outside {:?}",
            [l, t, r, b],
            [left, top, right, bottom]
        );
    }
    let [_, top, _, bottom] = editor.capture_targets["previewViewport"];
    assert!(bottom - top >= 100.);
    assert_eq!(editor.session.document(), &doc);
    let mut empty = doc.clone();
    empty.animations.clear();
    editor.session = Session::new(empty).unwrap();
    editor.preview.reset_document();
    settle(&mut editor, &ctx);
    assert_eq!(editor.preview.time, 0.);
    assert!(!editor.preview.playing);
    assert!(editor.preview.error.is_none());
    let mut no_skins = editor.session.document().clone();
    no_skins.skins.clear();
    no_skins.slots.clear();
    editor.session = Session::new(no_skins).unwrap();
    editor.preview.reset_document();
    settle(&mut editor, &ctx);
    editor.preview.pan = vec2(15., 20.);
    settle(&mut editor, &ctx);
    assert_eq!(editor.preview.pan, vec2(15., 20.));
    assert!(!editor.preview.fit);
    let mut huge = doc;
    huge.bones[0].setup.x = f32::MAX * 0.25;
    huge.animations[0].tracks = vec![Track {
        bone: 0,
        property: Property::Rotation,
        keys: vec![Keyframe {
            time: f32::MAX,
            value: 0.,
            curve: Curve::Linear,
        }],
    }];
    editor.session = Session::new(huge).unwrap();
    editor.preview.reset_document();
    editor.preview.reconcile(editor.session.document());
    editor.preview.seek(f64::from(f32::MAX), f32::MAX);
    settle(&mut editor, &ctx);
    assert_eq!(editor.preview.time, f32::MAX);
    assert!(editor.preview.error.is_none(), "{:?}", editor.preview.error);
    let [l, t, r, b] = editor.capture_targets["previewViewport"];
    let rect = egui::Rect::from_min_max(pos2(l, t), pos2(r, b));
    let scene = editor.preview_scene().unwrap();
    assert!(
        scene
            .attachments
            .iter()
            .flat_map(|draw| &draw.vertices)
            .all(|&point| editor.preview.screen(rect, point).is_finite())
    );
    assert!(
        editor.preview_diagnostic()["time"]
            .as_f64()
            .unwrap()
            .is_finite()
    );
}
