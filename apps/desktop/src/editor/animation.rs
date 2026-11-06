//! Animation authoring shell. Validation, pose/key conversion and history live in the engine.
use super::*;

impl Editor {
    pub(super) fn set_time(&mut self, time: f32) {
        if !time.is_finite() {
            return;
        }
        let time = time.max(0.0);
        if self.time != time {
            self.cancel_gesture();
            self.session.discard_pose(true);
            self.pose_view_generation = self.pose_view_generation.wrapping_add(1);
            self.time = time;
        }
    }
    pub(super) fn toggle_playback(&mut self, direction: PlayDirection) {
        let playing = !(self.playing && self.play_direction == direction);
        self.play_direction = direction;
        self.set_playing(playing);
    }
    pub(super) fn jump_key(&mut self, direction: PlayDirection) {
        self.playing = false;
        if let Some(time) = peachy_engine::adjacent_key_time(
            self.session.document(),
            self.animation,
            self.time,
            direction,
        ) {
            self.set_time(time);
        }
    }
    pub(super) fn set_playing(&mut self, playing: bool) {
        if playing {
            self.cancel_gesture();
            self.session.discard_pose(true);
        }
        self.playing = playing;
    }
    pub(super) fn set_setup_mode(&mut self, setup: bool) {
        if self.setup_mode != setup {
            self.weights.bind = None;
            self.cancel_gesture();
            self.session.discard_pose(false);
            self.setup_mode = setup;
            self.playing = false;
        }
    }
    pub(super) fn key_transform(&mut self, properties: &[Property]) {
        if !self.setup_mode
            && let Some(bone) = self.selected_bone()
        {
            self.cancel_gesture();
            self.playing = false;
            if let Err(error) = self
                .session
                .key_pose(self.animation, self.time, bone, properties)
            {
                self.error = Some(error);
            }
        }
    }
    pub(super) fn key_edited(&mut self, bone: Option<usize>) {
        if !self.setup_mode {
            self.cancel_gesture();
            self.playing = false;
            if let Err(error) = self.session.key_edited(self.animation, self.time, bone) {
                self.error = Some(error);
            }
        }
    }
    pub(super) fn key_state(&self, bone: usize, properties: &[Property]) -> KeyState {
        self.session.pose_edits().key_state(
            self.session.document(),
            self.animation,
            self.snapped_time(),
            bone,
            properties,
        )
    }
}

/// Frame arithmetic must stay finite even for a valid cursor near f32::MAX.
pub(super) fn frame_at(time: f32, fps: u32) -> f64 {
    (f64::from(time) * f64::from(fps)).round()
}

pub(super) fn snap_time(time: f32, fps: u32) -> f32 {
    (frame_at(time, fps) / f64::from(fps)) as f32
}

/// Draw our own key indicator instead of relying on a font's symbol coverage.
pub(super) fn pose_key_button(ui: &mut egui::Ui, state: KeyState) -> egui::Response {
    let color = match state {
        KeyState::Absent => MINT,
        KeyState::Edited => ACCENT,
        KeyState::Keyed => Color32::from_rgb(230, 85, 75),
    };
    let response = ui.add_sized([22.0, 18.0], egui::Button::new(""));
    let c = response.rect.center();
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            c + vec2(0., -4.),
            c + vec2(4., 0.),
            c + vec2(0., 4.),
            c + vec2(-4., 0.),
        ],
        if state == KeyState::Absent {
            Color32::TRANSPARENT
        } else {
            color
        },
        Stroke::new(1., color),
    ));
    response
}

#[cfg(test)]
mod tests {
    use super::super::dopesheet_selection::KeyId;
    use super::super::key_drag::{ChartGeometry, KeyDrag};
    use super::super::test_support::{editor_context, editor_fixture};
    use super::*;

    fn empty_editor() -> Editor {
        let mut editor = editor_fixture();
        let mut document = editor.session.document().clone();
        document.animations[0].tracks.clear();
        editor.session = Session::new(document).unwrap();
        editor.time = 0.;
        editor.reconcile_selection();
        editor
    }

