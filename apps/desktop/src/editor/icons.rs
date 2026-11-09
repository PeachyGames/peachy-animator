//! Independently drawn interface cues. No font glyphs, external assets or domain effects.
use eframe::egui::{
    self, Atom, Button, Color32, Rect, Response, Shape, Stroke, Ui, Vec2, pos2, vec2,
};

const SIZE: f32 = 16.0;

#[derive(Clone, Copy)]
pub(super) enum Icon {
    Open,
    Save,
    Undo,
    Redo,
    Export,
    Rename,
    Duplicate,
    Copy,
    Cut,
    Paste,
    Delete,
    Views,
    Tree,
    Dopesheet,
    Timeline,
    FirstFrame,
    LastFrame,
    PreviousFrame,
    NextFrame,
    PreviousKey,
    NextKey,
    Animations,
    Audio,
    Ghosting,
    Graph,
    MeshTools,
    Skins,
    SlotColor,
    Metrics,
    Outline,
    Playback,
    Preview,
    Weights,
    IkConstraint,
    PhysicsConstraint,
    ChooseBone,
    Bone,
    TargetBone,
    RestoreWorkspace,
    MinimizeViews,
    RestoreViews,
}

/// Atom rectangles keep the icon aligned by egui's own button layout, at any UI scale.
fn show(
    ui: &mut Ui,
    icon: Icon,
    label: Option<&str>,
    shortcut: Option<&str>,
    selected: bool,
) -> Response {
    let icon_id = ui.next_auto_id().with("icon");
    let mut atoms = egui::Atoms::new(Atom::custom(icon_id, Vec2::splat(SIZE)));
    if let Some(label) = label {
        atoms.push_right(label);
    }
    let mut button = Button::new(atoms)
        .selected(selected)
        .small()
        .min_size(vec2(24., 22.));
    if let Some(shortcut) = shortcut {
        button = button.shortcut_text(shortcut);
    }
    let output = button.atom_ui(ui);
    if let Some(rect) = output.rect(icon_id) {
        paint(ui, rect, icon, &output.response);
    }
    output.response
}

pub(super) fn toolbar_button(
    ui: &mut Ui,
    icon: Icon,
    name: &str,
    shortcut: &str,
    enabled: bool,
) -> Response {
    ui.add_enabled_ui(enabled, |ui| {
        let response = show(ui, icon, None, None, false);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), name)
        });
        let hint = if shortcut.is_empty() {
            name.to_owned()
        } else {
            format!("{name} · {shortcut}")
        };
        response
            .on_hover_text(hint.clone())
            .on_disabled_hover_text(hint)
    })
    .inner
}

pub(super) fn button(ui: &mut Ui, icon: Icon, label: &str, shortcut: Option<&str>) -> Response {
    show(ui, icon, Some(label), shortcut, false)
}

pub(super) fn tool_button(ui: &mut Ui, icon: Icon, label: &str, selected: bool) -> Response {
    show(ui, icon, Some(label), None, selected)
}

pub(super) fn menu_button<R>(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    content: impl FnOnce(&mut Ui) -> R,
) -> Response {
    let response = button(ui, icon, label, None);
    let r = response.rect;
    ui.painter().add(Shape::convex_polygon(
        vec![
            r.right_bottom() - vec2(5., 1.5),
            r.right_bottom() - vec2(1.5, 5.),
            r.right_bottom() - vec2(1.5, 1.5),
        ],
        ui.visuals().text_color(),
        Stroke::NONE,
    ));
    egui::Popup::menu(&response)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(content);
    response
}

pub(super) fn heading(ui: &mut Ui, icon: Icon, label: &str) {
    ui.horizontal(|ui| {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(SIZE), egui::Sense::hover());
        paint(ui, rect, icon, &response);
        ui.strong(label);
    });
}

pub(super) fn cue(ui: &mut Ui, icon: Icon) {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(SIZE), egui::Sense::hover());
    paint(ui, rect, icon, &response);
}

