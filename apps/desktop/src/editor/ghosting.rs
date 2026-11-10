//! Onion-skin presentation over independently sampled, checked engine scenes.
use super::*;
use peachy_engine::{GhostRange, GhostSample, GhostSampling, GhostSide, GhostSource, Scene};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GhostImages {
    Images,
    Solid,
}

struct GhostSection {
    before: bool,
    after: bool,
    before_frames: f32,
    after_frames: f32,
    step: f32,
    before_color: [u8; 3],
    after_color: [u8; 3],
}

impl GhostSection {
    fn range(&self) -> GhostRange {
        GhostRange {
            before: if self.before { self.before_frames } else { 0. },
            after: if self.after { self.after_frames } else { 0. },
            step: self.step,
        }
    }
}

pub(super) struct GhostingState {
    pub enabled: bool,
    pub on_top: bool,
    frames: GhostSection,
    keys: GhostSection,
    current: bool,
    current_color: [u8; 3],
    anchor: bool,
    looping: bool,
    images: GhostImages,
    opacity: f32,
    offset: [f32; 2],
    selection_only: bool,
    locked: bool,
    locked_bone: Option<usize>,
    pub solid_textures: BTreeMap<String, egui::TextureHandle>,
    pub error: Option<String>,
    pub rendered_samples: usize,
}

impl Default for GhostingState {
    fn default() -> Self {
        Self {
            enabled: false,
            on_top: false,
            frames: GhostSection {
                before: true,
                after: true,
                before_frames: 6.,
                after_frames: 6.,
                step: 3.,
                before_color: [85, 150, 240],
                after_color: [245, 100, 90],
            },
            keys: GhostSection {
                before: false,
                after: false,
                before_frames: 6.,
                after_frames: 6.,
                step: 1.,
                before_color: [100, 200, 220],
                after_color: [220, 170, 60],
            },
            current: false,
            current_color: [225, 225, 225],
            anchor: false,
            looping: false,
            images: GhostImages::Images,
            opacity: 0.3,
            offset: [0.; 2],
            selection_only: false,
            locked: false,
            locked_bone: None,
            solid_textures: BTreeMap::new(),
            error: None,
            rendered_samples: 0,
        }
    }
}

impl GhostingState {
    pub fn reset_document(&mut self) {
        self.locked = false;
        self.locked_bone = None;
        self.error = None;
        self.rendered_samples = 0;
    }

    pub fn reconcile(&mut self, bones: usize) {
        if self.locked_bone.is_some_and(|bone| bone >= bones) {
            self.locked = false;
            self.locked_bone = None;
        }
    }

    fn color(&self, sample: &GhostSample) -> Color32 {
        let section = if sample.source == GhostSource::KeyFrames {
            &self.keys
        } else {
            &self.frames
        };
        let [r, g, b] = match sample.side {
            GhostSide::Before => section.before_color,
            GhostSide::After => section.after_color,
            GhostSide::Current => self.current_color,
        };
        Color32::from_rgba_unmultiplied(r, g, b, (self.opacity * 255.).round() as u8)
    }
}

struct GhostPose {
    scene: Scene,
    origin: Pos2,
    color: Color32,
}

#[derive(Default)]
pub(super) struct GhostFrame {
    poses: Vec<GhostPose>,
}

impl GhostFrame {
    pub fn sample_count(&self) -> usize {
        self.poses.len()
    }
}

impl Editor {
    fn ghost_sampling(&self) -> GhostSampling {
        GhostSampling {
            frames: self.ghosting.frames.range(),
            key_frames: self.ghosting.keys.range(),
            current: self.ghosting.current,
            anchor: self.ghosting.anchor,
            looping: self.ghosting.looping && self.looping,
        }
    }

    fn ghost_bone(&self) -> Option<usize> {
        if !self.ghosting.selection_only {
            None
        } else if self.ghosting.locked {
            self.ghosting.locked_bone
        } else {
            self.selected_bone()
        }
    }

