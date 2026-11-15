//! Unmultiplied sampled slot color; this view owns input, Session owns pose/history.
use super::*;
use egui::ecolor::Hsva;
#[path = "slot_color_dialog.rs"]
mod dialog;
use dialog::ColorDialog;

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
    axes: Axes,
    auto_key: bool,
    pose_generation: u64,
}

#[derive(Clone, Copy, PartialEq)]
enum Input {
    Picker,
    Hex,
}

struct Edit {
    context: Context,
    baseline: [f32; 4],
    pose: peachy_engine::PoseEdits,
    control: egui::Id,
    input: Input,
    commit_requested: bool,
    finish_pending: bool,
}

pub(super) struct SlotColorView {
    context: Option<Context>,
    color: [u8; 4],
    raw: [f32; 4],
    hsv: Hsva,
    hex: String,
    edit: Option<Edit>,
    error: Option<String>,
    blocked: bool,
    epoch: u64,
    aborted: Option<egui::Id>,
    controls: Vec<egui::Id>,
    last_focus: Option<egui::Id>,
    dialog: Option<ColorDialog>,
}

impl Default for SlotColorView {
    fn default() -> Self {
        Self {
            context: None,
            color: [255; 4],
            raw: [1.; 4],
            hsv: slot_hsv([1.; 4]),
            hex: hex([255; 4]),
            edit: None,
            error: None,
            blocked: false,
            epoch: 0,
            aborted: None,
            controls: Vec::new(),
            last_focus: None,
            dialog: None,
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
    pub(super) fn slot_color_properties(&mut self, ui: &mut egui::Ui, slot: usize) {
        let Ok(pose) = self.current_slot_pose(slot) else {
            return;
        };
        let Ok(color) = peachy_engine::slot_color_bytes(pose.color) else {
            return;
        };
        ui.horizontal(|ui| {
            ui.label("Color");
            let (rect, response) = ui.allocate_exact_size(vec2(70., 20.), Sense::click());
            paint_swatch(ui.painter(), rect, color);
            self.record_target("slotColorButton", rect);
            if response
                .on_hover_text("Slot Color · RGBA before attachment tint")
                .clicked()
                && self.slot_authoring_input_available(ui.ctx())
            {
                self.open_slot_color_dialog(ui.ctx());
            }
            self.slot_property_key_controls(
                ui,
                slot,
                peachy_engine::SlotProperty::Color,
                "slotColor",
            );
        });
    }

    fn slot_color_context(&self) -> Option<Context> {
        let doc = self.session.document();
        let selection = self.selection.as_ref()?;
        let slot = self.selected_slot()?;
        Some(Context {
            slot,
            name: doc.slots[slot].name.clone(),
            selection: selection.clone(),
            skin: self.skin.clone(),
            setup: self.setup_mode,
            animation: self.animation,
            time: self.time,
            tool: self.tool,
            axes: self.axes,
            auto_key: self.auto_key,
            pose_generation: self.pose_view_generation,
        })
    }

    pub(super) fn slot_color_edit_active(&self) -> bool {
        self.slot_color.edit.is_some() || self.slot_color.dialog.is_some()
    }

    fn slot_color_input_visible(&self) -> bool {
        self.slot_color_visible() || self.slot_color.dialog.is_some()
    }

    fn slot_color_input_available(&self, ctx: &egui::Context) -> bool {
        ctx.input(|input| input.focused)
            && self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
            && !egui::Popup::is_any_open(ctx)
    }

    pub(super) fn cancel_slot_color_edit(&mut self) {
        let edit = self.slot_color.edit.take();
        let dialog = self.slot_color.dialog.take();
        if edit.is_some() || dialog.is_some() {
            self.session.cancel_transaction();
            self.slot_color.aborted = edit.map(|edit| edit.control).or(self.slot_color.last_focus);
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
        if self.slot_color.dialog.as_ref().is_some_and(|dialog| {
            Some(&dialog.context) != context.as_ref()
                || !self.slot_color_input_available(ctx)
                || self.session.pose_edits() != &dialog.pose
                || !self
                    .current_slot_pose(dialog.context.slot)
                    .is_ok_and(|pose| pose.color == dialog.color)
        }) {
            self.cancel_slot_color_edit();
        }
        if self.slot_color.edit.as_ref().is_some_and(|edit| {
            Some(&edit.context) != context.as_ref()
                || !self.slot_color_input_visible()
                || !ctx.input(|input| input.focused)
                || self.error.is_some()
                || self.rename.is_some()
                || self.duplicate_animation.is_some()
                || egui::Popup::is_any_open(ctx)
                || self.session.pose_edits() != &edit.pose
                || !self
                    .current_slot_pose(edit.context.slot)
                    .is_ok_and(|pose| pose.color == self.slot_color.raw)
        }) {
            self.cancel_slot_color_edit();
        }
        if let Some(id) = self.slot_color.aborted.take() {
            ctx.memory_mut(|memory| memory.surrender_focus(id));
        }
        let color = context.as_ref().and_then(|context| {
            self.current_slot_pose(context.slot)
                .ok()
                .map(|pose| pose.color)
        });
        if self.slot_color.context != context
            || (self.slot_color.edit.is_none()
                && color.is_some_and(|color| color != self.slot_color.raw))
        {
            self.slot_color.context = context;
            self.slot_color.epoch = self.slot_color.epoch.wrapping_add(1);
            self.slot_color.last_focus = None;
            self.slot_color.error = None;
            if let Some(color) = color {
                self.slot_color.raw = color;
                if let Ok(bytes) = peachy_engine::slot_color_bytes(color) {
                    self.slot_color.color = bytes;
                    self.slot_color.hsv = slot_hsv(color);
                    self.slot_color.hsv.a = color[3].clamp(0., 1.);
                    self.slot_color.hex = hex(bytes);
                }
            }
        }
    }

    fn begin_slot_color_edit(
        &mut self,
        ctx: &egui::Context,
        control: egui::Id,
        input: Input,
    ) -> bool {
        let Some(context) = self.slot_color_context() else {
            return false;
        };
        if self.slot_color.blocked
            || !self.slot_color_input_visible()
            || !self.slot_color_input_available(ctx)
        {
            return false;
        }
        if self.slot_color.dialog.is_none() {
            self.cancel_gesture();
            self.session.commit_transaction();
            self.session.begin_transaction();
        }
        self.slot_color.edit = Some(Edit {
            baseline: self.slot_color.raw,
            pose: self.session.pose_edits().clone(),
            context,
            control,
            input,
            commit_requested: false,
            finish_pending: false,
        });
        true
    }

    fn preview_slot_color(&mut self, color: [u8; 4]) {
        self.preview_slot_color_raw(color.map(|channel| f32::from(channel) / 255.));
    }

    fn preview_slot_color_raw(&mut self, color: [f32; 4]) {
        let Some(context) = self
            .slot_color
            .edit
            .as_ref()
            .map(|edit| &edit.context)
            .or_else(|| {
                self.slot_color
                    .dialog
                    .as_ref()
                    .map(|dialog| &dialog.context)
            })
            .cloned()
        else {
            return;
        };
        let target = if context.setup {
            EditTarget::Setup
        } else {
            EditTarget::Animate {
                animation: context.animation,
                time: context.time,
            }
        };
        let slot = context.slot;
        match self.session.preview_edit_slot_pose(
            slot,
            target,
            &[peachy_engine::SlotPoseEdit::Color { color }],
            context.auto_key,
        ) {
            Ok(()) => {
                if let Ok(pose) = self.current_slot_pose(slot) {
                    self.slot_color.raw = pose.color;
                    if let Ok(bytes) = peachy_engine::slot_color_bytes(pose.color) {
                        self.slot_color.color = bytes;
                    }
                }
                if let Some(edit) = self.slot_color.edit.as_mut() {
                    edit.pose = self.session.pose_edits().clone();
                }
                self.sync_slot_color_dialog();
                self.slot_color.error = None;
            }
            Err(error) => {
                self.cancel_slot_color_edit();
                self.slot_color.error = Some(error);
            }
        }
    }

    pub(super) fn slot_color_key_ready(&mut self) -> Result<(), String> {
        let Some(edit) = self.slot_color.edit.as_mut() else {
            return Ok(());
        };
        edit.finish_pending = false;
        if edit.input == Input::Hex {
            let color = parse_hex(&self.slot_color.hex)
                .ok_or_else(|| "Enter a valid #RRGGBBAA color before keying".to_owned())?;
            if color != self.slot_color.color {
                self.preview_slot_color(color);
            }
        }
        self.slot_color.error.clone().map_or(Ok(()), Err)
    }

    pub(super) fn seal_slot_color_for_key(&mut self) {
        if let Some(edit) = self.slot_color.edit.take() {
            self.slot_color.aborted = Some(edit.control);
            self.slot_color.blocked = true;
            self.slot_color.context = None;
        }
    }

    pub(super) fn hold_slot_color_key_error(&mut self, error: &str) {
        if let Some(edit) = self.slot_color.edit.as_mut() {
            edit.finish_pending = false;
            self.slot_color.error = Some(error.into());
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
            if self.slot_color.dialog.is_none() {
                self.session.commit_transaction();
            }
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
            .filter(|id| self.slot_color_input_visible() && self.slot_color.controls.contains(id));
    }

    pub(super) fn slot_color_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        if !self.slot_color_input_visible() || !ctx.input(|input| input.focused) {
            return false;
        }
        let escape = ctx.input(|input| input.key_pressed(egui::Key::Escape));
        let focus = ctx
            .memory(|memory| memory.focused())
            .or_else(|| escape.then_some(self.slot_color.last_focus).flatten());
        let owns_focus = focus.is_some_and(|id| self.slot_color.controls.contains(&id));
        if escape && (self.slot_color_edit_active() || owns_focus) {
            ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
            self.cancel_slot_color_edit();
            if let Some(id) = focus {
                ctx.memory_mut(|memory| memory.surrender_focus(id));
            }
            return true;
        }
        if (self.slot_color_edit_active() || owns_focus)
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
        if self.slot_color.dialog.is_some() && !ctx.text_edit_focused() {
            for (key, action) in [
                (egui::Key::S, 0),
                (egui::Key::O, 1),
                (egui::Key::E, 2),
                (egui::Key::Z, 3),
            ] {
                if ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, key)) {
                    self.cancel_slot_color_edit();
                    match action {
                        0 => self.save_project(),
                        1 => self.open_project(ctx),
                        2 => self.export(),
                        _ => {}
                    }
                    return true;
                }
            }
        }
        self.slot_color.dialog.is_some()
    }

    fn slot_color_control(&mut self, name: &str, response: &egui::Response) {
        self.record_target(name, response.rect);
        self.slot_color.controls.push(response.id);
    }

    pub(super) fn slot_color_panel(&mut self, ui: &mut egui::Ui) {
        if self.slot_color.dialog.is_some() {
            ui.label("Set slot color dialog is open.");
            return;
        }
        self.reconcile_slot_color_input(ui.ctx());
        self.slot_color.controls.clear();
        self.record_target("slotColorPanelClip", ui.clip_rect());
        // Widget rectangles round to pixels; keep the first rounded row inside
        // a sidebar clip whose resizable boundary may fall between pixels.
        ui.add_space((ui.clip_rect().top().ceil() - ui.cursor().top()).max(0.));
        let Some(context) = self.slot_color.context.clone() else {
            ui.label("Select a slot or attachment to inspect its slot color.");
            return;
        };
        let title = ui.add(egui::Label::new(format!("Slot: {}", context.name)).truncate());
        self.record_target("slotColorTarget", title.rect);
        let enabled = !self.slot_color.blocked
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
            if self.slot_color.dialog.is_none() { ui.label("Hex"); }
            let id = ui.make_persistent_id("hex");
            let response = ui.add_enabled(enabled,egui::TextEdit::singleline(&mut self.slot_color.hex)
                .id(id).desired_width(100.).char_limit(9).font(egui::TextStyle::Monospace))
                .on_hover_text("#RRGGBBAA, unmultiplied bytes. Enter/Tab applies; Escape or clicking away cancels.");
            response.widget_info(||egui::WidgetInfo::labeled(egui::WidgetType::TextEdit,enabled,"Slot Color RGBA hex"));
            self.slot_color_control("slotColorHex",&response);
            if enabled && (response.gained_focus() || response.has_focus()) && self.slot_color.edit.is_none() {
                self.begin_slot_color_edit(ui.ctx(),id,Input::Hex);
            }
            if self.slot_color.edit.as_ref().is_some_and(|edit|edit.input == Input::Hex && edit.control == id) {
                let commit = self.slot_color.edit.as_ref().unwrap().commit_requested;
                if commit {
                    self.slot_color.edit.as_mut().unwrap().commit_requested = false;
                    match parse_hex(&self.slot_color.hex) {
                        Some(color) => {
                            let changed = color != self.slot_color.color;
                            if changed {self.preview_slot_color(color);}
                            if changed {self.slot_color.hsv = slot_hsv(self.slot_color.raw);}
                            if let Some(edit) = self.slot_color.edit.as_mut() {edit.finish_pending = true;}
                        }
                        None => {
                            self.slot_color.error = Some("Use #RRGGBBAA: eight hex digits.".into());
                            response.request_focus();
                        }
                    }
                } else if response.lost_focus() && self.pose_key_request.is_none() {
                    if self.slot_color.dialog.is_some() {
                        if let Some(color) = parse_hex(&self.slot_color.hex) {
                            if color != self.slot_color.color { self.preview_slot_color(color); }
                            self.slot_color.edit = None;
                        } else { self.slot_color.error = Some("Use #RRGGBBAA: eight hex digits.".into()); }
                    } else { self.cancel_slot_color_edit(); }
                }
                if response.changed() && !commit {self.slot_color.error = None;}
            }
            if self.slot_color.dialog.is_none() {
                let (_,rect) = ui.allocate_space(vec2(44.,20.));
                self.record_target("slotColorSwatch",rect);
                paint_swatch(ui.painter(),rect,self.slot_color.color);
            }
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
        if !active && !self.begin_slot_color_edit(&response.ctx, response.id, Input::Picker) {
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
        let vertical = axis != 0 && response.rect.height() > response.rect.width();
        match axis {
            0 => {
                self.slot_color.hsv.s = x;
                self.slot_color.hsv.v = y;
            }
            1 => self.slot_color.hsv.h = if vertical { 1. - y } else { x },
            _ => self.slot_color.hsv.a = if vertical { y } else { x },
        }
        if old != self.slot_color.hsv {
            let color = slot_hsv_bytes(self.slot_color.hsv);
            let mut raw = self.slot_color.raw;
            if axis == 2 {
                raw[3] = self.slot_color.hsv.a;
            } else if color[..3] != self.slot_color.color[..3] {
                for index in 0..3 {
                    raw[index] = f32::from(color[index]) / 255.;
                }
            }
            if raw != self.slot_color.raw {
                self.preview_slot_color_raw(raw);
            }
            self.slot_color.hex = hex(color);
        }
    }

    pub(super) fn slot_color_diagnostic(&self) -> serde_json::Value {
        let context = self.slot_color_context();
        let raw = context.as_ref().and_then(|context| {
            self.current_slot_pose(context.slot)
                .ok()
                .map(|pose| pose.color)
        });
        let color = raw.and_then(|color| peachy_engine::slot_color_bytes(color).ok());
        serde_json::json!({"slot":context.as_ref().map(|context|context.slot),"name":context.as_ref().map(|context|&context.name),
        "color":color,"rawColor":raw,"hex":color.map(hex),"setup":self.setup_mode,"visible":self.slot_color_visible(),"dialog":self.slot_color.dialog.is_some(),
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

// Slot RGB is authored in straight color-channel space. HSV conversion must
// neither premultiply hidden RGB nor apply an extra gamma transform.
fn slot_hsv(color: [f32; 4]) -> Hsva {
    let [r, g, b, a] = color.map(|value| value.clamp(0., 1.));
    Hsva::from_rgba_unmultiplied(r, g, b, a)
}
fn slot_hsv_bytes(hsv: Hsva) -> [u8; 4] {
    hsv.to_rgba_unmultiplied()
        .map(|value| (value.clamp(0., 1.) * 255.).round() as u8)
}
fn slot_hsv_color(hsv: Hsva) -> Color32 {
    let [r, g, b, a] = slot_hsv_bytes(hsv);
    Color32::from_rgba_unmultiplied(r, g, b, a)
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
                slot_hsv_color(Hsva { h, s, v, a: 1. }),
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
        let color = slot_hsv_color(if alpha {
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
