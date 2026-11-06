//! Explicit per-tool selection; bind geometry and history belong to Session.
use super::*;

pub(super) fn bone_color(bone: usize) -> Color32 {
    [
        MINT,
        Color32::from_rgb(110, 165, 220),
        Color32::from_rgb(215, 130, 115),
        Color32::from_rgb(190, 145, 210),
        Color32::from_rgb(220, 190, 105),
        Color32::from_rgb(110, 195, 190),
    ][bone % 6]
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum BindMode {
    Mesh(AttachmentRef),
    Bone(usize),
}

#[derive(Default)]
pub(super) struct WeightsState {
    pub mesh: Option<AttachmentRef>,
    pub vertex: Option<usize>,
    pub bone: Option<usize>,
    pub bind: Option<BindMode>,
}

impl WeightsState {
    pub fn select_mesh(&mut self, key: AttachmentRef) {
        if self.mesh.as_ref() != Some(&key) {
            self.bind = None;
            self.mesh = Some(key);
            self.vertex = None;
            self.bone = None;
        }
    }
    pub fn reconcile(&mut self, document: &Document) {
        let mesh = self.mesh.as_ref().and_then(|key| document.attachment(key));
        if let Some(Attachment::Mesh(mesh)) = mesh {
            self.vertex = self.vertex.filter(|&vertex| vertex < mesh.vertex_count());
        } else {
            self.mesh = None;
            self.vertex = None;
            self.bone = None;
        }
        self.bone = self.bone.filter(|&bone| bone < document.bones.len());
        let valid = match &self.bind {
            Some(BindMode::Mesh(key)) => {
                self.mesh.as_ref() == Some(key)
                    && matches!(document.attachment(key), Some(Attachment::Mesh(_)))
            }
            Some(BindMode::Bone(bone)) => *bone < document.bones.len(),
            None => true,
        };
        if !valid {
            self.bind = None;
        }
    }
}

impl Editor {
    pub(super) fn open_weights_view(&mut self) {
        if let Some(views) = self.hidden_views.take() {
            self.show_tree = views.tree;
            self.show_dopesheet = views.dopesheet;
        }
        self.show_weights = true;
    }
    pub(super) fn weights_id() -> egui::Id {
        egui::Id::new("weights-view")
    }
    pub(super) fn weights_mesh_active(&self, key: &AttachmentRef) -> bool {
        let document = self.session.document();
        let Some(slot) = document.slots.iter().position(|slot| slot.name == key.slot) else {
            return false;
        };
        document.slots[slot].attachment.as_deref() == Some(key.placeholder.as_str())
            && document
                .resolve_attachment(&self.skin, slot, &key.placeholder)
                .is_some_and(|(resolved, attachment)| {
                    resolved == *key && matches!(attachment, Attachment::Mesh(_))
                })
    }
    pub(super) fn capture_weights_mesh(&mut self, key: &AttachmentRef) {
        if self.weights_mesh_active(key) {
            self.weights.select_mesh(key.clone());
        } else {
            self.weights = Default::default();
        }
    }
    pub(super) fn reconcile_weights(&mut self) {
        self.weights.reconcile(self.session.document());
        if self
            .weights
            .mesh
            .as_ref()
            .is_some_and(|key| !self.weights_mesh_active(key))
        {
            self.weights = Default::default();
        }
        if let Some(BindMode::Mesh(key)) = &self.weights.bind
            && !self.weights_mesh_active(key)
        {
            self.weights.bind = None;
        }
    }
    pub(super) fn begin_weights_bind(&mut self) {
        if self.weights.bind.take().is_some() {
            return;
        }
        self.cancel_gesture();
        self.reconcile_weights();
        let source = match self.selection.as_ref() {
            Some(TreeItem::Bone(bone)) => Some(BindMode::Bone(*bone)),
            _ => self.weights.mesh.as_ref().and_then(|key| {
                match self.session.document().attachment(key) {
                    Some(Attachment::Mesh(_)) => Some(BindMode::Mesh(key.clone())),
                    _ => None,
                }
            }),
        };
        self.weights.bind = source;
        if let Some(BindMode::Bone(bone)) = self.weights.bind {
            self.weights.bone = Some(bone);
        }
        self.show_weights = true;
        self.set_tool(Tool::Weights);
        self.status = match self.weights.bind {
            Some(BindMode::Mesh(_)) => {
                "Bind: click one bone in the Tree or viewport; Escape exits".into()
            }
            Some(BindMode::Bone(_)) => {
                "Bind: click a mesh in the Tree or viewport; Escape exits".into()
            }
            None => "Select a mesh or one bone to bind".into(),
        };
    }
    pub(super) fn weights_edit_target(&self) -> EditTarget {
        if self.setup_mode {
            EditTarget::Setup
        } else {
            EditTarget::Animate {
                animation: self.animation,
                time: self.time,
            }
        }
    }
    pub(super) fn weights_bind_selection(&mut self, item: &TreeItem) -> bool {
        let Some(mode) = self.weights.bind.clone() else {
            return false;
        };
        let pair = match (mode, item) {
            (BindMode::Mesh(key), TreeItem::Bone(bone)) => Some((key, *bone)),
            (BindMode::Bone(bone), TreeItem::Attachment(key))
                if self.weights_mesh_active(key)
                    && matches!(
                        self.session.document().attachment(key),
                        Some(Attachment::Mesh(_))
                    ) =>
            {
                Some((key.clone(), bone))
            }
            _ => None,
        };
        if let Some((key, bone)) = pair {
            self.weights.bind = None;
            self.cancel_gesture();
            self.session.commit_transaction();
            self.playing = false;
            let weighted = matches!(self.session.document().attachment(&key), Some(Attachment::Mesh(mesh)) if mesh.is_weighted());
            let result = if weighted {
                self.session
                    .add_mesh_bone_binding(key.clone(), bone, self.weights_edit_target())
            } else {
                self.session
                    .bind_mesh_to_bone(key.clone(), bone, self.weights_edit_target())
            };
            self.weights.select_mesh(key.clone());
            self.weights.bone = Some(bone);
            self.restore_selection(Some(TreeItem::Attachment(key)));
            match result {
                Ok(()) => {
                    let action = if weighted {
                        "Added zero-weight binding to"
                    } else {
                        "Bound mesh to"
                    };
                    self.status = format!(
                        "{action} {} at the current pose",
                        self.session.document().bones[bone].name
                    )
                }
                Err(error) => self.error = Some(error),
            }
            self.reconcile_selection();
        }
        true
    }
    pub(super) fn remove_weights_bindings(&mut self) {
        self.cancel_gesture();
        self.reconcile_weights();
        self.weights.bind = None;
        let Some(key) = self.weights.mesh.clone() else {
            return;
        };
        if !matches!(self.session.document().attachment(&key), Some(Attachment::Mesh(mesh)) if mesh.is_weighted())
        {
            return;
        }
        self.session.commit_transaction();
        self.playing = false;
        match self.session.unbind_mesh(key, self.weights_edit_target()) {
            Ok(()) => {
                self.weights.bone = None;
                self.status = "Removed all mesh bindings; current shape preserved".into();
            }
            Err(error) => self.error = Some(error),
        }
        self.reconcile_selection();
    }
    pub(super) fn weights_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        if !ctx.input(|input| input.focused)
            || self.error.is_some()
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
            || ctx.text_edit_focused()
            || egui::Popup::is_any_open(ctx)
        {
            return false;
        }
        let priority = self.weights.bind.is_some()
            || (self.tool == Tool::Weights
                && !self.dopesheet_has_focus(ctx)
                && (!ctx.egui_wants_keyboard_input()
                    || ctx.memory(|memory| memory.has_focus(Self::weights_id()))
                    || self.weight_control_has_focus(ctx)));
        if priority
            && [egui::Key::Space, egui::Key::Escape]
                .into_iter()
                .any(|key| ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key)))
        {
            self.cancel_gesture();
            if self.weights.bind.take().is_some() {
                self.status = "Bind mode ended".into();
            } else {
                self.weights.vertex = None;
                self.weights.bone = None;
            }
            return true;
        }
        false
    }
    pub(super) fn weights_diagnostic(&self) -> serde_json::Value {
        let scene = self.scene_for_view();
        let evaluation_error = scene.as_ref().err();
        let vertices = scene.as_ref().ok().and_then(|scene| {
            scene
                .attachments
                .iter()
                .find(|draw| Some(&draw.attachment) == self.weights.mesh.as_ref())
                .map(|draw| &draw.vertices)
        });
        serde_json::json!({"mesh":self.weights.mesh,"vertex":self.weights.vertex,"bone":self.weights.bone,
            "bindMode":match self.weights.bind {Some(BindMode::Mesh(_))=>Some("meshFirst"),Some(BindMode::Bone(_))=>Some("boneFirst"),None=>None},
            "worldVertices":vertices,"handles":if evaluation_error.is_some() { &[][..] } else { &self.weights_handles[..] },"evaluationError":evaluation_error,"visible":self.show_weights})
    }
    pub(super) fn weights_panel(&mut self, ui: &mut egui::Ui) {
        self.record_target("weightsPanel", ui.clip_rect());
        let focus = ui.interact(ui.max_rect(), Self::weights_id(), Sense::click());
        if focus.clicked() {
            focus.request_focus();
        }
        focus.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Weights view")
        });
        icons::heading(ui, icons::Icon::Weights, "Weights");
        self.reconcile_weights();
        let mesh_label = if matches!(self.weights.bind, Some(BindMode::Bone(_))) {
            "Mesh: click a mesh".into()
        } else {
            self.weights.mesh.as_ref().map_or_else(
                || "Mesh: select an active mesh".into(),
                |key| format!("Mesh: {}", key.placeholder),
            )
        };
        ui.label(mesh_label);
        let binding_bone = match self.weights.bind {
            Some(BindMode::Mesh(_)) => None,
            Some(BindMode::Bone(bone)) => Some(bone),
            None => self.weights.bone,
        };
        ui.label(binding_bone.map_or_else(
            || "Binding bone: select a bone".into(),
            |bone| format!("Binding bone: {}", self.session.document().bones[bone].name),
        ));
        ui.horizontal(|ui| {
            let can_bind = self.weights.bind.is_some() || self.selected_bone().is_some() || self.weights.mesh.as_ref().is_some_and(|key|
                matches!(self.session.document().attachment(key),Some(Attachment::Mesh(_))));
            let bind = ui.add_enabled(can_bind, egui::Button::selectable(self.weights.bind.is_some(), "Bind"))
                .on_hover_text("Bind an unweighted mesh to one bone, or add a new bone with zero weight to a weighted mesh. Click a bone or mesh after entering Bind mode.");
            self.record_target("weightsBind", bind.rect);
            if bind.clicked() { self.begin_weights_bind(); focus.request_focus(); }
            let can_remove = self.weights.bind.is_none() && self.weights.mesh.as_ref().is_some_and(|key|matches!(self.session.document().attachment(key),Some(Attachment::Mesh(mesh)) if mesh.is_weighted()));
            let remove = ui.add_enabled(can_remove, egui::Button::new("Remove all"))
                .on_hover_text("Remove every bound bone and keep the current mesh shape. Removing individual bones is not available yet.");
            self.record_target("weightsRemoveAll", remove.rect);
            if remove.clicked() { self.remove_weights_bindings(); focus.request_focus(); }
        });
        if self.weights.bind.is_some() {
            ui.label(RichText::new(&self.status).color(ACCENT));
        }
        let Some(key) = self.weights.mesh.clone() else {
            return;
        };
        let Some(Attachment::Mesh(mesh)) = self.session.document().attachment(&key) else {
            return;
        };
        ui.label(format!("{} vertices", mesh.vertex_count()));
        if let Some(vertex) = self.weights.vertex {
            ui.label(format!("Vertex {vertex}"));
        }
        let mut bones = Vec::new();
        let mut pose_bone = None;
        let mut picked_bone = None;
        if let MeshVertices::Weighted { influences } = &mesh.vertices {
            for influence in influences.iter().flatten() {
                if !bones.contains(&influence.bone) {
                    bones.push(influence.bone);
                }
            }
            for bone in bones {
                let weight = self.weights.vertex.map(|vertex| {
                    influences[vertex]
                        .iter()
                        .filter(|influence| influence.bone == bone)
                        .map(|influence| f64::from(influence.weight))
                        .sum::<f64>()
                });
                let row = ui
                    .horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(vec2(10., 10.), Sense::hover());
                        ui.painter().rect_filled(rect, 1., bone_color(bone));
                        ui.selectable_label(
                            self.weights.bone == Some(bone),
                            format!(
                                "{}   {}",
                                self.session.document().bones[bone].name,
                                weight.map_or_else(
                                    || "—".into(),
                                    |weight| format!("{:.2}%", weight * 100.)
                                )
                            ),
                        )
                    })
                    .inner;
                self.capture_targets.insert(
                    format!("weightsBone:{bone}"),
                    [
                        row.rect.min.x,
                        row.rect.min.y,
                        row.rect.max.x,
                        row.rect.max.y,
                    ],
                );
                if row.clicked() {
                    picked_bone = Some(bone);
                    focus.request_focus();
                }
                if row.secondary_clicked() {
                    pose_bone = Some(bone);
                }
            }
        } else {
            ui.label("Unweighted mesh");
        }
        if let Some(bone) = picked_bone {
            self.cancel_weight_edit();
            self.weights.bone = Some(bone);
        }
        if let Some(bone) = pose_bone {
            self.select(TreeItem::Bone(bone));
        }
        self.weight_controls(ui);
        ui.label(
            RichText::new(
                "Direct adjusts one vertex and one bone. Brush, Update bindings and individual removal are not available yet.",
            )
            .small()
            .color(MUTED),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{editor_context, mesh_editor};
    use super::*;

    fn world(editor: &Editor, key: &AttachmentRef) -> Vec<[f32; 2]> {
        scene_from_locals(
            editor.session.document(),
            &editor.skin,
            editor
                .session
                .local_pose(editor.active_animation(), editor.time),
        )
        .unwrap()
        .attachments
        .into_iter()
        .find(|draw| draw.attachment == *key)
        .unwrap()
        .vertices
    }
    fn same_shape(a: &[[f32; 2]], b: &[[f32; 2]]) {
        assert_eq!(a.len(), b.len());
        assert!(
            a.iter()
                .flatten()
                .zip(b.iter().flatten())
                .all(|(a, b)| (a - b).abs() < 0.001)
        );
    }
    #[test]
    fn retained_selection_prunes_stale_vertices_bones_and_bind_targets_without_redirecting() {
        let (editor, key) = mesh_editor(true);
        let document = editor.session.document();
        let mut state = WeightsState {
            mesh: Some(key.clone()),
            vertex: Some(usize::MAX),
            bone: Some(usize::MAX),
            bind: Some(BindMode::Bone(usize::MAX)),
        };
        state.reconcile(document);
        assert_eq!(state.mesh, Some(key.clone()));
        assert_eq!(state.vertex, None);
        assert_eq!(state.bone, None);
        assert_eq!(state.bind, None);
        state.bind = Some(BindMode::Mesh(key.clone()));
        state.reconcile(document);
        // A weighted mesh now remains a valid source for adding a genuinely new bone.
        assert_eq!(state.bind, Some(BindMode::Mesh(key.clone())));
        state.mesh = Some(AttachmentRef {
            placeholder: "missing mesh".into(),
            ..key
        });
        state.reconcile(document);
        assert!(state.mesh.is_none());
        assert!(state.vertex.is_none());
        assert!(state.bone.is_none());
    }
    fn chord(key: egui::Key) -> Vec<egui::Event> {
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            })
            .collect()
    }
    fn input(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>, text: bool) {
        let mut output = ctx.run_ui(
            egui::RawInput {
                events,
                focused: true,
                ..Default::default()
            },
            |ui| {
                editor.shortcuts(ui.ctx());
                editor.observe_timeline_input(ui.ctx());
                if text {
                    let mut value = String::new();
                    ui.add(egui::TextEdit::singleline(&mut value))
                        .request_focus();
                } else {
                    editor.weights_panel(ui);
                }
                editor.reconcile_selection();
            },
        );
        output.textures_delta.clear();
    }

    #[test]
    fn mesh_first_bind_and_remove_all_are_shape_preserving_single_undo_steps() {
        let (mut editor, key) = mesh_editor(false);
        editor.set_tool(Tool::Weights);
        let original = editor.session.document().clone();
        let shape = world(&editor, &key);
        editor.begin_weights_bind();
        assert!(matches!(editor.weights.bind, Some(BindMode::Mesh(_))));
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
        editor.select(TreeItem::Bone(7));
        assert!(editor.weights.bind.is_none());
        assert_eq!(editor.selected_attachment(), Some(&key));
        assert_eq!(editor.weights.bone, Some(7));
        let bound = editor.session.document().clone();
        let Some(Attachment::Mesh(mesh)) = bound.attachment(&key) else {
            unreachable!()
        };
        let MeshVertices::Weighted { influences } = &mesh.vertices else {
            unreachable!()
        };
        assert!(
            influences
                .iter()
                .all(|vertex| vertex.len() == 1 && vertex[0].bone == 7 && vertex[0].weight == 1.)
        );
        same_shape(&shape, &world(&editor, &key));
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
        editor.redo();
        assert_eq!(editor.session.document(), &bound);
        editor.remove_weights_bindings();
        assert!(!editor.selected_mesh_is_weighted());
        same_shape(&shape, &world(&editor, &key));
        editor.undo();
        assert_eq!(editor.session.document(), &bound);
        editor.redo();
        assert!(!editor.selected_mesh_is_weighted());
        assert_eq!(editor.weights.mesh, Some(key));
        assert!(editor.error.is_none());
    }

    #[test]
    fn bone_first_animate_bind_and_remove_keep_exact_pending_pose_time_and_keys() {
        let (mut editor, key) = mesh_editor(false);
        editor.set_setup_mode(false);
        editor.set_time(0.4);
        editor.select(TreeItem::Bone(7));
        let mut pose = editor.selected_transform().unwrap();
        pose.rotation += 13.;
        editor.edit_transform(pose, &[Property::Rotation]);
        let original = editor.session.document().clone();
        let pending = editor.session.pose_edits().clone();
        let locals = editor.session.local_pose(Some(0), editor.time);
        let shape = world(&editor, &key);
        editor.begin_weights_bind();
        assert_eq!(editor.weights.bind, Some(BindMode::Bone(7)));
        assert!(editor.weights.mesh.is_none());
        editor.select(TreeItem::Attachment(key.clone()));
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(editor.time, 0.4);
        assert!(!editor.setup_mode);
        assert_eq!(editor.session.document().animations, original.animations);
        assert_eq!(editor.session.local_pose(Some(0), editor.time), locals);
        same_shape(&shape, &world(&editor, &key));
        let bound = editor.session.document().clone();
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(editor.time, 0.4);
        editor.redo();
        editor.weights.vertex = Some(2);
        editor.set_tool(Tool::Rotate);
        editor.select(TreeItem::Bone(8));
        assert_eq!(editor.weights.mesh, Some(key.clone()));
        assert_eq!(editor.weights.vertex, Some(2));
        editor.playing = true;
        editor.remove_weights_bindings();
        assert!(!editor.playing);
        assert_eq!(editor.selected_bone(), Some(8));
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(editor.time, 0.4);
        assert_eq!(editor.session.local_pose(Some(0), editor.time), locals);
        same_shape(&shape, &world(&editor, &key));
        editor.undo();
        assert_eq!(editor.session.document(), &bound);
        assert_eq!(editor.session.pose_edits(), &pending);
    }

    #[test]
    fn failed_concrete_binding_exits_without_retry_or_changing_pose_document_or_redo() {
        let (mut editor, key) = mesh_editor(false);
        let mut document = editor.session.document().clone();
        document.bones[7].setup.scale_x = 0.;
        editor.session = Session::new(document).unwrap();
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "redo".into(),
            })
            .unwrap();
        editor.session.undo();
        editor.set_tool(Tool::Weights);
        let original = editor.session.document().clone();
        editor.begin_weights_bind();
        editor.select(TreeItem::Bone(7));
        assert!(editor.error.is_some());
        assert!(editor.weights.bind.is_none());
        assert_eq!(editor.weights.mesh, Some(key));
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
        assert!(!editor.session.can_undo());
        editor.select(TreeItem::Bone(6));
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
    }

    #[test]
    fn selection_history_restore_cancels_bind_before_replacing_the_displayed_mesh() {
        let (mut editor, first) = mesh_editor(false);
        let second = editor
            .session
            .document()
            .resolved_attachment(&editor.skin, 1)
            .unwrap()
            .0;
        let document = peachy_engine::apply_command(
            editor.session.document(),
            Command::ConvertRegionToMesh {
                attachment: second.clone(),
            },
        )
        .unwrap();
        editor.session = Session::new(document).unwrap();
        editor.set_tool(Tool::Weights);
        editor.begin_weights_bind();
        assert_eq!(editor.weights.bind, Some(BindMode::Mesh(first)));
        editor.restore_selection(Some(TreeItem::Attachment(second.clone())));
        assert!(editor.weights.bind.is_none());
        assert_eq!(editor.weights.mesh, Some(second));
        let original = editor.session.document().clone();
        editor.select(TreeItem::Bone(7));
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
        assert_eq!(editor.selected_bone(), Some(7));
    }

    #[test]
    fn active_appearance_reconciliation_keeps_default_fallback_but_never_redirects_edits() {
        let (mut editor, key) = mesh_editor(true);
        editor.set_tool(Tool::Weights);
        editor.weights.vertex = Some(4);
        editor.weights.bone = Some(8);
        editor.reconcile_weights();
        assert_eq!(editor.weights.vertex, None);
        editor.skin = "mint".into();
        editor.reconcile_weights();
        assert_eq!(editor.weights.mesh, Some(key.clone()));
        let mut document = editor.session.document().clone();
        let override_mesh = document.attachment(&key).unwrap().clone();
        document
            .skins
            .get_mut("mint")
            .unwrap()
            .entry(key.slot.clone())
            .or_default()
            .insert(key.placeholder.clone(), override_mesh);
        editor.session = Session::new(document).unwrap();
        editor.reconcile_selection();
        assert!(editor.weights.mesh.is_none());
        let original = editor.session.document().clone();
        editor.capture_weights_mesh(&key);
        editor.remove_weights_bindings();
        assert_eq!(editor.session.document(), &original);
        let active = AttachmentRef {
            skin: "mint".into(),
            ..key.clone()
        };
        editor.select(TreeItem::Attachment(active));
        assert!(editor.weights.mesh.is_some());
        editor
            .session
            .dispatch(Command::SetSlotAttachment {
                slot: 0,
                attachment: None,
            })
            .unwrap();
        editor.reconcile_selection();
        assert!(editor.weights.mesh.is_none());
        editor.undo();
        assert!(editor.weights.mesh.is_none());
    }

    #[test]
    fn weights_shortcuts_respect_text_modals_exit_bind_and_retain_tool_selection() {
        let (mut editor, key) = mesh_editor(false);
        let ctx = editor_context();
        input(&mut editor, &ctx, chord(egui::Key::G), false);
        assert!(editor.tool == Tool::Weights && editor.show_weights);
        editor.weights.vertex = Some(2);
        editor.weights.bone = Some(7);
        input(&mut editor, &ctx, chord(egui::Key::C), false);
        assert!(editor.tool == Tool::Rotate);
        editor.toggle_transform_tool();
        assert!(editor.tool == Tool::Weights);
        assert_eq!(editor.weights.mesh, Some(key));
        assert_eq!(editor.weights.vertex, Some(2));
        assert_eq!(editor.weights.bone, Some(7));
        editor.begin_weights_bind();
        let original = editor.session.document().clone();
        input(&mut editor, &ctx, chord(egui::Key::Escape), false);
        assert!(editor.weights.bind.is_none());
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
        editor.begin_weights_bind();
        input(&mut editor, &ctx, chord(egui::Key::Space), false);
        assert!(editor.weights.bind.is_none());
        assert_eq!(editor.session.document(), &original);
        editor.set_tool(Tool::Rotate);
        input(&mut editor, &ctx, vec![], true);
        input(&mut editor, &ctx, chord(egui::Key::G), true);
        assert!(editor.tool == Tool::Rotate);
        editor.error = Some("modal".into());
        input(&mut editor, &ctx, chord(egui::Key::G), false);
        assert!(editor.tool == Tool::Rotate);
        editor.error = None;
        editor.show_tree = true;
        editor.show_dopesheet = false;
        editor.show_weights = true;
        editor.toggle_views();
        assert!(!editor.show_tree && !editor.show_dopesheet && !editor.show_weights);
        editor.toggle_views();
        assert!(editor.show_tree && !editor.show_dopesheet && editor.show_weights);
    }

    #[test]
    fn focused_weights_view_allows_transform_history_save_and_hidden_workspace_reopening() {
        for (key, tool) in [
            (egui::Key::C, Tool::Rotate),
            (egui::Key::V, Tool::Move),
            (egui::Key::X, Tool::Scale),
        ] {
            let (mut editor, _) = mesh_editor(true);
            let ctx = editor_context();
            input(&mut editor, &ctx, chord(egui::Key::G), false);
            input(&mut editor, &ctx, chord(key), false);
            assert!(editor.tool == tool);
            editor.toggle_transform_tool();
            assert!(editor.tool == Tool::Weights);
        }
        let (mut editor, _) = mesh_editor(true);
        let ctx = editor_context();
        editor.show_weights = false;
        editor.toggle_views();
        input(&mut editor, &ctx, chord(egui::Key::G), false);
        assert!(
            editor.show_tree
                && editor.show_dopesheet
                && editor.show_weights
                && editor.hidden_views.is_none()
        );
        let original = editor.session.document().clone();
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "focus-check".into(),
            })
            .unwrap();
        let modified = editor.session.document().clone();
        let command = |key| {
            [true, false]
                .into_iter()
                .map(|pressed| egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                })
                .collect()
        };
        input(&mut editor, &ctx, command(egui::Key::Z), false);
        assert_eq!(editor.session.document(), &original);
        editor.redo();
        let folder = std::env::temp_dir().join(format!(
            "peachy-weights-focus-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&folder).unwrap();
        editor.project_path = Some(folder.join("rig.peachy"));
        editor.asset_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
        input(&mut editor, &ctx, command(egui::Key::S), false);
        let loaded = files::load(&folder.join("rig.peachy"));
        std::fs::remove_dir_all(&folder).unwrap();
        assert_eq!(loaded.unwrap(), modified);
        assert!(!editor.session.is_dirty());
    }

    #[test]
    fn bind_mode_cancels_on_context_history_and_visibility_changes_but_scrub_uses_live_pose() {
        let (mut editor, key) = mesh_editor(false);
        editor.set_tool(Tool::Weights);
        editor.weights.vertex = Some(2);
        editor.weights.bone = Some(7);
        editor.begin_weights_bind();
        editor.set_time(0.4);
        assert!(editor.weights.bind.is_some());
        editor.set_setup_mode(false);
        assert!(editor.weights.bind.is_none());
        editor.begin_weights_bind();
        editor.activate_animation(1);
        assert!(editor.weights.bind.is_none());
        editor.begin_weights_bind();
        editor.undo();
        assert!(editor.weights.bind.is_none());
        editor.begin_weights_bind();
        editor.redo();
        assert!(editor.weights.bind.is_none());
        editor.begin_weights_bind();
        editor.toggle_views();
        assert!(editor.weights.bind.is_none());
        editor.toggle_views();
        assert_eq!(editor.weights.mesh, Some(key.clone()));
        assert_eq!(editor.weights.vertex, Some(2));
        assert_eq!(editor.weights.bone, Some(7));
        editor.begin_weights_bind();
        editor.set_time(0.7);
        editor.select(TreeItem::Bone(7));
        assert!(editor.error.is_none());
        assert_eq!(editor.time, 0.7);
        let ctx = editor_context();
        editor.set_tool(Tool::Rotate);
        editor.select(TreeItem::Bone(8));
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.set_min_size(vec2(1000., 700.));
            editor.canvas(ui);
        });
        output.textures_delta.clear();
        assert_eq!(editor.selected_bone(), Some(8));
        assert_eq!(editor.weights_handles.len(), 4);
        assert_eq!(
            editor.weights_diagnostic()["worldVertices"],
            serde_json::to_value(world(&editor, &key)).unwrap()
        );
    }
}