fn paint(ui: &Ui, rect: Rect, icon: Icon, response: &Response) {
    let painter = ui.painter_at(rect.expand(0.5));
    let hot = response.hovered() || response.has_focus() || response.is_pointer_button_down_on();
    let ink = Color32::from_gray(if !response.enabled() {
        115
    } else if hot {
        235
    } else {
        185
    });
    let warm = if response.enabled() {
        Color32::from_rgb(225, 187, 93)
    } else {
        ink
    };
    let green = if response.enabled() {
        Color32::from_rgb(123, 182, 151)
    } else {
        ink
    };
    let red = if response.enabled() {
        Color32::from_rgb(193, 113, 116)
    } else {
        ink
    };
    let blue = if response.enabled() {
        Color32::from_rgb(115, 169, 200)
    } else {
        ink
    };
    let p = |x: f32, y: f32| {
        pos2(
            rect.left() + x * rect.width() / SIZE,
            rect.top() + y * rect.height() / SIZE,
        )
    };
    let stroke = |color| Stroke::new(1.2 * rect.width() / SIZE, color);
    let line = |points: &[[f32; 2]], color| {
        painter.add(Shape::line(
            points.iter().map(|q| p(q[0], q[1])).collect(),
            stroke(color),
        ));
    };
    let box_at = |min: [f32; 2], max: [f32; 2], color| {
        painter.rect_stroke(
            Rect::from_min_max(p(min[0], min[1]), p(max[0], max[1])),
            0.5,
            stroke(color),
            egui::StrokeKind::Inside,
        );
    };
    match icon {
        Icon::Open => {
            line(
                &[
                    [1.5, 12.5],
                    [1.5, 3.5],
                    [6., 3.5],
                    [7.5, 5.5],
                    [14., 5.5],
                    [14., 7.],
                ],
                ink,
            );
            painter.add(Shape::convex_polygon(
                vec![p(1.5, 13.), p(4., 7.), p(15., 7.), p(12.5, 13.)],
                ink.gamma_multiply(0.3),
                stroke(ink),
            ));
        }
        Icon::Save => {
            box_at([2., 1.5], [14., 14.5], ink);
            box_at([5., 1.5], [11., 6.5], ink);
            line(&[[9.5, 2.5], [9.5, 5.5]], ink);
            box_at([4.5, 9.], [11.5, 14.5], ink);
        }
        Icon::Undo | Icon::Redo => {
            let mirror = |x: f32| {
                if matches!(icon, Icon::Redo) {
                    SIZE - x
                } else {
                    x
                }
            };
            // A cubic bend and broad arrowhead express history direction without text.
            let control = [[13., 13.], [15., 5.], [9., 3.], [4., 6.]];
            let curve: Vec<_> = (0..=16)
                .map(|step| {
                    let t = step as f32 / 16.;
                    let u = 1. - t;
                    let q = [0, 1].map(|axis| {
                        u * u * u * control[0][axis]
                            + 3. * u * u * t * control[1][axis]
                            + 3. * u * t * t * control[2][axis]
                            + t * t * t * control[3][axis]
                    });
                    p(mirror(q[0]), q[1])
                })
                .collect();
            painter.add(Shape::line(curve, stroke(ink)));
            line(
                &[[mirror(6.), 2.5], [mirror(2.), 6.], [mirror(6.), 9.5]],
                ink,
            );
        }
        Icon::Export => {
            line(&[[2., 8.], [2., 14.], [14., 14.], [14., 8.]], ink);
            line(&[[8., 11.], [8., 2.], [4.5, 5.5]], ink);
            line(&[[8., 2.], [11.5, 5.5]], ink);
        }
        Icon::Rename => {
            line(&[[8., 2.], [13., 2.]], ink);
            line(&[[10.5, 2.], [10.5, 14.]], ink);
            line(&[[8., 14.], [13., 14.]], ink);
            line(&[[2., 5.], [2., 11.], [6., 11.], [6., 5.], [2., 5.]], ink);
        }
        Icon::Duplicate | Icon::Copy => {
            box_at([1.5, 1.5], [10.5, 11.5], ink);
            painter.rect_filled(
                Rect::from_min_max(p(5.5, 5.), p(14.5, 15.)),
                0.5,
                ui.visuals().panel_fill,
            );
            box_at([5.5, 5.], [14.5, 15.], ink);
            line(&[[8., 8.], [12., 8.]], ink);
            line(&[[8., 11.], [12., 11.]], ink);
        }
        Icon::Cut => {
            for y in [4.5, 11.5] {
                painter.circle_stroke(p(4., y), 2.5 * rect.width() / SIZE, stroke(ink));
            }
            line(&[[6., 6.], [14., 13.5]], ink);
            line(&[[6., 10.], [14., 2.5]], ink);
            painter.circle_filled(p(8., 8.), rect.width() / SIZE, ink);
        }
        Icon::Paste => {
            box_at([2.5, 3.], [13.5, 15.], ink);
            painter.rect_filled(
                Rect::from_min_max(p(5., 1.), p(11., 5.)),
                0.5,
                ui.visuals().panel_fill,
            );
            box_at([5., 1.], [11., 5.], ink);
            line(&[[5., 8.], [11., 8.]], ink);
            line(&[[5., 11.], [11., 11.]], ink);
        }
        Icon::Delete => {
            painter.circle_stroke(p(8., 8.), 6. * rect.width() / SIZE, stroke(ink));
            line(&[[5.5, 5.5], [10.5, 10.5]], ink);
            line(&[[10.5, 5.5], [5.5, 10.5]], ink);
        }
        Icon::MeshTools => {
            painter.circle_stroke(p(8., 8.), 6.5 * rect.width() / SIZE, stroke(ink));
            painter.circle_stroke(p(8., 8.), 3.5 * rect.width() / SIZE, stroke(warm));
            line(&[[2., 11.], [8., 4.], [14., 11.], [2., 11.]], ink);
            painter.circle_filled(p(8., 4.), 1.5 * rect.width() / SIZE, warm);
        }
        Icon::Weights => {
            let points = [p(3., 12.), p(8., 3.), p(13., 12.)];
            for (a, b) in [(0, 1), (1, 2), (2, 0)] {
                painter.line_segment([points[a], points[b]], stroke(ink));
            }
            for point in points {
                painter.circle_filled(point, 2. * rect.width() / SIZE, ink);
            }
        }
        Icon::IkConstraint => draw_ik(&painter, rect, ink),
        Icon::PhysicsConstraint => draw_physics(&painter, rect, ink),
        Icon::Bone => draw_bone(&painter, rect, ink),
        Icon::TargetBone => {
            painter.circle_stroke(p(8., 8.), 4. * rect.width() / SIZE, stroke(warm));
            line(&[[8., 1.], [8., 5.]], warm);
            line(&[[8., 11.], [8., 15.]], warm);
            line(&[[1., 8.], [5., 8.]], warm);
            line(&[[11., 8.], [15., 8.]], warm);
        }
        Icon::ChooseBone => {
            line(
                &[
                    [3., 12.],
                    [10.5, 2.5],
                    [14., 6.],
                    [6.5, 15.],
                    [3., 15.],
                    [3., 12.],
                ],
                ink,
            );
            line(&[[9., 4.5], [12.5, 8.]], ink);
        }
        Icon::Views | Icon::RestoreWorkspace => {
            box_at([1.5, 2.], [14.5, 14.], ink);
            line(&[[2., 5.], [14., 5.]], blue);
            line(&[[10., 5.], [10., 13.5]], ink);
            line(&[[2., 10.], [10., 10.]], warm);
            if matches!(icon, Icon::RestoreWorkspace) {
                painter.circle_filled(
                    p(12., 11.),
                    4. * rect.width() / SIZE,
                    ui.visuals().panel_fill,
                );
                line(&[[14.5, 12.5], [14.5, 9.], [10., 9.], [11.5, 7.5]], warm);
                line(&[[10., 9.], [11.5, 10.5]], warm);
            }
        }
        Icon::Tree => {
            line(&[[3.5, 4.], [3.5, 12.], [8.5, 12.]], warm);
            line(&[[3.5, 7.5], [8.5, 7.5]], warm);
            box_at([1.5, 1.5], [5.5, 5.5], warm);
            box_at([8., 5.5], [12., 9.5], warm);
            box_at([8., 10.5], [12., 14.5], warm);
        }
        Icon::Audio => {
            line(
                &[
                    [1., 8.],
                    [3., 8.],
                    [5., 3.],
                    [7., 13.],
                    [9., 5.],
                    [11., 11.],
                    [13., 8.],
                    [15., 8.],
                ],
                ink,
            );
            painter.circle_filled(p(5., 3.), 1.2 * rect.width() / SIZE, warm);
        }
        Icon::Animations => {
            box_at([2., 2.], [14., 14.], ink);
            for x in [4., 8., 12.] {
                line(&[[x, 2.], [x, 4.]], blue);
                line(&[[x, 12.], [x, 14.]], blue);
            }
            painter.add(Shape::convex_polygon(
                vec![p(6., 5.), p(11., 8.), p(6., 11.)],
                green,
                Stroke::NONE,
            ));
        }
        Icon::Graph => {
            line(&[[2., 2.], [2., 14.], [15., 14.]], ink);
            line(&[[3., 11.], [6., 5.], [10., 9.], [14., 3.]], warm);
            for (x, y) in [(3., 11.), (6., 5.), (10., 9.), (14., 3.)] {
                painter.circle_filled(p(x, y), rect.width() / SIZE, warm);
            }
        }
        Icon::Ghosting => {
            for (x, color) in [
                (1., blue.gamma_multiply(0.5)),
                (5., ink),
                (9., red.gamma_multiply(0.7)),
            ] {
                painter.circle_stroke(p(x + 2., 4.), 2. * rect.width() / SIZE, stroke(color));
                line(&[[x + 2., 6.], [x + 2., 11.], [x, 14.]], color);
                line(&[[x + 2., 11.], [x + 4., 14.]], color);
            }
        }
        Icon::SlotColor => {
            box_at([2., 2.], [14., 14.], ink);
            for (x, y, color) in [
                (5., 5., Color32::from_rgb(220, 95, 90)),
                (11., 5., Color32::from_rgb(90, 190, 145)),
                (5., 11., Color32::from_rgb(90, 145, 220)),
                (11., 11., warm),
            ] {
                painter.rect_filled(
                    Rect::from_center_size(p(x, y), Vec2::splat(4. * rect.width() / SIZE)),
                    0.,
                    color,
                );
            }
        }
        Icon::Skins => {
            line(
                &[
                    [2., 5.],
                    [5., 2.],
                    [6., 4.],
                    [10., 4.],
                    [11., 2.],
                    [14., 5.],
                    [12., 8.],
                    [11., 7.],
                    [11., 14.],
                    [5., 14.],
                    [5., 7.],
                    [4., 8.],
                    [2., 5.],
                ],
                warm,
            );
        }
        Icon::Metrics => {
            line(&[[2., 2.], [2., 14.], [15., 14.]], ink);
            for (x, y, color) in [(4., 8., green), (8., 4., warm), (12., 6., blue)] {
                box_at([x, y], [x + 2., 13.], color);
            }
        }
        Icon::Outline => {
            box_at([1., 2.], [15., 14.], ink);
            line(
                &[
                    [4., 11.],
                    [6., 6.],
                    [9., 4.],
                    [12., 8.],
                    [11., 11.],
                    [4., 11.],
                ],
                green,
            );
        }
        Icon::Preview => {
            box_at([1., 2.], [12., 12.], ink);
            box_at([4., 5.], [15., 15.], ink);
            painter.add(Shape::convex_polygon(
                vec![p(8., 7.), p(12., 10.), p(8., 13.)],
                warm,
                Stroke::NONE,
            ));
        }
        Icon::Playback => {
            painter.add(Shape::convex_polygon(
                vec![p(2., 2.), p(10., 6.), p(2., 10.)],
                green,
                Stroke::NONE,
            ));
            line(&[[2., 13.], [14., 13.]], ink);
            box_at([9., 11.], [12., 15.], warm);
        }
        Icon::Dopesheet => {
            line(&[[2., 3.], [14., 3.], [14., 14.], [2., 14.], [2., 3.]], ink);
            line(&[[2., 7.], [14., 7.]], ink.gamma_multiply(0.6));
            line(&[[2., 10.5], [14., 10.5]], ink.gamma_multiply(0.6));
            for (x, y, color) in [(5., 5., green), (10., 8.5, red), (7., 12., green)] {
                painter.add(Shape::convex_polygon(
                    vec![p(x, y - 1.8), p(x + 1.8, y), p(x, y + 1.8), p(x - 1.8, y)],
                    color,
                    Stroke::NONE,
                ));
            }
        }
        Icon::Timeline => {
            line(&[[1., 4.], [15., 4.]], ink);
            for x in [2., 5., 8., 11., 14.] {
                line(&[[x, 4.], [x, 7.]], ink);
            }
            line(&[[9., 2.], [9., 14.]], blue);
            painter.add(Shape::convex_polygon(
                vec![p(3., 10.), p(5., 12.), p(3., 14.), p(1., 12.)],
                warm,
                Stroke::NONE,
            ));
        }
        Icon::FirstFrame
        | Icon::LastFrame
        | Icon::PreviousFrame
        | Icon::NextFrame
        | Icon::PreviousKey
        | Icon::NextKey => {
            let forward = matches!(icon, Icon::LastFrame | Icon::NextFrame | Icon::NextKey);
            let q = |x: f32, y: f32| p(if forward { 16. - x } else { x }, y);
            painter.add(Shape::convex_polygon(
                vec![q(4., 8.), q(10., 3.), q(10., 13.)],
                ink,
                Stroke::NONE,
            ));
            if matches!(icon, Icon::FirstFrame | Icon::LastFrame) {
                painter.line_segment([q(2., 2.), q(2., 14.)], stroke(ink));
            } else if matches!(icon, Icon::PreviousFrame | Icon::NextFrame) {
                painter.line_segment([q(12., 3.), q(12., 13.)], stroke(ink));
            } else {
                painter.add(Shape::convex_polygon(
                    vec![q(12.5, 5.), q(15., 8.), q(12.5, 11.), q(10., 8.)],
                    warm,
                    Stroke::NONE,
                ));
            }
        }
        Icon::MinimizeViews => {
            box_at([1.5, 2.], [14.5, 14.], ink);
            line(&[[4., 10.5], [12., 10.5]], ink);
        }
        Icon::RestoreViews => {
            box_at([2., 5.], [11., 14.], ink);
            line(
                &[[5., 5.], [5., 2.], [14., 2.], [14., 11.], [11., 11.]],
                ink,
            );
        }
    }
}

