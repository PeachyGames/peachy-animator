//! The Properties swatch opens a frozen, single-transaction color dialog.
use super::*;

pub(super) struct ColorDialog {
    pub(super) context: Context,
    pub(super) pose: peachy_engine::PoseEdits,
    pub(super) color: [f32; 4],
    rgba: [String; 4],
    rgba_dirty: [bool; 4],
    hsb: [String; 3],
    hsb_dirty: [bool; 3],
}

impl Editor {
    pub(crate) fn slot_color_dialog_open(&self) -> bool {
        self.slot_color.dialog.is_some()
    }

    pub(super) fn open_slot_color_dialog(&mut self, ctx: &egui::Context) {
        if !self.slot_authoring_input_available(ctx) {
            return;
        }
        let Some(context) = self.slot_color_context() else {
            return;
        };
        self.cancel_gesture();
        self.playing = false;
        self.reconcile_slot_color_input(ctx);
        self.session.commit_transaction();
        self.session.begin_transaction();
        self.slot_color.dialog = Some(ColorDialog {
            context,
            pose: self.session.pose_edits().clone(),
            color: self.slot_color.raw,
            rgba: std::array::from_fn(|index| self.slot_color.color[index].to_string()),
            rgba_dirty: [false; 4],
            hsb: self.slot_color_hsb_text(),
            hsb_dirty: [false; 3],
        });
        self.slot_color.blocked = false;
        self.slot_color.error = None;
    }

    fn slot_color_hsb_text(&self) -> [String; 3] {
        [
            self.slot_color.hsv.h * 360.,
            self.slot_color.hsv.s * 100.,
            self.slot_color.hsv.v * 100.,
        ]
        .map(|value| format!("{value:.0}"))
    }

    pub(super) fn sync_slot_color_dialog(&mut self) {
        let hsb = self.slot_color_hsb_text();
        if let Some(dialog) = self.slot_color.dialog.as_mut() {
            dialog.pose = self.session.pose_edits().clone();
            dialog.color = self.slot_color.raw;
            for index in 0..4 {
                if !dialog.rgba_dirty[index] {
                    dialog.rgba[index] = self.slot_color.color[index].to_string();
                }
            }
            for (index, value) in hsb.into_iter().enumerate() {
                if !dialog.hsb_dirty[index] {
                    dialog.hsb[index] = value;
                }
            }
        }
    }

    fn apply_slot_color_dialog_field(&mut self, hsv: bool, index: usize) -> bool {
        let Some(dialog) = self.slot_color.dialog.as_ref() else {
            return false;
        };
        let (dirty, text) = if hsv {
            (dialog.hsb_dirty[index], &dialog.hsb[index])
        } else {
            (dialog.rgba_dirty[index], &dialog.rgba[index])
        };
        if !dirty {
            return true;
        }
        let Some(value) = text.parse::<f64>().ok().filter(|value| value.is_finite()) else {
            self.slot_color.error = Some("Enter a finite color value.".into());
            return false;
        };
        let mut raw = self.slot_color.raw;
        if hsv {
            let limit = if index == 0 { 360. } else { 100. };
            let value = value.clamp(0., limit) as f32 / limit as f32;
            match index {
                0 => self.slot_color.hsv.h = value,
                1 => self.slot_color.hsv.s = value,
                _ => self.slot_color.hsv.v = value,
            }
            let bytes = slot_hsv_bytes(self.slot_color.hsv);
            if bytes[..3] != self.slot_color.color[..3] {
                for channel in 0..3 {
                    raw[channel] = f32::from(bytes[channel]) / 255.;
                }
            }
            self.slot_color.dialog.as_mut().unwrap().hsb_dirty[index] = false;
        } else {
            let byte = value.clamp(0., 255.).trunc() as u8;
            if byte != self.slot_color.color[index] {
                raw[index] = f32::from(byte) / 255.;
            }
            self.slot_color.dialog.as_mut().unwrap().rgba_dirty[index] = false;
        }
        if raw != self.slot_color.raw {
            self.preview_slot_color_raw(raw);
        }
        if self.slot_color.dialog.is_none() {
            return false;
        }
        if !hsv {
            self.slot_color.hsv = slot_hsv(self.slot_color.raw);
            self.slot_color.hsv.a = self.slot_color.raw[3].clamp(0., 1.);
        }
        self.slot_color.error = None;
        self.slot_color.hex = hex(self.slot_color.color);
        self.sync_slot_color_dialog();
        true
    }

