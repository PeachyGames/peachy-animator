//! Physics constraint properties. Settings route through shared engine commands;
//! the editor does not simulate physics in this slice.
use super::*;
use peachy_document::{PhysicsConstraint, PhysicsProperty, ScaleYMode};

/// One numeric setting in the Tree properties grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PhysicsField {
    X,
    Y,
    Rotate,
    ScaleX,
    ShearX,
    Limit,
    Fps,
    Setting(PhysicsProperty),
}

impl PhysicsField {
    pub(super) const INFLUENCES: [(Self, &'static str); 5] = [
        (Self::X, "Translate X"),
        (Self::Y, "Translate Y"),
        (Self::Rotate, "Rotate"),
        (Self::ScaleX, "Scale X"),
        (Self::ShearX, "Shear X"),
    ];
    pub(super) fn read(self, physics: &PhysicsConstraint) -> f32 {
        match self {
            Self::X => physics.influence.x,
            Self::Y => physics.influence.y,
            Self::Rotate => physics.influence.rotate,
            Self::ScaleX => physics.influence.scale_x,
            Self::ShearX => physics.influence.shear_x,
            Self::Limit => physics.limit,
            Self::Fps => physics.fps,
            Self::Setting(property) => property.read(physics.setup),
        }
    }
    pub(super) fn write(self, physics: &mut PhysicsConstraint, value: f32) {
        match self {
            Self::X => physics.influence.x = value,
            Self::Y => physics.influence.y = value,
            Self::Rotate => physics.influence.rotate = value,
            Self::ScaleX => physics.influence.scale_x = value,
            Self::ShearX => physics.influence.shear_x = value,
            Self::Limit => physics.limit = value,
            Self::Fps => physics.fps = value,
            Self::Setting(property) => property.write(&mut physics.setup, value),
        }
    }
    /// Fractions are shown as percentages; other values in their own units.
    fn percent(self) -> bool {
        matches!(
            self,
            Self::X
                | Self::Y
                | Self::Rotate
                | Self::ScaleX
                | Self::ShearX
                | Self::Setting(
                    PhysicsProperty::Inertia | PhysicsProperty::Damping | PhysicsProperty::Mix
                )
        )
    }
    fn range(self) -> std::ops::RangeInclusive<f64> {
        match self {
            Self::Fps => 1.0..=240.0,
            Self::Limit => 0.0..=f64::from(f32::MAX),
            Self::Setting(PhysicsProperty::Mass) => 0.01..=f64::from(f32::MAX),
            Self::Setting(PhysicsProperty::Wind | PhysicsProperty::Gravity) => {
                f64::from(f32::MIN)..=f64::from(f32::MAX)
            }
            field if field.percent() => 0.0..=100.0,
            _ => 0.0..=f64::from(f32::MAX),
        }
    }
    fn target(self) -> String {
        match self {
            Self::X => "physicsX".into(),
            Self::Y => "physicsY".into(),
            Self::Rotate => "physicsRotate".into(),
            Self::ScaleX => "physicsScaleX".into(),
            Self::ShearX => "physicsShearX".into(),
            Self::Limit => "physicsLimit".into(),
            Self::Fps => "physicsFps".into(),
            Self::Setting(property) => format!("physics{}", property.label()),
        }
    }
}

impl Editor {
    pub(super) fn physics_constraint(&self, index: usize) -> Option<&PhysicsConstraint> {
        self.session.document().constraints.get(index)?.as_physics()
    }

    /// One shared path for every physics edit. Typed text applies once on commit
    /// (one undo step); a drag runs inside a Session transaction committed on release.
    fn set_physics(&mut self, index: usize, value: PhysicsConstraint, status: String) {
        if !self.setup_mode {
            self.status =
                "Physics settings are edited in Setup; keying them is not supported yet".into();
            return;
        }
        if self.dispatch(Command::SetPhysicsConstraint {
            constraint: index,
            value,
        }) {
            self.status = status;
        }
    }

