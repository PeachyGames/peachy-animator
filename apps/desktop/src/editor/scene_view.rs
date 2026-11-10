//! Checked scene output and view-coordinate guards, without changing authoring state.
use super::*;
use peachy_engine::{Scene, checked_scene_from_authored};

pub(super) fn screen_point(origin: Pos2, zoom: f32, point: [f32; 2]) -> Pos2 {
    let ordinary = origin + vec2(point[0] * zoom, -point[1] * zoom);
    if ordinary.is_finite() {
        return ordinary;
    }
    pos2(
        (f64::from(origin.x) + f64::from(point[0]) * f64::from(zoom)) as f32,
        (f64::from(origin.y) - f64::from(point[1]) * f64::from(zoom)) as f32,
    )
}

pub(super) fn view_world_point(origin: Pos2, zoom: f32, point: Pos2) -> [f32; 2] {
    let ordinary = [(point.x - origin.x) / zoom, -(point.y - origin.y) / zoom];
    if ordinary.into_iter().all(f32::is_finite) {
        return ordinary;
    }
    [
        ((f64::from(point.x) - f64::from(origin.x)) / f64::from(zoom)) as f32,
        (-(f64::from(point.y) - f64::from(origin.y)) / f64::from(zoom)) as f32,
    ]
}

/// Count lines from the viewport span, not by repeatedly adding to a large world offset.
pub(super) fn grid_lines(min: f32, max: f32, origin: f32, step: f32) -> impl Iterator<Item = f32> {
    let valid =
        [min, max, origin, step].into_iter().all(f32::is_finite) && step >= 8. && max >= min;
    let (first, count) = if valid {
        let (min, max, origin, step) = (
            f64::from(min),
            f64::from(max),
            f64::from(origin),
            f64::from(step),
        );
        (
            min - (min - origin).rem_euclid(step),
            (((max - min) / step).ceil() as usize).saturating_add(2),
        )
    } else {
        (0., 0)
    };
    (0..count).filter_map(move |index| {
        let coordinate = (first + index as f64 * f64::from(step)) as f32;
        (coordinate >= min && coordinate < max && coordinate.is_finite()).then_some(coordinate)
    })
}

/// Renderer colors are premultiplied only at the final drawing boundary.
/// Native slot bytes stay straight RGBA, including hidden RGB at alpha zero.
pub(super) fn attachment_tint([r, g, b, a]: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(r, g, b, a)
}

/// Tint for a draw with its slot blend mode. egui composites premultiplied
/// colors, so a premultiplied color with zero alpha adds to what is behind it:
/// additive is exact, screen is drawn additively (slightly brighter), and
/// multiply falls back to normal blending.
pub(super) fn draw_tint(color: [u8; 4], blend: peachy_document::BlendMode) -> Color32 {
    use peachy_document::BlendMode;
    match blend {
        BlendMode::Additive | BlendMode::Screen => {
            let tint = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
            Color32::from_rgba_premultiplied(tint.r(), tint.g(), tint.b(), 0)
        }
        BlendMode::Normal | BlendMode::Multiply => attachment_tint(color),
    }
}

pub(super) fn ghost_tint(ghost: Color32, slot: [u8; 4], solid: bool) -> Color32 {
    let base = ghost.to_srgba_unmultiplied();
    let multiply = |a: u8, b: u8| ((u16::from(a) * u16::from(b) + 127) / 255) as u8;
    let rgb = [0, 1, 2].map(|axis| {
        if solid {
            base[axis]
        } else {
            multiply(base[axis], slot[axis])
        }
    });
    attachment_tint([rgb[0], rgb[1], rgb[2], multiply(base[3], slot[3])])
}

