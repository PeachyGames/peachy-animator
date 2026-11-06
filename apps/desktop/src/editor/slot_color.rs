//! Exact unmultiplied slot bytes; this view owns input, Session owns transaction history.
use super::*;
use egui::ecolor::Hsva;

#[derive(Clone, PartialEq)]
struct Context {
    slot: usize,
    name: String,
    selection: TreeItem,
    skin: String,
    setup: bool,
    animation: usize,
    time: f32,
    tool: Tool,
}

#[derive(Clone, Copy, PartialEq)]
enum Input {
    Picker,
    Hex,
}

struct Edit {
    context: Context,
    baseline: [u8; 4],
    control: egui::Id,
    input: Input,
    commit_requested: bool,
    finish_pending: bool,
}

pub(super) struct SlotColorView {
    context: Option<Context>,
    color: [u8; 4],
    hsv: Hsva,
    hex: String,
    edit: Option<Edit>,
    error: Option<String>,
    blocked: bool,
    epoch: u64,
    aborted: Option<egui::Id>,
    controls: Vec<egui::Id>,
    last_focus: Option<egui::Id>,
}

impl Default for SlotColorView {
    fn default() -> Self {
        Self {
            context: None,
            color: [255; 4],
            hsv: Hsva::from_srgba_unmultiplied([255; 4]),
            hex: hex([255; 4]),
            edit: None,
            error: None,
            blocked: false,
            epoch: 0,
            aborted: None,
            controls: Vec::new(),
            last_focus: None,
        }
    }
}

impl SlotColorView {
    pub(super) fn reset_document(&mut self) {
        let epoch = self.epoch.wrapping_add(1);
        *self = Self {
            epoch,
            ..Default::default()
        };
    }
}

impl Editor {
    fn slot_color_context(&self) -> Option<Context> {
        let doc = self.session.document();
        let selection = self.selection.as_ref()?;
        let slot = match selection {
            TreeItem::Slot(slot) if *slot < doc.slots.len() => *slot,
            TreeItem::Attachment(key) if doc.attachment(key).is_some() => {
                doc.slots.iter().position(|slot| slot.name == key.slot)?
            }
            _ => return None,
        };
        Some(Context {
            slot,
            name: doc.slots[slot].name.clone(),
            selection: selection.clone(),
            skin: self.skin.clone(),
            setup: self.setup_mode,
            animation: self.animation,
            time: self.time,
            tool: self.tool,
        })
    }

    pub(super) fn slot_color_edit_active(&self) -> bool {
        self.slot_color.edit.is_some()
    }

    pub(super) fn cancel_slot_color_edit(&mut self) {
        if let Some(edit) = self.slot_color.edit.take() {
            self.session.cancel_transaction();
            self.slot_color.aborted = Some(edit.control);
            self.slot_color.blocked = true;
            self.slot_color.context = None;
            self.slot_color.epoch = self.slot_color.epoch.wrapping_add(1);
            self.slot_color.last_focus = None;
            self.slot_color.error = None;
        }
    }

