//! Native controls translate input into shared engine transform operations.
use super::*;
use peachy_engine::{set_transform_value, transform_values};

pub(super) struct SelectedPose {
    pub authored: Transform,
    pub applied: Transform,
    pub parent: Affine,
    pub world: Affine,
}

impl Tool {
    pub(super) fn transform_tool(self) -> Option<TransformTool> {
        match self {
            Self::Rotate => Some(TransformTool::Rotate),
            Self::Move => Some(TransformTool::Translate),
            Self::Scale => Some(TransformTool::Scale),
            Self::Shear => Some(TransformTool::Shear),
            Self::Mesh | Self::Weights => None,
        }
    }
    pub(super) fn properties(self) -> &'static [Property] {
        match self {
            Self::Rotate => &[Property::Rotation],
            Self::Move => &[Property::X, Property::Y],
            Self::Scale => &[Property::ScaleX, Property::ScaleY],
            Self::Shear => &[Property::ShearX, Property::ShearY],
            Self::Mesh | Self::Weights => &[],
        }
    }
}

pub(super) fn item_parent(item: &TreeItem, doc: &Document, world: &[Affine]) -> Option<Affine> {
    let identity = Affine::from_transform(Transform::default());
    match item {
        TreeItem::Bone(b) => Some(doc.bones.get(*b)?.parent.map_or(identity, |p| world[p])),
        TreeItem::Attachment(k) => Some(world[doc.slots.iter().find(|s| s.name == k.slot)?.bone]),
        _ => None,
    }
}
impl Editor {
    /// Setup angle wrapping is a presentation convention for ordinary FK only.
    /// IK mixes depend on raw turns, including when a currently zero mix is later enabled.
    pub(super) fn setup_rotation_value(&self, item: Option<&TreeItem>, rotation: f32) -> f32 {
        let ik_bone = match item {
            Some(TreeItem::Bone(bone)) => self
                .session
                .document()
                .constraints
                .iter()
                .filter_map(Constraint::as_ik)
                .any(|ik| ik.bones.contains(bone)),
            _ => false,
        };
        if ik_bone {
            rotation
        } else {
            rotation.rem_euclid(360.)
        }
    }
    pub(super) fn selected_pose(&self) -> Result<SelectedPose, String> {
        let item = self
            .selection
            .as_ref()
            .ok_or("Select an editable transform")?;
        let authored = self
            .selected_transform()
            .ok_or("Selected item has no editable transform")?;
        let pose = self.session.evaluated_pose(self.weights_edit_target())?;
        let parent = item_parent(item, self.session.document(), pose.world())
            .ok_or("Selected parent is unavailable")?;
        let (applied, world) = match item {
            TreeItem::Bone(bone) => (pose.applied_locals()[*bone], pose.world()[*bone]),
            TreeItem::Attachment(_) => (authored, parent.then(Affine::from_transform(authored))),
            _ => return Err("Selected item has no editable transform".into()),
        };
        Ok(SelectedPose {
            authored,
            applied,
            parent,
            world,
        })
    }
    #[cfg(test)]
    pub(super) fn selected_parent(&self) -> Result<Affine, String> {
        self.selected_pose().map(|pose| pose.parent)
    }
    pub(super) fn set_axes(&mut self, axes: Axes) {
        if self.axes != axes {
            self.cancel_gesture();
            self.axes = axes;
        }
    }
    pub(super) fn edit_display_value(&mut self, property: Property, value: f32) {
        if matches!(property, Property::ShearX | Property::ShearY) && self.selected_bone().is_none()
        {
            return;
        }
        self.cancel_gesture();
        let pose = match self.selected_pose() {
            Ok(pose) => pose,
            Err(error) => {
                self.evaluation_error = Some(error.clone());
                self.status = error;
                return;
            }
        };
        let (local, parent) = (pose.authored, pose.parent);
        match set_transform_value(local, parent, self.axes, property, value) {
            Ok(transform) => {
                let properties = if matches!(property, Property::X | Property::Y) {
                    &[Property::X, Property::Y][..]
                } else {
                    std::slice::from_ref(&property)
                };
                self.edit_transform(transform, properties);
            }
            Err(error) => self.error = Some(error),
        }
    }
    pub(super) fn nudge_transform(&mut self, delta: [f32; 2], fine: bool) {
        self.cancel_gesture();
        let pose = match self.selected_pose() {
            Ok(pose) => pose,
            Err(error) => {
                self.evaluation_error = Some(error.clone());
                self.status = error;
                return;
            }
        };
        let (local, parent) = (pose.authored, pose.parent);
        let shown = transform_values(local, parent, self.axes);
        let amount = if fine { 0.1 } else { 1. };
        match self.tool {
            Tool::Rotate => self.edit_display_value(
                Property::Rotation,
                shown.rotation + (delta[0] + delta[1]) * amount,
            ),
            Tool::Move => {
                let first = if delta[0] != 0. {
                    Property::X
                } else {
                    Property::Y
                };
                self.edit_display_value(first, first.read(shown) + (delta[0] + delta[1]) * amount);
            }
            Tool::Scale => {
                let property = if delta[0] != 0. {
                    Property::ScaleX
                } else {
                    Property::ScaleY
                };
                self.edit_display_value(
                    property,
                    property.read(local) + (delta[0] + delta[1]) * amount * 0.01,
                );
            }
            Tool::Shear => {
                let property = if delta[0] != 0. {
                    Property::ShearX
                } else {
                    Property::ShearY
                };
                self.edit_display_value(
                    property,
                    property.read(local) + (delta[0] + delta[1]) * amount,
                );
            }
            Tool::Mesh | Tool::Weights => {}
        }
    }
}

