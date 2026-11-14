use super::*;
use peachy_engine::{geometry_bounds, pick_attachment};

impl Editor {
    pub(super) fn canvas(&mut self, ui: &mut egui::Ui) {
        self.ghosting.rendered_samples = 0;
        self.ghosting.error = None;
        ui.horizontal(|ui| {
            let mode = ui.button(
                RichText::new(if self.setup_mode { "SETUP" } else { "ANIMATE" })
                    .strong()
                    .color(ACCENT),
            );
            self.record_target("mode", mode.rect);
            if mode.clicked() {
                self.cancel_gesture();
                self.set_setup_mode(!self.setup_mode);
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
                let bones = ui.checkbox(&mut self.show_bones, "Bones");
                self.record_target("showBones", bones.rect);
                let ghosts = ui
                    .add_enabled_ui(!self.setup_mode, |ui| {
                        ui.checkbox(&mut self.ghosting.enabled, "Ghosts")
                    })
                    .inner
                    .on_disabled_hover_text("Available in Animate mode.")
                    .on_hover_text("Onion-skin preview · settings in Views → Ghosting");
                self.record_target("ghostViewportEnabled", ghosts.rect);
                if ghosts.changed() {
                    self.cancel_gesture();
                }
            });
        });
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        self.record_target("viewport", rect);
        self.prepare_viewport_transform_press(ui, rect);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0., Color32::from_gray(43));
        let mut scene = match self.scene_for_view() {
            Ok(scene) => scene,
            Err(error) => {
                self.show_evaluation_error(ui, rect, error);
                return;
            }
        };
        scene
            .attachments
            .retain(|a| !self.hidden_slots.contains(&a.slot));
        scene
            .overlays
            .retain(|a| !self.hidden_slots.contains(&a.slot));
        scene.bounds = geometry_bounds(
            scene
                .attachments
                .iter()
                .flat_map(|a| a.vertices.iter().copied())
                .chain(
                    scene
                        .overlays
                        .iter()
                        .flat_map(|a| a.vertices.iter().copied()),
                ),
        );
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
        if let Err(error) = self.check_view_projection(&scene, rect, origin, zoom) {
            self.show_evaluation_error(ui, rect, error);
            return;
        }
        if self.evaluation_error.take().is_some() {
            ui.ctx().request_repaint();
        }
        let ghosts = self.ghost_frame(rect, origin, zoom);
        self.ghosting.error = ghosts.as_ref().err().cloned();
        self.ghosting.rendered_samples = ghosts.as_ref().map_or(0, |frame| frame.sample_count());
        let ghosts = ghosts.unwrap_or_default();
        let screen = |p: [f32; 2]| scene_view::screen_point(origin, zoom, p);
        let world_point = |p: Pos2| scene_view::view_world_point(origin, zoom, p);
        let grid = 50. * zoom;
        for x in scene_view::grid_lines(rect.left(), rect.right(), origin.x, grid) {
            painter.line_segment(
                [pos2(x, rect.top()), pos2(x, rect.bottom())],
                Stroke::new(1., Color32::from_gray(49)),
            );
        }
        for y in scene_view::grid_lines(rect.top(), rect.bottom(), origin.y, grid) {
            painter.line_segment(
                [pos2(rect.left(), y), pos2(rect.right(), y)],
                Stroke::new(1., Color32::from_gray(49)),
            );
        }
        painter.line_segment(
            [pos2(rect.left(), origin.y), pos2(rect.right(), origin.y)],
            Stroke::new(1., Color32::from_rgb(70, 47, 48)),
        );
        painter.line_segment(
            [pos2(origin.x, rect.top()), pos2(origin.x, rect.bottom())],
            Stroke::new(1., Color32::from_rgb(38, 67, 60)),
        );
        if !self.ghosting.on_top {
            self.paint_ghosts(&painter, &ghosts, zoom);
        }
        if let Some(error) = &self.ghosting.error {
            painter.text(
                rect.left_top() + vec2(10., 10.),
                egui::Align2::LEFT_TOP,
                format!("Ghosting: {error}"),
                FontId::proportional(12.),
                ACCENT,
            );
        }
        let selected_key = self.selected_attachment().cloned();
        if let Some(draw) = scene
            .attachments
            .iter()
            .find(|draw| Some(&draw.attachment) == self.weights.mesh.as_ref())
        {
            self.weights_handles = draw
                .vertices
                .iter()
                .map(|&point| {
                    let point = screen(point);
                    [point.x, point.y]
                })
                .collect();
        }
        for draw in &scene.attachments {
            if let Some(texture) = self.textures.get(&draw.image) {
                let mut mesh = egui::Mesh::with_texture(texture.id());
                // Clipped draws show only their clipped geometry.
                let (vertices, uvs, triangles) = draw.render_geometry();
                for (&point, &uv) in vertices.iter().zip(uvs) {
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: screen(point),
                        uv: pos2(uv[0], uv[1]),
                        color: scene_view::draw_tint(draw.color, draw.blend),
                    });
                }
                mesh.indices = triangles.to_vec();
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
        if self.ghosting.on_top {
            self.paint_ghosts(&painter, &ghosts, zoom);
        }
        let overlay_hit = self.paint_attachment_overlays(
            &painter,
            &scene.overlays,
            origin,
            zoom,
            response.interact_pointer_pos(),
        );
        let selected_bone = self.selected_bone();
        let doc = self.session.document();
        let mut nearest_bone: Option<(usize, f32)> = None;
        for (i, (bone, matrix)) in doc.bones.iter().zip(&scene.world).enumerate() {
            // Skin-inactive bones are not posed or drawn (Spine hides them too).
            if !self.show_bones || self.hidden_bones.contains(&i) || !scene.active_bones[i] {
                continue;
            }
            let (start, end) = (
                screen(matrix.point(0., 0.)),
                screen(matrix.point(bone.length, 0.)),
            );
            let selected = if self.tool == Tool::Weights {
                self.weights.bone == Some(i)
            } else {
                selected_bone == Some(i)
            };
            let bound = self.tool == Tool::Weights && self.weights.mesh.as_ref().is_some_and(|key|
                matches!(doc.attachment(key),Some(Attachment::Mesh(Mesh {vertices:MeshVertices::Weighted {influences},.. })) if influences.iter().flatten().any(|influence|influence.bone == i)));
            let color = if bound {
                weights::bone_color(i)
            } else if selected {
                ACCENT
            } else {
                MINT.gamma_multiply(0.8)
            };
            painter.line_segment(
                [start, end],
                Stroke::new(if selected { 2.5 } else { 1.3 }, color),
            );
            if doc
                .constraints
                .iter()
                .enumerate()
                .any(|(index, constraint)| {
                    constraint
                        .as_ik()
                        .is_some_and(|ik| scene.ik_poses[index].mix != 0. && ik.bones.contains(&i))
                })
            {
                painter.circle_stroke(
                    start,
                    if selected { 5. } else { 3. },
                    Stroke::new(1.5, color),
                );
            } else {
                painter.circle_filled(start, if selected { 5. } else { 3. }, color);
            }
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
            if (self.tree.filters.bones || self.weights.bind.is_some() || self.ik_target.is_some())
                && let Some(mouse) = response.interact_pointer_pos()
            {
                let distance = distance_to_segment(mouse, start, end);
                if distance < 8. && nearest_bone.is_none_or(|(_, d)| distance < d) {
                    nearest_bone = Some((i, distance));
                }
            }
        }
        let mut nearest_vertex = None;
        if matches!(self.tool, Tool::Mesh | Tool::Weights) {
            for draw in &scene.attachments {
                let Some(Attachment::Mesh(mesh)) = doc.attachment(&draw.attachment) else {
                    continue;
                };
                if self.tool == Tool::Weights
                    && Some(&draw.attachment) != self.weights.mesh.as_ref()
                {
                    continue;
                }
                let selected = if self.tool == Tool::Weights {
                    self.weights.mesh.as_ref() == Some(&draw.attachment)
                } else {
                    selected_key.as_ref() == Some(&draw.attachment)
                };
                let soft_weights = if selected
                    && self.tool == Tool::Mesh
                    && self.setup_mode
                    && !mesh.is_weighted()
                {
                    self.mesh_drag
                        .as_ref()
                        .filter(|drag| drag.attachment == draw.attachment)
                        .map(|drag| drag.translation.weights().to_vec())
                        .or_else(|| {
                            self.mesh_translation(&draw.attachment, self.mesh_vertex)
                                .ok()
                                .map(|translation| translation.weights().to_vec())
                        })
                } else {
                    None
                };
                if selected {
                    if self.tool == Tool::Mesh
                        && self.mesh_tools.show_radius
                        && self.setup_mode
                        && !mesh.is_weighted()
                        && let Some(&center) = draw.vertices.get(self.mesh_vertex)
                    {
                        let center = screen(center);
                        let radius = self.mesh_tools.settings.size * zoom;
                        let inner = radius * (1. - self.mesh_tools.settings.feather);
                        painter.circle_stroke(
                            center,
                            radius,
                            Stroke::new(1., Color32::from_rgb(91, 199, 224)),
                        );
                        painter.circle_stroke(
                            center,
                            inner,
                            Stroke::new(1., Color32::from_rgb(46, 92, 155)),
                        );
                        self.capture_targets.insert(
                            "meshSoftRadius".into(),
                            [
                                center.x - radius,
                                center.y - radius,
                                center.x + radius,
                                center.y + radius,
                            ],
                        );
                    }
                    if self.tool == Tool::Mesh {
                        self.mesh_handles = draw
                            .vertices
                            .iter()
                            .map(|&p| {
                                let p = screen(p);
                                [p.x, p.y]
                            })
                            .collect();
                    }
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
                        let selected_vertex = if self.tool == Tool::Weights {
                            self.weights.vertex == Some(i)
                        } else {
                            i == self.mesh_vertex
                        };
                        if self.tool == Tool::Weights {
                            self.capture_targets.insert(
                                format!("weightsVertex:{i}"),
                                [p.x - 5., p.y - 5., p.x + 5., p.y + 5.],
                            );
                        }
                        let color = if self.tool == Tool::Mesh {
                            let weight = soft_weights
                                .as_ref()
                                .and_then(|weights| weights.get(i))
                                .copied()
                                .unwrap_or(0.);
                            if selected_vertex {
                                Color32::from_rgb(91, 199, 224)
                            } else if weight > 0. {
                                let channel = |low: f32, high: f32| {
                                    (low + (high - low) * weight).round() as u8
                                };
                                Color32::from_rgb(
                                    channel(35., 91.),
                                    channel(73., 199.),
                                    channel(140., 224.),
                                )
                            } else {
                                Color32::from_gray(132)
                            }
                        } else if selected_vertex {
                            ACCENT
                        } else {
                            MINT
                        };
                        painter.circle_filled(p, if selected_vertex { 5. } else { 3.5 }, color);
                        painter.text(
                            p + vec2(7., -7.),
                            egui::Align2::LEFT_BOTTOM,
                            i.to_string(),
                            FontId::monospace(11.),
                            MINT,
                        );
                    }
                    // Choose the vertex under the press, before egui's drag threshold.
                    // A nearby vertex under the later pointer must not steal the drag.
                    if let Some(mouse) = ui
                        .input(|input| input.pointer.press_origin())
                        .or(response.interact_pointer_pos())
                    {
                        let distance = mouse.distance(p);
                        if distance < 12.
                            && nearest_vertex
                                .as_ref()
                                .is_none_or(|hit: &(AttachmentRef, usize, f32)| distance < hit.2)
                        {
                            nearest_vertex = Some((draw.attachment.clone(), i, distance));
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
                                    || !scene.active_bones[i]
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
        let picked = nearest_bone
            .map(|(i, _)| TreeItem::Bone(i))
            .or(overlay_hit)
            .or_else(|| {
                response
                    .interact_pointer_pos()
                    .and_then(|p| pick_attachment(&scene.attachments, world_point(p)))
                    .map(|draw| {
                        if self.tree.filters.attachments || self.weights.bind.is_some() {
                            TreeItem::Attachment(draw.attachment.clone())
                        } else if self.tree.filters.slots {
                            TreeItem::Slot(draw.slot)
                        } else {
                            TreeItem::Bone(doc.slots[draw.slot].bone)
                        }
                    })
            });
        let mesh_edit = self.mesh_drag.as_ref().and_then(|drag| {
            let mouse = response.interact_pointer_pos()?;
            let delta = [
                (mouse.x - drag.pointer.x) / drag.zoom,
                -(mouse.y - drag.pointer.y) / drag.zoom,
            ];
            Some(
                drag.translation
                    .world_command(drag.attachment.clone(), delta),
            )
        });
        let mut handle_axis = None;
        if self.transform_tool_editable(self.tool)
            && matches!(self.tool, Tool::Move | Tool::Scale | Tool::Shear)
            && let Some(local) = self.selected_transform()
            && let Some(parent) = self
                .selection
                .as_ref()
                .and_then(|item| super::tools::item_parent(item, doc, &scene.world))
        {
            let (applied, world) = match self.selection.as_ref() {
                Some(TreeItem::Bone(bone)) => (scene.applied_locals[*bone], scene.world[*bone]),
                _ => (local, parent.then(Affine::from_transform(local))),
            };
            let pivot = screen(world.point(0., 0.));
            self.record_target(
                "gizmoOrigin",
                egui::Rect::from_center_size(pivot, vec2(6., 6.)),
            );
            let basis = peachy_engine::transform_axes_from_world(
                applied,
                parent,
                world,
                self.axes,
                self.tool.transform_tool().unwrap(),
            );
            let mut nearest_handle = f32::INFINITY;
            for (axis, direction) in basis.into_iter().enumerate() {
                if !direction.into_iter().all(f32::is_finite)
                    || direction[0].hypot(direction[1]) == 0.
                {
                    continue;
                }
                let color = if axis == 0 {
                    Color32::from_rgb(225, 92, 85)
                } else {
                    Color32::from_rgb(100, 195, 115)
                };
                let unit = vec2(direction[0], -direction[1]);
                let tip = pivot + unit * 56.;
                painter.line_segment([pivot + unit * 15., tip], Stroke::new(2., color));
                if self.tool == Tool::Scale {
                    painter.rect_filled(egui::Rect::from_center_size(tip, vec2(8., 8.)), 0., color);
                } else if self.tool == Tool::Shear {
                    painter.circle_stroke(tip, 4.5, Stroke::new(2., color));
                    painter.line_segment(
                        [tip - vec2(3., 3.), tip + vec2(3., 3.)],
                        Stroke::new(1., color),
                    );
                } else {
                    let normal = vec2(-unit.y, unit.x);
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            tip + unit * 3.,
                            tip - unit * 6. + normal * 4.,
                            tip - unit * 6. - normal * 4.,
                        ],
                        color,
                        Stroke::NONE,
                    ));
                }
                painter.text(
                    tip + vec2(7., -7.),
                    egui::Align2::LEFT_BOTTOM,
                    if axis == 0 { "X" } else { "Y" },
                    FontId::proportional(11.),
                    color,
                );
                self.capture_targets.insert(
                    format!("gizmo:{}", if axis == 0 { "X" } else { "Y" }),
                    [tip.x - 4., tip.y - 4., tip.x + 4., tip.y + 4.],
                );
                if let Some(mouse) = ui
                    .input(|i| i.pointer.press_origin())
                    .or(response.interact_pointer_pos())
                {
                    let distance = distance_to_segment(mouse, pivot + unit * 20., tip);
                    if distance < 7. && distance < nearest_handle {
                        handle_axis = Some(axis);
                        nearest_handle = distance;
                    }
                }
            }
        }
        painter.text(
            rect.left_bottom() + vec2(12., -14.),
            egui::Align2::LEFT_BOTTOM,
            if self.weights.bind.is_some() || self.ik_target.is_some() {
                &self.status
            } else if self.tool == Tool::Weights {
                "Weights · select a vertex to inspect bindings · G"
            } else if self.tool == Tool::Shear {
                "Shear · drag X or Y handle · Z · Escape cancels"
            } else if self.tool == Tool::Mesh && !self.setup_mode {
                "Animate vertices · drag or nudge one vertex · K keys edited pose · Escape cancels"
            } else if self.tool == Tool::Mesh && self.selected_mesh_is_weighted() {
                "Weighted vertices · select to inspect bone influences"
            } else if self.tool == Tool::Mesh {
                "Setup vertices · drag to deform · Escape cancels"
            } else {
                "Select a bone or attachment · scroll to zoom · right-drag to pan"
            },
            FontId::proportional(12.),
            MUTED,
        );
        if self.error.is_some()
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
            || egui::Popup::is_any_open(ui.ctx())
            || !ui.input(|input| input.focused)
            || self.mesh_blocked
        {
            return;
        }
        if response.clicked_by(egui::PointerButton::Secondary) {
            self.toggle_transform_tool();
            return;
        }
        if self.ik_target.is_some() {
            if response.clicked() {
                if let Some(item) = picked {
                    self.select(item);
                } else {
                    self.status =
                        "Choose an existing target bone; new-target creation is not supported yet"
                            .into();
                }
            }
            return;
        }
        if self.weights.bind.is_some() {
            if response.clicked()
                && let Some(item) = picked
            {
                self.select(item);
            }
            return;
        }
        if matches!(self.tool, Tool::Weights | Tool::Mesh) {
            if (response.clicked() || response.drag_started_by(egui::PointerButton::Primary))
                && let Some((key, vertex, _)) = nearest_vertex
            {
                self.select(TreeItem::Attachment(key.clone()));
                if self.tool == Tool::Weights {
                    self.weights.vertex = Some(vertex);
                } else {
                    self.mesh_vertex = vertex;
                }
                if self.tool == Tool::Mesh
                    && response.drag_started_by(egui::PointerButton::Primary)
                    && let Some(pointer) = ui
                        .input(|i| i.pointer.press_origin())
                        .or(response.interact_pointer_pos())
                {
                    self.begin_mesh_drag(key, vertex, pointer);
                }
            } else if response.clicked() {
                if let Some(item) = picked {
                    self.select(item);
                } else if self.tool == Tool::Weights {
                    self.cancel_weight_edit();
                    self.weights.vertex = None;
                    self.weights.bone = None;
                } else {
                    self.deselect();
                }
            }
        } else {
            if response.clicked() && handle_axis.is_none() {
                if let Some(item) = picked.clone() {
                    self.select(item);
                } else {
                    self.deselect();
                }
            }
            if response.drag_started_by(egui::PointerButton::Primary) && !self.transform_blocked {
                if handle_axis.is_none()
                    && let Some(item) = picked
                {
                    self.select(item);
                }
                if let Some(pointer) = ui
                    .input(|i| i.pointer.press_origin())
                    .or(response.interact_pointer_pos())
                {
                    self.transform_blocked = false;
                    self.begin_viewport_transform(world_point(pointer), handle_axis);
                }
            }
        }
        if (response.dragged_by(egui::PointerButton::Primary) || response.drag_stopped())
            && let Some(command) = mesh_edit
        {
            match command {
                Ok(command) => {
                    if !self.dispatch(command) {
                        self.cancel_mesh_drag();
                    }
                }
                Err(error) => {
                    self.cancel_mesh_drag();
                    self.error = Some(error);
                }
            }
        }
        if (response.dragged_by(egui::PointerButton::Primary) || response.drag_stopped())
            && self.deform_drag.is_some()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            self.update_deform_drag(pointer);
        }
        if response.dragged_by(egui::PointerButton::Primary)
            && self.viewport_transform_active()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            self.update_viewport_transform(world_point(pointer), ui.input(|i| i.modifiers.shift));
        }
        if response.drag_stopped() {
            if self.viewport_transform_active()
                && let Some(pointer) = response.interact_pointer_pos()
            {
                self.update_viewport_transform(
                    world_point(pointer),
                    ui.input(|i| i.modifiers.shift),
                );
            }
            self.finish_viewport_transform();
            let setup_mesh = self.mesh_drag.take().is_some();
            let deform_mesh = self.deform_drag.take().is_some();
            if setup_mesh || deform_mesh {
                self.session.commit_transaction();
            }
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
