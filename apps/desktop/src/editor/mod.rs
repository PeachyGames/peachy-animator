mod mesh;
mod panels;
mod shortcuts;
mod viewport;
use eframe::egui::{self, Color32, FontId, Pos2, RichText, Sense, Stroke, Vec2, pos2, vec2};
use peachy_document::*;
use peachy_engine::{Command, EditTarget, Session, evaluate_scene, local_pose};
use peachy_io as files;
use std::{collections::BTreeMap, path::PathBuf};

const ACCENT: Color32 = Color32::from_rgb(235, 133, 44);
const MINT: Color32 = Color32::from_rgb(149, 188, 150);
const MUTED: Color32 = Color32::from_rgb(166, 166, 166);
const PANEL: Color32 = Color32::from_rgb(53, 53, 53);

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Rotate,
    Move,
    Mesh,
}

struct MeshDrag {
    attachment: AttachmentRef,
    vertex: usize,
    position: [f32; 2],
    pointer: Pos2,
}

pub(crate) struct Editor {
    session: Session,
    project_path: Option<PathBuf>,
    asset_root: PathBuf,
    textures: BTreeMap<String, egui::TextureHandle>,
    selected: usize,
    has_selection: bool,
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
    last_tool: Tool,
    show_tree: bool,
    show_dopesheet: bool,
    hidden_views: Option<(bool, bool)>,
    drag: Option<(Transform, Pos2)>,
    drag_preview: Option<Transform>,
    mesh_attachment: Option<AttachmentRef>,
    mesh_vertex: usize,
    mesh_triangle: usize,
    mesh_drag: Option<MeshDrag>,
    capture_targets: BTreeMap<String, [f32; 4]>,
    mesh_handles: Vec<[f32; 2]>,
    status: String,
    error: Option<String>,
    new_bone: String,
    capture: Option<PathBuf>,
    capture_directory: PathBuf,
    frames: usize,
}

