//! Shared native and headless editor context configuration.
use super::{ACCENT, PANEL};
use eframe::egui::{self, Color32, FontId, Stroke, vec2};

pub(super) fn configure_editor_context(ctx: &egui::Context) {
    ctx.options_mut(|o| o.zoom_with_keyboard = false);
    ctx.set_theme(egui::ThemePreference::Dark);
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = PANEL;
    style.visuals.window_fill = PANEL;
    style.visuals.selection.bg_fill = Color32::from_rgb(91, 68, 45);
    style.visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(106, 77, 47);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(77, 77, 77);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(66, 66, 66);
    style.spacing.item_spacing = vec2(6.0, 5.0);
    style.spacing.button_padding = vec2(7.0, 3.0);
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(13.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(13.0));
    ctx.set_style_of(egui::Theme::Dark, style);
}
