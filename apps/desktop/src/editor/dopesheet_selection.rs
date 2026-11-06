//! Pure presentation selection. Playhead movement never implies key selection.
use peachy_document::{Document, Property};
use peachy_engine::Command;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct KeyId {
    pub property: Property,
    /// The exact source time, including imported fractional-frame keys.
    pub time: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum SelectionScope {
    #[default]
    None,
    Rows,
    Shown,
}

#[derive(Default)]
pub(super) struct KeySelection {
    context: (Option<usize>, Option<usize>),
    keys: Vec<KeyId>,
    expansion: SelectionScope,
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
        let previous_len = self.keys.len();
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
        if self.keys.len() != previous_len {
            self.expansion = SelectionScope::None;
        }
    }
    pub fn select(&mut self, key: KeyId, toggle: bool) {
        self.expansion = SelectionScope::None;
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
    pub fn keys(&self) -> &[KeyId] {
        &self.keys
    }
    pub fn replace(&mut self, keys: Vec<KeyId>) {
        self.clear();
        for key in keys {
            if !self.keys.contains(&key) {
                self.keys.push(key);
            }
        }
    }
    pub fn clear(&mut self) {
        self.keys.clear();
        self.expansion = SelectionScope::None;
    }
    /// Expand once to selected property rows, then to every shown property row.
    /// Empty selections start at all shown keys. The phase is explicit because a
    /// singleton row still consumes the first expansion without changing keys.
    pub fn select_all(
        &mut self,
        document: &Document,
        animation: Option<usize>,
        bone: Option<usize>,
        shown_rows: &[Property],
    ) -> SelectionScope {
        self.reconcile(document, animation, bone);
        let Some((clip, bone)) = animation
            .and_then(|index| document.animations.get(index))
            .zip(bone.filter(|&index| index < document.bones.len()))
        else {
            self.clear();
            return SelectionScope::None;
        };
        let mut shown = Vec::new();
        for &property in shown_rows {
            if let Some(track) = clip
                .tracks
                .iter()
                .find(|track| track.bone == bone && track.property == property)
            {
                for key in &track.keys {
                    let key = KeyId {
                        property,
                        time: key.time,
                    };
                    if !shown.contains(&key) {
                        shown.push(key);
                    }
                }
            }
        }
        let previous_len = self.keys.len();
        self.keys.retain(|key| shown.contains(key));
        if self.keys.len() != previous_len {
            self.expansion = SelectionScope::None;
        }
        if shown.is_empty() {
            self.clear();
            return SelectionScope::None;
        }
        let scope = if self.keys.is_empty() || self.expansion != SelectionScope::None {
            SelectionScope::Shown
        } else {
            SelectionScope::Rows
        };
        self.keys = shown
            .into_iter()
            .filter(|key| {
                scope == SelectionScope::Shown
                    || self
                        .keys
                        .iter()
                        .any(|selected| selected.property == key.property)
            })
            .collect();
        self.expansion = scope;
        scope
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

    fn row(property: Property, times: &[f32]) -> Track {
        Track {
            bone: 0,
            property,
            keys: times
                .iter()
                .enumerate()
                .map(|(i, &time)| Keyframe {
                    time,
                    value: property.neutral() + i as f32,
                    curve: Curve::Linear,
                })
                .collect(),
        }
    }

    fn expansion_doc() -> Document {
        let mut doc = doc();
        doc.animations.truncate(1);
        doc.animations[0].tracks = vec![
            row(Property::X, &[0.12345, 0.123455, 0.4, 0.5]),
            row(Property::Y, &[0., 0.25, 0.5]),
            row(Property::Rotation, &[0.5]),
            row(Property::ScaleX, &[0.75]),
            row(Property::ScaleY, &[]),
        ];
        let mut other_bone = row(Property::Rotation, &[0.9]);
        other_bone.bone = 1;
        doc.animations[0].tracks.push(other_bone);
        let mut other_clip = doc.animations[0].clone();
        other_clip.name = "Other clip".into();
        other_clip.tracks = vec![row(Property::X, &[0.8])];
        doc.animations.push(other_clip);
        doc.validate().unwrap();
        doc
    }

    fn expand(selection: &mut KeySelection, doc: &Document) -> SelectionScope {
        selection.select_all(doc, Some(0), Some(0), &Property::ALL)
    }

    fn x(time: f32) -> KeyId {
        KeyId {
            property: Property::X,
            time,
        }
    }

    #[test]
    fn select_all_expands_one_key_to_row_then_shown_keys_without_merging_fractional_keys() {
        let doc = expansion_doc();
        let before = doc.clone();
        let mut selection = KeySelection::default();
        selection.reconcile(&doc, Some(0), Some(0));
        selection.select(x(0.12345), false);
        assert_eq!(selection.len(), 1);
        assert!(!selection.contains(x(0.123455)));

        assert_eq!(expand(&mut selection, &doc), SelectionScope::Rows);
        assert_eq!(selection.len(), 4);
        assert!(selection.contains(x(0.12345)));
        assert!(selection.contains(x(0.123455)));

        assert_eq!(expand(&mut selection, &doc), SelectionScope::Shown);
        assert_eq!(selection.len(), 9);
        let all = selection.keys().to_vec();
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Shown);
        assert_eq!(selection.keys(), all);
        assert!(!selection.contains(key(0.9)));
        assert!(!selection.contains(x(0.8)));
        assert_eq!(doc, before);
    }

    #[test]
    fn empty_selection_selects_only_shown_rows_in_current_bone_and_clip() {
        let doc = expansion_doc();
        let mut selection = KeySelection::default();
        let shown = [Property::Y, Property::X, Property::X];
        assert_eq!(
            selection.select_all(&doc, Some(0), Some(0), &shown),
            SelectionScope::Shown
        );
        assert_eq!(selection.len(), 7);
        assert_eq!(selection.keys()[0].property, Property::Y);
        assert!(!selection.contains(key(0.5)));
        assert!(!selection.contains(x(0.8)));

        // A hidden row cannot bring its other keys into a visible-row action.
        selection.select(key(0.5), false);
        assert_eq!(
            selection.select_all(&doc, Some(0), Some(0), &shown),
            SelectionScope::Shown
        );
        assert_eq!(selection.len(), 7);
        assert!(!selection.contains(key(0.5)));

        assert_eq!(
            selection.select_all(&doc, Some(0), Some(1), &Property::ALL),
            SelectionScope::Shown
        );
        assert_eq!(selection.keys(), &[key(0.9)]);
        assert_eq!(
            selection.select_all(&doc, Some(1), Some(0), &Property::ALL),
            SelectionScope::Shown
        );
        assert_eq!(selection.keys(), &[x(0.8)]);
    }

    #[test]
    fn multiple_selected_rows_expand_to_their_union_before_all_shown_rows() {
        let doc = expansion_doc();
        let mut selection = KeySelection::default();
        selection.reconcile(&doc, Some(0), Some(0));
        selection.select(x(0.4), false);
        selection.select(
            KeyId {
                property: Property::Y,
                time: 0.25,
            },
            true,
        );
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Rows);
        assert_eq!(selection.len(), 7);
        assert!(
            selection
                .keys()
                .iter()
                .all(|key| { matches!(key.property, Property::X | Property::Y) })
        );
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Shown);
        assert_eq!(selection.len(), 9);
    }

    #[test]
    fn singleton_rows_consume_first_phase_and_manual_actions_reset_it() {
        let doc = expansion_doc();
        let mut selection = KeySelection::default();
        selection.reconcile(&doc, Some(0), Some(0));
        selection.select(key(0.5), false);
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Rows);
        assert_eq!(selection.len(), 1);
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Shown);
        assert_eq!(selection.len(), 9);

        selection.select(key(0.5), false);
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Rows);
        selection.select(key(0.5), false);
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Rows);
        selection.replace(vec![key(0.5), key(0.5)]);
        assert_eq!(selection.len(), 1);
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Rows);
        selection.select(key(0.5), true);
        selection.select(key(0.5), true);
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Rows);

        selection.clear();
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Shown);
        assert_eq!(selection.len(), 9);
    }

    #[test]
    fn reconciliation_preserves_phase_until_context_changes_or_exact_keys_are_pruned() {
        let doc = expansion_doc();
        let mut selection = KeySelection::default();
        selection.reconcile(&doc, Some(0), Some(0));
        selection.select(x(0.12345), false);
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Rows);
        for _ in 0..3 {
            selection.reconcile(&doc, Some(0), Some(0));
        }
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Shown);

        selection.reconcile(&doc, Some(1), Some(0));
        assert!(selection.is_empty());
        selection.select(x(0.8), false);
        assert_eq!(
            selection.select_all(&doc, Some(1), Some(0), &Property::ALL),
            SelectionScope::Rows
        );
        selection.reconcile(&doc, Some(0), Some(0));
        assert!(selection.is_empty());
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Shown);

        // A vanished fractional key must not be reassigned to its close neighbor.
        selection.replace(vec![x(0.12345)]);
        let mut removed = doc.clone();
        removed.animations[0].tracks[0].keys.remove(0);
        selection.reconcile(&removed, Some(0), Some(0));
        assert!(selection.is_empty());
        assert!(!selection.contains(x(0.123455)));
    }

    #[test]
    fn stale_pruning_resets_expansion_and_undo_does_not_restore_stale_selection_or_change_history()
    {
        let before = expansion_doc();
        let mut session = Session::new(before.clone()).unwrap();
        let mut selection = KeySelection::default();
        selection.reconcile(session.document(), Some(0), Some(0));
        selection.select(x(0.12345), false);
        assert_eq!(
            expand(&mut selection, session.document()),
            SelectionScope::Rows
        );
        assert_eq!(selection.len(), 4);
        session
            .dispatch(Command::DeleteKey {
                animation: 0,
                bone: 0,
                property: Property::X,
                time: 0.12345,
            })
            .unwrap();
        selection.reconcile(session.document(), Some(0), Some(0));
        assert_eq!(selection.len(), 3);
        assert!(!selection.contains(x(0.12345)));
        assert!(selection.contains(x(0.123455)));

        session.undo();
        selection.reconcile(session.document(), Some(0), Some(0));
        assert_eq!(selection.len(), 3);
        assert!(!selection.contains(x(0.12345)));
        assert_eq!(
            expand(&mut selection, session.document()),
            SelectionScope::Rows
        );
        assert_eq!(selection.len(), 4);
        assert_eq!(session.document(), &before);
        assert!(!session.is_dirty());
        assert!(!session.can_undo());
        assert!(session.can_redo());
        assert_eq!(
            expand(&mut selection, session.document()),
            SelectionScope::Shown
        );
        assert_eq!(selection.len(), 9);
        session.redo();
        assert_eq!(session.document().animations[0].tracks[0].keys.len(), 3);
    }

    #[test]
    fn deleting_expanded_shown_keys_is_one_undo_step_and_preserves_other_contexts() {
        let before = expansion_doc();
        let mut session = Session::new(before.clone()).unwrap();
        let mut selection = KeySelection::default();
        assert_eq!(
            expand(&mut selection, session.document()),
            SelectionScope::Shown
        );
        session.dispatch(selection.deletion().unwrap()).unwrap();
        assert_eq!(
            session.document().animations[0].tracks,
            vec![before.animations[0].tracks[5].clone()]
        );
        assert_eq!(session.document().animations[1], before.animations[1]);
        selection.reconcile(session.document(), Some(0), Some(0));
        assert!(selection.is_empty());
        assert_eq!(
            expand(&mut selection, session.document()),
            SelectionScope::None
        );
        session.undo();
        assert_eq!(session.document(), &before);
        assert!(!session.can_undo());
        assert!(session.can_redo());
        selection.reconcile(session.document(), Some(0), Some(0));
        assert!(selection.is_empty());
        assert_eq!(
            expand(&mut selection, session.document()),
            SelectionScope::Shown
        );
        assert_eq!(selection.len(), 9);
        assert!(session.can_redo());
    }

    #[test]
    fn missing_invalid_or_empty_shown_context_clears_selection_and_cannot_delete() {
        let doc = expansion_doc();
        let mut selection = KeySelection::default();
        for (animation, bone, shown) in [
            (None, Some(0), Property::ALL.as_slice()),
            (Some(0), None, Property::ALL.as_slice()),
            (
                Some(doc.animations.len()),
                Some(0),
                Property::ALL.as_slice(),
            ),
            (Some(0), Some(doc.bones.len()), Property::ALL.as_slice()),
            (Some(0), Some(0), [].as_slice()),
            (Some(0), Some(0), [Property::ScaleY].as_slice()),
        ] {
            assert_eq!(expand(&mut selection, &doc), SelectionScope::Shown);
            assert_eq!(
                selection.select_all(&doc, animation, bone, shown),
                SelectionScope::None
            );
            assert!(selection.is_empty());
            assert!(selection.deletion().is_none());
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