    pub(super) fn ghost_frame(
        &self,
        rect: egui::Rect,
        origin: Pos2,
        zoom: f32,
    ) -> Result<GhostFrame, String> {
        if !self.ghosting.enabled || self.setup_mode || self.ghosting.opacity == 0. {
            return Ok(GhostFrame::default());
        }
        let doc = self.session.document();
        let samples =
            peachy_engine::ghost_samples(doc, self.animation, self.time, &self.ghost_sampling())?;
        let bone = self.ghost_bone();
        let mut frame = GhostFrame::default();
        for sample in samples {
            let mut scene = peachy_engine::checked_evaluate_scene(
                doc,
                Some(self.animation),
                sample.time,
                &self.skin,
                None,
            )?;
            scene.attachments.retain(|draw| {
                !self.hidden_slots.contains(&draw.slot)
                    && bone.is_none_or(|bone| doc.slots[draw.slot].bone == bone)
            });
            let offset = self
                .ghosting
                .offset
                .map(|value| (f64::from(value) * f64::from(sample.frame_offset)) as f32);
            let origin = scene_view::screen_point(origin, zoom, offset);
            self.check_view_projection(&scene, rect, origin, zoom)?;
            frame.poses.push(GhostPose {
                scene,
                origin,
                color: self.ghosting.color(&sample),
            });
        }
        Ok(frame)
    }

    pub(super) fn paint_ghosts(&self, painter: &egui::Painter, frame: &GhostFrame, zoom: f32) {
        let textures = if self.ghosting.images == GhostImages::Solid {
            &self.ghosting.solid_textures
        } else {
            &self.textures
        };
        for pose in &frame.poses {
            for draw in &pose.scene.attachments {
                let Some(texture) = textures.get(&draw.image) else {
                    continue;
                };
                let mut mesh = egui::Mesh::with_texture(texture.id());
                let (vertices, uvs, triangles) = draw.render_geometry();
                mesh.vertices = vertices
                    .iter()
                    .zip(uvs)
                    .map(|(&point, &uv)| egui::epaint::Vertex {
                        pos: scene_view::screen_point(pose.origin, zoom, point),
                        uv: pos2(uv[0], uv[1]),
                        color: scene_view::ghost_tint(
                            pose.color,
                            draw.color,
                            self.ghosting.images == GhostImages::Solid,
                        ),
                    })
                    .collect();
                mesh.indices = triangles.to_vec();
                painter.add(egui::Shape::mesh(mesh));
            }
        }
    }

