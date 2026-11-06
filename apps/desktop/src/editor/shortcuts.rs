//! Spine's default bindings for the currently implemented operations.
use super::*;

impl Editor {
    pub(super) fn cancel_gesture(&mut self) {
        self.cancel_timeline_interaction();
        self.cancel_graph_edit();
        self.cancel_slot_color_edit();
        self.cancel_ik_edit();
        self.ik_target = None;
        self.cancel_weight_edit();
        self.cancel_dopesheet_drag();
        self.cancel_transform_edit();
        self.cancel_mesh_drag();
    }
    pub(super) fn set_tool(&mut self, tool: Tool) {
        if tool == Tool::Weights {
            self.open_weights_view();
            if let Some(key) = self.selected_attachment().cloned() {
                self.capture_weights_mesh(&key);
            }
        }
        if tool == Tool::Mesh
            && !matches!(
                self.selected_attachment()
                    .and_then(|k| self.session.document().attachment(k)),
                Some(Attachment::Mesh(_))
            )
        {
            return;
        }
        if self.tool != tool {
            self.cancel_gesture();
            if tool != Tool::Weights {
                self.weights.bind = None;
            }
            if tool == Tool::Mesh {
                // Inspection is temporary; keep the previous transform pair intact.
                self.mesh_return_tool = Some(self.tool);
            } else {
                let previous = if self.tool == Tool::Mesh {
                    self.mesh_return_tool.take().unwrap_or(self.last_tool)
                } else {
                    self.tool
                };
                if previous != tool {
                    self.last_tool = previous;
                }
            }
            self.tool = tool;
        }
    }
    pub(super) fn return_from_mesh_tool(&mut self) {
        if self.tool == Tool::Mesh {
            self.set_tool(self.mesh_return_tool.unwrap_or(self.last_tool));
        }
    }
    pub(super) fn toggle_transform_tool(&mut self) {
        if self.tool == Tool::Mesh {
            self.return_from_mesh_tool();
        } else {
            self.set_tool(self.last_tool);
        }
    }
    pub(super) fn toggle_views(&mut self) {
        self.cancel_gesture();
        self.weights.bind = None;
        if let Some(views) = self.hidden_views.take() {
            self.show_tree = views.tree;
            self.show_dopesheet = views.dopesheet;
            self.show_timeline = views.timeline;
            self.show_weights = views.weights;
            self.view_tabs.open = views.dock;
        } else {
            self.hidden_views = Some(ViewVisibility {
                tree: self.show_tree,
                dopesheet: self.show_dopesheet,
                timeline: self.show_timeline,
                weights: self.show_weights,
                dock: self.view_tabs.open.clone(),
            });
            self.show_tree = false;
            self.show_dopesheet = false;
            self.show_timeline = false;
            self.show_weights = false;
            self.view_tabs.open.clear();
        }
    }

    pub(super) fn deselect(&mut self) {
        self.cancel_gesture();
        self.selection_history.record(&self.selection, &None);
        self.selection = None;
    }
    pub(super) fn step_frames(&mut self, delta: f32) {
        self.playing = false;
        self.set_time(self.snapped_time() + delta / self.session.document().fps as f32);
    }

    fn nudge(&mut self, delta: [f32; 2], fine: bool) {
        if self.key_drag.is_some() {
            return;
        }
        if self.tool == Tool::Mesh {
            if self.mesh_drag.is_none()
                && let Some(key) = self.selected_attachment().cloned()
                && self
                    .editable_mesh_position(&key, self.mesh_vertex)
                    .is_some()
            {
                match self
                    .mesh_translation(&key, self.mesh_vertex)
                    .and_then(|translation| translation.command(key, delta))
                {
                    Ok(command) => {
                        self.dispatch(command);
                    }
                    Err(error) => self.error = Some(error),
                }
            }
        } else if self.transform_tool_editable(self.tool) {
            self.nudge_transform(delta, fine);
        }
    }

