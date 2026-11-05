mod files;
use eframe::egui::{self, Color32, FontId, Pos2, RichText, Sense, Stroke, Vec2, pos2, vec2};
use peachy_document::*;
use peachy_engine::{Affine, Command, Session, local_pose, world_pose};
use std::{collections::BTreeMap, path::PathBuf};

const ACCENT: Color32 = Color32::from_rgb(248, 180, 151);
const MINT: Color32 = Color32::from_rgb(110, 201, 178);
const MUTED: Color32 = Color32::from_rgb(143, 155, 170);
const PANEL: Color32 = Color32::from_rgb(28, 33, 41);

fn main() -> eframe::Result {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let mut project = None;
    let mut capture = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--capture" {
            capture = args.get(i + 1).map(PathBuf::from);
            i += 2;
        } else {
            project = Some(PathBuf::from(&args[i]));
            i += 1;
        }
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1500.0, 960.0])
            .with_min_inner_size([1120.0, 740.0])
            .with_title("Peachy Animator — First Slice"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "Peachy Animator",
        options,
        Box::new(move |cc| Ok(Box::new(Editor::new(cc, project, capture)))),
    )
}

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Rotate,
    Move,
}

struct Editor {
    session: Session,
    project_path: Option<PathBuf>,
    asset_root: PathBuf,
    textures: BTreeMap<String, egui::TextureHandle>,
    selected: usize,
    animation: usize,
    skin: String,
    setup_mode: bool,
    playing: bool,
    looping: bool,
    time: f32,
    show_bones: bool,
    zoom: f32,
    pan: Vec2,
    fit: bool,
    tool: Tool,
    drag: Option<(Transform, Pos2)>,
    drag_preview: Option<Transform>,
    status: String,
    error: Option<String>,
    new_bone: String,
    capture: Option<PathBuf>,
    capture_directory: PathBuf,
    frames: usize,
}

