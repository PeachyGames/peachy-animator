//! Spine's default bindings for the currently implemented operations.
use super::*;

impl Editor {
    pub(super) fn cancel_gesture(&mut self) {
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
        self.selection = None;
    }
    fn step_frames(&mut self, delta: f32) {
        self.playing = false;
        self.time = (self.snapped_time() + delta / self.session.document().fps as f32)
            .clamp(0.0, self.duration());
    }

    fn nudge(&mut self, delta: [f32; 2]) {
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
        } else if self.transform_editable()
            && let Some(mut pose) = self.selected_transform()
        {
            pose.x += delta[0];
            pose.y += delta[1];
            self.edit_transform(pose, &[Property::X, Property::Y]);
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
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        // Consume modified keys first so Cmd+R never also advances a frame.
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
            && let Some(pose) = self.selected_transform()
        {
            self.edit_transform(pose, &Property::ALL);
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
        if ctx.input(|i| i.key_pressed(egui::Key::F9)) {
            self.toggle_views();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::C)) {
            self.set_tool(Tool::Rotate);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::V)) {
            self.set_tool(Tool::Move);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.deselect();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.mesh_drag.is_some() || self.drag.is_some() {
                self.cancel_gesture();
            } else {
                self.deselect();
            }
        }
        if !self.setup_mode {
            if ctx.input(|i| i.key_pressed(egui::Key::D)) {
                self.playing = !self.playing;
            }
            for (key, delta) in [(egui::Key::F, -1.0), (egui::Key::R, 1.0)] {
                if ctx.input(|i| i.key_pressed(key)) {
                    self.step_frames(delta * if modifiers.shift { 10.0 } else { 1.0 });
                }
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Q)) {
                self.time = 0.0;
                self.playing = false;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::E)) {
                self.time = self.duration();
                self.playing = false;
            }
            if self.selected_bone().is_some() && ctx.input(|i| i.key_pressed(egui::Key::L)) {
                let properties = match self.tool {
                    Tool::Rotate => &[Property::Rotation][..],
                    Tool::Move => &[Property::X, Property::Y][..],
                    Tool::Mesh => &[][..],
                };
                if !properties.is_empty()
                    && let Some(pose) = self.selected_transform()
                {
                    self.edit_transform(pose, properties);
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
                self.nudge(delta);
            }
        }
    }
}
