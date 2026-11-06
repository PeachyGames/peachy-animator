//! A fixed drag snapshot; screen input stays here and key transitions stay in the engine.
use super::dopesheet_selection::KeyId;
use super::*;
use peachy_engine::TimelineKey;

#[derive(Clone, Copy)]
pub(super) struct ChartGeometry {
    pub start: f32,
    pub width: f32,
    pub duration: f32,
}

pub(super) struct KeyDrag {
    pub animation: usize,
    pub bone: usize,
    pub geometry: ChartGeometry,
    pub sources: Vec<TimelineKey>,
    selection_before: Vec<KeyId>,
    origin_x: f32,
    pressed_time: f32,
    pub copy: bool,
    pub offset: f32,
    pub active: bool,
}

impl KeyDrag {
    pub fn new(
        animation: usize,
        bone: usize,
        pressed: KeyId,
        selected: &[KeyId],
        geometry: ChartGeometry,
        origin_x: f32,
        copy: bool,
    ) -> Option<Self> {
        if !geometry.start.is_finite()
            || !geometry.width.is_finite()
            || geometry.width <= 0.
            || !geometry.duration.is_finite()
            || geometry.duration <= 0.
            || !origin_x.is_finite()
        {
            return None;
        }
        let keys = if selected.contains(&pressed) {
            selected.to_vec()
        } else {
            vec![pressed]
        };
        Some(Self {
            animation,
            bone,
            geometry,
            sources: keys
                .iter()
                .map(|key| TimelineKey {
                    bone,
                    property: key.property,
                    time: key.time,
                })
                .collect(),
            selection_before: selected.to_vec(),
            origin_x,
            pressed_time: pressed.time,
            copy,
            offset: 0.,
            active: false,
        })
    }
    pub fn offset_at(&self, pointer_x: f32, fps: u32, unsnapped: bool) -> Option<f32> {
        let mut offset = (pointer_x - self.origin_x) / self.geometry.width * self.geometry.duration;
        if !offset.is_finite() || fps == 0 {
            return None;
        }
        if !unsnapped {
            offset = ((self.pressed_time + offset) * fps as f32).round() / fps as f32
                - self.pressed_time;
        }
        if !offset.is_finite() {
            return None;
        }
        let earliest = self.sources.iter().map(|key| key.time).reduce(f32::min)?;
        Some(offset.max(-earliest))
    }
    fn command(&self, delta: f32) -> Command {
        Command::MoveKeys {
            animation: self.animation,
            keys: self.sources.clone(),
            delta,
            copy: self.copy,
        }
    }
    fn selection_at(&self, offset: f32) -> Vec<KeyId> {
        self.sources
            .iter()
            .map(|key| KeyId {
                property: key.property,
                time: key.time + offset,
            })
            .collect()
    }
    pub fn diagnostic(&self) -> serde_json::Value {
        serde_json::json!({
            "active":self.active,"animation":self.animation,"bone":self.bone,
            "sources":self.sources,"offset":self.offset,"copy":self.copy,
            "pressedTime":self.pressed_time,"originX":self.origin_x,
            "chart":{"start":self.geometry.start,"width":self.geometry.width,"duration":self.geometry.duration}
        })
    }
}

impl Editor {
    pub(super) fn cancel_dopesheet_drag(&mut self) {
        if let Some(drag) = self.key_drag.take() {
            if drag.active {
                self.session.cancel_transaction();
            }
            self.key_selection.replace(drag.selection_before);
            self.reconcile_dopesheet_selection();
        }
    }
    pub(super) fn update_dopesheet_drag(&mut self, pointer_x: f32, unsnapped: bool) {
        let Some(mut drag) = self.key_drag.take() else {
            return;
        };
        if !drag.active {
            // Finish a previous numeric-control edit before capturing this gesture's base.
            self.session.commit_transaction();
            self.session.begin_transaction();
            drag.active = true;
            self.key_selection.replace(drag.selection_at(0.));
        }
        self.playing = false;
        let offset = drag.offset_at(pointer_x, self.session.document().fps, unsnapped);
        let result = offset
            .ok_or_else(|| "Invalid key drag position".to_string())
            .and_then(|offset| {
                self.session.preview_transaction(drag.command(offset))?;
                Ok(offset)
            });
        match result {
            Ok(offset) => {
                drag.offset = offset;
                self.key_selection.replace(drag.selection_at(offset));
                self.key_drag = Some(drag);
            }
            Err(error) => {
                self.session.cancel_transaction();
                self.key_selection.replace(drag.selection_before);
                self.reconcile_dopesheet_selection();
                self.error = Some(error);
            }
        }
    }
    pub(super) fn finish_dopesheet_drag(&mut self) {
        if let Some(drag) = self.key_drag.take()
            && drag.active
        {
            self.session.commit_transaction();
            self.status = if drag.offset == 0. {
                "Key timing unchanged".into()
            } else {
                format!(
                    "{} {} keys",
                    if drag.copy { "Duplicated" } else { "Moved" },
                    drag.sources.len()
                )
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(time: f32) -> KeyId {
        KeyId {
            property: Property::Rotation,
            time,
        }
    }
    fn geometry() -> ChartGeometry {
        ChartGeometry {
            start: 100.,
            width: 240.,
            duration: 2.4,
        }
    }
    #[test]
    fn fixed_mapping_snaps_relative_offsets_and_clamps_the_whole_group_at_zero() {
        let drag = KeyDrag::new(
            0,
            8,
            key(0.12345),
            &[key(0.12345), key(0.6)],
            geometry(),
            140.,
            false,
        )
        .unwrap();
        let offset = drag.offset_at(152., 30, false).unwrap();
        assert!((drag.selection_at(offset)[0].time - 7. / 30.).abs() < 1e-6);
        assert!((drag.offset_at(152., 30, true).unwrap() - 0.12).abs() < 1e-6);
        assert_eq!(drag.offset_at(-100., 30, false), Some(-0.12345));
        let selected = drag.selection_at(-0.12345);
        assert_eq!(selected[0].time, 0.);
        assert!((selected[1].time - 0.47655).abs() < 1e-6);
        assert_eq!(drag.geometry.duration, 2.4);
        assert!(drag.offset_at(f32::NAN, 30, false).is_none());
    }
    #[test]
    fn pressing_an_unselected_key_moves_only_it_and_keeps_the_prior_selection_for_cancel() {
        let drag = KeyDrag::new(
            0,
            8,
            key(0.9),
            &[key(0.3), key(0.6)],
            geometry(),
            140.,
            true,
        )
        .unwrap();
        assert_eq!(drag.selection_at(0.), vec![key(0.9)]);
        assert_eq!(drag.selection_before, vec![key(0.3), key(0.6)]);
        assert!(drag.copy);
        assert_eq!(drag.sources[0].bone, 8);
        assert!(!drag.active);
    }
}
