//! Pure typed presentation selection. Playhead movement never implies key selection.
use std::{collections::BTreeSet, sync::Arc};

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
    pub fn label(self) -> &'static str {
        match self {
            Self::Bone(Property::Rotation) => "Rotate",
            Self::Bone(Property::X) => "Translate X",
            Self::Bone(Property::Y) => "Translate Y",
            Self::Bone(Property::ScaleX) => "Scale X",
            Self::Bone(Property::ScaleY) => "Scale Y",
            Self::Bone(Property::ShearX) => "Shear X",
            Self::Bone(Property::ShearY) => "Shear Y",
            Self::Ik => "IK constraint",
            Self::Slider(SliderProperty::Time) => "Slider Time",
            Self::Slider(SliderProperty::Mix) => "Slider Mix",
            Self::Deform => "Deform",
            Self::SlotAttachment => "Attach",
            Self::SlotColor => "RGBA",
            Self::DrawOrder => "Default",
        }
    }
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

/// Presentation scope is independent of an explicit clipboard/edit target.
/// Owner order is meaningful; an empty owner list never means all owners.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum TimelineScope {
    #[default]
    All,
    Owners(Vec<KeyOwner>),
}
impl TimelineScope {
    fn canonicalized(&self, document: &Document) -> Result<Self, String> {
        match self {
            Self::All => Ok(Self::All),
            Self::Owners(owners) => {
                let mut canonical = Vec::with_capacity(owners.len());
                let mut seen = BTreeSet::new();
                for owner in owners {
                    let owner = match owner {
                        KeyOwner::Deform { attachment } => {
                            peachy_engine::deform_key_owner(document, attachment)?
                        }
                        _ => owner.clone(),
                    };
                    // A presentation union may contain linked aliases. The
                    // engine's explicit-owner query remains strict and sees
                    // only this stable, canonical union.
                    if seen.insert(owner.clone()) {
                        canonical.push(owner);
                    }
                }
                Ok(Self::Owners(canonical))
            }
        }
    }
    fn single_owner(&self) -> Option<&KeyOwner> {
        match self {
            Self::Owners(owners) if owners.len() == 1 => owners.first(),
            _ => None,
        }
    }
    fn diagnostic(&self) -> serde_json::Value {
        match self {
            Self::All => serde_json::json!({"type":"all"}),
            Self::Owners(owners) => serde_json::json!({"type":"owners","owners":owners}),
        }
    }
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
    requested_scope: TimelineScope,
    scope: TimelineScope,
    shown: Result<Arc<ShownKeys>, String>,
}

/// Gesture state only: metadata is always refreshed from the current Session
/// when restoring, so canceled preview times cannot prune the original keys.
#[derive(Clone, Debug)]
pub(super) struct KeySelectionSnapshot {
    context: (Option<usize>, TimelineScope),
    keys: Vec<AnimationKey>,
    primary: Option<AnimationKey>,
    expansion: SelectionScope,
}

