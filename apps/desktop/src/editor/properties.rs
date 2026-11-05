use super::*;

impl Editor {
    pub(super) fn inspector(&mut self, ui: &mut egui::Ui) {
        self.record_target("properties", ui.clip_rect());
        ui.label(RichText::new("Properties").strong());
        ui.separator();
        let Some(item) = self.selection.clone() else {
            ui.label("Select an item in the Tree or viewport.");
            return;
        };
        ui.label(RichText::new(item.label(self.session.document())).color(ACCENT));
        match item {
            TreeItem::Skeleton => {
                let doc = self.session.document();
                ui.label(format!(
                    "{} bones · {} slots · {} animations",
                    doc.bones.len(),
                    doc.slots.len(),
                    doc.animations.len()
                ));
                ui.label(format!("{} fps · native format {}", doc.fps, doc.format));
                ui.label(format!("Active skin: {}", self.skin));
            }
            TreeItem::Bone(bone) => self.bone_properties(ui, bone),
            TreeItem::Slot(slot) => self.slot_properties(ui, slot),
            TreeItem::Placeholder { slot, name } => {
                ui.label("Skin placeholder");
                ui.label(format!(
                    "Slot: {}",
                    self.session.document().slots[slot].name
                ));
                ui.label(format!("Active skin: {}", self.skin));
                if let Some((key, _)) = self
                    .session
                    .document()
                    .resolve_attachment(&self.skin, slot, &name)
                {
                    let key = key.clone();
                    if ui.button("Select active attachment").clicked() {
                        self.select(TreeItem::Attachment(key));
                    }
                } else {
                    ui.label("No attachment for the active skin.");
                }
            }
            TreeItem::Attachment(key) => {
                ui.label(format!("Slot: {}", key.slot));
                ui.label(format!("Skin: {}", key.skin));
                if let Some(value) = self.session.document().attachment(&key) {
                    ui.label(format!("Image: {}", value.path()));
                    match value {
                        Attachment::Region(r) => {
                            ui.label(format!("Region · {} × {}", r.width, r.height));
                        }
                        Attachment::Mesh(_) => {
                            ui.label("Unweighted mesh");
                        }
                    }
                }
                self.attachment_panel(ui);
                if !self.setup_mode {
                    ui.label(
                        RichText::new("Attachment transforms and vertices are edited in Setup.")
                            .small()
                            .color(MUTED),
                    );
                }
            }
            TreeItem::Skin(skin) => {
                let count = self.session.document().skins[&skin]
                    .values()
                    .map(|a| a.len())
                    .sum::<usize>();
                ui.label(format!("{} attachments", count));
                ui.label(if self.skin == skin {
                    "Active skin"
                } else {
                    "Selected for inspection; another skin is active."
                });
                if ui.button("Make active").clicked() {
                    self.activate_skin(skin);
                }
            }
            TreeItem::Animation(animation) => {
                let a = &self.session.document().animations[animation];
                ui.label(format!(
                    "{} frames · {} tracks",
                    (a.duration * self.session.document().fps as f32).round() as u32,
                    a.tracks.len()
                ));
                ui.label(if self.animation == animation {
                    "Active animation"
                } else {
                    "Selected for inspection; another animation is active."
                });
                if ui.button("Activate animation").clicked() {
                    self.activate_animation(animation);
                }
            }
            TreeItem::Folder(Folder::Skins) => {
                ui.label("Use a skin's visibility dot to make it active.");
                if ui.button("Clear active skin").clicked() {
                    self.activate_skin("default".into());
                }
            }
            TreeItem::Folder(Folder::Animations) => {
                ui.label("Use an animation's visibility dot to make it active. Selecting its name only selects its properties.");
            }
            TreeItem::Folder(Folder::Slots) => {
                ui.label("Slots are listed in draw order. Use their visibility dots to hide artwork temporarily.");
            }
        }
    }

    fn bone_properties(&mut self, ui: &mut egui::Ui, bone: usize) {
        let b = &self.session.document().bones[bone];
        let mut length = b.length;
        let parent = b
            .parent
            .map(|i| self.session.document().bones[i].name.clone())
            .unwrap_or_else(|| "None".into());
        ui.label(format!("Bone · Parent: {parent}"));
        ui.add_enabled_ui(self.setup_mode, |ui| {
            ui.horizontal(|ui| {
                ui.label("Length");
                let response = ui.add(
                    egui::DragValue::new(&mut length)
                        .range(0.0..=f32::MAX)
                        .speed(0.25),
                );
                self.record_target("boneLength", response.rect);
                if response.drag_started() {
                    self.session.begin_transaction();
                }
                if response.changed() {
                    self.dispatch(Command::SetBoneLength { bone, length });
                }
                if response.drag_stopped() {
                    self.session.commit_transaction();
                }
            });
        });
        if self.setup_mode {
            ui.collapsing("New bone", |ui| {
                ui.text_edit_singleline(&mut self.new_bone);
                if ui.button("Create child bone").clicked()
                    && self.dispatch(Command::AddBone {
                        bone: Bone {
                            name: self.new_bone.trim().into(),
                            parent: Some(bone),
                            length: 60.,
                            setup: Transform {
                                x: self.session.document().bones[bone].length,
                                ..Default::default()
                            },
                        },
                    })
                {
                    self.new_bone.clear();
                    self.select(TreeItem::Bone(self.session.document().bones.len() - 1));
                }
            });
        } else {
            self.bone_curves(ui, bone);
        }
    }

    fn slot_properties(&mut self, ui: &mut egui::Ui, slot: usize) {
        let s = &self.session.document().slots[slot];
        ui.label(format!(
            "Slot · Bone: {}",
            self.session.document().bones[s.bone].name
        ));
        let choices: BTreeSet<_> = self
            .session
            .document()
            .skins
            .values()
            .filter_map(|skin| skin.get(&s.name))
            .flat_map(|entries| entries.keys().cloned())
            .collect();
        let mut selected = s.attachment.clone();
        let before = selected.clone();
        ui.add_enabled_ui(self.setup_mode, |ui| {
            let combo = egui::ComboBox::from_id_salt("slot-attachment")
                .selected_text(selected.as_deref().unwrap_or("None"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected, None, "None");
                    for name in choices {
                        ui.selectable_value(&mut selected, Some(name.clone()), name);
                    }
                });
            self.record_target("slotAttachment", combo.response.rect);
        });
        if selected != before {
            self.dispatch(Command::SetSlotAttachment {
                slot,
                attachment: selected,
            });
        }
        if let Some((key, _)) = self
            .session
            .document()
            .resolved_attachment(&self.skin, slot)
        {
            let key = key.clone();
            let button = ui.button("Select attachment");
            self.record_target("selectSlotAttachment", button.rect);
            if button.clicked() {
                self.select(TreeItem::Attachment(key));
            }
        }
        if !self.setup_mode {
            ui.label(
                RichText::new("Setup attachment only; attachment timelines are not implemented.")
                    .small()
                    .color(MUTED),
            );
        }
    }
}