impl Editor {
    fn new(
        cc: &eframe::CreationContext<'_>,
        project: Option<PathBuf>,
        capture: Option<PathBuf>,
    ) -> Self {
        cc.egui_ctx.set_theme(egui::ThemePreference::Dark);
        let mut style = (*cc.egui_ctx.style_of(egui::Theme::Dark)).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = PANEL;
        style.visuals.window_fill = PANEL;
        style.visuals.selection.bg_fill = Color32::from_rgb(82, 66, 60);
        style.visuals.selection.stroke = Stroke::new(1.0, ACCENT);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(89, 71, 65);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(60, 68, 79);
        style.spacing.item_spacing = vec2(9.0, 9.0);
        style.spacing.button_padding = vec2(12.0, 7.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.0));
        cc.egui_ctx.set_style_of(egui::Theme::Dark, style);
        let demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/peach-bot/peach-bot.peachy");
        let path = project.clone().unwrap_or(demo);
        let loaded = files::load(&path);
        let (doc, asset_root, initial_error) = match loaded {
            Ok(doc) => (doc, path.parent().unwrap().to_path_buf(), None),
            Err(e) => (
                serde_json::from_str(include_str!("../../../examples/peach-bot/peach-bot.peachy"))
                    .expect("Bundled fixture is valid"),
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot"),
                Some(e),
            ),
        };
        let skin = preferred_skin(&doc);
        let capture_directory = capture
            .as_ref()
            .and_then(|p| p.parent())
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("peachy-animator-captures"));
        let project_path = if initial_error.is_none()
            && path.extension().and_then(|s| s.to_str()) == Some("peachy")
        {
            project
        } else {
            None
        };
        let mut editor = Self {
            session: Session::new(doc).expect("Validated fixture"),
            project_path,
            asset_root,
            textures: BTreeMap::new(),
            selected: 8,
            animation: 0,
            skin,
            setup_mode: false,
            playing: false,
            looping: true,
            time: 0.3,
            show_bones: true,
            zoom: 1.0,
            pan: Vec2::ZERO,
            fit: true,
            tool: Tool::Rotate,
            drag: None,
            drag_preview: None,
            status: "Original Peach Bot • select a bone and make it yours".into(),
            error: initial_error,
            new_bone: String::new(),
            capture,
            capture_directory,
            frames: 0,
        };
        editor.selected = editor
            .selected
            .min(editor.session.document().bones.len() - 1);
        editor.load_textures(&cc.egui_ctx);
        editor
    }

    fn load_textures(&mut self, ctx: &egui::Context) {
        self.textures.clear();
        let paths: std::collections::BTreeSet<_> = self
            .session
            .document()
            .skins
            .values()
            .flat_map(|s| s.values())
            .flat_map(|a| a.values())
            .map(|r| r.path.clone())
            .collect();
        let mut missing = vec![];
        for path in paths {
            let file = self.asset_root.join("images").join(format!("{path}.png"));
            match image::open(&file) {
                Ok(image) => {
                    let image = image.into_rgba8();
                    let size = [image.width() as usize, image.height() as usize];
                    self.textures.insert(
                        path.clone(),
                        ctx.load_texture(
                            path,
                            egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
                            egui::TextureOptions::LINEAR,
                        ),
                    );
                }
                Err(_) => missing.push(path),
            }
        }
        if !missing.is_empty() {
            self.error = Some(format!(
                "Missing loose PNG images: {}. Slice 1 loads images from an images/ folder next to the project; packed-atlas import is a later slice.",
                missing.join(", ")
            ));
        }
    }

    fn dispatch(&mut self, command: Command) {
        if let Err(e) = self.session.dispatch(command) {
            self.error = Some(e);
        }
    }
    fn active_animation(&self) -> Option<usize> {
        if self.setup_mode {
            None
        } else {
            Some(self.animation)
        }
    }
    fn duration(&self) -> f32 {
        self.session
            .document()
            .animations
            .get(self.animation)
            .map_or(1.0, |a| a.duration)
    }
    fn snapped_time(&self) -> f32 {
        let fps = self.session.document().fps as f32;
        (self.time * fps).round() / fps
    }

    fn key_command(&self, property: Property, local: Transform) -> Result<Command, String> {
        let setup = self.session.document().bones[self.selected].setup;
        let value = if matches!(property, Property::ScaleX | Property::ScaleY) {
            let base = property.read(setup);
            if base.abs() < 1e-8 {
                return Err("Cannot animate a zero setup scale".into());
            }
            property.read(local) / base
        } else {
            property.read(local) - property.read(setup)
        };
        let time = self.snapped_time().min(self.duration());
        let curve = self
            .session
            .document()
            .animations
            .get(self.animation)
            .and_then(|a| {
                a.tracks
                    .iter()
                    .find(|t| t.bone == self.selected && t.property == property)
            })
            .and_then(|t| t.keys.iter().find(|k| (k.time - time).abs() < 0.00001))
            .map_or(Curve::Linear, |k| k.curve);
        Ok(Command::SetKey {
            animation: self.animation,
            bone: self.selected,
            property,
            key: Keyframe { time, value, curve },
        })
    }

    fn edit_transform(&mut self, local: Transform, properties: &[Property]) {
        self.playing = false;
        if self.setup_mode {
            self.dispatch(Command::SetSetup {
                bone: self.selected,
                transform: local,
            });
        } else {
            let commands: Result<Vec<_>, _> = properties
                .iter()
                .map(|&p| self.key_command(p, local))
                .collect();
            match commands {
                Ok(commands) => self.dispatch(Command::Batch { commands }),
                Err(e) => self.error = Some(e),
            }
        }
    }

    fn confirm_discard(&self) -> bool {
        !self.session.is_dirty()
            || rfd::MessageDialog::new()
                .set_title("Unsaved changes")
                .set_description("Discard the changes in this project?")
                .set_buttons(rfd::MessageButtons::YesNo)
                .show()
                == rfd::MessageDialogResult::Yes
    }

    fn open_project(&mut self, ctx: &egui::Context) {
        if !self.confirm_discard() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Peachy project or Spine 4.3 JSON", &["peachy", "json"])
            .pick_file()
        {
            match files::load(&path).and_then(Session::new) {
                Ok(session) => {
                    self.session = session;
                    self.asset_root = path.parent().unwrap().to_path_buf();
                    self.project_path =
                        if path.extension().and_then(|s| s.to_str()) == Some("peachy") {
                            Some(path)
                        } else {
                            None
                        };
                    self.selected = 0;
                    self.animation = 0;
                    self.time = 0.0;
                    self.playing = false;
                    self.fit = true;
                    self.skin = preferred_skin(self.session.document());
                    self.error = None;
                    self.load_textures(ctx);
                    self.status = "Project opened".into();
                }
                Err(e) => self.error = Some(e),
            }
        }
    }

    fn save_project(&mut self) {
        let path = match &self.project_path {
            Some(path) => Some(path.clone()),
            None => rfd::FileDialog::new()
                .set_file_name("my-character.peachy")
                .add_filter("Peachy project", &["peachy"])
                .save_file(),
        };
        if let Some(mut path) = path {
            path.set_extension("peachy");
            match files::save(&path, self.session.document(), &self.asset_root) {
                Ok(()) => {
                    self.asset_root = path.parent().unwrap().to_path_buf();
                    self.project_path = Some(path);
                    self.session.mark_saved();
                    self.status = "Project and images saved".into();
                }
                Err(e) => self.error = Some(e),
            }
        }
    }

    fn export(&mut self) {
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("Choose parent folder for a new Spine export")
            .pick_folder()
        {
            match files::export_bundle(&folder, self.session.document(), &self.asset_root) {
                Ok(path) => self.status = format!("Spine 4.3 export saved to {}", path.display()),
                Err(e) => self.error = Some(e),
            }
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.confirm_discard() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F12)) {
            let _ = std::fs::create_dir_all(&self.capture_directory);
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            self.capture = Some(self.capture_directory.join(format!("editor-{stamp}.png")));
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save_project();
        }
        if ctx.input_mut(|i| {
            i.consume_key(
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                egui::Key::Z,
            )
        }) {
            self.session.redo();
        } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)) {
            self.session.undo();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.playing = !self.playing;
            self.setup_mode = false;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F)) {
            self.fit = true;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::R)) {
            self.tool = Tool::Rotate;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::V)) {
            self.tool = Tool::Move;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::K)) && !self.setup_mode {
            let pose = local_pose(self.session.document(), self.active_animation(), self.time);
            self.edit_transform(pose[self.selected], &Property::ALL);
        }
        for (key, delta) in [(egui::Key::ArrowLeft, -1.0), (egui::Key::ArrowRight, 1.0)] {
            if ctx.input(|i| i.key_pressed(key)) {
                self.playing = false;
                self.time = (self.snapped_time() + delta / self.session.document().fps as f32)
                    .clamp(0.0, self.duration());
            }
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("P").size(24.0).strong().color(ACCENT));
            ui.label(RichText::new("PEACHY ANIMATOR").strong().size(16.0));
            ui.separator();
            if ui.button("Open…").clicked() {
                self.open_project(ui.ctx());
            }
            if ui
                .button("Save")
                .on_hover_text("Save project · Cmd/Ctrl+S")
                .clicked()
            {
                self.save_project();
            }
            if ui
                .add_enabled(self.session.can_undo(), egui::Button::new("Undo"))
                .clicked()
            {
                self.session.undo();
            }
            if ui
                .add_enabled(self.session.can_redo(), egui::Button::new("Redo"))
                .clicked()
            {
                self.session.redo();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Export Spine 4.3…").color(Color32::from_rgb(32, 35, 39)),
                        )
                        .fill(ACCENT),
                    )
                    .clicked()
                {
                    self.export();
                }
                ui.label(RichText::new("FIRST SLICE  /  0.1.0").small().color(MUTED));
            });
        });
    }

    fn hierarchy(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.label(RichText::new("SCENE").small().color(MUTED));
        ui.heading(&self.session.document().name);
        ui.label(
            RichText::new(format!(
                "{} bones · {} slots",
                self.session.document().bones.len(),
                self.session.document().slots.len()
            ))
            .small()
            .color(MUTED),
        );
        ui.add_space(8.0);
        ui.separator();
        ui.horizontal(|ui| {
            if ui.selectable_label(self.setup_mode, "Setup").clicked() {
                self.setup_mode = true;
                self.playing = false;
            }
            if ui.selectable_label(!self.setup_mode, "Animate").clicked() {
                self.setup_mode = false;
            }
        });
        ui.add_space(8.0);
        let bones = self.session.document().bones.clone();
        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - 190.0)
            .show(ui, |ui| {
                for (i, bone) in bones.iter().enumerate() {
                    let mut depth = 0;
                    let mut parent = bone.parent;
                    while let Some(p) = parent {
                        depth += 1;
                        parent = bones[p].parent;
                    }
                    ui.horizontal(|ui| {
                        ui.add_space((depth as f32) * 12.0);
                        let label = if i == self.selected {
                            RichText::new(format!("◇  {}", bone.name)).color(ACCENT)
                        } else {
                            RichText::new(format!("◇  {}", bone.name))
                        };
                        if ui.selectable_label(i == self.selected, label).clicked() {
                            self.selected = i;
                        }
                    });
                }
            });
        ui.separator();
        ui.label(RichText::new("ADD A CHILD BONE").small().color(MUTED));
        ui.text_edit_singleline(&mut self.new_bone);
        if ui.button("+ Add to selected bone").clicked() {
            let parent = self.selected;
            self.dispatch(Command::AddBone {
                bone: Bone {
                    name: self.new_bone.trim().to_owned(),
                    parent: Some(parent),
                    length: 60.0,
                    setup: Transform {
                        x: self.session.document().bones[parent].length,
                        ..Default::default()
                    },
                },
            });
            if self.error.is_none() {
                self.selected = self.session.document().bones.len() - 1;
                self.new_bone.clear();
            }
        }
        ui.add_space(12.0);
        ui.label(
            RichText::new("A little rig. A real workflow.")
                .color(MUTED)
                .italics(),
        );
    }

    fn inspector(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.label(RichText::new("INSPECTOR").small().color(MUTED));
        ui.heading(&self.session.document().bones[self.selected].name);
        ui.label(
            RichText::new(if self.setup_mode {
                "Setup pose"
            } else {
                "Animation pose · edits create keys"
            })
            .color(MUTED)
            .small(),
        );
        ui.add_space(8.0);
        ui.separator();
        let mut pose =
            local_pose(self.session.document(), self.active_animation(), self.time)[self.selected];
        for property in Property::ALL {
            let mut value = property.read(pose);
            ui.horizontal(|ui| {
                ui.add_sized([105.0, 22.0], egui::Label::new(property.label()));
                let drag = ui.add(
                    egui::DragValue::new(&mut value)
                        .speed(if matches!(property, Property::ScaleX | Property::ScaleY) {
                            0.01
                        } else {
                            0.25
                        })
                        .max_decimals(2),
                );
                if drag.changed() {
                    property.write(&mut pose, value);
                    self.edit_transform(pose, &[property]);
                }
                if !self.setup_mode
                    && ui
                        .small_button("◇")
                        .on_hover_text("Key this property at the playhead")
                        .clicked()
                {
                    self.edit_transform(pose, &[property]);
                }
            });
        }
        ui.add_space(10.0);
        if !self.setup_mode {
            if ui.button("◇ Key entire transform  [K]").clicked() {
                self.edit_transform(pose, &Property::ALL);
            }
            ui.add_space(10.0);
            ui.separator();
            ui.label(RichText::new("KEY INTERPOLATION").small().color(MUTED));
            let time = self.snapped_time();
            let tracks = self
                .session
                .document()
                .animations
                .get(self.animation)
                .map(|a| a.tracks.clone())
                .unwrap_or_default();
            let mut found = false;
            let selected = self.selected;
            for track in tracks.iter().filter(|t| t.bone == selected) {
                if let Some(key) = track.keys.iter().find(|k| (k.time - time).abs() < 0.00001) {
                    found = true;
                    let mut curve = key.curve;
                    ui.label(track.property.label());
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(curve == Curve::Linear, "Linear")
                            .clicked()
                        {
                            curve = Curve::Linear;
                        }
                        if ui
                            .selectable_label(curve == Curve::Stepped, "Hold")
                            .clicked()
                        {
                            curve = Curve::Stepped;
                        }
                        if ui
                            .selectable_label(matches!(curve, Curve::Bezier(_)), "Ease")
                            .clicked()
                        {
                            curve = Curve::Bezier([0.42, 0.0, 0.58, 1.0]);
                        }
                    });
                    if let Curve::Bezier(ref mut handles) = curve {
                        for (i, label) in ["Time 1", "Value 1", "Time 2", "Value 2"]
                            .iter()
                            .enumerate()
                        {
                            let range = if i % 2 == 0 { 0.0..=1.0 } else { -1.0..=2.0 };
                            ui.add(egui::Slider::new(&mut handles[i], range).text(*label));
                        }
                        if handles[0] > handles[2] {
                            handles.swap(0, 2);
                        }
                        let (r, _) = ui
                            .allocate_exact_size(vec2(ui.available_width(), 66.0), Sense::hover());
                        let points: Vec<_> = (0..=40)
                            .map(|i| {
                                let t = i as f32 / 40.0;
                                pos2(
                                    r.left() + r.width() * t,
                                    r.bottom()
                                        - r.height() * peachy_engine::curve_progress(curve, t),
                                )
                            })
                            .collect();
                        ui.painter()
                            .rect_filled(r, 4.0, Color32::from_rgb(21, 26, 33));
                        ui.painter()
                            .add(egui::Shape::line(points, Stroke::new(2.0, MINT)));
                    }
                    if curve != key.curve {
                        self.dispatch(Command::SetKey {
                            animation: self.animation,
                            bone: self.selected,
                            property: track.property,
                            key: Keyframe { curve, ..*key },
                        });
                    }
                    if ui.small_button("Remove key").clicked() {
                        self.dispatch(Command::DeleteKey {
                            animation: self.animation,
                            bone: self.selected,
                            property: track.property,
                            time,
                        });
                    }
                }
            }
            if !found {
                ui.label(
                    RichText::new("Select a diamond on the timeline to edit its curve.")
                        .color(MUTED),
                );
            }
        }
        ui.add_space(14.0);
        ui.separator();
        ui.label(RichText::new("SKIN").small().color(MUTED));
        egui::ComboBox::from_id_salt("skin")
            .selected_text(&self.skin)
            .show_ui(ui, |ui| {
                for skin in self.session.document().skins.keys() {
                    ui.selectable_value(&mut self.skin, skin.clone(), skin);
                }
            });
    }

    fn timeline(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("TIMELINE").small().color(MUTED));
            if ui
                .button(if self.playing { "Pause" } else { "Play" })
                .on_hover_text("Space")
                .clicked()
            {
                self.playing = !self.playing;
                self.setup_mode = false;
            }
            if ui.small_button("|◀").clicked() {
                self.time = 0.0;
                self.playing = false;
            }
            egui::ComboBox::from_id_salt("animation")
                .selected_text(
                    self.session
                        .document()
                        .animations
                        .get(self.animation)
                        .map_or("No animation", |a| a.name.as_str()),
                )
                .show_ui(ui, |ui| {
                    for (i, animation) in self.session.document().animations.iter().enumerate() {
                        if ui
                            .selectable_value(&mut self.animation, i, &animation.name)
                            .changed()
                        {
                            self.time = 0.0;
                        }
                    }
                });
            ui.checkbox(&mut self.looping, "Loop");
            ui.label(
                RichText::new(format!(
                    "{:03} / {:03}  ·  {} fps",
                    (self.time * self.session.document().fps as f32).round() as u32,
                    (self.duration() * self.session.document().fps as f32).round() as u32,
                    self.session.document().fps
                ))
                .monospace()
                .color(ACCENT),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new("Click to scrub · arrows step frames · K adds keys")
                        .small()
                        .color(MUTED),
                );
            });
        });
        ui.add_space(5.0);
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), 157.0), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 5.0, Color32::from_rgb(21, 26, 33));
        let label_width = 160.0;
        let start = rect.left() + label_width;
        let width = rect.width() - label_width - 18.0;
        let duration = self.duration();
        for i in 0..=12 {
            let x = start + width * i as f32 / 12.0;
            painter.line_segment(
                [pos2(x, rect.top() + 24.0), pos2(x, rect.bottom())],
                Stroke::new(1.0, Color32::from_rgb(43, 49, 58)),
            );
            painter.text(
                pos2(x, rect.top() + 11.0),
                egui::Align2::CENTER_CENTER,
                format!("{:.1}", duration * i as f32 / 12.0),
                FontId::monospace(11.0),
                MUTED,
            );
        }
        let tracks = self
            .session
            .document()
            .animations
            .get(self.animation)
            .map(|a| a.tracks.clone())
            .unwrap_or_default();
        for (i, property) in Property::ALL.into_iter().enumerate() {
            let y = rect.top() + 38.0 + i as f32 * 24.0;
            painter.text(
                pos2(rect.left() + 12.0, y),
                egui::Align2::LEFT_CENTER,
                property.label(),
                FontId::proportional(12.0),
                MUTED,
            );
            if let Some(track) = tracks
                .iter()
                .find(|t| t.bone == self.selected && t.property == property)
            {
                for key in &track.keys {
                    let x = start + width * key.time / duration;
                    let p = pos2(x, y);
                    let selected = (key.time - self.snapped_time()).abs() < 0.00001;
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            p + vec2(0.0, -5.0),
                            p + vec2(5.0, 0.0),
                            p + vec2(0.0, 5.0),
                            p + vec2(-5.0, 0.0),
                        ],
                        if selected { ACCENT } else { MINT },
                        Stroke::NONE,
                    ));
                    if response.clicked()
                        && response
                            .interact_pointer_pos()
                            .is_some_and(|mouse| mouse.distance(p) < 10.0)
                    {
                        self.time = key.time;
                        self.playing = false;
                    }
                }
            }
        }
        let playhead = start + width * self.time / duration;
        painter.line_segment(
            [
                pos2(playhead, rect.top() + 22.0),
                pos2(playhead, rect.bottom()),
            ],
            Stroke::new(1.5, ACCENT),
        );
        painter.circle_filled(pos2(playhead, rect.top() + 22.0), 4.0, ACCENT);
        if (response.dragged() || response.clicked())
            && let Some(p) = response.interact_pointer_pos()
            && p.x >= start
        {
            self.playing = false;
            self.setup_mode = false;
            self.time = ((p.x - start) / width * duration).clamp(0.0, duration);
            self.time = self.snapped_time().min(duration);
        }
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.tool == Tool::Rotate, "Rotate  [R]")
                .clicked()
            {
                self.tool = Tool::Rotate;
            }
            if ui
                .selectable_label(self.tool == Tool::Move, "Move  [V]")
                .clicked()
            {
                self.tool = Tool::Move;
            }
            ui.separator();
            ui.checkbox(&mut self.show_bones, "Bones");
            if ui.button("Fit  [F]").clicked() {
                self.fit = true;
            }
            ui.label(RichText::new(format!("{}%", (self.zoom * 100.0) as u32)).color(MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(if self.setup_mode { "SETUP" } else { "ANIMATE" })
                        .color(ACCENT)
                        .small(),
                );
            });
        });
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 6.0, Color32::from_rgb(19, 24, 31));
        let doc = self.session.document();
        let mut locals = local_pose(doc, self.active_animation(), self.time);
        if let Some(preview) = self.drag_preview {
            locals[self.selected] = preview;
        }
        let world = world_pose(doc, &locals);
        if self.fit {
            let mut low = [f32::MAX; 2];
            let mut high = [f32::MIN; 2];
            for slot in &doc.slots {
                if let Some(region) = slot
                    .attachment
                    .as_ref()
                    .and_then(|a| find_region(doc, &self.skin, slot, a))
                {
                    let matrix = world[slot.bone].then(Affine::from_transform(region.transform));
                    for p in corners(region.width, region.height) {
                        let p = matrix.point(p[0], p[1]);
                        for i in 0..2 {
                            low[i] = low[i].min(p[i]);
                            high[i] = high[i].max(p[i]);
                        }
                    }
                }
            }
            if low[0].is_finite() && high[0] > low[0] {
                self.zoom = ((rect.width() - 140.0) / (high[0] - low[0]))
                    .min((rect.height() - 100.0) / (high[1] - low[1]))
                    .clamp(0.1, 4.0);
                self.pan = vec2(
                    -(low[0] + high[0]) * 0.5 * self.zoom,
                    (low[1] + high[1]) * 0.5 * self.zoom,
                );
            }
            self.fit = false;
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let previous = self.zoom;
                self.zoom = (self.zoom * (scroll * 0.0015).exp()).clamp(0.1, 6.0);
                self.pan *= self.zoom / previous;
            }
        }
        if response.dragged_by(egui::PointerButton::Secondary)
            || response.dragged_by(egui::PointerButton::Middle)
        {
            self.pan += response.drag_delta();
        }
        let origin = rect.center() + self.pan;
        let screen = |p: [f32; 2]| origin + vec2(p[0] * self.zoom, -p[1] * self.zoom);
        let grid = 50.0 * self.zoom;
        if grid >= 8.0 {
            let mut x = origin.x + (rect.left() - origin.x).div_euclid(grid) * grid;
            while x < rect.right() {
                painter.line_segment(
                    [pos2(x, rect.top()), pos2(x, rect.bottom())],
                    Stroke::new(1.0, Color32::from_rgb(29, 36, 45)),
                );
                x += grid;
            }
            let mut y = origin.y + (rect.top() - origin.y).div_euclid(grid) * grid;
            while y < rect.bottom() {
                painter.line_segment(
                    [pos2(rect.left(), y), pos2(rect.right(), y)],
                    Stroke::new(1.0, Color32::from_rgb(29, 36, 45)),
                );
                y += grid;
            }
        }
        painter.line_segment(
            [pos2(rect.left(), origin.y), pos2(rect.right(), origin.y)],
            Stroke::new(1.0, Color32::from_rgb(70, 47, 48)),
        );
        painter.line_segment(
            [pos2(origin.x, rect.top()), pos2(origin.x, rect.bottom())],
            Stroke::new(1.0, Color32::from_rgb(38, 67, 60)),
        );
        for slot in &doc.slots {
            let Some(region) = slot
                .attachment
                .as_ref()
                .and_then(|a| find_region(doc, &self.skin, slot, a))
            else {
                continue;
            };
            let Some(texture) = self.textures.get(&region.path) else {
                continue;
            };
            let matrix = world[slot.bone].then(Affine::from_transform(region.transform));
            let mut mesh = egui::Mesh::with_texture(texture.id());
            for (p, uv) in corners(region.width, region.height).into_iter().zip([
                [0., 1.],
                [1., 1.],
                [1., 0.],
                [0., 0.],
            ]) {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: screen(matrix.point(p[0], p[1])),
                    uv: pos2(uv[0], uv[1]),
                    color: Color32::WHITE,
                });
            }
            mesh.indices = vec![0, 1, 2, 0, 2, 3];
            painter.add(egui::Shape::mesh(mesh));
        }
        let mut nearest: Option<(usize, f32)> = None;
        for (i, (bone, matrix)) in doc.bones.iter().zip(&world).enumerate() {
            let start = screen(matrix.point(0.0, 0.0));
            let end = screen(matrix.point(bone.length, 0.0));
            if self.show_bones || i == self.selected {
                let color = if i == self.selected {
                    ACCENT
                } else {
                    Color32::from_rgba_unmultiplied(117, 205, 185, 175)
                };
                painter.line_segment(
                    [start, end],
                    Stroke::new(if i == self.selected { 2.5 } else { 1.3 }, color),
                );
                painter.circle_filled(start, if i == self.selected { 5.0 } else { 3.0 }, color);
                if i == self.selected {
                    painter.circle_stroke(start, 13.0, Stroke::new(1.0, ACCENT));
                    painter.text(
                        end + vec2(10.0, -10.0),
                        egui::Align2::LEFT_CENTER,
                        &bone.name,
                        FontId::proportional(12.0),
                        ACCENT,
                    );
                }
            }
            if let Some(mouse) = response.interact_pointer_pos() {
                let distance = distance_to_segment(mouse, start, end);
                if distance < 16.0 && nearest.is_none_or(|(_, best)| distance < best) {
                    nearest = Some((i, distance));
                }
            }
        }
        painter.text(
            rect.left_bottom() + vec2(16.0, -18.0),
            egui::Align2::LEFT_BOTTOM,
            "Drag a bone to pose · scroll to zoom · right-drag to pan",
            FontId::proportional(12.0),
            MUTED,
        );
        if response.clicked()
            && let Some((i, _)) = nearest
        {
            self.selected = i;
        }
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some((i, _)) = nearest
        {
            self.selected = i;
            self.playing = false;
            if let Some(mouse) = response.interact_pointer_pos() {
                self.drag = Some((locals[i], mouse));
            }
        }
        if response.dragged_by(egui::PointerButton::Primary)
            && let Some((initial, start)) = self.drag
        {
            let mut preview = initial;
            if let Some(mouse) = response.interact_pointer_pos() {
                match self.tool {
                    Tool::Move => {
                        let delta = (mouse - start) / self.zoom;
                        if let Some(parent) = doc.bones[self.selected].parent {
                            let matrix = world[parent];
                            let a = matrix.inverse_point(0.0, 0.0);
                            let b = matrix.inverse_point(delta.x, -delta.y);
                            if let (Some(a), Some(b)) = (a, b) {
                                preview.x += b[0] - a[0];
                                preview.y += b[1] - a[1];
                            }
                        } else {
                            preview.x += delta.x;
                            preview.y -= delta.y;
                        }
                    }
                    Tool::Rotate => {
                        let matrix = world[self.selected];
                        let pivot = screen(matrix.point(0.0, 0.0));
                        let convert = |p: Pos2| {
                            let x = (p.x - origin.x) / self.zoom;
                            let y = -(p.y - origin.y) / self.zoom;
                            if let Some(parent) = doc.bones[self.selected].parent {
                                world[parent]
                                    .inverse_point(x, y)
                                    .map(|p| vec2(p[0] - initial.x, p[1] - initial.y))
                            } else {
                                Some(vec2(x - initial.x, y - initial.y))
                            }
                        };
                        if start.distance(pivot) > 6.0 {
                            if let (Some(a), Some(b)) = (convert(start), convert(mouse)) {
                                let mut delta = (b.y.atan2(b.x) - a.y.atan2(a.x)).to_degrees();
                                while delta > 180.0 {
                                    delta -= 360.0;
                                }
                                while delta < -180.0 {
                                    delta += 360.0;
                                }
                                preview.rotation += delta;
                            }
                        } else {
                            preview.rotation += (mouse.x - start.x) * 0.5;
                        }
                    }
                }
                self.drag_preview = Some(preview);
            }
        }
        if response.drag_stopped() {
            if let Some(preview) = self.drag_preview.take() {
                let properties = if self.tool == Tool::Rotate {
                    &[Property::Rotation][..]
                } else {
                    &[Property::X, Property::Y][..]
                };
                self.edit_transform(preview, properties);
            }
            self.drag = None;
        }
    }
}