    pub(super) fn ghosting_panel(&mut self, ui: &mut egui::Ui) {
        self.record_target("ghostPanelClip", ui.clip_rect());
        let mut changed = false;
        let enabled = ui.checkbox(&mut self.ghosting.enabled, "Enable ghosting");
        self.record_target("ghostEnabled", enabled.rect);
        changed |= enabled.changed();
        let frames = egui::CollapsingHeader::new("Frames")
            .default_open(true)
            .show(ui, |ui| {
                changed |= section_controls(
                    ui,
                    &mut self.ghosting.frames,
                    "Frames",
                    false,
                    &mut self.capture_targets,
                );
                ui.horizontal(|ui| {
                    let current = ui.checkbox(&mut self.ghosting.current, "Current");
                    self.record_target("ghostCurrent", current.rect);
                    changed |= current.changed();
                    changed |= ui
                        .color_edit_button_srgb(&mut self.ghosting.current_color)
                        .changed();
                });
            });
        self.record_target("ghostSection:Frames", frames.header_response.rect);
        let keys = egui::CollapsingHeader::new("Key Frames").show(ui, |ui| {
            changed |= section_controls(
                ui,
                &mut self.ghosting.keys,
                "Keys",
                true,
                &mut self.capture_targets,
            );
        });
        self.record_target("ghostSection:Key Frames", keys.header_response.rect);
        let display = egui::CollapsingHeader::new("Display").show(ui, |ui| {
            let old = self.ghosting.images;
            let images = egui::ComboBox::from_id_salt("ghost-images")
                .selected_text(format!("{:?}", old))
                .show_ui(ui, |ui| {
                    for images in [GhostImages::Images, GhostImages::Solid] {
                        let response = ui.selectable_value(
                            &mut self.ghosting.images,
                            images,
                            format!("{images:?}"),
                        );
                        self.record_target(&format!("ghostImages:{images:?}"), response.rect);
                    }
                });
            self.record_target("ghostImagesMenu", images.response.rect);
            changed |= old != self.ghosting.images;
            let mut opacity = self.ghosting.opacity * 100.;
            changed |= numeric_slider(
                ui,
                "Opacity %",
                &mut opacity,
                0.0..=100.0,
                1.,
                "ghostOpacity",
                &mut self.capture_targets,
            );
            self.ghosting.opacity = opacity / 100.;
        });
        self.record_target("ghostSection:Display", display.header_response.rect);
        let options = egui::CollapsingHeader::new("Options").show(ui, |ui| {
            for (label, value) in [
                ("Anchor", &mut self.ghosting.anchor),
                ("On top", &mut self.ghosting.on_top),
                ("Loop", &mut self.ghosting.looping),
            ] {
                let response = ui.checkbox(value, label);
                target(
                    &mut self.capture_targets,
                    &format!("ghost{label}"),
                    response.rect,
                );
                changed |= response.changed();
            }
            ui.label(
                RichText::new("Offset · pixels per frame")
                    .small()
                    .color(MUTED),
            );
            for (axis, label) in [(0, "X offset"), (1, "Y offset")] {
                changed |= numeric_slider(
                    ui,
                    label,
                    &mut self.ghosting.offset[axis],
                    -50.0..=50.0,
                    0.25,
                    &format!("ghostOffset{axis}"),
                    &mut self.capture_targets,
                );
            }
            let selection = ui.checkbox(&mut self.ghosting.selection_only, "Selection");
            self.record_target("ghostSelection", selection.rect);
            changed |= selection.changed();
            selection.on_hover_text(
                "Show slots belonging to the selected bone. With no bone selected, show all slots.",
            );
            ui.add_enabled_ui(self.ghosting.selection_only, |ui| {
                ui.horizontal(|ui| {
                    let lock = ui.checkbox(&mut self.ghosting.locked, "Lock");
                    self.record_target("ghostLock", lock.rect);
                    if lock.changed() {
                        changed = true;
                        self.ghosting.locked_bone = self.selected_bone();
                    }
                    let refresh =
                        ui.add_enabled(self.ghosting.locked, egui::Button::new("Refresh").small());
                    self.record_target("ghostRefresh", refresh.rect);
                    if refresh.clicked() {
                        changed = true;
                        self.ghosting.locked_bone = self.selected_bone();
                    }
                });
            });
        });
        self.record_target("ghostSection:Options", options.header_response.rect);
        if changed {
            self.cancel_gesture();
        }
        if let Some(error) = &self.ghosting.error {
            ui.colored_label(ACCENT, error);
        } else {
            ui.label(
                RichText::new(format!("{} ghost poses", self.ghosting.rendered_samples))
                    .small()
                    .color(MUTED),
            );
        }
    }

    pub(super) fn ghosting_diagnostic(&self) -> serde_json::Value {
        let samples = peachy_engine::ghost_samples(
            self.session.document(),
            self.animation,
            self.time,
            &self.ghost_sampling(),
        );
        serde_json::json!({
            "enabled":self.ghosting.enabled, "active":self.ghosting.enabled && !self.setup_mode,
            "current":self.ghosting.current,"anchor":self.ghosting.anchor,
            "onTop":self.ghosting.on_top,"loop":self.ghosting.looping,"loopEffective":self.ghosting.looping && self.looping,
            "images":format!("{:?}", self.ghosting.images),"opacity":self.ghosting.opacity,
            "offset":self.ghosting.offset,"selection":self.ghosting.selection_only,
            "locked":self.ghosting.locked,"bone":self.ghost_bone(),
            "renderedSamples":self.ghosting.rendered_samples,"error":self.ghosting.error,
            "frames":{"before":self.ghosting.frames.before,"after":self.ghosting.frames.after,"beforeFrames":self.ghosting.frames.before_frames,"afterFrames":self.ghosting.frames.after_frames,"step":self.ghosting.frames.step},
            "keyFrames":{"before":self.ghosting.keys.before,"after":self.ghosting.keys.after,"beforeFrames":self.ghosting.keys.before_frames,"afterFrames":self.ghosting.keys.after_frames,"step":self.ghosting.keys.step},
            "samples":samples.ok().map(|samples|samples.into_iter().map(|sample|serde_json::json!({"time":sample.time,"frameOffset":sample.frame_offset,"source":format!("{:?}",sample.source),"side":format!("{:?}",sample.side)})).collect::<Vec<_>>())
        })
    }
}

fn target(targets: &mut BTreeMap<String, [f32; 4]>, name: &str, rect: egui::Rect) {
    targets.insert(
        name.into(),
        [rect.min.x, rect.min.y, rect.max.x, rect.max.y],
    );
}

