//! Constraint controls route explicit identities into shared commands and checked queries.
use super::*;

pub(super) struct IkEdit {
    constraint: usize,
    base: IkConstraint,
    animation: usize,
    time: f32,
    control: egui::Id,
    numeric: bool,
    touched: bool,
    percent: f64,
    finish_pending: bool,
    error: Option<String>,
}

pub(super) struct IkTargetPicker {
    constraint: usize,
    base: IkConstraint,
}

impl Editor {
    pub(super) fn ik_constraint(&self, index: usize) -> Option<&IkConstraint> {
        let ik = self.session.document().constraints.get(index)?.as_ik()?;
        Some(ik)
    }
    fn selected_ik(&self) -> Option<usize> {
        match self.selection {
            Some(TreeItem::Constraint(index)) => Some(index),
            _ => None,
        }
    }
    fn ik_evaluation_status(&mut self) {
        if let Err(error) = self.session.evaluated_pose(self.weights_edit_target()) {
            self.status = format!("IK settings updated; pose cannot be evaluated: {error}");
            self.evaluation_error = Some(error);
        }
    }
    fn begin_ik_edit(&mut self, control: egui::Id, numeric: bool) -> Result<(), String> {
        self.cancel_gesture();
        self.weights.bind = None;
        if !self.setup_mode {
            return Err(
                "IK settings are edited in Setup; constraint keys are not supported yet".into(),
            );
        }
        let constraint = self.selected_ik().ok_or("Select an IK constraint")?;
        let base = self
            .ik_constraint(constraint)
            .ok_or("Select an existing IK constraint")?
            .clone();
        self.session.commit_transaction();
        self.session.begin_transaction();
        self.playing = false;
        self.ik_edit = Some(IkEdit {
            constraint,
            percent: f64::from(base.setup.mix) * 100.,
            base,
            animation: self.animation,
            time: self.time,
            control,
            numeric,
            touched: false,
            finish_pending: false,
            error: None,
        });
        Ok(())
    }
    fn preview_ik_mix(&mut self, percent: f64) {
        let Some(edit) = self.ik_edit.as_mut() else {
            return;
        };
        let mut value = edit.base.clone();
        value.setup.mix = (percent / 100.) as f32;
        edit.percent = percent;
        let result = self.session.preview_transaction(Command::SetIkConstraint {
            constraint: edit.constraint,
            value,
        });
        edit.error = result.as_ref().err().cloned();
        match result {
            Ok(()) => {
                self.status = format!("IK Mix: {percent:.2}%");
                self.ik_evaluation_status();
            }
            Err(error) => self.status = error,
        }
    }
    pub(super) fn cancel_ik_edit(&mut self) {
        if let Some(edit) = self.ik_edit.take() {
            self.ik_blocked = Some(edit.control);
            self.session.cancel_transaction();
        }
    }
    fn finish_ik_edit(&mut self) {
        if self.ik_edit.take().is_some() {
            self.session.commit_transaction();
        }
    }
    pub(super) fn reconcile_ik(&mut self) {
        if self.ik_edit.as_ref().is_some_and(|edit| {
            !self.setup_mode
                || !self.show_tree
                || self.selected_ik() != Some(edit.constraint)
                || edit.animation != self.animation
                || edit.time != self.time
                || self.ik_constraint(edit.constraint).is_none_or(|ik| {
                    ik.name != edit.base.name
                        || ik.bones != edit.base.bones
                        || ik.target != edit.base.target
                        || ik.setup.bend_positive != edit.base.setup.bend_positive
                })
        }) {
            self.cancel_ik_edit();
        }
        if self.ik_target.as_ref().is_some_and(|picker| {
            !self.setup_mode
                || !self.show_tree
                || self.selected_ik() != Some(picker.constraint)
                || self.ik_constraint(picker.constraint) != Some(&picker.base)
        }) {
            self.ik_target = None;
        }
    }
    fn begin_ik_target(&mut self, constraint: usize) {
        self.cancel_gesture();
        self.weights.bind = None;
        if !self.setup_mode {
            return;
        }
        if let Some(base) = self.ik_constraint(constraint).cloned() {
            self.playing = false;
            self.ik_target = Some(IkTargetPicker { constraint, base });
            self.status =
                "Target: choose an existing bone in the Tree or viewport; Escape cancels".into();
        }
    }
    pub(super) fn ik_target_selection(&mut self, item: &TreeItem) -> bool {
        let Some(picker) = self.ik_target.take() else {
            return false;
        };
        let TreeItem::Bone(bone) = item else {
            self.ik_target = Some(picker);
            self.status =
                "Choose an existing target bone; creating a new target is not supported yet".into();
            return true;
        };
        if !self.setup_mode || self.ik_constraint(picker.constraint) != Some(&picker.base) {
            self.status = "Target choice canceled because its constraint changed".into();
            return true;
        }
        let mut value = picker.base;
        value.target = *bone;
        if self.dispatch(Command::SetIkConstraint {
            constraint: picker.constraint,
            value,
        }) {
            self.status = "IK target updated".into();
            self.ik_evaluation_status();
        }
        true
    }
    pub(super) fn ik_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        if !ctx.input(|input| input.focused)
            || self.error.is_some()
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
            || egui::Popup::is_any_open(ctx)
        {
            self.cancel_ik_edit();
            self.ik_target = None;
            return false;
        }
        if self.ik_target.is_some()
            && !ctx.text_edit_focused()
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.ik_target = None;
            self.status = "Target choice canceled".into();
            return true;
        }
        let Some(edit) = self.ik_edit.as_ref() else {
            return false;
        };
        let control = edit.control;
        let owns_text = ctx.memory(|memory| memory.has_focus(control));
        if owns_text && edit.numeric && transform_gesture::numeric_intent(ctx) {
            self.ik_edit.as_mut().unwrap().touched = true;
        }
        if owns_text
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::S))
        {
            self.cancel_ik_edit();
            ctx.memory_mut(|memory| memory.surrender_focus(control));
            self.save_project();
            return true;
        }
        if (!ctx.text_edit_focused() || owns_text)
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.cancel_ik_edit();
            ctx.memory_mut(|memory| memory.surrender_focus(control));
            return true;
        }
        false
    }
    fn ik_mix_response(&mut self, response: &egui::Response, percent: f64, numeric: bool) {
        if self.ik_blocked.is_some() {
            return;
        }
        let starts =
            response.gained_focus() && numeric || response.drag_started() || response.changed();
        if starts
            && self
                .ik_edit
                .as_ref()
                .is_none_or(|edit| edit.control != response.id)
        {
            self.finish_ik_edit();
            if let Err(error) = self.begin_ik_edit(response.id, numeric && response.has_focus()) {
                self.status = error;
                return;
            }
        }
        if self
            .ik_edit
            .as_ref()
            .is_some_and(|edit| edit.control == response.id)
        {
            if response.changed()
                && self
                    .ik_edit
                    .as_ref()
                    .is_some_and(|edit| (!edit.numeric || edit.touched) && edit.percent != percent)
            {
                self.preview_ik_mix(percent);
            }
            if response.lost_focus()
                || (!(numeric && response.has_focus())
                    && !response.ctx.input(|input| input.pointer.any_down()))
            {
                self.ik_edit.as_mut().unwrap().finish_pending = true;
            }
        }
    }
    fn ik_bone_row(&mut self, ui: &mut egui::Ui, index: usize, label: &str, bone: Option<usize>) {
        ui.label(label);
        ui.horizontal(|ui| {
            if let Some(bone) = bone {
                icons::cue(
                    ui,
                    if label == "Target" {
                        icons::Icon::TargetBone
                    } else {
                        icons::Icon::Bone
                    },
                );
                let link = ui.link(&self.session.document().bones[bone].name);
                self.record_target(&format!("ik{label}"), link.rect);
                if link.clicked() {
                    self.select(TreeItem::Bone(bone));
                }
            } else {
                ui.label(" ");
            }
            if label == "Target" {
                let choose = icons::toolbar_button(
                    ui,
                    icons::Icon::ChooseBone,
                    "Choose target bone",
                    "",
                    self.setup_mode,
                );
                self.record_target("ikTargetChoose", choose.rect);
                if choose.clicked() {
                    self.begin_ik_target(index);
                }
            }
        });
        ui.end_row();
    }
    fn ik_positive_control(&mut self, ui: &mut egui::Ui, index: usize, ik: &IkConstraint) {
        let label = ui.label("Options");
        self.record_target("ikOptions", label.rect);
        let mut positive = ik.setup.bend_positive;
        let response = ui.add_enabled(
            self.setup_mode,
            egui::Checkbox::new(&mut positive, "Positive"),
        );
        self.record_target("ikPositive", response.rect);
        if response.changed() {
            self.cancel_gesture();
            self.weights.bind = None;
            if let Some(mut value) = self.ik_constraint(index).cloned() {
                value.setup.bend_positive = positive;
                if self.dispatch(Command::SetIkConstraint {
                    constraint: index,
                    value,
                }) {
                    self.status = "IK bend direction updated".into();
                    self.ik_evaluation_status();
                }
            }
        }
        ui.end_row();
    }
    fn ik_mix_controls(&mut self, ui: &mut egui::Ui, index: usize, mix: f32) {
        if let Some(control) = self.ik_blocked {
            ui.memory_mut(|memory| memory.surrender_focus(control));
        }
        let mut percent = self
            .ik_edit
            .as_ref()
            .map_or(f64::from(mix) * 100., |edit| edit.percent);
        ui.label("Mix");
        ui.push_id(("ik-mix", index), |ui| {
            ui.add_enabled_ui(self.setup_mode && self.ik_target.is_none(), |ui| {
                ui.horizontal(|ui| {
                    let value = ui.add(
                        egui::DragValue::new(&mut percent)
                            .range(0.0..=100.0)
                            .fixed_decimals(2)
                            .suffix("%")
                            .speed(0.2),
                    );
                    self.record_target("ikMixValue", value.rect);
                    self.ik_mix_response(&value, percent, true);
                    let slider =
                        ui.add(egui::Slider::new(&mut percent, 0.0..=100.0).show_value(false));
                    self.record_target("ikMixSlider", slider.rect);
                    self.ik_mix_response(&slider, percent, false);
                });
            });
        });
        ui.end_row();
    }
    pub(super) fn ik_properties(&mut self, ui: &mut egui::Ui, index: usize) {
        self.reconcile_ik();
        let Some(ik) = self.ik_constraint(index).cloned() else {
            return;
        };
        egui::Grid::new(("ik-properties", index))
            .num_columns(2)
            .min_col_width(58.)
            .spacing(vec2(6., 3.))
            .striped(true)
            .show(ui, |ui| {
                self.ik_bone_row(ui, index, "Parent", ik.bones.first().copied());
                self.ik_bone_row(ui, index, "Child", ik.bones.get(1).copied());
                self.ik_bone_row(ui, index, "Target", Some(ik.target));
                if ik.bones.len() == 2 {
                    self.ik_positive_control(ui, index, &ik);
                }
                self.ik_mix_controls(ui, index, ik.setup.mix);
            });
        if !self.setup_mode {
            ui.label(
                RichText::new("Setup settings · IK constraint keys are not supported yet.")
                    .small()
                    .color(MUTED),
            );
        }
        if let Some(error) = self.ik_edit.as_ref().and_then(|edit| edit.error.as_ref()) {
            ui.label(RichText::new(error).small().color(ACCENT));
        }
        if let Err(error) = self.session.evaluated_pose(self.weights_edit_target()) {
            ui.label(
                RichText::new(format!("Pose cannot be evaluated: {error}"))
                    .small()
                    .color(ACCENT),
            );
            self.evaluation_error = Some(error);
        }
    }
    pub(super) fn settle_ik_edit(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.focused)
            || self.error.is_some()
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
            || egui::Popup::is_any_open(ctx)
        {
            self.cancel_ik_edit();
            self.ik_target = None;
        }
        self.reconcile_ik();
        if !ctx.input(|input| input.pointer.any_down()) {
            if self
                .ik_edit
                .as_ref()
                .is_some_and(|edit| edit.finish_pending)
            {
                self.finish_ik_edit();
            }
            self.ik_blocked = None;
        }
    }
    pub(super) fn ik_diagnostic(&self) -> serde_json::Value {
        serde_json::json!({"selected":self.selected_ik(),"targetPicker":self.ik_target.as_ref().map(|picker|picker.constraint),
            "edit":self.ik_edit.as_ref().map(|edit|serde_json::json!({"constraint":edit.constraint,"percent":edit.percent,
                "animation":edit.animation,"time":edit.time,"numeric":edit.numeric,"finishPending":edit.finish_pending,"error":edit.error}))})
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{
        editor_context, editor_fixture, mesh_editor, workspace_frame,
    };
    use super::*;

    fn install_ik(editor: &mut Editor, two: bool, mix: f32) -> usize {
        let mut document = editor.session.document().clone();
        let target = document.bones.len();
        document.bones.push(Bone {
            name: "ik-target".into(),
            parent: Some(0),
            length: 0.,
            setup: Transform {
                x: 130.,
                y: 200.,
                ..Default::default()
            },
        });
        document.constraints.push(Constraint::Ik(IkConstraint {
            name: "hand IK".into(),
            bones: if two { vec![7, 8] } else { vec![8] },
            target,
            setup: IkSetup {
                mix,
                bend_positive: true,
            },
        }));
        editor.session = Session::new(document).unwrap();
        editor.setup_mode = true;
        editor.select(TreeItem::Constraint(0));
        target
    }
    fn one_bone_editor(rotation: f32, setup: bool) -> Editor {
        let mut editor = editor_fixture();
        let mut document = editor.session.document().clone();
        document.bones = vec![
            Bone {
                name: "root".into(),
                parent: None,
                length: 0.,
                setup: Transform::default(),
            },
            Bone {
                name: "arm".into(),
                parent: Some(0),
                length: 10.,
                setup: Transform {
                    rotation,
                    ..Default::default()
                },
            },
            Bone {
                name: "target".into(),
                parent: Some(0),
                length: 0.,
                setup: Transform {
                    y: 10.,
                    ..Default::default()
                },
            },
        ];
        document.slots.clear();
        document.skins = BTreeMap::from([("default".into(), BTreeMap::new())]);
        document.animations = vec![Animation {
            name: "pose".into(),
            tracks: vec![],
            physics: vec![],
            physics_resets: vec![],
        }];
        document.constraints = vec![Constraint::Ik(IkConstraint {
            name: "arm IK".into(),
            bones: vec![1],
            target: 2,
            setup: IkSetup {
                mix: 0.5,
                bend_positive: true,
            },
        })];
        editor.session = Session::new(document).unwrap();
        editor.skin = "default".into();
        editor.tree = TreeState::for_document(editor.session.document(), &editor.skin);
        editor.selection = Some(TreeItem::Bone(1));
        editor.setup_mode = setup;
        editor.auto_key = false;
        editor.time = 0.;
        editor.set_axes(Axes::Local);
        editor
    }
    fn point(editor: &Editor, target: &str) -> Pos2 {
        let [left, top, right, bottom] = *editor.capture_targets.get(target).unwrap_or_else(|| {
            panic!(
                "Missing UI target {target}; available: {:?}",
                editor.capture_targets.keys().collect::<Vec<_>>()
            )
        });
        pos2((left + right) / 2., (top + bottom) / 2.)
    }
    fn pointer(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }
    fn click(editor: &mut Editor, ctx: &egui::Context, target: &str) {
        let pos = point(editor, target);
        workspace_frame(
            editor,
            ctx,
            vec![egui::Event::PointerMoved(pos), pointer(pos, true)],
        );
        workspace_frame(editor, ctx, vec![pointer(pos, false)]);
    }
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
    #[test]
    fn actual_workspace_mix_entry_is_one_undo_and_animate_controls_are_read_only() {
        let mut editor = editor_fixture();
        install_ik(&mut editor, true, 0.);
        let original = editor.session.document().clone();
        let ctx = editor_context();
        workspace_frame(&mut editor, &ctx, vec![]);
        let [left, top, right, bottom] = editor.capture_targets["properties"];
        let clip = egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom));
        for target in [
            "ikHeader",
            "ikOptions",
            "ikParent",
            "ikChild",
            "ikTarget",
            "ikTargetChoose",
            "ikPositive",
            "ikMixSlider",
            "ikMixValue",
        ] {
            let [left, top, right, bottom] = editor.capture_targets[target];
            assert!(
                clip.contains_rect(egui::Rect::from_min_max(
                    pos2(left, top),
                    pos2(right, bottom)
                )),
                "{target} must be inside Properties clip"
            );
        }
        assert!(
            editor.capture_targets["ikMixValue"][2] <= editor.capture_targets["ikMixSlider"][0]
        );
        assert_eq!(
            editor.capture_targets["ikParent"][0],
            editor.capture_targets["ikChild"][0]
        );
        assert_eq!(
            editor.capture_targets["ikParent"][0],
            editor.capture_targets["ikTarget"][0]
        );
        click(&mut editor, &ctx, "ikMixValue");
        workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("37.5".into())]);
        workspace_frame(&mut editor, &ctx, vec![]);
        assert!(editor.ik_edit.is_some());
        assert_eq!(editor.ik_constraint(0).unwrap().setup.mix, 0.375);
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::Enter, egui::Modifiers::NONE),
        );
        assert!(editor.ik_edit.is_none());
        let edited = editor.session.document().clone();
        assert!(editor.session.is_dirty());
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::Z, egui::Modifiers::COMMAND),
        );
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
        workspace_frame(
            &mut editor,
            &ctx,
            key(
                egui::Key::Z,
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
            ),
        );
        assert_eq!(editor.session.document(), &edited);
        editor.set_setup_mode(false);
        workspace_frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "ikMixSlider");
        click(&mut editor, &ctx, "ikPositive");
        assert!(editor.ik_edit.is_none());
        assert_eq!(editor.session.document(), &edited);
        assert!(editor.session.pose_edits().is_empty());
        assert!(editor.selected_bone().is_none());
        editor.nudge_transform([1., 0.], false);
        assert_eq!(editor.session.document(), &edited);
    }
    #[test]
    fn target_picker_cancels_mix_preview_preserves_pending_and_consumes_only_explicit_bones() {
        let mut editor = editor_fixture();
        let target = install_ik(&mut editor, true, 0.);
        let mut pose = editor.session.local_pose(Some(0), editor.time)[2];
        pose.rotation += 17.;
        editor
            .session
            .edit_pose(
                2,
                EditTarget::Animate {
                    animation: 0,
                    time: editor.time,
                },
                pose,
                &[Property::Rotation],
                false,
            )
            .unwrap();
        let pending = editor.session.pose_edits().clone();
        let original = editor.session.document().clone();
        let ctx = editor_context();
        workspace_frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "ikMixValue");
        workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("50".into())]);
        click(&mut editor, &ctx, "ikTargetChoose");
        assert!(editor.ik_edit.is_none());
        assert!(editor.ik_target.is_some());
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        editor.select(TreeItem::Skin("peach".into()));
        assert!(editor.ik_target.is_some());
        assert_eq!(editor.selection, Some(TreeItem::Constraint(0)));
        editor.select(TreeItem::Bone(0));
        assert!(editor.ik_target.is_none());
        assert_eq!(editor.ik_constraint(0).unwrap().target, 0);
        assert_eq!(editor.selection, Some(TreeItem::Constraint(0)));
        assert_eq!(editor.session.pose_edits(), &pending);
        editor.undo();
        assert_eq!(editor.ik_constraint(0).unwrap().target, target);
        assert_eq!(editor.session.pose_edits(), &pending);
        editor.begin_ik_target(0);
        editor.select(TreeItem::Bone(8));
        assert!(editor.ik_target.is_none());
        assert!(editor.error.is_some());
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
    }
    #[test]
    fn direct_and_ik_text_previews_cancel_before_capturing_the_other_baseline() {
        let (mut editor, attachment) = mesh_editor(true);
        install_ik(&mut editor, false, 0.);
        editor.weights.select_mesh(attachment);
        editor.weights.vertex = Some(0);
        editor.weights.bone = Some(0);
        editor.set_tool(Tool::Weights);
        editor.show_weights = true;
        let original = editor.session.document().clone();
        let ctx = editor_context();
        workspace_frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "weightsWeightValue");
        workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("60".into())]);
        assert!(editor.weight_edit.is_some());
        assert_ne!(editor.session.document(), &original);
        click(&mut editor, &ctx, "ikMixValue");
        workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("40".into())]);
        assert!(editor.weight_edit.is_none());
        assert!(editor.ik_edit.is_some());
        let mut only_mix = original.clone();
        let Constraint::Ik(ik) = &mut only_mix.constraints[0] else {
            unreachable!("fixture constraint is IK")
        };
        ik.setup.mix = 0.4;
        assert_eq!(editor.session.document(), &only_mix);
        click(&mut editor, &ctx, "weightsWeightValue");
        workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("50".into())]);
        assert!(editor.ik_edit.is_none());
        assert!(editor.weight_edit.is_some());
        assert_eq!(editor.ik_constraint(0).unwrap().setup.mix, 0.);
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::Escape, egui::Modifiers::NONE),
        );
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
    }

    #[test]
    fn rounded_invalid_or_cancelled_mix_text_preserves_bits_pending_and_redo() {
        let mut editor = editor_fixture();
        install_ik(&mut editor, false, 0.33333334);
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "redo sentinel".into(),
            })
            .unwrap();
        editor.undo();
        let original = editor.session.document().clone();
        let ctx = editor_context();
        workspace_frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "ikMixValue");
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::Tab, egui::Modifiers::NONE),
        );
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
        click(&mut editor, &ctx, "ikMixValue");
        workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("junk".into())]);
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::Enter, egui::Modifiers::NONE),
        );
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
        click(&mut editor, &ctx, "ikMixValue");
        workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("75".into())]);
        let mut stale = key(egui::Key::Escape, egui::Modifiers::NONE);
        stale.extend(key(egui::Key::Enter, egui::Modifiers::NONE));
        workspace_frame(&mut editor, &ctx, stale);
        assert!(editor.ik_edit.is_none());
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
        click(&mut editor, &ctx, "ikMixValue");
        workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("80".into())]);
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
                focused: false,
                events: vec![egui::Event::WindowFocused(false)],
                ..Default::default()
            },
            |ui| editor.workspace_ui(ui),
        );
        output.textures_delta.clear();
        assert!(editor.ik_edit.is_none());
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.can_redo());
    }

    #[test]
    fn held_mix_slider_escape_and_save_cannot_restart_the_cancelled_preview() {
        let mut editor = editor_fixture();
        install_ik(&mut editor, true, 0.);
        let original = editor.session.document().clone();
        let ctx = editor_context();
        workspace_frame(&mut editor, &ctx, vec![]);
        let folder = std::env::temp_dir().join(format!(
            "peachy-ik-save-{}-{}",
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
        for action in [egui::Key::Escape, egui::Key::S] {
            let start = point(&editor, "ikMixSlider");
            workspace_frame(
                &mut editor,
                &ctx,
                vec![egui::Event::PointerMoved(start), pointer(start, true)],
            );
            assert!(editor.ik_edit.is_some());
            assert_ne!(editor.session.document(), &original);
            workspace_frame(
                &mut editor,
                &ctx,
                key(
                    action,
                    if action == egui::Key::S {
                        egui::Modifiers::COMMAND
                    } else {
                        egui::Modifiers::NONE
                    },
                ),
            );
            assert!(editor.ik_edit.is_none());
            assert_eq!(editor.session.document(), &original);
            let [_, top, right, bottom] = editor.capture_targets["ikMixSlider"];
            let end = pos2(right - 2., (top + bottom) / 2.);
            workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
            workspace_frame(&mut editor, &ctx, vec![pointer(end, false)]);
            assert_eq!(editor.session.document(), &original);
            assert!(!editor.session.can_undo());
        }
        assert_eq!(files::load(&folder.join("rig.peachy")).unwrap(), original);
        assert!(!editor.session.is_dirty());
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn inspector_first_checked_error_never_substitutes_fk_or_changes_authored_state() {
        let mut editor = editor_fixture();
        install_ik(&mut editor, true, 1.);
        let mut document = editor.session.document().clone();
        document.bones[0].setup.scale_x = 2.;
        editor.session = Session::new(document).unwrap();
        editor.select(TreeItem::Bone(8));
        let mut local = editor.session.local_pose(Some(0), editor.time)[2];
        local.rotation += 17.;
        editor
            .session
            .edit_pose(
                2,
                EditTarget::Animate {
                    animation: 0,
                    time: editor.time,
                },
                local,
                &[Property::Rotation],
                false,
            )
            .unwrap();
        let original = editor.session.document().clone();
        let pending = editor.session.pose_edits().clone();
        let history = (editor.session.can_undo(), editor.session.can_redo());
        let ctx = editor_context();
        let mut output = ctx.run_ui(
            egui::RawInput {
                focused: true,
                ..Default::default()
            },
            |ui| editor.main_tools(ui),
        );
        output.textures_delta.clear();
        assert!(editor.evaluation_error.is_some());
        assert!(!editor.capture_targets.contains_key("value:Rotation"));
        assert!(editor.selected_pose().is_err());
        assert_eq!(
            editor.selected_transform().unwrap(),
            original.bones[8].setup
        );
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(
            (editor.session.can_undo(), editor.session.can_redo()),
            history
        );
    }

    #[test]
    fn applied_axes_tree_links_and_animated_targets_keep_authored_values_separate() {
        let mut editor = editor_fixture();
        let target = install_ik(&mut editor, true, 0.5);
        let mut document = editor.session.document().clone();
        document.animations[0].tracks.push(Track {
            bone: target,
            property: Property::X,
            keys: vec![
                Keyframe {
                    time: 0.,
                    value: 0.,
                    curve: Curve::Linear,
                },
                Keyframe {
                    time: 1.,
                    value: 40.,
                    curve: Curve::Linear,
                },
            ],
        });
        editor.session = Session::new(document).unwrap();
        let setup = editor.session.evaluated_pose(EditTarget::Setup).unwrap();
        assert_ne!(
            setup.authored_locals()[8].rotation,
            setup.applied_locals()[8].rotation
        );
        editor.select(TreeItem::Bone(8));
        editor.set_tool(Tool::Scale);
        let ctx = editor_context();
        workspace_frame(&mut editor, &ctx, vec![]);
        let origin = point(&editor, "gizmoOrigin");
        let tip = point(&editor, "gizmo:X");
        let world = setup.world()[8];
        let norm = world.a.hypot(world.c);
        assert!(((tip.x - origin.x) / 56. - world.a / norm).abs() < 1e-4);
        assert!(((tip.y - origin.y) / 56. + world.c / norm).abs() < 1e-4);
        let fk = peachy_engine::world_pose(editor.session.document(), setup.authored_locals());
        assert!((fk[8].a - world.a).abs() + (fk[8].c - world.c).abs() > 0.1);
        editor.begin_viewport_transform(world.point(20., 0.), Some(0));
        let capsule = editor.transform_gesture_diagnostic();
        assert_eq!(
            capsule["initialPose"],
            serde_json::to_value(setup.authored_locals()[8]).unwrap()
        );
        assert_eq!(
            capsule["appliedInitialPose"],
            serde_json::to_value(setup.applied_locals()[8]).unwrap()
        );
        editor.cancel_gesture();

        editor.tree.query = "ik".into();
        workspace_frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, &format!("ikAnnotation:{target}:0"));
        assert_eq!(editor.selection, Some(TreeItem::Constraint(0)));
        assert!(editor.selected_bone().is_none());
        // Properties renders before Tree actions; settle the new selection's panel.
        workspace_frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "ikTarget");
        assert_eq!(editor.selected_bone(), Some(target));
        editor.set_setup_mode(false);
        editor.set_time(0.25);
        editor.set_tool(Tool::Move);
        let original = editor.session.document().clone();
        let before = editor
            .session
            .evaluated_pose(editor.weights_edit_target())
            .unwrap();
        assert_eq!(before.authored_locals()[target].x, 140.);
        editor.nudge_transform([3., 0.], false);
        let after = editor
            .session
            .evaluated_pose(editor.weights_edit_target())
            .unwrap();
        assert_eq!(after.authored_locals()[target].x, 143.);
        let length = editor.session.document().bones[8].length;
        assert_ne!(
            before.world()[8].point(length, 0.),
            after.world()[8].point(length, 0.)
        );
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.pose_edits().has_bone(0, target));
        editor.undo();
        assert_eq!(
            editor.session.local_pose(Some(0), editor.time),
            before.authored_locals()
        );
        assert_eq!(editor.session.document(), &original);
    }

    #[test]
    fn tool_letters_work_after_target_links_but_text_modals_and_widget_navigation_keep_priority() {
        let mut editor = editor_fixture();
        let target = install_ik(&mut editor, true, 0.);
        let original = editor.session.document().clone();
        let ctx = editor_context();
        for (letter, tool) in [
            (egui::Key::V, Tool::Move),
            (egui::Key::C, Tool::Rotate),
            (egui::Key::X, Tool::Scale),
            (egui::Key::Z, Tool::Shear),
            (egui::Key::G, Tool::Weights),
        ] {
            editor.select(TreeItem::Constraint(0));
            workspace_frame(&mut editor, &ctx, vec![]);
            click(&mut editor, &ctx, "ikTarget");
            assert_eq!(editor.selected_bone(), Some(target));
            assert!(!ctx.text_edit_focused());
            workspace_frame(&mut editor, &ctx, key(letter, egui::Modifiers::NONE));
            assert!(editor.tool == tool);
            assert_eq!(editor.selected_bone(), Some(target));
            assert_eq!(editor.session.document(), &original);
        }
        editor.set_tool(Tool::Scale);
        workspace_frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "value:ShearY");
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::Tab, egui::Modifiers::NONE),
        );
        // Mouse clicks need not retain keyboard focus. Natural Tab traversal
        // into Local reproduces the non-text owner from native verification.
        assert!(ctx.egui_wants_keyboard_input());
        assert!(!ctx.text_edit_focused());
        let focus = ctx.memory(|memory| memory.focused()).unwrap();
        assert!(
            ctx.read_response(focus)
                .unwrap()
                .rect
                .contains(point(&editor, "axes:Local"))
        );
        workspace_frame(&mut editor, &ctx, key(egui::Key::C, egui::Modifiers::NONE));
        assert!(editor.tool == Tool::Rotate);
        for letter in [egui::Key::ArrowRight, egui::Key::Space, egui::Key::Enter] {
            workspace_frame(&mut editor, &ctx, key(letter, egui::Modifiers::NONE));
            assert_eq!(editor.selection, Some(TreeItem::Bone(target)));
            assert_eq!(editor.session.document(), &original);
            assert!(editor.session.pose_edits().is_empty());
        }
        let after_widget_activation = editor.tool;
        workspace_frame(&mut editor, &ctx, key(egui::Key::V, egui::Modifiers::SHIFT));
        assert!(editor.tool == after_widget_activation);
        editor.set_tool(Tool::Rotate);
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "history sentinel".into(),
            })
            .unwrap();
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::Z, egui::Modifiers::COMMAND),
        );
        assert!(editor.tool == Tool::Rotate);
        assert_eq!(editor.session.document(), &original);
        let popup = egui::Id::new("ik-tool-priority-popup");
        egui::Popup::open_id(&ctx, popup);
        workspace_frame(&mut editor, &ctx, key(egui::Key::Z, egui::Modifiers::NONE));
        assert!(editor.tool == Tool::Rotate);
        egui::Popup::close_id(&ctx, popup);
        let mut ordered = key(egui::Key::G, egui::Modifiers::NONE);
        ordered.extend(key(egui::Key::C, egui::Modifiers::NONE));
        workspace_frame(&mut editor, &ctx, ordered);
        assert!(editor.tool == Tool::Rotate);
        assert!(editor.last_tool == Tool::Weights);
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::C,
                physical_key: None,
                pressed: true,
                repeat: true,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(editor.last_tool == Tool::Weights);
        editor.select(TreeItem::Constraint(0));
        workspace_frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "ikTargetChoose");
        assert!(editor.ik_target.is_some());
        workspace_frame(&mut editor, &ctx, key(egui::Key::V, egui::Modifiers::NONE));
        assert!(editor.ik_target.is_none());
        assert!(editor.tool == Tool::Move);
        assert_eq!(editor.session.document(), &original);
        editor.set_tool(Tool::Rotate);
        workspace_frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "ikMixValue");
        assert!(ctx.text_edit_focused());
        let mut letters = Vec::new();
        for letter in [
            egui::Key::C,
            egui::Key::V,
            egui::Key::X,
            egui::Key::Z,
            egui::Key::G,
        ] {
            letters.extend(key(letter, egui::Modifiers::NONE));
        }
        letters.push(egui::Event::Text("cvxzg".into()));
        workspace_frame(&mut editor, &ctx, letters);
        assert!(editor.tool == Tool::Rotate);
        assert_eq!(editor.session.document(), &original);
        assert!(editor.ik_edit.is_some());
    }

    #[test]
    fn tool_switching_cancels_held_mix_direct_and_transform_previews_without_restart() {
        let mut editor = editor_fixture();
        install_ik(&mut editor, true, 0.);
        let original = editor.session.document().clone();
        let ctx = editor_context();
        workspace_frame(&mut editor, &ctx, vec![]);
        let start = point(&editor, "ikMixSlider");
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
        );
        assert!(editor.ik_edit.is_some());
        workspace_frame(&mut editor, &ctx, key(egui::Key::C, egui::Modifiers::NONE));
        assert!(editor.ik_edit.is_none());
        assert_eq!(editor.session.document(), &original);
        let end = start + vec2(45., 0.);
        workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
        workspace_frame(&mut editor, &ctx, vec![pointer(end, false)]);
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());

        let (mut editor, attachment) = mesh_editor(true);
        install_ik(&mut editor, false, 0.);
        editor.weights.select_mesh(attachment);
        editor.weights.vertex = Some(0);
        editor.weights.bone = Some(0);
        editor.set_tool(Tool::Weights);
        let original = editor.session.document().clone();
        let ctx = editor_context();
        workspace_frame(&mut editor, &ctx, vec![]);
        let start = point(&editor, "weightsWeightSlider");
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
        );
        assert!(editor.weight_edit.is_some());
        workspace_frame(&mut editor, &ctx, key(egui::Key::V, egui::Modifiers::NONE));
        assert!(editor.tool == Tool::Move);
        assert!(editor.weight_edit.is_none());
        let end = start + vec2(40., 0.);
        workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
        workspace_frame(&mut editor, &ctx, vec![pointer(end, false)]);
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());

        let mut editor = editor_fixture();
        editor.set_setup_mode(true);
        editor.set_tool(Tool::Move);
        let original = editor.session.document().clone();
        let ctx = editor_context();
        workspace_frame(&mut editor, &ctx, vec![]);
        let start = point(&editor, "gizmo:X");
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
        );
        let moved = start + vec2(30., 0.);
        workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(moved)]);
        assert!(editor.drag.is_some());
        assert_ne!(editor.session.document(), &original);
        workspace_frame(&mut editor, &ctx, key(egui::Key::X, egui::Modifiers::NONE));
        assert!(editor.tool == Tool::Scale);
        assert!(editor.drag.is_none());
        let end = moved + vec2(30., 0.);
        workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
        workspace_frame(&mut editor, &ctx, vec![pointer(end, false)]);
        assert_eq!(editor.session.document(), &original);
        assert!(editor.session.pose_edits().is_empty());
        assert!(!editor.session.can_undo());
    }

    #[test]
    fn setup_ik_rotation_relative_entry_and_viewport_preserve_semantic_raw_turns() {
        fn has_text(shape: &egui::Shape, text: &str) -> bool {
            match shape {
                egui::Shape::Text(shape) => shape.galley.text() == text,
                egui::Shape::Vec(shapes) => shapes.iter().any(|shape| has_text(shape, text)),
                _ => false,
            }
        }
        fn displays(editor: &mut Editor, ctx: &egui::Context, text: &str) -> bool {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, vec2(1500., 960.))),
                    focused: true,
                    ..Default::default()
                },
                |ui| editor.workspace_ui(ui),
            );
            let found = output
                .shapes
                .iter()
                .any(|shape| has_text(&shape.shape, text));
            output.textures_delta.clear();
            found
        }
        let mut editor = one_bone_editor(720., true);
        editor.set_tool(Tool::Rotate);
        let original = editor.session.document().clone();
        let ctx = editor_context();
        let initial_display = displays(&mut editor, &ctx, "720.00");
        let initial = editor.session.evaluated_pose(EditTarget::Setup).unwrap();
        assert!((initial.applied_locals()[1].rotation - 585.).abs() < 1e-3);
        click(&mut editor, &ctx, "value:Rotation");
        workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("+10".into())]);
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::Enter, egui::Modifiers::NONE),
        );
        assert_eq!(editor.session.document().bones[1].setup.rotation, 730.);
        assert!(
            initial_display,
            "The Setup field must display the authored rotation of 720 degrees"
        );
        assert!(displays(&mut editor, &ctx, "730.00"));
        let numeric = editor.session.evaluated_pose(EditTarget::Setup).unwrap();
        assert!((numeric.applied_locals()[1].rotation - 590.).abs() < 1e-3);
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::Z, egui::Modifiers::COMMAND),
        );
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());

        workspace_frame(&mut editor, &ctx, vec![]);
        let [left, top, right, bottom] = editor.capture_targets["viewport"];
        let origin = pos2((left + right) / 2., (top + bottom) / 2.) + editor.pan;
        let world = initial.world()[1];
        let start_world = world.point(30., 0.);
        let end_world = [-start_world[1], start_world[0]];
        let start = scene_view::screen_point(origin, editor.zoom, start_world);
        let end = scene_view::screen_point(origin, editor.zoom, end_world);
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
        );
        workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
        assert!(editor.drag.is_some());
        workspace_frame(&mut editor, &ctx, vec![pointer(end, false)]);
        assert!((editor.session.document().bones[1].setup.rotation - 810.).abs() < 1e-3);
        assert!(displays(&mut editor, &ctx, "810.00"));
        let rotated = editor.session.evaluated_pose(EditTarget::Setup).unwrap();
        assert!((rotated.applied_locals()[1].rotation - 630.).abs() < 1e-3);
        assert!(editor.session.pose_edits().is_empty());
        assert!(editor.session.document().animations[0].tracks.is_empty());
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::Z, egui::Modifiers::COMMAND),
        );
        assert_eq!(editor.session.document(), &original);
        workspace_frame(
            &mut editor,
            &ctx,
            key(
                egui::Key::Z,
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
            ),
        );
        assert!((editor.session.document().bones[1].setup.rotation - 810.).abs() < 1e-3);

        // Arrow nudging reaches the ordinary edit_display_value mutation path,
        // while numeric and viewport edits above use fixed-base previews.
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::ArrowRight, egui::Modifiers::NONE),
        );
        assert!((editor.session.document().bones[1].setup.rotation - 811.).abs() < 1e-3);
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::Z, egui::Modifiers::COMMAND),
        );
        assert!((editor.session.document().bones[1].setup.rotation - 810.).abs() < 1e-3);

        let mut fk = one_bone_editor(720., true);
        let mut document = fk.session.document().clone();
        document.constraints.clear();
        fk.session = Session::new(document).unwrap();
        let ctx = editor_context();
        assert!(displays(&mut fk, &ctx, "0.00"));
        click(&mut fk, &ctx, "value:Rotation");
        workspace_frame(&mut fk, &ctx, vec![egui::Event::Text("+10".into())]);
        workspace_frame(&mut fk, &ctx, key(egui::Key::Enter, egui::Modifiers::NONE));
        assert_eq!(fk.session.document().bones[1].setup.rotation, 10.);
    }

    #[test]
    fn actual_partial_ik_local_move_handle_uses_its_displayed_frame_and_one_pending_undo() {
        let mut editor = one_bone_editor(0., false);
        editor.set_tool(Tool::Move);
        let original = editor.session.document().clone();
        let ctx = editor_context();
        workspace_frame(&mut editor, &ctx, vec![]);
        let origin = point(&editor, "gizmoOrigin");
        let start = point(&editor, "gizmo:X");
        let direction = (start - origin).normalized();
        assert!(direction.x > 0.7 && direction.y < -0.7);
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
        );
        let end = start + direction * 30.;
        workspace_frame(&mut editor, &ctx, vec![egui::Event::PointerMoved(end)]);
        assert!(editor.drag.is_some());
        let local = editor.session.local_pose(Some(0), 0.)[1];
        let expected = 30. / 2f32.sqrt();
        assert!((local.x - expected).abs() < 1e-3);
        assert!((local.y - expected).abs() < 1e-3);
        workspace_frame(&mut editor, &ctx, vec![pointer(end, false)]);
        assert!(editor.drag.is_none());
        assert!(editor.evaluation_error.is_none());
        assert_eq!(editor.session.document(), &original);
        let pending = editor.session.pose_edits().clone();
        assert!(pending.has_bone(0, 1));
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::Z, egui::Modifiers::COMMAND),
        );
        assert!(editor.session.pose_edits().is_empty());
        assert!(!editor.session.can_undo());
        workspace_frame(
            &mut editor,
            &ctx,
            key(
                egui::Key::Z,
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
            ),
        );
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(editor.session.document(), &original);
    }
}