impl eframe::App for Editor {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.shortcuts(&ctx);
        if self.playing {
            self.time += ctx.input(|i| i.stable_dt).min(0.1);
            let duration = self.duration();
            if self.time > duration {
                if self.looping {
                    self.time %= duration;
                } else {
                    self.time = duration;
                    self.playing = false;
                }
            }
            ctx.request_repaint();
        }
        egui::Panel::top("toolbar")
            .frame(egui::Frame::new().fill(PANEL).inner_margin(14))
            .show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(23, 28, 35))
                    .inner_margin(8),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if self.session.is_dirty() {
                            "● Unsaved"
                        } else {
                            "● Saved"
                        })
                        .small()
                        .color(if self.session.is_dirty() {
                            ACCENT
                        } else {
                            MINT
                        }),
                    );
                    ui.separator();
                    ui.label(RichText::new(&self.status).small().color(MUTED));
                });
            });
        egui::Panel::bottom("timeline")
            .default_size(222.0)
            .min_size(200.0)
            .resizable(true)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(12))
            .show(ui, |ui| self.timeline(ui));
        egui::Panel::left("hierarchy")
            .default_size(232.0)
            .min_size(200.0)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(14))
            .show(ui, |ui| self.hierarchy(ui));
        egui::Panel::right("inspector")
            .default_size(280.0)
            .min_size(250.0)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(16))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.inspector(ui));
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(23, 28, 35))
                    .inner_margin(12),
            )
            .show(ui, |ui| self.canvas(ui));
        if let Some(error) = self.error.clone() {
            egui::Window::new("Project needs attention")
                .collapsible(false)
                .default_width(530.0)
                .show(&ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(420.0)
                        .show(ui, |ui| {
                            ui.label(error);
                        });
                    if ui.button("Close").clicked() {
                        self.error = None;
                    }
                });
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "{}{} — Peachy Animator",
            if self.session.is_dirty() { "* " } else { "" },
            self.session.document().name
        )));
        self.frames += 1;
        if self.capture.is_some() && self.frames == 30 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Screenshot { image, .. } = event
                && let Some(path) = self.capture.take()
            {
                let pixels: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                if let Err(e) = image::save_buffer(
                    &path,
                    &pixels,
                    image.size[0] as u32,
                    image.size[1] as u32,
                    image::ColorType::Rgba8,
                ) {
                    eprintln!("Capture failed: {e}");
                }
                let report = serde_json::json!({"document":self.session.document(),"selectedBone":self.selected,"animation":self.animation,"time":self.time,"playing":self.playing,"setupMode":self.setup_mode,"dirty":self.session.is_dirty(),"imageCount":self.textures.len()});
                let _ = std::fs::write(
                    path.with_extension("json"),
                    serde_json::to_vec_pretty(&report).unwrap_or_default(),
                );
            }
        }
        if self.capture.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(25));
        }
    }
}