    pub(super) fn reconcile_slot_color_input(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.pointer.primary_down()) {
            self.slot_color.blocked = false;
        }
        let context = self.slot_color_context();
        if self.slot_color.edit.as_ref().is_some_and(|edit| {
            Some(&edit.context) != context.as_ref()
                || !self.setup_mode
                || !self.slot_color_visible()
                || !ctx.input(|input| input.focused)
                || self.error.is_some()
                || self.rename.is_some()
                || self.duplicate_animation.is_some()
                || egui::Popup::is_any_open(ctx)
        }) {
            self.cancel_slot_color_edit();
        }
        if let Some(id) = self.slot_color.aborted.take() {
            ctx.memory_mut(|memory| memory.surrender_focus(id));
        }
        let color = context
            .as_ref()
            .map(|context| self.session.document().slots[context.slot].color);
        if self.slot_color.context != context
            || (self.slot_color.edit.is_none()
                && color.is_some_and(|color| color != self.slot_color.color))
        {
            self.slot_color.context = context;
            self.slot_color.epoch = self.slot_color.epoch.wrapping_add(1);
            self.slot_color.last_focus = None;
            self.slot_color.error = None;
            if let Some(color) = color {
                self.slot_color.color = color;
                self.slot_color.hsv = Hsva::from_srgba_unmultiplied(color);
                self.slot_color.hex = hex(color);
            }
        }
    }

    fn begin_slot_color_edit(&mut self, control: egui::Id, input: Input) -> bool {
        let Some(context) = self.slot_color_context().filter(|context| context.setup) else {
            return false;
        };
        if self.slot_color.blocked || !self.slot_color_visible() {
            return false;
        }
        self.cancel_gesture();
        self.session.commit_transaction();
        self.session.begin_transaction();
        self.slot_color.edit = Some(Edit {
            baseline: self.session.document().slots[context.slot].color,
            context,
            control,
            input,
            commit_requested: false,
            finish_pending: false,
        });
        true
    }

    fn preview_slot_color(&mut self, color: [u8; 4]) {
        let Some(edit) = self.slot_color.edit.as_ref() else {
            return;
        };
        match self.session.preview_transaction(Command::SetSlotColor {
            slot: edit.context.slot,
            color,
        }) {
            Ok(()) => {
                self.slot_color.color = color;
                self.slot_color.error = None;
            }
            Err(error) => {
                self.cancel_slot_color_edit();
                self.slot_color.error = Some(error);
            }
        }
    }

    pub(super) fn settle_slot_color_input(&mut self, ctx: &egui::Context) {
        self.reconcile_slot_color_input(ctx);
        if !ctx.input(|input| input.pointer.any_down())
            && self
                .slot_color
                .edit
                .as_ref()
                .is_some_and(|edit| edit.finish_pending)
        {
            let edit = self.slot_color.edit.take().unwrap();
            self.session.commit_transaction();
            if edit.input == Input::Hex {
                ctx.memory_mut(|memory| memory.surrender_focus(edit.control));
            }
            // Hue remains useful on gray even though it has no representation
            // in RGBA bytes. Keep our cache across our own no-op commits.
            self.slot_color.hex = hex(self.slot_color.color);
        }
    }

    pub(super) fn observe_slot_color_focus(&mut self, ctx: &egui::Context) {
        self.slot_color.last_focus = ctx
            .memory(|memory| memory.focused())
            .filter(|id| self.slot_color_visible() && self.slot_color.controls.contains(id));
    }

    pub(super) fn slot_color_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        if !self.slot_color_visible() || !ctx.input(|input| input.focused) {
            return false;
        }
        let escape = ctx.input(|input| input.key_pressed(egui::Key::Escape));
        let focus = ctx
            .memory(|memory| memory.focused())
            .or_else(|| escape.then_some(self.slot_color.last_focus).flatten());
        let owns_focus = focus.is_some_and(|id| self.slot_color.controls.contains(&id));
        if escape && (self.slot_color.edit.is_some() || owns_focus) {
            ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            self.cancel_slot_color_edit();
            if let Some(id) = focus {
                ctx.memory_mut(|memory| memory.surrender_focus(id));
            }
            return true;
        }
        if (self.slot_color.edit.is_some() || owns_focus)
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F9))
        {
            self.cancel_slot_color_edit();
            self.toggle_views();
            return true;
        }
        let commit_key = ctx.input(|input| {
            input.key_pressed(egui::Key::Enter) || input.key_pressed(egui::Key::Tab)
        });
        let owns_text = self.slot_color.edit.as_ref().is_some_and(|edit| {
            edit.input == Input::Hex && (focus == Some(edit.control) || commit_key)
        });
        if owns_text {
            for key in [egui::Key::S, egui::Key::O, egui::Key::E] {
                if ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, key)) {
                    self.cancel_slot_color_edit();
                    match key {
                        egui::Key::S => self.save_project(),
                        egui::Key::O => self.open_project(ctx),
                        _ => self.export(),
                    }
                    return true;
                }
            }
            if commit_key {
                self.slot_color.edit.as_mut().unwrap().commit_requested = true;
            }
            // TextEdit receives text, Tab/Enter and its own undo before global tools/history.
            return true;
        }
        false
    }

    fn slot_color_control(&mut self, name: &str, response: &egui::Response) {
        self.record_target(name, response.rect);
        self.slot_color.controls.push(response.id);
    }

    pub(super) fn slot_color_panel(&mut self, ui: &mut egui::Ui) {
        self.reconcile_slot_color_input(ui.ctx());
        self.slot_color.controls.clear();
        self.record_target("slotColorPanelClip", ui.clip_rect());
        let Some(context) = self.slot_color.context.clone() else {
            ui.label("Select a slot or attachment to inspect its slot color.");
            return;
        };
        let title = ui.add(egui::Label::new(format!("Slot: {}", context.name)).truncate());
        self.record_target("slotColorTarget", title.rect);
        let enabled = context.setup
            && !self.slot_color.blocked
            && ui.input(|input| input.focused)
            && self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
            && !egui::Popup::is_any_open(ui.ctx());
        ui.push_id(("slot-color", context.slot, self.slot_color.epoch), |ui| {
            self.slot_color_hex(ui, enabled);
            let color = self.slot_color.color;
            let feedback = ui.label(
                RichText::new(format!(
                    "RGBA {} {} {} {}",
                    color[0], color[1], color[2], color[3]
                ))
                .small()
                .monospace(),
            );
            self.record_target("slotColorRGBA", feedback.rect);
            if !context.setup {
                ui.label(
                    RichText::new("Setup color only. Slot color animation is not available yet.")
                        .small()
                        .color(MUTED),
                );
            }
            if let Some(error) = self.slot_color.error.as_ref() {
                ui.label(RichText::new(error).small().color(ACCENT));
            }
            let strip_height = ui.spacing().interact_size.y.max(16.);
            let remaining = ui.clip_rect().bottom() - ui.cursor().top();
            let height =
                (remaining - 2. * strip_height - 3. * ui.spacing().item_spacing.y).clamp(32., 160.);
            ui.add_enabled_ui(enabled, |ui| {
                let (_, rect) = ui.allocate_space(vec2(ui.available_width(), height));
                let response =
                    ui.interact(rect, ui.make_persistent_id("sv"), Sense::click_and_drag());
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Slider,
                        ui.is_enabled(),
                        "Slot Color saturation and value",
                    )
                });
                paint_sv(ui.painter(), rect, self.slot_color.hsv.h);
                let marker = pos2(
                    rect.left() + self.slot_color.hsv.s * rect.width(),
                    rect.bottom() - self.slot_color.hsv.v * rect.height(),
                );
                ui.painter()
                    .circle_stroke(marker, 3., Stroke::new(1., Color32::WHITE));
                self.slot_color_control("slotColorSV", &response);
                self.slot_color_picker_response(&response, 0);
                for (name, label, axis) in
                    [("slotColorHue", "Hue", 1), ("slotColorAlpha", "Alpha", 2)]
                {
                    ui.horizontal(|ui| {
                        ui.add_sized([36., strip_height], egui::Label::new(label));
                        let (_, rect) = ui.allocate_space(vec2(ui.available_width(), strip_height));
                        let response =
                            ui.interact(rect, ui.make_persistent_id(name), Sense::click_and_drag());
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Slider,
                                ui.is_enabled(),
                                format!("Slot Color {label}"),
                            )
                        });
                        paint_strip(ui.painter(), rect, self.slot_color.hsv, axis == 2);
                        let value = if axis == 1 {
                            self.slot_color.hsv.h
                        } else {
                            self.slot_color.hsv.a
                        };
                        let x = rect.left() + value * rect.width();
                        ui.painter().line_segment(
                            [pos2(x, rect.top()), pos2(x, rect.bottom())],
                            Stroke::new(1., Color32::WHITE),
                        );
                        self.slot_color_control(name, &response);
                        self.slot_color_picker_response(&response, axis);
                    });
                }
            });
        });
        if self
            .slot_color
            .edit
            .as_ref()
            .is_some_and(|edit| edit.input == Input::Picker)
            && !ui.input(|input| input.pointer.primary_down())
        {
            self.slot_color.edit.as_mut().unwrap().finish_pending = true;
        }
    }

    fn slot_color_hex(&mut self, ui: &mut egui::Ui, enabled: bool) {
        ui.horizontal(|ui| {
            ui.label("Hex");
            let id = ui.make_persistent_id("hex");
            let response = ui.add_enabled(enabled,egui::TextEdit::singleline(&mut self.slot_color.hex)
                .id(id).desired_width(100.).char_limit(9).font(egui::TextStyle::Monospace))
                .on_hover_text("#RRGGBBAA, unmultiplied bytes. Enter/Tab applies; Escape or clicking away cancels.");
            response.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::TextEdit,enabled,"Slot Color RGBA hex"));
            self.slot_color_control("slotColorHex",&response);
            if enabled && (response.gained_focus() || response.has_focus()) && self.slot_color.edit.is_none() {
                self.begin_slot_color_edit(id,Input::Hex);
            }
            if self.slot_color.edit.as_ref().is_some_and(|edit|edit.input == Input::Hex && edit.control == id) {
                let commit = self.slot_color.edit.as_ref().unwrap().commit_requested;
                if commit {
                    self.slot_color.edit.as_mut().unwrap().commit_requested = false;
                    match parse_hex(&self.slot_color.hex) {
                        Some(color) => {
                            let changed = color != self.slot_color.color;
                            self.preview_slot_color(color);
                            if changed {self.slot_color.hsv = Hsva::from_srgba_unmultiplied(color);}
                            if let Some(edit) = self.slot_color.edit.as_mut() {edit.finish_pending = true;}
                        }
                        None => {
                            self.slot_color.error = Some("Use #RRGGBBAA: eight hex digits.".into());
                            response.request_focus();
                        }
                    }
                } else if response.lost_focus() {
                    self.cancel_slot_color_edit();
                }
                if response.changed() && !commit {self.slot_color.error = None;}
            }
            let (_,rect) = ui.allocate_space(vec2(44.,20.));
            self.record_target("slotColorSwatch",rect);
            paint_swatch(ui.painter(),rect,self.slot_color.color);
        });
    }

    fn slot_color_picker_response(&mut self, response: &egui::Response, axis: u8) {
        if !response.enabled() || self.slot_color.blocked {
            return;
        }
        let pressed = response.is_pointer_button_down_on()
            && response
                .ctx
                .input(|input| input.pointer.button_pressed(egui::PointerButton::Primary));
        let active = self
            .slot_color
            .edit
            .as_ref()
            .is_some_and(|edit| edit.input == Input::Picker && edit.control == response.id);
        let released = active
            && response
                .ctx
                .input(|input| input.pointer.button_released(egui::PointerButton::Primary));
        if !(pressed
            || response.dragged_by(egui::PointerButton::Primary)
            || response.clicked()
            || released)
        {
            return;
        }
        if !active && !self.begin_slot_color_edit(response.id, Input::Picker) {
            return;
        }
        response.request_focus();
        let Some(pointer) = response
            .interact_pointer_pos()
            .or_else(|| response.ctx.input(|input| input.pointer.interact_pos()))
            .filter(|point| point.is_finite())
        else {
            return;
        };
        let x = ((pointer.x - response.rect.left()) / response.rect.width()).clamp(0., 1.);
        let y = ((response.rect.bottom() - pointer.y) / response.rect.height()).clamp(0., 1.);
        let old = self.slot_color.hsv;
        match axis {
            0 => {
                self.slot_color.hsv.s = x;
                self.slot_color.hsv.v = y;
            }
            1 => self.slot_color.hsv.h = x,
            _ => self.slot_color.hsv.a = x,
        }
        if old != self.slot_color.hsv {
            let color = self.slot_color.hsv.to_srgba_unmultiplied();
            self.preview_slot_color(color);
            self.slot_color.hex = hex(color);
        }
    }

    pub(super) fn slot_color_diagnostic(&self) -> serde_json::Value {
        let context = self.slot_color_context();
        let color = context
            .as_ref()
            .map(|context| self.session.document().slots[context.slot].color);
        serde_json::json!({"slot":context.as_ref().map(|context|context.slot),"name":context.as_ref().map(|context|&context.name),
        "color":color,"hex":color.map(hex),"setup":self.setup_mode,"visible":self.slot_color_visible(),
        "blocked":self.slot_color.blocked,"error":self.slot_color.error,"active":self.slot_color.edit.as_ref().map(|edit|serde_json::json!({
            "slot":edit.context.slot,"baseline":edit.baseline,"kind":if edit.input == Input::Hex {"hex"} else {"picker"},"finishPending":edit.finish_pending
        }))})
    }
}

