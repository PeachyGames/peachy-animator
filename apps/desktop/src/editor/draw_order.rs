//! Global Animate slot ordering; setup slot identities never move.
use super::*;
use peachy_engine::{DrawOrderPlacement, move_draw_order_slot, place_draw_order_slot};

#[cfg(test)]
#[path = "draw_order_tests.rs"]
mod tests;

#[derive(Default)]
pub(super) struct DrawOrderView {
    drag: Option<DrawOrderDrag>,
    blocked: bool,
    #[cfg(test)]
    row_ids: BTreeMap<usize, egui::Id>,
}

struct DrawOrderDrag {
    slot: usize,
    order: Vec<usize>,
    expected_order: Vec<usize>,
    expected_keys: Vec<DrawOrderKey>,
    rows: Vec<(usize, egui::Rect)>,
    drop_rect: egui::Rect,
    animation: usize,
    time: f32,
    skin: String,
    selection: Option<TreeItem>,
    filters: tree_model::Filters,
    query: String,
    collapsed: BTreeSet<String>,
    auto_key: bool,
    tool: Tool,
    axes: Axes,
    pose_generation: u64,
    finish_pending: bool,
}

impl Editor {
    #[cfg(test)]
    pub(super) fn reset_draw_order_row_ids(&mut self) {
        self.draw_order.row_ids.clear();
    }

    #[cfg(test)]
    pub(super) fn record_draw_order_row_id(&mut self, slot: usize, id: egui::Id) {
        self.draw_order.row_ids.insert(slot, id);
    }

    pub(super) fn current_draw_order(&self) -> Result<Vec<usize>, String> {
        self.session.draw_order(self.weights_edit_target())
    }

    pub(super) fn draw_order_input_available(&self, ctx: &egui::Context) -> bool {
        !self.setup_mode
            && self.active_animation().is_some()
            && self.slot_authoring_input_available(ctx)
    }

    pub(super) fn draw_order_edit_active(&self) -> bool {
        self.draw_order.drag.is_some()
    }

    pub(super) fn cancel_draw_order_edit(&mut self) {
        if self.draw_order.drag.take().is_some() {
            self.session.cancel_transaction();
            self.draw_order.blocked = true;
        }
    }

