//! View discovery and small, real panels over the shared document/evaluated scene.
use super::icons::Icon;
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum DockView {
    Animations,
    Ghosting,
    Metrics,
    Outline,
    Playback,
    Skins,
    Weights,
}

#[cfg(test)]
#[path = "views_tests.rs"]
mod tests;

impl DockView {
    const ALL: [Self; 7] = [
        Self::Animations,
        Self::Ghosting,
        Self::Metrics,
        Self::Outline,
        Self::Playback,
        Self::Skins,
        Self::Weights,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::Animations => "Animations",
            Self::Ghosting => "Ghosting",
            Self::Metrics => "Metrics",
            Self::Outline => "Outline",
            Self::Playback => "Playback",
            Self::Skins => "Skins",
            Self::Weights => "Weights",
        }
    }
    fn icon(self) -> Icon {
        match self {
            Self::Animations => Icon::Animations,
            Self::Ghosting => Icon::Ghosting,
            Self::Metrics => Icon::Metrics,
            Self::Outline => Icon::Outline,
            Self::Playback => Icon::Playback,
            Self::Skins => Icon::Skins,
            Self::Weights => Icon::Weights,
        }
    }

    fn available_in_setup(self) -> bool {
        !matches!(self, Self::Animations | Self::Ghosting | Self::Playback)
    }
}

pub(super) struct ViewTabs {
    pub open: BTreeSet<DockView>,
    active: DockView,
    animation_query: String,
    skin_query: String,
    outline_zoom: f32,
    outline_pan: Vec2,
    outline_fit: bool,
}

impl Default for ViewTabs {
    fn default() -> Self {
        Self {
            open: BTreeSet::new(),
            active: DockView::Weights,
            animation_query: String::new(),
            skin_query: String::new(),
            outline_zoom: 1.,
            outline_pan: Vec2::ZERO,
            outline_fit: true,
        }
    }
}

impl ViewTabs {
    pub(super) fn reset_document(&mut self) {
        self.animation_query.clear();
        self.skin_query.clear();
        self.outline_zoom = 1.;
        self.outline_pan = Vec2::ZERO;
        self.outline_fit = true;
    }
}

/// The public guide's complete view inventory. Unsupported entries remain discoverable.
const UNIMPLEMENTED: [(&str, &str); 5] = [
    (
        "Audio",
        "Audio assets, waveform display and playback are not implemented.",
    ),
    (
        "Graph",
        "The editable value graph is not implemented. Use Dopesheet for supported key timing and curves.",
    ),
    (
        "Mesh Tools",
        "A separate Mesh Tools view is not implemented. Select an attachment and use Vertices for the current mesh subset.",
    ),
    (
        "Preview",
        "Independent animation playback and track mixing are not implemented. Outline follows the current pose.",
    ),
    (
        "Slot Color",
        "Slot color data and color timelines are not implemented.",
    ),
];

impl Editor {
    fn dock_view_open(&self, view: DockView) -> bool {
        if view == DockView::Weights {
            self.show_weights
        } else {
            self.view_tabs.open.contains(&view)
        }
    }

    fn visible_dock_views(&self) -> Vec<DockView> {
        DockView::ALL
            .into_iter()
            .filter(|&view| {
                self.dock_view_open(view) && (!self.setup_mode || view.available_in_setup())
            })
            .collect()
    }

    pub(super) fn has_dock_views(&self) -> bool {
        !self.visible_dock_views().is_empty()
    }

    pub(super) fn focus_weights_tab(&mut self) {
        self.view_tabs.active = DockView::Weights;
    }

    fn open_dock_view(&mut self, view: DockView) {
        self.cancel_gesture();
        self.weights.bind = None;
        self.restore_hidden_views();
        if view == DockView::Weights {
            self.show_weights = true;
        } else {
            self.view_tabs.open.insert(view);
        }
        self.view_tabs.active = view;
    }

    pub(super) fn restore_hidden_views(&mut self) {
        if let Some(views) = self.hidden_views.take() {
            self.show_tree = views.tree;
            self.show_dopesheet = views.dopesheet;
            self.show_timeline = views.timeline;
            self.show_weights = views.weights;
            self.view_tabs.open = views.dock;
        }
    }