    fn file_history_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.open_project(ctx);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save_project();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::E)) {
            self.export();
        }
        if ctx.input_mut(|i| {
            i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::Z,
            )
        }) {
            self.redo();
        } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
            self.undo();
        } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)) {
            self.redo();
        }
    }

    fn tool_shortcuts(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.focused) {
            return;
        }
        let tools = ctx.input_mut(|input| {
            let mut tools = Vec::new();
            input.events.retain(|event| {
                let egui::Event::Key {
                    key,
                    pressed: true,
                    repeat,
                    modifiers,
                    ..
                } = event
                else {
                    return true;
                };
                if *modifiers != egui::Modifiers::NONE {
                    return true;
                }
                let tool = match key {
                    egui::Key::C => Tool::Rotate,
                    egui::Key::V => Tool::Move,
                    egui::Key::X => Tool::Scale,
                    egui::Key::Z => Tool::Shear,
                    egui::Key::G => Tool::Weights,
                    _ => return true,
                };
                if !repeat {
                    tools.push(tool);
                }
                false
            });
            tools
        });
        for tool in tools {
            // A tool action ends a preview even when that tool is already selected.
            self.cancel_gesture();
            self.set_tool(tool);
            if tool == Tool::Weights {
                ctx.memory_mut(|memory| memory.request_focus(Self::weights_id()));
            }
        }
    }

    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.confirm_discard() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F12)) {
            let _ = std::fs::create_dir_all(&self.capture_directory);
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            self.capture = Some(self.capture_directory.join(format!("editor-{stamp}.png")));
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        if self.slot_color_shortcuts(ctx)
            || self.ik_shortcuts(ctx)
            || self.transform_shortcuts(ctx)
            || self.weight_edit_shortcuts(ctx)
            || self.timeline_view_shortcuts(ctx)
        {
            return;
        }
        if self.error.is_some()
            || egui::Popup::is_any_open(ctx)
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
        {
            return;
        }
        if self.preview_shortcuts(ctx) {
            return;
        }
        // Buttons and other navigable controls request keyboard input too. Global
        // file/history actions remain available there, while text owns its undo.
        if !ctx.text_edit_focused() {
            self.file_history_shortcuts(ctx);
            self.tool_shortcuts(ctx);
        }
        if self.graph_shortcuts(ctx) {
            return;
        }
        if self.weights_shortcuts(ctx) {
            return;
        }
        if ctx.egui_wants_keyboard_input()
            && !self.dopesheet_has_focus(ctx)
            && !self.timeline_has_focus(ctx)
            && !ctx.memory(|memory| memory.has_focus(Self::weights_id()))
            && !self.weight_control_has_focus(ctx)
        {
            return;
        }
        if self.key_drag.is_some() && ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.cancel_dopesheet_drag();
            return;
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F2)) {
            if let Some(item) = self.selection.clone() {
                self.begin_rename(item);
            }
            return;
        }
        self.timeline_clipboard_shortcuts(ctx);
        // Consume modified keys first so Cmd+R never also advances a frame.
        for (index, key) in [
            (0, egui::Key::Num0),
            (1, egui::Key::Num1),
            (2, egui::Key::Num2),
            (3, egui::Key::Num3),
            (4, egui::Key::Num4),
            (5, egui::Key::Num5),
            (6, egui::Key::Num6),
            (7, egui::Key::Num7),
            (8, egui::Key::Num8),
            (9, egui::Key::Num9),
        ] {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, key)) {
                self.selection_history.store(index, &self.selection);
                self.status = format!(
                    "Selection group {index} {}",
                    if self.selection.is_some() {
                        "stored"
                    } else {
                        "cleared"
                    }
                );
            } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key))
                && let Some(item) = self.selection_history.recall(index)
            {
                self.select(item);
            }
        }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::L))
            && !self.setup_mode
            && self.selected_bone().is_some()
            && self.selected_transform().is_some()
        {
            self.key_transform(&Property::ALL);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::R)) {
            self.looping = !self.looping;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::H)) {
            self.hidden_bones.clear();
            self.hidden_slots.clear();
            self.show_bones = true;
        }
        let modifiers = ctx.input(|i| i.modifiers);
        if modifiers.command || modifiers.ctrl || modifiers.alt {
            return;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::PageUp))
            && let Some(item) = self.selection_history.previous(&self.selection)
        {
            self.restore_selection(item);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::PageDown))
            && let Some(item) = self.selection_history.next(&self.selection)
        {
            self.restore_selection(item);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F9)) {
            self.toggle_views();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.deselect();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.mesh_drag.is_some() || self.drag.is_some() || self.key_drag.is_some() {
                self.cancel_gesture();
            } else {
                self.deselect();
            }
        }
        if !self.setup_mode {
            if ctx.input(|i| i.key_pressed(egui::Key::K)) {
                self.key_edited(None);
            }
            if ctx.input(|i| i.key_pressed(egui::Key::A)) {
                self.toggle_playback(PlayDirection::Backward);
            }
            if ctx.input(|i| i.key_pressed(egui::Key::W)) {
                self.jump_key(PlayDirection::Forward);
            }
            if ctx.input(|i| i.key_pressed(egui::Key::S)) {
                self.jump_key(PlayDirection::Backward);
            }
            if ctx.input(|i| i.key_pressed(egui::Key::D)) {
                self.toggle_playback(PlayDirection::Forward);
            }
            for (key, delta) in [(egui::Key::F, -1.0), (egui::Key::R, 1.0)] {
                if ctx.input(|i| i.key_pressed(key)) {
                    self.step_frames(delta * if modifiers.shift { 10.0 } else { 1.0 });
                }
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Q)) {
                self.set_time(0.0);
                self.playing = false;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::E)) {
                self.set_time(peachy_engine::last_key_time(
                    self.session.document(),
                    self.animation,
                ));
                self.playing = false;
            }
            if self.selected_bone().is_some() && ctx.input(|i| i.key_pressed(egui::Key::L)) {
                let properties = self.tool.properties();
                if !properties.is_empty() && self.selected_transform().is_some() {
                    self.key_transform(properties);
                }
            }
        }
        for (key, delta) in [
            (egui::Key::ArrowLeft, [-1., 0.]),
            (egui::Key::ArrowRight, [1., 0.]),
            (egui::Key::ArrowUp, [0., 1.]),
            (egui::Key::ArrowDown, [0., -1.]),
        ] {
            if ctx.input(|i| i.key_pressed(key)) {
                self.nudge(delta, modifiers.shift);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{editor_context, editor_fixture};
    use super::*;

    fn chord(modifiers: egui::Modifiers) -> Vec<egui::Event> {
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key: egui::Key::Z,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            })
            .collect()
    }

    #[test]
    fn focused_text_keeps_undo_redo_without_consuming_document_history() {
        let mut editor = editor_fixture();
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "committed document edit".into(),
            })
            .unwrap();
        let document = editor.session.document().clone();
        let ctx = editor_context();
        let field = egui::Id::new("shortcut-text-priority");
        let mut text = "base".to_string();
        let mut frame = |events: Vec<egui::Event>, focus: bool| {
            let mut available = false;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    focused: true,
                    events,
                    ..Default::default()
                },
                |ui| {
                    editor.shortcuts(ui.ctx());
                    available = ui.input(|input| input.key_pressed(egui::Key::Z));
                    if focus {
                        ui.memory_mut(|memory| memory.request_focus(field));
                    }
                    ui.add(egui::TextEdit::singleline(&mut text).id(field));
                },
            );
            output.textures_delta.clear();
            (available, text.clone(), editor.session.document().clone())
        };
        frame(vec![], true);
        let (_, typed, _) = frame(vec![egui::Event::Text(" authored".into())], false);
        frame(vec![], false);
        assert!(ctx.text_edit_focused());
        let (available, undone, after_undo) = frame(chord(egui::Modifiers::COMMAND), false);
        assert!(available, "TextEdit must receive its undo event");
        assert_ne!(undone, typed);
        assert_eq!(after_undo, document);
        let (available, redone, after_redo) = frame(
            chord(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT),
            false,
        );
        assert!(available, "TextEdit must receive its redo event");
        assert_eq!(redone, typed);
        assert_eq!(after_redo, document);
        assert!(editor.session.can_undo());
        assert!(!editor.session.can_redo());
    }

    #[test]
    fn popup_and_modal_keep_history_events_with_a_normal_control_focused() {
        let mut editor = editor_fixture();
        let original = editor.session.document().clone();
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "committed document edit".into(),
            })
            .unwrap();
        let edited = editor.session.document().clone();
        let ctx = editor_context();
        let frame = |editor: &mut Editor, events| {
            let mut available = false;
            let mut output = ctx.run_ui(
                egui::RawInput {
                    focused: true,
                    events,
                    ..Default::default()
                },
                |ui| {
                    editor.shortcuts(ui.ctx());
                    available = ui.input(|input| input.key_pressed(egui::Key::Z));
                    ui.button("Normal focused control").request_focus();
                },
            );
            output.textures_delta.clear();
            available
        };
        frame(&mut editor, vec![]);
        assert!(ctx.egui_wants_keyboard_input());
        assert!(!ctx.text_edit_focused());
        let popup = egui::Id::new("shortcut-priority-popup");
        egui::Popup::open_id(&ctx, popup);
        assert!(frame(&mut editor, chord(egui::Modifiers::COMMAND)));
        assert_eq!(editor.session.document(), &edited);
        egui::Popup::close_id(&ctx, popup);
        editor.error = Some("Modal owns keyboard input".into());
        assert!(frame(&mut editor, chord(egui::Modifiers::COMMAND)));
        assert_eq!(editor.session.document(), &edited);
        editor.error = None;
        assert!(!frame(&mut editor, chord(egui::Modifiers::COMMAND)));
        assert_eq!(editor.session.document(), &original);
    }
}
