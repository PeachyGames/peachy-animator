use super::icons::Icon;
use super::*;

impl Editor {
    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        let command = if cfg!(target_os = "macos") {
            "Cmd"
        } else {
            "Ctrl"
        };
        ui.horizontal(|ui| {
            let menu = ui.menu_button(RichText::new("Peachy").strong().color(ACCENT), |ui| {
                let open = icons::button(
                    ui,
                    Icon::Open,
                    "Open Project…",
                    Some(&format!("{command}+O")),
                );
                self.record_target("fileOpen", open.rect);
                if open.clicked() {
                    ui.close();
                    self.open_project(ui.ctx());
                }
                let save = icons::button(ui, Icon::Save, "Save", Some(&format!("{command}+S")));
                self.record_target("fileSave", save.rect);
                if save.clicked() {
                    ui.close();
                    self.save_project();
                }
                let export =
                    icons::button(ui, Icon::Export, "Export…", Some(&format!("{command}+E")));
                self.record_target("fileExport", export.rect);
                if export.clicked() {
                    ui.close();
                    self.export();
                }
            });
            self.record_target("fileMenu", menu.response.rect);
            let open = icons::toolbar_button(
                ui,
                Icon::Open,
                "Open Project",
                &format!("{command}+O"),
                true,
            );
            self.record_target("toolbarOpen", open.rect);
            if open.clicked() {
                self.open_project(ui.ctx());
            }
            let save = icons::toolbar_button(ui, Icon::Save, "Save", &format!("{command}+S"), true);
            self.record_target("toolbarSave", save.rect);
            if save.clicked() {
                self.save_project();
            }
            let undo = icons::toolbar_button(
                ui,
                Icon::Undo,
                "Undo",
                &format!("{command}+Z"),
                self.session.can_undo(),
            );
            self.record_target("toolbarUndo", undo.rect);
            if undo.clicked() {
                self.undo();
            }
            let redo = icons::toolbar_button(
                ui,
                Icon::Redo,
                "Redo",
                &format!("{command}+Shift+Z"),
                self.session.can_redo(),
            );
            self.record_target("toolbarRedo", redo.rect);
            if redo.clicked() {
                self.redo();
            }
            ui.separator();
            ui.label(
                RichText::new(format!(
                    "{}{}",
                    if self.session.is_dirty() { "* " } else { "" },
                    self.session.document().name
                ))
                .color(MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let views = icons::menu_button(ui, Icon::Views, "Views", |ui| {
                    let tree = icons::toggle(ui, Icon::Tree, "Tree", &mut self.show_tree);
                    self.record_target("viewTree", tree.rect);
                    ui.add_enabled_ui(!self.setup_mode, |ui| {
                        let dopesheet = icons::toggle(
                            ui,
                            Icon::Dopesheet,
                            "Dopesheet",
                            &mut self.show_dopesheet,
                        );
                        self.record_target("viewDopesheet", dopesheet.rect);
                    });
                    ui.separator();
                    let weights =
                        icons::toggle(ui, Icon::Weights, "Weights", &mut self.show_weights);
                    self.record_target("viewWeights", weights.rect);
                    if weights.changed() {
                        self.cancel_weight_edit();
                        if self.show_weights {
                            self.open_weights_view();
                        } else {
                            self.weights.bind = None;
                        }
                    }
                    ui.separator();
                    let restore = icons::button(
                        ui,
                        Icon::RestoreWorkspace,
                        "Restore default workspace",
                        None,
                    );
                    self.record_target("viewRestore", restore.rect);
                    if restore.clicked() {
                        self.cancel_weight_edit();
                        self.show_tree = true;
                        self.show_dopesheet = true;
                        self.show_weights = false;
                        self.weights.bind = None;
                        self.hidden_views = None;
                        ui.close();
                    }
                });
                self.record_target("viewsMenu", views.rect);
                let hidden = self.hidden_views.is_some();
                let toggle = icons::button(
                    ui,
                    if hidden {
                        Icon::RestoreViews
                    } else {
                        Icon::MinimizeViews
                    },
                    if hidden {
                        "Restore views"
                    } else {
                        "Hide views"
                    },
                    None,
                )
                .on_hover_text("Minimize/restore views · F9");
                self.record_target("hideViews", toggle.rect);
                if toggle.clicked() {
                    self.toggle_views();
                }
            });
        });
    }

    pub(super) fn main_tools(&mut self, ui: &mut egui::Ui) {
        self.record_target("mainTools", ui.max_rect());
        self.record_target("mainToolsClip", ui.clip_rect());
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                for (tool, label, properties) in layout::TRANSFORM_ROWS {
                    self.transform_row(ui, label, properties, Some(tool));
                }
            });
            ui.separator();
            ui.vertical(|ui| {
                for (axes, label) in [
                    (Axes::Local, "Local"),
                    (Axes::Parent, "Parent"),
                    (Axes::World, "World"),
                ] {
                    let r = ui.selectable_label(self.axes == axes, label);
                    self.record_target(&format!("axes:{label}"), r.rect);
                    if r.clicked() {
                        self.set_axes(axes);
                    }
                }
            });
            ui.separator();
            ui.vertical(|ui| {
                if let Some(item) = &self.selection {
                    ui.label(RichText::new(item.label(self.session.document())).color(ACCENT));
                } else {
                    ui.label(RichText::new("No selection").color(MUTED));
                }
                let weights = icons::tool_button(
                    ui,
                    icons::Icon::Weights,
                    "Weights [G]",
                    self.tool == Tool::Weights,
                )
                .on_hover_text("Weights tool · G");
                self.record_target("weightsTool", weights.rect);
                if weights.clicked() {
                    self.show_weights = true;
                    self.set_tool(Tool::Weights);
                }
                if !self.setup_mode {
                    let auto_key = ui.checkbox(&mut self.auto_key, "Auto Key");
                    self.record_target("autoKey", auto_key.rect);
                    if auto_key.changed() {
                        self.cancel_gesture();
                    }
                    auto_key.on_hover_text("Set keys automatically when a transform changes.");
                    let key = ui.add_enabled(
                        !self.session.pose_edits().is_empty(),
                        egui::Button::new("Key Edited  [K]").small(),
                    );
                    self.record_target("keyEdited", key.rect);
                    if key.clicked() {
                        self.key_edited(None);
                    }
                }
            });
        });
    }

    fn transform_row(
        &mut self,
        ui: &mut egui::Ui,
        label: &str,
        properties: &[Property],
        tool: Option<Tool>,
    ) {
        let local = self.selected_transform().unwrap_or_default();
        let parent = self
            .selected_parent()
            .unwrap_or_else(|| Affine::from_transform(Transform::default()));
        let mut shown = peachy_engine::transform_values(local, parent, self.axes);
        if self.setup_mode {
            shown.rotation = shown.rotation.rem_euclid(360.);
        }
        let editable = tool.map_or_else(
            || self.transform_editable(),
            |tool| self.transform_tool_editable(tool),
        );
        let scope = (
            self.selection.as_ref().map(TreeItem::key),
            format!("{:?}", self.axes),
            self.setup_mode,
            self.animation,
            self.pose_view_generation,
            label,
        );
        ui.push_id(scope, |ui| { ui.horizontal(|ui| {
            if let Some(tool) = tool {
                let response = ui.add_sized(
                    [112.0, 22.0],
                    egui::Button::selectable(self.tool == tool, label),
                );
                let target = match tool { Tool::Rotate=>"rotateTool",Tool::Move=>"translateTool",Tool::Scale=>"scaleTool",Tool::Shear=>"shearTool",Tool::Mesh=>"meshTool",Tool::Weights=>"weightsTool" };
                self.capture_targets.insert(
                    target.into(),
                    [
                        response.rect.min.x,
                        response.rect.min.y,
                        response.rect.max.x,
                        response.rect.max.y,
                    ],
                );
                if response.clicked() {
                    self.set_tool(tool);
                }
            } else {
                ui.add_sized([112.0, 22.0], egui::Label::new(label));
            }
            ui.add_enabled_ui(editable, |ui| {
                if tool == Some(Tool::Shear) { ui.label(RichText::new("Local").small().color(MUTED)).on_hover_text("Shear angles are edited in local coordinates."); }
                for &property in properties {
                    let current = property.read(if property.is_scale() || matches!(property, Property::ShearX | Property::ShearY) {local}else{shown});
                    if property != Property::Rotation {
                        ui.label(if matches!(property, Property::X | Property::ScaleX | Property::ShearX) {"X"} else {"Y"});
                    }
                    let id = ui.next_auto_id();
                    self.prepare_transform_numeric(ui, id);
                    let (mut value, base) = self.transform_numeric_values(id, current);
                    let focused = ui.memory(|memory|memory.has_focus(id));
                    let world_scale = property.is_scale() && self.axes==Axes::World && !focused && !ui.ctx().is_being_dragged(id);
                    let display = property.read(shown);
                    let response = ui.add(egui::DragValue::new(&mut value)
                        .speed(if property.is_scale() {0.01} else {0.25})
                        .max_decimals(2).update_while_editing(false)
                        .custom_parser(move |text|peachy_engine::parse_transform_input(text,base).map(f64::from))
                        .custom_formatter(move |value,_|if focused {value.to_string()}else{format!("{:.2}",if world_scale {f64::from(display)}else{value})}));
                    self.record_target(&format!("value:{property:?}"),response.rect);
                    if matches!(property,Property::ShearX|Property::ShearY) {response.clone().on_hover_text("Shear angles are edited in local coordinates.");}
                    if property.is_scale() && self.axes==Axes::World {response.clone().on_hover_text("World scale is shown; entry and dragging edit Local scale.");}
                    self.transform_numeric_response(&response,property,value);
                }
                if !self.setup_mode {
                    let state = self.selected_bone().map_or(KeyState::Absent, |b| self.key_state(b, properties));
                    let key = super::animation::pose_key_button(ui, state);
                    self.record_target(&format!("key:{:?}", properties[0]), key.rect);
                    if key.on_hover_text("Green: no key · Orange: unkeyed change · Red: keyed\nClick to key this transform; L keys the active tool.").clicked() {
                        self.key_transform(properties);
                    }
                }
            });
        }); });
    }

    fn exact_inspected_key(&self, bone: usize, property: Property, time: f32) -> Option<Keyframe> {
        self.session
            .document()
            .animations
            .get(self.animation)?
            .tracks
            .iter()
            .find(|track| track.bone == bone && track.property == property)?
            .keys
            .iter()
            .find(|key| key.time == time)
            .copied()
    }
    pub(super) fn set_key_curve(
        &mut self,
        bone: usize,
        property: Property,
        time: f32,
        curve: Curve,
    ) {
        self.cancel_gesture();
        // A live Auto Key preview is not the source of the inspected key's value/identity.
        let Some(key) = self.exact_inspected_key(bone, property, time) else {
            return;
        };
        if key.curve != curve {
            self.playing = false;
            self.dispatch(Command::SetKey {
                animation: self.animation,
                bone,
                property,
                key: Keyframe { curve, ..key },
            });
        }
    }
    pub(super) fn remove_inspected_key(&mut self, bone: usize, property: Property, time: f32) {
        self.cancel_gesture();
        if self.exact_inspected_key(bone, property, time).is_some() {
            self.dispatch(Command::DeleteKey {
                animation: self.animation,
                bone,
                property,
                time,
            });
        }
    }
    pub(super) fn bone_curves(&mut self, ui: &mut egui::Ui, bone: usize) {
        ui.add_space(10.0);
        ui.separator();
        ui.label(RichText::new("Key interpolation").small().color(MUTED));
        let time = self.time;
        let mut ambiguous = false;
        let keys: Vec<_> = self
            .session
            .document()
            .animations
            .get(self.animation)
            .into_iter()
            .flat_map(|a| a.tracks.iter())
            .filter(|t| t.bone == bone)
            .filter_map(
                |track| match peachy_engine::keyframe_at(&track.keys, time) {
                    Ok(key) => key.map(|key| (track.property, key)),
                    Err(_) => {
                        ambiguous = true;
                        None
                    }
                },
            )
            .collect();
        let mut found = false;
        for (property, key) in keys {
            found = true;
            let mut curve = key.curve;
            ui.label(property.label());
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(curve == Curve::Linear, "Linear")
                    .clicked()
                {
                    curve = Curve::Linear;
                }
                if ui
                    .selectable_label(curve == Curve::Stepped, "Stepped")
                    .clicked()
                {
                    curve = Curve::Stepped;
                }
                if ui
                    .selectable_label(matches!(curve, Curve::Bezier(_)), "Bezier")
                    .clicked()
                {
                    curve = Curve::Bezier([0.42, 0.0, 0.58, 1.0]);
                }
            });
            if let Curve::Bezier(ref mut handles) = curve {
                for (i, label) in ["Time 1", "Value 1", "Time 2", "Value 2"]
                    .iter()
                    .enumerate()
                {
                    let range = if i % 2 == 0 { 0.0..=1.0 } else { -1.0..=2.0 };
                    let response = ui.add(egui::Slider::new(&mut handles[i], range).text(*label));
                    if response.gained_focus() {
                        self.cancel_transform_edit();
                    }
                    if response.drag_started() {
                        self.cancel_transform_edit();
                        self.cancel_weight_edit();
                        self.session.begin_transaction();
                    }
                }
                if handles[0] > handles[2] {
                    handles.swap(0, 2);
                }
                let (r, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 66.0), Sense::hover());
                let points: Vec<_> = (0..=40)
                    .map(|i| {
                        let t = i as f32 / 40.0;
                        pos2(
                            r.left() + r.width() * t,
                            r.bottom() - r.height() * peachy_engine::curve_progress(curve, t),
                        )
                    })
                    .collect();
                ui.painter()
                    .rect_filled(r, 4.0, Color32::from_rgb(21, 26, 33));
                ui.painter()
                    .add(egui::Shape::line(points, Stroke::new(2.0, MINT)));
            }
            if curve != key.curve {
                self.set_key_curve(bone, property, key.time, curve);
            }
            if ui.small_button("Remove key").clicked() {
                self.remove_inspected_key(bone, property, key.time);
            }
        }
        if ambiguous {
            ui.label(
                RichText::new("Select an exact key to edit its interpolation.")
                    .small()
                    .color(MUTED),
            );
        }
        if !found && !ambiguous {
            ui.label(
                RichText::new("Select a key in the Dopesheet to edit its interpolation.")
                    .color(MUTED),
            );
        }
    }
}