    pub(super) fn views_menu(&mut self, ui: &mut egui::Ui) {
        for view in DockView::ALL {
            let enabled = !self.setup_mode || view.available_in_setup();
            let response = ui
                .add_enabled_ui(enabled, |ui| {
                    icons::tool_button(ui, view.icon(), view.label(), self.dock_view_open(view))
                })
                .inner
                .on_disabled_hover_text("Available in Animate mode.");
            self.record_target(&format!("view{}", view.label()), response.rect);
            if response.clicked() {
                self.open_dock_view(view);
                ui.close();
            }
        }
        let dopesheet = ui
            .add_enabled_ui(!self.setup_mode, |ui| {
                icons::tool_button(ui, Icon::Dopesheet, "Dopesheet", self.show_dopesheet)
            })
            .inner
            .on_disabled_hover_text("Available in Animate mode.");
        self.record_target("viewDopesheet", dopesheet.rect);
        if dopesheet.clicked() {
            self.cancel_gesture();
            self.restore_hidden_views();
            self.show_dopesheet = true;
            ui.memory_mut(|memory| memory.request_focus(Self::dopesheet_id()));
            ui.close();
        }
        let timeline = ui
            .add_enabled_ui(!self.setup_mode, |ui| {
                icons::tool_button(ui, Icon::Timeline, "Timeline", self.show_timeline)
            })
            .inner
            .on_disabled_hover_text("Available in Animate mode.");
        self.record_target("viewTimeline", timeline.rect);
        if timeline.clicked() {
            self.cancel_gesture();
            self.restore_hidden_views();
            self.show_timeline = true;
            ui.memory_mut(|memory| memory.request_focus(Self::timeline_id()));
            ui.close();
        }
        let tree = icons::tool_button(ui, Icon::Tree, "Tree", self.show_tree);
        self.record_target("viewTree", tree.rect);
        if tree.clicked() {
            self.cancel_gesture();
            self.restore_hidden_views();
            self.show_tree = true;
            ui.close();
        }
        ui.separator();
        ui.label(RichText::new("Not implemented yet").small().color(MUTED));
        for (label, reason) in UNIMPLEMENTED {
            let response = ui
                .add_enabled(false, egui::Button::new(label))
                .on_disabled_hover_text(reason);
            self.record_target(&format!("view{label}"), response.rect);
        }
        ui.separator();
        let restore = icons::button(
            ui,
            Icon::RestoreWorkspace,
            "Restore default workspace",
            None,
        );
        self.record_target("viewRestore", restore.rect);
        if restore.clicked() {
            self.cancel_gesture();
            self.show_tree = true;
            self.show_dopesheet = true;
            self.show_timeline = false;
            self.show_weights = false;
            self.view_tabs.open.clear();
            self.ghosting.enabled = false;
            self.weights.bind = None;
            self.hidden_views = None;
            ui.close();
        }
    }

