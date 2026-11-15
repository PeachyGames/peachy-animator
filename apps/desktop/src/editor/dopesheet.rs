//! Dopesheet controls adapt pointer/keyboard input to shared commands.
use super::dopesheet_selection::KeyId;
use super::icons::Icon;
use super::key_drag::{ChartGeometry, KeyDrag, primary_pointer_event};
use super::*;

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
        self.key_selection
            .reconcile(self.session.document(), animation, owner);
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
            Some(TreeItem::Constraint(constraint)) => Some(peachy_engine::KeyOwner::Ik {
                constraint: *constraint,
            }),
            Some(TreeItem::Attachment(attachment)) if self.attachment_editable(attachment) => {
                peachy_engine::deform_key_owner(self.session.document(), attachment).ok()
            }
            _ => None,
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
        let response = ui.interact(grid, Self::dopesheet_id(), Sense::click_and_drag());
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
        let owner = self.timeline_owner();
        if owner == Some(peachy_engine::KeyOwner::DrawOrder {}) {
            let group = egui::Rect::from_min_size(rect.min, vec2(label_width, 24.));
            painter.text(
                pos2(group.left() + 12., group.center().y),
                egui::Align2::LEFT_CENTER,
                "Draw order",
                FontId::proportional(12.),
                Color32::from_rgb(80, 157, 224),
            );
            self.record_target("timelineGroup:drawOrder", group);
        }
        let rows = dopesheet_selection::rows(owner.clone());
        let mut key_targets = Vec::new();
        for (i, row) in rows.into_iter().enumerate() {
            let y = rect.top() + 26.0 + pitch * (i as f32 + 0.5);
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
                        Property::ScaleX | Property::ScaleY => Color32::from_rgb(211, 116, 116),
                        Property::ShearX | Property::ShearY => Color32::from_rgb(190, 145, 210),
                    };
                    (
                        property.label().to_owned(),
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
                dopesheet_selection::TimelineRow::Deform => {
                    let Some(peachy_engine::KeyOwner::Deform { attachment }) = &owner else {
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
            let label_rect = egui::Rect::from_min_max(
                pos2(rect.left() + 6.0, y - 10.0),
                pos2(rect.left() + label_width - 6.0, y + 10.0),
            )
            .intersect(rect);
            let label = ui
                .interact(
                    label_rect,
                    ui.id().with(("timeline-row", i)),
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
            self.record_target(&target, label.rect);
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
            let keys = owner.clone().map_or_else(Vec::new, |owner| {
                dopesheet_selection::row_keys(self.session.document(), self.animation, owner, row)
            });
            for (index, key_id) in keys.into_iter().enumerate() {
                let p = pos2(geometry.x_at(key_id.time()), y);
                let key_rect = egui::Rect::from_center_size(p, vec2(9., 9.));
                key_targets.push((key_id.clone(), p));
                let target = match &key_id {
                    KeyId::Bone { property, .. } => format!("timelineKey:{property:?}:{index}"),
                    KeyId::Ik { constraint, .. } => format!("timelineKey:ik:{constraint}:{index}"),
                    KeyId::Deform { .. } => format!("timelineKey:deform:{index}"),
                    KeyId::SlotAttachment { .. } => format!("timelineKey:attachment:{index}"),
                    KeyId::SlotColor { .. } => format!("timelineKey:color:{index}"),
                    KeyId::DrawOrder { .. } => format!("timelineKey:drawOrder:{index}"),
                };
                self.record_target(&target, key_rect);
                painter.rect_filled(
                    egui::Rect::from_center_size(p, vec2(5., 13.)),
                    0.5,
                    if self.key_selection.contains(key_id) {
                        ACCENT
                    } else {
                        color
                    },
                );
            }
        }
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
            self.key_drag = KeyDrag::new(
                self.animation,
                pressed,
                self.key_selection.keys(),
                geometry,
                origin.x,
                modifiers,
            );
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
                self.set_time(key.time());
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
        // Keep one line reserved: selection changes must not resize the view and move key targets.
        let footer = ui.add(
            egui::Label::new(
                RichText::new(format!(
                    "{} selected · Delete to remove · Cmd/Ctrl-click toggles · Drag to retime",
                    self.key_selection.len()
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
