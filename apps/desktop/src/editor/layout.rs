//! Shared content metrics keep transform rows and the Dopesheet inside their panels.
use super::*;

pub(super) const TRANSFORM_ROWS: [(Tool, &str, &[Property]); 4] = [
    (Tool::Rotate, "Rotate  [C]", &[Property::Rotation]),
    (Tool::Move, "Translate  [V]", &[Property::X, Property::Y]),
    (
        Tool::Scale,
        "Scale  [X]",
        &[Property::ScaleX, Property::ScaleY],
    ),
    (
        Tool::Shear,
        "Shear  [Z]",
        &[Property::ShearX, Property::ShearY],
    ),
];

pub(super) fn main_tools_height(ui: &egui::Ui) -> f32 {
    let spacing = ui.spacing().item_spacing.y;
    let button = ui
        .spacing()
        .interact_size
        .y
        .max(ui.text_style_height(&egui::TextStyle::Button) + 2. * ui.spacing().button_padding.y)
        .max(22.);
    let transforms = TRANSFORM_ROWS.len() as f32 * button
        + TRANSFORM_ROWS.len().saturating_sub(1) as f32 * spacing;
    // Reserve the Animate ancillary column even in Setup to avoid moving the canvas.
    let ancillary = ui.text_style_height(&egui::TextStyle::Body) + 3. * button + 3. * spacing;
    transforms.max(ancillary) + 14.
}

pub(super) fn timeline_row_pitch(ui: &egui::Ui) -> f32 {
    (ui.text_style_height(&egui::TextStyle::Body) + 11.).max(24.)
}
pub(super) fn timeline_chart_height(ui: &egui::Ui) -> f32 {
    // One Bone group heading plus all seven authored scalar channels.
    24. + (Property::ALL.len() + 1) as f32 * timeline_row_pitch(ui) + 13.
}
pub(super) fn timeline_panel_height(ui: &egui::Ui) -> f32 {
    let header = ui
        .spacing()
        .interact_size
        .y
        .max(ui.text_style_height(&egui::TextStyle::Button) + 2. * ui.spacing().button_padding.y)
        .max(22.);
    header
        + 5.
        + timeline_chart_height(ui)
        + ui.text_style_height(&egui::TextStyle::Small)
        + 2. * ui.spacing().item_spacing.y
        + 12.
}

pub(super) fn animation_panel_height(ui: &egui::Ui) -> f32 {
    (ui.ctx().content_rect().height() * 0.43).max(timeline_panel_height(ui) + 28.)
}

pub(super) fn compact_timeline_height(ui: &egui::Ui) -> f32 {
    let row = ui.spacing().interact_size.y.max(22.);
    2. * row + 44. + 2. * ui.spacing().item_spacing.y + 12.
}

fn control_height(ui: &egui::Ui) -> f32 {
    ui.spacing().interact_size.y.max(22.)
}

pub(super) fn canvas_header_height(ui: &egui::Ui) -> f32 {
    control_height(ui) + ui.spacing().item_spacing.y
}

pub(super) fn tree_min_height(ui: &egui::Ui) -> f32 {
    // Heading/filter/search, the Skeleton row, then four complete Bone bands.
    3. * control_height(ui)
        + 4. * ui.spacing().item_spacing.y
        + 5. * (control_height(ui) + ui.spacing().item_spacing.y)
        + 12.
}

pub(super) fn tree_footer_height(ui: &egui::Ui) -> f32 {
    control_height(ui) + 6.
}

pub(super) fn animation_min_height(ui: &egui::Ui) -> f32 {
    // Shared tabs/transport, ruler, one full virtual row and the footer. In a
    // tightly stacked workspace the remaining channels stay scrollable.
    2. * control_height(ui)
        + 26.
        + timeline_row_pitch(ui)
        + ui.text_style_height(&egui::TextStyle::Small)
        + 4. * ui.spacing().item_spacing.y
        + 12.
}