#[cfg(test)]
mod shear_tests {
    use super::super::test_support::{editor_context, editor_fixture};
    use super::*;

    fn chord(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
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
    fn input(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>, text: bool) {
        let mut output = ctx.run_ui(
            egui::RawInput {
                focused: true,
                events,
                ..Default::default()
            },
            |ui| {
                editor.shortcuts(ui.ctx());
                if text {
                    let mut value = String::new();
                    ui.add(egui::TextEdit::singleline(&mut value))
                        .request_focus();
                } else {
                    editor.main_tools(ui);
                }
            },
        );
        output.textures_delta.clear();
    }
    #[test]
    fn shear_uses_local_values_and_paired_keys_without_redirecting_nonbone_selections() {
        let mut editor = editor_fixture();
        editor.set_tool(Tool::Shear);
        editor.set_axes(Axes::World);
        editor.edit_display_value(Property::ShearX, 23.5);
        editor.edit_display_value(Property::ShearY, -17.25);
        let local = editor.selected_transform().unwrap();
        assert_eq!((local.shear_x, local.shear_y), (23.5, -17.25));
        assert!(!editor.session.pose_edits().is_empty());
        assert!(
            editor.session.document().animations[0]
                .tracks
                .iter()
                .all(|track| !matches!(track.property, Property::ShearX | Property::ShearY))
        );
        editor.key_transform(Tool::Shear.properties());
        for property in [Property::ShearX, Property::ShearY] {
            assert!(
                editor.session.document().animations[0]
                    .tracks
                    .iter()
                    .any(|track| track.bone == 8 && track.property == property)
            );
        }
        editor.set_time(0.6);
        editor.auto_key = true;
        editor.edit_display_value(Property::ShearX, 35.);
        for property in [Property::ShearX, Property::ShearY] {
            assert!(
                editor.session.document().animations[0]
                    .tracks
                    .iter()
                    .find(|track| track.bone == 8 && track.property == property)
                    .unwrap()
                    .keys
                    .iter()
                    .any(|key| key.time == 0.6)
            );
        }
        editor.set_setup_mode(true);
        let region = editor
            .session
            .document()
            .resolved_attachment(&editor.skin, 0)
            .unwrap()
            .0;
        editor.select(TreeItem::Attachment(region));
        let original = editor.session.document().clone();
        editor.edit_display_value(Property::ShearX, 75.);
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.transform_tool_editable(Tool::Shear));
        editor.select(TreeItem::Slot(0));
        editor.edit_display_value(Property::ShearY, 45.);
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.selected_bone(), None);
    }
    #[test]
    fn z_respects_modified_history_text_and_modals_and_is_a_real_last_tool() {
        let mut editor = editor_fixture();
        let ctx = editor_context();
        let original = editor.session.document().clone();
        editor
            .session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "history".into(),
            })
            .unwrap();
        input(
            &mut editor,
            &ctx,
            chord(egui::Key::Z, egui::Modifiers::COMMAND),
            false,
        );
        assert_eq!(editor.session.document(), &original);
        assert!(editor.tool == Tool::Rotate);
        input(
            &mut editor,
            &ctx,
            chord(
                egui::Key::Z,
                egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
            ),
            false,
        );
        assert_eq!(editor.session.document().animations[0].name, "history");
        assert!(editor.tool == Tool::Rotate);
        input(
            &mut editor,
            &ctx,
            chord(egui::Key::Z, egui::Modifiers::NONE),
            false,
        );
        assert!(editor.tool == Tool::Shear);
        editor.toggle_transform_tool();
        assert!(editor.tool == Tool::Rotate);
        editor.toggle_transform_tool();
        assert!(editor.tool == Tool::Shear);
        editor.set_tool(Tool::Rotate);
        input(&mut editor, &ctx, vec![], true);
        input(
            &mut editor,
            &ctx,
            chord(egui::Key::Z, egui::Modifiers::NONE),
            true,
        );
        assert!(editor.tool == Tool::Rotate);
        editor.error = Some("modal".into());
        input(
            &mut editor,
            &ctx,
            chord(egui::Key::Z, egui::Modifiers::NONE),
            false,
        );
        assert!(editor.tool == Tool::Rotate);
    }
}
