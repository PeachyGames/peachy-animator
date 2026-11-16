//! Workspace pane composition and bounded view-space allocation.
use super::*;

impl Editor {
    pub(super) fn workspace_panels(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
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
        if self.animation_views_open() {
            let height = layout::animation_panel_height(ui);
            let compact = if self.show_timeline {
                layout::compact_timeline_height(ui)
            } else {
                0.
            };
            let tools_scroll = if self.show_weights && self.curves_visible() {
                ui.spacing().scroll.bar_width + ui.spacing().item_spacing.y
            } else {
                0.
            };
            let canvas_header = layout::canvas_header_height(ui);
            let canvas =
                200. + canvas_header + layout::main_tools_height(ui) + tools_scroll + compact;
            let sidebar = if self.show_tree {
                layout::tree_min_height(ui)
                    + layout::tree_footer_height(ui)
                    + 12.
                    + if self.has_dock_views() {
                        64. + self.auxiliary_min_height()
                    } else {
                        96.
                    }
            } else {
                0.
            };
            let minimum = if self.graph_visible() {
                // Preserve Graph's existing plot/control minimum. Its expanded
                // controls plus every stacked view need a separate scroll pass.
                layout::timeline_panel_height(ui) + 28.
            } else {
                layout::animation_min_height(ui)
            };
            let maximum = (ui.available_height() - canvas.max(sidebar)).max(minimum);
            layout::adaptive_bottom_panel(
                ui,
                "animation-views",
                layout::PanelHeightBudget {
                    default: height,
                    minimum,
                    maximum,
                },
                egui::Frame::new().fill(PANEL).inner_margin(6),
                |ui| self.animation_views(ui),
            );
        }
        if self.show_tree || self.has_dock_views() {
            egui::Panel::right("tree")
                .default_size(310.0)
                .min_size(260.0)
                .resizable(true)
                .frame(egui::Frame::new().fill(PANEL).inner_margin(6))
                .show(ui, |ui| {
                    if self.show_tree {
                        egui::Panel::bottom("tree-creation-footer")
                            .resizable(false)
                            .frame(egui::Frame::new().fill(PANEL).inner_margin(3))
                            .show(ui, |ui| self.tree_creation_footer(ui));
                        let auxiliary = self.has_dock_views();
                        let properties_min = if auxiliary { 64. } else { 96. };
                        let properties_max = (ui.available_height()
                            - layout::tree_min_height(ui)
                            - if auxiliary {
                                self.auxiliary_min_height()
                            } else {
                                0.
                            })
                        .max(properties_min);
                        let default = (ui.available_height() * if auxiliary { 0.35 } else { 0.45 })
                            .clamp(properties_min, 350.);
                        layout::adaptive_bottom_panel(
                            ui,
                            "item-properties",
                            layout::PanelHeightBudget {
                                default,
                                minimum: properties_min,
                                maximum: properties_max,
                            },
                            egui::Frame::new().fill(PANEL).inner_margin(6),
                            |ui| {
                                // Keep fields and sliders out of the vertical
                                // scrollbar's hit area, including a coalesced
                                // press and move near a control's right edge.
                                ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
                                egui::ScrollArea::vertical()
                                    .id_salt("properties-scroll")
                                    .min_scrolled_height(0.)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| self.inspector(ui));
                            },
                        );
                    }
                    if self.show_tree && self.has_dock_views() {
                        let minimum = self.auxiliary_min_height();
                        let default = (ui.available_height() * 0.65).clamp(minimum, 260.);
                        let maximum =
                            (ui.available_height() - layout::tree_min_height(ui)).max(minimum);
                        layout::adaptive_bottom_panel(
                            ui,
                            "auxiliary-view-panel",
                            layout::PanelHeightBudget {
                                default,
                                minimum,
                                maximum,
                            },
                            egui::Frame::new().fill(PANEL).inner_margin(6),
                            |ui| self.auxiliary_dock(ui),
                        );
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
        if self.curves_visible() {
            // With both vertical views open, reserve the Weights minimum and a
            // usable canvas before restoring any persisted Curves width.
            let max_width = if self.show_weights {
                (ui.available_width() - 230. - 320.).clamp(230., 400.)
            } else {
                400.
            };
            egui::Panel::right("curves-view")
                .default_size(275_f32.min(max_width))
                .min_size(230.)
                .max_size(max_width)
                .resizable(true)
                .frame(egui::Frame::new().fill(PANEL).inner_margin(6))
                .show(ui, |ui| self.curves_panel(ui));
        }
        if self.show_weights {
            let viewport_min = if self.curves_visible() { 320. } else { 400. };
            let max_width = (ui.available_width() - viewport_min).clamp(230., 400.);
            egui::Panel::right("weights-pane")
                .default_size(290_f32.min(max_width))
                .min_size(230.)
                .max_size(max_width)
                .resizable(true)
                .frame(egui::Frame::new().fill(PANEL).inner_margin(6))
                .show(ui, |ui| self.weights_panel(ui));
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_gray(43))
                    .inner_margin(0),
            )
            .show(ui, |ui| {
                let narrow_tools = self.show_weights && self.curves_visible();
                let height = layout::main_tools_height(ui)
                    + if narrow_tools {
                        ui.spacing().scroll.bar_width + ui.spacing().item_spacing.y
                    } else {
                        0.
                    };
                egui::Panel::bottom("main-tools")
                    .exact_size(height)
                    .frame(egui::Frame::new().fill(PANEL).inner_margin(7))
                    .show(ui, |ui| {
                        if narrow_tools {
                            egui::ScrollArea::horizontal()
                                .id_salt("main-tools-horizontal")
                                .min_scrolled_width(0.)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    // A horizontal overflow surface must keep
                                    // each toolbar column at its natural width.
                                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                                    self.main_tools(ui);
                                });
                        } else {
                            self.main_tools(ui);
                        }
                    });
                self.canvas(ui);
            });
    }
}