    fn render_timeline(editor: &mut Editor) {
        let ctx = editor_context();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.set_min_size(vec2(1000., 240.));
            editor.timeline(ui);
        });
        output.textures_delta.clear();
        assert!(
            editor
                .capture_targets
                .values()
                .flatten()
                .all(|v| v.is_finite())
        );
        let track = editor.capture_targets["timelineTrack"];
        assert!(track[2] > track[0]);
    }

    #[test]
    fn empty_and_frame_zero_clips_render_and_allow_dragging_without_a_reserve_key() {
        let mut editor = empty_editor();
        let original = editor.session.document().clone();
        assert_eq!(editor.animation_end(), 0.);
        assert_eq!(editor.display_extent(), 1.);
        render_timeline(&mut editor);
        assert_eq!(editor.session.document(), &original);
        editor.key_transform(&[Property::Rotation]);
        assert_eq!(editor.animation_end(), 0.);
        render_timeline(&mut editor);
        let key_rect = editor.capture_targets["timelineKey:Rotation:0"];
        let track_rect = editor.capture_targets["timelineTrack"];
        assert_eq!((key_rect[0] + key_rect[2]) / 2., track_rect[0]);
        let geometry = ChartGeometry {
            start: track_rect[0],
            width: track_rect[2] - track_rect[0],
            extent: editor.display_extent(),
        };
        let drag = KeyDrag::new(
            0,
            8,
            KeyId {
                property: Property::Rotation,
                time: 0.,
            },
            &[],
            geometry,
            geometry.start,
            egui::Modifiers::NONE,
        )
        .unwrap();
        assert!(
            drag.offset_at(geometry.start + geometry.width / 2., 30, false)
                .unwrap()
                > 0.
        );
        assert_eq!(geometry.time_at(geometry.start), Some(0.));
    }

    #[test]
    fn seeking_and_unkeyed_pose_past_end_leave_keys_untouched_until_explicit_keying() {
        let mut editor = empty_editor();
        let original = editor.session.document().clone();
        editor.set_time(2.37);
        editor.reconcile_selection();
        assert_eq!(editor.time, 2.37);
        assert_eq!(editor.animation_end(), 0.);
        assert_eq!(editor.display_extent(), 2.37);
        assert_eq!(editor.session.document(), &original);
        assert!(!editor.session.can_undo());
        let mut pose = editor.selected_transform().unwrap();
        pose.rotation += 23.;
        editor.edit_transform(pose, &[Property::Rotation]);
        assert_eq!(editor.animation_end(), 0.);
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.selected_transform(), Some(pose));
        assert!(!editor.session.pose_edits().is_empty());
        assert!(!editor.session.is_dirty());
        let pending = editor.session.pose_edits().clone();
        editor.key_transform(&[Property::Rotation]);
        assert_eq!(editor.animation_end(), snap_time(2.37, original.fps));
        assert_eq!(editor.time, 2.37);
        assert!(editor.session.pose_edits().is_empty());
        assert!(editor.session.is_dirty());
        let tracks = &editor.session.document().animations[0].tracks;
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].keys.len(), 1);
        assert_eq!(tracks[0].keys[0].time, editor.animation_end());
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.session.pose_edits(), &pending);
        assert_eq!(editor.animation_end(), 0.);
        assert_eq!(editor.time, 2.37);
        editor.redo();
        assert_eq!(editor.animation_end(), snap_time(2.37, original.fps));
        assert_eq!(editor.time, 2.37);
    }

    #[test]
    fn shrinking_end_preserves_cursor_and_a_drag_uses_its_original_display_extent() {
        let mut editor = empty_editor();
        editor.set_time(2.);
        editor.key_transform(&[Property::Rotation]);
        let end_key = KeyId {
            property: Property::Rotation,
            time: 2.,
        };
        editor.key_selection.replace(vec![end_key]);
        editor.set_time(0.4);
        let original = editor.session.document().clone();
        let geometry = ChartGeometry {
            start: 0.,
            width: 200.,
            extent: editor.display_extent(),
        };
        editor.key_drag = KeyDrag::new(
            0,
            8,
            end_key,
            &[end_key],
            geometry,
            200.,
            egui::Modifiers::NONE,
        );
        editor.update_dopesheet_drag(100., false);
        assert!(editor.error.is_none());
        assert_eq!(editor.animation_end(), 1.);
        assert_eq!(editor.key_drag.as_ref().unwrap().geometry.extent, 2.);
        render_timeline(&mut editor);
        let key_rect = editor.capture_targets["timelineKey:Rotation:0"];
        assert_eq!((key_rect[0] + key_rect[2]) / 2., 100.);
        editor.cancel_dopesheet_drag();
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.animation_end(), 2.);
        editor.set_time(3.2);
        editor.delete_dopesheet_keys();
        assert_eq!(editor.animation_end(), 0.);
        assert_eq!(editor.time, 3.2);
        assert_eq!(editor.display_extent(), 3.2);
        editor.undo();
        assert_eq!(editor.session.document(), &original);
        assert_eq!(editor.animation_end(), 2.);
        assert_eq!(editor.time, 3.2);
    }

    #[test]
    fn finite_large_cursor_and_frame_arithmetic_remain_usable_and_invalid_seeks_are_ignored() {
        let mut editor = empty_editor();
        editor.key_transform(&[Property::Rotation]);
        editor.set_time(f32::MAX);
        editor.reconcile_selection();
        assert_eq!(editor.time, f32::MAX);
        assert_eq!(editor.snapped_time(), f32::MAX);
        assert!(frame_at(editor.time, u32::MAX).is_finite());
        assert_eq!(editor.display_extent(), f32::MAX);
        assert_eq!(editor.animation_end(), 0.);
        render_timeline(&mut editor);
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            editor.set_time(invalid);
            assert_eq!(editor.time, f32::MAX);
        }
        editor.set_time(-1.);
        assert_eq!(editor.time, 0.);
    }
}