fn hex(color: [u8; 4]) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        color[0], color[1], color[2], color[3]
    )
}
fn parse_hex(text: &str) -> Option<[u8; 4]> {
    let digits = text.trim().strip_prefix('#').unwrap_or(text.trim());
    if digits.len() != 8 || !digits.is_ascii() {
        return None;
    }
    let mut color = [0; 4];
    for (index, channel) in color.iter_mut().enumerate() {
        *channel = u8::from_str_radix(&digits[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(color)
}

fn checker(painter: &egui::Painter, rect: egui::Rect) {
    let count = (rect.width() / 8.).ceil() as usize;
    for x in 0..count {
        for y in 0..3 {
            let min = rect.min + vec2(x as f32 * 8., y as f32 * 8.);
            let tile = egui::Rect::from_min_max(min, (min + vec2(8., 8.)).min(rect.max));
            if tile.is_positive() {
                painter.rect_filled(
                    tile,
                    0.,
                    Color32::from_gray(if (x + y) % 2 == 0 { 75 } else { 130 }),
                );
            }
        }
    }
}

fn paint_swatch(painter: &egui::Painter, rect: egui::Rect, color: [u8; 4]) {
    checker(painter, rect);
    painter.rect_filled(
        rect,
        0.,
        Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]),
    );
    let opaque = egui::Rect::from_min_max(pos2(rect.center().x, rect.top()), rect.max);
    painter.rect_filled(opaque, 0., Color32::from_rgb(color[0], color[1], color[2]));
}

fn paint_sv(painter: &egui::Painter, rect: egui::Rect, h: f32) {
    const N: u32 = 16;
    let mut mesh = egui::Mesh::default();
    for y in 0..=N {
        for x in 0..=N {
            let s = x as f32 / N as f32;
            let v = 1. - y as f32 / N as f32;
            mesh.colored_vertex(
                pos2(
                    rect.left() + s * rect.width(),
                    rect.bottom() - v * rect.height(),
                ),
                Color32::from(Hsva { h, s, v, a: 1. }),
            );
        }
    }
    for y in 0..N {
        for x in 0..N {
            let a = y * (N + 1) + x;
            mesh.add_triangle(a, a + 1, a + N + 1);
            mesh.add_triangle(a + 1, a + N + 2, a + N + 1);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn paint_strip(painter: &egui::Painter, rect: egui::Rect, hsv: Hsva, alpha: bool) {
    if alpha {
        checker(painter, rect);
    }
    let mut mesh = egui::Mesh::default();
    const N: u32 = 32;
    for index in 0..=N {
        let value = index as f32 / N as f32;
        let color = Color32::from(if alpha {
            Hsva { a: value, ..hsv }
        } else {
            Hsva {
                h: value,
                s: 1.,
                v: 1.,
                a: 1.,
            }
        });
        let x = rect.left() + value * rect.width();
        mesh.colored_vertex(pos2(x, rect.top()), color);
        mesh.colored_vertex(pos2(x, rect.bottom()), color);
    }
    for index in 0..N {
        let a = index * 2;
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
    painter.add(egui::Shape::mesh(mesh));
}

#[cfg(test)]
#[path = "slot_color_tests.rs"]
mod tests;