impl Editor {
    fn record_target(&mut self, name: &str, rect: egui::Rect) {
        self.capture_targets.insert(
            name.into(),
            [rect.min.x, rect.min.y, rect.max.x, rect.max.y],
        );
    }
    pub(crate) fn new(
        cc: &eframe::CreationContext<'_>,
        project: Option<PathBuf>,
        capture: Option<PathBuf>,
    ) -> Self {
        cc.egui_ctx.set_theme(egui::ThemePreference::Dark);
        let mut style = (*cc.egui_ctx.style_of(egui::Theme::Dark)).clone();
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
        cc.egui_ctx.set_style_of(egui::Theme::Dark, style);
        let demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/peach-bot/peach-bot.peachy");
        let path = project.clone().unwrap_or(demo);
        let loaded = files::load(&path);
        let (doc, asset_root, initial_error) = match loaded {
            Ok(doc) => (doc, path.parent().unwrap().to_path_buf(), None),
            Err(e) => (
                serde_json::from_str(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../examples/peach-bot/peach-bot.peachy"
                )))
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
            has_selection: true,
            animation: 0,
            skin,
            setup_mode: true,
            playing: false,
            looping: true,
            time: 0.0,
            show_bones: true,
            zoom: 1.0,
            pan: Vec2::ZERO,
            fit: true,
            tool: Tool::Rotate,
            last_tool: Tool::Move,
            show_tree: true,
            show_dopesheet: true,
            hidden_views: None,
            drag: None,
            drag_preview: None,
            mesh_attachment: None,
            mesh_vertex: 0,
            mesh_triangle: 0,
            mesh_drag: None,
            capture_targets: BTreeMap::new(),
            mesh_handles: vec![],
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
        let paths: Vec<_> = self
            .session
            .document()
            .image_paths()
            .into_iter()
            .map(str::to_owned)
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

    fn dispatch(&mut self, command: Command) -> bool {
        match self.session.dispatch(command) {
            Ok(()) => {
                self.reconcile_selection();
                true
            }
            Err(error) => {
                self.error = Some(error);
                false
            }
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

    fn edit_transform(&mut self, local: Transform, properties: &[Property]) {
        if !self.has_selection {
            return;
        }
        self.playing = false;
        let target = if self.setup_mode {
            EditTarget::Setup
        } else {
            EditTarget::Animate {
                animation: self.animation,
                time: self.time.clamp(0.0, self.duration()),
            }
        };
        let _ = self.dispatch(Command::SetPose {
            bone: self.selected,
            target,
            transform: local,
            properties: properties.to_vec(),
        });
    }

    fn reconcile_selection(&mut self) {
        (self.selected, self.animation) =
            reconciled_indices(self.session.document(), self.selected, self.animation);
        self.time = self.time.clamp(0.0, self.duration());
        if self.session.document().animations.is_empty() {
            self.playing = false;
            self.setup_mode = true;
        }
        self.reconcile_attachment();
    }
    fn undo(&mut self) {
        self.session.undo();
        self.drag = None;
        self.drag_preview = None;
        self.mesh_drag = None;
        self.reconcile_selection();
    }
    fn redo(&mut self) {
        self.session.redo();
        self.drag = None;
        self.drag_preview = None;
        self.mesh_drag = None;
        self.reconcile_selection();
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
                    self.has_selection = true;
                    self.animation = 0;
                    self.time = 0.0;
                    self.playing = false;
                    self.fit = true;
                    self.skin = preferred_skin(self.session.document());
                    self.mesh_attachment = None;
                    self.mesh_drag = None;
                    self.reconcile_selection();
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
}

impl eframe::App for Editor {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.capture_targets.clear();
        self.mesh_handles.clear();
        self.reconcile_selection();
        self.shortcuts(&ctx);
        self.reconcile_selection();
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
            .frame(egui::Frame::new().fill(PANEL).inner_margin(5))
            .show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_gray(38))
                    .inner_margin(4),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if self.session.is_dirty() {
                            "Modified"
                        } else {
                            "Saved"
                        })
                        .small()
                        .color(if self.session.is_dirty() {
                            ACCENT
                        } else {
                            MUTED
                        }),
                    );
                    ui.separator();
                    ui.label(RichText::new(&self.status).small().color(MUTED));
                });
            });
        if self.show_tree {
            egui::Panel::right("tree")
                .default_size(310.0)
                .min_size(260.0)
                .resizable(true)
                .frame(egui::Frame::new().fill(PANEL).inner_margin(6))
                .show(ui, |ui| {
                    egui::Panel::bottom("item-properties")
                        .default_size(350.0)
                        .min_size(160.0)
                        .resizable(true)
                        .frame(egui::Frame::new().fill(PANEL).inner_margin(6))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("properties-scroll")
                                .show(ui, |ui| self.inspector(ui));
                        });
                    egui::CentralPanel::default()
                        .frame(egui::Frame::new().fill(PANEL).inner_margin(3))
                        .show(ui, |ui| self.hierarchy(ui));
                });
        }
        if !self.setup_mode && self.show_dopesheet {
            egui::Panel::bottom("dopesheet")
                .default_size(235.0)
                .min_size(180.0)
                .resizable(true)
                .frame(egui::Frame::new().fill(PANEL).inner_margin(6))
                .show(ui, |ui| self.timeline(ui));
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_gray(43))
                    .inner_margin(0),
            )
            .show(ui, |ui| {
                egui::Panel::bottom("main-tools")
                    .exact_size(106.0)
                    .frame(egui::Frame::new().fill(PANEL).inner_margin(7))
                    .show(ui, |ui| self.main_tools(ui));
                self.canvas(ui);
            });
        if !ctx.input(|i| i.pointer.any_down()) {
            self.session.commit_transaction();
        }
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
                let report = serde_json::json!({"document":self.session.document(),"selectedBone":self.selected,"hasSelection":self.has_selection,"tool":match self.tool {Tool::Rotate=>"rotate",Tool::Move=>"translate",Tool::Mesh=>"vertices"},"treeVisible":self.show_tree,"dopesheetVisible":!self.setup_mode && self.show_dopesheet,"animation":self.animation,"time":self.time,"playing":self.playing,"looping":self.looping,"zoom":self.zoom,"setupMode":self.setup_mode,"dirty":self.session.is_dirty(),"imageCount":self.textures.len(),"selectedAttachment":self.mesh_attachment,"selectedVertex":self.mesh_vertex,"targets":self.capture_targets,"meshHandles":self.mesh_handles,"innerSize":[ctx.content_rect().width(),ctx.content_rect().height()]});
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

fn reconciled_indices(doc: &Document, bone: usize, animation: usize) -> (usize, usize) {
    (
        bone.min(doc.bones.len().saturating_sub(1)),
        animation.min(doc.animations.len().saturating_sub(1)),
    )
}

trait PropertyLabel {
    fn label(self) -> &'static str;
}
impl PropertyLabel for Property {
    fn label(self) -> &'static str {
        match self {
            Self::X => "Position X",
            Self::Y => "Position Y",
            Self::Rotation => "Rotation",
            Self::ScaleX => "Scale X",
            Self::ScaleY => "Scale Y",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_is_valid_after_undoing_a_selected_bone_addition() {
        let doc: Document = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/peach-bot/peach-bot.peachy"
        )))
        .unwrap();
        let mut session = Session::new(doc).unwrap();
        session
            .dispatch(Command::AddBone {
                bone: Bone {
                    name: "new".into(),
                    parent: Some(0),
                    length: 30.0,
                    setup: Transform::default(),
                },
            })
            .unwrap();
        let selected = session.document().bones.len() - 1;
        session.undo();
        let (bone, animation) = reconciled_indices(session.document(), selected, 999);
        assert!(session.document().bones.get(bone).is_some());
        assert!(session.document().animations.get(animation).is_some());
    }
}
