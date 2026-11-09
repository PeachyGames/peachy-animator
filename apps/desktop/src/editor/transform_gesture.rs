//! Frozen transform input; Session owns the document, pending pose and transaction baseline.
use super::*;

#[derive(Clone, Copy)]
enum Input {
    Viewport {
        start: [f32; 2],
        axis: Option<usize>,
    },
    Numeric {
        control: egui::Id,
        property: Property,
        base: f32,
        value: f32,
        touched: bool,
        valid_text: bool,
        text: bool,
    },
}

pub(super) struct TransformEdit {
    item: TreeItem,
    initial: Transform,
    applied_initial: Transform,
    applied_world: Affine,
    parent: Affine,
    tool: Tool,
    axes: Axes,
    setup: bool,
    animation: usize,
    time: f32,
    auto_key: bool,
    properties: Vec<Property>,
    input: Input,
    finish_pending: bool,
    error: Option<String>,
}

pub(super) fn numeric_intent(ctx: &egui::Context) -> bool {
    ctx.input(|input| input.events.iter().any(|event| {
        matches!(event, egui::Event::Text(_) | egui::Event::Paste(_) | egui::Event::Cut
            | egui::Event::Ime(egui::ImeEvent::Commit(_) | egui::ImeEvent::DeleteSurrounding { .. })
            | egui::Event::Key { key: egui::Key::Backspace | egui::Key::Delete | egui::Key::ArrowUp | egui::Key::ArrowDown, pressed: true, .. })
        || matches!(event, egui::Event::Key { key: egui::Key::Z | egui::Key::Y | egui::Key::X, pressed: true, modifiers, .. } if modifiers.command)
    }))
}

