use super::*;

impl Editor {
    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button(RichText::new("Peachy").strong().color(ACCENT), |ui| {
                if ui.button("Open Project…    Cmd/Ctrl+O").clicked() {
                    ui.close();
                    self.open_project(ui.ctx());
                }
                if ui.button("Save    Cmd/Ctrl+S").clicked() {
                    ui.close();
                    self.save_project();
                }
                if ui.button("Export…    Cmd/Ctrl+E").clicked() {
                    ui.close();
                    self.export();
                }
            });
            if ui.small_button("Open").clicked() {
                self.open_project(ui.ctx());
            }
            if ui.small_button("Save").clicked() {
                self.save_project();
            }
            if ui
                .add_enabled(self.session.can_undo(), egui::Button::new("Undo").small())
                .clicked()
            {
                self.undo();
            }
            if ui
                .add_enabled(self.session.can_redo(), egui::Button::new("Redo").small())
                .clicked()
            {
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
                ui.menu_button("Views", |ui| {
                    ui.checkbox(&mut self.show_tree, "Tree");
                    ui.add_enabled_ui(!self.setup_mode, |ui| {
                        ui.checkbox(&mut self.show_dopesheet, "Dopesheet");
                    });
                    if ui.button("Restore default workspace").clicked() {
                        self.show_tree = true;
                        self.show_dopesheet = true;
                        self.hidden_views = None;
                        ui.close();
                    }
                });
                if ui
                    .small_button("Hide views")
                    .on_hover_text("Minimize/restore views · F9")
                    .clicked()
                {
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
        let time = self.snapped_time();
        let keys: Vec<_> = self
            .session
            .document()
            .animations
            .get(self.animation)
            .into_iter()
            .flat_map(|a| a.tracks.iter())
            .filter(|t| t.bone == bone)
            .filter_map(|track| {
                track
                    .keys
                    .iter()
                    .find(|k| same_key_time(k.time, time))
                    .map(|key| (track.property, *key))
            })
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
                    time,
                });
            }
        }
        if !found {
            ui.label(
                RichText::new("Select a diamond on the timeline to edit its curve.").color(MUTED),
            );
        }
    }
    pub(super) fn timeline(&mut self, ui: &mut egui::Ui) {
        self.record_target("dopesheet", ui.max_rect());
        ui.horizontal(|ui| {
            ui.label(RichText::new("Dopesheet").small().color(MUTED));
            for (direction, name) in [
                (PlayDirection::Backward, "playBackward"),
                (PlayDirection::Forward, "playForward"),
            ] {
                let active = self.playing && self.play_direction == direction;
                let response = transport_button(ui, direction, active);
                self.record_target(name, response.rect);
                if response
                    .on_hover_text(if direction == PlayDirection::Forward {
                        "Play forward/stop · D"
                    } else {
                        "Play backward/stop · A"
                    })
                    .clicked()
                {
                    self.toggle_playback(direction);
                }
            }
            if ui.small_button("|◀").clicked() {
                self.set_time(0.0);
                self.playing = false;
            }
            egui::ComboBox::from_id_salt("animation")
                .selected_text(
                    self.session
                        .document()
                        .animations
                        .get(self.animation)
                        .map_or("No animation", |a| a.name.as_str()),
                )
                .show_ui(ui, |ui| {
                    let names: Vec<_> = self
                        .session
                        .document()
                        .animations
                        .iter()
                        .map(|a| a.name.clone())
                        .collect();
                    for (i, name) in names.iter().enumerate() {
                        if ui.selectable_label(self.animation == i, name).clicked() {
                            self.activate_animation(i);
                        }
                    }
                });
            ui.checkbox(&mut self.looping, "Loop");
            ui.label(
                RichText::new(format!(
                    "{:03} / {:03}  ·  {} fps",
                    (self.time * self.session.document().fps as f32).round() as u32,
                    (self.duration() * self.session.document().fps as f32).round() as u32,
                    self.session.document().fps
                ))
                .monospace()
                .color(ACCENT),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new("A / D: play · W / S: keys · R / F: frames")
                        .small()
                        .color(MUTED),
                );
            });
        });
        ui.add_space(5.0);
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), 157.0), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 5.0, Color32::from_rgb(21, 26, 33));
        let label_width = 160.0;
        let start = rect.left() + label_width;
        let width = rect.width() - label_width - 18.0;
        let duration = self.duration();
        self.record_target(
            "timelineTrack",
            egui::Rect::from_min_max(pos2(start, rect.top()), pos2(start + width, rect.bottom())),
        );
        for i in 0..=12 {
            let x = start + width * i as f32 / 12.0;
            painter.line_segment(
                [pos2(x, rect.top() + 24.0), pos2(x, rect.bottom())],
                Stroke::new(1.0, Color32::from_rgb(43, 49, 58)),
            );
            painter.text(
                pos2(x, rect.top() + 11.0),
                egui::Align2::CENTER_CENTER,
                format!(
                    "{}",
                    (duration * self.session.document().fps as f32 * i as f32 / 12.0).round()
                        as u32
                ),
                FontId::monospace(11.0),
                MUTED,
            );
        }
        let tracks = self
            .session
            .document()
            .animations
            .get(self.animation)
            .map_or_else(Vec::new, |a| a.tracks.clone());
        for (i, property) in Property::ALL.into_iter().enumerate() {
            let y = rect.top() + 38.0 + i as f32 * 24.0;
            painter.text(
                pos2(rect.left() + 12.0, y),
                egui::Align2::LEFT_CENTER,
                property.label(),
                FontId::proportional(12.0),
                MUTED,
            );
            if let Some(track) = tracks
                .iter()
                .find(|t| self.selected_bone() == Some(t.bone) && t.property == property)
            {
                for key in &track.keys {
                    let x = start + width * key.time / duration;
                    let p = pos2(x, y);
                    let selected = same_key_time(key.time, self.snapped_time());
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            p + vec2(0.0, -5.0),
                            p + vec2(5.0, 0.0),
                            p + vec2(0.0, 5.0),
                            p + vec2(-5.0, 0.0),
                        ],
                        if selected { ACCENT } else { MINT },
                        Stroke::NONE,
                    ));
                    if response.clicked()
                        && response
                            .interact_pointer_pos()
                            .is_some_and(|mouse| mouse.distance(p) < 10.0)
                    {
                        self.set_time(key.time);
                        self.playing = false;
                    }
                }
            }
        }
        let playhead = start + width * self.time / duration;
        painter.line_segment(
            [
                pos2(playhead, rect.top() + 22.0),
                pos2(playhead, rect.bottom()),
            ],
            Stroke::new(1.5, ACCENT),
        );
        painter.circle_filled(pos2(playhead, rect.top() + 22.0), 4.0, ACCENT);
        if (response.dragged() || response.clicked())
            && let Some(p) = response.interact_pointer_pos()
            && p.x >= start
        {
            self.playing = false;
            let fps = self.session.document().fps as f32;
            let time = ((p.x - start) / width * duration).clamp(0.0, duration);
            self.set_time((time * fps).round() / fps);
        }
    }
}

fn transport_button(ui: &mut egui::Ui, direction: PlayDirection, active: bool) -> egui::Response {
    let response = ui.add_sized([25., 22.], egui::Button::selectable(active, ""));
    let center = response.rect.center();
    let color = if active { ACCENT } else { Color32::LIGHT_GRAY };
    if active {
        ui.painter().rect_filled(
            egui::Rect::from_center_size(center, vec2(8., 8.)),
            0.,
            color,
        );
    } else {
        let sign = if direction == PlayDirection::Forward {
            1.
        } else {
            -1.
        };
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                center + vec2(-4. * sign, -5.),
                center + vec2(-4. * sign, 5.),
                center + vec2(5. * sign, 0.),
            ],
            color,
            Stroke::NONE,
        ));
    }
    response
}
