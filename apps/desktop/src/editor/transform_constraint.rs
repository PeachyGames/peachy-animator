//! Native Transform properties; constraint state and conversions stay shared.
use super::transform_constraint_edit::ConstraintField;
use super::*;
use peachy_engine::{TransformMappingRangeField, transform_mapping_range};

impl Editor {
    fn record_transform_target(&mut self, name: &str, rect: egui::Rect) {
        if rect.is_positive() && rect.is_finite() {
            self.record_target(name, rect);
        } else {
            self.capture_targets.remove(name);
        }
    }

    pub(super) fn transform_constraint(&self, index: usize) -> Option<&TransformConstraint> {
        self.session
            .document()
            .constraints
            .get(index)?
            .as_transform()
    }
    fn transform_bone_link(&mut self, ui: &mut egui::Ui, bone: usize, target: &str) {
        ui.horizontal(|ui| {
            icons::cue(ui, icons::Icon::Bone);
            let response = ui.link(&self.session.document().bones[bone].name);
            let visible = response.rect.intersect(ui.clip_rect());
            if visible.is_positive() {
                self.record_transform_target(target, visible);
            }
            if response.clicked() {
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
            ui.label("Transform Constraint");
        });
        egui::Grid::new(("transform-links", index))
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Source");
                self.transform_bone_link(ui, transform.source, "transformSource");
                ui.end_row();
                for (row, &bone) in transform.bones.iter().enumerate() {
                    ui.label(if row == 0 { "Targets" } else { "" });
                    self.transform_bone_link(ui, bone, &format!("transformBone{row}"));
                    ui.end_row();
                }
            });
        let offset = egui::CollapsingHeader::new("Offset")
            .id_salt(("transform-offsets", index))
            .default_open(false)
            .show(ui, |ui| {
                let cross = transform.mappings.iter().any(|mapping| {
                    mapping.from == TransformProperty::X
                        && mapping.to.len() == 1
                        && mapping.to[0].property == TransformProperty::Rotate
                });
                if cross {
                    ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
                    let scroll = egui::ScrollArea::horizontal()
                        .id_salt(("transform-offset-horizontal", index))
                        .min_scrolled_width(0.)
                        .auto_shrink([false, true])
                        .scroll_bar_visibility(
                            egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                        )
                        .show(ui, |ui| {
                            ui.set_width(435.);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.;
                                ui.add_sized([85., 22.], egui::Label::new("Translate"));
                                for property in [TransformProperty::X, TransformProperty::Y] {
                                    self.constraint_number(
                                        ui,
                                        index,
                                        ConstraintField::Offset(property),
                                        Some(f64::from(transform.offsets.get(property))),
                                        &format!("transformOffset{}", property.spine_name()),
                                        60.,
                                    );
                                    self.constraint_offset_slider(
                                        ui,
                                        index,
                                        property,
                                        transform.offsets.get(property),
                                    );
                                }
                            });
                        });
                    self.record_transform_target(
                        "transformOffsetScroll",
                        scroll.inner_rect.intersect(ui.clip_rect()),
                    );
                } else {
                    for property in transform.targets() {
                        ui.horizontal(|ui| {
                            ui.add_sized([85., 22.], egui::Label::new(property.label()));
                            self.constraint_number(
                                ui,
                                index,
                                ConstraintField::Offset(property),
                                Some(f64::from(transform.offsets.get(property))),
                                &format!("transformOffset{}", property.spine_name()),
                                70.,
                            );
                        });
                    }
                }
            });
        self.record_transform_target(
            "transformOffsetGroup",
            offset.header_response.rect.intersect(ui.clip_rect()),
        );
        let mix = egui::CollapsingHeader::new("Mix")
            .id_salt(("transform-mix", index))
            .default_open(true)
            .show(ui, |ui| {
                for property in transform.targets() {
                    ui.horizontal(|ui| {
                        ui.add_sized([85., 22.], egui::Label::new(property.label()));
                        self.constraint_number(
                            ui,
                            index,
                            ConstraintField::Mix(property),
                            Some(f64::from(transform.mix.get(property))),
                            &format!("transformMix{}", property.spine_name()),
                            60.,
                        );
                        self.constraint_mix_slider(
                            ui,
                            index,
                            property,
                            transform.mix.get(property),
                        );
                    });
                }
            });
        self.record_transform_target(
            "transformMixGroup",
            mix.header_response.rect.intersect(ui.clip_rect()),
        );
        let properties = egui::CollapsingHeader::new("Properties")
            .id_salt(("transform-properties", index))
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let response = ui
                            .add_enabled(false, egui::Button::new("Match"))
                            .on_disabled_hover_text("Matching mapping ranges is not available.");
                        self.record_transform_target(
                            "transformMatch",
                            response.rect.intersect(ui.clip_rect()),
                        );
                    });
                });
                ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
                let scroll = egui::ScrollArea::horizontal()
                    .id_salt(("transform-properties-horizontal", index))
                    .min_scrolled_width(0.)
                    .auto_shrink([false, true])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        ui.set_width(430.);
                        self.transform_mapping_table(ui, index, &transform);
                    });
                self.record_transform_target(
                    "transformPropertiesScroll",
                    scroll.inner_rect.intersect(ui.clip_rect()),
                );
            });
        self.record_transform_target(
            "transformPropertiesGroup",
            properties.header_response.rect.intersect(ui.clip_rect()),
        );
        if !self.setup_mode {
            ui.label(
                RichText::new("Setup settings · Transform constraint timelines are not available.")
                    .small()
                    .color(MUTED),
            );
        }
        if let Some(error) = &self.transform_constraint_view.error {
            ui.colored_label(ACCENT, error);
        }
        if let Err(error) = self.session.evaluated_pose(self.weights_edit_target()) {
            ui.add(
                egui::Label::new(
                    RichText::new(format!("Pose cannot be evaluated: {error}"))
                        .small()
                        .color(ACCENT),
                )
                .wrap(),
            );
        }
    }
    fn transform_mapping_table(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        transform: &TransformConstraint,
    ) {
        let pitch = 26.;
        let (table, _) = ui.allocate_exact_size(vec2(430., pitch * 6.), Sense::hover());
        self.record_transform_target("transformPropertiesClip", ui.clip_rect());
        let visible = table.intersect(ui.clip_rect());
        if visible.is_positive() {
            self.record_transform_target("transformPropertiesTable", visible);
        }
        let painter = ui.painter_at(table);
        let color = |property: TransformProperty| match property {
            TransformProperty::Rotate => Color32::from_rgb(92, 205, 84),
            TransformProperty::X => Color32::from_rgb(64, 183, 218),
            TransformProperty::Y => Color32::from_rgb(75, 140, 220),
            TransformProperty::ScaleX => Color32::from_rgb(220, 100, 100),
            TransformProperty::ScaleY => Color32::from_rgb(224, 146, 84),
            TransformProperty::ShearY => Color32::from_rgb(205, 190, 88),
        };
        let row_y = |property: TransformProperty| {
            table.top()
                + TransformProperty::ALL
                    .iter()
                    .position(|&p| p == property)
                    .unwrap() as f32
                    * pitch
                + pitch / 2.
        };
        for (row, property) in TransformProperty::ALL.into_iter().enumerate() {
            let y = table.top() + row as f32 * pitch;
            painter.rect_filled(
                egui::Rect::from_min_size(pos2(table.left(), y), vec2(430., pitch)),
                0.,
                Color32::from_gray(if row % 2 == 0 { 53 } else { 58 }),
            );
            painter.circle_filled(
                pos2(table.left() + 198., row_y(property)),
                5.,
                color(property).gamma_multiply(if property == TransformProperty::X {
                    1.
                } else {
                    0.45
                }),
            );
            painter.circle_filled(
                pos2(table.left() + 316., row_y(property)),
                5.,
                color(property).gamma_multiply(if property == TransformProperty::Rotate {
                    1.
                } else {
                    0.45
                }),
            );
            painter.text(
                pos2(table.left() + 6., row_y(property)),
                egui::Align2::LEFT_CENTER,
                property.label(),
                FontId::proportional(12.),
                MUTED,
            );
        }
        for mapping in &transform.mappings {
            for target in &mapping.to {
                let from = pos2(table.left() + 198., row_y(mapping.from));
                let to = pos2(table.left() + 316., row_y(target.property));
                painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
                    [from, from + vec2(65., 0.), to - vec2(65., 0.), to],
                    false,
                    Color32::TRANSPARENT,
                    Stroke::new(1.5, color(target.property)),
                ));
            }
        }
        let bounded = transform.mappings.iter().any(|mapping| {
            mapping.from == TransformProperty::X
                && mapping.to.len() == 1
                && mapping.to[0].property == TransformProperty::Rotate
        });
        let range = transform_mapping_range(
            self.session.document(),
            index,
            TransformProperty::X,
            TransformProperty::Rotate,
        )
        .ok();
        for (row, property) in TransformProperty::ALL.into_iter().enumerate() {
            let y = table.top() + row as f32 * pitch + 1.;
            for (column, label, field, target) in [
                (
                    0,
                    "Min",
                    TransformMappingRangeField::SourceMin,
                    "transformSourceMin",
                ),
                (
                    1,
                    "Max",
                    TransformMappingRangeField::SourceMax,
                    "transformSourceMax",
                ),
                (
                    2,
                    "Min",
                    TransformMappingRangeField::TargetMin,
                    "transformTargetMin",
                ),
                (
                    3,
                    "Max",
                    TransformMappingRangeField::TargetMax,
                    "transformTargetMax",
                ),
            ] {
                let source = column < 2;
                let x = table.left()
                    + if source {
                        90. + column as f32 * 52.
                    } else {
                        322. + (column - 2) as f32 * 54.
                    };
                let cell = egui::Rect::from_min_size(
                    pos2(x, y),
                    vec2(if source { 52. } else { 54. }, 24.),
                );
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .id(egui::Id::new(("mapping-cell", index, property, column)))
                        .max_rect(cell)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                child.set_clip_rect(cell.intersect(ui.clip_rect()));
                let active = bounded
                    && ((source && property == TransformProperty::X)
                        || (!source && property == TransformProperty::Rotate));
                if active && let Some(range) = range {
                    self.constraint_number(
                        &mut child,
                        index,
                        ConstraintField::Range(field),
                        range.value(field),
                        target,
                        cell.width(),
                    );
                } else {
                    let shown = if active {
                        "—".into()
                    } else if source {
                        transform
                            .mappings
                            .iter()
                            .find(|mapping| mapping.from == property)
                            .filter(|_| column == 0)
                            .map_or_else(|| label.into(), |mapping| mapping.offset.to_string())
                    } else {
                        transform
                            .mappings
                            .iter()
                            .flat_map(|mapping| &mapping.to)
                            .find(|target| target.property == property)
                            .map_or_else(
                                || label.into(),
                                |target| {
                                    if column == 2 {
                                        target.offset.to_string()
                                    } else {
                                        target.max.to_string()
                                    }
                                },
                            )
                    };
                    child.add_enabled(false,egui::Label::new(shown).truncate()).on_disabled_hover_text("Range editing is available only for Translate X to Rotate. A zero-scale source maximum is undefined.");
                }
            }
        }
        ui.horizontal(|ui| {
            ui.label("Options");
            let mut source_local = transform.local_source;
            let mut target_local = transform.local_target;
            let mut additive = transform.additive;
            let a = ui
                .add_enabled(false, egui::Checkbox::new(&mut source_local, "Local"))
                .on_disabled_hover_text("Local source mappings are not available.");
            self.record_transform_target("transformSourceLocal", a.rect.intersect(ui.clip_rect()));
            let b = ui
                .add_enabled(false, egui::Checkbox::new(&mut target_local, "Local"))
                .on_disabled_hover_text("Local target mappings are not available.");
            self.record_transform_target("transformTargetLocal", b.rect.intersect(ui.clip_rect()));
            let c = ui
                .add_enabled(false, egui::Checkbox::new(&mut additive, "Additive"))
                .on_disabled_hover_text("Additive mappings are not available.");
            self.record_transform_target("transformAdditive", c.rect.intersect(ui.clip_rect()));
            let mut clamp = transform.clamp;
            let mut candidate = transform.clone();
            candidate.clamp = !clamp;
            let enabled = self.setup_mode
                && self.weights_input_available(ui.ctx())
                && candidate.supported_subset().is_ok();
            let response = ui.add_enabled(enabled, egui::Checkbox::new(&mut clamp, "Clamp"));
            let visible = response.rect.intersect(ui.clip_rect());
            if visible.is_positive() {
                self.record_transform_target("transformClamp", visible);
            }
            if response.changed() {
                self.set_constraint_clamp(index, clamp);
            }
        });
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
        click(&mut editor, &ctx, "transformOffsetGroup");
        for _ in 0..5 {
            workspace_frame(&mut editor, &ctx, vec![]);
        }
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
