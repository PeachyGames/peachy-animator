//! One input lease and frozen transaction lifecycle for every curve surface.
use super::*;
use peachy_engine::AnimationKey;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CurveInputOwner {
    DeformGraph,
    SlotGraph,
    ScalarGraph,
    SliderGraph,
    Curves,
}

#[derive(Default)]
pub(super) struct CurveInteraction {
    owner: Option<CurveInputOwner>,
    blocked: bool,
}

pub(super) struct CurveGesture {
    pub owner: CurveInputOwner,
    pub handle: usize,
    pub rect: egui::Rect,
    pub bounds: [f64; 4],
    pub finish_pending: bool,
    animation: usize,
    cursor: f32,
    skin: String,
    selection: TreeSelection,
    keys: Vec<AnimationKey>,
    tool: Tool,
    axes: Axes,
    auto_key: bool,
    pose_generation: u64,
    pose: peachy_engine::PoseEdits,
}

impl Editor {
    pub(super) fn begin_curve_gesture(
        &mut self,
        owner: CurveInputOwner,
        handle: usize,
        rect: egui::Rect,
        bounds: [f64; 4],
    ) -> CurveGesture {
        self.cancel_gesture();
        self.session.commit_transaction();
        self.session.begin_transaction();
        self.playing = false;
        self.curve_interaction.owner = Some(owner);
        self.curve_interaction.blocked = false;
        CurveGesture {
            owner,
            handle,
            rect,
            bounds,
            finish_pending: false,
            animation: self.animation,
            cursor: self.time,
            skin: self.skin.clone(),
            selection: self.selection.clone(),
            keys: self.key_selection.keys().to_vec(),
            tool: self.tool,
            axes: self.axes,
            auto_key: self.auto_key,
            pose_generation: self.pose_view_generation,
            pose: self.session.pose_edits().clone(),
        }
    }

    pub(super) fn curve_gesture_context_valid(
        &self,
        edit: &CurveGesture,
        ctx: &egui::Context,
        visible: bool,
    ) -> bool {
        visible
            && !self.setup_mode
            && ctx.input(|input| input.focused)
            && self.error.is_none()
            && self.rename.is_none()
            && self.duplicate_animation.is_none()
            && self.export_dialog.is_none()
            && !self.slot_color_dialog_open()
            && !egui::Popup::is_any_open(ctx)
            && !ctx.text_edit_focused()
            && self.curve_interaction.owner == Some(edit.owner)
            && self.animation == edit.animation
            && self.time.to_bits() == edit.cursor.to_bits()
            && self.skin == edit.skin
            && self.selection == edit.selection
            && self.key_selection.keys() == edit.keys.as_slice()
            && self.tool == edit.tool
            && self.axes == edit.axes
            && self.auto_key == edit.auto_key
            && self.pose_view_generation == edit.pose_generation
            && self.session.pose_edits() == &edit.pose
    }

    pub(super) fn cancel_curve_gesture(&mut self, owner: CurveInputOwner) {
        if self.curve_interaction.owner == Some(owner) {
            self.session.cancel_transaction();
            self.curve_interaction.owner = None;
            self.curve_interaction.blocked = true;
        }
    }

    pub(super) fn curve_gesture_blocked(&mut self, ctx: &egui::Context) -> bool {
        if !ctx.input(|input| input.pointer.primary_down()) {
            self.curve_interaction.blocked = false;
        }
        self.curve_interaction.blocked
    }

    pub(super) fn settle_curve_gesture(
        &mut self,
        owner: CurveInputOwner,
        ctx: &egui::Context,
        finish_pending: bool,
    ) -> bool {
        if finish_pending
            && !ctx.input(|input| input.pointer.primary_down())
            && self.curve_interaction.owner == Some(owner)
        {
            self.curve_interaction.owner = None;
            self.session.commit_transaction();
            return true;
        }
        false
    }

    pub(super) fn curve_gesture_coordinates(
        edit: &CurveGesture,
        pointer: Pos2,
    ) -> Option<[f64; 2]> {
        if !pointer.is_finite()
            || !edit.rect.is_finite()
            || edit.rect.width() <= 0.
            || edit.rect.height() <= 0.
            || !edit.bounds.into_iter().all(f64::is_finite)
            || edit.bounds[1] <= edit.bounds[0]
            || edit.bounds[3] <= edit.bounds[2]
        {
            return None;
        }
        Some([
            edit.bounds[0]
                + (f64::from(pointer.x) - f64::from(edit.rect.left()))
                    / f64::from(edit.rect.width())
                    * (edit.bounds[1] - edit.bounds[0]),
            edit.bounds[2]
                + (f64::from(edit.rect.bottom()) - f64::from(pointer.y))
                    / f64::from(edit.rect.height())
                    * (edit.bounds[3] - edit.bounds[2]),
        ])
    }
}