    pub(super) fn auxiliary_dock(&mut self, ui: &mut egui::Ui) {
        let visible = self.visible_dock_views();
        if !visible.contains(&self.view_tabs.active) {
            let Some(&view) = visible.first() else { return };
            self.view_tabs.active = view;
        }
        if visible.len() > 1 {
            egui::ScrollArea::horizontal()
                .id_salt("view-tabs")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for &view in &visible {
                            let tab = icons::tool_button(
                                ui,
                                view.icon(),
                                view.label(),
                                self.view_tabs.active == view,
                            );
                            self.record_target(&format!("viewTab:{}", view.label()), tab.rect);
                            if tab.clicked() {
                                self.cancel_gesture();
                                self.weights.bind = None;
                                self.view_tabs.active = view;
                            }
                        }
                    });
                });
        }
        let view = self.view_tabs.active;
        if view != DockView::Weights {
            ui.horizontal(|ui| {
                icons::heading(ui, view.icon(), view.label());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close = icons::toolbar_button(
                        ui,
                        Icon::Delete,
                        &format!("Close {} view", view.label()),
                        "",
                        true,
                    );
                    self.record_target(&format!("closeView:{}", view.label()), close.rect);
                    if close.clicked() {
                        self.cancel_gesture();
                        self.weights.bind = None;
                        if view == DockView::Weights {
                            self.show_weights = false;
                        } else {
                            self.view_tabs.open.remove(&view);
                        }
                    }
                });
            });
            ui.separator();
        }
        if view == DockView::Outline {
            self.outline_panel(ui);
        } else {
            egui::ScrollArea::vertical()
                .id_salt(("auxiliary-view", view))
                .auto_shrink([false, false])
                .show(ui, |ui| match view {
                    DockView::Animations => self.animations_panel(ui),
                    DockView::Ghosting => self.ghosting_panel(ui),
                    DockView::Metrics => self.metrics_panel(ui),
                    DockView::Playback => self.playback_panel(ui),
                    DockView::Skins => self.skins_panel(ui),
                    DockView::Weights => self.weights_panel(ui),
                    DockView::Outline => unreachable!(),
                });
        }
    }

    fn animations_panel(&mut self, ui: &mut egui::Ui) {
        let search = ui.add(
            egui::TextEdit::singleline(&mut self.view_tabs.animation_query)
                .hint_text("Search animations"),
        );
        self.record_target("animationsSearch", search.rect);
        let query = self.view_tabs.animation_query.trim().to_lowercase();
        let names: Vec<_> = self
            .session
            .document()
            .animations
            .iter()
            .enumerate()
            .filter(|(_, animation)| animation.name.to_lowercase().contains(&query))
            .map(|(index, animation)| (index, animation.name.clone()))
            .collect();
        if names.is_empty() {
            ui.label("No matching animations.");
        }
        for (index, name) in names {
            let row = ui.selectable_label(self.animation == index, name);
            self.record_target(&format!("animationsRow:{index}"), row.rect);
            if row.secondary_clicked() {
                self.select(TreeItem::Animation(index));
            } else if row.double_clicked() {
                self.begin_rename(TreeItem::Animation(index));
            } else if row.clicked() {
                self.activate_animation(index);
            }
        }
    }

    fn skins_panel(&mut self, ui: &mut egui::Ui) {
        let search = ui.add(
            egui::TextEdit::singleline(&mut self.view_tabs.skin_query).hint_text("Search skins"),
        );
        self.record_target("skinsSearch", search.rect);
        let clear = ui.button("Clear skin");
        self.record_target("skinsClear", clear.rect);
        if clear.clicked() {
            self.activate_skin("default".into());
        }
        let query = self.view_tabs.skin_query.trim().to_lowercase();
        let names: Vec<_> = self
            .session
            .document()
            .skins
            .keys()
            .filter(|name| name.as_str() != "default" && name.to_lowercase().contains(&query))
            .cloned()
            .collect();
        if names.is_empty() {
            ui.label("No matching skins.");
        }
        for name in names {
            let row = ui.selectable_label(self.skin == name, &name);
            self.record_target(&format!("skinsRow:{name}"), row.rect);
            if row.secondary_clicked() {
                self.select(TreeItem::Skin(name));
            } else if row.clicked() {
                self.activate_skin(name);
            }
        }
    }

    fn metrics_panel(&mut self, ui: &mut egui::Ui) {
        let doc = self.session.document();
        for (label, count) in [
            ("Bones", doc.bones.len()),
            ("Constraints", doc.constraints.len()),
            ("Slots", doc.slots.len()),
            (
                "Total attachments",
                doc.skins
                    .values()
                    .flat_map(|skin| skin.values())
                    .map(BTreeMap::len)
                    .sum(),
            ),
        ] {
            ui.label(format!("{label}: {count}"));
        }
        match self.scene_for_view() {
            Ok(scene) => {
                let draws: Vec<_> = scene
                    .attachments
                    .iter()
                    .filter(|draw| !self.hidden_slots.contains(&draw.slot))
                    .collect();
                for (label, count) in [
                    ("Visible attachments", draws.len()),
                    (
                        "Vertices",
                        draws.iter().map(|draw| draw.vertices.len()).sum(),
                    ),
                    (
                        "Triangles",
                        draws.iter().map(|draw| draw.triangles.len() / 3).sum(),
                    ),
                ] {
                    ui.label(format!("{label}: {count}"));
                }
            }
            Err(error) => {
                ui.colored_label(ACCENT, format!("Geometry unavailable: {error}"));
            }
        }
        if !self.setup_mode
            && let Some(animation) = doc.animations.get(self.animation)
        {
            ui.separator();
            ui.label(format!("Scalar tracks: {}", animation.tracks.len()));
            ui.label(format!(
                "Keys: {}",
                animation
                    .tracks
                    .iter()
                    .map(|track| track.keys.len())
                    .sum::<usize>()
            ));
        }
    }

    fn outline_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let fit = ui.small_button("Fit");
            self.record_target("outlineFit", fit.rect);
            if fit.clicked() {
                self.view_tabs.outline_fit = true;
            }
            ui.label(RichText::new("Current pose").small().color(MUTED));
        });
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        self.record_target("outlineViewport", rect);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0., Color32::from_gray(43));
        let mut scene = match self.scene_for_view() {
            Ok(scene) => scene,
            Err(error) => {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("Pose unavailable\n{error}"),
                    FontId::proportional(11.),
                    ACCENT,
                );
                return;
            }
        };
        scene
            .attachments
            .retain(|draw| !self.hidden_slots.contains(&draw.slot));
        scene.bounds = peachy_engine::attachment_bounds(&scene.attachments);
        if self.view_tabs.outline_fit {
            if let Some(bounds) = scene.bounds {
                let zoom = ((rect.width() - 20.) / bounds.width().max(1.))
                    .min((rect.height() - 20.) / bounds.height().max(1.))
                    .clamp(0.01, 6.);
                let center = bounds.center();
                self.view_tabs.outline_zoom = zoom;
                self.view_tabs.outline_pan = vec2(-center[0] * zoom, center[1] * zoom);
            }
            self.view_tabs.outline_fit = false;
        }
        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll != 0. {
                let previous = self.view_tabs.outline_zoom;
                self.view_tabs.outline_zoom = (previous * (scroll * 0.0015).exp()).clamp(0.01, 6.);
                self.view_tabs.outline_pan *= self.view_tabs.outline_zoom / previous;
            }
        }
        if response.dragged_by(egui::PointerButton::Secondary)
            || response.dragged_by(egui::PointerButton::Middle)
        {
            self.view_tabs.outline_pan += response.drag_delta();
        }
        let zoom = self.view_tabs.outline_zoom;
        let origin = rect.center() + self.view_tabs.outline_pan;
        if self
            .check_view_projection(&scene, rect, origin, zoom)
            .is_err()
        {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Use Fit to reset the view.",
                FontId::proportional(11.),
                ACCENT,
            );
            return;
        }
        if response.clicked()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let point = scene_view::view_world_point(origin, zoom, pointer);
            self.pan = vec2(-point[0] * self.zoom, point[1] * self.zoom);
            self.fit = false;
        }
        for draw in &scene.attachments {
            if let Some(texture) = self.textures.get(&draw.image) {
                let mut mesh = egui::Mesh::with_texture(texture.id());
                for (&point, &uv) in draw.vertices.iter().zip(&draw.uvs) {
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: scene_view::screen_point(origin, zoom, point),
                        uv: pos2(uv[0], uv[1]),
                        color: Color32::WHITE,
                    });
                }
                mesh.indices = draw.triangles.clone();
                painter.add(egui::Shape::mesh(mesh));
            }
        }
    }

    pub(super) fn views_diagnostic(&self) -> serde_json::Value {
        serde_json::json!({"open": self.visible_dock_views().iter().map(|view| view.label()).collect::<Vec<_>>(), "storedOpen":DockView::ALL.into_iter().filter(|&view| self.dock_view_open(view)).map(DockView::label).collect::<Vec<_>>(), "active":self.view_tabs.active.label(), "tree":self.show_tree,"dopesheet":self.show_dopesheet && !self.setup_mode,"timeline":self.show_timeline && !self.setup_mode,"storedTimeline":self.show_timeline,"weights":self.show_weights,"minimized":self.hidden_views.is_some(), "outlineZoom": self.view_tabs.outline_zoom, "outlinePan":[self.view_tabs.outline_pan.x,self.view_tabs.outline_pan.y],"unimplemented":UNIMPLEMENTED.map(|(name,_)|name)})
    }
}