fn corners(width: f32, height: f32) -> [[f32; 2]; 4] {
    [
        [-width / 2.0, -height / 2.0],
        [width / 2.0, -height / 2.0],
        [width / 2.0, height / 2.0],
        [-width / 2.0, height / 2.0],
    ]
}
fn distance_to_segment(point: Pos2, start: Pos2, end: Pos2) -> f32 {
    let v = end - start;
    let length = v.length_sq();
    if length < 1.0 {
        return point.distance(start);
    }
    let t = ((point - start).dot(v) / length).clamp(0.0, 1.0);
    point.distance(start + v * t)
}

fn find_region<'a>(
    doc: &'a Document,
    skin: &str,
    slot: &Slot,
    attachment: &str,
) -> Option<&'a Region> {
    doc.skins
        .get(skin)
        .and_then(|s| s.get(&slot.name))
        .and_then(|r| r.get(attachment))
        .or_else(|| {
            doc.skins
                .get("default")
                .and_then(|s| s.get(&slot.name))
                .and_then(|r| r.get(attachment))
        })
}

fn preferred_skin(doc: &Document) -> String {
    if doc.skins.contains_key("peach") {
        return "peach".into();
    }
    doc.skins
        .keys()
        .find(|name| name.as_str() != "default")
        .or_else(|| doc.skins.keys().next())
        .cloned()
        .unwrap_or_default()
}