#[cfg(test)]
mod curve_preview_tests {
    use super::super::test_support::editor_fixture;
    use super::*;

    fn preview(editor: &mut Editor) {
        editor.set_tool(Tool::Shear);
        editor.auto_key = true;
        let local = editor.selected_transform().unwrap();
        let parent = editor.selected_parent().unwrap();
        editor.begin_viewport_transform(parent.point(local.x + 40., local.y), Some(0));
        editor.update_viewport_transform(
            parent.point(
                local.x + 40. * 20f32.to_radians().cos(),
                local.y + 40. * 20f32.to_radians().sin(),
            ),
            false,
        );
    }
    #[test]
    fn editing_a_curve_uses_the_original_key_value_and_groups_after_preview_cancellation() {
        let mut editor = editor_fixture();
        editor.set_time(0.4);
        let mut document = editor.session.document().clone();
        document.animations[0].tracks.push(Track {
            bone: 8,
            property: Property::ShearX,
            keys: vec![Keyframe {
                time: 0.4,
                value: 3.,
                curve: Curve::Linear,
            }],
        });
        editor.session = Session::new(document).unwrap();
        let original = editor.session.document().clone();
        preview(&mut editor);
        assert!((editor.selected_transform().unwrap().shear_x - 23.).abs() < 0.001);
        editor.set_key_curve(8, Property::ShearX, 0.4, Curve::Stepped);
        assert!(editor.drag.is_none());
        let key = editor
            .exact_inspected_key(8, Property::ShearX, 0.4)
            .unwrap();
        assert_eq!(key.value, 3.);
        assert_eq!(key.curve, Curve::Stepped);
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
        preview(&mut editor);
        editor.cancel_transform_edit();
        editor.session.begin_transaction();
        editor.set_key_curve(
            8,
            Property::ShearX,
            0.4,
            Curve::Bezier([0.3, 0.2, 0.7, 0.8]),
        );
        editor.set_key_curve(
            8,
            Property::ShearX,
            0.4,
            Curve::Bezier([0.4, 0.2, 0.7, 0.8]),
        );
        editor.session.commit_transaction();
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
    }
    #[test]
    fn preview_only_curve_and_delete_actions_cannot_create_or_retarget_a_nearby_key() {
        let mut editor = editor_fixture();
        editor.set_time(0.4);
        let mut document = editor.session.document().clone();
        document.animations[0].tracks.push(Track {
            bone: 8,
            property: Property::ShearX,
            keys: vec![Keyframe {
                time: 0.400005,
                value: 7.,
                curve: Curve::Linear,
            }],
        });
        editor.session = Session::new(document).unwrap();
        let original = editor.session.document().clone();
        // AutoKey's existing SetPose near-key policy can edit the nearby key. The inspector's
        // cancellation/requery must still use the original exact identity/value, never its preview.
        preview(&mut editor);
        editor.set_key_curve(8, Property::ShearY, 0.4, Curve::Stepped);
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
        editor.transform_blocked = false;
        preview(&mut editor);
        editor.remove_inspected_key(8, Property::ShearY, 0.4);
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
        editor.set_key_curve(8, Property::ShearX, 0.4, Curve::Stepped);
        editor.remove_inspected_key(8, Property::ShearX, 0.4);
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
    }
}
