//! Shared content metrics keep transform rows and the Dopesheet inside their panels.
use super::*;

pub(super) const TRANSFORM_ROWS: [(Tool, &str, &[Property]); 4] = [
    (Tool::Rotate, "Rotate  [C]", &[Property::Rotation]),
    (Tool::Move, "Translate  [V]", &[Property::X, Property::Y]),
    (
        Tool::Scale,
        "Scale  [X]",
        &[Property::ScaleX, Property::ScaleY],
    ),
    (
        Tool::Shear,
        "Shear  [Z]",
        &[Property::ShearX, Property::ShearY],
    ),
];

pub(super) fn main_tools_height(ui: &egui::Ui) -> f32 {
    let spacing = ui.spacing().item_spacing.y;
    let button = ui
        .spacing()
        .interact_size
        .y
        .max(ui.text_style_height(&egui::TextStyle::Button) + 2. * ui.spacing().button_padding.y)
        .max(22.);
    let transforms = TRANSFORM_ROWS.len() as f32 * button
        + TRANSFORM_ROWS.len().saturating_sub(1) as f32 * spacing;
    // Reserve the Animate ancillary column even in Setup to avoid moving the canvas.
    let ancillary = ui.text_style_height(&egui::TextStyle::Body) + 3. * button + 3. * spacing;
    transforms.max(ancillary) + 14.
}

pub(super) fn timeline_row_pitch(ui: &egui::Ui) -> f32 {
    (ui.text_style_height(&egui::TextStyle::Body) + 11.).max(24.)
}
pub(super) fn timeline_chart_height(ui: &egui::Ui) -> f32 {
    24. + Property::ALL.len() as f32 * timeline_row_pitch(ui) + 13.
}
pub(super) fn timeline_panel_height(ui: &egui::Ui) -> f32 {
    let header = ui
        .spacing()
        .interact_size
        .y
        .max(ui.text_style_height(&egui::TextStyle::Button) + 2. * ui.spacing().button_padding.y)
        .max(22.);
    header
        + 5.
        + timeline_chart_height(ui)
        + ui.text_style_height(&egui::TextStyle::Small)
        + 2. * ui.spacing().item_spacing.y
        + 12.
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{editor_context, editor_fixture};
    use super::*;

    fn target(editor: &Editor, name: &str) -> egui::Rect {
        let [left, top, right, bottom] = editor.capture_targets[name];
        egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom))
    }
    #[test]
    fn seven_dopesheet_rows_and_four_transform_rows_fit_minimum_panels_with_real_editor_style() {
        let mut editor = editor_fixture();
        let ctx = editor_context();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1120., 740.))),
                focused: true,
                ..Default::default()
            },
            |ui| {
                egui::Panel::right("test-tree")
                    .exact_size(310.)
                    .show(ui, |_| {});
                let height = timeline_panel_height(ui);
                egui::Panel::bottom("test-dopesheet")
                    .exact_size(height)
                    .frame(egui::Frame::new().inner_margin(6))
                    .show(ui, |ui| editor.timeline(ui));
                egui::CentralPanel::default().show(ui, |ui| {
                    let height = main_tools_height(ui);
                    egui::Panel::bottom("test-tools")
                        .exact_size(height)
                        .frame(egui::Frame::new().inner_margin(7))
                        .show(ui, |ui| editor.main_tools(ui));
                });
            },
        );
        output.textures_delta.clear();
        let clip = target(&editor, "dopesheetClip");
        for property in Property::ALL {
            let row = target(&editor, &format!("timelineRow:{property:?}"));
            assert!(row.width() > 0. && row.height() > 0., "{property:?}");
            assert!(
                clip.contains_rect(row),
                "{property:?}: {row:?} outside {clip:?}"
            );
        }
        assert!(clip.contains_rect(target(&editor, "timelineFooter")));
        let clip = target(&editor, "mainToolsClip");
        for property in Property::ALL {
            let value = target(&editor, &format!("value:{property:?}"));
            assert!(
                clip.contains_rect(value),
                "{property:?}: {value:?} outside {clip:?}"
            );
        }
    }
}
