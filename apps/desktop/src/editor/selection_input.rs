//! Selection modifiers belong to the pointer press, including fast Command release.
use super::*;

#[derive(Default)]
pub(super) struct SelectionInput {
    press: Option<(egui::Id, TreeItem, egui::Modifiers)>,
    viewport: Option<ViewportPress>,
}

#[derive(Clone)]
pub(super) enum ViewportPressTarget {
    Empty,
    Item(TreeItem),
    Gizmo(usize),
}

#[derive(Clone)]
pub(super) struct ViewportPress {
    pub target: ViewportPressTarget,
    pub world: [f32; 2],
    pub modifiers: egui::Modifiers,
    position: Pos2,
    selection: TreeSelection,
    setup: bool,
    animation: usize,
    time: f32,
    skin: String,
    tool: Tool,
    axes: Axes,
    auto_key: bool,
    pose_generation: u64,
    document_generation: u64,
    rect: egui::Rect,
    pan: Vec2,
    zoom: f32,
}

impl ViewportPress {
    fn matches(&self, editor: &Editor, rect: egui::Rect) -> bool {
        self.selection == editor.selection
            && self.setup == editor.setup_mode
            && self.animation == editor.animation
            && self.time == editor.time
            && self.skin == editor.skin
            && self.tool == editor.tool
            && self.axes == editor.axes
            && self.auto_key == editor.auto_key
            && self.pose_generation == editor.pose_view_generation
            && self.document_generation == editor.session.document_generation()
            && self.rect == rect
            && self.pan == editor.pan
            && self.zoom == editor.zoom
            && match &self.target {
                ViewportPressTarget::Item(item) => item.exists(editor.session.document()),
                _ => true,
            }
    }
}
impl SelectionInput {
    pub fn clear(&mut self) {
        self.press = None;
        self.viewport = None;
    }
    pub fn observe(&mut self, response: &egui::Response, item: &TreeItem) {
        if !response.ctx.input(|input| input.focused) {
            self.clear();
        } else if (response.is_pointer_button_down_on() || response.clicked())
            && let Some((_, modifiers)) = response
                .ctx
                .input(|input| key_drag::primary_pointer_event(&input.events, true))
        {
            self.press = Some((response.id, item.clone(), modifiers));
        }
    }
    pub fn modifiers(&mut self, response: &egui::Response, item: &TreeItem) -> egui::Modifiers {
        let frozen = self
            .press
            .as_ref()
            .filter(|(id, target, _)| *id == response.id && target == item)
            .map(|(_, _, modifiers)| *modifiers);
        let modifiers = frozen.unwrap_or_else(|| {
            response.ctx.input(|input| {
                key_drag::primary_pointer_event(&input.events, true)
                    .or_else(|| key_drag::primary_pointer_event(&input.events, false))
                    .map_or(input.modifiers, |(_, modifiers)| modifiers)
            })
        });
        if response.clicked() || response.drag_stopped() {
            self.clear();
        }
        modifiers
    }
}

impl Editor {
    pub(super) fn observe_viewport_press(
        &mut self,
        response: &egui::Response,
        rect: egui::Rect,
        picked: Option<TreeItem>,
        axis: Option<usize>,
        world: Option<[f32; 2]>,
    ) {
        if let Some((position, modifiers)) = response
            .ctx
            .input(|input| key_drag::primary_pointer_event(&input.events, true))
            && response.interact_rect.contains(position)
            && (response.is_pointer_button_down_on()
                || response.clicked()
                || response.drag_started_by(egui::PointerButton::Primary)
                || response.drag_stopped_by(egui::PointerButton::Primary))
            && let Some(world) = world.filter(|world| world.iter().all(|value| value.is_finite()))
        {
            self.selection_input.viewport = Some(ViewportPress {
                target: axis.map_or_else(
                    || picked.map_or(ViewportPressTarget::Empty, ViewportPressTarget::Item),
                    ViewportPressTarget::Gizmo,
                ),
                position,
                world,
                modifiers,
                selection: self.selection.clone(),
                setup: self.setup_mode,
                animation: self.animation,
                time: self.time,
                skin: self.skin.clone(),
                tool: self.tool,
                axes: self.axes,
                auto_key: self.auto_key,
                pose_generation: self.pose_view_generation,
                document_generation: self.session.document_generation(),
                rect,
                pan: self.pan,
                zoom: self.zoom,
            });
        }
        if self
            .selection_input
            .viewport
            .as_ref()
            .is_some_and(|press| !press.matches(self, rect))
        {
            self.selection_input.viewport = None;
        }
    }

    pub(super) fn take_viewport_press(&mut self) -> Option<ViewportPress> {
        self.selection_input.viewport.take()
    }

    pub(super) fn settle_viewport_press(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.pointer.primary_down()) {
            self.selection_input.viewport = None;
        }
    }

    pub(super) fn viewport_press_diagnostic(&self) -> serde_json::Value {
        self.selection_input.viewport.as_ref().map_or(serde_json::Value::Null, |press| {
            serde_json::json!({
                "position":[press.position.x,press.position.y],"world":press.world,
                "target":match &press.target {
                    ViewportPressTarget::Empty=>serde_json::json!({"type":"empty"}),
                    ViewportPressTarget::Item(item)=>item.diagnostic(),
                    ViewportPressTarget::Gizmo(axis)=>serde_json::json!({"type":"gizmo","axis":axis}),
                },
                "selection":press.selection.diagnostic(),"setup":press.setup,
                "animation":press.animation,"time":press.time,"skin":press.skin,
                "command":press.modifiers.command,"ctrl":press.modifiers.ctrl,
            })
        })
    }

    pub(super) fn select_with_modifiers(&mut self, item: TreeItem, modifiers: egui::Modifiers) {
        if self.slider_source_picker_active()
            || self.ik_target.is_some()
            || self.weights.bind.is_some()
        {
            self.select(item);
            return;
        }
        let TreeItem::Bone(bone) = item else {
            self.select(item);
            return;
        };
        if !(modifiers.command || modifiers.ctrl) {
            self.select(TreeItem::Bone(bone));
            return;
        }
        if bone >= self.session.document().bones.len() {
            return;
        }
        let mut next = self.selection.clone();
        next.toggle_bone(bone, selection::PrimaryPolicy::Keep);
        self.cancel_gesture();
        self.selection_history.record(&self.selection, &next);
        self.restore_tree_selection(next);
    }
}