    fn physics_number(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        physics: &PhysicsConstraint,
        field: PhysicsField,
        label: &str,
    ) {
        ui.label(label);
        let raw = f64::from(field.read(physics));
        let mut shown = if field.percent() { raw * 100. } else { raw };
        let response = ui.add_enabled(
            self.setup_mode,
            egui::DragValue::new(&mut shown)
                .range(field.range())
                // Imported values outside the UI range must never be rewritten by display.
                .clamp_existing_to_range(false)
                // Typed text applies once, on Enter or focus loss: one undo step.
                .update_while_editing(false)
                .max_decimals(if field.percent() { 2 } else { 3 })
                .suffix(if field.percent() { "%" } else { "" })
                .speed(if field.percent() || field == PhysicsField::Fps {
                    0.25
                } else {
                    1.0
                }),
        );
        self.record_target(&field.target(), response.rect);
        if response.drag_started() {
            self.cancel_ik_edit();
            self.cancel_transform_edit();
            self.cancel_weight_edit();
            self.session.begin_transaction();
        }
        if response.changed() {
            let value = if field.percent() { shown / 100. } else { shown } as f32;
            let mut edited = physics.clone();
            field.write(&mut edited, value);
            self.set_physics(index, edited, format!("Physics {label}: {}", fmt(shown)));
        }
        if response.drag_stopped() {
            self.session.commit_transaction();
        }
    }

    fn physics_global(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        physics: &PhysicsConstraint,
        property: PhysicsProperty,
    ) {
        let mut global = property.global(physics.global);
        let response = ui
            .add_enabled(self.setup_mode, egui::Checkbox::new(&mut global, "Global"))
            .on_hover_text("Controlled by global physics timelines shared with other constraints");
        self.record_target(&format!("physics{}Global", property.label()), response.rect);
        if response.changed() {
            let mut edited = physics.clone();
            property.set_global(&mut edited.global, global);
            self.set_physics(
                index,
                edited,
                format!(
                    "Physics {} Global {}",
                    property.label(),
                    if global { "on" } else { "off" }
                ),
            );
        }
    }

    pub(super) fn physics_properties(&mut self, ui: &mut egui::Ui, index: usize) {
        let Some(physics) = self.physics_constraint(index).cloned() else {
            return;
        };
        ui.horizontal(|ui| {
            icons::cue(ui, icons::Icon::PhysicsConstraint);
            ui.label(RichText::new("Physics").color(ACCENT));
        });
        egui::Grid::new(("physics-properties", index))
            .num_columns(3)
            .min_col_width(58.)
            .spacing(vec2(6., 3.))
            .striped(true)
            .show(ui, |ui| {
                ui.label("Bone");
                ui.horizontal(|ui| {
                    icons::cue(ui, icons::Icon::Bone);
                    let link = ui.link(&self.session.document().bones[physics.bone].name);
                    self.record_target("physicsBone", link.rect);
                    if link.clicked() {
                        self.select(TreeItem::Bone(physics.bone));
                    }
                });
                ui.end_row();
                for (field, label) in PhysicsField::INFLUENCES {
                    self.physics_number(ui, index, &physics, field, label);
                    ui.end_row();
                }
                ui.label("Scale Y");
                let mut mode = physics.scale_y_mode;
                ui.add_enabled_ui(self.setup_mode, |ui| {
                    let combo = egui::ComboBox::from_id_salt(("physics-scale-y", index))
                        .selected_text(match mode {
                            ScaleYMode::None => "None",
                            ScaleYMode::Uniform => "Uniform",
                            ScaleYMode::Volume => "Volume",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut mode, ScaleYMode::None, "None");
                            ui.selectable_value(&mut mode, ScaleYMode::Uniform, "Uniform");
                            ui.selectable_value(&mut mode, ScaleYMode::Volume, "Volume");
                        });
                    self.record_target("physicsScaleY", combo.response.rect);
                });
                if mode != physics.scale_y_mode {
                    let mut edited = physics.clone();
                    edited.scale_y_mode = mode;
                    self.set_physics(index, edited, "Physics Scale Y mode updated".into());
                }
                ui.end_row();
                self.physics_number(ui, index, &physics, PhysicsField::Limit, "Limit");
                ui.end_row();
                self.physics_number(ui, index, &physics, PhysicsField::Fps, "FPS");
                ui.end_row();
                for property in PhysicsProperty::ALL {
                    self.physics_number(
                        ui,
                        index,
                        &physics,
                        PhysicsField::Setting(property),
                        property.label(),
                    );
                    self.physics_global(ui, index, &physics, property);
                    ui.end_row();
                }
            });
        let keyed = self.physics_keys_summary(index);
        if !self.setup_mode {
            ui.label(
                RichText::new(
                    "Setup settings · Keying physics in the editor is not supported yet.",
                )
                .small()
                .color(MUTED),
            );
        }
        if let Some(keyed) = keyed {
            ui.label(RichText::new(keyed).small().color(MUTED));
        }
        ui.label(
            RichText::new(
                "Preview does not simulate physics yet. Settings and keys are saved and exported for the game runtime.",
            )
            .small()
            .color(MUTED),
        );
    }

