//! Transform constraint properties. Values route through shared engine commands;
//! only the evaluated subset (world mode, identity Rotate/X/Y mappings) exists.
use super::*;
use peachy_document::{TransformConstraint, TransformProperty};

impl Editor {
    pub(super) fn transform_constraint(&self, index: usize) -> Option<&TransformConstraint> {
        self.session
            .document()
            .constraints
            .get(index)?
            .as_transform()
    }

    fn set_transform(&mut self, index: usize, value: TransformConstraint, status: String) {
        if !self.setup_mode {
            self.status =
                "Transform constraint settings are edited in Setup; keying them is not supported yet"
                    .into();
            return;
        }
        if self.dispatch(Command::SetTransformConstraint {
            constraint: index,
            value,
        }) {
            self.status = status;
        }
    }

    /// One numeric field: typed text applies once (one undo step); a drag is one
    /// Session transaction committed on release. Display never clamps data.
    #[allow(clippy::too_many_arguments)]
    fn transform_number(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        value: f32,
        percent: bool,
        target: String,
        label: String,
        write: impl Fn(&mut TransformConstraint, f32),
    ) {
        let mut shown = f64::from(value) * if percent { 100. } else { 1. };
        let response = ui.add_enabled(
            self.setup_mode,
            egui::DragValue::new(&mut shown)
                .range(f64::from(f32::MIN)..=f64::from(f32::MAX))
                .clamp_existing_to_range(false)
                .update_while_editing(false)
                .max_decimals(2)
                .suffix(if percent { "%" } else { "" })
                .speed(0.25),
        );
        self.record_target(&target, response.rect);
        if response.drag_started() {
            self.cancel_ik_edit();
            self.cancel_transform_edit();
            self.cancel_weight_edit();
            self.session.begin_transaction();
        }
        if response.changed()
            && let Some(mut edited) = self.transform_constraint(index).cloned()
        {
            write(
                &mut edited,
                (shown / if percent { 100. } else { 1. }) as f32,
            );
            self.set_transform(index, edited, format!("{label}: {shown:.2}"));
        }
        if response.drag_stopped() {
            self.session.commit_transaction();
        }
    }

    fn bone_link(&mut self, ui: &mut egui::Ui, bone: usize, target: &str) {
        ui.horizontal(|ui| {
            icons::cue(ui, icons::Icon::Bone);
            let link = ui.link(&self.session.document().bones[bone].name);
            self.record_target(target, link.rect);
            if link.clicked() {
                self.select(TreeItem::Bone(bone));
            }
        });
    }

