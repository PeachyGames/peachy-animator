//! Dopesheet controls adapt pointer/keyboard input to shared commands.
use super::dopesheet_selection::KeyId;
use super::icons::Icon;
use super::key_drag::{ChartGeometry, KeyDrag, primary_pointer_event};
use super::*;
use peachy_engine::{KeyOwner, UnsupportedAnimationTimeline};

#[cfg(test)]
#[path = "multi_owner_dopesheet_tests.rs"]
mod multi_owner_tests;

#[derive(Clone)]
enum ShownRow {
    Owner(KeyOwner),
    Channel(KeyOwner, dopesheet_selection::TimelineRow),
    ReadOnly(UnsupportedAnimationTimeline),
}

pub(super) fn owner_id(owner: &KeyOwner) -> String {
    match owner {
        KeyOwner::Bone { bone } => format!("bone:{bone}"),
        KeyOwner::Ik { constraint } => format!("ik:{constraint}"),
        KeyOwner::Slider { constraint } => format!("slider:{constraint}"),
        KeyOwner::Slot { slot } => format!("slot:{slot}"),
        KeyOwner::DrawOrder {} => "drawOrder".into(),
        KeyOwner::Deform { attachment } => {
            format!("deform:{}", serde_json::to_string(attachment).unwrap())
        }
    }
}