    pub(super) fn reconcile_draw_order_input(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.pointer.primary_down()) {
            self.draw_order.blocked = false;
        }
        if self.draw_order.drag.as_ref().is_some_and(|drag| {
            !self.draw_order_input_available(ctx)
                || !self.show_tree
                || self.hidden_views.is_some()
                || ctx.text_edit_focused()
                || self.animation != drag.animation
                || self.time.to_bits() != drag.time.to_bits()
                || self.skin != drag.skin
                || self.selection != drag.selection
                || self.auto_key != drag.auto_key
                || self.tool != drag.tool
                || self.axes != drag.axes
                || self.pose_view_generation != drag.pose_generation
                || self.tree.filters != drag.filters
                || self.tree.query != drag.query
                || self.tree.collapsed != drag.collapsed
                || self
                    .session
                    .document()
                    .animations
                    .get(drag.animation)
                    .is_none_or(|clip| clip.draw_order != drag.expected_keys)
                || self
                    .current_draw_order()
                    .map_or(true, |order| order != drag.expected_order)
        }) {
            self.cancel_draw_order_edit();
        }
    }

    pub(super) fn begin_draw_order_drag(
        &mut self,
        slot: usize,
        rows: &[(usize, egui::Rect)],
        drop_rect: egui::Rect,
        ctx: &egui::Context,
    ) {
        if self.draw_order.blocked
            || !self.draw_order_input_available(ctx)
            || ctx.text_edit_focused()
        {
            return;
        }
        let Ok(order) = self.current_draw_order() else {
            return;
        };
        if !order.contains(&slot)
            || rows.is_empty()
            || !drop_rect.is_finite()
            || !drop_rect.is_positive()
        {
            return;
        }
        self.select(TreeItem::DrawOrderSlot { slot });
        self.tree.reveal = None;
        self.session.commit_transaction();
        self.session.begin_transaction();
        self.playing = false;
        self.draw_order.drag = Some(DrawOrderDrag {
            slot,
            expected_order: order.clone(),
            order,
            expected_keys: self.session.document().animations[self.animation]
                .draw_order
                .clone(),
            rows: rows.to_vec(),
            drop_rect,
            animation: self.animation,
            time: self.time,
            skin: self.skin.clone(),
            selection: self.selection.clone(),
            filters: self.tree.filters,
            query: self.tree.query.clone(),
            collapsed: self.tree.collapsed.clone(),
            auto_key: self.auto_key,
            tool: self.tool,
            axes: self.axes,
            pose_generation: self.pose_view_generation,
            finish_pending: false,
        });
    }

    pub(super) fn update_draw_order_pointer(
        &mut self,
        ctx: &egui::Context,
        rows: &[(usize, egui::Rect)],
        drop_rect: Option<egui::Rect>,
    ) {
        self.reconcile_draw_order_input(ctx);
        if self.draw_order.drag.as_ref().is_some_and(|drag| {
            drop_rect != Some(drag.drop_rect)
                || drag.rows.len() != rows.len()
                || drag
                    .rows
                    .iter()
                    .zip(rows)
                    .any(|((_, a), (_, b))| a.min != b.min || a.height() != b.height())
        }) {
            self.cancel_draw_order_edit();
        }
        let release = ctx.input(|input| key_drag::primary_pointer_event(&input.events, false));
        let pointer = release
            .map(|(point, _)| point)
            .or_else(|| ctx.input(|input| input.pointer.interact_pos()));
        let Some(mut drag) = self.draw_order.drag.take() else {
            return;
        };
        let hit = pointer
            .filter(|point| point.is_finite())
            .and_then(|pointer| {
                drag.rows.iter().find_map(|(anchor, rect)| {
                    let band = egui::Rect::from_min_max(
                        pos2(drag.drop_rect.left(), rect.top().max(drag.drop_rect.top())),
                        pos2(
                            drag.drop_rect.right(),
                            rect.bottom().min(drag.drop_rect.bottom()),
                        ),
                    );
                    (band.is_positive() && band.contains(pointer))
                        .then_some((pointer, *anchor, *rect))
                })
            });
        if (release.is_some() && hit.is_none())
            || (pointer.is_none() && !ctx.input(|input| input.pointer.primary_down()))
        {
            self.session.cancel_transaction();
            self.draw_order.blocked = true;
            self.pose_key_request = None;
            return;
        }
        if let Some((pointer, anchor, rect)) = hit {
            let placement = if pointer.y < rect.center().y {
                DrawOrderPlacement::InFront
            } else {
                DrawOrderPlacement::Behind
            };
            let result = place_draw_order_slot(&drag.order, drag.slot, anchor, placement).and_then(
                |order| {
                    if order != drag.expected_order {
                        self.session.preview_edit_draw_order(
                            EditTarget::Animate {
                                animation: drag.animation,
                                time: drag.time,
                            },
                            order,
                            drag.auto_key,
                        )?;
                    }
                    self.session.draw_order(EditTarget::Animate {
                        animation: drag.animation,
                        time: drag.time,
                    })
                },
            );
            match result {
                Ok(order) => {
                    drag.expected_order = order;
                    drag.expected_keys = self.session.document().animations[drag.animation]
                        .draw_order
                        .clone();
                }
                Err(error) => {
                    self.session.cancel_transaction();
                    self.draw_order.blocked = true;
                    self.pose_key_request = None;
                    self.status = error;
                    return;
                }
            }
        }
        drag.finish_pending |= release.is_some();
        self.draw_order.drag = Some(drag);
    }

    pub(super) fn settle_draw_order_input(&mut self, ctx: &egui::Context) {
        self.reconcile_draw_order_input(ctx);
        if !ctx.input(|input| input.pointer.primary_down())
            && self
                .draw_order
                .drag
                .as_ref()
                .is_some_and(|drag| drag.finish_pending)
        {
            self.draw_order.drag = None;
            self.session.commit_transaction();
        }
    }

    pub(super) fn move_selected_draw_order(&mut self, delta: i32) {
        if self.setup_mode {
            return;
        }
        let Some(slot) = self
            .selection
            .as_ref()
            .and_then(|item| item.slot(self.session.document()))
        else {
            return;
        };
        match self
            .current_draw_order()
            .and_then(|order| move_draw_order_slot(&order, slot, delta))
        {
            Ok(order) => self.edit_draw_order(order),
            Err(error) => self.status = error,
        }
    }

    fn edit_draw_order(&mut self, order: Vec<usize>) {
        if self.setup_mode
            || self
                .current_draw_order()
                .is_ok_and(|current| current == order)
        {
            return;
        }
        self.cancel_gesture();
        self.playing = false;
        match self
            .session
            .edit_draw_order(self.weights_edit_target(), order, self.auto_key)
        {
            Ok(()) => {
                self.status = if self.auto_key {
                    "Keyed draw order"
                } else {
                    "Draw order edited; press K to key"
                }
                .into()
            }
            Err(error) => self.status = error,
        }
    }

    pub(super) fn draw_order_state(&self) -> Result<KeyState, String> {
        self.session.pose_edits().draw_order_key_state(
            self.session.document(),
            self.animation,
            self.time,
        )
    }

    pub(super) fn key_draw_order(&mut self) {
        self.request_pose_key(pose_key::PoseKeyAction::DrawOrder);
    }

    pub(super) fn seal_draw_order_for_key(&mut self) {
        if self.draw_order.drag.take().is_some() {
            self.draw_order.blocked = true;
        }
    }

    pub(super) fn delete_draw_order_key(&mut self) {
        if self.setup_mode {
            return;
        }
        match peachy_engine::draw_order_key_time(self.session.document(), self.animation, self.time)
        {
            Ok(Some(time)) => {
                self.cancel_gesture();
                self.dispatch(Command::DeleteAnimationKeys {
                    animation: self.animation,
                    keys: vec![peachy_engine::AnimationKey::DrawOrder { time }],
                });
            }
            Ok(None) => {}
            Err(error) => self.status = error,
        }
    }

    pub(super) fn draw_order_properties(&mut self, ui: &mut egui::Ui) {
        if self.setup_mode {
            ui.label("Draw order is authored in Animate.");
            return;
        }
        if let Some(TreeItem::DrawOrderSlot { slot }) = self.selection {
            ui.label(format!(
                "Slot: {}",
                self.session.document().slots[slot].name
            ));
        }
        ui.label("Drag slots in the Tree, or use = / -. Hold Shift to move five places.");
        let footer_space = (ui.clip_rect().bottom() - ui.cursor().top() - 28.).max(0.);
        if footer_space.is_finite() {
            ui.add_space(footer_space);
        }
        let setup: Vec<_> = (0..self.session.document().slots.len()).collect();
        let enabled = self.draw_order_input_available(ui.ctx())
            && self.current_draw_order().is_ok_and(|order| order != setup);
        let reset = ui.add_enabled(enabled, egui::Button::new("Reset"));
        self.record_target("drawOrderReset", reset.rect);
        if reset.clicked() {
            self.edit_draw_order(setup);
        }
    }

    pub(super) fn draw_order_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        if self.draw_order_edit_active() {
            self.file_history_shortcuts(ctx);
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::K)) {
                self.key_edited(None);
            }
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                self.cancel_draw_order_edit();
            }
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F9)) {
                self.toggle_views();
            }
            return true;
        }
        if !self.draw_order_input_available(ctx) || ctx.text_edit_focused() {
            return false;
        }
        let Some(TreeItem::DrawOrderSlot { .. } | TreeItem::Slot(_)) = self.selection else {
            return false;
        };
        let delta = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if !modifiers.command && !modifiers.ctrl && !modifiers.alt => {
                    let step = if modifiers.shift { 5 } else { 1 };
                    match key {
                        egui::Key::Equals | egui::Key::Plus => Some(step),
                        egui::Key::Minus => Some(-step),
                        _ => None,
                    }
                }
                _ => None,
            })
        });
        if let Some(delta) = delta {
            self.move_selected_draw_order(delta);
            return true;
        }
        false
    }

    pub(super) fn set_auto_key_enabled(&mut self, enabled: bool) {
        if enabled == self.auto_key {
            return;
        }
        self.cancel_gesture();
        if enabled
            && !self.setup_mode
            && !self.session.pose_edits().is_empty()
            && let Err(error) = self.session.key_edited(self.animation, self.time, None)
        {
            self.status = format!("Cannot enable Auto Key: {error}");
            return;
        }
        self.auto_key = enabled;
        if enabled {
            self.playing = false;
        }
    }

    pub(super) fn draw_order_diagnostic(&self) -> serde_json::Value {
        serde_json::json!({"order":self.current_draw_order().ok(),"state":self.draw_order_state().ok(),"active":self.draw_order_edit_active(),"blocked":self.draw_order.blocked,"slot":self.draw_order.drag.as_ref().map(|drag|drag.slot),"dropArea":self.draw_order.drag.as_ref().map(|drag|[drag.drop_rect.left(),drag.drop_rect.top(),drag.drop_rect.right(),drag.drop_rect.bottom()])})
    }
}