#[derive(Default)]
pub(super) struct KeySelection {
    context: (Option<usize>, TimelineScope),
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
    /// Cache validated metadata, never mesh payloads, by the accepted Session
    /// document and canonical ordered scope. View-specific read-only masks are
    /// reapplied separately after preparation, including on skin changes.
    pub fn prepare(
        &mut self,
        session: &Session,
        animation: Option<usize>,
        requested_scope: TimelineScope,
    ) -> Result<(), String> {
        let generation = session.document_generation();
        if self.cache.as_ref().is_none_or(|cached| {
            cached.generation != generation
                || cached.animation != animation
                || cached.requested_scope != requested_scope
        }) {
            let shown = (|| {
                let scope = requested_scope.canonicalized(session.document())?;
                if let Some(cached) = &self.cache
                    && cached.generation == generation
                    && cached.animation == animation
                    && cached.scope == scope
                    && let Ok(shown) = &cached.shown
                {
                    return Ok((scope, Arc::clone(shown)));
                }
                let Some(animation) = animation else {
                    return Ok((scope, Arc::new(ShownKeys::default())));
                };
                let (owners, unsupported) = match &scope {
                    TimelineScope::Owners(owners) => (owners.clone(), None),
                    TimelineScope::All => {
                        let inventory = animation_key_inventory(session.document(), animation)?;
                        (inventory.owners, Some(inventory.unsupported_timelines))
                    }
                };
                let query = animation_keys(session.document(), animation, &owners)?;
                let shown = Arc::new(ShownKeys {
                    owners,
                    keys: query.keys,
                    unsupported: unsupported.unwrap_or(query.unsupported_timelines),
                    readonly_owners: vec![],
                });
                Ok((scope, shown))
            })();
            let (scope, shown) = match shown {
                Ok((scope, shown)) => (scope, Ok(shown)),
                Err(error) => (requested_scope.clone(), Err(error)),
            };
            self.cache = Some(TimelineCache {
                generation,
                animation,
                requested_scope,
                scope,
                shown,
            });
        }
        let shown = self
            .cache
            .as_ref()
            .unwrap()
            .shown
            .as_ref()
            .map(Arc::clone)
            .map_err(Clone::clone)?;
        let scope = self.cache.as_ref().unwrap().scope.clone();
        self.reconcile_keys(animation, scope, &shown.keys);
        Ok(())
    }
    fn reconcile_keys(
        &mut self,
        animation: Option<usize>,
        scope: TimelineScope,
        shown_keys: &[AnimationKey],
    ) {
        if self.context.0 != animation {
            self.keys.clear();
            self.primary = None;
            self.expansion = SelectionScope::None;
        }
        if self.context != (animation, scope.clone()) {
            self.expansion = SelectionScope::None;
        }
        self.context = (animation, scope);
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
    }
    pub fn scope(&self) -> &TimelineScope {
        &self.context.1
    }
    pub fn snapshot(&self) -> KeySelectionSnapshot {
        KeySelectionSnapshot {
            context: self.context.clone(),
            keys: self.keys.clone(),
            primary: self.primary.clone(),
            expansion: self.expansion,
        }
    }
    /// Call after canceling the Session transaction. Restore the exact selection
    /// phase only in its original canonical context; otherwise keep its valid
    /// intersection. As with prepare, the shell refreshes the current skin's
    /// read-only mask after restoration.
    pub fn restore_snapshot(
        &mut self,
        session: &Session,
        animation: Option<usize>,
        scope: TimelineScope,
        snapshot: KeySelectionSnapshot,
    ) -> Result<(), String> {
        // Keep the current view mask, not the snapshot's historical mask, when
        // canceling a preview advances the Session's document generation.
        let readonly = self.shown().readonly_owners.clone();
        self.prepare(session, animation, scope)?;
        let owners = self.shown().owners.clone();
        self.mark_readonly(
            readonly
                .into_iter()
                .filter(|owner| owners.contains(owner))
                .collect(),
        );
        let same_context = self.context == snapshot.context;
        let shown = self.shown();
        let original_len = snapshot.keys.len();
        self.keys = if self.context.0 == snapshot.context.0 {
            snapshot
                .keys
                .into_iter()
                .filter(|key| {
                    shown.keys.contains(key) && !shown.readonly_owners.contains(&key.owner())
                })
                .collect()
        } else {
            vec![]
        };
        self.set_primary(snapshot.primary);
        self.expansion = if same_context && self.keys.len() == original_len {
            snapshot.expansion
        } else {
            SelectionScope::None
        };
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
        let scope = owner.map_or(TimelineScope::All, |owner| {
            TimelineScope::Owners(vec![owner])
        });
        if self.context != (animation, scope.clone()) {
            self.expansion = SelectionScope::None;
        }
        self.context = (animation, scope);
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
            let previous_len = self.keys.len();
            self.keys
                .retain(|key| !shown.readonly_owners.contains(&key.owner()));
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
        serde_json::json!({"animation":self.context.0,"owner":self.context.1.single_owner(),"scope":self.context.1.diagnostic(),"keys":self.keys,"primary":self.primary,
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
        assert_eq!(
            Property::ALL.map(|property| TimelineRow::Bone(property).label()),
            [
                "Translate X",
                "Translate Y",
                "Rotate",
                "Scale X",
                "Scale Y",
                "Shear X",
                "Shear Y",
            ]
        );
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

    fn scope_doc() -> (Document, AttachmentRef, AttachmentRef, usize) {
        let mut doc = expansion_doc();
        let (source, mut mesh) = doc
            .skins
            .iter()
            .flat_map(|(skin, slots)| {
                slots.iter().flat_map(move |(slot, attachments)| {
                    attachments
                        .iter()
                        .filter_map(move |(placeholder, attachment)| match attachment {
                            Attachment::Mesh(mesh) if mesh.link.is_none() => Some((
                                AttachmentRef {
                                    skin: skin.clone(),
                                    slot: slot.clone(),
                                    placeholder: placeholder.clone(),
                                },
                                mesh.clone(),
                            )),
                            _ => None,
                        })
                })
            })
            .next()
            .unwrap();
        mesh.sequence = Some(SequenceDef {
            count: 3,
            start: 1,
            digits: 1,
            setup_index: 0,
        });
        let alias = AttachmentRef {
            placeholder: "original scope alias".into(),
            ..source.clone()
        };
        let mut linked = mesh.clone();
        linked.link = Some(MeshLink {
            skin: source.skin.clone(),
            source: source.placeholder.clone(),
            timelines: true,
        });
        let attachments = doc
            .skins
            .get_mut(&source.skin)
            .unwrap()
            .get_mut(&source.slot)
            .unwrap();
        attachments.insert(source.placeholder.clone(), Attachment::Mesh(mesh.clone()));
        attachments.insert(alias.placeholder.clone(), Attachment::Mesh(linked));
        let slot = doc
            .slots
            .iter()
            .position(|slot| slot.name == source.slot)
            .unwrap();
        doc.animations[0].deform_tracks = vec![DeformTrack {
            attachment: source.clone(),
            keys: vec![DeformKey {
                time: 0.125,
                offsets: vec![1.; mesh.vertices.deform_len()],
                curve: Curve::Linear,
            }],
        }];
        doc.animations[0].inherit_tracks = vec![InheritTrack {
            bone: 0,
            keys: vec![InheritKey {
                time: 0.,
                inherit: Inherit::Normal,
            }],
        }];
        doc.animations[0].sequence_tracks = vec![SequenceTrack {
            attachment: source.clone(),
            keys: vec![SequenceKey {
                time: 0.125,
                mode: SequenceMode::Hold,
                index: 1,
                delay: 0.25,
            }],
        }];
        doc.events.push(EventDef::new("original scope marker"));
        doc.animations[0].events.push(EventKey {
            time: 0.25,
            event: doc.events.len() - 1,
            int: None,
            float: None,
            string: None,
            volume: None,
            balance: None,
        });
        doc.validate().unwrap();
        (doc, source, alias, slot)
    }

    #[test]
    fn timeline_scope_all_empty_and_unkeyed_owner_are_distinct_and_invalid_queries_do_not_publish()
    {
        let (doc, _, _, _) = scope_doc();
        let session = Session::new(doc).unwrap();
        let mut selection = KeySelection::default();
        selection
            .prepare(&session, Some(0), TimelineScope::All)
            .unwrap();
        assert!(selection.shown().keys.contains(&KeyId::Bone {
            bone: 1,
            property: Property::Rotation,
            time: 0.9
        }));
        assert!(
            selection
                .shown()
                .unsupported
                .iter()
                .any(|track| matches!(track, UnsupportedAnimationTimeline::Event { .. }))
        );
        selection
            .prepare(&session, Some(0), TimelineScope::Owners(vec![]))
            .unwrap();
        assert_eq!(selection.scope(), &TimelineScope::Owners(vec![]));
        assert!(selection.shown().owners.is_empty());
        assert!(selection.shown().keys.is_empty());
        assert!(selection.shown().unsupported.is_empty());
        let unkeyed = TimelineScope::Owners(vec![KeyOwner::DrawOrder {}]);
        selection
            .prepare(&session, Some(0), unkeyed.clone())
            .unwrap();
        assert_eq!(selection.scope(), &unkeyed);
        assert_eq!(selection.shown().owners, vec![KeyOwner::DrawOrder {}]);
        assert!(selection.shown().keys.is_empty());
        selection
            .prepare(
                &session,
                Some(0),
                TimelineScope::Owners(vec![KeyOwner::Bone { bone: 0 }]),
            )
            .unwrap();
        selection.select(x(0.4), false);
        let selected = selection.snapshot();
        for scope in [
            TimelineScope::Owners(vec![
                KeyOwner::Bone { bone: 0 },
                KeyOwner::Bone { bone: usize::MAX },
            ]),
            TimelineScope::Owners(vec![KeyOwner::Deform {
                attachment: AttachmentRef {
                    skin: "missing".into(),
                    slot: "missing".into(),
                    placeholder: "missing".into(),
                },
            }]),
        ] {
            assert!(selection.prepare(&session, Some(0), scope).is_err());
            assert_eq!(selection.keys(), selected.keys);
            assert_eq!(selection.primary(), selected.primary.as_ref());
            assert_eq!(selection.scope(), &selected.context.1);
            assert!(
                selection.shown().keys.is_empty(),
                "a failed query must not display the previous scope's metadata"
            );
        }
    }

    #[test]
    fn timeline_scope_compound_aliases_keep_order_scoped_metadata_and_phase_without_requery() {
        let (doc, source, alias, slot) = scope_doc();
        let session = Session::new(doc.clone()).unwrap();
        let mut selection = KeySelection::default();
        let source_owner = KeyOwner::Deform {
            attachment: source.clone(),
        };
        let scope = TimelineScope::Owners(vec![
            KeyOwner::Bone { bone: 1 },
            KeyOwner::Deform { attachment: alias },
            KeyOwner::Slot { slot },
            KeyOwner::Bone { bone: 0 },
            source_owner.clone(),
        ]);
        selection.prepare(&session, Some(0), scope).unwrap();
        let owners = vec![
            KeyOwner::Bone { bone: 1 },
            source_owner.clone(),
            KeyOwner::Slot { slot },
            KeyOwner::Bone { bone: 0 },
        ];
        assert_eq!(selection.scope(), &TimelineScope::Owners(owners.clone()));
        assert_eq!(selection.shown().owners, owners);
        assert_eq!(
            selection.shown().unsupported.len(),
            2,
            "Sequence is listed once even when Slot and Deform both contain it"
        );
        assert!(
            selection.shown().unsupported.iter().any(|track| matches!(
                track,
                UnsupportedAnimationTimeline::Inherit { bone: 0, .. }
            ))
        );
        assert!(selection.shown().unsupported.iter().any(|track| matches!(track, UnsupportedAnimationTimeline::Sequence { attachment, .. } if *attachment == source)));
        assert!(
            !selection
                .shown()
                .unsupported
                .iter()
                .any(|track| matches!(track, UnsupportedAnimationTimeline::Event { .. }))
        );
        let deform = KeyId::Deform {
            attachment: source,
            time: 0.125,
        };
        selection.select(deform, false);
        selection.select(x(0.123455), true);
        selection.set_primary(Some(x(0.123455)));
        assert_eq!(selection.select_all_shown().unwrap(), SelectionScope::Rows);
        let expected = selection.keys().to_vec();
        let metadata = selection.shown();
        selection
            .prepare(&session, Some(0), TimelineScope::Owners(owners.clone()))
            .unwrap();
        assert!(
            Arc::ptr_eq(&metadata, &selection.shown()),
            "canonical alias spelling changes reuse validated metadata"
        );
        assert_eq!(selection.keys(), expected);
        assert_eq!(selection.primary(), Some(&x(0.123455)));
        assert_eq!(selection.expansion, SelectionScope::Rows);
        let mut reversed = owners;
        reversed.reverse();
        selection
            .prepare(&session, Some(0), TimelineScope::Owners(reversed.clone()))
            .unwrap();
        assert_eq!(selection.shown().owners, reversed);
        assert_eq!(selection.keys(), expected);
        assert_eq!(selection.primary(), Some(&x(0.123455)));
        assert_eq!(selection.expansion, SelectionScope::None);
        assert_eq!(session.document(), &doc);
    }

    #[test]
    fn timeline_scope_snapshot_restores_fractional_selection_after_fixed_base_previews_and_cancel()
    {
        let mut session = Session::new(expansion_doc()).unwrap();
        let mut pose = session.local_pose(Some(0), 0.4)[1];
        pose.rotation += 9.25;
        session
            .edit_pose(
                1,
                peachy_engine::EditTarget::Animate {
                    animation: 0,
                    time: 0.4,
                },
                pose,
                &[Property::Rotation],
                false,
            )
            .unwrap();
        session
            .dispatch(Command::RenameAnimation {
                animation: 0,
                name: "redo witness".into(),
            })
            .unwrap();
        session.undo();
        assert!(session.can_redo());
        let before = session.document().clone();
        let pending = session.pose_edits().clone();
        let redo = session.can_redo();
        let scope =
            TimelineScope::Owners(vec![KeyOwner::Bone { bone: 0 }, KeyOwner::Bone { bone: 1 }]);
        let mut selection = KeySelection::default();
        selection.prepare(&session, Some(0), scope.clone()).unwrap();
        selection.select(x(0.12345), false);
        let other = KeyId::Bone {
            bone: 1,
            property: Property::Rotation,
            time: 0.9,
        };
        selection.select(other.clone(), true);
        selection.set_primary(Some(other));
        assert_eq!(selection.select_all_shown().unwrap(), SelectionScope::Rows);
        let snapshot = selection.snapshot();
        let expected = selection.keys().to_vec();
        let generation = session.document_generation();
        session.begin_transaction();
        for delta in [0.1, 0.2] {
            let incoming = session
                .preview_move_animation_keys(0, &expected, delta, false)
                .unwrap();
            selection.prepare(&session, Some(0), scope.clone()).unwrap();
            selection.replace(incoming);
        }
        session.cancel_transaction();
        assert_ne!(session.document_generation(), generation);
        selection
            .restore_snapshot(&session, Some(0), scope, snapshot.clone())
            .unwrap();
        assert_eq!(selection.keys(), expected);
        assert_eq!(selection.primary(), snapshot.primary.as_ref());
        assert_eq!(selection.expansion, SelectionScope::Rows);
        assert_eq!(session.document(), &before);
        assert_eq!(session.pose_edits(), &pending);
        assert_eq!(session.can_redo(), redo);
    }

    #[test]
    fn timeline_scope_restore_prunes_changed_scope_and_current_readonly_mask_without_cross_clip_aliasing()
     {
        let mut doc = expansion_doc();
        doc.animations[1].tracks[0].keys[0].time = 0.4;
        let session = Session::new(doc).unwrap();
        let all_bones =
            TimelineScope::Owners(vec![KeyOwner::Bone { bone: 0 }, KeyOwner::Bone { bone: 1 }]);
        let other_scope = TimelineScope::Owners(vec![KeyOwner::Bone { bone: 1 }]);
        let other = KeyId::Bone {
            bone: 1,
            property: Property::Rotation,
            time: 0.9,
        };
        let mut selection = KeySelection::default();
        selection
            .prepare(&session, Some(0), all_bones.clone())
            .unwrap();
        selection.select(x(0.4), false);
        selection.select(other.clone(), true);
        selection.set_primary(Some(other.clone()));
        assert_eq!(selection.select_all_shown().unwrap(), SelectionScope::Rows);
        let snapshot = selection.snapshot();
        selection
            .restore_snapshot(&session, Some(0), other_scope.clone(), snapshot.clone())
            .unwrap();
        assert_eq!(selection.keys(), std::slice::from_ref(&other));
        assert_eq!(selection.primary(), Some(&other));
        assert_eq!(selection.expansion, SelectionScope::None);
        selection.mark_readonly(vec![KeyOwner::Bone { bone: 1 }]);
        selection
            .restore_snapshot(&session, Some(0), other_scope, snapshot.clone())
            .unwrap();
        assert!(selection.is_empty());
        assert!(selection.primary().is_none());
        assert_eq!(selection.expansion, SelectionScope::None);
        selection
            .restore_snapshot(&session, Some(1), all_bones, snapshot)
            .unwrap();
        assert!(
            selection.is_empty(),
            "equal key IDs in another clip do not restore old selection"
        );
        assert!(selection.deletion().is_none());
    }

    #[test]
    fn timeline_scope_cancel_refreshes_metadata_but_keeps_current_readonly_mask_outside_snapshot() {
        let mut session = Session::new(expansion_doc()).unwrap();
        let scope = TimelineScope::Owners(vec![KeyOwner::Bone { bone: 0 }]);
        let mut selection = KeySelection::default();
        selection.prepare(&session, Some(0), scope.clone()).unwrap();
        selection.select(x(0.12345), false);
        assert_eq!(selection.select_all_shown().unwrap(), SelectionScope::Rows);
        let snapshot = selection.snapshot();
        let sources = selection.keys().to_vec();
        let before = session.document().clone();
        session.begin_transaction();
        let incoming = session
            .preview_move_animation_keys(0, &sources, 0.1, false)
            .unwrap();
        selection.prepare(&session, Some(0), scope.clone()).unwrap();
        selection.replace(incoming);
        selection.mark_readonly(vec![KeyOwner::Bone { bone: 0 }]);
        let preview_metadata = selection.shown();
        session.cancel_transaction();
        selection
            .restore_snapshot(&session, Some(0), scope, snapshot)
            .unwrap();
        assert!(!Arc::ptr_eq(&preview_metadata, &selection.shown()));
        assert_eq!(
            selection.shown().readonly_owners,
            vec![KeyOwner::Bone { bone: 0 }]
        );
        assert!(selection.is_empty());
        assert!(selection.primary().is_none());
        assert_eq!(selection.expansion, SelectionScope::None);
        assert_eq!(session.document(), &before);
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
