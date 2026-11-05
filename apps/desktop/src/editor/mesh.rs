use super::*;

impl Editor {
    pub(super) fn enter_mesh_tool(&mut self) {
        if let Some(key) = self.selected_attachment().cloned()
            && self.attachment_editable(&key)
            && matches!(
                self.session.document().attachment(&key),
                Some(Attachment::Mesh(_))
            )
        {
            self.set_tool(Tool::Mesh);
            self.setup_mode = true;
            self.playing = false;
        }
    }
    pub(super) fn reconcile_attachment(&mut self) {
        if let Some(Attachment::Mesh(m)) = self
            .selected_attachment()
            .and_then(|k| self.session.document().attachment(k))
        {
            self.mesh_vertex = self.mesh_vertex.min(m.vertices.len() - 1);
            self.mesh_triangle = self.mesh_triangle.min(m.triangles.len() / 3 - 1);
        } else {
            self.mesh_vertex = 0;
            self.mesh_triangle = 0;
        }
    }
    pub(super) fn attachment_panel(&mut self, ui: &mut egui::Ui) {
        let Some(key) = self.selected_attachment().cloned() else {
            return;
        };
        if !self.attachment_editable(&key) {
            ui.label("Activate this skin to edit the attachment.");
            return;
        }
        let Some(attachment) = self.session.document().attachment(&key) else {
            return;
        };
        match attachment {
            Attachment::Region(_) => {
                let mut convert_to_mesh = false;
                let convert = ui.add_enabled(
                    self.setup_mode,
                    egui::Checkbox::new(&mut convert_to_mesh, "Mesh"),
                );
                self.capture_targets.insert(
                    "convertMesh".into(),
                    [
                        convert.rect.min.x,
                        convert.rect.min.y,
                        convert.rect.max.x,
                        convert.rect.max.y,
                    ],
                );
                if convert.clicked()
                    && self.dispatch(Command::ConvertRegionToMesh { attachment: key })
                {
                    self.enter_mesh_tool();
                    self.status = "Region converted to a four-vertex mesh".into();
                }
                if !self.setup_mode {
                    ui.label(
                        RichText::new("Switch to Setup to convert this region.")
                            .small()
                            .color(MUTED),
                    );
                }
            }
            Attachment::Mesh(m) => {
                let count = m.vertices.len();
                let triangles = m.triangles.len() / 3;
                ui.label(format!("{} vertices · {} triangles", count, triangles));
                let vertices = ui.button("Vertices");
                self.capture_targets.insert(
                    "meshVertices".into(),
                    [
                        vertices.rect.min.x,
                        vertices.rect.min.y,
                        vertices.rect.max.x,
                        vertices.rect.max.y,
                    ],
                );
                if vertices.clicked() {
                    self.enter_mesh_tool();
                }
                if self.setup_mode {
                    ui.add(egui::Slider::new(&mut self.mesh_vertex, 0..=count - 1).text("Vertex"));
                    let Some(Attachment::Mesh(m)) = self.session.document().attachment(&key) else {
                        return;
                    };
                    let mut position = m.vertices[self.mesh_vertex];
                    let uv = m.uvs[self.mesh_vertex];
                    let mut changed = false;
                    let mut stopped = false;
                    for (axis, label) in ["Local X", "Local Y"].into_iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(label);
                            let response = ui.add(
                                egui::DragValue::new(&mut position[axis])
                                    .speed(0.25)
                                    .max_decimals(2),
                            );
                            if response.drag_started() {
                                self.session.begin_transaction();
                            }
                            changed |= response.changed();
                            stopped |= response.drag_stopped();
                        });
                    }
                    if changed {
                        self.dispatch(Command::SetMeshVertex {
                            attachment: key.clone(),
                            vertex: self.mesh_vertex,
                            position,
                        });
                    }
                    if stopped {
                        self.session.commit_transaction();
                    }
                    ui.label(
                        RichText::new(format!("Fixed UV: {:.3}, {:.3}", uv[0], uv[1]))
                            .small()
                            .color(MUTED),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.mesh_triangle, 0..=triangles - 1)
                            .text("Triangle"),
                    );
                    let split = ui.button("Split triangle at center");
                    self.capture_targets.insert(
                        "splitTriangle".into(),
                        [
                            split.rect.min.x,
                            split.rect.min.y,
                            split.rect.max.x,
                            split.rect.max.y,
                        ],
                    );
                    if split.clicked()
                        && self.dispatch(Command::SplitMeshTriangle {
                            attachment: key,
                            triangle: self.mesh_triangle,
                        })
                    {
                        self.mesh_vertex = count;
                        self.status = "Added an interior vertex; UVs and hull preserved".into();
                    }
                }
                ui.label(RichText::new("Drag setup vertices to deform the image. UV editing, weights and deform keys are later slices.").small().color(MUTED));
            }
        }
        ui.separator();
    }
}