impl Editor {
    pub(super) fn dopesheet_id() -> egui::Id {
        egui::Id::new("dopesheet-keys")
    }
    pub(super) fn dopesheet_has_focus(&self, ctx: &egui::Context) -> bool {
        !self.setup_mode
            && self.dopesheet_visible()
            && self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
            && self.export_dialog.is_none()
            && !egui::Popup::is_any_open(ctx)
            && !ctx.text_edit_focused()
            && ctx.memory(|memory| memory.has_focus(Self::dopesheet_id()))
    }
    pub(super) fn reconcile_dopesheet_selection(&mut self) {
        let animation = self.active_animation();
        let owner = self.timeline_owner();
        if let Err(error) = self.key_selection.prepare(&self.session, animation, owner) {
            self.key_selection.clear();
            self.status = error;
        }
        let readonly = self
            .key_selection
            .shown()
            .owners
            .iter()
            .filter(|owner| !self.timeline_owner_editable(owner))
            .cloned()
            .collect();
        self.key_selection.mark_readonly(readonly);
    }
    pub(super) fn timeline_owner(&self) -> Option<peachy_engine::KeyOwner> {
        match self.selection.as_ref() {
            Some(TreeItem::DrawOrder | TreeItem::DrawOrderSlot { .. }) => {
                Some(peachy_engine::KeyOwner::DrawOrder {})
            }
            Some(TreeItem::Bone(bone)) => Some(peachy_engine::KeyOwner::Bone { bone: *bone }),
            Some(TreeItem::Slot(slot)) => Some(peachy_engine::KeyOwner::Slot { slot: *slot }),
            Some(TreeItem::Placeholder { slot, .. }) => {
                Some(peachy_engine::KeyOwner::Slot { slot: *slot })
            }
            Some(TreeItem::Constraint(constraint))
                if self.slider_constraint(*constraint).is_some() =>
            {
                Some(peachy_engine::KeyOwner::Slider {
                    constraint: *constraint,
                })
            }
            Some(TreeItem::Constraint(constraint)) if self.ik_constraint(*constraint).is_some() => {
                Some(peachy_engine::KeyOwner::Ik {
                    constraint: *constraint,
                })
            }
            Some(TreeItem::Attachment(attachment)) if self.attachment_editable(attachment) => {
                peachy_engine::deform_key_owner(self.session.document(), attachment).ok()
            }
            _ => None,
        }
    }
    /// A key's explicit primary owner drives Graph dispatch; Tree selection only
    /// supplies a fallback when there is no authored key selection.
    pub(super) fn graph_owner(&self) -> Option<KeyOwner> {
        self.primary_graph_key()
            .map(|key| key.owner())
            .filter(|owner| self.timeline_owner_editable(owner))
            .or_else(|| self.retained_graph_owner())
            .or_else(|| self.timeline_owner())
    }
    pub(super) fn primary_graph_key(&self) -> Option<peachy_engine::AnimationKey> {
        self.preferred_scalar_curve_selection()
            .or_else(|| self.key_selection.primary().cloned())
    }
    pub(super) fn timeline_owner_editable(&self, owner: &KeyOwner) -> bool {
        let KeyOwner::Deform { attachment } = owner else {
            return true;
        };
        if self.attachment_editable(attachment) {
            return true;
        }
        self.session.document().skins.iter().any(|(skin, slots)| {
            (skin == "default" || *skin == self.skin)
                && slots.iter().any(|(slot, placeholders)| {
                    placeholders.keys().any(|placeholder| {
                        let candidate = AttachmentRef {
                            skin: skin.clone(),
                            slot: slot.clone(),
                            placeholder: placeholder.clone(),
                        };
                        self.session.document().deform_target(&candidate) == *attachment
                    })
                })
        })
    }
    fn timeline_owner_name(&self, owner: &KeyOwner) -> String {
        let doc = self.session.document();
        match owner {
            KeyOwner::Bone { bone } => doc
                .bones
                .get(*bone)
                .map_or("Bone", |bone| &bone.name)
                .into(),
            KeyOwner::Ik { constraint } | KeyOwner::Slider { constraint } => doc
                .constraints
                .get(*constraint)
                .map_or("Constraint", Constraint::name)
                .into(),
            KeyOwner::Slot { slot } => doc
                .slots
                .get(*slot)
                .map_or("Slot", |slot| &slot.name)
                .into(),
            KeyOwner::Deform { attachment } => {
                format!("{}/{}", attachment.slot, attachment.placeholder)
            }
            KeyOwner::DrawOrder {} => "Draw order".into(),
        }
    }
    fn select_timeline_owner(&mut self, owner: KeyOwner) {
        let item = match owner {
            KeyOwner::Bone { bone } => TreeItem::Bone(bone),
            KeyOwner::Ik { constraint } | KeyOwner::Slider { constraint } => {
                TreeItem::Constraint(constraint)
            }
            KeyOwner::Slot { slot } => TreeItem::Slot(slot),
            KeyOwner::Deform { attachment } => TreeItem::Attachment(attachment),
            KeyOwner::DrawOrder {} => TreeItem::DrawOrder,
        };
        self.select(item);
    }
    fn unsupported_timeline_name(&self, timeline: &UnsupportedAnimationTimeline) -> String {
        let doc = self.session.document();
        match timeline {
            UnsupportedAnimationTimeline::Inherit { bone, .. } => {
                format!("Inherit: {}", doc.bones[*bone].name)
            }
            UnsupportedAnimationTimeline::Sequence { attachment, .. } => {
                format!("Sequence: {}/{}", attachment.slot, attachment.placeholder)
            }
            UnsupportedAnimationTimeline::Path {
                constraint,
                property,
                ..
            } => format!("Path {property:?}: {}", doc.constraints[*constraint].name()),
            UnsupportedAnimationTimeline::Physics {
                constraint,
                property,
                ..
            } => format!(
                "Physics {property:?}: {}",
                constraint.map_or("Global", |index| doc.constraints[index].name())
            ),
            UnsupportedAnimationTimeline::PhysicsReset { constraint, .. } => format!(
                "Physics Reset: {}",
                constraint.map_or("Global", |index| doc.constraints[index].name())
            ),
            UnsupportedAnimationTimeline::Event { .. } => "Events".into(),
        }
    }
    pub(super) fn delete_dopesheet_keys(&mut self) {
        self.cancel_gesture();
        self.reconcile_dopesheet_selection();
        if let Some(command) = self.key_selection.deletion() {
            self.playing = false;
            self.dispatch(command);
        }
    }
    pub(super) fn animation_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            self.timeline_clipboard_toolbar(ui);
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
                    "{:03.0} / {:03.0}  ·  {} fps",
                    animation::frame_at(self.time, self.session.document().fps),
                    animation::frame_at(self.animation_end(), self.session.document().fps),
                    self.session.document().fps
                ))
                .monospace()
                .color(ACCENT),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let close = icons::toolbar_button(
                    ui,
                    Icon::Delete,
                    if self.graph_visible() {
                        "Close Graph view"
                    } else {
                        "Close Dopesheet view"
                    },
                    "",
                    true,
                );
                self.record_target(
                    if self.graph_visible() {
                        "closeView:Graph"
                    } else {
                        "closeView:Dopesheet"
                    },
                    close.rect,
                );
                if close.clicked() {
                    self.cancel_gesture();
                    if self.graph_visible() {
                        self.view_tabs.open.remove(&super::views::DockView::Graph);
                        self.view_tabs.animation_view = super::views::AnimationView::Dopesheet;
                        if self.show_dopesheet {
                            ui.memory_mut(|memory| memory.request_focus(Self::dopesheet_id()));
                        } else {
                            ui.memory_mut(|memory| memory.surrender_focus(Self::graph_id()));
                        }
                    } else {
                        self.show_dopesheet = false;
                        if self.view_tabs.open.contains(&super::views::DockView::Graph) {
                            self.view_tabs.animation_view = super::views::AnimationView::Graph;
                            ui.memory_mut(|memory| memory.request_focus(Self::graph_id()));
                        } else {
                            ui.memory_mut(|memory| memory.surrender_focus(Self::dopesheet_id()));
                        }
                    }
                }
                ui.label(
                    RichText::new("A / D: play · W / S: keys · R / F: frames")
                        .small()
                        .color(MUTED),
                );
            });
        });
    }

    pub(super) fn timeline(&mut self, ui: &mut egui::Ui) {
        self.reconcile_dopesheet_selection();
        let view_rect = ui.max_rect();
        self.record_target("dopesheet", view_rect);
        self.record_target("dopesheetClip", ui.clip_rect());
        self.animation_controls(ui);
        ui.add_space(5.0);
        let label_width = 240.0;
        let pitch = layout::timeline_row_pitch(ui);
        let (_, rect) = ui.allocate_space(vec2(
            ui.available_width(),
            layout::timeline_chart_height(ui),
        ));
        // Property controls own the label column; only the grid can scrub or retime keys.
        let grid = egui::Rect::from_min_max(
            pos2((rect.left() + label_width).min(rect.right()), rect.top()),
            rect.max,
        );
        let ruler = egui::Rect::from_min_max(
            grid.min,
            pos2(grid.right(), (rect.top() + 26.).min(rect.bottom())),
        );
        let ruler_response = ui.interact(
            ruler,
            Self::dopesheet_id().with("ruler"),
            Sense::click_and_drag(),
        );
        ruler_response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Dopesheet ruler")
        });
        self.record_target("timelineRuler", ruler.intersect(ui.clip_rect()));
        let rows_grid = egui::Rect::from_min_max(pos2(grid.left(), ruler.bottom()), grid.max);
        let response = ui.interact(rows_grid, Self::dopesheet_id(), Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Dopesheet keys")
        });
        self.record_target("timelineCanvas", rect);
        if response.clicked_elsewhere()
            && ui.input(|input| {
                input
                    .pointer
                    .interact_pos()
                    .is_none_or(|pointer| !view_rect.contains(pointer))
            })
        {
            response.surrender_focus();
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 5.0, Color32::from_rgb(21, 26, 33));
        let geometry = self.key_drag.as_ref().map_or(
            ChartGeometry {
                start: rect.left() + label_width,
                width: (rect.width() - label_width - 18.0).max(1.0),
                extent: self.display_extent(),
            },
            |drag| drag.geometry,
        );
        let ChartGeometry {
            start,
            width,
            extent,
        } = geometry;
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
                    "{:.0}",
                    (f64::from(extent) * f64::from(self.session.document().fps) * f64::from(i)
                        / 12.0)
                        .round()
                ),
                FontId::monospace(11.0),
                MUTED,
            );
        }
        if self.timeline_owner() == Some(KeyOwner::DrawOrder {}) {
            self.record_target(
                "timelineGroup:drawOrder",
                egui::Rect::from_min_size(rect.min, vec2(label_width, 24.)),
            );
            painter.text(
                pos2(rect.left() + 12., rect.top() + 12.),
                egui::Align2::LEFT_CENTER,
                "Draw order",
                FontId::proportional(12.),
                Color32::from_rgb(80, 157, 224),
            );
        }
        let shown = self.key_selection.shown();
        let single_owner = self.timeline_owner().is_some();
        let mut rows = Vec::new();
        for owner in &shown.owners {
            if !single_owner {
                rows.push(ShownRow::Owner(owner.clone()));
            }
            for row in dopesheet_selection::rows(Some(owner.clone())) {
                if single_owner
                    || shown
                        .keys
                        .iter()
                        .any(|key| key.owner() == *owner && row.matches(key.clone()))
                {
                    rows.push(ShownRow::Channel(owner.clone(), row));
                }
            }
        }
        rows.extend(shown.unsupported.iter().cloned().map(ShownRow::ReadOnly));
        let mut key_targets = Vec::new();
        let row_area = egui::Rect::from_min_max(pos2(rect.left(), rect.top() + 26.), rect.max);
        let mut row_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("dopesheet-rows")
                .max_rect(row_area),
        );
        row_ui.spacing_mut().item_spacing.y = 0.;
        egui::ScrollArea::vertical()
            .id_salt("dopesheet-row-scroll")
            .auto_shrink([false, false])
            .max_height(row_area.height())
            .show_rows(&mut row_ui, pitch, rows.len(), |ui, range| {
                let rows_clip = ui.clip_rect().intersect(row_area);
                self.record_target("timelineRowsClip", rows_clip);
                let painter = ui.painter_at(rows_clip);
                for i in range {
                    let (_, row_rect) = ui.allocate_space(vec2(ui.available_width(), pitch));
                    let y = row_rect.center().y;
                    let (row_owner, row) = match &rows[i] {
                        ShownRow::Owner(owner) => {
                            let group = egui::Rect::from_min_max(
                                pos2(rect.left() + 6., row_rect.top()),
                                pos2(rect.left() + label_width - 6., row_rect.bottom()),
                            )
                            .intersect(rows_clip);
                            let name = self.timeline_owner_name(owner);
                            let response = ui.interact(
                                group,
                                ui.id().with(("timeline-owner", owner_id(owner))),
                                Sense::click(),
                            );
                            painter.text(
                                pos2(group.left() + 6., y),
                                egui::Align2::LEFT_CENTER,
                                &name,
                                FontId::proportional(12.),
                                Color32::LIGHT_GRAY,
                            );
                            self.record_target(
                                &format!("timelineOwner:{}", owner_id(owner)),
                                group,
                            );
                            if response.clicked() {
                                self.select_timeline_owner(owner.clone());
                            }
                            continue;
                        }
                        ShownRow::ReadOnly(timeline) => {
                            let name = self.unsupported_timeline_name(timeline);
                            painter.text(
                                pos2(rect.left() + 12., y),
                                egui::Align2::LEFT_CENTER,
                                format!("{name} · read-only"),
                                FontId::proportional(12.),
                                MUTED,
                            );
                            for time in timeline.times() {
                                let x = (f64::from(geometry.start)
                                    + f64::from(geometry.width) * *time
                                        / f64::from(geometry.extent))
                                    as f32;
                                let point = pos2(x, y);
                                if point.is_finite()
                                    && rows_clip.contains(point)
                                    && grid.contains(point)
                                {
                                    painter.rect_stroke(
                                        egui::Rect::from_center_size(point, vec2(5., 11.)),
                                        0.,
                                        Stroke::new(1., MUTED),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                            }
                            let target = row_rect.intersect(rows_clip);
                            if target.is_positive() {
                                self.record_target(&format!("timelineReadOnly:{i}"), target);
                            }
                            continue;
                        }
                        ShownRow::Channel(owner, row) => (owner.clone(), *row),
                    };
                    let editable = self.timeline_owner_editable(&row_owner);
                    let owner = Some(row_owner.clone());
                    let (name, tool, target, color) = match row {
                        dopesheet_selection::TimelineRow::Bone(property) => {
                            let tool = match property {
                                Property::Rotation => Tool::Rotate,
                                Property::X | Property::Y => Tool::Move,
                                Property::ScaleX | Property::ScaleY => Tool::Scale,
                                Property::ShearX | Property::ShearY => Tool::Shear,
                            };
                            let color = match property {
                                Property::Rotation => Color32::from_rgb(109, 189, 126),
                                Property::X | Property::Y => Color32::from_rgb(105, 168, 225),
                                Property::ScaleX | Property::ScaleY => {
                                    Color32::from_rgb(211, 116, 116)
                                }
                                Property::ShearX | Property::ShearY => {
                                    Color32::from_rgb(190, 145, 210)
                                }
                            };
                            (
                                row.label().to_owned(),
                                Some(tool),
                                format!("timelineRow:{property:?}"),
                                color,
                            )
                        }
                        dopesheet_selection::TimelineRow::Ik => {
                            let Some(peachy_engine::KeyOwner::Ik { constraint }) = &owner else {
                                continue;
                            };
                            (
                                "IK constraint".to_owned(),
                                None,
                                format!("timelineRow:ik:{constraint}"),
                                MINT,
                            )
                        }
                        dopesheet_selection::TimelineRow::Slider(property) => (
                            format!(
                                "Slider {}",
                                if property == SliderProperty::Time {
                                    "Time"
                                } else {
                                    "Mix"
                                }
                            ),
                            None,
                            format!("timelineRow:slider:{property:?}"),
                            if property == SliderProperty::Time {
                                Color32::from_rgb(90, 180, 205)
                            } else {
                                Color32::from_rgb(195, 120, 205)
                            },
                        ),
                        dopesheet_selection::TimelineRow::Deform => {
                            let Some(peachy_engine::KeyOwner::Deform { attachment }) = &owner
                            else {
                                continue;
                            };
                            let name = self
                                .session
                                .document()
                                .attachment(attachment)
                                .and_then(|value| value.name().or_else(|| value.path()))
                                .unwrap_or(&attachment.placeholder);
                            (
                                format!("Deform: {name}"),
                                None,
                                "timelineRow:deform".to_owned(),
                                Color32::from_rgb(45, 165, 235),
                            )
                        }
                        dopesheet_selection::TimelineRow::SlotAttachment => (
                            if let Some(peachy_engine::KeyOwner::Slot { slot }) = &owner {
                                format!("Attach: {}", self.session.document().slots[*slot].name)
                            } else {
                                "Attach".to_owned()
                            },
                            None,
                            "timelineRow:attachment".to_owned(),
                            Color32::from_rgb(180, 184, 190),
                        ),
                        dopesheet_selection::TimelineRow::SlotColor => (
                            if let Some(peachy_engine::KeyOwner::Slot { slot }) = &owner {
                                format!("RGBA: {}", self.session.document().slots[*slot].name)
                            } else {
                                "RGBA".to_owned()
                            },
                            None,
                            "timelineRow:color".to_owned(),
                            Color32::from_rgb(224, 106, 184),
                        ),
                        dopesheet_selection::TimelineRow::DrawOrder => (
                            "Default".to_owned(),
                            None,
                            "timelineRow:drawOrder".to_owned(),
                            Color32::from_rgb(80, 157, 224),
                        ),
                    };
                    let channel_id = match row {
                        dopesheet_selection::TimelineRow::Bone(property) => format!("{property:?}"),
                        dopesheet_selection::TimelineRow::Slider(property) => {
                            format!("{property:?}")
                        }
                        dopesheet_selection::TimelineRow::Ik => "Mix".into(),
                        dopesheet_selection::TimelineRow::Deform => "deform".into(),
                        dopesheet_selection::TimelineRow::SlotAttachment => "attachment".into(),
                        dopesheet_selection::TimelineRow::SlotColor => "color".into(),
                        dopesheet_selection::TimelineRow::DrawOrder => "Default".into(),
                    };
                    let qualified = format!("{}:{channel_id}", owner_id(&row_owner));
                    let label_rect = egui::Rect::from_min_max(
                        pos2(rect.left() + 6.0, y - 10.0),
                        pos2(rect.left() + label_width - 6.0, y + 10.0),
                    )
                    .intersect(rows_clip);
                    let label = ui
                        .interact(
                            label_rect,
                            ui.id().with(("timeline-row", &qualified)),
                            Sense::click(),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    let label = if let Some(tool) = tool {
                        let (name, shortcut) = match tool {
                            Tool::Rotate => ("Rotate", "C"),
                            Tool::Move => ("Translate", "V"),
                            Tool::Scale => ("Scale", "X"),
                            Tool::Shear => ("Shear", "Z"),
                            _ => ("Transform", ""),
                        };
                        label.on_hover_text(format!("Select {name} tool · {shortcut}"))
                    } else {
                        label.on_hover_text(match row {
                            dopesheet_selection::TimelineRow::Slider(_) => {
                                "Slider Time/Mix keys; select a channel to edit its authored keys."
                            }
                            dopesheet_selection::TimelineRow::Deform => &name,
                            dopesheet_selection::TimelineRow::SlotAttachment => {
                                "Slot attachment swaps are stepped; None hides the slot."
                            }
                            dopesheet_selection::TimelineRow::SlotColor => {
                                "Slot RGBA color keys preserve their four channel curves."
                            }
                            dopesheet_selection::TimelineRow::DrawOrder => {
                                "Global slot order keys hold until the next key."
                            }
                            _ => "IK constraint keys group Mix and Positive",
                        })
                    };
                    label.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &name)
                    });
                    if label.rect.is_positive() {
                        self.record_target(&format!("timelineRow:{qualified}"), label.rect);
                        if single_owner {
                            self.record_target(&target, label.rect);
                        }
                    }
                    if label.clicked() {
                        if let Some(tool) = tool {
                            self.set_tool(tool);
                        }
                        ui.ctx()
                            .memory_mut(|memory| memory.request_focus(Self::dopesheet_id()));
                    }
                    ui.painter_at(label_rect).text(
                        pos2(
                            rect.left()
                                + if row == dopesheet_selection::TimelineRow::DrawOrder {
                                    26.
                                } else {
                                    12.
                                },
                            y,
                        ),
                        egui::Align2::LEFT_CENTER,
                        name,
                        FontId::proportional(12.0),
                        if label.hovered() || label.has_focus() {
                            ACCENT
                        } else {
                            MUTED
                        },
                    );
                    let keys = shown
                        .keys
                        .iter()
                        .filter(|key| key.owner() == row_owner && row.matches((*key).clone()));
                    for (index, key_id) in keys.enumerate() {
                        let p = pos2(geometry.x_at(key_id.time()), y);
                        let key_rect = egui::Rect::from_center_size(p, vec2(9., 9.));
                        if !rows_clip.contains(p) || !grid.contains(p) {
                            continue;
                        }
                        if editable {
                            key_targets.push((key_id.clone(), p));
                        }
                        if editable {
                            self.record_target(
                                &format!("timelineKey:{qualified}:{index}"),
                                key_rect.intersect(rows_clip),
                            );
                        }
                        if single_owner && editable {
                            let legacy = match key_id {
                                KeyId::Slider {
                                    constraint,
                                    property,
                                    ..
                                } => {
                                    format!("timelineKey:slider:{constraint}:{property:?}:{index}")
                                }
                                KeyId::Bone { property, .. } => {
                                    format!("timelineKey:{property:?}:{index}")
                                }
                                KeyId::Ik { constraint, .. } => {
                                    format!("timelineKey:ik:{constraint}:{index}")
                                }
                                KeyId::Deform { .. } => format!("timelineKey:deform:{index}"),
                                KeyId::SlotAttachment { .. } => {
                                    format!("timelineKey:attachment:{index}")
                                }
                                KeyId::SlotColor { .. } => format!("timelineKey:color:{index}"),
                                KeyId::DrawOrder { .. } => format!("timelineKey:drawOrder:{index}"),
                            };
                            self.record_target(&legacy, key_rect.intersect(rows_clip));
                        }
                        painter.rect_filled(
                            egui::Rect::from_center_size(p, vec2(5., 13.)),
                            0.5,
                            if self.key_selection.contains(key_id.clone()) {
                                ACCENT
                            } else if editable {
                                color
                            } else {
                                MUTED
                            },
                        );
                    }
                }
            });
        let playhead = geometry.x_at(self.time);
        painter.line_segment(
            [
                pos2(playhead, rect.top() + 22.0),
                pos2(playhead, rect.bottom()),
            ],
            Stroke::new(1.5, ACCENT),
        );
        painter.circle_filled(pos2(playhead, rect.top() + 22.0), 4.0, ACCENT);
        let hit_key = |pointer: Pos2| {
            key_targets
                .iter()
                .filter(|(_, point)| point.distance(pointer) < 9.)
                .min_by(|(_, a), (_, b)| a.distance(pointer).total_cmp(&b.distance(pointer)))
                .map(|(key, _)| key.clone())
        };
        let (press_event, release_event) = ui.input(|input| {
            (
                primary_pointer_event(&input.events, true),
                primary_pointer_event(&input.events, false),
            )
        });
        if self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
            && self.export_dialog.is_none()
            && !egui::Popup::is_any_open(ui.ctx())
            && ui.input(|input| input.focused)
            && (response.contains_pointer() || response.is_pointer_button_down_on())
            && let Some((origin, modifiers)) = press_event
            && let Some(pressed) = hit_key(origin)
        {
            self.cancel_gesture();
            self.reconcile_dopesheet_selection();
            let presentation_owner = self.timeline_owner();
            let primary_before = self.key_selection.primary().cloned();
            self.key_drag = KeyDrag::new(
                self.animation,
                pressed,
                self.key_selection.keys(),
                geometry,
                origin.x,
                modifiers,
            );
            if let Some(drag) = self.key_drag.as_mut() {
                drag.presentation_owner = presentation_owner;
                drag.primary_before = primary_before;
            }
            response.request_focus();
        }
        let released = response.drag_stopped_by(egui::PointerButton::Primary);
        if let Some(pointer) = response.interact_pointer_pos()
            && self.key_drag.as_ref().is_some_and(|drag| {
                drag.should_preview(
                    pointer.x,
                    ui.ctx()
                        .options(|options| options.input_options.max_click_dist),
                    response.dragged_by(egui::PointerButton::Primary),
                    released,
                    response.clicked(),
                )
            })
        {
            let unsnapped = release_event.map_or_else(
                || ui.input(|input| input.modifiers.shift),
                |(_, modifiers)| modifiers.shift,
            );
            self.update_dopesheet_drag(pointer.x, unsnapped);
            response.request_focus();
            ui.ctx().request_repaint();
        }
        if released && self.key_drag.as_ref().is_some_and(|drag| drag.active) {
            self.finish_dopesheet_drag();
        } else if response.clicked()
            || (released && self.key_drag.as_ref().is_some_and(|drag| !drag.active))
        {
            // A press that never crossed the drag threshold keeps ordinary click semantics.
            let toggle = self.key_drag.as_ref().map_or_else(
                || release_event.is_some_and(|(_, modifiers)| modifiers.command || modifiers.ctrl),
                |drag| drag.toggle,
            );
            self.finish_dopesheet_drag();
            response.request_focus();
            let pointer = release_event
                .map(|(position, _)| position)
                .or_else(|| response.interact_pointer_pos());
            if let Some(key) = pointer.and_then(hit_key) {
                self.playing = false;
                if !toggle {
                    self.set_time(key.time());
                }
                if response.double_clicked() {
                    self.cancel_gesture();
                    self.dispatch(Command::DeleteAnimationKeys {
                        animation: self.animation,
                        keys: vec![key.clone()],
                    });
                } else {
                    self.key_selection.select(key, toggle);
                }
            } else {
                self.key_selection.clear();
                if let Some(pointer) = response.interact_pointer_pos() {
                    self.scrub_dopesheet(pointer, geometry);
                }
            }
        } else if released {
            self.finish_dopesheet_drag();
        } else if self.key_drag.is_none()
            && response.dragged_by(egui::PointerButton::Primary)
            && ui
                .input(|input| input.pointer.press_origin())
                .and_then(hit_key)
                .is_none()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            response.request_focus();
            self.scrub_dopesheet(pointer, geometry);
        }
        if self.slot_authoring_input_available(ui.ctx())
            && (ruler_response.clicked()
                || ruler_response.dragged_by(egui::PointerButton::Primary)
                || ruler_response.drag_stopped_by(egui::PointerButton::Primary))
            && let Some(pointer) = ruler_response.interact_pointer_pos()
        {
            ui.memory_mut(|memory| memory.request_focus(Self::dopesheet_id()));
            self.scrub_dopesheet(pointer, geometry);
        }
        // Keep one line reserved: selection changes must not resize the view and move key targets.
        let footer = ui.add(
            egui::Label::new(
                RichText::new(format!(
                    "{} selected · Delete to remove · Cmd/Ctrl-click toggles · Drag to retime{}",
                    self.key_selection.len(),
                    if shown.unsupported.is_empty() && shown.readonly_owners.is_empty() {
                        ""
                    } else {
                        " · read-only timelines shown; whole-group edits unavailable"
                    }
                ))
                .small()
                .color(MUTED),
            )
            .truncate(),
        );
        self.record_target("timelineFooter", footer.rect);
    }
    fn scrub_dopesheet(&mut self, pointer: Pos2, geometry: ChartGeometry) {
        if pointer.x >= geometry.start
            && let Some(time) = geometry.time_at(pointer.x)
        {
            self.playing = false;
            self.set_time(animation::snap_time(time, self.session.document().fps));
        }
    }
}

pub(super) fn transport_button(
    ui: &mut egui::Ui,
    direction: PlayDirection,
    active: bool,
) -> egui::Response {
    let response = ui.add_sized([25., 22.], egui::Button::selectable(active, ""));
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            if active {
                "Stop playback"
            } else if direction == PlayDirection::Forward {
                "Play forward"
            } else {
                "Play backward"
            },
        )
    });
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
