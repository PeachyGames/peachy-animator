use super::*;

impl Editor {
    pub(super) fn canvas(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mode = ui.add(egui::Button::new(
                RichText::new(if self.setup_mode { "SETUP" } else { "ANIMATE" })
                    .strong()
                    .color(ACCENT),
            ));
            self.capture_targets.insert(
                "mode".into(),
                [
                    mode.rect.min.x,
                    mode.rect.min.y,
                    mode.rect.max.x,
                    mode.rect.max.y,
                ],
            );
            if mode.clicked() {
                self.setup_mode = !self.setup_mode;
                self.playing = false;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{}%", (self.zoom * 100.0) as u32))
                        .small()
                        .color(MUTED),
                );
                let fit = ui.small_button("Fit");
                self.record_target("fit", fit.rect);
                if fit.clicked() {
                    self.fit = true;
                }
                ui.checkbox(&mut self.show_bones, "Bones");
            });
        });
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        self.record_target("viewport", rect);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_gray(43));
        let doc = self.session.document();
        let scene = evaluate_scene(
            doc,
            self.active_animation(),
            self.time,
            &self.skin,
            self.drag_preview.map(|p| (self.selected, p)),
        );
        let locals = &scene.locals;
        let world = &scene.world;
        if self.fit {
            if let Some(bounds) = scene.bounds
                && bounds.width() > 0.0
                && bounds.height() > 0.0
            {
                self.zoom = ((rect.width() - 140.0) / bounds.width())
                    .min((rect.height() - 100.0) / bounds.height())
                    .clamp(0.1, 4.0);
                let center = bounds.center();
                self.pan = vec2(-center[0] * self.zoom, center[1] * self.zoom);
            }
            self.fit = false;
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let previous = self.zoom;
                self.zoom = (self.zoom * (scroll * 0.0015).exp()).clamp(0.1, 6.0);
                self.pan *= self.zoom / previous;
            }
        }
        if response.dragged_by(egui::PointerButton::Secondary)
            || response.dragged_by(egui::PointerButton::Middle)
        {
            self.pan += response.drag_delta();
        }
        let origin = rect.center() + self.pan;
        let screen = |p: [f32; 2]| origin + vec2(p[0] * self.zoom, -p[1] * self.zoom);
        let grid = 50.0 * self.zoom;
        if grid >= 8.0 {
            let mut x = origin.x + (rect.left() - origin.x).div_euclid(grid) * grid;
            while x < rect.right() {
                painter.line_segment(
                    [pos2(x, rect.top()), pos2(x, rect.bottom())],
                    Stroke::new(1.0, Color32::from_gray(49)),
                );
                x += grid;
            }
            let mut y = origin.y + (rect.top() - origin.y).div_euclid(grid) * grid;
            while y < rect.bottom() {
                painter.line_segment(
                    [pos2(rect.left(), y), pos2(rect.right(), y)],
                    Stroke::new(1.0, Color32::from_gray(49)),
                );
                y += grid;
            }
        }
        painter.line_segment(
            [pos2(rect.left(), origin.y), pos2(rect.right(), origin.y)],
            Stroke::new(1.0, Color32::from_rgb(70, 47, 48)),
        );
        painter.line_segment(
            [pos2(origin.x, rect.top()), pos2(origin.x, rect.bottom())],
            Stroke::new(1.0, Color32::from_rgb(38, 67, 60)),
        );
        for draw in &scene.attachments {
            let Some(texture) = self.textures.get(&draw.image) else {
                continue;
            };
            let mut mesh = egui::Mesh::with_texture(texture.id());
            for (&point, &uv) in draw.vertices.iter().zip(&draw.uvs) {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: screen(point),
                    uv: pos2(uv[0], uv[1]),
                    color: Color32::WHITE,
                });
            }
            mesh.indices = draw.triangles.clone();
            painter.add(egui::Shape::mesh(mesh));
        }
        let mut nearest_vertex: Option<(AttachmentRef, usize, usize, [f32; 2], f32)> = None;
        if self.tool == Tool::Mesh && self.setup_mode {
            for draw in &scene.attachments {
                let Some(Attachment::Mesh(m)) = doc.attachment(&draw.attachment) else {
                    continue;
                };
                let selected =
                    self.has_selection && self.mesh_attachment.as_ref() == Some(&draw.attachment);
                if selected {
                    self.mesh_handles = draw
                        .vertices
                        .iter()
                        .map(|&p| {
                            let p = screen(p);
                            [p.x, p.y]
                        })
                        .collect();
                    for t in draw.triangles.as_chunks::<3>().0 {
                        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                            painter.line_segment(
                                [
                                    screen(draw.vertices[a as usize]),
                                    screen(draw.vertices[b as usize]),
                                ],
                                Stroke::new(1.0, MINT.gamma_multiply(0.65)),
                            );
                        }
                    }
                }
                for (i, &p) in draw.vertices.iter().enumerate() {
                    let p = screen(p);
                    if selected {
                        painter.circle_filled(
                            p,
                            if i == self.mesh_vertex { 5.0 } else { 3.5 },
                            if i == self.mesh_vertex { ACCENT } else { MINT },
                        );
                        painter.text(
                            p + vec2(7.0, -7.0),
                            egui::Align2::LEFT_BOTTOM,
                            i.to_string(),
                            FontId::monospace(11.0),
                            MINT,
                        );
                    }
                    if let Some(mouse) = response.interact_pointer_pos() {
                        let distance = mouse.distance(p);
                        if distance < 12.0
                            && nearest_vertex.as_ref().is_none_or(|hit| distance < hit.4)
                        {
                            nearest_vertex = Some((
                                draw.attachment.clone(),
                                i,
                                doc.slots[draw.slot].bone,
                                m.vertices[i],
                                distance,
                            ));
                        }
                    }
                }
            }
        }
        let mut nearest: Option<(usize, f32)> = None;
        for (i, (bone, matrix)) in doc.bones.iter().zip(world).enumerate() {
            let start = screen(matrix.point(0.0, 0.0));
            let end = screen(matrix.point(bone.length, 0.0));
            if self.show_bones || (self.has_selection && i == self.selected) {
                let color = if self.has_selection && i == self.selected {
                    ACCENT
                } else {
                    Color32::from_rgba_unmultiplied(117, 205, 185, 175)
                };
                painter.line_segment(
                    [start, end],
                    Stroke::new(
                        if self.has_selection && i == self.selected {
                            2.5
                        } else {
                            1.3
                        },
                        color,
                    ),
                );
                painter.circle_filled(
                    start,
                    if self.has_selection && i == self.selected {
                        5.0
                    } else {
                        3.0
                    },
                    color,
                );
                if self.has_selection && i == self.selected {
                    painter.circle_stroke(start, 13.0, Stroke::new(1.0, ACCENT));
                    painter.text(
                        end + vec2(10.0, -10.0),
                        egui::Align2::LEFT_CENTER,
                        &bone.name,
                        FontId::proportional(12.0),
                        ACCENT,
                    );
                }
            }
            if let Some(mouse) = response.interact_pointer_pos() {
                let distance = distance_to_segment(mouse, start, end);
                if distance < 16.0 && nearest.is_none_or(|(_, best)| distance < best) {
                    nearest = Some((i, distance));
                }
            }
        }
        painter.text(
            rect.left_bottom() + vec2(16.0, -18.0),
            egui::Align2::LEFT_BOTTOM,
            if self.tool == Tool::Mesh {
                "Setup vertices · drag to deform · Escape cancels · UVs stay fixed"
            } else {
                "Drag a bone to pose · scroll to zoom · right-drag to pan"
            },
            FontId::proportional(12.0),
            MUTED,
        );
        if response.clicked_by(egui::PointerButton::Secondary) {
            self.set_tool(self.last_tool);
            return;
        }
        if self.tool == Tool::Mesh {
            if (response.clicked() || response.drag_started_by(egui::PointerButton::Primary))
                && let Some((key, vertex, bone, position, _)) = nearest_vertex
            {
                self.selected = bone;
                self.has_selection = true;
                self.mesh_attachment = Some(key.clone());
                self.mesh_vertex = vertex;
                if response.drag_started_by(egui::PointerButton::Primary)
                    && let Some(pointer) = response.interact_pointer_pos()
                {
                    self.mesh_drag = Some(MeshDrag {
                        attachment: key,
                        vertex,
                        position,
                        pointer,
                    });
                    self.session.begin_transaction();
                }
            }
            let mut edit = None;
            if response.dragged_by(egui::PointerButton::Primary)
                && self.setup_mode
                && let Some(drag) = &self.mesh_drag
                && let Some(mouse) = response.interact_pointer_pos()
            {
                let matrix = world[self.selected];
                let to_local = |p: Pos2| {
                    matrix
                        .inverse_point((p.x - origin.x) / self.zoom, -(p.y - origin.y) / self.zoom)
                };
                if let (Some(a), Some(b)) = (to_local(drag.pointer), to_local(mouse)) {
                    edit = Some(Command::SetMeshVertex {
                        attachment: drag.attachment.clone(),
                        vertex: drag.vertex,
                        position: [
                            drag.position[0] + b[0] - a[0],
                            drag.position[1] + b[1] - a[1],
                        ],
                    });
                }
            }
            if let Some(command) = edit {
                self.dispatch(command);
            }
            if response.drag_stopped() {
                self.session.commit_transaction();
                self.mesh_drag = None;
            }
            return;
        }
        if response.clicked() && nearest.is_none() {
            self.deselect();
            return;
        }
        if response.clicked()
            && let Some((i, _)) = nearest
        {
            self.selected = i;
            self.has_selection = true;
        }
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(i) = nearest
                .map(|(i, _)| i)
                .or_else(|| self.has_selection.then_some(self.selected))
        {
            self.selected = i;
            self.has_selection = true;
            self.playing = false;
            if let Some(mouse) = response.interact_pointer_pos() {
                self.drag = Some((locals[i], mouse));
            }
        }
        if response.dragged_by(egui::PointerButton::Primary)
            && let Some((initial, start)) = self.drag
        {
            let mut preview = initial;
            if let Some(mouse) = response.interact_pointer_pos() {
                match self.tool {
                    Tool::Move => {
                        let delta = (mouse - start) / self.zoom;
                        if let Some(parent) = doc.bones[self.selected].parent {
                            let matrix = world[parent];
                            let a = matrix.inverse_point(0.0, 0.0);
                            let b = matrix.inverse_point(delta.x, -delta.y);
                            if let (Some(a), Some(b)) = (a, b) {
                                preview.x += b[0] - a[0];
                                preview.y += b[1] - a[1];
                            }
                        } else {
                            preview.x += delta.x;
                            preview.y -= delta.y;
                        }
                    }
                    Tool::Mesh => {}
                    Tool::Rotate => {
                        let matrix = world[self.selected];
                        let pivot = screen(matrix.point(0.0, 0.0));
                        let convert = |p: Pos2| {
                            let x = (p.x - origin.x) / self.zoom;
                            let y = -(p.y - origin.y) / self.zoom;
                            if let Some(parent) = doc.bones[self.selected].parent {
                                world[parent]
                                    .inverse_point(x, y)
                                    .map(|p| vec2(p[0] - initial.x, p[1] - initial.y))
                            } else {
                                Some(vec2(x - initial.x, y - initial.y))
                            }
                        };
                        if start.distance(pivot) > 6.0 {
                            if let (Some(a), Some(b)) = (convert(start), convert(mouse)) {
                                let mut delta = (b.y.atan2(b.x) - a.y.atan2(a.x)).to_degrees();
                                while delta > 180.0 {
                                    delta -= 360.0;
                                }
                                while delta < -180.0 {
                                    delta += 360.0;
                                }
                                preview.rotation += delta;
                                if ui.input(|i| i.modifiers.shift) {
                                    preview.rotation = (preview.rotation / 15.0).round() * 15.0;
                                }
                            }
                        } else {
                            preview.rotation += (mouse.x - start.x) * 0.5;
                        }
                    }
                }
                self.drag_preview = Some(preview);
            }
        }
        if response.drag_stopped() {
            if let Some(preview) = self.drag_preview.take() {
                let properties = if self.tool == Tool::Rotate {
                    &[Property::Rotation][..]
                } else {
                    &[Property::X, Property::Y][..]
                };
                self.edit_transform(preview, properties);
            }
            self.drag = None;
        }
    }
}

fn distance_to_segment(point: Pos2, start: Pos2, end: Pos2) -> f32 {
    let v = end - start;
    let length = v.length_sq();
    if length < 1.0 {
        return point.distance(start);
    }
    let t = ((point - start).dot(v) / length).clamp(0.0, 1.0);
    point.distance(start + v * t)
}
