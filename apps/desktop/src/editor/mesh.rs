use super::*;

const WEIGHT_EDIT_HINT: &str = "Use Weights [G] to manage bindings. Weighted Setup vertex transforms and topology editing are not available yet.";

pub(super) struct DeformDrag {
    pub attachment: AttachmentRef,
    pub vertex: usize,
    pub target: EditTarget,
    pub auto_key: bool,
    pub pointer: Pos2,
    pub zoom: f32,
}

#[cfg(test)]
#[path = "deform_authoring_tests.rs"]
mod deform_authoring_tests;

impl Editor {
    pub(super) fn enter_mesh_tool(&mut self) {
        let Some(key) = self.selected_attachment() else {
            return;
        };
        if !self.attachment_editable(key) {
            return;
        }
        let Some(Attachment::Mesh(_)) = self.session.document().attachment(key) else {
            return;
        };
        self.set_tool(Tool::Mesh);
        // Vertices edits Setup geometry or the current Animate deform pose.
        // Entering the tool never changes mode or discards a pending pose.
        self.playing = false;
    }
    pub(super) fn selected_mesh_is_weighted(&self) -> bool {
        let Some(Attachment::Mesh(mesh)) = self
            .selected_attachment()
            .and_then(|key| self.session.document().attachment(key))
        else {
            return false;
        };
        mesh.is_weighted()
    }
    pub(super) fn weighted_mesh_count(&self) -> usize {
        self.session
            .document()
            .skins
            .values()
            .flat_map(|skin| skin.values())
            .flat_map(|attachments| attachments.values())
            .filter(|attachment| matches!(attachment, Attachment::Mesh(mesh) if mesh.is_weighted()))
            .count()
    }
    pub(super) fn mesh_inspection(&self) -> Option<serde_json::Value> {
        let key = self.selected_attachment()?;
        let Attachment::Mesh(mesh) = self.session.document().attachment(key)? else {
            return None;
        };
        let influences = match &mesh.vertices {
            MeshVertices::Weighted { influences } => {
                influences.get(self.mesh_vertex).map(|vertex| {
                    vertex
                        .iter()
                        .map(|influence| {
                            serde_json::json!({
                                "bone":influence.bone,
                                "name":self.session.document().bones[influence.bone].name,
                                "position":influence.position,
                                "weight":influence.weight
                            })
                        })
                        .collect::<Vec<_>>()
                })
            }
            MeshVertices::SlotLocal { .. } => None,
        };
        Some(serde_json::json!({
            "weighted":mesh.is_weighted(), "vertexCount":mesh.vertex_count(),
            "triangleCount":mesh.triangles.len()/3,"selectedVertex":self.mesh_vertex,
            "localPosition":mesh.slot_local_positions().and_then(|positions|positions.get(self.mesh_vertex)),
            "uv":mesh.uvs.get(self.mesh_vertex),"influences":influences
        }))
    }
    pub(super) fn editable_mesh_position(
        &self,
        key: &AttachmentRef,
        vertex: usize,
    ) -> Option<[f32; 2]> {
        if !self.setup_mode || !self.attachment_editable(key) {
            return None;
        }
        let Attachment::Mesh(mesh) = self.session.document().attachment(key)? else {
            return None;
        };
        mesh.slot_local_positions()?.get(vertex).copied()
    }
    pub(super) fn begin_mesh_drag(&mut self, key: AttachmentRef, vertex: usize, pointer: Pos2) {
        if !self.setup_mode {
            self.begin_deform_drag(key, vertex, pointer);
            return;
        }
        if self.mesh_blocked || self.editable_mesh_position(&key, vertex).is_none() {
            return;
        }
        match self.mesh_translation(&key, vertex) {
            Ok(translation) => {
                self.session.begin_transaction();
                self.mesh_drag = Some(MeshDrag {
                    attachment: key,
                    vertex,
                    translation,
                    pointer,
                    zoom: self.zoom,
                });
            }
            Err(error) => self.error = Some(error),
        }
    }
    pub(super) fn deform_world_position(
        &self,
        key: &AttachmentRef,
        vertex: usize,
    ) -> Option<[f32; 2]> {
        if self.setup_mode || !self.attachment_editable(key) {
            return None;
        }
        self.scene_for_view()
            .ok()?
            .attachments
            .iter()
            .find(|draw| &draw.attachment == key && !self.hidden_slots.contains(&draw.slot))?
            .vertices
            .get(vertex)
            .copied()
    }
    fn begin_deform_drag(&mut self, key: AttachmentRef, vertex: usize, pointer: Pos2) {
        if self.mesh_blocked || self.deform_world_position(&key, vertex).is_none() {
            return;
        }
        let target = EditTarget::Animate {
            animation: self.animation,
            time: self.time,
        };
        if let Err(error) = self.session.deform_offsets(&key, target) {
            self.error = Some(error);
            return;
        }
        self.cancel_gesture();
        self.session.commit_transaction();
        self.session.begin_transaction();
        self.playing = false;
        self.deform_drag = Some(DeformDrag {
            attachment: key,
            vertex,
            target,
            auto_key: self.auto_key,
            pointer,
            zoom: self.zoom,
        });
    }
    pub(super) fn update_deform_drag(&mut self, pointer: Pos2) {
        let Some(drag) = self.deform_drag.as_ref() else {
            return;
        };
        let delta = [
            (pointer.x - drag.pointer.x) / drag.zoom,
            -(pointer.y - drag.pointer.y) / drag.zoom,
        ];
        let result = self.session.preview_translate_deform_vertex(
            drag.attachment.clone(),
            drag.vertex,
            drag.target,
            delta,
            drag.auto_key,
        );
        match result {
            Ok(()) => {
                self.status = if drag.auto_key {
                    "Keyed mesh deform".into()
                } else {
                    "Unkeyed mesh deform · K to key".into()
                }
            }
            Err(error) => {
                self.cancel_mesh_drag();
                self.error = Some(error);
            }
        }
    }
    pub(super) fn nudge_deform_vertex(&mut self, key: AttachmentRef, delta: [f32; 2]) {
        if self.deform_world_position(&key, self.mesh_vertex).is_none() {
            return;
        }
        self.playing = false;
        match self.session.translate_deform_vertex(
            key,
            self.mesh_vertex,
            EditTarget::Animate {
                animation: self.animation,
                time: self.time,
            },
            delta,
            self.auto_key,
        ) {
            Ok(()) => {
                self.status = if self.auto_key {
                    "Keyed mesh deform".into()
                } else {
                    "Unkeyed mesh deform · K to key".into()
                }
            }
            Err(error) => self.error = Some(error),
        }
    }
    pub(super) fn seal_deform_for_key(&mut self) {
        if self.deform_drag.take().is_some() {
            self.mesh_blocked = true;
        }
    }
    fn set_mesh_vertex(&mut self, key: AttachmentRef, vertex: usize, position: [f32; 2]) {
        if self.editable_mesh_position(&key, vertex).is_some() {
            self.dispatch(Command::SetMeshVertex {
                attachment: key,
                vertex,
                position,
            });
        }
    }
    fn split_mesh_triangle(&mut self, key: AttachmentRef) {
        if self
            .editable_mesh_position(&key, self.mesh_vertex)
            .is_none()
        {
            return;
        }
        let Some(Attachment::Mesh(mesh)) = self.session.document().attachment(&key) else {
            return;
        };
        let count = mesh.vertex_count();
        if self.dispatch(Command::SplitMeshTriangle {
            attachment: key,
            triangle: self.mesh_triangle,
        }) {
            self.mesh_vertex = count;
            self.status = "Added an interior vertex; UVs and hull preserved".into();
        }
    }
    pub(super) fn reconcile_attachment(&mut self) {
        if let Some(Attachment::Mesh(m)) = self
            .selected_attachment()
            .and_then(|k| self.session.document().attachment(k))
        {
            self.mesh_vertex = self.mesh_vertex.min(m.vertex_count() - 1);
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
        if matches!(attachment, Attachment::Clipping(_) | Attachment::Path(_)) {
            ui.label("Clipping polygons and paths are not editable yet.");
            return;
        }
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
                let count = m.vertex_count();
                let triangles = m.triangles.len() / 3;
                let weighted = m.is_weighted();
                ui.label(format!("{} vertices · {} triangles", count, triangles));
                let vertices = ui.button("Vertices").on_hover_text(if !self.setup_mode {
                    "Pose a mesh vertex in Animate; Auto Key saves a deform key, or press K to key the edited pose."
                } else if weighted {
                    "Select weighted vertices to inspect their bone influences."
                } else {
                    "Edit setup vertices."
                });
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
                let soft = ui
                    .small_button("Mesh Tools")
                    .on_hover_text("Soft selection for Setup vertex drags and arrow nudges");
                self.record_target("meshToolsOpen", soft.rect);
                if soft.clicked() {
                    self.open_mesh_tools_view();
                }
                {
                    let vertex = ui.add(
                        egui::Slider::new(&mut self.mesh_vertex, 0..=count - 1).text("Vertex"),
                    );
                    self.record_target("meshVertex", vertex.rect);
                    let Some(Attachment::Mesh(m)) = self.session.document().attachment(&key) else {
                        return;
                    };
                    let position = m.slot_local_positions().map(|p| p[self.mesh_vertex]);
                    let uv = m.uvs[self.mesh_vertex];
                    if !self.setup_mode {
                        self.deform_panel(ui, &key);
                    } else if let Some(mut position) = position {
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
                                self.record_target(
                                    if axis == 0 {
                                        "meshLocalX"
                                    } else {
                                        "meshLocalY"
                                    },
                                    response.rect,
                                );
                                if response.drag_started() {
                                    self.session.begin_transaction();
                                }
                                changed |= response.changed();
                                stopped |= response.drag_stopped();
                            });
                        }
                        if changed {
                            self.set_mesh_vertex(key.clone(), self.mesh_vertex, position);
                        }
                        if stopped {
                            self.session.commit_transaction();
                        }
                    } else if let MeshVertices::Weighted { influences } = &m.vertices {
                        ui.label(RichText::new("Bone influences").small().color(MUTED));
                        egui::Grid::new("mesh-bone-influences")
                            .num_columns(2)
                            .show(ui, |ui| {
                                for influence in &influences[self.mesh_vertex] {
                                    ui.label(&self.session.document().bones[influence.bone].name);
                                    ui.label(format!("{:.2}%", influence.weight * 100.));
                                    ui.end_row();
                                }
                            });
                        ui.label(RichText::new(WEIGHT_EDIT_HINT).small().color(MUTED));
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
                    let split = ui
                        .add_enabled(
                            self.setup_mode && !weighted,
                            egui::Button::new("Split triangle at center"),
                        )
                        .on_disabled_hover_text(if weighted {
                            WEIGHT_EDIT_HINT
                        } else {
                            "Switch to Setup to split a triangle."
                        });
                    self.capture_targets.insert(
                        "splitTriangle".into(),
                        [
                            split.rect.min.x,
                            split.rect.min.y,
                            split.rect.max.x,
                            split.rect.max.y,
                        ],
                    );
                    if split.clicked() {
                        self.split_mesh_triangle(key);
                    }
                }
                ui.label(RichText::new(if !self.setup_mode {
                    "Drag or nudge one vertex to pose the mesh. Auto Key writes deform keys; K keys an unkeyed pose. Soft selection is available in Setup."
                } else if weighted {
                    "Select vertices to inspect weights. Pose the influencing bones in Animate."
                } else {
                    "Drag setup vertices to deform the image. Switch to Animate to author deform keys. UV editing is not available yet."
                }).small().color(MUTED));
            }
            Attachment::Clipping(_) | Attachment::Path(_) => unreachable!("returned above"),
        }
        ui.separator();
    }
    fn deform_panel(&mut self, ui: &mut egui::Ui, key: &AttachmentRef) {
        let state = match self.session.pose_edits().deform_key_state(
            self.session.document(),
            self.animation,
            self.time,
            key,
        ) {
            Ok(state) => state,
            Err(error) => {
                ui.label(RichText::new(error).small().color(ACCENT));
                return;
            }
        };
        let visible = self.deform_world_position(key, self.mesh_vertex);
        ui.horizontal(|ui| {
            ui.label("Deform");
            let key_button = ui.add_enabled_ui(visible.is_some(), |ui| {
                super::animation::pose_key_button(ui, state)
            }).inner.on_hover_text("Key this mesh's complete deform pose at the current frame.");
            self.record_target("meshDeformKey", key_button.rect);
            if key_button.clicked() {
                self.request_pose_key(super::pose_key::PoseKeyAction::Deform {
                    attachment: key.clone(),
                });
                ui.ctx().request_repaint();
            }
            let delete = ui.add_enabled(state == KeyState::Keyed,
                egui::Button::new("Delete key"))
                .on_hover_text("Delete this mesh's deform key at the current frame. Pending poses must be keyed or discarded first.");
            self.record_target("meshDeformDelete", delete.rect);
            if delete.clicked() {
                self.cancel_gesture();
                self.dispatch(Command::DeleteDeformKey {
                    animation: self.animation, attachment: key.clone(), time: self.time,
                });
                ui.ctx().request_repaint();
            }
        });
        if let Some([x, y]) = visible {
            ui.label(
                RichText::new(format!("World position: {x:.2}, {y:.2}"))
                    .small()
                    .color(MUTED),
            );
        } else {
            ui.label(
                RichText::new("Show this attachment at the current frame to pose its vertices.")
                    .small()
                    .color(MUTED),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{editor_context, mesh_editor};
    use super::*;

    fn frame(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>, canvas: bool) {
        let mut output = ctx.run_ui(
            egui::RawInput {
                events,
                focused: true,
                ..Default::default()
            },
            |ui| {
                ui.set_min_size(vec2(1000., 700.));
                editor.capture_targets.clear();
                editor.mesh_handles.clear();
                if canvas {
                    editor.canvas(ui);
                } else {
                    editor.attachment_panel(ui);
                }
                editor.shortcuts(ui.ctx());
                editor.observe_timeline_input(ui.ctx());
                editor.reconcile_selection();
            },
        );
        output.textures_delta.clear();
    }

    fn arrow() -> egui::Event {
        egui::Event::Key {
            key: egui::Key::ArrowRight,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn mouse(position: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn right_click_canvas(editor: &mut Editor, ctx: &egui::Context) {
        frame(editor, ctx, vec![], true);
        let viewport = editor.capture_targets["viewport"];
        let point = pos2(viewport[0] + 25., viewport[1] + 25.);
        let button = |pressed| egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(
            editor,
            ctx,
            vec![egui::Event::PointerMoved(point), button(true)],
            true,
        );
        frame(editor, ctx, vec![button(false)], true);
    }

    #[test]
    fn mesh_inspection_restores_each_transform_and_preserves_its_right_click_pair() {
        for (active, prior) in [
            (Tool::Rotate, Tool::Move),
            (Tool::Move, Tool::Scale),
            (Tool::Scale, Tool::Rotate),
            (Tool::Shear, Tool::Rotate),
            (Tool::Rotate, Tool::Shear),
        ] {
            for select_bone in [true, false] {
                let (mut editor, _) = mesh_editor(true);
                editor.tool = active;
                editor.last_tool = prior;
                let original = editor.session.document().clone();
                editor.enter_mesh_tool();
                assert!(editor.tool == Tool::Mesh);
                assert!(editor.mesh_return_tool == Some(active));
                assert!(editor.last_tool == prior);
                let ctx = editor_context();
                if select_bone {
                    editor.select(TreeItem::Bone(8));
                } else {
                    right_click_canvas(&mut editor, &ctx);
                }
                assert!(editor.tool == active);
                assert!(editor.last_tool == prior);
                assert!(editor.mesh_return_tool.is_none());
                right_click_canvas(&mut editor, &ctx);
                assert!(editor.tool == prior);
                assert!(editor.last_tool == active);
                right_click_canvas(&mut editor, &ctx);
                assert!(editor.tool == active);
                assert!(editor.last_tool == prior);
                assert_eq!(editor.session.document(), &original);
                assert!(!editor.session.can_undo());
            }
        }
    }

    #[test]
    fn explicit_transform_shortcuts_from_mesh_update_the_previous_transform_coherently() {
        for (active, prior) in [
            (Tool::Rotate, Tool::Move),
            (Tool::Move, Tool::Scale),
            (Tool::Scale, Tool::Rotate),
            (Tool::Shear, Tool::Rotate),
            (Tool::Rotate, Tool::Shear),
        ] {
            for (requested, key) in [
                (Tool::Rotate, egui::Key::C),
                (Tool::Move, egui::Key::V),
                (Tool::Scale, egui::Key::X),
                (Tool::Shear, egui::Key::Z),
            ] {
                let (mut editor, _) = mesh_editor(true);
                editor.tool = active;
                editor.last_tool = prior;
                editor.enter_mesh_tool();
                let ctx = editor_context();
                frame(
                    &mut editor,
                    &ctx,
                    vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    true,
                );
                let previous = if requested == active { prior } else { active };
                assert!(editor.tool == requested);
                assert!(editor.last_tool == previous);
                assert!(editor.mesh_return_tool.is_none());
                right_click_canvas(&mut editor, &ctx);
                assert!(editor.tool == previous);
                assert!(editor.last_tool == requested);
                right_click_canvas(&mut editor, &ctx);
                assert!(editor.tool == requested);
                assert!(editor.last_tool == previous);
                assert!(!editor.session.can_undo());
            }
        }
    }

    #[test]
    fn weighted_inspection_in_animate_preserves_pose_history_and_uses_world_handles() {
        let (mut editor, key) = mesh_editor(true);
        editor.set_setup_mode(false);
        let original = editor.session.document().clone();
        editor.enter_mesh_tool();
        assert!(!editor.setup_mode);
        assert!(editor.tool == Tool::Mesh);
        let ctx = editor_context();
        frame(&mut editor, &ctx, vec![], true);
        let before = editor.mesh_handles.clone();
        assert_eq!(before.len(), 4);
        assert!(!editor.session.can_undo());
        editor.select(TreeItem::Bone(8));
        assert!(editor.tool == Tool::Rotate);
        assert!(editor.last_tool == Tool::Move);
        let mut pose = editor.selected_transform().unwrap();
        pose.x += 30.;
        editor.edit_transform(pose, &[Property::X]);
        let pending = editor.session.pose_edits().clone();
        editor.select(TreeItem::Attachment(key.clone()));
        editor.enter_mesh_tool();
        assert!(!editor.setup_mode);
        assert!(editor.selected_bone().is_none());
        assert_eq!(editor.session.pose_edits(), &pending);
        editor.mesh_vertex = usize::MAX;
        editor.reconcile_attachment();
        assert_eq!(editor.mesh_vertex, 3);
        frame(&mut editor, &ctx, vec![], true);
        assert_ne!(editor.mesh_handles, before);
        let scene = editor.scene_for_view().unwrap();
        let draw = scene
            .attachments
            .iter()
            .find(|draw| draw.attachment == key)
            .unwrap();
        let viewport = editor.capture_targets["viewport"];
        let origin = pos2(
            (viewport[0] + viewport[2]) / 2.,
            (viewport[1] + viewport[3]) / 2.,
        ) + editor.pan;
        let world = draw.vertices[3];
        let expected = origin + vec2(world[0] * editor.zoom, -world[1] * editor.zoom);
        assert!(
            expected.distance(pos2(editor.mesh_handles[3][0], editor.mesh_handles[3][1])) < 1e-3
        );
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert!(!editor.session.is_dirty());
        editor.undo();
        assert!(editor.session.pose_edits().is_empty());
        assert!(!editor.session.can_undo());
        editor.redo();
        assert_eq!(editor.session.pose_edits(), &pending);
    }

    #[test]
    fn weighted_edit_guards_and_disabled_widgets_never_mutate_document_pose_or_history() {
        let (mut editor, key) = mesh_editor(true);
        let mut pose = editor.session.local_pose(Some(0), 0.)[8];
        pose.rotation += 19.;
        editor
            .session
            .edit_pose(
                8,
                EditTarget::Animate {
                    animation: 0,
                    time: 0.,
                },
                pose,
                &[Property::Rotation],
                false,
            )
            .unwrap();
        let original = editor.session.document().clone();
        let pending = editor.session.pose_edits().clone();
        editor.enter_mesh_tool();
        editor.begin_mesh_drag(key.clone(), 1, Pos2::ZERO);
        assert!(editor.mesh_drag.is_none());
        editor.set_mesh_vertex(key.clone(), 1, [123., 456.]);
        editor.split_mesh_triangle(key);
        let ctx = editor_context();
        frame(&mut editor, &ctx, vec![arrow()], false);
        assert!(editor.capture_targets.contains_key("meshVertex"));
        assert!(editor.capture_targets.contains_key("splitTriangle"));
        assert!(!editor.capture_targets.contains_key("meshLocalX"));
        assert!(!editor.capture_targets.contains_key("meshLocalY"));
        let split = editor.capture_targets["splitTriangle"];
        let point = pos2((split[0] + split[2]) / 2., (split[1] + split[3]) / 2.);
        frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(point), mouse(point, true)],
            false,
        );
        frame(&mut editor, &ctx, vec![mouse(point, false)], false);
        let inspection = editor.mesh_inspection().unwrap();
        assert_eq!(editor.weighted_mesh_count(), 1);
        assert_eq!(inspection["weighted"], true);
        assert_eq!(inspection["influences"][0]["weight"], 0.25);
        assert_eq!(inspection["influences"][1]["bone"], 8);
        assert!(inspection["localPosition"].is_null());
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert!(editor.error.is_none());
        editor.undo();
        assert!(editor.session.pose_edits().is_empty());
        assert!(!editor.session.can_undo());
    }

    #[test]
    fn weighted_setup_canvas_inspection_does_not_replace_a_pose_transaction() {
        let (mut editor, _) = mesh_editor(true);
        editor.enter_mesh_tool();
        let original = editor.session.document().clone();
        // This canvas-only harness excludes the full app's idle commit policy.
        // Inspection must not replace the pose transaction with a mesh edit transaction.
        let mut pose = editor.session.local_pose(Some(0), 0.)[8];
        pose.rotation += 7.;
        editor.session.begin_transaction();
        editor
            .session
            .edit_pose(
                8,
                EditTarget::Animate {
                    animation: 0,
                    time: 0.,
                },
                pose,
                &[Property::Rotation],
                false,
            )
            .unwrap();
        let pending = editor.session.pose_edits().clone();
        let ctx = editor_context();
        frame(&mut editor, &ctx, vec![], true);
        let point = pos2(editor.mesh_handles[2][0], editor.mesh_handles[2][1]);
        frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(point), mouse(point, true)],
            true,
        );
        frame(&mut editor, &ctx, vec![mouse(point, false)], true);
        assert_eq!(editor.mesh_vertex, 2);
        frame(&mut editor, &ctx, vec![mouse(point, true)], true);
        frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(point + vec2(8., 0.))],
            true,
        );
        assert!(editor.mesh_drag.is_none());
        frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(point + vec2(40., 0.))],
            true,
        );
        frame(
            &mut editor,
            &ctx,
            vec![mouse(point + vec2(40., 0.), false)],
            true,
        );
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert!(editor.session.can_undo());
        assert!(!editor.session.is_dirty());
        assert!(editor.setup_mode);
        assert!(editor.error.is_none());
        editor.session.cancel_transaction();
        assert!(editor.session.pose_edits().is_empty());
        assert!(!editor.session.can_undo());
    }

    #[test]
    fn unweighted_mesh_controls_still_edit_nudge_subdivide_and_cancel_atomically() {
        let (mut editor, key) = mesh_editor(false);
        editor.enter_mesh_tool();
        let original = editor.session.document().clone();
        let position = editor.editable_mesh_position(&key, 0).unwrap();
        let ctx = editor_context();
        frame(&mut editor, &ctx, vec![arrow()], false);
        assert!(editor.capture_targets.contains_key("meshLocalX"));
        assert_eq!(
            editor.editable_mesh_position(&key, 0).unwrap(),
            [position[0] + 1., position[1]]
        );
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        editor.set_mesh_vertex(key.clone(), 0, [position[0] + 2., position[1]]);
        assert_ne!(editor.session.document(), &original);
        editor.undo();
        editor.begin_mesh_drag(key.clone(), 0, Pos2::ZERO);
        assert!(editor.mesh_drag.is_some());
        editor.set_mesh_vertex(key.clone(), 0, [position[0] + 3., position[1]]);
        editor.cancel_gesture();
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
        editor.split_mesh_triangle(key);
        assert_eq!(editor.mesh_vertex, 4);
        assert_eq!(editor.mesh_inspection().unwrap()["vertexCount"], 5);
        editor.undo();
        assert_eq!(editor.session.document(), &original);
    }
}
