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
                    let restore = icons::button(
                        ui,
                        Icon::RestoreWorkspace,
                        "Restore default workspace",
                        None,
                    );
                    self.record_target("viewRestore", restore.rect);
                    if restore.clicked() {
                        self.show_tree = true;
                        self.show_dopesheet = true;
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
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                self.transform_row(ui, "Rotate  [C]", &[Property::Rotation], Some(Tool::Rotate));
                self.transform_row(
                    ui,
                    "Translate  [V]",
                    &[Property::X, Property::Y],
                    Some(Tool::Move),
                );
                self.transform_row(
                    ui,
                    "Scale  [X]",
                    &[Property::ScaleX, Property::ScaleY],
                    Some(Tool::Scale),
                );
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
                if !self.setup_mode {
                    let auto_key = ui.checkbox(&mut self.auto_key, "Auto Key");
                    self.record_target("autoKey", auto_key.rect);
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
        let editable = self.transform_editable();
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
                let target = match tool { Tool::Rotate=>"rotateTool",Tool::Move=>"translateTool",Tool::Scale=>"scaleTool",Tool::Mesh=>"meshTool" };
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
                for &property in properties {
                    let mut value = property.read(if property.is_scale(){local}else{shown});
                    if property != Property::Rotation {
                        ui.label(if matches!(property, Property::X | Property::ScaleX) {
                            "X"
                        } else {
                            "Y"
                        });
                    }
                    let id=ui.next_auto_id();
                    // egui may parse on two focus-loss frames. Keep relative entry idempotent.
                    let entry_id=id.with("entry-base");
                    let focused=ui.memory(|m|m.has_focus(id));
                    let base=ui.data_mut(|data| {
                        let (previous,was_focused,committed)=data.get_temp::<(f32,bool,Option<f32>)>(entry_id).unwrap_or((value,false,None));
                        let new_entry=focused && !was_focused;
                        let base=if new_entry {value}else{previous};
                        if focused || data.get_temp::<(f32,bool,Option<f32>)>(entry_id).is_some() {data.insert_temp(entry_id,(base,focused,if new_entry {None}else{committed}));}
                        base
                    });
                    let local_scale_edit=ui.memory(|m|m.has_focus(id)) || ui.ctx().is_being_dragged(id);
                    let world_scale=property.is_scale() && self.axes==Axes::World && !local_scale_edit;
                    let display=property.read(shown);
                    let response = ui.add(
                        egui::DragValue::new(&mut value)
                            .speed(if property.is_scale() { 0.01 } else { 0.25 })
                            .max_decimals(2)
                            .update_while_editing(false)
                            .custom_parser(move |text| peachy_engine::parse_transform_input(text,base).map(f64::from))
                            .custom_formatter(move |v,_| format!("{:.2}",if world_scale{f64::from(display)}else{v})),
                    );
                    self.record_target(&format!("value:{property:?}"), response.rect);
                    if response.clicked(){self.playing=false;}
                    if property.is_scale() && self.axes==Axes::World { response.clone().on_hover_text("World scale is shown; entry and dragging edit Local scale."); }
                    if response.drag_started() {
                        ui.data_mut(|data|data.insert_temp(entry_id,(base,focused,None::<f32>)));
                        self.session.begin_transaction();
                    }
                    if response.changed() && ui.data(|data|data.get_temp::<(f32,bool,Option<f32>)>(entry_id).and_then(|entry|entry.2))!=Some(value) {
                        self.edit_display_value(property,value);
                        ui.data_mut(|data|data.insert_temp(entry_id,(base,focused,Some(value))));
                    }
                    if response.drag_stopped() {
                        self.session.commit_transaction();
                    }
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
                    if response.drag_started() {
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
                self.playing = false;
                self.dispatch(Command::SetKey {
                    animation: self.animation,
                    bone,
                    property,
                    key: Keyframe { curve, ..key },
                });
            }
            if ui.small_button("Remove key").clicked() {
                self.dispatch(Command::DeleteKey {
                    animation: self.animation,
                    bone,
                    property,
                    time: key.time,
                });
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