pub(super) fn draw_ik(painter: &egui::Painter, rect: Rect, color: Color32) {
    let p = |x: f32, y: f32| {
        pos2(
            rect.left() + x * rect.width() / 16.,
            rect.top() + y * rect.height() / 16.,
        )
    };
    let stroke = Stroke::new(rect.width() / 12., color);
    painter.line_segment([p(2., 13.), p(6., 5.)], stroke);
    painter.line_segment([p(6., 5.), p(12., 9.)], stroke);
    painter.circle_filled(p(2., 13.), rect.width() / 10., color);
    painter.circle_filled(p(6., 5.), rect.width() / 10., color);
    painter.circle_stroke(p(12., 9.), rect.width() / 5., stroke);
}

/// Original pendulum cue: a pivot, a swinging arm and motion arcs.
pub(super) fn draw_physics(painter: &egui::Painter, rect: Rect, color: Color32) {
    let p = |x: f32, y: f32| {
        pos2(
            rect.left() + x * rect.width() / 16.,
            rect.top() + y * rect.height() / 16.,
        )
    };
    let stroke = Stroke::new(rect.width() / 12., color);
    painter.circle_filled(p(8., 2.5), rect.width() / 10., color);
    painter.line_segment([p(8., 2.5), p(11., 11.)], stroke);
    painter.circle_filled(p(11., 11.), rect.width() / 6., color);
    for [a, b, c] in [
        [[3., 9.], [3.6, 11.5], [5., 13.5]],
        [[1., 8.], [1.8, 11.5], [3.4, 14.5]],
    ] {
        painter.line_segment([p(a[0], a[1]), p(b[0], b[1])], stroke);
        painter.line_segment([p(b[0], b[1]), p(c[0], c[1])], stroke);
    }
}

pub(super) fn draw_bone(painter: &egui::Painter, rect: Rect, color: Color32) {
    let a = rect.left_bottom() + vec2(rect.width() * 0.2, -rect.height() * 0.2);
    let b = rect.right_top() + vec2(-rect.width() * 0.2, rect.height() * 0.2);
    painter.line_segment([a, b], Stroke::new(rect.width() * 0.18, color));
    for point in [a, b] {
        painter.circle_filled(point, rect.width() * 0.16, color);
    }
}