    /// Read-only summary of keyed settings in the active animation, including
    /// global timelines that apply through this constraint's Global flags.
    fn physics_keys_summary(&self, index: usize) -> Option<String> {
        let doc = self.session.document();
        let animation = doc.animations.get(self.animation)?;
        let physics = doc.constraints.get(index)?.as_physics()?;
        let mut parts = vec![];
        for track in &animation.physics {
            let applies = match track.constraint {
                Some(constraint) => constraint == index,
                None => track.property.global(physics.global),
            };
            if applies && !track.keys.is_empty() {
                let scope = if track.constraint.is_none() {
                    " (global)"
                } else {
                    ""
                };
                parts.push(format!(
                    "{}{scope} ×{}",
                    track.property.label(),
                    track.keys.len()
                ));
            }
        }
        for reset in &animation.physics_resets {
            if reset
                .constraint
                .is_none_or(|constraint| constraint == index)
                && !reset.times.is_empty()
            {
                let scope = if reset.constraint.is_none() {
                    " (all)"
                } else {
                    ""
                };
                parts.push(format!("Reset{scope} ×{}", reset.times.len()));
            }
        }
        (!parts.is_empty()).then(|| format!("Keyed in {}: {}", animation.name, parts.join(", ")))
    }
}

fn fmt(value: f64) -> String {
    let text = format!("{value:.3}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{editor_context, editor_fixture, workspace_frame};
    use super::*;
    use peachy_document::{PhysicsConstraint, PhysicsReset, PhysicsTrack};

    fn install(editor: &mut Editor) -> Document {
        let mut document = editor.session.document().clone();
        let mut sway = PhysicsConstraint::new("hand sway", 8);
        sway.influence.rotate = 1.;
        document.constraints.push(Constraint::Physics(sway));
        document.animations[0].physics.push(PhysicsTrack {
            constraint: Some(0),
            property: PhysicsProperty::Strength,
            keys: vec![Keyframe {
                time: 0.,
                value: 60.,
                curve: Curve::Linear,
            }],
        });
        document.animations[0].physics_resets.push(PhysicsReset {
            constraint: None,
            times: vec![0.5],
        });
        editor.session = Session::new(document.clone()).unwrap();
        editor.setup_mode = true;
        editor.animation = 0;
        editor.select(TreeItem::Constraint(0));
        document
    }
    fn rect(editor: &Editor, target: &str) -> egui::Rect {
        let [left, top, right, bottom] = *editor.capture_targets.get(target).unwrap_or_else(|| {
            panic!(
                "Missing UI target {target}; available: {:?}",
                editor.capture_targets.keys().collect::<Vec<_>>()
            )
        });
        egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom))
    }
    fn pointer(pos: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }
    /// Scroll Properties like a user until the control is fully visible.
    fn reveal(editor: &mut Editor, ctx: &egui::Context, target: &str) {
        for _ in 0..40 {
            let clip = rect(editor, "properties");
            let control = rect(editor, target);
            if clip.contains_rect(control) {
                return;
            }
            let delta = if control.top() < clip.top() {
                60.
            } else {
                -60.
            };
            workspace_frame(
                editor,
                ctx,
                vec![
                    egui::Event::PointerMoved(clip.center()),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: vec2(0., delta),
                        phase: egui::TouchPhase::Move,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            workspace_frame(editor, ctx, vec![]);
        }
        panic!("{target} could not be scrolled into the Properties clip");
    }
    fn click(editor: &mut Editor, ctx: &egui::Context, target: &str) {
        reveal(editor, ctx, target);
        let pos = rect(editor, target).center();
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
    fn workspace_shows_every_physics_setting_inside_properties() {
        let mut editor = editor_fixture();
        install(&mut editor);
        let ctx = editor_context();
        workspace_frame(&mut editor, &ctx, vec![]);
        let clip = rect(&editor, "properties");
        let mut targets = vec!["physicsBone".to_owned(), "physicsScaleY".into()];
        targets.extend(PhysicsField::INFLUENCES.map(|(field, _)| field.target()));
        targets.extend([PhysicsField::Limit, PhysicsField::Fps].map(PhysicsField::target));
        for property in PhysicsProperty::ALL {
            targets.push(PhysicsField::Setting(property).target());
            targets.push(format!("physics{}Global", property.label()));
        }
        // The first rows are visible without scrolling; the rest scroll into view.
        for target in ["physicsBone", "physicsX", "physicsRotate"] {
            assert!(clip.contains_rect(rect(&editor, target)), "{target}");
        }
        for target in &targets {
            reveal(&mut editor, &ctx, target);
            let control = rect(&editor, target);
            assert!(control.left() >= clip.left() && control.right() <= clip.right());
        }
        assert_eq!(
            editor.physics_keys_summary(0).as_deref(),
            Some("Keyed in wave: Strength ×1, Reset (all) ×1")
        );
    }

    #[test]
    fn typed_entry_and_global_toggle_are_single_undo_steps_and_animate_is_read_only() {
        let mut editor = editor_fixture();
        let original = install(&mut editor);
        let ctx = editor_context();
        workspace_frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "physicsStrength");
        for digit in ["2", "5", "0"] {
            workspace_frame(&mut editor, &ctx, vec![egui::Event::Text(digit.into())]);
        }
        // Nothing applies until the entry is committed.
        assert_eq!(editor.session.document(), &original);
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::Enter, egui::Modifiers::NONE),
        );
        workspace_frame(&mut editor, &ctx, vec![]);
        let typed = editor.physics_constraint(0).unwrap().clone();
        assert_eq!(typed.setup.strength, 250.);
        let mut expected = original.constraints[0].as_physics().unwrap().clone();
        expected.setup.strength = 250.;
        assert_eq!(typed, expected);
        click(&mut editor, &ctx, "physicsWindGlobal");
        workspace_frame(&mut editor, &ctx, vec![]);
        assert!(editor.physics_constraint(0).unwrap().global.wind);
        editor.undo();
        assert_eq!(editor.physics_constraint(0).unwrap(), &typed);
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());

        editor.redo();
        let edited = editor.session.document().clone();
        editor.set_setup_mode(false);
        workspace_frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "physicsMixGlobal");
        click(&mut editor, &ctx, "physicsMass");
        workspace_frame(&mut editor, &ctx, vec![egui::Event::Text("9".into())]);
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::Enter, egui::Modifiers::NONE),
        );
        assert_eq!(editor.session.document(), &edited);
    }

    #[test]
    fn rename_routes_to_the_physics_command_and_keeps_keyed_references() {
        let mut editor = editor_fixture();
        let original = install(&mut editor);
        let ctx = editor_context();
        workspace_frame(&mut editor, &ctx, vec![]);
        click(&mut editor, &ctx, "rename");
        assert!(editor.rename.is_some());
        workspace_frame(
            &mut editor,
            &ctx,
            vec![egui::Event::Text("tail sway".into())],
        );
        workspace_frame(
            &mut editor,
            &ctx,
            key(egui::Key::Enter, egui::Modifiers::NONE),
        );
        assert!(editor.rename.is_none());
        let doc = editor.session.document();
        assert_eq!(doc.constraints[0].name(), "tail sway");
        assert_eq!(doc.animations[0].physics[0].constraint, Some(0));
        let mut expected = original.clone();
        expected.constraints[0].set_name("tail sway".into());
        assert_eq!(doc, &expected);
        editor.undo();
        assert_eq!(editor.session.document(), &original);
    }
}
