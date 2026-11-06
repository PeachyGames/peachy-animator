//! Playback settings are session presentation state, never animation data or history.
use super::*;

impl Editor {
    fn set_playback_speed(&mut self, speed: f32) {
        if speed.is_finite() {
            let speed = speed.clamp(0.01, 2.);
            if self.playback_speed != speed {
                self.cancel_gesture();
                self.playback_speed = speed;
            }
        }
    }

    pub(super) fn advance_playback(&mut self, elapsed: f32) {
        // Limit a stalled UI's elapsed time before applying the preview multiplier.
        // Frame/key navigation and authoring still use the document's actual FPS.
        let step = peachy_engine::advance_playback(
            self.time,
            elapsed.min(0.1) * self.playback_speed,
            self.animation_end(),
            self.looping,
            self.play_direction,
        );
        self.time = step.time;
        self.playing = step.playing;
    }

    pub(super) fn playback_panel(&mut self, ui: &mut egui::Ui) {
        self.record_target("playbackPanelClip", ui.clip_rect());
        let fps = self.session.document().fps;
        ui.horizontal(|ui| {
            ui.label("Timeline FPS");
            ui.label(RichText::new(fps.to_string()).monospace())
                .on_hover_text("Project frame rate. Changing timeline FPS is not available yet.");
        });
        ui.add_space(4.);
        ui.label("Speed");
        let mut percent = self.playback_speed * 100.;
        ui.horizontal(|ui| {
            ui.spacing_mut().slider_width = (ui.available_width() - 80.).clamp(50., 180.);
            let slider = ui.add(
                egui::Slider::new(&mut percent, 1.0..=200.0)
                    .show_value(false)
                    .integer(),
            );
            self.record_target("playbackSpeed", slider.rect);
            let value = ui.add(
                egui::DragValue::new(&mut percent)
                    .range(1.0..=200.0)
                    .suffix("%")
                    .max_decimals(0)
                    .update_while_editing(false),
            );
            value.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::DragValue,
                    ui.is_enabled(),
                    "Playback speed percent",
                )
            });
            self.record_target("playbackSpeedValue", value.rect);
            if slider.changed() || value.changed() {
                self.set_playback_speed(percent / 100.);
            }
        });
        ui.horizontal_wrapped(|ui| {
            for percent in [25, 50, 75, 100, 200] {
                let speed = percent as f32 / 100.;
                let preset = ui.selectable_label(
                    (self.playback_speed - speed).abs() < 0.00001,
                    format!("{percent}%"),
                );
                self.record_target(&format!("playbackSpeed:{percent}"), preset.rect);
                if preset.clicked() {
                    self.set_playback_speed(speed);
                }
            }
        });
        ui.separator();
        let name = self
            .session
            .document()
            .animations
            .get(self.animation)
            .map_or("No animation", |animation| animation.name.as_str());
        ui.label(RichText::new(name).color(ACCENT));
        ui.horizontal_wrapped(|ui| {
            for (direction, name, hint) in [
                (
                    PlayDirection::Backward,
                    "Backward",
                    "Play backward/stop · A",
                ),
                (PlayDirection::Forward, "Forward", "Play forward/stop · D"),
            ] {
                let active = self.playing && self.play_direction == direction;
                let response = dopesheet::transport_button(ui, direction, active);
                self.record_target(&format!("playback{name}"), response.rect);
                if response.on_hover_text(hint).clicked() {
                    self.toggle_playback(direction);
                }
            }
            let looping = ui.checkbox(&mut self.looping, "Loop");
            self.record_target("playbackLoop", looping.rect);
            looping.on_hover_text("Loop playback · Cmd/Ctrl+R");
            ui.label(
                RichText::new(format!(
                    "{:03.0} / {:03.0}",
                    animation::frame_at(self.time, fps),
                    animation::frame_at(self.animation_end(), fps),
                ))
                .monospace(),
            )
            .on_hover_text("Current frame / last keyed frame");
        });
        ui.horizontal_wrapped(|ui| {
            for (name, label, hint) in [
                ("Start", "Start", "First frame · Q"),
                ("PreviousFrame", "Previous frame", "Previous frame · F"),
                ("NextFrame", "Next frame", "Next frame · R"),
                ("End", "End", "Last key · E"),
            ] {
                let response = ui.small_button(label);
                self.record_target(&format!("playback{name}"), response.rect);
                if response.on_hover_text(hint).clicked() {
                    match name {
                        "PreviousFrame" => self.step_frames(-1.),
                        "NextFrame" => self.step_frames(1.),
                        _ => {
                            self.playing = false;
                            self.set_time(if name == "Start" {
                                0.
                            } else {
                                self.animation_end()
                            });
                        }
                    }
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (direction, name, label, hint) in [
                (
                    PlayDirection::Backward,
                    "PreviousKey",
                    "Previous key",
                    "Previous key · S",
                ),
                (
                    PlayDirection::Forward,
                    "NextKey",
                    "Next key",
                    "Next key · W",
                ),
            ] {
                let response = ui.small_button(label);
                self.record_target(&format!("playback{name}"), response.rect);
                if response.on_hover_text(hint).clicked() {
                    self.jump_key(direction);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::editor_fixture;
    use super::*;

    #[test]
    fn speed_scales_both_directions_and_uses_shared_loop_and_stop_boundaries() {
        let mut editor = editor_fixture();
        let original = editor.session.document().clone();
        editor.set_time(0.2);
        editor.set_playing(true);
        editor.set_playback_speed(0.5);
        editor.advance_playback(0.08);
        assert!((editor.time - 0.24).abs() < 0.000001);
        editor.set_playback_speed(2.);
        editor.advance_playback(0.08);
        assert!((editor.time - 0.4).abs() < 0.000001);
        editor.play_direction = PlayDirection::Backward;
        editor.advance_playback(0.08);
        assert!((editor.time - 0.24).abs() < 0.000001);
        editor.set_time(0.02);
        editor.advance_playback(0.08);
        assert!((editor.time - (editor.animation_end() - 0.14)).abs() < 0.000001);
        assert!(editor.playing);
        editor.looping = false;
        editor.set_time(0.02);
        editor.advance_playback(0.08);
        assert_eq!(editor.time, 0.);
        assert!(!editor.playing);
        editor.play_direction = PlayDirection::Forward;
        editor.set_time(editor.animation_end() - 0.01);
        editor.set_playing(true);
        editor.advance_playback(0.08);
        assert_eq!(editor.time, editor.animation_end());
        assert!(!editor.playing);
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
    }

    #[test]
    fn speed_is_bounded_transient_and_preserves_pending_pose_and_history() {
        let mut editor = editor_fixture();
        let original = editor.session.document().clone();
        let mut pose = editor.selected_transform().unwrap();
        pose.rotation += 13.;
        editor.edit_transform(pose, &[Property::Rotation]);
        let pending = editor.session.pose_edits().clone();
        let time = editor.time;
        let history = (editor.session.can_undo(), editor.session.can_redo());
        for (speed, expected) in [(0.5, 0.5), (0., 0.01), (300., 2.), (f32::NAN, 2.)] {
            editor.set_playback_speed(speed);
            assert_eq!(editor.playback_speed, expected);
        }
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(editor.time, time);
        assert_eq!(editor.session.document(), &original);
        assert_eq!(
            (editor.session.can_undo(), editor.session.can_redo()),
            history
        );
        assert!(!editor.session.is_dirty());
        editor.set_playback_speed(0.5);
        editor.activate_animation(1);
        assert_eq!(editor.playback_speed, 0.5);
        editor.step_frames(1.);
        assert!((editor.time - 1. / original.fps as f32).abs() < 0.000001);
    }

    #[test]
    fn speed_handles_empty_clips_and_limits_stalls_before_scaling() {
        let mut editor = editor_fixture();
        editor.set_time(0.);
        editor.set_playback_speed(2.);
        editor.advance_playback(10.);
        assert!((editor.time - 0.2).abs() < 0.000001);
        let mut document = editor.session.document().clone();
        document.animations[0].tracks.clear();
        editor.session = Session::new(document).unwrap();
        editor.set_playing(true);
        editor.advance_playback(0.08);
        assert_eq!(editor.time, 0.);
        assert!(!editor.playing);
    }
}