impl Editor {
    pub(super) fn scene_for_view(&self) -> Result<Scene, String> {
        checked_scene_from_authored(
            self.session.document(),
            &self.skin,
            &self.session.authored_pose(self.weights_edit_target())?,
        )
    }
    pub(super) fn check_view_projection(
        &self,
        scene: &Scene,
        rect: egui::Rect,
        origin: Pos2,
        zoom: f32,
    ) -> Result<(), String> {
        if !origin.is_finite() || !zoom.is_finite() || zoom <= 0. {
            return Err(
                "View coordinates are outside the supported range. Use Fit to reset the view."
                    .into(),
            );
        }
        if ![rect.min, rect.max]
            .into_iter()
            .flat_map(|point| view_world_point(origin, zoom, point))
            .all(f32::is_finite)
        {
            return Err(
                "View coordinates are outside the supported range. Use Fit to reset the view."
                    .into(),
            );
        }
        let vertices = scene
            .attachments
            .iter()
            .flat_map(|draw| draw.vertices.iter().copied());
        let bones = self
            .session
            .document()
            .bones
            .iter()
            .zip(&scene.world)
            .enumerate()
            .filter(|(index, _)| self.show_bones && !self.hidden_bones.contains(index))
            .flat_map(|(_, (bone, world))| [world.point(0., 0.), world.point(bone.length, 0.)]);
        if !vertices.chain(bones).all(|point| {
            point.into_iter().all(f32::is_finite) && screen_point(origin, zoom, point).is_finite()
        }) {
            return Err("Pose coordinates cannot be displayed at this view scale. Use Fit or adjust the pose.".into());
        }
        Ok(())
    }
    pub(super) fn show_evaluation_error(&mut self, ui: &egui::Ui, rect: egui::Rect, error: String) {
        self.cancel_mesh_drag();
        self.mesh_handles.clear();
        self.weights_handles.clear();
        self.record_target("evaluationError", rect);
        ui.painter_at(rect).text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("Pose cannot be displayed\n{error}"),
            FontId::proportional(13.),
            ACCENT,
        );
        if self.evaluation_error.as_ref() != Some(&error) {
            ui.ctx().request_repaint();
        }
        self.evaluation_error = Some(error);
        if !ui.input(|input| input.pointer.any_down()) {
            self.finish_viewport_transform();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{editor_context, editor_fixture};
    use super::*;

    #[test]
    fn scene_uses_animated_ik_capsule_and_reuses_the_checked_applied_frame() {
        let editor = super::super::test_support::animated_ik_editor();
        let scene = editor.scene_for_view().unwrap();
        assert_eq!(scene.ik_poses[1].mix, 0.375);
        assert!(scene.ik_poses[1].bend_positive);
        let pose = editor
            .session
            .evaluated_pose(editor.weights_edit_target())
            .unwrap();
        assert_eq!(
            serde_json::to_value(&scene.world).unwrap(),
            serde_json::to_value(pose.world()).unwrap()
        );
        assert_eq!(scene.applied_locals, pose.applied_locals());
        assert_eq!(scene.locals, pose.authored_locals());
        let setup = editor.session.evaluated_pose(EditTarget::Setup).unwrap();
        assert_ne!(scene.world[2].point(1., 0.), setup.world()[2].point(1., 0.));
        assert!(!editor.session.can_undo());
    }

    #[test]
    fn large_finite_grid_offsets_remain_bounded_and_projection_failures_are_explicit() {
        // The old f32 x+=step loop stalled at -1.8889466e22 for this finite view.
        let lines = grid_lines(0., 1500., -3.1e29, 15.5).collect::<Vec<_>>();
        assert!(!lines.is_empty() && lines.len() < 100);
        assert!(lines.windows(2).all(|pair| pair[1] > pair[0]));
        assert!(
            lines
                .iter()
                .all(|&value| value.is_finite() && (0.0..1500.0).contains(&value))
        );
        let editor = editor_fixture();
        let scene = editor.scene_for_view().unwrap();
        assert!(
            editor
                .check_view_projection(
                    &scene,
                    egui::Rect::from_min_size(Pos2::ZERO, vec2(1000., 700.)),
                    pos2(f32::INFINITY, 0.),
                    1.
                )
                .is_err()
        );
        assert!(screen_point(pos2(-2e38, 0.), 2., [2e38, 0.]).is_finite());
    }
    #[test]
    fn invalid_evaluation_skips_geometry_and_reports_it_without_changing_document_or_pending_pose()
    {
        let mut editor = editor_fixture();
        let mut document = editor.session.document().clone();
        let track = document.animations[0]
            .tracks
            .iter_mut()
            .find(|track| track.bone == 8 && track.property == Property::X)
            .unwrap();
        track.keys = vec![
            Keyframe {
                time: 0.,
                value: 2e38,
                curve: Curve::Bezier([0.25, 2., 0.75, 2.]),
            },
            Keyframe {
                time: 1.,
                value: 3e38,
                curve: Curve::Linear,
            },
        ];
        editor.session = Session::new(document).unwrap();
        editor.time = 0.5;
        let mut local = editor.session.local_pose(Some(0), editor.time)[7];
        local.rotation += 17.;
        editor
            .session
            .edit_pose(
                7,
                EditTarget::Animate {
                    animation: 0,
                    time: editor.time,
                },
                local,
                &[Property::Rotation],
                false,
            )
            .unwrap();
        let original = editor.session.document().clone();
        let pending = editor.session.pose_edits().clone();
        let undo = editor.session.can_undo();
        let dirty = editor.session.is_dirty();
        let ctx = editor_context();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1100., 700.))),
                focused: true,
                ..Default::default()
            },
            |ui| {
                editor.canvas(ui);
                editor.finish_idle_transactions(ui.ctx());
            },
        );
        output.textures_delta.clear();
        assert!(
            editor
                .evaluation_error
                .as_ref()
                .unwrap()
                .contains("nonfinite")
        );
        assert!(editor.mesh_handles.is_empty() && editor.weights_handles.is_empty());
        assert!(editor.capture_targets.contains_key("evaluationError"));
        assert!(editor.weights_diagnostic()["evaluationError"].is_string());
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(editor.session.can_undo(), undo);
        assert_eq!(editor.session.is_dirty(), dirty);
        assert_eq!(editor.time, 0.5);
        assert_eq!(editor.selected_bone(), Some(8));
    }
}

