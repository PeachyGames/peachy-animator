use super::*;

impl Editor {
    pub(super) fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("P").size(24.0).strong().color(ACCENT));
            ui.label(RichText::new("PEACHY ANIMATOR").strong().size(16.0));
            ui.separator();
            if ui.button("Open…").clicked() {
                self.open_project(ui.ctx());
            }
            if ui
                .button("Save")
                .on_hover_text("Save project · Cmd/Ctrl+S")
                .clicked()
            {
                self.save_project();
            }
            if ui
                .add_enabled(self.session.can_undo(), egui::Button::new("Undo"))
                .clicked()
            {
                self.undo();
            }
            if ui
                .add_enabled(self.session.can_redo(), egui::Button::new("Redo"))
                .clicked()
            {
                self.redo();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Export Spine 4.3…").color(Color32::from_rgb(32, 35, 39)),
                        )
                        .fill(ACCENT),
                    )
                    .clicked()
                {
                    self.export();
                }
                ui.label(RichText::new("MESH SLICE  /  0.1.0").small().color(MUTED));
            });
        });
    }

    pub(super) fn hierarchy(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.label(RichText::new("SCENE").small().color(MUTED));
        ui.heading(&self.session.document().name);
        ui.label(
            RichText::new(format!(
                "{} bones · {} slots",
                self.session.document().bones.len(),
                self.session.document().slots.len()
            ))
            .small()
            .color(MUTED),
        );
        ui.add_space(8.0);
        ui.separator();
        ui.horizontal(|ui| {
            if ui.selectable_label(self.setup_mode, "Setup").clicked() {
                self.setup_mode = true;
                self.playing = false;
            }
            if ui.selectable_label(!self.setup_mode, "Animate").clicked() {
                self.setup_mode = false;
            }
        });
        ui.add_space(8.0);
        let bones = &self.session.document().bones;
        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - 190.0)
            .show(ui, |ui| {
                for (i, bone) in bones.iter().enumerate() {
                    let mut depth = 0;
                    let mut parent = bone.parent;
                    while let Some(p) = parent {
                        depth += 1;
                        parent = bones[p].parent;
                    }
                    ui.horizontal(|ui| {
                        ui.add_space((depth as f32) * 12.0);
                        let label = if i == self.selected {
                            RichText::new(format!("◇  {}", bone.name)).color(ACCENT)
                        } else {
                            RichText::new(format!("◇  {}", bone.name))
                        };
                        if ui.selectable_label(i == self.selected, label).clicked() {
                            self.selected = i;
                        }
                    });
                }
            });
        ui.separator();
        ui.label(RichText::new("ADD A CHILD BONE").small().color(MUTED));
        ui.text_edit_singleline(&mut self.new_bone);
        if ui.button("+ Add to selected bone").clicked() {
            let parent = self.selected;
            let applied = self.dispatch(Command::AddBone {
                bone: Bone {
                    name: self.new_bone.trim().to_owned(),
                    parent: Some(parent),
                    length: 60.0,
                    setup: Transform {
                        x: self.session.document().bones[parent].length,
                        ..Default::default()
                    },
                },
            });
            if applied {
                self.selected = self.session.document().bones.len() - 1;
                self.new_bone.clear();
            }
        }
        ui.add_space(12.0);
        ui.label(
            RichText::new("A little rig. A real workflow.")
                .color(MUTED)
                .italics(),
        );
    }

    pub(super) fn inspector(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.label(RichText::new("INSPECTOR").small().color(MUTED));
        ui.heading(&self.session.document().bones[self.selected].name);
        ui.label(
            RichText::new(if self.setup_mode {
                "Setup pose"
            } else {
                "Animation pose · edits create keys"
            })
            .color(MUTED)
            .small(),
        );
        ui.add_space(8.0);
        ui.separator();
        let mut pose =
            local_pose(self.session.document(), self.active_animation(), self.time)[self.selected];
        self.attachment_panel(ui);
        for property in Property::ALL {
            let mut value = property.read(pose);
            ui.horizontal(|ui| {
                ui.add_sized([105.0, 22.0], egui::Label::new(property.label()));
                let drag = ui.add(
                    egui::DragValue::new(&mut value)
                        .speed(if matches!(property, Property::ScaleX | Property::ScaleY) {
                            0.01
                        } else {
                            0.25
                        })
                        .max_decimals(2),
                );
                if drag.drag_started() {
                    self.session.begin_transaction();
                }
                if drag.changed() {
                    property.write(&mut pose, value);
                    self.edit_transform(pose, &[property]);
                }
                if drag.drag_stopped() {
                    self.session.commit_transaction();
                }
                if !self.setup_mode
                    && ui
                        .small_button("◇")
                        .on_hover_text("Key this property at the playhead")
                        .clicked()
                {
                    self.edit_transform(pose, &[property]);
                }
            });
        }
        ui.add_space(10.0);
        if !self.setup_mode {
            if ui.button("◇ Key entire transform  [K]").clicked() {
                self.edit_transform(pose, &Property::ALL);
            }
            ui.add_space(10.0);
            ui.separator();
            ui.label(RichText::new("KEY INTERPOLATION").small().color(MUTED));
            let time = self.snapped_time();
            let keys: Vec<_> = self
                .session
                .document()
                .animations
                .get(self.animation)
                .into_iter()
                .flat_map(|a| a.tracks.iter())
                .filter(|t| t.bone == self.selected)
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
                        .selectable_label(curve == Curve::Stepped, "Hold")
                        .clicked()
                    {
                        curve = Curve::Stepped;
                    }
                    if ui
                        .selectable_label(matches!(curve, Curve::Bezier(_)), "Ease")
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
                        let response =
                            ui.add(egui::Slider::new(&mut handles[i], range).text(*label));
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
                        bone: self.selected,
                        property,
                        key: Keyframe { curve, ..key },
                    });
                }
                if ui.small_button("Remove key").clicked() {
                    self.dispatch(Command::DeleteKey {
                        animation: self.animation,
                        bone: self.selected,
                        property,
                        time,
                    });
                }
            }
            if !found {
                ui.label(
                    RichText::new("Select a diamond on the timeline to edit its curve.")
                        .color(MUTED),
                );
            }
        }
        ui.add_space(14.0);
        ui.separator();
        ui.label(RichText::new("SKIN").small().color(MUTED));
        egui::ComboBox::from_id_salt("skin")
            .selected_text(&self.skin)
            .show_ui(ui, |ui| {
                for skin in self.session.document().skins.keys() {
                    ui.selectable_value(&mut self.skin, skin.clone(), skin);
                }
            });
    }

    pub(super) fn timeline(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("TIMELINE").small().color(MUTED));
            if ui
                .button(if self.playing { "Pause" } else { "Play" })
                .on_hover_text("Space")
                .clicked()
            {
                self.playing = !self.playing;
                self.setup_mode = false;
            }
            if ui.small_button("|◀").clicked() {
                self.time = 0.0;
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
                    for (i, animation) in self.session.document().animations.iter().enumerate() {
                        if ui
                            .selectable_value(&mut self.animation, i, &animation.name)
                            .changed()
                        {
                            self.time = 0.0;
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
                    RichText::new("Click to scrub · arrows step frames · K adds keys")
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
        for i in 0..=12 {
            let x = start + width * i as f32 / 12.0;
            painter.line_segment(
                [pos2(x, rect.top() + 24.0), pos2(x, rect.bottom())],
                Stroke::new(1.0, Color32::from_rgb(43, 49, 58)),
            );
            painter.text(
                pos2(x, rect.top() + 11.0),
                egui::Align2::CENTER_CENTER,
                format!("{:.1}", duration * i as f32 / 12.0),
                FontId::monospace(11.0),
                MUTED,
            );
        }
        let tracks = self
            .session
            .document()
            .animations
            .get(self.animation)
            .map_or(&[][..], |a| a.tracks.as_slice());
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
                .find(|t| t.bone == self.selected && t.property == property)
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
                        self.time = key.time;
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
            self.setup_mode = false;
            self.time = ((p.x - start) / width * duration).clamp(0.0, duration);
            self.time = self.snapped_time().min(duration);
        }
    }
}
