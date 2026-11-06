mod animation;
mod animation_management;
mod dopesheet;
mod dopesheet_selection;
mod ghosting;
mod graph;
mod icons;
mod ik;
mod key_drag;
mod layout;
mod mesh;
mod mesh_tools;
mod panels;
mod playback;
mod preview;
mod properties;
mod rename;
mod scene_view;
mod selection;
mod selection_history;
mod shortcuts;
mod style;
#[cfg(test)]
mod test_support;
mod timeline_clipboard;
mod timeline_model;
mod timeline_view;
mod tools;
mod transform_gesture;
use transform_gesture::TransformEdit;
mod tree;
mod tree_model;
use selection::{Folder, TreeItem};
use tree_model::TreeState;
mod viewport;
mod views;
mod weight_direct;
mod weights;
use eframe::egui::{self, Color32, FontId, Pos2, RichText, Sense, Stroke, Vec2, pos2, vec2};
use peachy_document::*;
#[cfg(test)]
use peachy_engine::scene_from_locals;
use peachy_engine::{
    Affine, Axes, Command, EditTarget, KeyState, PlayDirection, Session, TransformTool,
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
    Shear,
    Mesh,
    Weights,
}

struct ViewVisibility {
    tree: bool,
    dopesheet: bool,
    timeline: bool,
    weights: bool,
    dock: BTreeSet<views::DockView>,
}

struct MeshDrag {
    attachment: AttachmentRef,
    vertex: usize,
    translation: peachy_engine::MeshTranslation,
    pointer: Pos2,
    zoom: f32,
}

pub(crate) struct Editor {
    session: Session,
    project_path: Option<PathBuf>,
    asset_root: PathBuf,
    textures: BTreeMap<String, egui::TextureHandle>,
    selection: Option<TreeItem>,
    selection_history: selection_history::SelectionHistory,
    key_selection: dopesheet_selection::KeySelection,
    key_drag: Option<key_drag::KeyDrag>,
    timeline_clipboard: Option<peachy_engine::KeyClipboard>,
    timeline_input: timeline_clipboard::TimelineInput,
    tree: TreeState,
    hidden_bones: BTreeSet<usize>,
    hidden_slots: BTreeSet<usize>,
    animation: usize,
    skin: String,
    setup_mode: bool,
    playing: bool,
    play_direction: PlayDirection,
    playback_speed: f32,
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
    mesh_return_tool: Option<Tool>,
    axes: Axes,
    show_tree: bool,
    show_dopesheet: bool,
    show_timeline: bool,
    timeline_view: timeline_view::TimelineView,
    show_weights: bool,
    hidden_views: Option<ViewVisibility>,
    view_tabs: views::ViewTabs,
    ghosting: ghosting::GhostingState,
    preview: preview::PreviewView,
    graph: graph::GraphView,
    weights: weights::WeightsState,
    weight_edit: Option<weight_direct::WeightEdit>,
    weight_blocked: Option<egui::Id>,
    weight_controls: Option<[egui::Id; 2]>,
    weights_handles: Vec<[f32; 2]>,
    ik_edit: Option<ik::IkEdit>,
    ik_blocked: Option<egui::Id>,
    ik_target: Option<ik::IkTargetPicker>,
    drag: Option<TransformEdit>,
    transform_blocked: bool,
    transform_blocked_control: Option<egui::Id>,
    mesh_vertex: usize,
    mesh_triangle: usize,
    mesh_drag: Option<MeshDrag>,
    mesh_blocked: bool,
    mesh_tools: mesh_tools::MeshToolsView,
    capture_targets: BTreeMap<String, [f32; 4]>,
    mesh_handles: Vec<[f32; 2]>,
    status: String,
    error: Option<String>,
    evaluation_error: Option<String>,
    new_bone: String,
    rename: Option<rename::RenameState>,
    duplicate_animation: Option<animation_management::DuplicateAnimationState>,
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
        style::configure_editor_context(&cc.egui_ctx);
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
            key_selection: Default::default(),
            key_drag: None,
            timeline_clipboard: None,
            timeline_input: Default::default(),
            tree,
            hidden_bones: BTreeSet::new(),
            hidden_slots: BTreeSet::new(),
            animation: 0,
            skin,
            setup_mode: true,
            playing: false,
            play_direction: PlayDirection::Forward,
            playback_speed: 1.,
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
            mesh_return_tool: None,
            axes: Axes::Parent,
            show_tree: true,
            show_dopesheet: true,
            show_timeline: false,
            timeline_view: Default::default(),
            show_weights: false,
            weights: Default::default(),
            weight_edit: None,
            weight_blocked: None,
            weight_controls: None,
            weights_handles: Vec::new(),
            ik_edit: None,
            ik_blocked: None,
            ik_target: None,
            hidden_views: None,
            view_tabs: Default::default(),
            ghosting: Default::default(),
            preview: Default::default(),
            graph: Default::default(),
            drag: None,
            transform_blocked: false,
            transform_blocked_control: None,

