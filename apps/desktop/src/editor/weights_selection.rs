//! Transient vertex selection; Direct arithmetic and document history stay in Session.
use super::*;

#[cfg(test)]
#[path = "weights_selection_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PointerMode {
    Marquee,
    Relative,
}

pub(super) struct WeightsPointer {
    mesh: AttachmentRef,
    vertices: Vec<usize>,
    bone: Option<usize>,
    skin: String,
    setup: bool,
    animation: usize,
    time: f32,
    viewport: egui::Rect,
    origin: Pos2,
    zoom: f32,
    start: Pos2,
    last: Pos2,
    handles: Vec<(usize, Pos2)>,
    mode: PointerMode,
    additive: bool,
    started: bool,
}

pub(super) struct WeightsVertexPress {
    mesh: AttachmentRef,
    vertex: usize,
    additive: bool,
}

impl Editor {
    pub(super) fn weights_viewport_id() -> egui::Id {
        egui::Id::new("weights-viewport")
    }

    pub(super) fn weights_input_available(&self, ctx: &egui::Context) -> bool {
        ctx.input(|input| input.focused)
            && self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
            && self.export_dialog.is_none()
            && !self.slot_color_dialog_open()
            && !egui::Popup::is_any_open(ctx)
    }

    pub(super) fn cancel_weights_pointer(&mut self) {
        if self.weights.vertex_press.take().is_some() {
            self.weights.pointer_blocked = true;
        }
        if let Some(drag) = self.weights.pointer.take() {
            self.weights.vertices = drag.vertices;
            self.weights.bone = drag.bone;
            self.weights.pointer_blocked = true;
        }
    }

    pub(super) fn reconcile_weights_pointer(&mut self, ctx: &egui::Context) {
        // A canceled gesture still owns its release frame. A later primary
        // press is a fresh gesture, even without an intervening idle frame.
        if ctx.input(|input| {
            input.pointer.button_pressed(egui::PointerButton::Primary)
                || (!input.pointer.primary_down()
                    && !input.pointer.button_released(egui::PointerButton::Primary))
        }) {
            self.weights.pointer_blocked = false;
        }
        let invalid = self.weights.pointer.as_ref().is_some_and(|drag| {
            !self.weights_input_available(ctx)
                || !self.show_weights
                || self.tool != Tool::Weights
                || self.weights.bind.is_some()
                || self.weights.mesh.as_ref() != Some(&drag.mesh)
                || !self.weights_mesh_active(&drag.mesh)
                || self.skin != drag.skin
                || self.setup_mode != drag.setup
                || self.animation != drag.animation
                || self.time != drag.time
                || !matches!(self.session.document().attachment(&drag.mesh), Some(Attachment::Mesh(mesh)) if mesh.vertex_count() == drag.handles.len())
        });
        if invalid {
            self.cancel_weights_pointer();
            self.cancel_weight_edit();
        }
    }

    fn pick_weights_vertex(&mut self, key: AttachmentRef, vertex: usize, additive: bool) {
        self.cancel_gesture();
        self.select(TreeItem::Attachment(key));
        if additive {
            match self.weights.vertices.binary_search(&vertex) {
                Ok(index) => {
                    self.weights.vertices.remove(index);
                }
                Err(index) => self.weights.vertices.insert(index, vertex),
            }
        } else {
            self.weights.vertices = vec![vertex];
        }
    }