pub(super) fn timeline_chart_height_in(ui: &egui::Ui) -> f32 {
    let footer = ui.text_style_height(&egui::TextStyle::Small) + ui.spacing().item_spacing.y;
    (ui.available_height() - footer).max(26.)
}

#[derive(Clone, Copy)]
pub(super) struct PanelHeightBudget {
    pub default: f32,
    pub minimum: f32,
    pub maximum: f32,
}

#[derive(Clone, Copy)]
struct PreferredPanelHeight(f32);

/// Keep the native panel state equal to its actual rectangle, while retaining
/// the user's unconstrained preference across temporary workspace limits.
pub(super) fn adaptive_bottom_panel<R>(
    ui: &mut egui::Ui,
    name: &'static str,
    budget: PanelHeightBudget,
    frame: egui::Frame,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let panel_id = egui::Id::new(name);
    let preference_id = panel_id.with("workspace-preferred-height");
    let state = egui::containers::panel::PanelState::load(ui.ctx(), panel_id);
    let mut preferred = ui
        .ctx()
        .data_mut(|data| data.get_temp::<PreferredPanelHeight>(preference_id))
        .map_or_else(
            || state.map_or(budget.default, |state| state.size().y),
            |value| value.0,
        );
    if let Some(mut state) = state {
        state.outer_rect.min.y = state.outer_rect.max.y - preferred;
        ui.ctx()
            .data_mut(|data| data.insert_persisted(panel_id, state));
    }
    let before = ui.available_rect_before_wrap();
    let maximum = budget.maximum.max(budget.minimum).min(before.height());
    let minimum = budget.minimum.min(maximum);
    let expected = preferred.clamp(minimum, maximum);
    let response = egui::Panel::bottom(name)
        .default_size(preferred)
        .min_size(minimum)
        .max_size(maximum)
        .resizable(true)
        .frame(frame)
        .show(ui, contents);
    let actual = before.bottom() - ui.available_rect_before_wrap().bottom();
    // Panel edges round to physical pixels. Only a real resize away from the
    // constrained request replaces the remembered preference.
    // egui 0.36's native Panel resize response is distinct from its contents.
    // Observe it without creating a competing widget or changing hit geometry.
    let resizing = ui
        .ctx()
        .read_response(panel_id.with("__resize"))
        .is_some_and(|response| {
            response.dragged_by(egui::PointerButton::Primary)
                || response.drag_stopped_by(egui::PointerButton::Primary)
        });
    if resizing && (actual - expected).abs() > 0.5 / ui.ctx().pixels_per_point() {
        preferred = actual;
    }
    ui.ctx().data_mut(|data| {
        data.insert_temp(preference_id, PreferredPanelHeight(preferred));
    });
    response
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{editor_context, editor_fixture, mesh_editor};
    use super::*;

    fn target(editor: &Editor, name: &str) -> egui::Rect {
        let [left, top, right, bottom] = *editor.capture_targets.get(name).unwrap_or_else(|| {
            let relevant: Vec<_> = editor
                .capture_targets
                .iter()
                .filter(|(key, _)| {
                    key.starts_with("timelineRow:")
                        || key.starts_with("timelineOwner:")
                        || key.starts_with("value:")
                        || matches!(
                            key.as_str(),
                            "dopesheetClip"
                                | "timelineRowsClip"
                                | "timelineFooter"
                                | "mainToolsClip"
                        )
                })
                .collect();
            panic!("Missing layout target {name}; relevant targets: {relevant:?}")
        });
        egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom))
    }

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
    fn pointer(point: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }
    fn click(editor: &mut Editor, ctx: &egui::Context, size: Vec2, name: &str) {
        let point = target(editor, name).center();
        for pressed in [true, false] {
            frame(
                editor,
                ctx,
                size,
                vec![egui::Event::PointerMoved(point), pointer(point, pressed)],
            );
        }
        settle(editor, ctx, size);
    }
    fn wheel(editor: &mut Editor, ctx: &egui::Context, size: Vec2, point: Pos2, delta: f32) {
        frame(
            editor,
            ctx,
            size,
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
        settle(editor, ctx, size);
    }
    fn workspace_fixture() -> Editor {
        let mut editor = editor_fixture();
        let mut doc = peachy_io::decode_project(include_str!(
            "../../../../examples/peach-bot/multi-bone-bot.peachy"
        ))
        .unwrap();
        doc.animations[2] = Animation::new(
            "Seven authored properties",
            Property::ALL
                .into_iter()
                .map(|property| Track {
                    bone: 2,
                    property,
                    keys: [0., 1.]
                        .map(|time| Keyframe {
                            time,
                            value: property.neutral(),
                            curve: Curve::Linear,
                        })
                        .to_vec(),
                })
                .collect(),
        );
        editor.session = Session::new(doc).unwrap();
        editor.skin = "default".into();
        editor.session.set_skin(&editor.skin);
        editor.animation = 2;
        editor.time = 0.5;
        editor.setup_mode = false;
        editor.show_timeline = true;
        editor.show_dopesheet = true;
        editor.show_tree = true;
        editor.selection = Some(TreeItem::Bone(2)).into();
        editor.tree = TreeState::for_document(editor.session.document(), &editor.skin);
        editor.tree.filters = tree_model::Filters {
            bones: true,
            slots: false,
            attachments: false,
            skins: false,
            animations: false,
            constraints: false,
        };
        editor.tree.collapsed.clear();
        editor.key_selection = Default::default();
        let mut pose = editor.session.local_pose(Some(2), editor.time)[6];
        pose.rotation += 13.5;
        editor
            .session
            .edit_pose(
                6,
                EditTarget::Animate {
                    animation: 2,
                    time: editor.time,
                },
                pose,
                &[Property::Rotation],
                false,
            )
            .unwrap();
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "layout redo".into(),
            })
            .unwrap();
        editor.session.undo();
        editor
    }
    fn data_state(editor: &Editor) -> serde_json::Value {
        serde_json::json!({"document":editor.session.document(),"pending":editor.session.pose_edits(),
            "selection":editor.selection.diagnostic(),"animation":editor.animation,"time":editor.time,"skin":editor.skin,
            "undo":editor.session.can_undo(),"redo":editor.session.can_redo(),"dirty":editor.session.is_dirty()})
    }

    #[test]
    fn workspace_minimum_stacked_views_retain_canvas_four_tree_rows_and_real_controls() {
        for playback in [false, true] {
            let mut editor = workspace_fixture();
            let ctx = editor_context();
            let size = vec2(1120., 740.);
            settle(&mut editor, &ctx, size);
            if playback {
                click(&mut editor, &ctx, size, "viewsMenu");
                click(&mut editor, &ctx, size, "viewPlayback");
            }
            let before = data_state(&editor);
            let canvas = target(&editor, "viewport");
            assert!(
                canvas.height() >= 200. && canvas.width() >= 320.,
                "canvas={canvas:?}, playback={playback}"
            );
            let tree = target(&editor, "tree");
            let search = target(&editor, "treeSearch");
            let rows = egui::Rect::from_min_max(pos2(tree.left(), search.bottom() + 4.), tree.max);
            let visible: Vec<_> = editor
                .capture_targets
                .iter()
                .filter(|(name, rect)| {
                    name.starts_with("bone:")
                        && rows.contains_rect(egui::Rect::from_min_max(
                            pos2(rect[0], rect[1]),
                            pos2(rect[2], rect[3]),
                        ))
                })
                .map(|(name, _)| name)
                .collect();
            assert!(
                visible.len() >= 4,
                "Only {visible:?} fit in Tree rows {rows:?}, playback={playback}"
            );
            let tools = target(&editor, "mainToolsClip");
            for property in Property::ALL {
                assert!(tools.contains_rect(target(&editor, &format!("value:{property:?}"))));
            }
            for name in ["autoKey", "keyEdited", "key:X"] {
                assert!(tools.contains_rect(target(&editor, name)));
            }
            let compact = target(&editor, "compactTimelineClip");
            for name in ["compactFrameValue", "compactRuler", "compactZoomKeys"] {
                assert!(compact.contains_rect(target(&editor, name)));
            }
            assert!(
                target(&editor, "dopesheetClip").contains_rect(target(&editor, "timelineFooter"))
            );
            assert_eq!(data_state(&editor), before);
        }
    }

    #[test]
    fn workspace_minimum_dopesheet_scroll_reaches_every_authored_channel_without_data_changes() {
        for playback in [false, true] {
            let mut editor = workspace_fixture();
            let ctx = editor_context();
            let size = vec2(1120., 740.);
            settle(&mut editor, &ctx, size);
            if playback {
                click(&mut editor, &ctx, size, "viewsMenu");
                click(&mut editor, &ctx, size, "viewPlayback");
            }
            let before = data_state(&editor);
            let mut seen = BTreeSet::new();
            for _ in 0..32 {
                let clip = target(&editor, "timelineRowsClip");
                for property in Property::ALL {
                    let name = format!("timelineRow:{property:?}");
                    if editor.capture_targets.contains_key(&name)
                        && clip.contains_rect(target(&editor, &name))
                    {
                        seen.insert(property);
                    }
                }
                if seen.len() == Property::ALL.len() {
                    break;
                }
                wheel(&mut editor, &ctx, size, clip.center(), -12.);
            }
            assert_eq!(seen, Property::ALL.into_iter().collect());
            assert_eq!(data_state(&editor), before);
        }
    }

    #[test]
    fn workspace_minimum_properties_scroll_reaches_deform_actions_with_usable_tree_and_canvas() {
        let (mut editor, _) = mesh_editor(false);
        let ctx = editor_context();
        let size = vec2(1120., 740.);
        editor.set_setup_mode(false);
        editor.show_timeline = true;
        settle(&mut editor, &ctx, size);
        let before = data_state(&editor);
        let mut reached = false;
        for _ in 0..24 {
            let clip = target(&editor, "properties");
            reached = ["meshDeformKey", "meshDeformDelete"]
                .into_iter()
                .all(|name| {
                    editor.capture_targets.contains_key(name)
                        && clip.contains_rect(target(&editor, name))
                        && target(&editor, name).height() >= 17.
                });
            if reached {
                break;
            }
            wheel(
                &mut editor,
                &ctx,
                size,
                pos2(clip.left() + 12., clip.center().y),
                -80.,
            );
        }
        assert!(
            reached,
            "Deform actions must remain reachable through the actual Properties scroll area"
        );
        assert!(target(&editor, "viewport").height() >= 200.);
        assert_eq!(data_state(&editor), before);
    }

    fn panel_rect(ctx: &egui::Context, name: &str) -> egui::Rect {
        egui::containers::panel::PanelState::load(ctx, egui::Id::new(name))
            .unwrap()
            .outer_rect
    }
    fn grow_panel(editor: &mut Editor, ctx: &egui::Context, size: Vec2, name: &str, amount: f32) {
        let id = egui::Id::new(name).with("__resize");
        let response = ctx.read_response(id).unwrap();
        let start = response.rect.center() + vec2(0., 2.);
        assert!(response.interact_rect.contains(start));
        frame(editor, ctx, size, vec![egui::Event::PointerMoved(start)]);
        frame(editor, ctx, size, vec![pointer(start, true)]);
        frame(editor, ctx, size, vec![]);
        assert!(ctx.read_response(id).unwrap().is_pointer_button_down_on());
        let end = start - vec2(0., amount);
        for fraction in [0.25, 0.5, 1.] {
            frame(
                editor,
                ctx,
                size,
                vec![egui::Event::PointerMoved(start + (end - start) * fraction)],
            );
        }
        frame(editor, ctx, size, vec![pointer(end, false)]);
        settle(editor, ctx, size);
    }

    #[test]
    fn workspace_adaptive_heights_restore_real_user_resizes_after_temporary_minimum_bounds() {
        let mut editor = workspace_fixture();
        editor.show_timeline = false;
        let ctx = editor_context();
        // Both panels need genuine growth headroom before constraining them.
        // At 1000px the expanded animation dock already caps Properties at the
        // four-Bone Tree reservation, so a further 30px resize cannot succeed.
        let large = vec2(1500., 1200.);
        let small = vec2(1120., 740.);
        settle(&mut editor, &ctx, large);
        let before = data_state(&editor);
        let original_animation = panel_rect(&ctx, "animation-views").height();
        grow_panel(&mut editor, &ctx, large, "animation-views", 40.);
        let animation = panel_rect(&ctx, "animation-views").height();
        assert!(animation > original_animation + 20.);
        let original_properties = panel_rect(&ctx, "item-properties").height();
        grow_panel(&mut editor, &ctx, large, "item-properties", 30.);
        let properties = panel_rect(&ctx, "item-properties").height();
        assert!(properties > original_properties + 10.);
        editor.show_timeline = true;
        settle(&mut editor, &ctx, small);
        assert!(panel_rect(&ctx, "animation-views").height() < animation);
        assert!(panel_rect(&ctx, "item-properties").height() < properties);
        assert!(target(&editor, "viewport").height() >= 200.);
        editor.show_timeline = false;
        settle(&mut editor, &ctx, large);
        assert!((panel_rect(&ctx, "animation-views").height() - animation).abs() < 1.);
        assert!((panel_rect(&ctx, "item-properties").height() - properties).abs() < 1.);
        assert_eq!(data_state(&editor), before);
    }
    #[test]
    fn seven_dopesheet_rows_and_four_transform_rows_fit_minimum_panels_with_real_editor_style() {
        let mut editor = editor_fixture();
        let mut document = editor.session.document().clone();
        document.animations[0] = Animation::new(
            "Seven authored channels",
            Property::ALL
                .into_iter()
                .map(|property| Track {
                    bone: 8,
                    property,
                    keys: vec![Keyframe {
                        time: 0.,
                        value: property.neutral(),
                        curve: Curve::Linear,
                    }],
                })
                .collect(),
        );
        editor.session = Session::new(document).unwrap();
        editor.session.set_skin(&editor.skin);
        editor.key_selection = Default::default();
        editor.reconcile_selection();
        let ctx = editor_context();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1120., 740.))),
                focused: true,
                ..Default::default()
            },
            |ui| {
                egui::Panel::right("test-tree")
                    .exact_size(310.)
                    .show(ui, |_| {});
                let height = timeline_panel_height(ui);
                egui::Panel::bottom("test-dopesheet")
                    .exact_size(height)
                    .frame(egui::Frame::new().inner_margin(6))
                    .show(ui, |ui| editor.timeline(ui));
                egui::CentralPanel::default().show(ui, |ui| {
                    let height = main_tools_height(ui);
                    egui::Panel::bottom("test-tools")
                        .exact_size(height)
                        .frame(egui::Frame::new().inner_margin(7))
                        .show(ui, |ui| editor.main_tools(ui));
                });
            },
        );
        output.textures_delta.clear();
        let clip = target(&editor, "dopesheetClip");
        let header = target(&editor, "timelineOwner:bone:8");
        assert!(clip.contains_rect(header));
        for property in Property::ALL {
            let row = target(&editor, &format!("timelineRow:{property:?}"));
            assert!(row.width() > 0. && row.height() > 0., "{property:?}");
            assert!(
                clip.contains_rect(row),
                "{property:?}: {row:?} outside {clip:?}"
            );
        }
        assert!(clip.contains_rect(target(&editor, "timelineFooter")));
        let clip = target(&editor, "mainToolsClip");
        for property in Property::ALL {
            let value = target(&editor, &format!("value:{property:?}"));
            assert!(
                clip.contains_rect(value),
                "{property:?}: {value:?} outside {clip:?}"
            );
        }
    }
}