            mesh_vertex: 0,
            mesh_triangle: 0,
            mesh_drag: None,
            mesh_blocked: false,
            mesh_tools: Default::default(),
            capture_targets: BTreeMap::new(),
            mesh_handles: vec![],
            status: "Original Peach Bot • select a bone and make it yours".into(),
            error: initial_error,
            evaluation_error: None,
            new_bone: String::new(),
            rename: None,
            duplicate_animation: None,
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
        self.ghosting.solid_textures.clear();
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
                    let mut solid = image.clone();
                    for pixel in solid.pixels_mut() {
                        pixel.0[..3].fill(255);
                    }
                    self.ghosting.solid_textures.insert(
                        path.clone(),
                        ctx.load_texture(
                            format!("ghost-solid:{path}"),
                            egui::ColorImage::from_rgba_unmultiplied(size, solid.as_raw()),
                            egui::TextureOptions::LINEAR,
                        ),
                    );
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
        self.cancel_graph_edit();
        self.cancel_ik_edit();
        self.ik_target = None;
        self.cancel_weight_edit();
        self.cancel_transform_edit();
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
    fn animation_end(&self) -> f32 {
        self.session
            .document()
            .animations
            .get(self.animation)
            .map_or(0.0, Animation::duration)
    }
    fn display_extent(&self) -> f32 {
        // Bounded view policy until timeline pan/zoom: show at least one second and
        // include the authoring cursor without adding document keys or a reserve.
        self.animation_end().max(self.time).max(1.0)
    }
    fn snapped_time(&self) -> f32 {
        animation::snap_time(self.time, self.session.document().fps)
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
    fn transform_tool_editable(&self, tool: Tool) -> bool {
        self.transform_editable() && (tool != Tool::Shear || self.selected_bone().is_some())
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
                    Some(r.transform.into())
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
        self.cancel_graph_edit();
        self.cancel_ik_edit();
        self.ik_target = None;
        self.cancel_weight_edit();
        if properties
            .iter()
            .any(|p| matches!(p, Property::ShearX | Property::ShearY))
            && !matches!(item, TreeItem::Bone(_))
        {
            self.status = "Shear edits require a bone selection".into();
            return;
        }
        if self.setup_mode && properties.contains(&Property::Rotation) {
            local.rotation = self.setup_rotation_value(Some(&item), local.rotation);
        }
        self.playing = false;
        match item {
            TreeItem::Bone(bone) => {
                let target = if self.setup_mode {
                    EditTarget::Setup
                } else {
                    EditTarget::Animate {
                        animation: self.animation,
                        time: self.time,
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
                match RegionTransform::try_from(local) {
                    Ok(transform) => {
                        self.dispatch(Command::SetRegionTransform {
                            attachment,
                            transform,
                        });
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            _ => {}
        }
    }

    fn select(&mut self, item: TreeItem) {
        if self.ik_target_selection(&item) {
            return;
        }
        if self.weights_bind_selection(&item) {
            return;
        }
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
        self.weights.bind = None;
        self.selection = item.clone();
        let Some(item) = item else { return };
        if let Some(key) = item.attachment() {
            self.capture_weights_mesh(key);
        }
        self.mesh_vertex = 0;
        self.mesh_triangle = 0;
        if self.tool == Tool::Mesh
            && !matches!(
                item.attachment()
                    .and_then(|k| self.session.document().attachment(k)),
                Some(Attachment::Mesh(_))
            )
        {
            self.return_from_mesh_tool();
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
            self.weights.bind = None;
            self.cancel_gesture();
            self.session.discard_pose(false);
            self.pose_view_generation = self.pose_view_generation.wrapping_add(1);
            self.animation = animation;
            self.time = 0.0;
            self.timeline_view.reset_range();
            self.setup_mode = false;
        }
    }
    fn reconcile_selection(&mut self) {
        self.reconcile_transform_edit();
        self.reconcile_weight_edit();
        if self.key_drag.as_ref().is_some_and(|drag| {
            self.setup_mode
                || !self.show_dopesheet
                || self.animation != drag.animation
                || self.selected_bone() != Some(drag.bone)
        }) {
            self.cancel_dopesheet_drag();
        }
        selection::reconcile(&mut self.selection, self.session.document());
        self.ghosting.reconcile(self.session.document().bones.len());
        self.reconcile_ik();
        self.selection_history.reconcile(self.session.document());
        self.animation = self
            .animation
            .min(self.session.document().animations.len().saturating_sub(1));
        if !selection::active_skin_exists(self.session.document(), &self.skin) {
            self.skin = preferred_skin(self.session.document());
        }
        self.time = if self.time.is_finite() {
            self.time.max(0.0)
        } else {
            0.0
        };
        if self.session.document().animations.is_empty() {
            self.playing = false;
            self.setup_mode = true;
        }
        self.hidden_bones
            .retain(|&i| i < self.session.document().bones.len());
        self.hidden_slots
            .retain(|&i| i < self.session.document().slots.len());
        self.reconcile_attachment();
        self.reconcile_weights();
        self.reconcile_dopesheet_selection();
    }
    fn undo(&mut self) {
        self.cancel_timeline_interaction();
        if self.graph_edit_active() {
            self.cancel_graph_edit();
            return;
        }
        self.ik_target = None;
        if self.ik_edit.is_some() {
            self.cancel_ik_edit();
            return;
        }
        self.weights.bind = None;
        if self.drag.is_some() {
            self.cancel_transform_edit();
            return;
        }
        if self.weight_edit.is_some() {
            self.cancel_weight_edit();
            return;
        }
        if self.key_drag.is_some() {
            self.cancel_dopesheet_drag();
            return;
        }
        if self.mesh_drag.is_some() {
            self.cancel_mesh_drag();
            return;
        }
        self.session.undo();
        self.key_selection.clear();
        self.drag = None;
        self.mesh_drag = None;
        self.reconcile_selection();
    }
    fn redo(&mut self) {
        self.cancel_timeline_interaction();
        if self.graph_edit_active() {
            self.cancel_graph_edit();
            return;
        }
        self.ik_target = None;
        if self.ik_edit.is_some() {
            self.cancel_ik_edit();
            return;
        }
        self.weights.bind = None;
        if self.drag.is_some() {
            self.cancel_transform_edit();
            return;
        }
        if self.weight_edit.is_some() {
            self.cancel_weight_edit();
            return;
        }
        if self.key_drag.is_some() {
            self.cancel_dopesheet_drag();
            return;
        }
        if self.mesh_drag.is_some() {
            self.cancel_mesh_drag();
            return;
        }
        self.session.redo();
        self.key_selection.clear();
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
        self.cancel_gesture();
        if !self.confirm_discard() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Peachy project or Spine 4.3 JSON", &["peachy", "json"])
            .pick_file()
        {
            match files::load(&path).and_then(Session::new) {
                Ok(session) => {
                    self.return_from_mesh_tool();
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
                    self.key_selection = Default::default();
                    self.key_drag = None;
                    self.timeline_clipboard = None;
                    self.timeline_input = Default::default();
                    self.weights = Default::default();
                    self.view_tabs.reset_document();
                    self.timeline_view.reset_document();
                    self.ghosting.reset_document();
                    self.preview.reset_document();
                    self.graph.reset_document();
                    self.weight_blocked = None;
                    self.weight_controls = None;
                    self.ik_edit = None;
                    self.ik_blocked = None;
                    self.ik_target = None;
                    self.transform_blocked = false;
                    self.transform_blocked_control = None;
                    self.weights_handles.clear();
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
                    self.rename = None;
                    self.duplicate_animation = None;
                    self.reconcile_selection();
                    self.error = None;
                    self.evaluation_error = None;
                    self.load_textures(ctx);
                    self.status = "Project opened".into();
                }
                Err(e) => self.error = Some(e),
            }
        }
    }

    fn save_project(&mut self) {
        self.cancel_gesture();
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
        self.cancel_gesture();
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

impl Editor {
    fn workspace_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if !ctx.input(|input| input.focused) {
            self.cancel_gesture();
        }
        self.reconcile_mesh_input(&ctx);
        self.mesh_tools.show_radius = false;
        self.capture_targets.clear();
        self.mesh_handles.clear();
        self.weights_handles.clear();
        self.reconcile_selection();
        self.reconcile_timeline_input(&ctx);
        self.reconcile_graph_input(&ctx);
        self.shortcuts(&ctx);
        self.advance_preview(&ctx);
        self.observe_timeline_input(&ctx);
        self.reconcile_selection();
        if self.playing {
            self.advance_playback(ctx.input(|i| i.stable_dt));
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
                    ui.label(
                        RichText::new(self.evaluation_error.as_deref().unwrap_or(&self.status))
                            .small()
                            .color(if self.evaluation_error.is_some() {
                                ACCENT
                            } else {
                                MUTED
                            }),
                    );
                });
            });
        if self.show_tree || self.has_dock_views() {
            egui::Panel::right("tree")
                .default_size(310.0)
                .min_size(260.0)
                .resizable(true)
                .frame(egui::Frame::new().fill(PANEL).inner_margin(6))
                .show(ui, |ui| {
                    if self.show_tree {
                        egui::Panel::bottom("item-properties")
                            .default_size(350.0)
                            .min_size(160.0)
                            .resizable(true)
                            .frame(egui::Frame::new().fill(PANEL).inner_margin(6))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("properties-scroll")
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| self.inspector(ui));
                            });
                    }
                    if self.show_tree && self.has_dock_views() {
                        egui::Panel::bottom("auxiliary-view-panel")
                            .default_size(260.0)
                            .min_size(160.0)
                            .resizable(true)
                            .frame(egui::Frame::new().fill(PANEL).inner_margin(6))
                            .show(ui, |ui| self.auxiliary_dock(ui));
                    }
                    egui::CentralPanel::default()
                        .frame(egui::Frame::new().fill(PANEL).inner_margin(3))
                        .show(ui, |ui| {
                            if self.show_tree {
                                self.hierarchy(ui);
                            } else {
                                self.auxiliary_dock(ui);
                            }
                        });
                });
        }
        if !self.setup_mode && self.show_dopesheet {
            let height = layout::timeline_panel_height(ui);
            egui::Panel::bottom("dopesheet")
                .default_size(height)
                .min_size(height)
                .resizable(true)
                .frame(egui::Frame::new().fill(PANEL).inner_margin(6))
                .show(ui, |ui| self.timeline(ui));
        }
        if !self.setup_mode && self.show_timeline {
            let height = layout::compact_timeline_height(ui);
            let enabled = self.error.is_none()
                && self.rename.is_none()
                && self.duplicate_animation.is_none()
                && !egui::Popup::is_any_open(&ctx)
                && ctx.input(|input| input.focused);
            egui::Panel::bottom("compact-timeline")
                .exact_size(height)
                .frame(egui::Frame::new().fill(PANEL).inner_margin(6))
                .show(ui, |ui| {
                    ui.add_enabled_ui(enabled, |ui| self.compact_timeline(ui));
                });
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_gray(43))
                    .inner_margin(0),
            )
            .show(ui, |ui| {
                let height = layout::main_tools_height(ui);
                egui::Panel::bottom("main-tools")
                    .exact_size(height)
                    .frame(egui::Frame::new().fill(PANEL).inner_margin(7))
                    .show(ui, |ui| self.main_tools(ui));
                self.canvas(ui);
            });
        self.finish_idle_transactions(&ctx);
        self.observe_preview_focus(&ctx);
        self.observe_graph_focus(&ctx);
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
        self.duplicate_animation_dialog(&ctx);
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
                let mut report = serde_json::json!({
                    "document":self.session.document(),"selection":self.selection.as_ref().map(TreeItem::diagnostic),
                    "selectedBone":self.selected_bone(),"hasSelection":self.selection.is_some(),
                    "tool":match self.tool {Tool::Rotate=>"rotate",Tool::Move=>"translate",Tool::Scale=>"scale",Tool::Shear=>"shear",Tool::Mesh=>"vertices",Tool::Weights=>"weights"},
                    "treeVisible":self.show_tree,"dopesheetVisible":!self.setup_mode && self.show_dopesheet,"showDopesheet":self.show_dopesheet,
                    "animation":self.animation,"skin":self.skin,"time":self.time,"playing":self.playing,"playDirection":self.play_direction,"looping":self.looping,"zoom":self.zoom,
                    "axes":self.axes,"toolValues":match self.selected_pose() {Ok(pose)=>Some(peachy_engine::transform_values(pose.authored,pose.parent,self.axes)),Err(_)=>None},
                    "autoKey":self.auto_key,"unkeyedPose":self.session.pose_edits(),"selectedTransform":self.selected_transform(),"locals":self.session.local_pose(self.active_animation(),self.time),
                    "keyStates":self.selected_bone().map(|bone| [Property::Rotation,Property::X,Property::ScaleX,Property::ShearX].map(|p| self.key_state(bone,&peachy_engine::transform_group(&[p])))),
                    "setupMode":self.setup_mode,"dirty":self.session.is_dirty(),"canUndo":self.session.can_undo(),"canRedo":self.session.can_redo(),"imageCount":self.textures.len(),
                    "selectedKeys":self.key_selection.diagnostic(),"dopesheetFocused":self.dopesheet_has_focus(&ctx),
                    "windowFocused":ctx.input(|input|input.focused),
                    "keyDrag":self.key_drag.as_ref().map(key_drag::KeyDrag::diagnostic),
                    "clipboardKeys":self.timeline_clipboard.as_ref().map_or(0, |clipboard| clipboard.keys.len()),
                    "selectAllHeld":self.timeline_input.select_all_held(),
                    "selectedAttachment":self.selected_attachment(),"selectedVertex":self.mesh_vertex,"hiddenSlots":self.hidden_slots,
                    "pointer":ctx.input(|i|i.pointer.hover_pos().map(|p|[p.x,p.y])),
                    "targets":self.capture_targets,"meshHandles":self.mesh_handles,"innerSize":[ctx.content_rect().width(),ctx.content_rect().height()]
                });
                report["views"] = self.views_diagnostic();
                report["compactTimeline"] = self.timeline_view_diagnostic();
                report["ghosting"] = self.ghosting_diagnostic();
                report["preview"] = self.preview_diagnostic();
                report["graph"] = self.graph_diagnostic();
                report["playbackSpeed"] = self.playback_speed.into();
                report["animationEnd"] = self.animation_end().into();
                report["weightedMeshCount"] = self.weighted_mesh_count().into();
                report["weightsContext"] = self.weights_diagnostic();
                report["weightsDirect"] = self.weight_direct_diagnostic();
                report["ik"] = self.ik_diagnostic();
                report["evaluatedPose"] = match self
                    .session
                    .evaluated_pose(self.weights_edit_target())
                {
                    Ok(pose) => {
                        serde_json::json!({"authoredLocals":pose.authored_locals(),"appliedLocals":pose.applied_locals(),"world":pose.world()})
                    }
                    Err(error) => serde_json::json!({"error":error}),
                };
                report["transformGesture"] = self.transform_gesture_diagnostic();
                report["evaluationError"] = self.evaluation_error.clone().into();
                report["shearDisplayPolicy"] = "localPendingComparison".into();
                report["meshInspection"] = self.mesh_inspection().unwrap_or_default();
                report["meshTools"] = self.mesh_tools_diagnostic();
                report["displayExtent"] = self
                    .key_drag
                    .as_ref()
                    .map_or_else(|| self.display_extent(), |drag| drag.geometry.extent)
                    .into();
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

impl eframe::App for Editor {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.workspace_ui(ui);
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
            Self::ShearX => "Shear X",
            Self::ShearY => "Shear Y",
        }
    }
}
