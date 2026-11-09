use super::*;
use tree_model::{Entry, Filters, project, row_visible};

enum Action {
    Key(usize),
    Select(TreeItem),
    Rename(TreeItem),
    Dot(TreeItem, bool),
    Branch(usize, bool),
    All(bool),
}

impl Editor {
    pub(super) fn hierarchy(&mut self, ui: &mut egui::Ui) {
        self.record_target("tree", ui.max_rect());
        let mut actions = vec![];
        ui.horizontal(|ui| {
            super::icons::heading(ui, super::icons::Icon::Tree, "Tree");
            ui.menu_button(
                RichText::new("Filter").color(if self.tree.filters == Filters::default() {
                    MUTED
                } else {
                    ACCENT
                }),
                |ui| {
                    ui.checkbox(&mut self.tree.filters.bones, "Bones");
                    ui.checkbox(&mut self.tree.filters.slots, "Slots");
                    ui.checkbox(&mut self.tree.filters.attachments, "Attachments");
                    ui.checkbox(&mut self.tree.filters.skins, "Skins");
                    ui.checkbox(&mut self.tree.filters.animations, "Animations");
                    ui.checkbox(&mut self.tree.filters.constraints, "Constraints");
                    if ui.button("Reset").clicked() {
                        self.tree.filters = Filters::default();
                    }
                },
            );
            ui.menu_button("Settings", |ui| {
                ui.checkbox(
                    &mut self.tree.all_skin_attachments,
                    "Show all skin attachments",
                );
            });
            if ui.small_button("+").on_hover_text("Expand all").clicked() {
                actions.push(Action::All(false));
            }
            if ui.small_button("−").on_hover_text("Collapse all").clicked() {
                actions.push(Action::All(true));
            }
            let close = icons::toolbar_button(ui, icons::Icon::Delete, "Close Tree view", "", true);
            self.record_target("closeView:Tree", close.rect);
            if close.clicked() {
                self.cancel_gesture();
                self.weights.bind = None;
                self.show_tree = false;
            }
        });
        ui.horizontal(|ui| {
            for (label, filters) in [
                ("All", Filters::default()),
                (
                    "Bones",
                    Filters {
                        bones: true,
                        slots: false,
                        attachments: false,
                        skins: false,
                        animations: false,
                        constraints: false,
                    },
                ),
                (
                    "Slots",
                    Filters {
                        bones: false,
                        slots: true,
                        attachments: false,
                        skins: false,
                        animations: false,
                        constraints: false,
                    },
                ),
                (
                    "Attachments",
                    Filters {
                        bones: false,
                        slots: false,
                        attachments: true,
                        skins: false,
                        animations: false,
                        constraints: false,
                    },
                ),
            ] {
                let r = ui.small_button(label);
                self.record_target(&format!("filter:{label}"), r.rect);
                if r.clicked() {
                    self.tree.filters = filters;
                }
            }
        });
        let search = ui.add(
            egui::TextEdit::singleline(&mut self.tree.query)
                .hint_text("Search")
                .desired_width(f32::INFINITY),
        );
        self.record_target("treeSearch", search.rect);
        ui.separator();
        let entries = project(
            self.session.document(),
            &self.skin,
            self.setup_mode,
            &self.tree,
        );
        let mut children = vec![false; entries.len()];
        for e in &entries {
            if let Some(p) = e.parent {
                children[p] = true;
            }
        }
        egui::ScrollArea::vertical()
            .id_salt("tree-scroll")
            .show(ui, |ui| {
                for (index, e) in entries.iter().enumerate() {
                    if !row_visible(&entries, index, &self.tree) {
                        continue;
                    }
                    let key = e.item.key();
                    let dot = self.tree_dot(&e.item);
                    let selected = self.selection.as_ref() == Some(&e.item);
                    ui.horizontal(|ui| {
                        let (r, response) = ui.allocate_exact_size(vec2(15., 19.), Sense::click());
                        if let Some(active) = dot {
                            ui.painter().circle(
                                r.center(),
                                3.,
                                if active { ACCENT } else { Color32::TRANSPARENT },
                                Stroke::new(1., MUTED),
                            );
                            if r.intersects(ui.clip_rect()) {
                                self.record_target(&format!("dot:{key}"), r);
                            }
                            if response.clicked() || response.secondary_clicked() {
                                actions.push(Action::Dot(
                                    e.item.clone(),
                                    response.secondary_clicked(),
                                ));
                            }
                        }
                        ui.add_space(e.depth as f32 * 11.);
                        let (arrow, click) = ui.allocate_exact_size(vec2(13., 19.), Sense::click());
                        let collapsed =
                            self.tree.collapsed.contains(&key) && self.tree.query.trim().is_empty();
                        if children[index] {
                            let c = arrow.center();
                            let points = if collapsed {
                                vec![c + vec2(-2., -4.), c + vec2(3., 0.), c + vec2(-2., 4.)]
                            } else {
                                vec![c + vec2(-4., -2.), c + vec2(4., -2.), c + vec2(0., 3.)]
                            };
                            ui.painter().add(egui::Shape::convex_polygon(
                                points,
                                MUTED,
                                Stroke::NONE,
                            ));
                            if click.clicked() {
                                actions.push(Action::Branch(index, false));
                            }
                        }
                        let inactive = e
                            .item
                            .attachment()
                            .is_some_and(|k| !self.attachment_editable(k))
                            || matches!(e.item, TreeItem::Constraint(index) if self.ik_constraint(index).is_some_and(|ik|ik.setup.mix == 0.) || self.physics_constraint(index).is_some_and(|physics|physics.setup.mix == 0.));
                        let row = ui.selectable_label(
                            selected,
                            RichText::new(format!("    {}", e.label)).color(if selected {
                                ACCENT
                            } else if inactive {
                                MUTED
                            } else {
                                Color32::LIGHT_GRAY
                            }),
                        );
                        let physics = matches!(e.item, TreeItem::Constraint(index) if self.physics_constraint(index).is_some());
                        draw_icon(
                            ui.painter(),
                            row.rect.min + vec2(4., 4.),
                            &e.item,
                            physics,
                            if selected { ACCENT } else { MUTED },
                        );
                        if row.rect.intersects(ui.clip_rect()) {
                            self.record_target(&key, row.rect);
                        }
                        if row.double_clicked() {
                            actions.push(Action::Rename(e.item.clone()));
                        } else if row.clicked() {
                            actions.push(Action::Select(e.item.clone()));
                        }
                        if row.secondary_clicked() && children[index] {
                            actions.push(Action::Branch(index, true));
                        }
                        if self.tree.reveal.as_ref() == Some(&key) {
                            row.scroll_to_me(Some(egui::Align::Center));
                            self.tree.reveal = None;
                        }
                        if !self.setup_mode
                            && let TreeItem::Bone(bone) = e.item
                            && self.session.pose_edits().has_bone(self.animation, bone)
                        {
                            let key = super::animation::pose_key_button(ui, KeyState::Edited);
                            self.record_target(&format!("keyBone:{bone}"), key.rect);
                            if key
                                .on_hover_text("Key this bone's edited transforms")
                                .clicked()
                            {
                                actions.push(Action::Key(bone));
                            }
                        }
                        if let Some(k) = e.item.attachment() {
                            row.on_hover_text(format!(
                                "Skin: {}\nSlot: {}\nPlaceholder: {}",
                                k.skin, k.slot, k.placeholder
                            ));
                        }
                        if let TreeItem::Bone(bone) = e.item {
                            let annotations: Vec<_> = self.session.document().constraints.iter().enumerate().filter_map(|(index,constraint)| {
                                let ik=constraint.as_ik()?;
                                (ik.bones.contains(&bone) || ik.target==bone).then(||(index,ik.name.clone()))
                            }).collect();
                            if !annotations.is_empty() {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                                    for (constraint,name) in annotations.into_iter().rev() {
                                        let annotation=icons::toolbar_button(ui,icons::Icon::IkConstraint,&format!("Select IK constraint: {name}"),"",true);
                                        self.record_target(&format!("ikAnnotation:{bone}:{constraint}"),annotation.rect);
                                        if annotation.clicked() { actions.push(Action::Select(TreeItem::Constraint(constraint))); }
                                    }
                                });
                            }
                        }
                    });
                }
            });
        for action in actions {
            match action {
                Action::Key(bone) => self.key_edited(Some(bone)),
                Action::Select(item) => {
                    self.select(item);
                    // A clicked row is already visible. Scrolling it here can move
                    // the target away between the two clicks of a double-click.
                    self.tree.reveal = None;
                }
                Action::Rename(item) => {
                    self.select(item.clone());
                    self.tree.reveal = None;
                    self.begin_rename(item);
                }
                Action::Dot(item, branch) => self.toggle_tree_dot(item, branch, &entries),
                Action::All(collapsed) => {
                    for e in &entries {
                        if collapsed {
                            self.tree.collapsed.insert(e.item.key());
                        } else {
                            self.tree.collapsed.remove(&e.item.key());
                        }
                    }
                }
                Action::Branch(index, subtree) => {
                    let collapse = !self.tree.collapsed.contains(&entries[index].item.key());
                    for (i, e) in entries.iter().enumerate() {
                        if i == index || (subtree && descendant(&entries, i, index)) {
                            if collapse {
                                self.tree.collapsed.insert(e.item.key());
                            } else {
                                self.tree.collapsed.remove(&e.item.key());
                            }
                        }
                    }
                }
            }
        }
    }

    fn tree_dot(&self, item: &TreeItem) -> Option<bool> {
        let doc = self.session.document();
        match item {
            TreeItem::Bone(i) => Some(!self.hidden_bones.contains(i)),
            TreeItem::Slot(i) => Some(!self.hidden_slots.contains(i)),
            TreeItem::Skin(s) => Some(&self.skin == s),
            TreeItem::Folder(Folder::Skins) => Some(self.skin == "default"),
            TreeItem::Animation(i) => Some(self.animation == *i),
            TreeItem::Attachment(k) => item.slot(doc).map(|slot| {
                doc.resolved_attachment(&self.skin, slot)
                    .is_some_and(|(shown, _)| &shown == k)
            }),
            TreeItem::Placeholder { slot, name } => {
                Some(doc.slots[*slot].attachment.as_ref() == Some(name))
            }
            _ => None,
        }
    }
    fn toggle_tree_dot(&mut self, item: TreeItem, branch: bool, entries: &[Entry]) {
        let was_visible = self.tree_dot(&item).unwrap_or(false);
        match &item {
            TreeItem::Bone(bone) => {
                let hide = !self.hidden_bones.contains(bone);
                if let Some(index) = entries.iter().position(|e| e.item == item) {
                    for (_, e) in entries
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i == index || (branch && descendant(entries, *i, index)))
                    {
                        match e.item {
                            TreeItem::Bone(b) => set_hidden(&mut self.hidden_bones, b, hide),
                            TreeItem::Slot(s) => set_hidden(&mut self.hidden_slots, s, hide),
                            _ => {}
                        }
                    }
                }
            }
            TreeItem::Slot(slot) => {
                let hide = !self.hidden_slots.contains(slot);
                set_hidden(&mut self.hidden_slots, *slot, hide);
            }
            TreeItem::Skin(s) => self.activate_skin(if self.skin == *s {
                "default".into()
            } else {
                s.clone()
            }),
            TreeItem::Folder(Folder::Skins) => self.activate_skin("default".into()),
            TreeItem::Animation(i) => self.activate_animation(*i),
            TreeItem::Attachment(k) => {
                if self.setup_mode {
                    if k.skin != "default" && k.skin != self.skin {
                        self.activate_skin(k.skin.clone());
                    }
                    if let Some(slot) = item.slot(self.session.document()) {
                        self.dispatch(Command::SetSlotAttachment {
                            slot,
                            attachment: if was_visible {
                                None
                            } else {
                                Some(k.placeholder.clone())
                            },
                        });
                    }
                } else {
                    self.status="Attachment timelines are not implemented; change the setup attachment in Setup.".into();
                }
            }
            TreeItem::Placeholder { slot, name } if self.setup_mode => {
                self.toggle_slot_attachment(*slot, name);
            }
            _ => {}
        }
    }
    fn toggle_slot_attachment(&mut self, slot: usize, name: &str) {
        let attachment = if self.session.document().slots[slot].attachment.as_deref() == Some(name)
        {
            None
        } else {
            Some(name.into())
        };
        self.dispatch(Command::SetSlotAttachment { slot, attachment });
    }
}

