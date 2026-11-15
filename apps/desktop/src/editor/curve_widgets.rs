//! Curve surfaces share numeric focus/escape handling and interpolation glyphs.
use super::*;

pub(super) struct HandleFields {
    pub changed: bool,
    pub apply: bool,
    pub cancel: bool,
    pub rects: Vec<egui::Rect>,
}

pub(super) fn handle_fields(
    ui: &mut egui::Ui,
    id: &str,
    text: &mut [String; 4],
    changed: bool,
    enabled: bool,
    horizontal: bool,
    labels: [&str; 4],
) -> HandleFields {
    let mut output = HandleFields {
        changed,
        apply: false,
        cancel: false,
        rects: Vec::new(),
    };
    egui::Grid::new(("curve-handle-fields", id))
        .num_columns(if horizontal { 8 } else { 2 })
        .show(ui, |ui| {
            for index in 0..4 {
                ui.label(labels[index]);
                let response = ui.add_enabled(
                    enabled,
                    egui::TextEdit::singleline(&mut text[index])
                        .id_salt((id, index))
                        .event_filter(egui::EventFilter {
                            horizontal_arrows: true,
                            vertical_arrows: true,
                            escape: true,
                            ..Default::default()
                        })
                        .desired_width(90.),
                );
                output.rects.push(response.rect);
                output.changed |= response.changed();
                if response.has_focus()
                    && ui.input_mut(|input| {
                        input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                    })
                {
                    output.cancel = true;
                    response.surrender_focus();
                }
                output.apply |= output.changed && response.lost_focus();
                if !horizontal {
                    ui.end_row();
                }
            }
            if horizontal {
                ui.end_row();
            }
        });
    output
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InterpolationGlyph {
    Linear,
    Stepped,
    Bezier,
}

pub(super) fn interpolation_button(
    ui: &mut egui::Ui,
    glyph: InterpolationGlyph,
    selected: bool,
    enabled: bool,
) -> egui::Response {
    let response = ui.add_enabled(
        enabled,
        egui::Button::new("")
            .min_size(vec2(28., 23.))
            .selected(selected),
    );
    let rect = response.rect.shrink2(vec2(5., 5.));
    let ink = if response.enabled() {
        ui.visuals().text_color()
    } else {
        MUTED
    };
    let a = pos2(rect.left(), rect.bottom());
    let b = pos2(rect.right(), rect.top());
    let painter = ui.painter_at(response.rect);
    let points = match glyph {
        InterpolationGlyph::Linear => vec![a, b],
        InterpolationGlyph::Stepped => vec![a, pos2(rect.right(), rect.bottom()), b],
        InterpolationGlyph::Bezier => (0..=16)
            .map(|n| {
                let t = n as f32 / 16.;
                pos2(
                    rect.left() + rect.width() * t,
                    rect.bottom() - rect.height() * t * t * (3. - 2. * t),
                )
            })
            .collect(),
    };
    painter.add(egui::Shape::line(points, Stroke::new(1.3, ink)));
    for p in [a, b] {
        painter.rect_filled(egui::Rect::from_center_size(p, vec2(3., 3.)), 0., ink);
    }
    response.on_hover_text(match glyph {
        InterpolationGlyph::Linear => "Linear",
        InterpolationGlyph::Stepped => "Stepped",
        InterpolationGlyph::Bezier => "Bezier",
    })
}