    pub(super) fn weights_viewport_input(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        nearest: Option<(AttachmentRef, usize, f32)>,
        picked: Option<TreeItem>,
        projection: (egui::Rect, Pos2, f32),
    ) {
        let (viewport, origin, zoom) = projection;
        let _ = ui.interact(
            viewport,
            Self::weights_viewport_id(),
            Sense::focusable_noninteractive(),
        );
        if !self.show_weights
            || !self.weights_input_available(ui.ctx())
            || self.weights.pointer_blocked
        {
            return;
        }
        let additive = ui.input(|input| {
            input
                .events
                .iter()
                .rev()
                .find_map(|event| match event {
                    egui::Event::PointerButton {
                        button: egui::PointerButton::Primary,
                        modifiers,
                        ..
                    } => Some(modifiers.command),
                    _ => None,
                })
                .unwrap_or(input.modifiers.command)
        });
        if self.weights.vertex_press.is_none()
            && self.weights.pointer.is_none()
            && ui.input(|input| input.pointer.button_pressed(egui::PointerButton::Primary))
            && (response.is_pointer_button_down_on() || response.clicked())
            && let Some((mesh, vertex, _)) = nearest.as_ref()
        {
            let additive = ui.input(|input| {
                input
                    .events
                    .iter()
                    .find_map(|event| match event {
                        egui::Event::PointerButton {
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers,
                            ..
                        } => Some(modifiers.command),
                        _ => None,
                    })
                    .unwrap_or(additive)
            });
            self.weights.vertex_press = Some(WeightsVertexPress {
                mesh: mesh.clone(),
                vertex: *vertex,
                additive,
            });
        }
        if self.weights.pointer.is_none()
            && (response.clicked() || response.drag_started_by(egui::PointerButton::Primary))
            && let Some((key, vertex, _)) = nearest.as_ref()
        {
            let press = self
                .weights
                .vertex_press
                .take()
                .unwrap_or_else(|| WeightsVertexPress {
                    mesh: key.clone(),
                    vertex: *vertex,
                    additive,
                });
            self.pick_weights_vertex(press.mesh, press.vertex, press.additive);
            ui.memory_mut(|memory| memory.request_focus(Self::weights_viewport_id()));
            return;
        }
        if ui.input(|input| input.pointer.button_released(egui::PointerButton::Primary)) {
            self.weights.vertex_press = None;
        }
        if self.weights.pointer.is_none()
            && nearest.is_none()
            && response.is_pointer_button_down_on()
            && ui.input(|input| input.pointer.button_pressed(egui::PointerButton::Primary))
            && let Some(start) = ui.input(|input| input.pointer.press_origin())
            && viewport.contains(start)
            && !self.weights_handles.is_empty()
            && let Some(mesh) = self
                .weights
                .mesh
                .clone()
                .filter(|mesh| self.weights_mesh_active(mesh))
        {
            let mode = if additive || self.weights.vertices.is_empty() {
                PointerMode::Marquee
            } else {
                PointerMode::Relative
            };
            self.cancel_gesture();
            self.weights.pointer = Some(WeightsPointer {
                mesh,
                vertices: self.weights.vertices.clone(),
                bone: self.weights.bone,
                skin: self.skin.clone(),
                setup: self.setup_mode,
                animation: self.animation,
                time: self.time,
                viewport,
                origin,
                zoom,
                start,
                last: start,
                handles: self
                    .weights_handles
                    .iter()
                    .enumerate()
                    .map(|(index, point)| (index, pos2(point[0], point[1])))
                    .collect(),
                mode,
                additive,
                started: false,
            });
            ui.memory_mut(|memory| memory.request_focus(Self::weights_viewport_id()));
        }
        if let Some(mut drag) = self.weights.pointer.take() {
            if drag.viewport != viewport || drag.origin != origin || drag.zoom != zoom {
                self.weights.pointer = Some(drag);
                self.cancel_weights_pointer();
                self.cancel_weight_edit();
                return;
            }
            let pointer = ui
                .input(|input| input.pointer.latest_pos())
                .unwrap_or(drag.last);
            if !drag.started && pointer.distance(drag.start) > 10. {
                if drag.mode == PointerMode::Relative
                    && let Err(error) = self.begin_weight_viewport_edit()
                {
                    self.status = error;
                    self.weights.pointer_blocked = true;
                    return;
                }
                drag.started = true;
                // The first threshold-crossing move establishes the editing
                // origin; subsequent moves advance the engine gesture.
                drag.last = pointer;
            }
            if drag.started {
                if drag.mode == PointerMode::Marquee {
                    let selection =
                        egui::Rect::from_two_pos(drag.start, pointer).intersect(viewport);
                    let mut vertices = if drag.additive {
                        drag.vertices.clone()
                    } else {
                        Vec::new()
                    };
                    vertices.extend(drag.handles.iter().filter_map(|&(index, point)| {
                        (point.is_finite() && viewport.contains(point) && selection.contains(point))
                            .then_some(index)
                    }));
                    vertices.sort_unstable();
                    vertices.dedup();
                    self.weights.vertices = vertices;
                    ui.painter_at(viewport).rect_stroke(
                        selection,
                        0.,
                        Stroke::new(1., ACCENT),
                        egui::StrokeKind::Inside,
                    );
                } else if pointer.y != drag.last.y {
                    self.preview_weight_viewport_delta(drag.last.y - pointer.y);
                }
            }
            drag.last = pointer;
            if ui.input(|input| input.pointer.primary_down()) {
                self.weights.pointer = Some(drag);
            } else if drag.started {
                if drag.mode == PointerMode::Relative {
                    self.finish_weight_viewport_edit();
                }
            } else if !drag.additive {
                self.weights.vertices.clear();
                self.weights.bone = None;
            }
            return;
        }
        if response.clicked() {
            self.weights.vertex_press = None;
            self.cancel_gesture();
            if let Some(item) = picked {
                self.select(item);
            } else if !additive {
                self.weights.vertices.clear();
                self.weights.bone = None;
            }
            ui.memory_mut(|memory| memory.request_focus(Self::weights_viewport_id()));
        }
    }

    pub(super) fn select_all_weights_vertices(&mut self) {
        self.cancel_gesture();
        self.reconcile_weights();
        if let Some(Attachment::Mesh(mesh)) = self
            .weights
            .mesh
            .as_ref()
            .and_then(|key| self.session.document().attachment(key))
        {
            self.weights.vertices = (0..mesh.vertex_count()).collect();
            self.status = format!("Selected {} vertices", self.weights.vertices.len());
        }
    }

    pub(super) fn weights_pointer_diagnostic(&self) -> serde_json::Value {
        self.weights.pointer.as_ref().map_or(serde_json::Value::Null, |drag| {
            serde_json::json!({"mesh":drag.mesh,"verticesBefore":drag.vertices,"started":drag.started,
                "mode":match drag.mode { PointerMode::Marquee=>"marquee",PointerMode::Relative=>"relative" },
                "start":[drag.start.x,drag.start.y],"last":[drag.last.x,drag.last.y],
                "blocked":self.weights.pointer_blocked})
        })
    }
}
