mod animation;
mod icons;
mod mesh;
mod panels;
mod properties;
mod rename;
mod selection;
mod selection_history;
mod shortcuts;
mod tools;
mod tree;
mod tree_model;
use selection::{Folder, TreeItem};
use tree_model::TreeState;
mod viewport;
use eframe::egui::{self, Color32, FontId, Pos2, RichText, Sense, Stroke, Vec2, pos2, vec2};
use peachy_document::*;
use peachy_engine::{
    Affine, Axes, Command, EditTarget, KeyState, PlayDirection, Session, TransformTool,
    scene_from_locals,
};
use peachy_io as files;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

const ACCENT: Color32 = Color32::from_rgb(235, 133, 44);
const MINT: Color32 = Color32::from_rgb(149, 188, 150);
const MUTED: Color32 = Color32::from_rgb(166, 166, 166);
const PANEL: Color32 = Color32::from_rgb(53, 53, 53);

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Rotate,
    Move,
    Scale,
    Mesh,
}

struct MeshDrag {
    attachment: AttachmentRef,
    vertex: usize,
    position: [f32; 2],
    pointer: Pos2,
}

struct TransformDrag {
    item: TreeItem,
    initial: Transform,
    parent: Affine,
    axis: Option<usize>,
    pointer: Pos2,
}

pub(crate) struct Editor {
    session: Session,
    project_path: Option<PathBuf>,
    asset_root: PathBuf,
    textures: BTreeMap<String, egui::TextureHandle>,
    selection: Option<TreeItem>,
    selection_history: selection_history::SelectionHistory,
    tree: TreeState,
    hidden_bones: BTreeSet<usize>,
    hidden_slots: BTreeSet<usize>,
    animation: usize,
    skin: String,
    setup_mode: bool,
    playing: bool,
    play_direction: PlayDirection,
    looping: bool,
    auto_key: bool,
    time: f32,
    pose_view_generation: u64,
    show_bones: bool,
    zoom: f32,
    pan: Vec2,
    fit: bool,
    tool: Tool,
    last_tool: Tool,
    axes: Axes,
    show_tree: bool,
    show_dopesheet: bool,
    hidden_views: Option<(bool, bool)>,
    drag: Option<TransformDrag>,
    mesh_vertex: usize,
    mesh_triangle: usize,
    mesh_drag: Option<MeshDrag>,
    capture_targets: BTreeMap<String, [f32; 4]>,
    mesh_handles: Vec<[f32; 2]>,
    status: String,
    error: Option<String>,
    new_bone: String,
    rename: Option<rename::RenameState>,
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
        crate::platform::configure_shortcuts();
        cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
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
        let tree = TreeState::for_document(&doc, &skin);
        let initial_bone = 8.min(doc.bones.len() - 1);
        let mut editor = Self {
            session: Session::new(doc).expect("Validated fixture"),
            project_path,
            asset_root,
            textures: BTreeMap::new(),
            selection: Some(TreeItem::Bone(initial_bone)),
            selection_history: Default::default(),
            tree,
            hidden_bones: BTreeSet::new(),
            hidden_slots: BTreeSet::new(),
            animation: 0,
            skin,
            setup_mode: true,
            playing: false,
            play_direction: PlayDirection::Forward,
            looping: true,
            auto_key: true,
            time: 0.0,
            pose_view_generation: 0,
            show_bones: true,
            zoom: 1.0,
            pan: Vec2::ZERO,
            fit: true,
            tool: Tool::Rotate,
            last_tool: Tool::Move,
            axes: Axes::Parent,
            show_tree: true,
            show_dopesheet: true,
            hidden_views: None,
            drag: None,

            mesh_vertex: 0,
            mesh_triangle: 0,
            mesh_drag: None,
            capture_targets: BTreeMap::new(),
            mesh_handles: vec![],
            status: "Original Peach Bot • select a bone and make it yours".into(),
            error: initial_error,
            new_bone: String::new(),
            rename: None,
            capture,
            capture_directory,
            frames: 0,
        };
        editor.reconcile_selection();
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

