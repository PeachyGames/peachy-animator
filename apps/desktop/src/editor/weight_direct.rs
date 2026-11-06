//! Direct control routing only. Session owns redistribution and fixed-base history.
use super::*;
use peachy_engine::MeshBindPose;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct WeightTarget {
    mesh: AttachmentRef,
    vertex: usize,
    bone: usize,
}

pub(super) struct WeightEdit {
    target: WeightTarget,
    pose: MeshBindPose,
    setup: bool,
    animation: usize,
    time: f32,
    control: egui::Id,
    numeric: bool,
    numeric_touched: bool,
    percent: f64,
    finish_pending: bool,
    error: Option<String>,
}

impl Editor {
    fn direct_target(&self) -> Result<(WeightTarget, f64), String> {
        let mesh = self.weights.mesh.clone().ok_or("Select an active mesh")?;
        if !self.weights_mesh_active(&mesh) {
            return Err("Select an active mesh".into());
        }
        let vertex = self.weights.vertex.ok_or("Select one vertex")?;
        let bone = self.weights.bone.ok_or("Select one bound bone")?;
        let Some(Attachment::Mesh(data)) = self.session.document().attachment(&mesh) else {
            return Err("Select a mesh".into());
        };
        let MeshVertices::Weighted { influences } = &data.vertices else {
            return Err("Bind the mesh before adjusting weights".into());
        };
        let entries = influences.get(vertex).ok_or("Select a valid vertex")?;
        let mut seen = BTreeSet::new();
        if entries.iter().any(|entry| !seen.insert(entry.bone)) {
            return Err("Repeated bone bindings on this vertex are ambiguous".into());
        }
        let entry = entries.iter().find(|entry| entry.bone == bone).ok_or(
            "Selected bone has no binding on this vertex; sparse bindings cannot be adjusted yet",
        )?;
        Ok((
            WeightTarget { mesh, vertex, bone },
            f64::from(entry.weight) * 100.,
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
        self.session.begin_transaction();
        self.weight_edit = Some(WeightEdit {
            target,
            pose,
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
        edit.percent = percent;
        let result = self
            .session
            .preview_transaction(Command::SetMeshBoneWeight {
                attachment: edit.target.mesh.clone(),
                vertex: edit.target.vertex,
                bone: edit.target.bone,
                weight: (percent / 100.) as f32,
                pose: edit.pose.clone(),
            });
        edit.error = result.as_ref().err().cloned();
        self.status = match &result {
            Ok(()) => format!("Weight: {percent:.2}%"),
            Err(error) => error.clone(),
        };
        result
    }
    pub(super) fn cancel_weight_edit(&mut self) {
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
        if !ctx.input(|input| input.focused)
            || self.error.is_some()
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
            || egui::Popup::is_any_open(ctx)
        {
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
    fn weight_response(&mut self, response: &egui::Response, percent: f64, numeric: bool) {
        if self.weight_blocked.is_some() {
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
                .is_some_and(|edit| edit.percent != percent);
            if response.changed() && explicit && value_changed {
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
        if egui::Popup::is_any_open(ui.ctx()) {
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
            && ui.input(|input| input.focused)
            && !egui::Popup::is_any_open(ui.ctx());
        let mut percent = self.weight_edit.as_ref().map_or_else(
            || target.as_ref().map_or(0., |(_, percent)| *percent),
            |edit| edit.percent,
        );
        // A context change gets new widget IDs, so old numeric text cannot commit to a different target.
        let mesh_id = self
            .weights
            .mesh
            .as_ref()
            .map(|mesh| (&mesh.skin, &mesh.slot, &mesh.placeholder));
        let control_scope = ui.make_persistent_id((
            "direct-weight",
            mesh_id,
            self.weights.vertex,
            self.weights.bone,
        ));
        let row = ui
            .push_id(control_scope, |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Weight");
                        let slider = ui.add(
                            egui::Slider::new(&mut percent, 0.0..=100.0)
                                .show_value(false)
                                .clamping(egui::SliderClamping::Edits),
                        );
                        self.record_target("weightsWeightSlider", slider.rect);
                        self.weight_response(&slider, percent, false);
                        let value = ui.add(
                            egui::DragValue::new(&mut percent)
                                .range(0.0..=100.0)
                                .clamp_existing_to_range(false)
                                .speed(0.2)
                                .fixed_decimals(2)
                                .suffix("%"),
                        );
                        self.weight_controls = Some([slider.id, value.id]);
                        self.record_target("weightsWeightValue", value.rect);
                        self.weight_response(&value, percent, true);
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
        self.settle_transform_edit(ctx);
        if !ctx.input(|input| input.pointer.any_down())
            && self
                .weight_edit
                .as_ref()
                .is_some_and(|edit| edit.finish_pending)
        {
            self.finish_weight_edit();
        }
        if self.weight_edit.is_none()
            && self.drag.is_none()
            && !ctx.input(|input| input.pointer.any_down())
        {
            self.session.commit_transaction();
        }
    }
    pub(super) fn weight_direct_diagnostic(&self) -> serde_json::Value {
        let current = self.direct_target();
        serde_json::json!({"mode":"direct","mesh":self.weights.mesh,"vertex":self.weights.vertex,"bone":self.weights.bone,
            "percent":current.as_ref().ok().map(|(_,percent)| percent),"unavailable":current.err(),
            "blocked":self.weight_blocked.is_some(),"active":self.weight_edit.as_ref().map(|edit| serde_json::json!({"mesh":edit.target.mesh,"vertex":edit.target.vertex,"bone":edit.target.bone,
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
        editor.weights.vertex = Some(0);
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
    #[test]
    fn initial_bind_add_zero_then_direct_changes_only_the_explicit_vertex_in_one_undo_step() {
        let (mut editor, key) = mesh_editor(false);
        editor.set_tool(Tool::Weights);
        editor.begin_weights_bind();
        editor.select(TreeItem::Bone(0));
        assert!(editor.error.is_none());
        let initial = editor.session.document().clone();
        editor.weights.vertex = Some(0);
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
