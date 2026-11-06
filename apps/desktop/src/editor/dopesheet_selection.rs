//! Pure presentation selection. Playhead movement never implies key selection.
use peachy_document::{Document, Property};
use peachy_engine::Command;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct KeyId {
    pub property: Property,
    /// The exact source time, including imported fractional-frame keys.
    pub time: f32,
}

#[derive(Default)]
pub(super) struct KeySelection {
    context: (Option<usize>, Option<usize>),
    keys: Vec<KeyId>,
}

impl KeySelection {
    pub fn reconcile(
        &mut self,
        document: &Document,
        animation: Option<usize>,
        bone: Option<usize>,
    ) {
        let context = (animation, bone);
        if self.context != context {
            self.clear();
            self.context = context;
        }
        self.keys.retain(|key| {
            animation
                .and_then(|i| document.animations.get(i))
                .is_some_and(|clip| {
                    clip.tracks.iter().any(|track| {
                        Some(track.bone) == bone
                            && track.property == key.property
                            && track.keys.iter().any(|k| k.time == key.time)
                    })
                })
        });
    }
    pub fn select(&mut self, key: KeyId, toggle: bool) {
        if toggle {
            if let Some(index) = self.keys.iter().position(|selected| *selected == key) {
                self.keys.remove(index);
            } else {
                self.keys.push(key);
            }
        } else {
            self.keys.clear();
            self.keys.push(key);
        }
    }
    pub fn contains(&self, key: KeyId) -> bool {
        self.keys.contains(&key)
    }
    pub fn clear(&mut self) {
        self.keys.clear();
    }
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
    pub fn len(&self) -> usize {
        self.keys.len()
    }
    pub fn deletion(&self) -> Option<Command> {
        let (Some(animation), Some(bone)) = self.context else {
            return None;
        };
        if self.keys.is_empty() {
            return None;
        }
        Some(Command::Batch {
            commands: self
                .keys
                .iter()
                .map(|key| Command::DeleteKey {
                    animation,
                    bone,
                    property: key.property,
                    time: key.time,
                })
                .collect(),
        })
    }
    pub fn diagnostic(&self) -> serde_json::Value {
        serde_json::json!({
            "animation": self.context.0,
            "bone": self.context.1,
            "keys": self.keys.iter().map(|k|serde_json::json!({"property":k.property,"time":k.time})).collect::<Vec<_>>()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use peachy_document::*;
    use peachy_engine::Session;

    fn doc() -> Document {
        let mut doc: Document = serde_json::from_str(include_str!(
            "../../../../examples/peach-bot/mesh-bot.peachy"
        ))
        .unwrap();
        doc.animations[0].tracks = vec![Track {
            bone: 0,
            property: Property::Rotation,
            keys: vec![
                Keyframe {
                    time: 0.12345,
                    value: 10.,
                    curve: Curve::Stepped,
                },
                Keyframe {
                    time: 0.123455,
                    value: 20.,
                    curve: Curve::Bezier([0.2, 0.3, 0.8, 0.9]),
                },
                Keyframe {
                    time: 0.5,
                    value: 30.,
                    curve: Curve::Linear,
                },
            ],
        }];
        doc
    }
    fn key(time: f32) -> KeyId {
        KeyId {
            property: Property::Rotation,
            time,
        }
    }

    #[test]
    fn selection_is_exact_and_toggle_does_not_select_nearby_fractional_keys() {
        let doc = doc();
        let mut selection = KeySelection::default();
        selection.reconcile(&doc, Some(0), Some(0));
        selection.select(key(0.12345), false);
        assert!(selection.contains(key(0.12345)));
        assert!(!selection.contains(key(0.123455)));
        selection.select(key(0.5), true);
        assert_eq!(selection.len(), 2);
        selection.select(key(0.12345), true);
        assert!(!selection.contains(key(0.12345)));
        assert!(selection.contains(key(0.5)));
        selection.select(key(0.123455), false);
        assert_eq!(selection.len(), 1);
        selection.clear();
        assert!(selection.deletion().is_none());
    }

    #[test]
    fn changed_context_and_deleted_keys_cannot_redirect_selection() {
        let mut doc = doc();
        let mut selection = KeySelection::default();
        selection.reconcile(&doc, Some(0), Some(0));
        selection.select(key(0.12345), false);
        doc.animations[0].tracks[0].keys.remove(0);
        selection.reconcile(&doc, Some(0), Some(0));
        assert_eq!(selection.len(), 0);
        selection.select(key(0.5), false);
        selection.reconcile(&doc, Some(0), Some(1));
        assert!(selection.deletion().is_none());
        selection.reconcile(&doc, Some(0), Some(0));
        selection.select(key(0.5), false);
        selection.reconcile(&doc, None, Some(0));
        assert!(selection.deletion().is_none());
    }

    #[test]
    fn selected_key_deletion_is_one_undo_entry_and_preserves_neighbor_values_and_curves() {
        let before = doc();
        let mut selection = KeySelection::default();
        selection.reconcile(&before, Some(0), Some(0));
        selection.select(key(0.12345), false);
        selection.select(key(0.5), true);
        let mut session = Session::new(before.clone()).unwrap();
        session.dispatch(selection.deletion().unwrap()).unwrap();
        assert_eq!(
            session.document().animations[0].tracks[0].keys,
            vec![before.animations[0].tracks[0].keys[1]]
        );
        selection.reconcile(session.document(), Some(0), Some(0));
        assert_eq!(selection.len(), 0);
        session.undo();
        assert_eq!(session.document(), &before);
        session.redo();
        assert_eq!(session.document().animations[0].tracks[0].keys.len(), 1);
    }
}