    pub(super) fn transform_constraint_properties(&mut self, ui: &mut egui::Ui, index: usize) {
        let Some(transform) = self.transform_constraint(index).cloned() else {
            return;
        };
        ui.horizontal(|ui| {
            icons::cue(ui, icons::Icon::TransformConstraint);
            ui.label(RichText::new("Transform · World").color(ACCENT));
        });
        egui::Grid::new(("transform-properties", index))
            .num_columns(3)
            .min_col_width(58.)
            .spacing(vec2(6., 3.))
            .striped(true)
            .show(ui, |ui| {
                ui.label("Source");
                self.bone_link(ui, transform.source, "transformSource");
                ui.end_row();
                for (i, &bone) in transform.bones.iter().enumerate() {
                    ui.label(if i == 0 { "Bones" } else { "" });
                    self.bone_link(ui, bone, &format!("transformBone{i}"));
                    ui.end_row();
                }
                ui.label("");
                ui.label(RichText::new("Mix").small().color(MUTED));
                ui.label(RichText::new("Offset").small().color(MUTED));
                ui.end_row();
                for property in transform.targets() {
                    ui.label(property.label());
                    let name = property.spine_name();
                    self.transform_number(
                        ui,
                        index,
                        transform.mix.get(property),
                        true,
                        format!("transformMix{name}"),
                        format!("{} Mix", property.label()),
                        move |t, v| t.mix.set(property, v),
                    );
                    self.transform_number(
                        ui,
                        index,
                        transform.offsets.get(property),
                        false,
                        format!("transformOffset{name}"),
                        format!("{} Offset", property.label()),
                        move |t, v| t.offsets.set(property, v),
                    );
                    ui.end_row();
                }
            });
        if transform.targets().is_empty() {
            ui.label(
                RichText::new("No mapped properties; this constraint has no effect.")
                    .small()
                    .color(MUTED),
            );
        }
        if !self.setup_mode {
            ui.label(
                RichText::new(
                    "Setup settings · Keying transform constraints is not supported yet.",
                )
                .small()
                .color(MUTED),
            );
        }
        if let Err(error) = self.session.evaluated_pose(self.weights_edit_target()) {
            ui.label(
                RichText::new(format!("Pose cannot be evaluated: {error}"))
                    .small()
                    .color(ACCENT),
            );
        }
        let _ = TransformProperty::ALL;
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
    use super::*;
    use peachy_document::{TransformMapping, TransformTarget, TransformValues};

    fn install(editor: &mut Editor) -> Document {
        let mut document = editor.session.document().clone();
        document
            .constraints
            .push(Constraint::Transform(TransformConstraint {
                name: "hand follow".into(),
                bones: vec![5],
                source: 8,
                offsets: TransformValues {
                    x: 4.,
                    ..TransformValues::default()
                },
                mix: TransformValues {
                    x: 0.5,
                    y: 0.5,
                    rotate: 0.25,
                    ..TransformValues::default()
                },
                mappings: [
                    TransformProperty::Rotate,
                    TransformProperty::X,
                    TransformProperty::Y,
                ]
                .map(|p| TransformMapping {
                    from: p,
                    offset: 0.,
                    to: vec![TransformTarget::identity(p)],
                })
                .to_vec(),
                local_source: false,
                local_target: false,
                additive: false,
                clamp: false,
            }));
        editor.session = Session::new(document.clone()).unwrap();
        editor.setup_mode = true;
        editor.select(TreeItem::Constraint(0));
        document
    }
    fn rect(editor: &Editor, target: &str) -> egui::Rect {
        let [l, t, r, b] = editor.capture_targets[target];
        egui::Rect::from_min_max(pos2(l, t), pos2(r, b))
    }
    fn click(editor: &mut Editor, ctx: &egui::Context, target: &str) {
        let pos = rect(editor, target).center();
        for pressed in [true, false] {
            workspace_frame(
                editor,
                ctx,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
    }
    fn key(key: egui::Key) -> Vec<egui::Event> {
        [true, false]
            .into_iter()
            .map(|pressed| egui::Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            })
            .collect()
    }

    #[test]
    fn panel_lists_mapped_properties_and_typed_mix_is_one_undo_step() {
        let mut editor = editor_fixture();
        let original = install(&mut editor);
        let ctx = editor_context();
        workspace_frame(&mut editor, &ctx, vec![]);
        let clip = rect(&editor, "properties");
        for target in [
            "transformSource",
            "transformBone0",
            "transformMixrotate",
            "transformMixx",
            "transformOffsetx",
        ] {
            assert!(clip.contains_rect(rect(&editor, target)), "{target}");
        }
        assert!(!editor.capture_targets.contains_key("transformMixscaleX"));
        click(&mut editor, &ctx, "transformMixx");
        for digit in ["-", "1", "0", "0"] {
            workspace_frame(&mut editor, &ctx, vec![egui::Event::Text(digit.into())]);
        }
        assert_eq!(editor.session.document(), &original);
        workspace_frame(&mut editor, &ctx, key(egui::Key::Enter));
        workspace_frame(&mut editor, &ctx, vec![]);
        assert_eq!(editor.transform_constraint(0).unwrap().mix.x, -1.);
        // The pose reflects the edit, then one undo restores the document.
        assert!(editor.session.evaluated_pose(EditTarget::Setup).is_ok());
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        editor.set_setup_mode(false);
        workspace_frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "transformMixx");
        workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("5".into())]);
        workspace_frame(&mut editor, &ctx, key(egui::Key::Enter));
        assert_eq!(editor.session.document(), &original);
    }
}
