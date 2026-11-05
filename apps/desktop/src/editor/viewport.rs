use super::*;
use peachy_engine::{Affine, attachment_bounds, pick_attachment};

impl Editor {
    pub(super) fn canvas(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mode = ui.button(
                RichText::new(if self.setup_mode { "SETUP" } else { "ANIMATE" })
                    .strong()
                    .color(ACCENT),
            );
            self.record_target("mode", mode.rect);
            if mode.clicked() {
                self.cancel_gesture();
                self.setup_mode = !self.setup_mode;
                self.playing = false;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{}%", (self.zoom * 100.) as u32))
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
        painter.rect_filled(rect, 0., Color32::from_gray(43));
        let doc = self.session.document();
        let mut scene = evaluate_scene(doc, self.active_animation(), self.time, &self.skin, None);
        scene
            .attachments
            .retain(|a| !self.hidden_slots.contains(&a.slot));
        scene.bounds = attachment_bounds(&scene.attachments);
        if self.fit {
            if let Some(b) = scene.bounds
                && b.width() > 0.
                && b.height() > 0.
            {
                self.zoom = ((rect.width() - 140.) / b.width())
                    .min((rect.height() - 100.) / b.height())
                    .clamp(0.1, 4.);
                let c = b.center();
                self.pan = vec2(-c[0] * self.zoom, c[1] * self.zoom);
            }
            self.fit = false;
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0. {
                let previous = self.zoom;
                self.zoom = (self.zoom * (scroll * 0.0015).exp()).clamp(0.1, 6.);
                self.pan *= self.zoom / previous;
            }
        }
        if response.dragged_by(egui::PointerButton::Secondary)
            || response.dragged_by(egui::PointerButton::Middle)
        {
            self.pan += response.drag_delta();
        }
        let origin = rect.center() + self.pan;
        let zoom = self.zoom;
        let screen = |p: [f32; 2]| origin + vec2(p[0] * zoom, -p[1] * zoom);
        let world_point = |p: Pos2| [(p.x - origin.x) / zoom, -(p.y - origin.y) / zoom];
        let grid = 50. * zoom;
        if grid >= 8. {
            let mut x = origin.x + (rect.left() - origin.x).div_euclid(grid) * grid;
            while x < rect.right() {
                painter.line_segment(
                    [pos2(x, rect.top()), pos2(x, rect.bottom())],
                    Stroke::new(1., Color32::from_gray(49)),
                );
                x += grid;
            }
            let mut y = origin.y + (rect.top() - origin.y).div_euclid(grid) * grid;
            while y < rect.bottom() {
                painter.line_segment(
                    [pos2(rect.left(), y), pos2(rect.right(), y)],
                    Stroke::new(1., Color32::from_gray(49)),
                );
                y += grid;
            }
        }
        painter.line_segment(
            [pos2(rect.left(), origin.y), pos2(rect.right(), origin.y)],
            Stroke::new(1., Color32::from_rgb(70, 47, 48)),
        );
        painter.line_segment(
            [pos2(origin.x, rect.top()), pos2(origin.x, rect.bottom())],
            Stroke::new(1., Color32::from_rgb(38, 67, 60)),
        );
        let selected_key = self.selected_attachment().cloned();
        for draw in &scene.attachments {
            if let Some(texture) = self.textures.get(&draw.image) {
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
            if selected_key.as_ref() == Some(&draw.attachment) {
                let points = draw
                    .vertices
                    .iter()
                    .take(draw.hull)
                    .map(|&p| screen(p))
                    .collect();
                painter.add(egui::Shape::closed_line(points, Stroke::new(1., ACCENT)));
            }
        }
        let selected_bone = self.selected_bone();
        let mut nearest_bone: Option<(usize, f32)> = None;
        for (i, (bone, matrix)) in doc.bones.iter().zip(&scene.world).enumerate() {
            if !self.show_bones || self.hidden_bones.contains(&i) {
                continue;
            }
            let (start, end) = (
                screen(matrix.point(0., 0.)),
                screen(matrix.point(bone.length, 0.)),
            );
            let selected = selected_bone == Some(i);
            let color = if selected {
                ACCENT
            } else {
                MINT.gamma_multiply(0.8)
            };
            painter.line_segment(
                [start, end],
                Stroke::new(if selected { 2.5 } else { 1.3 }, color),
            );
            painter.circle_filled(start, if selected { 5. } else { 3. }, color);
            if selected {
                painter.circle_stroke(start, 13., Stroke::new(1., ACCENT));
                painter.text(
                    end + vec2(10., -10.),
                    egui::Align2::LEFT_CENTER,
                    &bone.name,
                    FontId::proportional(12.),
                    ACCENT,
                );
            }
            if self.tree.filters.bones
                && let Some(mouse) = response.interact_pointer_pos()
            {
                let distance = distance_to_segment(mouse, start, end);
                if distance < 8. && nearest_bone.is_none_or(|(_, d)| distance < d) {
                    nearest_bone = Some((i, distance));
                }
            }
        }
        let mut nearest_vertex = None;
        if self.tool == Tool::Mesh && self.setup_mode {
            for draw in &scene.attachments {
                let Some(Attachment::Mesh(mesh)) = doc.attachment(&draw.attachment) else {
                    continue;
                };
                let selected = selected_key.as_ref() == Some(&draw.attachment);
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
                                Stroke::new(1., MINT),
                            );
                        }
                    }
                }
                for (i, &p) in draw.vertices.iter().enumerate() {
                    let p = screen(p);
                    if selected {
                        painter.circle_filled(
                            p,
                            if i == self.mesh_vertex { 5. } else { 3.5 },
                            if i == self.mesh_vertex { ACCENT } else { MINT },
                        );
                        painter.text(
                            p + vec2(7., -7.),
                            egui::Align2::LEFT_BOTTOM,
                            i.to_string(),
                            FontId::monospace(11.),
                            MINT,
                        );
                    }
                    if let Some(mouse) = response.interact_pointer_pos() {
                        let distance = mouse.distance(p);
                        if distance < 12.
                            && nearest_vertex.as_ref().is_none_or(
                                |hit: &(AttachmentRef, usize, [f32; 2], f32)| distance < hit.3,
                            )
                        {
                            nearest_vertex =
                                Some((draw.attachment.clone(), i, mesh.vertices[i], distance));
                        }
                    }
                }
            }
        }
        if self.capture.is_some() {
            // Original-scene hit points make native verification independent of hand-picked pixels.
            for draw in &scene.attachments {
                for t in draw.triangles.as_chunks::<3>().0 {
                    let p = [0, 1].map(|axis| {
                        draw.vertices[t[0] as usize][axis] * 0.2
                            + draw.vertices[t[1] as usize][axis] * 0.3
                            + draw.vertices[t[2] as usize][axis] * 0.5
                    });
                    let screen_p = screen(p);
                    let clear =
                        doc.bones
                            .iter()
                            .zip(&scene.world)
                            .enumerate()
                            .all(|(i, (b, m))| {
                                !self.show_bones
                                    || self.hidden_bones.contains(&i)
                                    || distance_to_segment(
                                        screen_p,
                                        screen(m.point(0., 0.)),
                                        screen(m.point(b.length, 0.)),
                                    ) > 10.
                            });
                    if rect.contains(screen_p)
                        && clear
                        && pick_attachment(&scene.attachments, p)
                            .is_some_and(|hit| hit.slot == draw.slot)
                    {
                        self.capture_targets.insert(
                            format!("pick:slot:{}", draw.slot),
                            [screen_p.x, screen_p.y, screen_p.x + 1., screen_p.y + 1.],
                        );
                        break;
                    }
                }
            }
        }
        let picked = nearest_bone.map(|(i, _)| TreeItem::Bone(i)).or_else(|| {
            response
                .interact_pointer_pos()
                .and_then(|p| pick_attachment(&scene.attachments, world_point(p)))
                .map(|draw| {
                    if self.tree.filters.attachments {
                        TreeItem::Attachment(draw.attachment.clone())
                    } else if self.tree.filters.slots {
                        TreeItem::Slot(draw.slot)
                    } else {
                        TreeItem::Bone(doc.slots[draw.slot].bone)
                    }
                })
        });
        let mesh_edit = self.mesh_drag.as_ref().and_then(|drag| {
            let bone = doc
                .slots
                .iter()
                .find(|s| s.name == drag.attachment.slot)?
                .bone;
            let matrix = scene.world[bone];
            let mouse = response.interact_pointer_pos()?;
            let start = world_point(drag.pointer);
            let end = world_point(mouse);
            let a = matrix.inverse_point(start[0], start[1])?;
            let b = matrix.inverse_point(end[0], end[1])?;
            Some(Command::SetMeshVertex {
                attachment: drag.attachment.clone(),
                vertex: drag.vertex,
                position: [
                    drag.position[0] + b[0] - a[0],
                    drag.position[1] + b[1] - a[1],
                ],
            })
        });
        let transform_edit = self.drag.as_ref().and_then(|drag| {
            let parent = match &drag.item {
                TreeItem::Bone(b) => doc.bones[*b]
                    .parent
                    .map(|p| scene.world[p])
                    .unwrap_or_else(|| Affine::from_transform(Transform::default())),
                TreeItem::Attachment(k) => {
                    scene.world[doc.slots.iter().find(|s| s.name == k.slot)?.bone]
                }
                _ => return None,
            };
            let mouse = response.interact_pointer_pos()?;
            let start = world_point(drag.pointer);
            let end = world_point(mouse);
            let a = parent.inverse_point(start[0], start[1])?;
            let b = parent.inverse_point(end[0], end[1])?;
            let mut transform = drag.initial;
            if self.tool == Tool::Move {
                transform.x += b[0] - a[0];
                transform.y += b[1] - a[1];
            } else {
                let a = vec2(a[0] - transform.x, a[1] - transform.y);
                let b = vec2(b[0] - transform.x, b[1] - transform.y);
                if a.length() * zoom > 6. {
                    let delta = (b.y.atan2(b.x) - a.y.atan2(a.x)).to_degrees();
                    transform.rotation += (delta + 180.).rem_euclid(360.) - 180.;
                } else {
                    transform.rotation += (mouse.x - drag.pointer.x) * 0.5;
                }
                if ui.input(|i| i.modifiers.shift) {
                    transform.rotation = (transform.rotation / 15.).round() * 15.;
                }
            }
            Some((drag.item.clone(), transform))
        });
        painter.text(
            rect.left_bottom() + vec2(12., -14.),
            egui::Align2::LEFT_BOTTOM,
            if self.tool == Tool::Mesh {
                "Setup vertices · drag to deform · Escape cancels"
            } else {
                "Select a bone or attachment · scroll to zoom · right-drag to pan"
            },
            FontId::proportional(12.),
            MUTED,
        );
        if response.clicked_by(egui::PointerButton::Secondary) {
            self.set_tool(self.last_tool);
            return;
        }
        if self.tool == Tool::Mesh && self.setup_mode {
            if (response.clicked() || response.drag_started_by(egui::PointerButton::Primary))
                && let Some((key, vertex, position, _)) = nearest_vertex
            {
                self.select(TreeItem::Attachment(key.clone()));
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
            } else if response.clicked() {
                if let Some(item) = picked {
                    self.select(item);
                } else {
                    self.deselect();
                }
            }
        } else {
            if response.clicked() {
                if let Some(item) = picked.clone() {
                    self.select(item);
                } else {
                    self.deselect();
                }
            }
            if response.drag_started_by(egui::PointerButton::Primary) {
                if let Some(item) = picked {
                    self.select(item);
                }
                if self.transform_editable()
                    && let (Some(item), Some(initial), Some(pointer)) = (
                        self.selection.clone(),
                        self.selected_transform(),
                        response.interact_pointer_pos(),
                    )
                {
                    self.playing = false;
                    self.drag = Some(TransformDrag {
                        item,
                        initial,
                        pointer,
                    });
                    self.session.begin_transaction();
                }
            }
        }
        if response.dragged_by(egui::PointerButton::Primary) {
            if let Some(command) = mesh_edit {
                self.dispatch(command);
            }
            if let Some((item, transform)) = transform_edit {
                let properties = if self.tool == Tool::Rotate {
                    &[Property::Rotation][..]
                } else {
                    &[Property::X, Property::Y][..]
                };
                self.edit_item_transform(item, transform, properties);
            }
        }
        if response.drag_stopped() {
            self.session.commit_transaction();
            self.drag = None;
            self.mesh_drag = None;
        }
    }
}

fn distance_to_segment(point: Pos2, start: Pos2, end: Pos2) -> f32 {
    let v = end - start;
    let length = v.length_sq();
    if length < 1. {
        return point.distance(start);
    }
    point.distance(start + v * ((point - start).dot(v) / length).clamp(0., 1.))
}
