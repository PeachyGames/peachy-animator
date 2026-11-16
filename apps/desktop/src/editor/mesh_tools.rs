//! Session-only soft-selection controls; geometry and edit commands live in engine.
use super::*;
use peachy_engine::{MeshSoftSelection, MeshTranslation};

#[derive(Default)]
pub(super) struct MeshToolsView {
    pub settings: MeshSoftSelection,
    pub show_radius: bool,
}

impl Editor {
    pub(super) fn mesh_translation(
        &self,
        key: &AttachmentRef,
        vertex: usize,
    ) -> Result<MeshTranslation, String> {
        if self.editable_mesh_position(key, vertex).is_none() {
            return Err("Soft selection requires an editable unweighted Setup mesh".into());
        }
        let doc = self.session.document();
        let Some(Attachment::Mesh(mesh)) = doc.attachment(key) else {
            return Err("Select a mesh attachment".into());
        };
        let slot = doc
            .slots
            .iter()
            .find(|slot| slot.name == key.slot)
            .ok_or("Unknown mesh slot")?;
        let scene = self.scene_for_view()?;
        MeshTranslation::new(
            mesh,
            scene.world[slot.bone],
            vertex,
            self.mesh_tools.settings,
        )
    }

    pub(super) fn cancel_mesh_drag(&mut self) {
        let setup_drag = self.mesh_drag.take().is_some();
        let deform_drag = self.deform_drag.take().is_some();
        if setup_drag || deform_drag {
            self.session.cancel_transaction();
            self.mesh_blocked = true;
        }
    }

    pub(super) fn reconcile_mesh_input(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.pointer.primary_down()) {
            self.mesh_blocked = false;
        }
        if !ctx.input(|input| input.focused)
            || self.error.is_some()
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
            || egui::Popup::is_any_open(ctx)
            || self.mesh_drag.as_ref().is_some_and(|drag| {
                self.tool != Tool::Mesh
                    || self.selected_attachment() != Some(&drag.attachment)
                    || self.mesh_vertex != drag.vertex
                    || self
                        .editable_mesh_position(&drag.attachment, drag.vertex)
                        .is_none()
            })
            || self.deform_drag.as_ref().is_some_and(|drag| {
                self.tool != Tool::Mesh
                    || self.selected_attachment() != Some(&drag.attachment)
                    || self.mesh_vertex != drag.vertex
                    || self.setup_mode
                    || !matches!(drag.target, EditTarget::Animate { animation, time }
                        if animation == self.animation && time == self.time)
                    || self
                        .deform_world_position(&drag.attachment, drag.vertex)
                        .is_none()
            })
        {
            self.cancel_mesh_drag();
        }
    }

    pub(super) fn mesh_tools_panel(&mut self, ui: &mut egui::Ui) {
        self.record_target("meshSoftPanelClip", ui.clip_rect());
        let size = ui
            .add_enabled(
                self.setup_mode,
                egui::Slider::new(&mut self.mesh_tools.settings.size, 0.0..=1000.0)
                    .text("Size")
                    .suffix(" px")
                    .max_decimals(2),
            )
            .on_hover_text(
                "World-space soft-selection radius. Zero moves only the selected vertex.",
            );
        self.record_target("meshSoftSize", size.rect);
        let mut feather = self.mesh_tools.settings.feather * 100.;
        let feather_response = ui
            .add_enabled(
                self.setup_mode,
                egui::Slider::new(&mut feather, 0.0..=100.0)
                    .text("Feather")
                    .suffix("%")
                    .max_decimals(1),
            )
            .on_hover_text(
                "Percentage of the radius used for linear falloff. Zero gives a hard edge.",
            );
        self.record_target("meshSoftFeather", feather_response.rect);
        let hull = ui
            .add_enabled(
                self.setup_mode,
                egui::Checkbox::new(&mut self.mesh_tools.settings.hull_vertices, "Hull vertices"),
            )
            .on_hover_text(
                "Affect neighboring hull vertices. The explicitly selected vertex always moves.",
            );
        self.record_target("meshSoftHull", hull.rect);
        self.mesh_tools.show_radius = size.hovered()
            || size.dragged()
            || feather_response.hovered()
            || feather_response.dragged();
        if size.changed() || feather_response.changed() || hull.changed() {
            self.cancel_gesture();
            self.mesh_tools.settings.feather = feather / 100.;
        }
        ui.separator();
        let key = self.selected_attachment().cloned();
        let mesh = key
            .as_ref()
            .and_then(|key| self.session.document().attachment(key));
        if let Some(Attachment::Mesh(mesh)) = mesh {
            let weighted = mesh.is_weighted();
            ui.label(format!(
                "{} · vertex {}",
                key.as_ref().unwrap().placeholder,
                self.mesh_vertex
            ));
            let edit = ui
                .add_enabled(
                    key.as_ref()
                        .is_some_and(|key| self.attachment_editable(key)),
                    egui::Button::new("Vertices"),
                )
                .on_disabled_hover_text("Activate this attachment's skin to edit its vertices.")
                .on_hover_text(if !self.setup_mode {
                    "Pose one vertex in Animate. Soft selection is available in Setup."
                } else if weighted {
                    "Inspect weighted vertices; use Weights for binding edits."
                } else {
                    "Enter Setup vertex editing."
                });
            self.record_target("meshSoftVertices", edit.rect);
            if edit.clicked() {
                self.enter_mesh_tool();
            }
            if !self.setup_mode {
                ui.label(
                    RichText::new(
                        "Animate moves one vertex. Switch to Setup to use soft selection.",
                    )
                    .small()
                    .color(MUTED),
                );
            } else if weighted {
                ui.label(
                    RichText::new(
                        "Weighted soft selection is not available. Use Weights to edit bindings.",
                    )
                    .small()
                    .color(MUTED),
                );
            } else if let Some(key) = key.as_ref() {
                match self.mesh_translation(key, self.mesh_vertex) {
                    Ok(translation) => {
                        let weights = self
                            .mesh_drag
                            .as_ref()
                            .filter(|drag| &drag.attachment == key)
                            .map_or_else(
                                || translation.weights(),
                                |drag| drag.translation.weights(),
                            );
                        let affected = weights.iter().filter(|&&weight| weight > 0.).count();
                        ui.label(
                            RichText::new(format!("{affected} affected vertices"))
                                .small()
                                .color(MUTED),
                        );
                    }
                    Err(error) => {
                        ui.label(RichText::new(error).small().color(ACCENT));
                    }
                }
            }
        } else {
            ui.label("Select a mesh attachment, then use Vertices.");
        }
        ui.label(
            RichText::new("Size affects Setup vertex drags and arrow-key nudges.")
                .small()
                .color(MUTED),
        );
    }

    pub(super) fn mesh_tools_diagnostic(&self) -> serde_json::Value {
        let current = self
            .selected_attachment()
            .ok_or_else(|| "Select a mesh attachment".to_string())
            .and_then(|key| self.mesh_translation(key, self.mesh_vertex));
        let settings = self.mesh_tools.settings;
        serde_json::json!({
            "size":settings.size, "feather":settings.feather,
            "hullVertices":settings.hull_vertices, "radiusVisible":self.mesh_tools.show_radius,
            "weights":self.mesh_drag.as_ref().map(|drag|drag.translation.weights())
                .or_else(||current.as_ref().ok().map(|translation|translation.weights())),
            "unavailable":current.err(), "dragging":self.mesh_drag.is_some(),
            "deformDragging":self.deform_drag.is_some(), "blocked":self.mesh_blocked
        })
    }
}

#[cfg(test)]
#[path = "mesh_tools_tests.rs"]
mod tests;
