//! Spine's default bindings for the currently implemented operations.
use super::*;

impl Editor {
    pub(super) fn cancel_gesture(&mut self) {
        self.cancel_dopesheet_drag();
        let transform_drag = self.drag.take().is_some();
        let mesh_drag = self.mesh_drag.take().is_some();
        if transform_drag || mesh_drag {
            self.session.cancel_transaction();
        }
    }
    pub(super) fn set_tool(&mut self, tool: Tool) {
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
            if self.tool != Tool::Mesh && tool != Tool::Mesh {
                self.last_tool = self.tool;
            }
            self.tool = tool;
        }
    }
    pub(super) fn toggle_views(&mut self) {
        self.cancel_dopesheet_drag();
        if let Some((tree, dopesheet)) = self.hidden_views.take() {
            self.show_tree = tree;
            self.show_dopesheet = dopesheet;
        } else {
            self.hidden_views = Some((self.show_tree, self.show_dopesheet));
            self.show_tree = false;
            self.show_dopesheet = false;
        }
    }

    pub(super) fn deselect(&mut self) {
        self.cancel_gesture();
        self.selection_history.record(&self.selection, &None);
        self.selection = None;
    }
    fn step_frames(&mut self, delta: f32) {
        self.playing = false;
        self.set_time(self.snapped_time() + delta / self.session.document().fps as f32);
    }

    fn nudge(&mut self, delta: [f32; 2], fine: bool) {
        if self.key_drag.is_some() {
            return;
        }
        if self.tool == Tool::Mesh {
            if self.setup_mode
                && let Some(key) = self.selected_attachment().cloned()
                && self.attachment_editable(&key)
                && let Some(Attachment::Mesh(mesh)) = self.session.document().attachment(&key)
            {
                let p = mesh.vertices[self.mesh_vertex];
                self.dispatch(Command::SetMeshVertex {
                    attachment: key,
                    vertex: self.mesh_vertex,
                    position: [p[0] + delta[0], p[1] + delta[1]],
                });
            }
        } else if self.transform_editable() {
            self.nudge_transform(delta, fine);
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
        if self.rename.is_some()
            || self.duplicate_animation.is_some()
            || (ctx.egui_wants_keyboard_input() && !self.dopesheet_has_focus(ctx))
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
        if self.dopesheet_has_focus(ctx)
            && ctx.input_mut(|input| {
                input.consume_key(egui::Modifiers::NONE, egui::Key::Delete)
                    || input.consume_key(egui::Modifiers::NONE, egui::Key::Backspace)
            })
        {
            self.delete_dopesheet_keys();
        }
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

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.open_project(ctx);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save_project();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::E)) {
            self.export();
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
        if ctx.input(|i| i.key_pressed(egui::Key::C)) {
            self.set_tool(Tool::Rotate);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::X)) {
            self.set_tool(Tool::Scale);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::V)) {
            self.set_tool(Tool::Move);
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