fn numeric_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
    name: &str,
    targets: &mut BTreeMap<String, [f32; 4]>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_sized([82., 20.], egui::Label::new(label));
        ui.spacing_mut().slider_width = (ui.available_width() - 64.).clamp(40., 140.);
        let slider = ui.add(
            egui::Slider::new(value, range.clone())
                .show_value(false)
                .step_by(f64::from(step)),
        );
        target(targets, name, slider.rect);
        let number = ui.add(
            egui::DragValue::new(value)
                .range(range)
                .speed(step)
                .max_decimals(2)
                .update_while_editing(false),
        );
        number.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::DragValue, ui.is_enabled(), label)
        });
        target(targets, &format!("{name}Value"), number.rect);
        changed = slider.changed() || number.changed();
    });
    changed
}

fn section_controls(
    ui: &mut egui::Ui,
    section: &mut GhostSection,
    name: &str,
    keys: bool,
    targets: &mut BTreeMap<String, [f32; 4]>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        for (label, value, color) in [
            ("Before", &mut section.before, &mut section.before_color),
            ("After", &mut section.after, &mut section.after_color),
        ] {
            let response = ui.checkbox(value, label);
            target(targets, &format!("ghost{name}{label}"), response.rect);
            changed |= response.changed();
            changed |= ui.color_edit_button_srgb(color).changed();
        }
    });
    let old_step = section.step;
    changed |= numeric_slider(
        ui,
        "Before frames",
        &mut section.before_frames,
        0.0..=60.0,
        0.25,
        &format!("ghost{name}BeforeFrames"),
        targets,
    );
    changed |= numeric_slider(
        ui,
        "After frames",
        &mut section.after_frames,
        0.0..=60.0,
        0.25,
        &format!("ghost{name}AfterFrames"),
        targets,
    );
    changed |= numeric_slider(
        ui,
        if keys { "Key frame step" } else { "Frame step" },
        &mut section.step,
        if keys { 1.0..=16.0 } else { 0.25..=60.0 },
        if keys { 1. } else { 0.25 },
        &format!("ghost{name}Step"),
        targets,
    );
    if keys && section.step != old_step {
        section.step = section.step.round();
    }
    changed
}

#[cfg(test)]
#[path = "ghosting_tests.rs"]
mod tests;

#[cfg(test)]
mod slot_tint_tests {
    use super::super::test_support::{editor_context, editor_fixture, mesh_colors};
    use super::*;

    #[test]
    fn images_multiply_slot_rgba_and_solid_keeps_ghost_rgb_with_slot_alpha() {
        let ctx = editor_context();
        let mut editor = editor_fixture();
        let mut doc = editor.session.document().clone();
        let slot = doc
            .slots
            .iter()
            .position(|slot| slot.name == "body")
            .unwrap();
        doc.slots[slot].color = [128, 255, 0, 64];
        editor.session = Session::new(doc).unwrap();
        let texture = ctx.load_texture(
            "ghost-slot-tint-proof",
            egui::ColorImage::from_rgba_unmultiplied([1, 1], &[255; 4]),
            egui::TextureOptions::default(),
        );
        let id = texture.id();
        editor.textures.insert("torso".into(), texture.clone());
        editor
            .ghosting
            .solid_textures
            .insert("torso".into(), texture);
        for (mode, color) in [
            (GhostImages::Images, [60, 180, 0, 64]),
            (GhostImages::Solid, [120, 180, 240, 64]),
        ] {
            editor.ghosting.images = mode;
            let frame = GhostFrame {
                poses: vec![GhostPose {
                    scene: editor.scene_for_view().unwrap(),
                    origin: pos2(300., 300.),
                    color: Color32::from_rgb(120, 180, 240),
                }],
            };
            let mut output = ctx.run_ui(
                egui::RawInput {
                    focused: true,
                    ..Default::default()
                },
                |ui| editor.paint_ghosts(ui.painter(), &frame, 1.),
            );
            let colors = mesh_colors(&output.shapes, id);
            output.textures_delta.clear();
            assert_eq!(colors.len(), 4);
            assert!(
                colors
                    .iter()
                    .all(|&actual| actual == scene_view::attachment_tint(color))
            );
        }
        assert_eq!(
            editor.session.document().slots[slot].color,
            [128, 255, 0, 64]
        );
        assert!(!editor.session.can_undo());
    }
}
