//! Animation authoring shell. Validation, pose/key conversion and history live in the engine.
use super::*;

impl Editor {
    pub(super) fn set_time(&mut self, time: f32) {
        let time = time.clamp(0.0, self.duration());
        if self.time != time {
            self.cancel_gesture();
            self.session.discard_pose(true);
            self.pose_view_generation = self.pose_view_generation.wrapping_add(1);
            self.time = time;
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
