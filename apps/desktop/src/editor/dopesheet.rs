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
            && self.show_dopesheet
            && self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
            && !egui::Popup::is_any_open(ctx)
            && !ctx.text_edit_focused()
            && ctx.memory(|memory| memory.has_focus(Self::dopesheet_id()))
    }
    pub(super) fn reconcile_dopesheet_selection(&mut self) {
        let animation = self.active_animation();
        let bone = self.selected_bone();
        self.key_selection
            .reconcile(self.session.document(), animation, bone);
    }
    pub(super) fn delete_dopesheet_keys(&mut self) {
        self.cancel_gesture();
        self.reconcile_dopesheet_selection();
        if let Some(command) = self.key_selection.deletion() {
            self.playing = false;
            self.dispatch(command);
        }
    }
    pub(super) fn timeline(&mut self, ui: &mut egui::Ui) {
        self.reconcile_dopesheet_selection();
        let view_rect = ui.max_rect();
        self.record_target("dopesheet", view_rect);
        ui.horizontal(|ui| {
            icons::heading(ui, Icon::Dopesheet, "Dopesheet");
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
        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), 157.0));
        let response = ui.interact(rect, Self::dopesheet_id(), Sense::click_and_drag());
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
        let label_width = 160.0;
        let geometry = self.key_drag.as_ref().map_or(
            ChartGeometry {
                start: rect.left() + label_width,
                width: rect.width() - label_width - 18.0,
                duration: self.duration(),
            },
            |drag| drag.geometry,
        );
        let ChartGeometry {
            start,
            width,
            duration,
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
        let mut key_targets = Vec::new();
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
                for (index, key) in track.keys.iter().enumerate() {
                    let x = start + width * key.time / duration;
                    let p = pos2(x, y);
                    let key_id = KeyId {
                        property,
                        time: key.time,
                    };
                    let key_rect = egui::Rect::from_center_size(p, vec2(9., 9.));
                    key_targets.push((key_id, p));
                    self.record_target(&format!("timelineKey:{property:?}:{index}"), key_rect);
                    let color = match property {
                        Property::Rotation => Color32::from_rgb(109, 189, 126),
                        Property::X | Property::Y => Color32::from_rgb(105, 168, 225),
                        Property::ScaleX | Property::ScaleY => Color32::from_rgb(211, 116, 116),
                    };
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
        let hit_key = |pointer: Pos2| {
            key_targets
                .iter()
                .filter(|(_, point)| point.distance(pointer) < 9.)
                .min_by(|(_, a), (_, b)| a.distance(pointer).total_cmp(&b.distance(pointer)))
                .map(|(key, _)| *key)
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
            && !egui::Popup::is_any_open(ui.ctx())
            && ui.input(|input| input.focused)
            && (response.contains_pointer() || response.is_pointer_button_down_on())
            && let Some((origin, modifiers)) = press_event
            && let Some(pressed) = hit_key(origin)
            && let Some(bone) = self.selected_bone()
        {
            self.cancel_gesture();
            self.reconcile_dopesheet_selection();
            self.key_drag = KeyDrag::new(
                self.animation,
                bone,
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
                self.set_time(key.time);
                if response.double_clicked() {
                    self.cancel_gesture();
                    if let Some(bone) = self.selected_bone() {
                        self.dispatch(Command::DeleteKey {
                            animation: self.animation,
                            bone,
                            property: key.property,
                            time: key.time,
                        });
                    }
                } else {
                    self.key_selection.select(key, toggle);
                }
            } else {
                self.key_selection.clear();
                if let Some(pointer) = response.interact_pointer_pos() {
                    self.scrub_dopesheet(pointer, start, width, duration);
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
            self.scrub_dopesheet(pointer, start, width, duration);
        }
        if !self.key_selection.is_empty() {
            ui.label(
                RichText::new(format!(
                    "{} selected · Delete to remove · Cmd/Ctrl-click toggles · Drag to retime",
                    self.key_selection.len()
                ))
                .small()
                .color(MUTED),
            );
        }
    }
    fn scrub_dopesheet(&mut self, pointer: Pos2, start: f32, width: f32, duration: f32) {
        if pointer.x >= start {
            self.playing = false;
            let fps = self.session.document().fps as f32;
            let time = ((pointer.x - start) / width * duration).clamp(0.0, duration);
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
