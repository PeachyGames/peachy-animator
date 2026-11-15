//! Direct control routing only. Session owns redistribution and fixed-base history.
use super::*;
use peachy_engine::{
    MeshBindPose, MeshWeightGesture, MeshWeightSelection, MeshWeightValue, mesh_bone_weights,
};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct WeightTarget {
    mesh: AttachmentRef,
    vertices: Vec<usize>,
    bone: usize,
}

pub(super) struct WeightEdit {
    target: WeightTarget,
    pose: MeshBindPose,
    gesture: MeshWeightGesture,
    setup: bool,
    animation: usize,
    time: f32,
    control: egui::Id,
    numeric: bool,
    numeric_touched: bool,
    percent: Option<f64>,
    finish_pending: bool,
    error: Option<String>,
}

impl Editor {
    fn direct_target(&self) -> Result<(WeightTarget, Option<f64>), String> {
        let mesh = self.weights.mesh.clone().ok_or("Select an active mesh")?;
        if !self.weights_mesh_active(&mesh) {
            return Err("Select an active mesh".into());
        }
        if self.weights.vertices.is_empty() {
            return Err("Select vertices".into());
        }
        let bone = self.weights.bone.ok_or("Select one bound bone")?;
        let query = mesh_bone_weights(
            self.session.document(),
            &MeshWeightSelection {
                attachment: mesh.clone(),
                vertices: self.weights.vertices.clone(),
                bone,
            },
        )?;
        Ok((
            WeightTarget {
                mesh,
                vertices: query.selection.vertices,
                bone,
            },
            match query.value {
                MeshWeightValue::Uniform(weight) => Some(f64::from(weight) * 100.),
                MeshWeightValue::Mixed => None,
            },
        ))
    }
    fn begin_weight_edit(&mut self, control: egui::Id, numeric: bool) -> Result<(), String> {
        self.cancel_gesture();
        self.set_tool(Tool::Weights);
        self.reconcile_weights();
        let (target, percent) = self.direct_target()?;
        self.session.commit_transaction();
        self.playing = false;
        let pose = self.session.mesh_bind_pose(self.weights_edit_target())?;
        let gesture = MeshWeightGesture::new(
            self.session.document(),
            MeshWeightSelection {
                attachment: target.mesh.clone(),
                vertices: target.vertices.clone(),
                bone: target.bone,
            },
            pose.clone(),
        )?;
        self.session.begin_transaction();
        self.weight_edit = Some(WeightEdit {
            target,
            pose,
            gesture,
            setup: self.setup_mode,
            animation: self.animation,
            time: self.time,
            control,
            numeric,
            numeric_touched: false,
            percent,
            finish_pending: false,
            error: None,
        });
        Ok(())
    }
    fn preview_weight(&mut self, percent: f64) -> Result<(), String> {
        let edit = self.weight_edit.as_mut().ok_or("No active Weight edit")?;
        edit.percent = Some(percent);
        let result = edit
            .gesture
            .absolute((percent / 100.) as f32)
            .and_then(|candidate| {
                self.session.preview_mesh_bone_weights(&candidate)?;
                edit.gesture = candidate;
                Ok(())
            });
        edit.error = result.as_ref().err().cloned();
        self.status = match &result {
            Ok(()) => format!("Weight: {percent:.2}%"),
            Err(error) => error.clone(),
        };
        result
    }
    pub(super) fn begin_weight_viewport_edit(&mut self) -> Result<(), String> {
        self.begin_weight_edit(Self::weights_viewport_id(), false)
    }
    pub(super) fn preview_weight_viewport_delta(&mut self, pixels: f32) {
        let Some(edit) = self.weight_edit.as_mut() else {
            return;
        };
        let result = edit.gesture.relative(pixels / 350.).and_then(|candidate| {
            self.session.preview_mesh_bone_weights(&candidate)?;
            edit.percent = match candidate.query().value {
                MeshWeightValue::Uniform(weight) => Some(f64::from(weight) * 100.),
                MeshWeightValue::Mixed => None,
            };
            edit.gesture = candidate;
            Ok(())
        });
        edit.error = result.as_ref().err().cloned();
        if let Err(error) = result {
            self.status = error;
        }
    }
    pub(super) fn finish_weight_viewport_edit(&mut self) {
        if let Some(edit) = self.weight_edit.as_mut() {
            edit.finish_pending = true;
        }
    }
    pub(super) fn cancel_weight_edit(&mut self) {
        self.cancel_weights_pointer();
        if let Some(edit) = self.weight_edit.take() {
            // Ignore stale text/lost-focus responses and an ongoing pointer until release.
            self.weight_blocked = Some(edit.control);
            self.session.cancel_transaction();
        }
    }
    fn finish_weight_edit(&mut self) {
        if self.weight_edit.take().is_some() {
            self.session.commit_transaction();
        }
    }
    pub(super) fn reconcile_weight_edit(&mut self) {
        if self.weight_edit.as_ref().is_some_and(|edit| {
            !self.show_weights
                || self.tool != Tool::Weights
                || self.weights.bind.is_some()
                || edit.setup != self.setup_mode
                || edit.animation != self.animation
                || edit.time != self.time
                || !edit.gesture.matches_document(self.session.document())
                || self
                    .direct_target()
                    .map_or(true, |(target, _)| target != edit.target)
        }) {
            self.cancel_weight_edit();
        }
    }
    pub(super) fn weight_control_has_focus(&self, ctx: &egui::Context) -> bool {
        !ctx.text_edit_focused()
            && self.weight_controls.is_some_and(|ids| {
                ctx.memory(|memory| ids.into_iter().any(|id| memory.has_focus(id)))
            })
    }
    pub(super) fn weight_edit_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        let Some(edit) = self.weight_edit.as_ref() else {
            return false;
        };
        if !self.weights_input_available(ctx) {
            self.cancel_weight_edit();
            return false;
        }
        let id = edit.control;
        let owns_focus = ctx.memory(|memory| memory.has_focus(id));
        if owns_focus
            && edit.numeric
            && ctx.input(|input| {
                input.events.iter().any(|event| {
                    matches!(
                        event,
                        egui::Event::Text(_)
                            | egui::Event::Paste(_)
                            | egui::Event::Cut
                            | egui::Event::Ime(
                                egui::ImeEvent::Commit(_)
                                    | egui::ImeEvent::DeleteSurrounding { .. }
                            )
                            | egui::Event::Key {
                                key: egui::Key::Backspace
                                    | egui::Key::Delete
                                    | egui::Key::ArrowUp
                                    | egui::Key::ArrowDown,
                                pressed: true,
                                ..
                            }
                    ) || matches!(event, egui::Event::Key {
                        key: egui::Key::Z | egui::Key::Y | egui::Key::X,
                        pressed: true, modifiers, ..
                    } if modifiers.command)
                })
            })
        {
            self.weight_edit.as_mut().unwrap().numeric_touched = true;
        }
        if owns_focus
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::S))
        {
            self.cancel_weight_edit();
            ctx.memory_mut(|memory| memory.surrender_focus(id));
            self.save_project();
            return true;
        }
        if (!ctx.text_edit_focused() || owns_focus)
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.cancel_weight_edit();
            ctx.memory_mut(|memory| {
                memory.surrender_focus(id);
                memory.request_focus(Self::weights_id());
            });
            self.status = "Weight edit canceled".into();
            return true;
        }
        false
    }
    fn weight_response(
        &mut self,
        response: &egui::Response,
        percent: f64,
        numeric: bool,
        parsed: Option<f64>,
    ) {
        if self.weight_blocked.is_some() || !self.weights_input_available(&response.ctx) {
            return;
        }
        let starts =
            response.gained_focus() && numeric || response.drag_started() || response.changed();
        if starts
            && self
                .weight_edit
                .as_ref()
                .is_none_or(|edit| edit.control != response.id)
        {
            // Moving from the numeric entry to the slider commits the previous explicit edit.
            self.finish_weight_edit();
            if let Err(error) = self.begin_weight_edit(response.id, numeric && response.has_focus())
            {
                self.status = error;
                return;
            }
        }
        if self
            .weight_edit
            .as_ref()
            .is_some_and(|edit| edit.control == response.id)
        {
            // DragValue reparses its rounded display on blur. Focus alone is not an edit.
            let explicit = self
                .weight_edit
                .as_ref()
                .is_some_and(|edit| !edit.numeric || edit.numeric_touched);
            let value_changed = self
                .weight_edit
                .as_ref()
                .is_some_and(|edit| edit.percent != Some(percent));
            let entered_mixed_zero = numeric
                && parsed == Some(percent)
                && self
                    .weight_edit
                    .as_ref()
                    .is_some_and(|edit| edit.percent.is_none());
            if (response.changed() || entered_mixed_zero) && explicit && value_changed {
                let _ = self.preview_weight(percent);
            }
            let editing_text = numeric && response.has_focus();
            if response.lost_focus()
                || (!editing_text && !response.ctx.input(|input| input.pointer.any_down()))
            {
                // Defer until all click actions have run. A Save/context button can
                // lose numeric focus on press but only perform its action on release.
                self.weight_edit.as_mut().unwrap().finish_pending = true;
            }
        }
    }
    pub(super) fn weight_controls(&mut self, ui: &mut egui::Ui) {
        self.reconcile_weight_edit();
        if !self.weights_input_available(ui.ctx()) {
            self.cancel_weight_edit();
        }
        if let Some(control) = self.weight_blocked {
            ui.memory_mut(|memory| memory.surrender_focus(control));
        }
        let direct = ui
            .horizontal(|ui| {
                ui.label("Mode:");
                ui.label("Direct")
            })
            .inner;
        self.record_target("weightsDirect", direct.rect);
        let target = self.direct_target();
        let enabled = target.is_ok()
            && self.weights.bind.is_none()
            && self.weights.pointer.is_none()
            && self.weights.vertex_press.is_none()
            && self.weights_input_available(ui.ctx());
        let shown = self.weight_edit.as_ref().map_or_else(
            || target.as_ref().ok().and_then(|(_, percent)| *percent),
            |edit| edit.percent,
        );
        let mut percent = shown.unwrap_or(0.);
        let mixed = target.is_ok() && shown.is_none();
        // A context change gets new widget IDs, so old numeric text cannot commit to a different target.
        let mesh_id = self
            .weights
            .mesh
            .as_ref()
            .map(|mesh| (&mesh.skin, &mesh.slot, &mesh.placeholder));
        let control_scope = ui.make_persistent_id((
            "direct-weight",
            mesh_id,
            &self.weights.vertices,
            self.weights.bone,
        ));
        let row = ui
            .scope_builder(egui::UiBuilder::new().id(control_scope), |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    ui.horizontal(|ui| {
                        // Keep the numeric control accessible at the vertical
                        // pane's minimum width instead of growing its parent.
                        ui.spacing_mut().slider_width = (ui.available_width() - 130.).max(32.);
                        ui.label("Weight");
                        let parsed = std::cell::Cell::new(None);
                        let formatted = std::cell::RefCell::new(None);
                        let value = ui
                            .scope_builder(
                                egui::UiBuilder::new().id(control_scope.with("numeric")),
                                |ui| {
                                    ui.add_sized(
                                        [70., 22.],
                                        egui::DragValue::new(&mut percent)
                                            .range(0.0..=100.0)
                                            .clamp_existing_to_range(false)
                                            .speed(0.2)
                                            .fixed_decimals(2)
                                            .custom_formatter(|value, _| {
                                                let text = if mixed {
                                                    String::new()
                                                } else {
                                                    format!("{value:.2}")
                                                };
                                                *formatted.borrow_mut() = Some(text.clone());
                                                text
                                            })
                                            .custom_parser(|text| {
                                                let value = text
                                                    .trim()
                                                    .parse::<f64>()
                                                    .ok()
                                                    .filter(|value| value.is_finite());
                                                parsed.set(value);
                                                value
                                            })
                                            .suffix(if mixed { "" } else { "%" }),
                                    )
                                },
                            )
                            .inner;
                        self.weights.formatted_percent = formatted.into_inner();
                        self.record_target("weightsWeightValue", value.rect);
                        self.weight_response(&value, percent, true, parsed.get());
                        let slider = ui
                            .scope_builder(
                                egui::UiBuilder::new().id(control_scope.with("slider")),
                                |ui| {
                                    ui.add(
                                        egui::Slider::new(&mut percent, 0.0..=100.0)
                                            .show_value(false)
                                            .clamping(egui::SliderClamping::Edits),
                                    )
                                },
                            )
                            .inner;
                        self.weight_controls = Some([slider.id, value.id]);
                        self.record_target("weightsWeightSlider", slider.rect);
                        self.weight_response(&slider, percent, false, None);
                    })
                    .response
                })
                .inner
            })
            .inner;
        self.record_target("weightsWeight", row.rect);
        if !ui.input(|input| input.pointer.any_down()) {
            self.weight_blocked = None;
        }
        if let Err(reason) = target {
            ui.label(RichText::new(reason).small().color(MUTED));
        }
        if let Some(error) = self
            .weight_edit
            .as_ref()
            .and_then(|edit| edit.error.as_ref())
        {
            ui.label(RichText::new(error).small().color(ACCENT));
        }
    }
    pub(super) fn finish_idle_transactions(&mut self, ctx: &egui::Context) {
        self.settle_slot_color_input(ctx);
        self.settle_graph_input(ctx);
        self.settle_draw_order_input(ctx);
        // A menu/modal can open during this frame, after keyboard routing queued an action.
        if !ctx.input(|input| input.focused)
            || self.error.is_some()
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
            || egui::Popup::is_any_open(ctx)
        {
            self.cancel_gesture();
        }
        self.settle_ik_bend();
        self.settle_pose_key();
        self.settle_ik_edit(ctx);
        self.settle_transform_edit(ctx);
        self.settle_constraint_input(ctx);
        if !ctx.input(|input| input.pointer.any_down())
            && self
                .weight_edit
                .as_ref()
                .is_some_and(|edit| edit.finish_pending)
        {
            self.finish_weight_edit();
        }
        if self.weight_edit.is_none()
            && !self.constraint_edit_active()
            && !self.graph_edit_active()
            && !self.draw_order_edit_active()
            && !self.slot_color_edit_active()
            && self.ik_edit.is_none()
            && self.drag.is_none()
            && !ctx.input(|input| input.pointer.any_down())
        {
            self.session.commit_transaction();
        }
    }
    pub(super) fn weight_direct_diagnostic(&self) -> serde_json::Value {
        let current = self.direct_target();
        let mixed = current.as_ref().is_ok_and(|(_, percent)| percent.is_none());
        serde_json::json!({"mode":"direct","mesh":self.weights.mesh,"vertices":self.weights.vertices,"vertex":(self.weights.vertices.len()==1).then(||self.weights.vertices[0]),"bone":self.weights.bone,
            "percent":current.as_ref().ok().map(|(_,percent)| percent),"unavailable":current.err(),
            "mixed":mixed,"formattedPercent":self.weights.formatted_percent,
            "blocked":self.weight_blocked.is_some(),"active":self.weight_edit.as_ref().map(|edit| serde_json::json!({"mesh":edit.target.mesh,"vertices":edit.target.vertices,"bone":edit.target.bone,
                "setup":edit.setup,"animation":edit.animation,"time":edit.time,"numeric":edit.numeric,"percent":edit.percent,"error":edit.error,"pose":edit.pose}))})
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{editor_context, mesh_editor};
    use super::*;

    fn ready() -> (Editor, AttachmentRef) {
        let (mut editor, key) = mesh_editor(true);
        editor.set_tool(Tool::Weights);
        editor.weights.vertices = vec![0];
        editor.weights.bone = Some(8);
        (editor, key)
    }
    fn entries(editor: &Editor, key: &AttachmentRef, vertex: usize) -> Vec<BoneInfluence> {
        let Some(Attachment::Mesh(mesh)) = editor.session.document().attachment(key) else {
            panic!()
        };
        let MeshVertices::Weighted { influences } = &mesh.vertices else {
            panic!()
        };
        influences[vertex].clone()
    }
    fn chord(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            })
            .collect()
    }
    fn frame(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>) {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(700., 700.))),
                focused: true,
                events,
                ..Default::default()
            },
            |ui| {
                editor.reconcile_selection();
                editor.shortcuts(ui.ctx());
                egui::Panel::top("test-toolbar").show(ui, |ui| editor.toolbar(ui));
                egui::CentralPanel::default().show(ui, |ui| editor.weights_panel(ui));
                editor.finish_idle_transactions(ui.ctx());
            },
        );
        output.textures_delta.clear();
    }
    fn click(editor: &mut Editor, ctx: &egui::Context, target: &str) {
        let rect = editor.capture_targets[target];
        let pos = pos2((rect[0] + rect[2]) / 2., (rect[1] + rect[3]) / 2.);
        frame(
            editor,
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        frame(
            editor,
            ctx,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    fn pane_frame(editor: &mut Editor, ctx: &egui::Context, size: Vec2, events: Vec<egui::Event>) {
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
    fn pane_rect(editor: &Editor, name: &str) -> egui::Rect {
        let [left, top, right, bottom] = *editor
            .capture_targets
            .get(name)
            .unwrap_or_else(|| panic!("Missing capture target {name}"));
        egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom))
    }
    #[test]
    fn weights_pane_has_top_controls_scrollable_bones_and_visible_footer_at_minimum_window() {
        for both in [false, true] {
            let (mut editor, _) = ready();
            if both {
                editor.set_setup_mode(false);
                editor.open_curves_view();
                // The retained Weights mesh remains explicit while a bone with
                // scalar keys supplies the simultaneously visible Graph.
                editor.select(TreeItem::Bone(8));
                editor.open_graph_view();
            }
            let ctx = editor_context();
            for _ in 0..4 {
                pane_frame(&mut editor, &ctx, vec2(1120., 740.), vec![]);
            }
            let viewport = pane_rect(&editor, "viewport");
            let weights = pane_rect(&editor, "weightsPanel");
            let tree = pane_rect(&editor, "tree");
            assert!(
                viewport.width() >= 300. && viewport.height() >= 100.,
                "{viewport:?}"
            );
            assert!(viewport.right() <= weights.left());
            assert!(weights.right() <= tree.left());
            assert!(!editor.capture_targets.contains_key("viewTab:Weights"));
            for name in [
                "closeView:Weights",
                "weightsDirect",
                "weightsWeightSlider",
                "weightsWeightValue",
                "weightsBonesClip",
                "weightsFooter",
                "weightsBind",
                "weightsRemoveAll",
            ] {
                let rect = pane_rect(&editor, name);
                assert!(
                    rect.is_finite() && weights.contains_rect(rect),
                    "{name}: {rect:?} outside {weights:?}"
                );
            }
            let bones = pane_rect(&editor, "weightsBonesClip");
            assert!(pane_rect(&editor, "weightsWeightValue").bottom() < bones.top());
            assert!(bones.bottom() <= pane_rect(&editor, "weightsFooter").top());
            if both {
                let curves = pane_rect(&editor, "curvesView");
                let bottom = pane_rect(&editor, "animationViews");
                assert!(weights.right() <= curves.left() && curves.right() <= tree.left());
                assert!(weights.bottom() <= bottom.top() && curves.bottom() <= bottom.top());
                assert!(pane_rect(&editor, "graphPlot").width() > 1000.);
            }
        }
    }
    #[test]
    fn weights_pane_preview_keeps_both_vertical_panes_stable_and_escape_restores_document() {
        for size in [vec2(1120., 740.), vec2(1500., 960.)] {
            let (mut editor, _) = ready();
            editor.set_setup_mode(false);
            editor.open_curves_view();
            let ctx = editor_context();
            for _ in 0..4 {
                pane_frame(&mut editor, &ctx, size, vec![]);
            }
            let before = editor.session.document().clone();
            let bounds =
                ["weightsPanel", "curvesView", "viewport"].map(|name| pane_rect(&editor, name));
            let slider = pane_rect(&editor, "weightsWeightSlider");
            let press = pos2(slider.left() + slider.width() * 0.2, slider.center().y);
            pane_frame(
                &mut editor,
                &ctx,
                size,
                vec![
                    egui::Event::PointerMoved(press),
                    egui::Event::PointerButton {
                        pos: press,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            pane_frame(
                &mut editor,
                &ctx,
                size,
                vec![egui::Event::PointerMoved(pos2(slider.center().x, press.y))],
            );
            assert!(editor.weight_edit.is_some());
            assert_ne!(editor.session.document(), &before);
            for (name, expected) in ["weightsPanel", "curvesView", "viewport"]
                .into_iter()
                .zip(bounds)
            {
                assert_eq!(pane_rect(&editor, name), expected, "{name}");
            }
            pane_frame(
                &mut editor,
                &ctx,
                size,
                chord(egui::Key::Escape, egui::Modifiers::NONE),
            );
            assert!(editor.weight_edit.is_none());
            assert_eq!(editor.session.document(), &before);
            pane_frame(
                &mut editor,
                &ctx,
                size,
                vec![egui::Event::PointerButton {
                    pos: press,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            assert_eq!(editor.session.document(), &before);
        }
    }
    #[test]
    fn weights_pane_narrow_toolbar_scroll_reaches_controls_without_exposing_hidden_targets() {
        let (mut editor, _) = ready();
        editor.set_setup_mode(false);
        editor.open_curves_view();
        let ctx = editor_context();
        let size = vec2(1120., 740.);
        for _ in 0..4 {
            pane_frame(&mut editor, &ctx, size, vec![]);
        }
        let mut reached = BTreeSet::new();
        // Include intermediate offsets: the axes column can sit between the
        // two extremes when content is wider than twice the visible toolbar.
        for delta in [0., -100., -100., -100., -100., -10000., 10000.] {
            let clip = pane_rect(&editor, "mainToolsClip");
            let point = pos2(clip.left() + 10., clip.center().y);
            pane_frame(
                &mut editor,
                &ctx,
                size,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: vec2(delta, 0.),
                        modifiers: egui::Modifiers::NONE,
                        phase: egui::TouchPhase::Move,
                    },
                ],
            );
            for _ in 0..4 {
                pane_frame(&mut editor, &ctx, size, vec![]);
            }
            let clip = pane_rect(&editor, "mainToolsClip");
            for name in Property::ALL
                .into_iter()
                .map(|property| format!("value:{property:?}"))
                .chain([
                    "weightsTool".into(),
                    "axes:World".into(),
                    "autoKey".into(),
                    "keyEdited".into(),
                ])
            {
                if editor.capture_targets.contains_key(&name) {
                    let rect = pane_rect(&editor, &name);
                    assert!(rect.is_positive() && rect.is_finite() && clip.contains_rect(rect));
                    assert!(
                        rect.height() < clip.height() / 2.,
                        "wrapped toolbar control {name}: {rect:?}"
                    );
                    reached.insert(name);
                }
            }
        }
        for name in Property::ALL
            .into_iter()
            .map(|property| format!("value:{property:?}"))
            .chain([
                "weightsTool".into(),
                "axes:World".into(),
                "autoKey".into(),
                "keyEdited".into(),
            ])
        {
            assert!(
                reached.contains(&name),
                "toolbar control was unreachable: {name}; reached {reached:?}"
            );
        }
    }
    #[test]
    fn initial_bind_add_zero_then_direct_changes_only_the_explicit_vertex_in_one_undo_step() {
        let (mut editor, key) = mesh_editor(false);
        editor.set_tool(Tool::Weights);
        editor.begin_weights_bind();
        editor.select(TreeItem::Bone(0));
        assert!(editor.error.is_none());
        let initial = editor.session.document().clone();
        editor.weights.vertices = vec![0];
        editor.begin_weights_bind();
        editor.select(TreeItem::Bone(8));
        assert!(editor.error.is_none());
        let added = editor.session.document().clone();
        assert_eq!(
            entries(&editor, &key, 0)
                .iter()
                .map(|i| (i.bone, i.weight))
                .collect::<Vec<_>>(),
            vec![(0, 1.), (8, 0.)]
        );
        editor
            .begin_weight_edit(egui::Id::new("test"), false)
            .unwrap();
        editor.preview_weight(20.).unwrap();
        editor.preview_weight(30.).unwrap();
        editor.finish_weight_edit();
        assert_eq!(
            entries(&editor, &key, 0)
                .iter()
                .map(|i| (i.bone, i.weight))
                .collect::<Vec<_>>(),
            vec![(0, 0.7), (8, 0.3)]
        );
        assert_eq!(
            entries(&editor, &key, 1)
                .iter()
                .map(|i| i.weight)
                .collect::<Vec<_>>(),
            vec![1., 0.]
        );
        let edited = editor.session.document().clone();
        editor.undo();
        assert_eq!(editor.session.document(), &added);
        editor.undo();
        assert_eq!(editor.session.document(), &initial);
        editor.redo();
        editor.redo();
        assert_eq!(editor.session.document(), &edited);
        editor.begin_weights_bind();
        editor.select(TreeItem::Bone(8));
        assert!(editor.error.as_ref().unwrap().contains("already bound"));
        assert!(editor.weights.bind.is_none());
        assert_eq!(editor.session.document(), &edited);
    }
    #[test]
    fn animate_preview_freezes_pose_and_time_and_undo_restores_exact_pending_locals() {
        let (mut editor, key) = ready();
        editor.set_setup_mode(false);
        editor.set_time(0.37);
        let mut transform = editor.session.local_pose(Some(0), editor.time)[8];
        transform.rotation += 23.;
        transform.x += 11.;
        editor
            .session
            .edit_pose(
                8,
                editor.weights_edit_target(),
                transform,
                &[Property::Rotation, Property::X],
                false,
            )
            .unwrap();
        let pose = editor.session.pose_edits().clone();
        let locals = editor.session.local_pose(Some(0), editor.time);
        let original = editor.session.document().clone();
        editor
            .begin_weight_edit(egui::Id::new("numeric"), true)
            .unwrap();
        assert_eq!(editor.weight_edit.as_ref().unwrap().pose.locals, locals);
        assert_eq!(editor.weight_edit.as_ref().unwrap().time, 0.37);
        editor.preview_weight(30.).unwrap();
        editor.preview_weight(40.).unwrap();
        editor.finish_weight_edit();
        assert_eq!(editor.session.pose_edits(), &pose);
        assert_eq!(editor.session.local_pose(Some(0), editor.time), locals);
        assert_eq!(editor.time, 0.37);
        assert_eq!(editor.session.document().animations, original.animations);
        assert_eq!(entries(&editor, &key, 0)[1].weight, 0.4);
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pose);
        assert_eq!(editor.session.local_pose(Some(0), editor.time), locals);
    }
    #[test]
    fn failed_noop_and_cancelled_previews_preserve_redo_and_do_not_redirect_context() {
        let (mut editor, key) = ready();
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "redo".into(),
            })
            .unwrap();
        editor.undo();
        let original = editor.session.document().clone();
        let id = egui::Id::new("control");
        editor.begin_weight_edit(id, false).unwrap();
        editor.preview_weight(75.).unwrap();
        editor.finish_weight_edit();
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
        editor.begin_weight_edit(id, false).unwrap();
        editor.preview_weight(30.).unwrap();
        let preview = editor.session.document().clone();
        assert!(editor.preview_weight(f64::NAN).is_err());
        assert_eq!(editor.session.document(), &preview);
        editor.weights.bone = Some(0);
        editor.reconcile_selection();
        assert!(editor.weight_edit.is_none());
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
        editor.weight_blocked = None;
        editor.begin_weight_edit(id, false).unwrap();
        editor.preview_weight(50.).unwrap();
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
        editor.redo();
        assert_eq!(editor.session.document().animations[0].name, "redo");
        assert_eq!(entries(&editor, &key, 0)[1].weight, 0.75);
    }
    #[test]
    fn numeric_frames_preview_across_pointer_idle_enter_commits_and_escape_restores_baseline() {
        let (mut editor, key) = ready();
        let ctx = editor_context();
        frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "weightsWeightValue");
        frame(&mut editor, &ctx, vec![egui::Event::Text("30".into())]);
        assert!(ctx.text_edit_focused());
        assert_eq!(entries(&editor, &key, 0)[1].weight, 0.3);
        assert!(editor.weight_edit.is_some());
        frame(&mut editor, &ctx, vec![]);
        frame(&mut editor, &ctx, vec![egui::Event::Text("5".into())]);
        assert_eq!(entries(&editor, &key, 0)[1].weight, 1.); // 305 explicitly clamps to100.
        frame(
            &mut editor,
            &ctx,
            chord(egui::Key::Escape, egui::Modifiers::NONE),
        );
        assert_eq!(entries(&editor, &key, 0)[1].weight, 0.75);
        assert!(editor.weight_edit.is_none());
        assert!(!editor.session.can_undo());
        click(&mut editor, &ctx, "weightsWeightValue");
        frame(&mut editor, &ctx, vec![egui::Event::Text("30".into())]);
        frame(
            &mut editor,
            &ctx,
            chord(egui::Key::Enter, egui::Modifiers::NONE),
        );
        assert!(editor.weight_edit.is_none());
        assert_eq!(entries(&editor, &key, 0)[1].weight, 0.3);
        editor.undo();
        assert_eq!(entries(&editor, &key, 0)[1].weight, 0.75);
        assert!(!editor.session.can_undo());
        editor.redo();
        assert_eq!(entries(&editor, &key, 0)[1].weight, 0.3);
    }
    fn slider_down(editor: &mut Editor, ctx: &egui::Context) -> (Pos2, Pos2) {
        frame(editor, ctx, vec![]);
        let rect = editor.capture_targets["weightsWeightSlider"];
        let start = pos2(
            rect[0] + (rect[2] - rect[0]) * 0.3,
            (rect[1] + rect[3]) / 2.,
        );
        let end = pos2(rect[0] + (rect[2] - rect[0]) * 0.8, start.y);
        frame(
            editor,
            ctx,
            vec![
                egui::Event::PointerMoved(start),
                egui::Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert!(editor.weight_edit.is_some());
        (start, end)
    }
    fn release(editor: &mut Editor, ctx: &egui::Context, end: Pos2) {
        frame(editor, ctx, vec![egui::Event::PointerMoved(end)]);
        frame(
            editor,
            ctx,
            vec![egui::Event::PointerButton {
                pos: end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
    }
    fn save_folder(editor: &mut Editor) -> PathBuf {
        let folder = std::env::temp_dir().join(format!(
            "peachy-direct-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&folder).unwrap();
        editor.project_path = Some(folder.join("rig.peachy"));
        editor.asset_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
        folder
    }
    #[test]
    fn held_pointer_escape_history_and_save_cannot_restart_a_cancelled_weight_gesture() {
        for key in [egui::Key::Escape, egui::Key::Z, egui::Key::S] {
            let (mut editor, attachment) = ready();
            let ctx = editor_context();
            let folder = save_folder(&mut editor);
            let original = editor.session.document().clone();
            let (_, end) = slider_down(&mut editor, &ctx);
            assert_ne!(editor.session.document(), &original);
            let modifiers = if key == egui::Key::Escape {
                egui::Modifiers::NONE
            } else {
                egui::Modifiers::COMMAND
            };
            frame(&mut editor, &ctx, chord(key, modifiers));
            assert_eq!(editor.session.document(), &original, "key {key:?}");
            assert!(editor.weight_edit.is_none());
            release(&mut editor, &ctx, end);
            assert_eq!(editor.session.document(), &original, "release {key:?}");
            assert!(!editor.session.can_undo());
            assert_eq!(entries(&editor, &attachment, 0)[1].weight, 0.75);
            if key == egui::Key::S {
                let loaded = files::load(&folder.join("rig.peachy")).unwrap();
                assert_eq!(loaded.animations, original.animations);
                assert_eq!(
                    loaded.attachment(&attachment),
                    original.attachment(&attachment)
                );
            }
            std::fs::remove_dir_all(folder).unwrap();
        }
    }
    #[test]
    fn numeric_save_cancels_before_write_and_stale_enter_or_lost_focus_cannot_reapply() {
        let (mut editor, key) = ready();
        let folder = save_folder(&mut editor);
        let ctx = editor_context();
        frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "weightsWeightValue");
        frame(&mut editor, &ctx, vec![egui::Event::Text("30".into())]);
        assert_eq!(entries(&editor, &key, 0)[1].weight, 0.3);
        frame(
            &mut editor,
            &ctx,
            chord(egui::Key::S, egui::Modifiers::COMMAND),
        );
        frame(
            &mut editor,
            &ctx,
            chord(egui::Key::Enter, egui::Modifiers::NONE),
        );
        assert_eq!(entries(&editor, &key, 0)[1].weight, 0.75);
        assert!(!editor.session.can_undo());
        let saved = files::load(&folder.join("rig.peachy")).unwrap();
        assert_eq!(
            saved.attachment(&key),
            editor.session.document().attachment(&key)
        );
        std::fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn inspector_pose_edit_during_direct_text_cancels_preview_before_new_transaction() {
        let (mut editor, key) = ready();
        editor.set_setup_mode(false);
        editor.select(TreeItem::Bone(8));
        let ctx = editor_context();
        frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "weightsWeightValue");
        frame(&mut editor, &ctx, vec![egui::Event::Text("30".into())]);
        let original = editor.weight_edit.as_ref().unwrap().pose.locals[8];
        let mut local = original;
        local.rotation += 17.;
        // Use the real transform-edit boundary while the previous Direct text is still focused.
        editor.edit_transform(local, &[Property::Rotation]);
        frame(
            &mut editor,
            &ctx,
            chord(egui::Key::Enter, egui::Modifiers::NONE),
        );
        assert_eq!(entries(&editor, &key, 0)[1].weight, 0.75);
        assert_eq!(
            editor.session.local_pose(Some(0), editor.time)[8].rotation,
            local.rotation
        );
        assert!(!editor.session.pose_edits().is_empty());
        editor.undo();
        assert_eq!(editor.session.local_pose(Some(0), editor.time)[8], original);
        assert!(editor.session.pose_edits().is_empty());
        assert!(!editor.session.can_undo());
    }
    #[test]
    fn numeric_text_tool_priority_tab_commit_and_window_focus_loss_are_explicit() {
        let (mut editor, key) = ready();
        editor.set_tool(Tool::Rotate);
        let ctx = editor_context();
        frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "weightsWeightValue");
        frame(
            &mut editor,
            &ctx,
            chord(egui::Key::C, egui::Modifiers::NONE),
        );
        assert!(
            editor.tool == Tool::Weights
                && editor.last_tool == Tool::Rotate
                && ctx.text_edit_focused()
        );
        frame(&mut editor, &ctx, vec![egui::Event::Text("30".into())]);
        frame(
            &mut editor,
            &ctx,
            chord(egui::Key::Tab, egui::Modifiers::NONE),
        );
        assert!(editor.weight_edit.is_none());
        assert_eq!(entries(&editor, &key, 0)[1].weight, 0.3);
        editor.undo();
        click(&mut editor, &ctx, "weightsWeightValue");
        frame(&mut editor, &ctx, vec![egui::Event::Text("40".into())]);
        let mut output = ctx.run_ui(
            egui::RawInput {
                focused: false,
                events: vec![egui::Event::WindowFocused(false)],
                ..Default::default()
            },
            |ui| {
                editor.shortcuts(ui.ctx());
                editor.weights_panel(ui);
                editor.finish_idle_transactions(ui.ctx());
            },
        );
        output.textures_delta.clear();
        assert!(editor.weight_edit.is_none());
        assert_eq!(entries(&editor, &key, 0)[1].weight, 0.75);
        assert!(editor.session.can_redo());
    }
    #[test]
    fn real_toolbar_save_click_cancels_numeric_preview_on_release_before_persisting() {
        let (mut editor, key) = ready();
        let folder = save_folder(&mut editor);
        let ctx = editor_context();
        frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "weightsWeightValue");
        frame(&mut editor, &ctx, vec![egui::Event::Text("30".into())]);
        let rect = editor.capture_targets["toolbarSave"];
        let pos = pos2((rect[0] + rect[2]) / 2., (rect[1] + rect[3]) / 2.);
        frame(
            &mut editor,
            &ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert_eq!(entries(&editor, &key, 0)[1].weight, 0.3);
        assert!(editor.weight_edit.is_some());
        assert!(!folder.join("rig.peachy").exists());
        frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(editor.weight_edit.is_none());
        assert_eq!(entries(&editor, &key, 0)[1].weight, 0.75);
        assert!(!editor.session.can_undo());
        let saved = files::load(&folder.join("rig.peachy")).unwrap();
        assert_eq!(
            saved.attachment(&key),
            editor.session.document().attachment(&key)
        );
        std::fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn focusing_and_blurring_rounded_nonunit_numeric_weight_is_not_an_authoring_edit() {
        let (mut editor, key) = ready();
        let mut document = editor.session.document().clone();
        let Attachment::Mesh(mesh) = document.attachment_mut(&key).unwrap() else {
            panic!()
        };
        let MeshVertices::Weighted { influences } = &mut mesh.vertices else {
            panic!()
        };
        influences[0][0].weight = 0.234567;
        influences[0][1].weight = 0.33333334;
        editor.session = Session::new(document).unwrap();
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "redo-preserved".into(),
            })
            .unwrap();
        editor.undo();
        let original = editor.session.document().clone();
        let ctx = editor_context();
        frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "weightsWeightValue");
        frame(
            &mut editor,
            &ctx,
            chord(egui::Key::Tab, egui::Modifiers::NONE),
        );
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
        assert!(!editor.session.can_undo());
        click(&mut editor, &ctx, "weightsWeightValue");
        frame(&mut editor, &ctx, vec![egui::Event::Text("33.33".into())]);
        frame(
            &mut editor,
            &ctx,
            chord(egui::Key::Enter, egui::Modifiers::NONE),
        );
        assert_ne!(editor.session.document(), &original); // Explicit input intentionally normalizes the touched vertex.
        assert!(!editor.session.can_redo());
        editor.undo();
        assert_eq!(editor.session.document(), &original);
    }
    #[test]
    fn invalid_numeric_text_preserves_raw_nonunit_weights_and_redo() {
        let (mut editor, key) = ready();
        let mut document = editor.session.document().clone();
        let Attachment::Mesh(mesh) = document.attachment_mut(&key).unwrap() else {
            panic!()
        };
        let MeshVertices::Weighted { influences } = &mut mesh.vertices else {
            panic!()
        };
        influences[0][0].weight = 0.234567;
        influences[0][1].weight = 0.33333334;
        editor.session = Session::new(document).unwrap();
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "redo".into(),
            })
            .unwrap();
        editor.undo();
        let original = editor.session.document().clone();
        let ctx = editor_context();
        frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "weightsWeightValue");
        frame(&mut editor, &ctx, vec![egui::Event::Text("junk".into())]);
        assert_eq!(editor.session.document(), &original);
        frame(
            &mut editor,
            &ctx,
            chord(egui::Key::Enter, egui::Modifiers::NONE),
        );
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
        assert!(editor.session.can_redo());
    }
}
