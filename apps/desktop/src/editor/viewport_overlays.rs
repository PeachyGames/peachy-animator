//! Read-only guides for non-image attachments. Vertex editing stays in engine
//! commands; clicking a guide only updates the shared typed Tree selection.
use super::*;
use peachy_engine::{AttachmentOverlay, OverlayKind, path_cubics};

impl Editor {
    pub(super) fn paint_attachment_overlays(
        &mut self,
        painter: &egui::Painter,
        overlays: &[AttachmentOverlay],
        origin: Pos2,
        zoom: f32,
        pointer: Option<Pos2>,
    ) -> Option<TreeItem> {
        let mut hit = None;
        for overlay in overlays {
            let selected = self.selected_attachment() == Some(&overlay.attachment);
            let color = overlay
                .color
                .map(scene_view::attachment_tint)
                .unwrap_or_else(|| match overlay.kind {
                    OverlayKind::Clipping => Color32::from_rgb(58, 58, 206),
                    OverlayKind::Path { .. } => Color32::from_rgb(255, 127, 0),
                });
            let stroke = Stroke::new(if selected { 2. } else { 1. }, color);
            let points: Vec<_> = overlay
                .vertices
                .iter()
                .map(|&p| scene_view::screen_point(origin, zoom, p))
                .collect();
            let mut lines = vec![];
            match overlay.kind {
                OverlayKind::Clipping => {
                    painter.add(egui::Shape::closed_line(points.clone(), stroke));
                    lines.extend(points.windows(2).map(|p| [p[0], p[1]]));
                    if let (Some(&first), Some(&last)) = (points.first(), points.last()) {
                        lines.push([last, first]);
                    }
                    if selected {
                        for &point in &points {
                            painter.circle_filled(point, 2., Color32::from_rgb(110, 230, 235));
                        }
                    }
                }
                OverlayKind::Path { closed } => {
                    let curves = path_cubics(&overlay.vertices, closed);
                    for curve in curves {
                        let shape = egui::epaint::CubicBezierShape::from_points_stroke(
                            curve.map(|p| scene_view::screen_point(origin, zoom, p)),
                            false,
                            Color32::TRANSPARENT,
                            stroke,
                        );
                        lines.extend(shape.flatten(Some(0.5)).windows(2).map(|p| [p[0], p[1]]));
                        painter.add(shape);
                    }
                    if selected {
                        for knot in points.as_chunks::<3>().0 {
                            painter.line_segment([knot[0], knot[1]], Stroke::new(1., color));
                            painter.line_segment([knot[1], knot[2]], Stroke::new(1., color));
                            let vertex_color = Color32::from_rgb(110, 230, 235);
                            painter.rect_filled(
                                egui::Rect::from_center_size(knot[1], vec2(5., 5.)),
                                0.,
                                vertex_color,
                            );
                            for &handle in &[knot[0], knot[2]] {
                                painter.circle_filled(handle, 2., vertex_color);
                            }
                        }
                        if !closed && points.len() >= 6 {
                            let n = points.len();
                            for (end, handle) in
                                [(points[1], points[2]), (points[n - 2], points[n - 3])]
                            {
                                let direction = (end - handle).normalized();
                                if direction.is_finite() {
                                    painter.extend(egui::Shape::dashed_line(
                                        &[end, end + direction * 60.],
                                        Stroke::new(1., color),
                                        4.,
                                        4.,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            let pickable = self.tree.filters.attachments || self.tree.filters.slots;
            if pickable
                && pointer.is_some_and(|p| {
                    lines
                        .iter()
                        .any(|&[a, b]| distance_to_segment(p, a, b) < 6.)
                })
            {
                hit = Some(if self.tree.filters.attachments {
                    TreeItem::Attachment(overlay.attachment.clone())
                } else {
                    TreeItem::Slot(overlay.slot)
                });
            }
            // One clear edge per overlay lets native acceptance click geometry,
            // independently of its Tree row or hand-picked screen coordinates.
            if self.capture.is_some()
                && let Some([a, b]) = lines.iter().find(|[a, b]| {
                    painter.clip_rect().contains(a.lerp(*b, 0.5)) && a.distance(*b) > 3.
                })
            {
                let point = a.lerp(*b, 0.5);
                self.record_target(
                    &format!(
                        "overlay:{}",
                        TreeItem::Attachment(overlay.attachment.clone()).key()
                    ),
                    egui::Rect::from_center_size(point, vec2(1., 1.)),
                );
            }
        }
        hit
    }
}

fn distance_to_segment(point: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    if ab.length_sq() == 0. {
        return point.distance(a);
    }
    point.distance(a + ab * ((point - a).dot(ab) / ab.length_sq()).clamp(0., 1.))
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
    use super::*;

    fn editor() -> Editor {
        let mut editor = editor_fixture();
        let doc: Document = serde_json::from_str(include_str!(
            "../../../../examples/peach-bot/path-bot.peachy"
        ))
        .unwrap();
        editor.session = Session::new(doc).unwrap();
        editor.selection = None;
        editor.show_bones = false;
        editor.fit = true;
        editor.capture = Some(std::env::temp_dir().join("peachy-overlay-test.png"));
        editor
    }

    #[test]
    fn mouse_picks_evaluated_curve_as_an_attachment_and_cannot_edit_cached_bone() {
        let mut editor = editor();
        let ctx = editor_context();
        let original = editor.session.document().clone();
        for _ in 0..3 {
            workspace_frame(&mut editor, &ctx, vec![]);
        }
        let (target, rect) = editor
            .capture_targets
            .iter()
            .find(|(key, _)| key.starts_with("overlay:"))
            .unwrap();
        let target = target.clone();
        let point = pos2((rect[0] + rect[2]) * 0.5, (rect[1] + rect[3]) * 0.5);
        for pressed in [true, false] {
            workspace_frame(
                &mut editor,
                &ctx,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        assert_eq!(
            target,
            format!("overlay:{}", editor.selection.as_ref().unwrap().key())
        );
        assert!(matches!(editor.selection, Some(TreeItem::Attachment(_))));
        assert!(editor.selected_bone().is_none());
        assert!(!editor.transform_tool_editable(Tool::Move));
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::ArrowRight,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Space,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(editor.selection.is_none());
    }

    #[test]
    fn hidden_slots_drop_guides_and_fit_handles_documents_without_images() {
        let mut editor = editor();
        let mut doc = editor.session.document().clone();
        for slot in &mut doc.slots {
            if !matches!(slot.name.as_str(), "rail" | "leg-rail") {
                slot.attachment = None;
            }
        }
        editor.session = Session::new(doc).unwrap();
        let ctx = editor_context();
        assert!(editor.scene_for_view().unwrap().attachments.is_empty());
        for _ in 0..3 {
            workspace_frame(&mut editor, &ctx, vec![]);
        }
        assert!(!editor.fit && editor.zoom > 0. && editor.evaluation_error.is_none());
        let scene = editor.scene_for_view().unwrap();
        assert_eq!(scene.overlays.len(), 2);
        for overlay in scene.overlays {
            editor.hidden_slots.insert(overlay.slot);
        }
        workspace_frame(&mut editor, &ctx, vec![]);
        assert!(
            !editor
                .capture_targets
                .keys()
                .any(|key| key.starts_with("overlay:"))
        );
        assert!(!editor.session.can_undo());
    }

    #[test]
    fn projection_checks_guides_before_egui_receives_them() {
        let editor = editor();
        let mut scene = editor.scene_for_view().unwrap();
        scene.overlays[0].vertices[0] = [f32::MAX, 0.];
        assert!(
            editor
                .check_view_projection(
                    &scene,
                    egui::Rect::from_min_size(Pos2::ZERO, vec2(1000., 700.)),
                    Pos2::ZERO,
                    6.
                )
                .is_err()
        );
    }
}