    fn slot_color_dialog_fields(&mut self, ui: &mut egui::Ui, hsv: bool) {
        let labels: &[&str] = if hsv {
            &["H", "S", "B"]
        } else {
            &["R", "G", "B", "A"]
        };
        for (index, label) in labels.iter().enumerate() {
            let Some(dialog) = self.slot_color.dialog.as_ref() else {
                return;
            };
            let mut text = if hsv {
                dialog.hsb[index].clone()
            } else {
                dialog.rgba[index].clone()
            };
            let response = ui
                .horizontal(|ui| {
                    ui.add_sized([14., 20.], egui::Label::new(*label));
                    ui.add(
                        egui::TextEdit::singleline(&mut text)
                            .id_salt(("slot-color-channel", hsv, index))
                            .desired_width(47.),
                    )
                })
                .inner;
            self.slot_color_control(
                &format!(
                    "slotColorDialog{}:{index}",
                    if hsv { "HSB" } else { "RGBA" }
                ),
                &response,
            );
            if response.changed()
                && let Some(dialog) = self.slot_color.dialog.as_mut()
            {
                if hsv {
                    dialog.hsb[index] = text;
                    dialog.hsb_dirty[index] = true;
                } else {
                    dialog.rgba[index] = text;
                    dialog.rgba_dirty[index] = true;
                }
            }
            if (response.lost_focus()
                || (response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter))))
                && !self.apply_slot_color_dialog_field(hsv, index)
            {
                response.request_focus();
            }
        }
    }

    fn accept_slot_color_dialog(&mut self) {
        for hsv in [true, false] {
            for index in 0..if hsv { 3 } else { 4 } {
                if !self.apply_slot_color_dialog_field(hsv, index) {
                    return;
                }
            }
        }
        let Some(bytes) = parse_hex(&self.slot_color.hex) else {
            self.slot_color.error = Some("Use #RRGGBBAA: eight hex digits.".into());
            return;
        };
        if bytes != self.slot_color.color {
            self.preview_slot_color(bytes);
        }
        if self.slot_color.dialog.take().is_none() {
            return;
        }
        self.slot_color.edit = None;
        self.slot_color.context = None;
        self.slot_color.last_focus = None;
        self.slot_color.epoch = self.slot_color.epoch.wrapping_add(1);
        self.session.commit_transaction();
    }

    pub(crate) fn slot_color_dialog(&mut self, ctx: &egui::Context) {
        if self.slot_color.dialog.is_none() {
            return;
        }
        self.reconcile_slot_color_input(ctx);
        if self.slot_color.dialog.is_none() {
            return;
        }
        self.sync_slot_color_dialog();
        self.slot_color.controls.clear();
        let response = egui::Modal::new(egui::Id::new("set-slot-color")).show(ctx, |ui| {
            ui.heading("Set slot color");
            ui.separator();
            ui.horizontal(|ui| {
                let side = 430.;
                for (name, axis) in [
                    ("slotColorDialogAlpha", 2),
                    ("slotColorDialogSV", 0),
                    ("slotColorDialogHue", 1),
                ] {
                    let (_, rect) =
                        ui.allocate_space(vec2(if axis == 0 { side } else { 26. }, side));
                    let input =
                        ui.interact(rect, ui.make_persistent_id(name), Sense::click_and_drag());
                    if axis == 0 {
                        paint_sv(ui.painter(), rect, self.slot_color.hsv.h);
                        ui.painter().circle_stroke(
                            pos2(
                                rect.left() + self.slot_color.hsv.s * rect.width(),
                                rect.bottom() - self.slot_color.hsv.v * rect.height(),
                            ),
                            3.,
                            Stroke::new(1., Color32::WHITE),
                        );
                    } else {
                        paint_vertical_strip(ui.painter(), rect, self.slot_color.hsv, axis == 2);
                        let fraction = if axis == 2 {
                            1. - self.slot_color.hsv.a
                        } else {
                            self.slot_color.hsv.h
                        };
                        let y = rect.top() + fraction * rect.height();
                        ui.painter().line_segment(
                            [pos2(rect.left(), y), pos2(rect.right(), y)],
                            Stroke::new(1., Color32::WHITE),
                        );
                    }
                    self.slot_color_control(name, &input);
                    self.slot_color_picker_response(&input, axis);
                }
                ui.vertical(|ui| {
                    ui.set_width(105.);
                    let (_, rect) = ui.allocate_space(vec2(90., 70.));
                    paint_swatch(ui.painter(), rect, self.slot_color.color);
                    self.record_target("slotColorDialogSwatch", rect);
                    self.slot_color_dialog_fields(ui, true);
                    ui.separator();
                    self.slot_color_dialog_fields(ui, false);
                    self.slot_color_hex(ui, true);
                });
            });
            if let Some(error) = self.slot_color.error.as_ref() {
                ui.colored_label(ACCENT, error);
            }
            ui.separator();
            ui.horizontal(|ui| {
                let ok = ui.button("OK");
                self.record_target("slotColorDialogOK", ok.rect);
                let cancel = ui.button("Cancel");
                self.record_target("slotColorDialogCancel", cancel.rect);
                if cancel.clicked() {
                    self.cancel_slot_color_edit();
                } else if ok.clicked() {
                    self.accept_slot_color_dialog();
                }
            });
        });
        self.record_target("slotColorDialog", response.response.rect);
        if response.should_close() && self.slot_color.dialog.is_some() {
            self.cancel_slot_color_edit();
        }
    }
}

fn paint_vertical_strip(painter: &egui::Painter, rect: egui::Rect, hsv: Hsva, alpha: bool) {
    if alpha {
        for x in 0..(rect.width() / 8.).ceil() as usize {
            for y in 0..(rect.height() / 8.).ceil() as usize {
                let min = rect.min + vec2(x as f32 * 8., y as f32 * 8.);
                painter.rect_filled(
                    egui::Rect::from_min_max(min, (min + vec2(8., 8.)).min(rect.max)),
                    0.,
                    Color32::from_gray(if (x + y) % 2 == 0 { 75 } else { 130 }),
                );
            }
        }
    }
    let mut mesh = egui::Mesh::default();
    for index in 0..=32 {
        let fraction = index as f32 / 32.;
        let color = slot_hsv_color(if alpha {
            Hsva {
                a: 1. - fraction,
                ..hsv
            }
        } else {
            Hsva {
                h: fraction,
                s: 1.,
                v: 1.,
                a: 1.,
            }
        });
        let y = rect.top() + fraction * rect.height();
        mesh.colored_vertex(pos2(rect.left(), y), color);
        mesh.colored_vertex(pos2(rect.right(), y), color);
    }
    for index in 0..32 {
        let a = index * 2;
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
    painter.add(egui::Shape::mesh(mesh));
}