impl Editor {
    fn begin_transform_edit(
        &mut self,
        properties: Vec<Property>,
        mut input: Input,
    ) -> Result<(), String> {
        self.cancel_gesture();
        self.session.commit_transaction();
        if !self.transform_editable()
            || properties.is_empty()
            || properties
                .iter()
                .any(|p| matches!(p, Property::ShearX | Property::ShearY))
                && self.selected_bone().is_none()
        {
            return Err("Select a bone for Shear, or an editable transform".into());
        }
        let pose = self.selected_pose()?;
        let (initial, parent) = (pose.authored, pose.parent);
        if let Input::Numeric {
            property,
            ref mut base,
            ref mut value,
            ..
        } = input
        {
            let current = property.read(
                if property.is_scale() || matches!(property, Property::ShearX | Property::ShearY) {
                    initial
                } else {
                    peachy_engine::transform_values(initial, parent, self.axes)
                },
            );
            *base = current;
            *value = current;
        }
        self.playing = false;
        self.transform_blocked = false;
        self.session.begin_transaction();
        self.drag = Some(TransformEdit {
            item: self.selection.clone().unwrap(),
            initial,
            applied_initial: pose.applied,
            applied_world: pose.world,
            parent,
            tool: self.tool,
            axes: self.axes,
            setup: self.setup_mode,
            animation: self.animation,
            time: self.time,
            auto_key: self.auto_key,
            properties,
            input,
            finish_pending: false,
            error: None,
        });
        Ok(())
    }
    pub(super) fn cancel_transform_edit(&mut self) {
        if let Some(edit) = self.drag.take() {
            if let Input::Numeric { control, .. } = edit.input {
                self.transform_blocked_control = Some(control);
            }
            self.transform_blocked = true;
            self.session.cancel_transaction();
        }
    }
    pub(super) fn transform_key_ready(&mut self) -> Result<(), String> {
        let Some(edit) = self.drag.as_mut() else {
            return Ok(());
        };
        edit.finish_pending = false;
        if matches!(
            edit.input,
            Input::Numeric {
                valid_text: false,
                ..
            }
        ) {
            return Err("Enter a valid transform value before keying".into());
        }
        edit.error.clone().map_or(Ok(()), Err)
    }
    pub(super) fn seal_transform_for_key(&mut self) {
        if let Some(edit) = self.drag.take() {
            if let Input::Numeric { control, .. } = edit.input {
                self.transform_blocked_control = Some(control);
            }
            self.transform_blocked = true;
        }
    }
    pub(super) fn hold_transform_key_error(&mut self, error: &str) {
        if let Some(edit) = &mut self.drag {
            edit.finish_pending = false;
            edit.error = Some(error.into());
        }
    }
    fn finish_transform_edit(&mut self) {
        if self.drag.take().is_some() {
            self.session.commit_transaction();
        }
    }
    pub(super) fn reconcile_transform_edit(&mut self) {
        if self.drag.as_ref().is_some_and(|edit| {
            self.selection.as_ref() != Some(&edit.item)
                || !edit.item.exists(self.session.document())
                || self.tool != edit.tool
                || self.axes != edit.axes
                || self.setup_mode != edit.setup
                || self.animation != edit.animation
                || self.time != edit.time
                || self.auto_key != edit.auto_key
        }) {
            self.cancel_transform_edit();
        }
    }
    fn preview_transform(&mut self, mut transform: Transform) -> Result<(), String> {
        let edit = self.drag.as_ref().ok_or("No active transform gesture")?;
        if edit.setup && edit.properties.contains(&Property::Rotation) {
            transform.rotation = self.setup_rotation_value(Some(&edit.item), transform.rotation);
        }
        let target = if edit.setup {
            EditTarget::Setup
        } else {
            EditTarget::Animate {
                animation: edit.animation,
                time: edit.time,
            }
        };
        let result = match &edit.item {
            TreeItem::Bone(bone) => self.session.preview_edit_pose(
                *bone,
                target,
                transform,
                &edit.properties,
                edit.auto_key,
            ),
            TreeItem::Attachment(attachment) if edit.setup => RegionTransform::try_from(transform)
                .and_then(|transform| {
                    self.session
                        .preview_transaction(Command::SetRegionTransform {
                            attachment: attachment.clone(),
                            transform,
                        })
                }),
            _ => Err("Selected item has no editable transform".into()),
        };
        self.drag.as_mut().unwrap().error = result.as_ref().err().cloned();
        if let Err(error) = &result {
            self.status = error.clone();
        }
        result
    }
    pub(super) fn prepare_viewport_transform_press(&mut self, ui: &egui::Ui, rect: egui::Rect) {
        if self
            .drag
            .as_ref()
            .is_some_and(|edit| matches!(edit.input, Input::Numeric { .. }))
            && ui.input(|input| {
                input.pointer.primary_pressed()
                    && input
                        .pointer
                        .press_origin()
                        .is_some_and(|point| rect.contains(point))
            })
        {
            self.cancel_transform_edit();
            // This is a fresh pointer press, not continuation of a canceled held gesture.
            self.transform_blocked = false;
        }
    }
    pub(super) fn begin_viewport_transform(&mut self, start: [f32; 2], axis: Option<usize>) {
        if self.transform_blocked || !self.transform_tool_editable(self.tool) {
            return;
        }
        let properties = self.tool.properties().to_vec();
        if let Err(error) = self.begin_transform_edit(properties, Input::Viewport { start, axis }) {
            self.status = error;
        }
    }
    pub(super) fn viewport_transform_active(&self) -> bool {
        self.drag
            .as_ref()
            .is_some_and(|edit| matches!(edit.input, Input::Viewport { .. }))
    }
    pub(super) fn update_viewport_transform(&mut self, end: [f32; 2], snap_rotation: bool) {
        let Some(edit) = self.drag.as_ref() else {
            return;
        };
        let Input::Viewport { start, axis } = edit.input else {
            return;
        };
        let Some(tool) = edit.tool.transform_tool() else {
            return;
        };
        let result = peachy_engine::drag_transform_in_frame(
            edit.initial,
            edit.applied_initial,
            edit.parent,
            edit.applied_world,
            peachy_engine::TransformGesture {
                tool,
                axes: edit.axes,
                axis,
                start,
                end,
                snap_rotation,
            },
        );
        let result = result.and_then(|transform| self.preview_transform(transform));
        if let Err(error) = result {
            if let Some(edit) = self.drag.as_mut() {
                edit.error = Some(error.clone());
            }
            self.status = error;
        }
    }
    pub(super) fn finish_viewport_transform(&mut self) {
        if let Some(edit) = self.drag.as_mut()
            && matches!(edit.input, Input::Viewport { .. })
        {
            edit.finish_pending = true;
        }
    }
    pub(super) fn transform_numeric_values(&self, id: egui::Id, current: f32) -> (f32, f32) {
        match self.drag.as_ref().map(|edit| edit.input) {
            Some(Input::Numeric {
                control,
                base,
                value,
                ..
            }) if control == id => (value, base),
            _ => (current, current),
        }
    }
    pub(super) fn prepare_transform_numeric(&mut self, ui: &mut egui::Ui, id: egui::Id) {
        if self.transform_blocked_control == Some(id) {
            if ui.memory(|memory| memory.has_focus(id) && !memory.had_focus_last_frame(id)) {
                // Fresh keyboard/accessibility entry resets DragValue's old edit state.
                self.transform_blocked_control = None;
            } else {
                ui.memory_mut(|memory| memory.surrender_focus(id));
            }
        }
    }
    pub(super) fn transform_numeric_response(
        &mut self,
        response: &egui::Response,
        property: Property,
        value: f32,
        parsed: Option<bool>,
    ) {
        if self.transform_blocked || !response.enabled() {
            return;
        }
        let explicit_start =
            response.gained_focus() || response.clicked() || response.drag_started();
        if self.transform_blocked_control == Some(response.id) {
            if !explicit_start {
                return;
            }
            self.transform_blocked_control = None;
        }
        let owns = self.drag.as_ref().is_some_and(
            |edit| matches!(edit.input,Input::Numeric {control,..} if control==response.id),
        );
        if !owns && explicit_start {
            // Moving between numeric fields completes the previous explicit numeric entry.
            if self
                .drag
                .as_ref()
                .is_some_and(|edit| matches!(edit.input, Input::Numeric { .. }))
            {
                self.finish_transform_edit();
            }
            let properties = if matches!(property, Property::X | Property::Y) {
                vec![Property::X, Property::Y]
            } else {
                vec![property]
            };
            let current = match self.selected_pose() {
                Ok(pose) => {
                    let (local, parent) = (pose.authored, pose.parent);
                    property.read(
                        if property.is_scale()
                            || matches!(property, Property::ShearX | Property::ShearY)
                        {
                            local
                        } else {
                            peachy_engine::transform_values(local, parent, self.axes)
                        },
                    )
                }
                Err(error) => {
                    self.evaluation_error = Some(error.clone());
                    self.status = error;
                    return;
                }
            };
            let base = current;
            if let Err(error) = self.begin_transform_edit(
                properties,
                Input::Numeric {
                    control: response.id,
                    property,
                    base,
                    value: base,
                    touched: false,
                    valid_text: true,
                    text: response.has_focus(),
                },
            ) {
                self.status = error;
                return;
            }
        }
        let Some(edit) = self.drag.as_mut() else {
            return;
        };
        let Input::Numeric {
            control,
            touched,
            valid_text,
            text,
            value: requested,
            ..
        } = &mut edit.input
        else {
            return;
        };
        if *control != response.id {
            return;
        }
        *touched |= numeric_intent(&response.ctx);
        if response.has_focus() || response.is_pointer_button_down_on() {
            edit.finish_pending = false;
        }
        if let Some(valid) = parsed {
            *valid_text = valid;
        }
        let changes =
            *valid_text && response.changed() && (!*text || *touched) && value != *requested;
        if changes {
            *requested = value;
        }
        let finish = response.lost_focus()
            || (!response.has_focus() && !response.ctx.input(|input| input.pointer.any_down()));
        edit.finish_pending |= finish;
        if changes {
            let edit = self.drag.as_ref().unwrap();
            let result = peachy_engine::set_transform_value(
                edit.initial,
                edit.parent,
                edit.axes,
                property,
                value,
            )
            .and_then(|transform| self.preview_transform(transform));
            if let Err(error) = result {
                self.drag.as_mut().unwrap().error = Some(error.clone());
                self.status = error;
            }
        }
    }
    pub(super) fn transform_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        let Some(edit) = self.drag.as_ref() else {
            return false;
        };
        if !ctx.input(|input| input.focused)
            || self.error.is_some()
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
            || egui::Popup::is_any_open(ctx)
        {
            self.cancel_transform_edit();
            return false;
        }
        let control = match edit.input {
            Input::Numeric { control, .. } => Some(control),
            _ => None,
        };
        let owns_text = control.is_some_and(|id| ctx.memory(|memory| memory.has_focus(id)));
        if owns_text
            && numeric_intent(ctx)
            && let Some(edit) = self.drag.as_mut()
            && let Input::Numeric { touched, .. } = &mut edit.input
        {
            *touched = true;
        }
        if owns_text
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::S))
        {
            self.cancel_transform_edit();
            if let Some(id) = control {
                ctx.memory_mut(|memory| memory.surrender_focus(id));
            }
            self.save_project();
            return true;
        }
        if (!ctx.text_edit_focused() || owns_text)
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.cancel_transform_edit();
            if let Some(id) = control {
                ctx.memory_mut(|memory| memory.surrender_focus(id));
            }
            return true;
        }
        false
    }
    pub(super) fn settle_transform_edit(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.focused)
            || self.error.is_some()
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
            || egui::Popup::is_any_open(ctx)
        {
            self.cancel_transform_edit();
        }
        self.reconcile_transform_edit();
        if !ctx.input(|input| input.pointer.any_down()) {
            if self.drag.as_ref().is_some_and(|edit| {
                edit.finish_pending
                    && edit.error.is_none()
                    && !matches!(
                        edit.input,
                        Input::Numeric {
                            valid_text: false,
                            ..
                        }
                    )
            }) {
                self.finish_transform_edit();
            }
            self.transform_blocked = false;
        }
    }
    pub(super) fn transform_gesture_diagnostic(&self) -> serde_json::Value {
        self.drag.as_ref().map_or(serde_json::Value::Null,|edit|serde_json::json!({
            "item":edit.item.diagnostic(),"tool":match edit.tool {Tool::Rotate=>"rotate",Tool::Move=>"translate",Tool::Scale=>"scale",Tool::Shear=>"shear",Tool::Mesh=>"mesh",Tool::Weights=>"weights"},"initialPose":edit.initial,"appliedInitialPose":edit.applied_initial,"appliedWorld":edit.applied_world,"parent":edit.parent,"axes":edit.axes,
            "setup":edit.setup,"animation":edit.animation,"time":edit.time,"autoKey":edit.auto_key,
            "properties":edit.properties,"error":edit.error,"finishPending":edit.finish_pending,
            "input":match edit.input {Input::Viewport {start,axis}=>serde_json::json!({"kind":"viewport","start":start,"axis":axis}),
                Input::Numeric {property,base,value,text,..}=>serde_json::json!({"kind":"numeric","property":property,"base":base,"value":value,"text":text})}
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{editor_context, editor_fixture, mesh_editor};
    use super::*;

    fn key(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers,
            })
            .collect()
    }
    fn frame(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>) {
        frame_with_focus(editor, ctx, events, None);
    }
    fn frame_with_focus(
        editor: &mut Editor,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        focus: Option<egui::Id>,
    ) {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
                focused: true,
                events,
                ..Default::default()
            },
            |ui| {
                editor.reconcile_selection();
                editor.shortcuts(ui.ctx());
                if let Some(id) = focus {
                    ui.memory_mut(|memory| memory.request_focus(id));
                }
                egui::Panel::top("test-toolbar").show(ui, |ui| editor.toolbar(ui));
                if editor.show_weights {
                    egui::Panel::right("test-weights")
                        .exact_size(310.)
                        .show(ui, |ui| editor.weights_panel(ui));
                }
                egui::CentralPanel::default().show(ui, |ui| {
                    let height = layout::main_tools_height(ui);
                    egui::Panel::bottom("test-tools")
                        .exact_size(height)
                        .frame(egui::Frame::new().inner_margin(7))
                        .show(ui, |ui| editor.main_tools(ui));
                    editor.canvas(ui);
                });
                editor.finish_idle_transactions(ui.ctx());
            },
        );
        output.textures_delta.clear();
    }
    fn point(editor: &Editor, name: &str) -> Pos2 {
        let r = editor.capture_targets[name];
        pos2((r[0] + r[2]) / 2., (r[1] + r[3]) / 2.)
    }
    fn button(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }
    fn click(editor: &mut Editor, ctx: &egui::Context, name: &str) {
        let pos = point(editor, name);
        frame(
            editor,
            ctx,
            vec![egui::Event::PointerMoved(pos), button(pos, true)],
        );
        frame(editor, ctx, vec![button(pos, false)]);
    }
    #[test]
    fn fixed_viewport_pose_previews_cancel_with_exact_pending_pose_redo_and_frozen_context() {
        let mut editor = editor_fixture();
        editor.set_tool(Tool::Shear);
        let mut pose = editor.selected_transform().unwrap();
        pose.rotation += 17.;
        editor.edit_transform(pose, &[Property::Rotation]);
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "redo".into(),
            })
            .unwrap();
        editor.undo();
        let original = editor.session.document().clone();
        let pending = editor.session.pose_edits().clone();
        let parent = editor.selected_parent().unwrap();
        let local = editor.selected_transform().unwrap();
        let start = parent.point(local.x + 40., local.y);
        editor.begin_viewport_transform(start, Some(0));
        let end = |angle: f32| {
            parent.point(
                local.x + 40. * angle.to_radians().cos(),
                local.y + 40. * angle.to_radians().sin(),
            )
        };
        editor.update_viewport_transform(end(20.), false);
        editor.update_viewport_transform(end(40.), false);
        let current = editor.selected_transform().unwrap();
        assert!((current.shear_x - 40.).abs() < 0.001);
        assert_eq!(current.rotation, local.rotation);
        assert_eq!(current.shear_y, local.shear_y);
        assert_eq!(editor.session.document(), &original);
        let capsule = editor.drag.as_ref().unwrap();
        assert_eq!(capsule.time, 0.42);
        assert_eq!(capsule.initial, local);
        assert!(!capsule.auto_key);
        editor.auto_key = true;
        editor.reconcile_selection();
        assert!(editor.drag.is_none());
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert!(editor.session.can_redo());
    }
    #[test]
    fn real_numeric_enter_tab_and_unmodified_z_keep_pairs_and_one_step_history() {
        let mut editor = editor_fixture();
        let ctx = editor_context();
        editor.set_tool(Tool::Shear);
        let original = editor.session.document().clone();
        frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "value:ShearX");
        frame(&mut editor, &ctx, vec![egui::Event::Text("+25".into())]);
        assert_eq!(editor.selected_transform().unwrap().shear_x, 0.);
        assert!(editor.drag.is_some());
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::Enter, egui::Modifiers::NONE),
        );
        assert!(editor.drag.is_none());
        assert_eq!(editor.selected_transform().unwrap().shear_x, 25.);
        assert_eq!(editor.session.document(), &original);
        editor.undo();
        assert_eq!(editor.selected_transform().unwrap().shear_x, 0.);
        assert!(!editor.session.can_undo());
        editor.redo();
        assert_eq!(editor.selected_transform().unwrap().shear_x, 25.);
        click(&mut editor, &ctx, "value:ShearY");
        frame(&mut editor, &ctx, vec![egui::Event::Text("-15".into())]);
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::Tab, egui::Modifiers::NONE),
        );
        assert_eq!(editor.selected_transform().unwrap().shear_y, -15.);
        editor.key_transform(Tool::Shear.properties());
        assert!(
            editor.session.document().animations[0]
                .tracks
                .iter()
                .any(|track| track.bone == 8 && track.property == Property::ShearX)
        );
        assert!(
            editor.session.document().animations[0]
                .tracks
                .iter()
                .any(|track| track.bone == 8 && track.property == Property::ShearY)
        );
    }
    #[test]
    fn direct_and_transform_numeric_capture_cancel_the_previous_owner_before_stale_blur() {
        let (mut editor, mesh) = mesh_editor(true);
        let ctx = editor_context();
        editor.set_setup_mode(false);
        editor.set_time(0.37);
        editor.set_tool(Tool::Weights);
        editor.weights.vertex = Some(0);
        editor.weights.bone = Some(8);
        editor.select(TreeItem::Bone(8));
        let baseline = editor.session.document().attachment(&mesh).unwrap().clone();
        frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "weightsWeightValue");
        frame(&mut editor, &ctx, vec![egui::Event::Text("30".into())]);
        assert!(editor.weight_edit.is_some());
        click(&mut editor, &ctx, "value:ShearX");
        assert!(editor.weight_edit.is_none());
        assert!(editor.drag.is_some());
        assert_eq!(
            editor.session.document().attachment(&mesh).unwrap(),
            &baseline
        );
        frame(&mut editor, &ctx, vec![egui::Event::Text("15".into())]);
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::Enter, egui::Modifiers::NONE),
        );
        assert_eq!(editor.selected_transform().unwrap().shear_x, 15.);
        click(&mut editor, &ctx, "value:ShearX");
        frame(&mut editor, &ctx, vec![egui::Event::Text("25".into())]);
        click(&mut editor, &ctx, "weightsWeightValue");
        assert!(editor.drag.is_none());
        assert!(editor.weight_edit.is_some());
        frame(&mut editor, &ctx, vec![egui::Event::Text("40".into())]);
        assert_eq!(editor.selected_transform().unwrap().shear_x, 15.);
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::Escape, egui::Modifiers::NONE),
        );
        assert_eq!(
            editor.session.document().attachment(&mesh).unwrap(),
            &baseline
        );
        assert_eq!(editor.time, 0.37);
        assert_eq!(editor.selected_transform().unwrap().shear_x, 15.);
    }
    fn turn(start: Pos2, pivot: Pos2, degrees: f32) -> Pos2 {
        let d = start - pivot;
        let (s, c) = degrees.to_radians().sin_cos();
        pivot + vec2(d.x * c + d.y * s, -d.x * s + d.y * c)
    }
    #[test]
    fn actual_shear_handles_edit_only_the_selected_axis_and_commit_one_pending_pose_step() {
        for (name, property, other) in [
            ("gizmo:X", Property::ShearX, Property::ShearY),
            ("gizmo:Y", Property::ShearY, Property::ShearX),
        ] {
            let mut editor = editor_fixture();
            let ctx = editor_context();
            editor.set_tool(Tool::Shear);
            frame(&mut editor, &ctx, vec![]);
            let start = point(&editor, name);
            let pivot = point(&editor, "gizmoOrigin");
            let end = turn(start, pivot, 20.);
            let original = editor.session.document().clone();
            frame(
                &mut editor,
                &ctx,
                vec![egui::Event::PointerMoved(start), button(start, true)],
            );
            frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
            assert!(editor.viewport_transform_active());
            assert!((property.read(editor.selected_transform().unwrap()) - 20.).abs() < 0.001);
            assert_eq!(other.read(editor.selected_transform().unwrap()), 0.);
            frame(&mut editor, &ctx, vec![button(end, false)]);
            assert!(editor.drag.is_none());
            assert_eq!(editor.session.document(), &original);
            editor.undo();
            assert!(editor.session.pose_edits().is_empty());
            assert!(!editor.session.can_undo());
        }
    }
    #[test]
    fn held_handle_escape_or_save_does_not_restart_and_unobserved_empty_space_errors_visibly() {
        for cancel in [egui::Key::Escape, egui::Key::S] {
            let mut editor = editor_fixture();
            let ctx = editor_context();
            editor.set_tool(Tool::Shear);
            let folder = std::env::temp_dir().join(format!(
                "peachy-shear-cancel-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&folder).unwrap();
            editor.project_path = Some(folder.join("rig.peachy"));
            editor.asset_root =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
            frame(&mut editor, &ctx, vec![]);
            let start = point(&editor, "gizmo:X");
            let pivot = point(&editor, "gizmoOrigin");
            let end = turn(start, pivot, 20.);
            frame(
                &mut editor,
                &ctx,
                vec![egui::Event::PointerMoved(start), button(start, true)],
            );
            frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
            assert!(editor.viewport_transform_active());
            frame(
                &mut editor,
                &ctx,
                key(
                    cancel,
                    if cancel == egui::Key::S {
                        egui::Modifiers::COMMAND
                    } else {
                        egui::Modifiers::NONE
                    },
                ),
            );
            assert!(editor.session.pose_edits().is_empty());
            assert!(editor.drag.is_none());
            let end = turn(start, pivot, 35.);
            frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
            frame(&mut editor, &ctx, vec![button(end, false)]);
            assert!(editor.session.pose_edits().is_empty());
            assert!(!editor.session.can_undo());
            if cancel == egui::Key::S {
                assert!(folder.join("rig.peachy").exists());
            }
            std::fs::remove_dir_all(folder).unwrap();
        }
        let mut editor = editor_fixture();
        let ctx = editor_context();
        editor.set_tool(Tool::Shear);
        frame(&mut editor, &ctx, vec![]);
        let rect = editor.capture_targets["viewport"];
        let start = pos2(rect[0] + 30., rect[3] - 40.);
        let end = start + vec2(25., 10.);
        frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(start), button(start, true)],
        );
        frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
        assert!(editor.drag.as_ref().unwrap().error.is_some());
        assert!(!editor.status.is_empty());
        assert!(editor.session.pose_edits().is_empty());
        frame(&mut editor, &ctx, vec![button(end, false)]);
        assert!(!editor.session.can_undo());
    }
    #[test]
    fn auto_key_viewport_uses_frozen_pair_and_undo_restores_overwritten_keys() {
        let mut editor = editor_fixture();
        editor.set_tool(Tool::Shear);
        editor.set_time(0.4);
        editor.auto_key = true;
        let mut document = editor.session.document().clone();
        for property in [Property::ShearX, Property::ShearY] {
            document.animations[0].tracks.push(Track {
                bone: 8,
                property,
                keys: vec![Keyframe {
                    time: 0.4,
                    value: 3.,
                    curve: Curve::Stepped,
                }],
            });
        }
        editor.session = Session::new(document).unwrap();
        let original = editor.session.document().clone();
        let parent = editor.selected_parent().unwrap();
        let local = editor.selected_transform().unwrap();
        let start = parent.point(local.x + 40., local.y);
        editor.begin_viewport_transform(start, Some(1));
        let end = parent.point(
            local.x + 40. * 20f32.to_radians().cos(),
            local.y + 40. * 20f32.to_radians().sin(),
        );
        editor.update_viewport_transform(end, false);
        assert!(editor.session.pose_edits().is_empty());
        assert!((editor.selected_transform().unwrap().shear_y - 23.).abs() < 0.001);
        let modified = editor.session.document().clone();
        assert_ne!(&modified, &original);
        editor.finish_viewport_transform();
        let ctx = editor_context();
        frame(&mut editor, &ctx, vec![]);
        assert!(editor.drag.is_none());
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        editor.redo();
        assert_eq!(editor.session.document(), &modified);
    }
    #[test]
    fn rounded_or_invalid_numeric_blur_is_noop_and_real_save_click_cancels_before_write() {
        let mut editor = editor_fixture();
        let mut document = editor.session.document().clone();
        document.bones[8].setup.shear_x = 0.33333334;
        editor.session = Session::new(document).unwrap();
        editor.set_tool(Tool::Shear);
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "redo".into(),
            })
            .unwrap();
        editor.undo();
        let original = editor.session.document().clone();
        let ctx = editor_context();
        frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "value:ShearX");
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::Tab, egui::Modifiers::NONE),
        );
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.pose_edits().is_empty());
        assert!(editor.session.can_redo());
        click(&mut editor, &ctx, "value:ShearX");
        frame(&mut editor, &ctx, vec![egui::Event::Text("junk".into())]);
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::Enter, egui::Modifiers::NONE),
        );
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.pose_edits().is_empty());
        assert!(editor.session.can_redo());
        let folder = std::env::temp_dir().join(format!(
            "peachy-shear-save-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&folder).unwrap();
        editor.project_path = Some(folder.join("rig.peachy"));
        editor.asset_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
        click(&mut editor, &ctx, "value:ShearX");
        frame(&mut editor, &ctx, vec![egui::Event::Text("15".into())]);
        let save = point(&editor, "toolbarSave");
        frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(save), button(save, true)],
        );
        assert!(editor.drag.is_some());
        assert!(!folder.join("rig.peachy").exists());
        frame(&mut editor, &ctx, vec![button(save, false)]);
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::Enter, egui::Modifiers::NONE),
        );
        assert!(editor.drag.is_none());
        assert!(editor.session.pose_edits().is_empty());
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
        let saved = files::load(&folder.join("rig.peachy")).unwrap();
        assert_eq!(saved.bones, original.bones);
        std::fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn enter_tab_to_local_axes_keeps_global_history_and_save_shortcuts() {
        let mut editor = editor_fixture();
        editor.set_setup_mode(true);
        editor.set_tool(Tool::Shear);
        let original = editor.session.document().clone();
        let ctx = editor_context();
        frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "value:ShearX");
        frame(&mut editor, &ctx, vec![egui::Event::Text("23.5".into())]);
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::Enter, egui::Modifiers::NONE),
        );
        let first = editor.session.document().clone();
        assert_eq!(first.bones[8].setup.shear_x, 23.5);
        click(&mut editor, &ctx, "value:ShearY");
        frame(&mut editor, &ctx, vec![egui::Event::Text("-17.25".into())]);
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::Tab, egui::Modifiers::NONE),
        );
        let edited = editor.session.document().clone();
        assert_eq!(edited.bones[8].setup.shear_y, -17.25);
        assert!(editor.drag.is_none());
        assert!(!ctx.text_edit_focused());
        assert!(ctx.egui_wants_keyboard_input());
        let focus = ctx.memory(|memory| memory.focused()).unwrap();
        let focus_rect = ctx.read_response(focus).unwrap().rect;
        assert!(focus_rect.contains(point(&editor, "axes:Local")));

        frame(
            &mut editor,
            &ctx,
            key(egui::Key::Z, egui::Modifiers::COMMAND),
        );
        assert_eq!(editor.session.document(), &first);
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::Z, egui::Modifiers::COMMAND),
        );
        assert_eq!(editor.session.document(), &original);
        frame(
            &mut editor,
            &ctx,
            key(
                egui::Key::Z,
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
            ),
        );
        assert_eq!(editor.session.document(), &first);
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::Y, egui::Modifiers::COMMAND),
        );
        assert_eq!(editor.session.document(), &edited);

        // Normal focused controls retain their own unmodified/arrow routing.
        let before = editor.selected_transform();
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::ArrowRight, egui::Modifiers::NONE),
        );
        assert_eq!(editor.selected_transform(), before);
        let folder = std::env::temp_dir().join(format!(
            "peachy-focused-control-save-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&folder).unwrap();
        editor.project_path = Some(folder.join("rig.peachy"));
        editor.asset_root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::S, egui::Modifiers::COMMAND),
        );
        assert_eq!(
            files::load(&folder.join("rig.peachy")).unwrap().bones,
            edited.bones
        );
        assert!(!editor.session.is_dirty());
        std::fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn escape_then_keyboard_tab_can_reenter_the_same_numeric_control() {
        let mut editor = editor_fixture();
        let ctx = editor_context();
        editor.set_tool(Tool::Shear);
        frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "value:ShearX");
        let control = match editor.drag.as_ref().unwrap().input {
            Input::Numeric { control, .. } => control,
            _ => panic!(),
        };
        frame(&mut editor, &ctx, vec![egui::Event::Text("25".into())]);
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::Escape, egui::Modifiers::NONE),
        );
        assert!(editor.session.pose_edits().is_empty());
        let mut entered = false;
        for _ in 0..64 {
            frame(
                &mut editor,
                &ctx,
                key(egui::Key::Tab, egui::Modifiers::NONE),
            );
            if editor.drag.as_ref().is_some_and(
                |edit| matches!(edit.input,Input::Numeric {control:id,..} if id==control),
            ) {
                entered = true;
                break;
            }
        }
        assert!(
            entered,
            "Canceled field must be reachable by normal Tab traversal"
        );
        frame(&mut editor, &ctx, vec![egui::Event::Text("15".into())]);
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::Enter, egui::Modifiers::NONE),
        );
        assert_eq!(editor.selected_transform().unwrap().shear_x, 15.);
        click(&mut editor, &ctx, "value:ShearX");
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::Escape, egui::Modifiers::NONE),
        );
        // A host/accessibility focus transfer occurs before the widget is rendered.
        // Verify actual text reaches the model, not just that focus was requested.
        frame_with_focus(&mut editor, &ctx, vec![], Some(control));
        assert!(ctx.text_edit_focused());
        frame(&mut editor, &ctx, vec![egui::Event::Text("20".into())]);
        frame(
            &mut editor,
            &ctx,
            key(egui::Key::Enter, egui::Modifiers::NONE),
        );
        assert_eq!(editor.selected_transform().unwrap().shear_x, 20.);
    }
}