fn descendant(entries: &[Entry], index: usize, ancestor: usize) -> bool {
    let mut p = entries[index].parent;
    while let Some(i) = p {
        if i == ancestor {
            return true;
        }
        p = entries[i].parent;
    }
    false
}
fn set_hidden(set: &mut BTreeSet<usize>, index: usize, hidden: bool) {
    if hidden {
        set.insert(index);
    } else {
        set.remove(&index);
    }
}
fn draw_icon(p: &egui::Painter, origin: Pos2, item: &TreeItem, physics: bool, color: Color32) {
    let stroke = Stroke::new(1., color);
    let r = egui::Rect::from_min_size(origin, vec2(10., 10.));
    match item {
        TreeItem::Constraint(_) if physics => icons::draw_physics(p, r, color),
        TreeItem::Constraint(_) | TreeItem::Folder(Folder::Constraints) => {
            icons::draw_ik(p, r, color)
        }
        TreeItem::Bone(_) => {
            icons::draw_bone(p, r, color);
        }
        TreeItem::Animation(_) => {
            p.add(egui::Shape::convex_polygon(
                vec![origin, origin + vec2(10., 5.), origin + vec2(0., 10.)],
                color,
                Stroke::NONE,
            ));
        }
        TreeItem::Placeholder { .. } => {
            p.add(egui::Shape::closed_line(
                vec![
                    r.center_top(),
                    r.right_center(),
                    r.center_bottom(),
                    r.left_center(),
                ],
                stroke,
            ));
        }
        TreeItem::Skin(_) => {
            p.rect_stroke(r, 2., stroke, egui::StrokeKind::Inside);
            p.line_segment([origin + vec2(2., 3.), origin + vec2(8., 3.)], stroke);
        }
        TreeItem::Attachment(_) => {
            p.rect_stroke(r, 0., stroke, egui::StrokeKind::Inside);
            p.line_segment([r.left_bottom(), r.right_top()], stroke);
        }
        _ => {
            p.rect_stroke(r, 1., stroke, egui::StrokeKind::Inside);
        }
    }
}
