//! Pure typed presentation selection. Playhead movement never implies key selection.
use std::sync::Arc;

#[cfg(test)]
use peachy_document::Document;
use peachy_document::{Property, SliderProperty};
use peachy_engine::{
    AnimationKey, Command, KeyOwner, Session, UnsupportedAnimationTimeline,
    animation_key_inventory, animation_keys,
};

pub(super) type KeyId = AnimationKey;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TimelineRow {
    Bone(Property),
    Ik,
    Slider(SliderProperty),
    Deform,
    SlotAttachment,
    SlotColor,
    DrawOrder,
}
impl TimelineRow {
    pub fn matches(self, key: AnimationKey) -> bool {
        match (self, key) {
            (Self::Bone(a), AnimationKey::Bone { property: b, .. }) => a == b,
            (Self::Ik, AnimationKey::Ik { .. }) => true,
            (Self::Slider(a), AnimationKey::Slider { property: b, .. }) => a == b,
            (Self::Deform, AnimationKey::Deform { .. }) => true,
            (Self::SlotAttachment, AnimationKey::SlotAttachment { .. }) => true,
            (Self::SlotColor, AnimationKey::SlotColor { .. }) => true,
            (Self::DrawOrder, AnimationKey::DrawOrder { .. }) => true,
            _ => false,
        }
    }
}
pub(super) fn rows(owner: Option<KeyOwner>) -> Vec<TimelineRow> {
    match owner {
        Some(KeyOwner::Bone { .. }) => Property::ALL.into_iter().map(TimelineRow::Bone).collect(),
        Some(KeyOwner::Ik { .. }) => vec![TimelineRow::Ik],
        Some(KeyOwner::Slider { .. }) => SliderProperty::ALL
            .into_iter()
            .map(TimelineRow::Slider)
            .collect(),
        Some(KeyOwner::Deform { .. }) => vec![TimelineRow::Deform],
        Some(KeyOwner::Slot { .. }) => vec![TimelineRow::SlotAttachment, TimelineRow::SlotColor],
        Some(KeyOwner::DrawOrder {}) => vec![TimelineRow::DrawOrder],
        None => vec![],
    }
}
#[cfg(test)]
pub(super) fn row_keys(
    document: &Document,
    animation: usize,
    owner: KeyOwner,
    row: TimelineRow,
) -> Vec<AnimationKey> {
    let Some(clip) = document.animations.get(animation) else {
        return vec![];
    };
    match (owner, row) {
        (KeyOwner::Slider { constraint }, TimelineRow::Slider(property)) => clip
            .slider_tracks
            .iter()
            .find(|track| track.constraint == constraint && track.property == property)
            .map_or_else(Vec::new, |track| {
                track
                    .keys
                    .iter()
                    .map(|key| AnimationKey::Slider {
                        constraint,
                        property,
                        time: key.time,
                    })
                    .collect()
            }),
        (KeyOwner::DrawOrder {}, TimelineRow::DrawOrder) => clip
            .draw_order
            .iter()
            .map(|key| AnimationKey::DrawOrder { time: key.time })
            .collect(),
        (KeyOwner::Bone { bone }, TimelineRow::Bone(property)) => clip
            .tracks
            .iter()
            .find(|track| track.bone == bone && track.property == property)
            .map_or_else(Vec::new, |track| {
                track
                    .keys
                    .iter()
                    .map(|key| AnimationKey::Bone {
                        bone,
                        property,
                        time: key.time,
                    })
                    .collect()
            }),
        (KeyOwner::Ik { constraint }, TimelineRow::Ik) => clip
            .ik_tracks
            .iter()
            .find(|track| track.constraint == constraint)
            .map_or_else(Vec::new, |track| {
                track
                    .keys
                    .iter()
                    .map(|key| AnimationKey::Ik {
                        constraint,
                        time: key.time,
                    })
                    .collect()
            }),
        (KeyOwner::Deform { attachment }, TimelineRow::Deform) => clip
            .deform_tracks
            .iter()
            .find(|track| track.attachment == attachment)
            .map_or_else(Vec::new, |track| {
                track
                    .keys
                    .iter()
                    .map(|key| AnimationKey::Deform {
                        attachment: attachment.clone(),
                        time: key.time,
                    })
                    .collect()
            }),
        (KeyOwner::Slot { slot }, TimelineRow::SlotAttachment) => clip
            .attachment_tracks
            .iter()
            .find(|track| track.slot == slot)
            .map_or_else(Vec::new, |track| {
                track
                    .keys
                    .iter()
                    .map(|key| AnimationKey::SlotAttachment {
                        slot,
                        time: key.time,
                    })
                    .collect()
            }),
        (KeyOwner::Slot { slot }, TimelineRow::SlotColor) => clip
            .color_tracks
            .iter()
            .find(|track| track.slot == slot)
            .map_or_else(Vec::new, |track| {
                track
                    .keys
                    .iter()
                    .map(|key| AnimationKey::SlotColor {
                        slot,
                        time: key.time,
                    })
                    .collect()
            }),
        _ => vec![],
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum SelectionScope {
    #[default]
    None,
    Rows,
    Shown,
}

#[derive(Clone, Default)]
pub(super) struct ShownKeys {
    pub owners: Vec<KeyOwner>,
    pub keys: Vec<AnimationKey>,
    pub unsupported: Vec<UnsupportedAnimationTimeline>,
    pub readonly_owners: Vec<KeyOwner>,
}

struct TimelineCache {
    generation: u64,
    animation: Option<usize>,
    owner: Option<KeyOwner>,
    shown: Result<Arc<ShownKeys>, String>,
}

#[derive(Default)]
pub(super) struct KeySelection {
    context: (Option<usize>, Option<KeyOwner>),
    keys: Vec<AnimationKey>,
    primary: Option<AnimationKey>,
    cache: Option<TimelineCache>,
    expansion: SelectionScope,
}
impl KeySelection {
    #[cfg(test)]
    pub fn invalidate_cache(&mut self) {
        self.cache = None;
    }
    /// Cache validated metadata, never mesh payloads, by the accepted Session document.
    pub fn prepare(
        &mut self,
        session: &Session,
        animation: Option<usize>,
        owner: Option<KeyOwner>,
    ) -> Result<(), String> {
        let generation = session.document_generation();
        if self.cache.as_ref().is_none_or(|cached| {
            cached.generation != generation
                || cached.animation != animation
                || cached.owner != owner
        }) {
            let shown = (|| {
                let Some(animation) = animation else {
                    return Ok(Arc::new(ShownKeys::default()));
                };
                let (owners, unsupported) = if let Some(owner) = &owner {
                    (vec![owner.clone()], None)
                } else {
                    let inventory = animation_key_inventory(session.document(), animation)?;
                    (inventory.owners, Some(inventory.unsupported_timelines))
                };
                let query = animation_keys(session.document(), animation, &owners)?;
                Ok(Arc::new(ShownKeys {
                    owners,
                    keys: query.keys,
                    unsupported: unsupported.unwrap_or(query.unsupported_timelines),
                    readonly_owners: vec![],
                }))
            })();
            self.cache = Some(TimelineCache {
                generation,
                animation,
                owner: owner.clone(),
                shown,
            });
        }
        let shown = self
            .cache
            .as_ref()
            .unwrap()
            .shown
            .as_ref()
            .map_err(Clone::clone)?;
        let shown_keys = &shown.keys;
        if self.context.0 != animation {
            self.keys.clear();
            self.primary = None;
            self.expansion = SelectionScope::None;
        }
        if self.context != (animation, owner.clone()) {
            self.expansion = SelectionScope::None;
        }
        self.context = (animation, owner);
        let previous_len = self.keys.len();
        self.keys.retain(|key| shown_keys.contains(key));
        if self.keys.len() != previous_len {
            self.expansion = SelectionScope::None;
        }
        if self
            .primary
            .as_ref()
            .is_none_or(|key| !self.keys.contains(key))
        {
            self.primary = self.keys.first().cloned();
        }
        Ok(())
    }
    pub fn shown(&self) -> Arc<ShownKeys> {
        self.cache
            .as_ref()
            .and_then(|cache| cache.shown.as_ref().ok())
            .cloned()
            .unwrap_or_default()
    }
    #[cfg(test)]
    pub fn reconcile(
        &mut self,
        document: &Document,
        animation: Option<usize>,
        owner: Option<KeyOwner>,
    ) {
        // Legacy pure selection fixtures do not have a live Session cache token.
        let shown = animation.map_or_else(Vec::new, |animation| {
            if let Some(owner) = &owner {
                rows(Some(owner.clone()))
                    .into_iter()
                    .flat_map(|row| row_keys(document, animation, owner.clone(), row))
                    .collect()
            } else {
                animation_key_inventory(document, animation)
                    .ok()
                    .and_then(|inventory| {
                        animation_keys(document, animation, &inventory.owners).ok()
                    })
                    .map_or_else(Vec::new, |query| query.keys)
            }
        });
        if self.context.0 != animation {
            self.clear();
        }
        if self.context != (animation, owner.clone()) {
            self.expansion = SelectionScope::None;
        }
        self.context = (animation, owner);
        let previous_len = self.keys.len();
        self.keys.retain(|key| shown.contains(key));
        if self.keys.len() != previous_len {
            self.expansion = SelectionScope::None;
        }
        if self
            .primary
            .as_ref()
            .is_none_or(|key| !self.keys.contains(key))
        {
            self.primary = self.keys.first().cloned();
        }
    }
    pub fn mark_readonly(&mut self, owners: Vec<KeyOwner>) {
        if let Some(cache) = &mut self.cache
            && let Ok(shown) = &mut cache.shown
        {
            if shown.readonly_owners != owners {
                Arc::make_mut(shown).readonly_owners = owners;
            }
            self.keys
                .retain(|key| !shown.readonly_owners.contains(&key.owner()));
            if self
                .primary
                .as_ref()
                .is_none_or(|key| !self.keys.contains(key))
            {
                self.primary = self.keys.first().cloned();
            }
        }
    }
    pub fn primary(&self) -> Option<&AnimationKey> {
        self.primary.as_ref()
    }
    pub fn set_primary(&mut self, primary: Option<AnimationKey>) {
        self.primary = primary
            .filter(|key| self.keys.contains(key))
            .or_else(|| self.keys.first().cloned());
    }
    pub fn select(&mut self, key: AnimationKey, toggle: bool) {
        self.expansion = SelectionScope::None;
        if toggle {
            if let Some(index) = self.keys.iter().position(|selected| selected == &key) {
                self.keys.remove(index);
            } else {
                self.keys.push(key);
            }
        } else {
            self.keys.clear();
            self.keys.push(key);
        }
        if !toggle
            || self
                .primary
                .as_ref()
                .is_none_or(|key| !self.keys.contains(key))
        {
            self.primary = self.keys.first().cloned();
        }
    }
    pub fn contains(&self, key: AnimationKey) -> bool {
        self.keys.contains(&key)
    }
    pub fn keys(&self) -> &[AnimationKey] {
        &self.keys
    }
    pub fn replace(&mut self, keys: Vec<AnimationKey>) {
        self.clear();
        for key in keys {
            if !self.keys.contains(&key) {
                self.keys.push(key);
            }
        }
        self.primary = self.keys.first().cloned();
    }
    pub fn clear(&mut self) {
        self.keys.clear();
        self.primary = None;
        self.expansion = SelectionScope::None;
    }
    /// Expand to selected rows, then all shown rows. A grouped IK key has one row.
    #[cfg(test)]
    pub fn select_all(
        &mut self,
        document: &Document,
        animation: Option<usize>,
        owner: Option<KeyOwner>,
        shown_rows: &[TimelineRow],
    ) -> SelectionScope {
        self.reconcile(document, animation, owner.clone());
        let Some((animation, owner)) = animation.zip(owner) else {
            self.clear();
            return SelectionScope::None;
        };
        let mut shown = Vec::new();
        for &row in shown_rows {
            for key in row_keys(document, animation, owner.clone(), row) {
                if !shown.contains(&key) {
                    shown.push(key);
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
                    || shown_rows.iter().any(|&row| {
                        row.matches(key.clone())
                            && self.keys.iter().any(|selected| {
                                selected.owner() == key.owner() && row.matches(selected.clone())
                            })
                    })
            })
            .collect();
        self.set_primary(self.primary.clone());
        self.expansion = scope;
        scope
    }
    pub fn select_all_shown(&mut self) -> Result<SelectionScope, String> {
        let shown = self.shown();
        let scope = if self.keys.is_empty() || self.expansion != SelectionScope::None {
            SelectionScope::Shown
        } else {
            SelectionScope::Rows
        };
        if scope == SelectionScope::Shown
            && (!shown.unsupported.is_empty() || !shown.readonly_owners.is_empty())
        {
            return Err(
                "Shown timelines include read-only keys. Select editable key rows explicitly."
                    .into(),
            );
        }
        let selected = &self.keys;
        let keys = shown
            .keys
            .iter()
            .filter(|key| !shown.readonly_owners.contains(&key.owner()))
            .filter(|key| {
                scope == SelectionScope::Shown
                    || selected.iter().any(|selected| {
                        selected.owner() == key.owner()
                            && rows(Some(key.owner())).into_iter().any(|row| {
                                row.matches((**key).clone()) && row.matches(selected.clone())
                            })
                    })
            })
            .cloned()
            .collect();
        let primary = self.primary.clone();
        self.replace(keys);
        self.set_primary(primary);
        self.expansion = if self.keys.is_empty() {
            SelectionScope::None
        } else {
            scope
        };
        Ok(self.expansion)
    }
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
    pub fn len(&self) -> usize {
        self.keys.len()
    }
    pub fn deletion(&self) -> Option<Command> {
        let (Some(animation), _) = &self.context else {
            return None;
        };
        (!self.keys.is_empty()).then(|| Command::DeleteAnimationKeys {
            animation: *animation,
            keys: self.keys.clone(),
        })
    }
    pub fn diagnostic(&self) -> serde_json::Value {
        serde_json::json!({"animation":self.context.0,"owner":self.context.1,"keys":self.keys,"primary":self.primary,
            "shownOwners":self.cache.as_ref().and_then(|cache| cache.shown.as_ref().ok()).map(|shown| &shown.owners),
            "readOnlyTimelines":self.cache.as_ref().and_then(|cache| cache.shown.as_ref().ok()).map(|shown| &shown.unsupported),
            "readOnlyOwners":self.cache.as_ref().and_then(|cache| cache.shown.as_ref().ok()).map(|shown| &shown.readonly_owners),
            "documentGeneration":self.cache.as_ref().map(|cache| cache.generation)})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use peachy_document::*;
    use peachy_engine::Session;

    #[test]
    fn grouped_ik_selection_never_aliases_bone_or_other_constraint_identities() {
        let editor = super::super::test_support::animated_ik_editor();
        let document = editor.session.document();
        let mut selection = KeySelection::default();
        selection.reconcile(document, Some(0), Some(KeyOwner::Bone { bone: 1 }));
        selection.select(
            KeyId::Bone {
                bone: 1,
                property: Property::Rotation,
                time: 0.,
            },
            false,
        );
        let owner = Some(KeyOwner::Ik { constraint: 1 });
        selection.reconcile(document, Some(0), owner.clone());
        assert!(selection.is_empty());
        assert_eq!(
            selection.select_all(document, Some(0), owner.clone(), &rows(owner)),
            SelectionScope::Shown
        );
        assert_eq!(
            selection.keys(),
            &[
                KeyId::Ik {
                    constraint: 1,
                    time: 0.
                },
                KeyId::Ik {
                    constraint: 1,
                    time: 1.
                }
            ]
        );
        selection.reconcile(document, Some(0), Some(KeyOwner::Ik { constraint: 0 }));
        assert!(selection.is_empty());
        assert!(selection.deletion().is_none());
    }

    fn doc() -> Document {
        let mut doc: Document = peachy_io::decode_project(include_str!(
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
        KeyId::Bone {
            bone: 0,
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
        selection.select_all(
            doc,
            Some(0),
            Some(KeyOwner::Bone { bone: 0 }),
            &rows(Some(KeyOwner::Bone { bone: 0 })),
        )
    }

    fn x(time: f32) -> KeyId {
        KeyId::Bone {
            bone: 0,
            property: Property::X,
            time,
        }
    }

    #[test]
    fn select_all_expands_one_key_to_row_then_shown_keys_without_merging_fractional_keys() {
        let doc = expansion_doc();
        let before = doc.clone();
        let mut selection = KeySelection::default();
        selection.reconcile(&doc, Some(0), Some(KeyOwner::Bone { bone: 0 }));
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
        let shown = [
            TimelineRow::Bone(Property::Y),
            TimelineRow::Bone(Property::X),
            TimelineRow::Bone(Property::X),
        ];
        assert_eq!(
            selection.select_all(&doc, Some(0), Some(KeyOwner::Bone { bone: 0 }), &shown),
            SelectionScope::Shown
        );
        assert_eq!(selection.len(), 7);
        assert!(matches!(
            selection.keys()[0],
            AnimationKey::Bone {
                property: Property::Y,
                ..
            }
        ));
        assert!(!selection.contains(key(0.5)));
        assert!(!selection.contains(x(0.8)));

        // A hidden row cannot bring its other keys into a visible-row action.
        selection.select(key(0.5), false);
        assert_eq!(
            selection.select_all(&doc, Some(0), Some(KeyOwner::Bone { bone: 0 }), &shown),
            SelectionScope::Shown
        );
        assert_eq!(selection.len(), 7);
        assert!(!selection.contains(key(0.5)));

        assert_eq!(
            selection.select_all(
                &doc,
                Some(0),
                Some(KeyOwner::Bone { bone: 1 }),
                &rows(Some(KeyOwner::Bone { bone: 0 }))
            ),
            SelectionScope::Shown
        );
        assert_eq!(
            selection.keys(),
            &[KeyId::Bone {
                bone: 1,
                property: Property::Rotation,
                time: 0.9
            }]
        );
        assert_eq!(
            selection.select_all(
                &doc,
                Some(1),
                Some(KeyOwner::Bone { bone: 0 }),
                &rows(Some(KeyOwner::Bone { bone: 0 }))
            ),
            SelectionScope::Shown
        );
        assert_eq!(selection.keys(), &[x(0.8)]);
    }

    #[test]
    fn multiple_selected_rows_expand_to_their_union_before_all_shown_rows() {
        let doc = expansion_doc();
        let mut selection = KeySelection::default();
        selection.reconcile(&doc, Some(0), Some(KeyOwner::Bone { bone: 0 }));
        selection.select(x(0.4), false);
        selection.select(
            KeyId::Bone {
                bone: 0,
                property: Property::Y,
                time: 0.25,
            },
            true,
        );
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Rows);
        assert_eq!(selection.len(), 7);
        assert!(selection.keys().iter().all(|key| {
            matches!(
                key,
                AnimationKey::Bone {
                    property: Property::X | Property::Y,
                    ..
                }
            )
        }));
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Shown);
        assert_eq!(selection.len(), 9);
    }

    #[test]
    fn singleton_rows_consume_first_phase_and_manual_actions_reset_it() {
        let doc = expansion_doc();
        let mut selection = KeySelection::default();
        selection.reconcile(&doc, Some(0), Some(KeyOwner::Bone { bone: 0 }));
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
        selection.reconcile(&doc, Some(0), Some(KeyOwner::Bone { bone: 0 }));
        selection.select(x(0.12345), false);
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Rows);
        for _ in 0..3 {
            selection.reconcile(&doc, Some(0), Some(KeyOwner::Bone { bone: 0 }));
        }
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Shown);

        selection.reconcile(&doc, Some(1), Some(KeyOwner::Bone { bone: 0 }));
        assert!(selection.is_empty());
        selection.select(x(0.8), false);
        assert_eq!(
            selection.select_all(
                &doc,
                Some(1),
                Some(KeyOwner::Bone { bone: 0 }),
                &rows(Some(KeyOwner::Bone { bone: 0 }))
            ),
            SelectionScope::Rows
        );
        selection.reconcile(&doc, Some(0), Some(KeyOwner::Bone { bone: 0 }));
        assert!(selection.is_empty());
        assert_eq!(expand(&mut selection, &doc), SelectionScope::Shown);

        // A vanished fractional key must not be reassigned to its close neighbor.
        selection.replace(vec![x(0.12345)]);
        let mut removed = doc.clone();
        removed.animations[0].tracks[0].keys.remove(0);
        selection.reconcile(&removed, Some(0), Some(KeyOwner::Bone { bone: 0 }));
        assert!(selection.is_empty());
        assert!(!selection.contains(x(0.123455)));
    }

    #[test]
    fn stale_pruning_resets_expansion_and_undo_does_not_restore_stale_selection_or_change_history()
    {
        let before = expansion_doc();
        let mut session = Session::new(before.clone()).unwrap();
        let mut selection = KeySelection::default();
        selection.reconcile(
            session.document(),
            Some(0),
            Some(KeyOwner::Bone { bone: 0 }),
        );
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
        selection.reconcile(
            session.document(),
            Some(0),
            Some(KeyOwner::Bone { bone: 0 }),
        );
        assert_eq!(selection.len(), 3);
        assert!(!selection.contains(x(0.12345)));
        assert!(selection.contains(x(0.123455)));

        session.undo();
        selection.reconcile(
            session.document(),
            Some(0),
            Some(KeyOwner::Bone { bone: 0 }),
        );
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
        selection.reconcile(
            session.document(),
            Some(0),
            Some(KeyOwner::Bone { bone: 0 }),
        );
        assert!(selection.is_empty());
        assert_eq!(
            expand(&mut selection, session.document()),
            SelectionScope::None
        );
        session.undo();
        assert_eq!(session.document(), &before);
        assert!(!session.can_undo());
        assert!(session.can_redo());
        selection.reconcile(
            session.document(),
            Some(0),
            Some(KeyOwner::Bone { bone: 0 }),
        );
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
        let all = rows(Some(KeyOwner::Bone { bone: 0 }));
        for (animation, bone, shown) in [
            (None, Some(KeyOwner::Bone { bone: 0 }), all.as_slice()),
            (Some(0), None, all.as_slice()),
            (
                Some(doc.animations.len()),
                Some(KeyOwner::Bone { bone: 0 }),
                all.as_slice(),
            ),
            (
                Some(0),
                Some(KeyOwner::Bone {
                    bone: doc.bones.len(),
                }),
                all.as_slice(),
            ),
            (Some(0), Some(KeyOwner::Bone { bone: 0 }), [].as_slice()),
            (
                Some(0),
                Some(KeyOwner::Bone { bone: 0 }),
                [TimelineRow::Bone(Property::ScaleY)].as_slice(),
            ),
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
        selection.reconcile(&doc, Some(0), Some(KeyOwner::Bone { bone: 0 }));
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
        selection.reconcile(&doc, Some(0), Some(KeyOwner::Bone { bone: 0 }));
        selection.select(key(0.12345), false);
        doc.animations[0].tracks[0].keys.remove(0);
        selection.reconcile(&doc, Some(0), Some(KeyOwner::Bone { bone: 0 }));
        assert_eq!(selection.len(), 0);
        selection.select(key(0.5), false);
        selection.reconcile(&doc, Some(0), Some(KeyOwner::Bone { bone: 1 }));
        assert!(selection.deletion().is_none());
        selection.reconcile(&doc, Some(0), Some(KeyOwner::Bone { bone: 0 }));
        selection.select(key(0.5), false);
        selection.reconcile(&doc, None, Some(KeyOwner::Bone { bone: 0 }));
        assert!(selection.deletion().is_none());
    }

    #[test]
    fn selected_key_deletion_is_one_undo_entry_and_preserves_neighbor_values_and_curves() {
        let before = doc();
        let mut selection = KeySelection::default();
        selection.reconcile(&before, Some(0), Some(KeyOwner::Bone { bone: 0 }));
        selection.select(key(0.12345), false);
        selection.select(key(0.5), true);
        let mut session = Session::new(before.clone()).unwrap();
        session.dispatch(selection.deletion().unwrap()).unwrap();
        assert_eq!(
            session.document().animations[0].tracks[0].keys,
            vec![before.animations[0].tracks[0].keys[1]]
        );
        selection.reconcile(
            session.document(),
            Some(0),
            Some(KeyOwner::Bone { bone: 0 }),
        );
        assert_eq!(selection.len(), 0);
        session.undo();
        assert_eq!(session.document(), &before);
        session.redo();
        assert_eq!(session.document().animations[0].tracks[0].keys.len(), 1);
    }
}
