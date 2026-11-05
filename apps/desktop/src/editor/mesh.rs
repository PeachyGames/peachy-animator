use super::*;

impl Editor {
    pub(super) fn enter_mesh_tool(&mut self) {
        self.tool = Tool::Mesh;
        self.setup_mode = true;
        self.playing = false;
        self.drag = None;
        self.drag_preview = None;
        self.reconcile_attachment();
    }

    pub(super) fn reconcile_attachment(&mut self) {
        let choices = self.attachment_choices();
        if !self
            .mesh_attachment
            .as_ref()
            .is_some_and(|key| choices.contains(key))
        {
            self.mesh_attachment = choices.into_iter().next();
            self.mesh_vertex = 0;
            self.mesh_triangle = 0;
        }
        if let Some(Attachment::Mesh(m)) = self
            .mesh_attachment
            .as_ref()
            .and_then(|k| self.session.document().attachment(k))
        {
            self.mesh_vertex = self.mesh_vertex.min(m.vertices.len() - 1);
            self.mesh_triangle = self.mesh_triangle.min(m.triangles.len() / 3 - 1);
        } else {
            self.mesh_vertex = 0;
            self.mesh_triangle = 0;
        }
    }

    fn attachment_choices(&self) -> Vec<AttachmentRef> {
        self.session
            .document()
            .slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.bone == self.selected)
            .filter_map(|(i, _)| {
                self.session
                    .document()
                    .resolved_attachment(&self.skin, i)
                    .map(|(key, _)| key)
            })
            .collect()
    }
    pub(super) fn attachment_panel(&mut self, ui: &mut egui::Ui) {
        self.reconcile_attachment();
        let choices = self.attachment_choices();
        if choices.is_empty() {
            return;
        }
        ui.label(RichText::new("ATTACHMENT").small().color(MUTED));
        egui::ComboBox::from_id_salt("attachment")
            .selected_text(
                self.mesh_attachment
                    .as_ref()
                    .map_or("", |k| k.slot.as_str()),
            )
            .show_ui(ui, |ui| {
                for key in choices {
                    let label = key.slot.clone();
                    ui.selectable_value(&mut self.mesh_attachment, Some(key), label);
                }
            });
        let Some(key) = self.mesh_attachment.clone() else {
            return;
        };
        ui.label(
            RichText::new(format!("Skin: {}", key.skin))
                .small()
                .color(MUTED),
        );
        let Some(attachment) = self.session.document().attachment(&key) else {
            return;
        };
        match attachment {
            Attachment::Region(_) => {
                let convert = ui.add_enabled(self.setup_mode, egui::Button::new("Convert to mesh"));
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
                if ui.button("Setup vertices  [M]").clicked() {
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