    fn selected_bone(&self) -> Option<usize> {
        self.selection.as_ref().and_then(TreeItem::bone)
    }
    fn selected_attachment(&self) -> Option<&AttachmentRef> {
        self.selection.as_ref().and_then(TreeItem::attachment)
    }
    fn transform_editable(&self) -> bool {
        match &self.selection {
            Some(TreeItem::Bone(_)) => true,
            Some(TreeItem::Attachment(k)) => {
                self.setup_mode
                    && self.attachment_editable(k)
                    && matches!(
                        self.session.document().attachment(k),
                        Some(Attachment::Region(_))
                    )
            }
            _ => false,
        }
    }
    fn selected_transform(&self) -> Option<Transform> {
        let doc = self.session.document();
        match self.selection.as_ref()? {
            TreeItem::Bone(i) => self
                .session
                .local_pose(self.active_animation(), self.time)
                .get(*i)
                .copied(),
            TreeItem::Attachment(key) if self.attachment_editable(key) => {
                if let Some(Attachment::Region(r)) = doc.attachment(key) {
                    Some(r.transform)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    fn attachment_editable(&self, key: &AttachmentRef) -> bool {
        key.skin == "default" || key.skin == self.skin
    }
    fn edit_transform(&mut self, local: Transform, properties: &[Property]) {
        if let Some(item) = self.selection.clone() {
            self.edit_item_transform(item, local, properties);
        }
    }
    fn edit_item_transform(
        &mut self,
        item: TreeItem,
        mut local: Transform,
        properties: &[Property],
    ) {
        if self.setup_mode && properties.contains(&Property::Rotation) {
            local.rotation = local.rotation.rem_euclid(360.);
        }
        self.playing = false;
        match item {
            TreeItem::Bone(bone) => {
                let target = if self.setup_mode {
                    EditTarget::Setup
                } else {
                    EditTarget::Animate {
                        animation: self.animation,
                        time: self.time.clamp(0.0, self.duration()),
                    }
                };
                if let Err(error) =
                    self.session
                        .edit_pose(bone, target, local, properties, self.auto_key)
                {
                    self.error = Some(error);
                }
            }
            TreeItem::Attachment(attachment)
                if self.setup_mode && self.attachment_editable(&attachment) =>
            {
                self.dispatch(Command::SetRegionTransform {
                    attachment,
                    transform: local,
                });
            }
            _ => {}
        }
    }

    fn select(&mut self, item: TreeItem) {
        self.cancel_gesture();
        if !item.exists(self.session.document()) {
            return;
        }
        self.selection_history
            .record(&self.selection, &Some(item.clone()));
        self.restore_selection(Some(item));
    }
    fn restore_selection(&mut self, item: Option<TreeItem>) {
        self.cancel_gesture();
        self.selection = item.clone();
        let Some(item) = item else { return };
        self.mesh_vertex = 0;
        self.mesh_triangle = 0;
        if self.tool == Tool::Mesh
            && !matches!(
                item.attachment()
                    .and_then(|k| self.session.document().attachment(k)),
                Some(Attachment::Mesh(_))
            )
        {
            self.tool = self.last_tool;
        }
        self.tree
            .reveal(self.session.document(), &self.skin, self.setup_mode, &item);
    }
    fn activate_skin(&mut self, skin: String) {
        if selection::active_skin_exists(self.session.document(), &skin) {
            self.cancel_gesture();
            self.skin = skin;
            self.reconcile_selection();
        }
    }
    fn activate_animation(&mut self, animation: usize) {
        if animation < self.session.document().animations.len() {
            self.cancel_gesture();
            self.session.discard_pose(false);
            self.pose_view_generation = self.pose_view_generation.wrapping_add(1);
            self.animation = animation;
            self.time = 0.0;
            self.setup_mode = false;
        }
    }
    fn reconcile_selection(&mut self) {
        selection::reconcile(&mut self.selection, self.session.document());
        self.selection_history.reconcile(self.session.document());
        self.animation = self
            .animation
            .min(self.session.document().animations.len().saturating_sub(1));
        if !selection::active_skin_exists(self.session.document(), &self.skin) {
            self.skin = preferred_skin(self.session.document());
        }
        self.time = self.time.clamp(0.0, self.duration());
        if self.session.document().animations.is_empty() {
            self.playing = false;
            self.setup_mode = true;
        }
        self.hidden_bones
            .retain(|&i| i < self.session.document().bones.len());
        self.hidden_slots
            .retain(|&i| i < self.session.document().slots.len());
        self.reconcile_attachment();
    }
    fn undo(&mut self) {
        self.session.undo();
        self.drag = None;
        self.mesh_drag = None;
        self.reconcile_selection();
    }
    fn redo(&mut self) {
        self.session.redo();
        self.drag = None;
        self.mesh_drag = None;
        self.reconcile_selection();
    }

    fn confirm_discard(&self) -> bool {
        (!self.session.is_dirty() && self.session.pose_edits().is_empty())
            || rfd::MessageDialog::new()
                .set_title("Unsaved changes")
                .set_description("Discard unsaved project changes and any unkeyed pose?")
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
                    self.pose_view_generation = self.pose_view_generation.wrapping_add(1);
                    self.asset_root = path.parent().unwrap().to_path_buf();
                    self.project_path =
                        if path.extension().and_then(|s| s.to_str()) == Some("peachy") {
                            Some(path)
                        } else {
                            None
                        };
                    self.selection_history = Default::default();
                    self.selection = Some(TreeItem::Skeleton);
                    self.animation = 0;
                    self.time = 0.0;
                    self.playing = false;
                    self.fit = true;
                    self.skin = preferred_skin(self.session.document());
                    self.tree = TreeState::for_document(self.session.document(), &self.skin);
                    self.hidden_bones.clear();
                    self.hidden_slots.clear();
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
                    self.status = if self.session.pose_edits().is_empty() {
                        "Project and images saved"
                    } else {
                        "Project saved; unkeyed pose is not saved. Press K to key it."
                    }
                    .into();
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
                Ok(path) => {
                    self.status = format!(
                        "Spine 4.3 export saved to {}{}",
                        path.display(),
                        if self.session.pose_edits().is_empty() {
                            ""
                        } else {
                            "; unkeyed pose excluded — press K to key it"
                        }
                    )
                }
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
            let step = peachy_engine::advance_playback(
                self.time,
                ctx.input(|i| i.stable_dt).min(0.1),
                self.duration(),
                self.looping,
                self.play_direction,
            );
            self.time = step.time;
            self.playing = step.playing;
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
                    if !self.session.pose_edits().is_empty() {
                        ui.label(
                            RichText::new("Unkeyed pose · K to key")
                                .small()
                                .color(ACCENT),
                        );
                    }
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
        self.rename_dialog(&ctx);
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
                let report = serde_json::json!({
                    "document":self.session.document(),"selection":self.selection.as_ref().map(TreeItem::diagnostic),
                    "selectedBone":self.selected_bone(),"hasSelection":self.selection.is_some(),
                    "tool":match self.tool {Tool::Rotate=>"rotate",Tool::Move=>"translate",Tool::Scale=>"scale",Tool::Mesh=>"vertices"},
                    "treeVisible":self.show_tree,"dopesheetVisible":!self.setup_mode && self.show_dopesheet,"showDopesheet":self.show_dopesheet,
                    "animation":self.animation,"skin":self.skin,"time":self.time,"playing":self.playing,"playDirection":self.play_direction,"looping":self.looping,"zoom":self.zoom,
                    "axes":self.axes,"toolValues":self.selected_transform().zip(self.selected_parent()).map(|(local,parent)|peachy_engine::transform_values(local,parent,self.axes)),
                    "autoKey":self.auto_key,"unkeyedPose":self.session.pose_edits(),"selectedTransform":self.selected_transform(),"locals":self.session.local_pose(self.active_animation(),self.time),
                    "keyStates":self.selected_bone().map(|bone| [Property::Rotation,Property::X,Property::ScaleX].map(|p| self.key_state(bone,&peachy_engine::transform_group(&[p])))),
                    "setupMode":self.setup_mode,"dirty":self.session.is_dirty(),"canUndo":self.session.can_undo(),"canRedo":self.session.can_redo(),"imageCount":self.textures.len(),
                    "selectedAttachment":self.selected_attachment(),"selectedVertex":self.mesh_vertex,"hiddenSlots":self.hidden_slots,
                    "pointer":ctx.input(|i|i.pointer.hover_pos().map(|p|[p.x,p.y])),
                    "targets":self.capture_targets,"meshHandles":self.mesh_handles,"innerSize":[ctx.content_rect().width(),ctx.content_rect().height()]
                });
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
