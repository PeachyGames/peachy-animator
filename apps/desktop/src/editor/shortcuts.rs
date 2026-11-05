//! Spine's default bindings for the currently implemented operations.
use super::*;

impl Editor {
    pub(super) fn set_tool(&mut self, tool: Tool) {
        if self.tool != tool {
            if self.mesh_drag.take().is_some() {
                self.session.cancel_transaction();
            }
            self.last_tool = self.tool;
            self.tool = tool;
            self.drag = None;
            self.drag_preview = None;
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
        if self.mesh_drag.take().is_some() {
            self.session.cancel_transaction();
        }
        self.drag = None;
        self.drag_preview = None;
        self.has_selection = false;
    }

    fn step_frames(&mut self, delta: f32) {
        self.playing = false;
        self.time = (self.snapped_time() + delta / self.session.document().fps as f32)
            .clamp(0.0, self.duration());
    }

    fn nudge(&mut self, delta: [f32; 2]) {
        if !self.has_selection {
            return;
        }
        if self.tool == Tool::Mesh {
            if self.setup_mode
                && let Some(key) = self.mesh_attachment.clone()
                && let Some(Attachment::Mesh(mesh)) = self.session.document().attachment(&key)
            {
                let p = mesh.vertices[self.mesh_vertex];
                self.dispatch(Command::SetMeshVertex {
                    attachment: key,
                    vertex: self.mesh_vertex,
                    position: [p[0] + delta[0], p[1] + delta[1]],
                });
            }
        } else {
            let mut pose = local_pose(self.session.document(), self.active_animation(), self.time)
                [self.selected];
            pose.x += delta[0];
            pose.y += delta[1];
            self.edit_transform(pose, &[Property::X, Property::Y]);
        }
    }

    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.confirm_discard() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if ctx.egui_wants_keyboard_input() {
            return;
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
        {
            let pose = local_pose(self.session.document(), self.active_animation(), self.time)
                [self.selected];
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
            if self.mesh_drag.take().is_some() {
                self.session.cancel_transaction();
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
            if self.has_selection && ctx.input(|i| i.key_pressed(egui::Key::L)) {
                let properties = match self.tool {
                    Tool::Rotate => &[Property::Rotation][..],
                    Tool::Move => &[Property::X, Property::Y][..],
                    Tool::Mesh => &[][..],
                };
                if !properties.is_empty() {
                    let pose =
                        local_pose(self.session.document(), self.active_animation(), self.time)
                            [self.selected];
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