#[cfg(test)]
mod slot_tint_tests {
    use super::super::test_support::{editor_context, editor_fixture, mesh_colors};
    use super::*;

    #[test]
    fn viewport_outline_and_preview_draw_the_owning_slot_rgba() {
        for view in [
            None,
            Some(views::DockView::Outline),
            Some(views::DockView::Preview),
        ] {
            let ctx = editor_context();
            let mut editor = editor_fixture();
            let mut doc = editor.session.document().clone();
            let slot = doc
                .slots
                .iter()
                .position(|slot| slot.name == "body")
                .unwrap();
            let color = [255, 64, 0, 128];
            doc.slots[slot].color = color;
            editor.session = Session::new(doc).unwrap();
            editor.show_tree = false;
            editor.show_dopesheet = false;
            if let Some(view) = view {
                editor.view_tabs.open.insert(view);
            }
            let texture = ctx.load_texture(
                "slot-tint-proof",
                egui::ColorImage::from_rgba_unmultiplied([1, 1], &[255; 4]),
                egui::TextureOptions::default(),
            );
            let id = texture.id();
            editor.textures.insert("torso".into(), texture);
            let mut found = Vec::new();
            for _ in 0..3 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        focused: true,
                        screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
                        ..Default::default()
                    },
                    |ui| editor.workspace_ui(ui),
                );
                found = mesh_colors(&output.shapes, id);
                output.textures_delta.clear();
            }
            assert_eq!(found.len(), if view.is_some() { 8 } else { 4 }, "{view:?}");
            assert!(
                found.iter().all(|&value| value == attachment_tint(color)),
                "{view:?}: {found:?}"
            );
            assert_eq!(editor.session.document().slots[slot].color, color);
            assert!(!editor.session.can_undo());
        }
    }
}
