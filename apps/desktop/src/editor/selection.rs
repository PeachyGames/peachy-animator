//! Pure presentation identities. A selected slot is never an implicit bone edit.
use peachy_document::{AttachmentRef, Document};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Folder {
    Skins,
    Animations,
    Slots,
    Constraints,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeItem {
    Skeleton,
    Folder(Folder),
    Bone(usize),
    Slot(usize),
    DrawOrder,
    DrawOrderSlot { slot: usize },
    Placeholder { slot: usize, name: String },
    Attachment(AttachmentRef),
    Skin(String),
    Animation(usize),
    Constraint(usize),
}

/// One authoritative presentation selection. Bone groups retain click order;
/// every other item remains an explicit singleton.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct TreeSelection {
    members: Vec<TreeItem>,
    primary: Option<TreeItem>,
}

#[derive(Clone, Copy)]
pub(super) enum PrimaryPolicy {
    Keep,
}

impl TreeSelection {
    pub fn new(members: Vec<TreeItem>, primary: Option<TreeItem>) -> Result<Self, String> {
        if members
            .iter()
            .enumerate()
            .any(|(i, item)| members[..i].contains(item))
        {
            return Err("Selection members must be unique".into());
        }
        if members.len() > 1 && !members.iter().all(|item| matches!(item, TreeItem::Bone(_))) {
            return Err("Only bones can form a multiple selection".into());
        }
        if members.is_empty() != primary.is_none()
            || primary.as_ref().is_some_and(|item| !members.contains(item))
        {
            return Err("Selection primary must be one of its members".into());
        }
        Ok(Self { members, primary })
    }
    pub fn singleton(item: TreeItem) -> Self {
        Self::new(vec![item.clone()], Some(item)).expect("A singleton selection is valid")
    }
    pub fn members(&self) -> &[TreeItem] {
        &self.members
    }
    pub fn primary(&self) -> Option<&TreeItem> {
        self.primary.as_ref()
    }
    pub fn single(&self) -> Option<&TreeItem> {
        (self.members.len() == 1).then(|| &self.members[0])
    }
    pub fn contains(&self, item: &TreeItem) -> bool {
        self.members.contains(item)
    }
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
    pub fn len(&self) -> usize {
        self.members.len()
    }
    pub fn bones(&self) -> Vec<usize> {
        self.members.iter().filter_map(TreeItem::bone).collect()
    }
    pub fn set_primary(&mut self, item: TreeItem) -> Result<(), String> {
        if !self.members.contains(&item) {
            return Err("Selection primary must be a member".into());
        }
        self.primary = Some(item);
        Ok(())
    }
    fn choose_primary(&mut self, policy: PrimaryPolicy) {
        let primary = match policy {
            PrimaryPolicy::Keep
                if self
                    .primary
                    .as_ref()
                    .is_some_and(|item| self.members.contains(item)) =>
            {
                self.primary.clone()
            }
            _ => self.members.first().cloned(),
        };
        if let Some(primary) = primary {
            self.set_primary(primary)
                .expect("Primary was chosen from the member set");
        } else {
            self.primary = None;
        }
    }
    pub fn toggle_bone(&mut self, bone: usize, policy: PrimaryPolicy) {
        let item = TreeItem::Bone(bone);
        if self
            .members
            .iter()
            .any(|member| !matches!(member, TreeItem::Bone(_)))
        {
            self.members.clear();
            self.primary = None;
        }
        if let Some(index) = self.members.iter().position(|member| member == &item) {
            self.members.remove(index);
        } else {
            self.members.push(item);
        }
        self.choose_primary(policy);
    }
    pub fn reconcile(&mut self, doc: &Document) {
        self.members.retain(|item| item.exists(doc));
        self.choose_primary(PrimaryPolicy::Keep);
    }
    pub fn remap_constraints(&mut self, mapping: &peachy_engine::ConstraintIndexRemap) {
        self.members = self
            .members
            .drain(..)
            .filter_map(|item| remap_constraint(item, mapping))
            .collect();
        self.primary = self
            .primary
            .take()
            .and_then(|item| remap_constraint(item, mapping));
        self.choose_primary(PrimaryPolicy::Keep);
    }
    pub fn diagnostic(&self) -> Value {
        json!({"members":self.members.iter().map(TreeItem::diagnostic).collect::<Vec<_>>(),"primary":self.primary().map(TreeItem::diagnostic)})
    }
}

impl From<Option<TreeItem>> for TreeSelection {
    fn from(item: Option<TreeItem>) -> Self {
        item.map_or_else(Self::default, Self::singleton)
    }
}
impl From<TreeItem> for TreeSelection {
    fn from(item: TreeItem) -> Self {
        Self::singleton(item)
    }
}

#[cfg(test)]
impl PartialEq<Option<TreeItem>> for TreeSelection {
    fn eq(&self, item: &Option<TreeItem>) -> bool {
        match item {
            Some(item) => self.single() == Some(item),
            None => self.is_empty(),
        }
    }
}
#[cfg(test)]
impl PartialEq<TreeSelection> for Option<TreeItem> {
    fn eq(&self, selection: &TreeSelection) -> bool {
        selection == self
    }
}

impl TreeItem {
    pub fn key(&self) -> String {
        match self {
            Self::Skeleton => "skeleton".into(),
            Self::Folder(f) => format!("folder:{f:?}"),
            Self::Bone(i) => format!("bone:{i}"),
            Self::Slot(i) => format!("slot:{i}"),
            Self::DrawOrder => "drawOrder".into(),
            Self::DrawOrderSlot { slot } => format!("drawOrderSlot:{slot}"),
            Self::Placeholder { slot, name } => format!("placeholder:{}", json!([slot, name])),
            Self::Attachment(k) => format!("attachment:{}", json!([k.skin, k.slot, k.placeholder])),
            Self::Skin(s) => format!("skin:{s}"),
            Self::Animation(i) => format!("animation:{i}"),
            Self::Constraint(i) => format!("constraint:{i}"),
        }
    }
    pub fn exists(&self, doc: &Document) -> bool {
        match self {
            Self::Skeleton | Self::Folder(_) | Self::DrawOrder => true,
            Self::Bone(i) => *i < doc.bones.len(),
            Self::Slot(i) | Self::DrawOrderSlot { slot: i } => *i < doc.slots.len(),
            Self::Placeholder { slot, name } => doc.slots.get(*slot).is_some_and(|s| {
                doc.skins
                    .values()
                    .any(|skin| skin.get(&s.name).is_some_and(|a| a.contains_key(name)))
            }),
            Self::Attachment(k) => doc.attachment(k).is_some(),
            Self::Skin(s) => doc.skins.contains_key(s),
            Self::Animation(i) => *i < doc.animations.len(),
            Self::Constraint(i) => *i < doc.constraints.len(),
        }
    }
    pub fn label(&self, doc: &Document) -> String {
        match self {
            Self::Skeleton => doc.name.clone(),
            Self::Folder(f) => format!("{f:?}"),
            Self::Bone(i) => doc.bones.get(*i).map_or("?", |b| b.name.as_str()).into(),
            Self::Slot(i) | Self::DrawOrderSlot { slot: i } => {
                doc.slots.get(*i).map_or("?", |s| s.name.as_str()).into()
            }
            Self::DrawOrder => "Draw order".into(),
            Self::Placeholder { name, .. } => name.clone(),
            Self::Attachment(k) => doc
                .attachment(k)
                .and_then(|a| a.name())
                .unwrap_or(&k.placeholder)
                .into(),
            Self::Skin(s) => s.clone(),
            Self::Animation(i) => doc
                .animations
                .get(*i)
                .map_or("?", |a| a.name.as_str())
                .into(),
            Self::Constraint(i) => doc
                .constraints
                .get(*i)
                .map_or("?", peachy_document::Constraint::name)
                .into(),
        }
    }
    pub fn bone(&self) -> Option<usize> {
        if let Self::Bone(i) = self {
            Some(*i)
        } else {
            None
        }
    }
    pub fn attachment(&self) -> Option<&AttachmentRef> {
        if let Self::Attachment(k) = self {
            Some(k)
        } else {
            None
        }
    }
    pub fn slot(&self, doc: &Document) -> Option<usize> {
        match self {
            Self::Slot(i) | Self::DrawOrderSlot { slot: i } | Self::Placeholder { slot: i, .. } => {
                Some(*i)
            }
            Self::Attachment(k) => doc.slots.iter().position(|s| s.name == k.slot),
            _ => None,
        }
    }
    pub fn diagnostic(&self) -> Value {
        match self {
            Self::Skeleton => json!({"type":"skeleton"}),
            Self::Folder(f) => json!({"type":"folder","name":format!("{f:?}")}),
            Self::Bone(i) => json!({"type":"bone","index":i}),
            Self::Slot(i) => json!({"type":"slot","index":i}),
            Self::DrawOrder => json!({"type":"drawOrder"}),
            Self::DrawOrderSlot { slot } => json!({"type":"drawOrderSlot","slot":slot}),
            Self::Placeholder { slot, name } => {
                json!({"type":"placeholder","slot":slot,"name":name})
            }
            Self::Attachment(k) => json!({"type":"attachment","key":k}),
            Self::Skin(s) => json!({"type":"skin","name":s}),
            Self::Animation(i) => json!({"type":"animation","index":i}),
            Self::Constraint(i) => json!({"type":"constraint","index":i}),
        }
    }
}

pub(super) fn reconcile(selection: &mut TreeSelection, doc: &Document) {
    selection.reconcile(doc);
}

/// Structural edits supply the identity mapping; an absent target never falls back.
pub(super) fn remap_constraint(
    item: TreeItem,
    mapping: &peachy_engine::ConstraintIndexRemap,
) -> Option<TreeItem> {
    match item {
        TreeItem::Constraint(index) => mapping
            .map_index(index)
            .ok()
            .flatten()
            .map(TreeItem::Constraint),
        other => Some(other),
    }
}

/// "default" also represents no named skin when no default attachments exist.
pub fn active_skin_exists(doc: &Document, skin: &str) -> bool {
    skin == "default" || doc.skins.contains_key(skin)
}

#[cfg(test)]
mod ordered_selection_tests {
    use super::*;

    #[test]
    fn ordered_selection_preserves_first_primary_and_toggle_order_without_rebinding() {
        let mut selection = TreeSelection::singleton(TreeItem::Bone(4));
        selection.toggle_bone(2, PrimaryPolicy::Keep);
        assert_eq!(selection.bones(), vec![4, 2]);
        assert_eq!(selection.primary(), Some(&TreeItem::Bone(4)));
        assert!(selection.single().is_none());
        selection.toggle_bone(4, PrimaryPolicy::Keep);
        assert_eq!(selection.bones(), vec![2]);
        assert_eq!(selection.primary(), Some(&TreeItem::Bone(2)));
        selection.toggle_bone(4, PrimaryPolicy::Keep);
        assert_eq!(selection.bones(), vec![2, 4]);
        assert_eq!(selection.primary(), Some(&TreeItem::Bone(2)));
        selection.toggle_bone(3, PrimaryPolicy::Keep);
        assert_eq!(selection.bones(), vec![2, 4, 3]);
        assert_eq!(selection.primary(), Some(&TreeItem::Bone(2)));
        selection.set_primary(TreeItem::Bone(3)).unwrap();
        assert_eq!(selection.primary(), Some(&TreeItem::Bone(3)));
        selection.set_primary(TreeItem::Bone(2)).unwrap();
        assert_eq!(selection.bones(), vec![2, 4, 3]);
        assert!(selection.set_primary(TreeItem::Bone(999)).is_err());
        let mut independent = TreeSelection::new(
            vec![TreeItem::Bone(4), TreeItem::Bone(2), TreeItem::Bone(3)],
            Some(TreeItem::Bone(4)),
        )
        .unwrap();
        independent.toggle_bone(4, PrimaryPolicy::Keep);
        assert_eq!(independent.bones(), vec![2, 3]);
        assert_eq!(independent.primary(), Some(&TreeItem::Bone(2)));
    }

    #[test]
    fn ordered_selection_rejects_mixed_duplicate_and_external_primary_snapshots() {
        for (members, primary) in [
            (
                vec![TreeItem::Bone(1), TreeItem::Slot(0)],
                Some(TreeItem::Bone(1)),
            ),
            (
                vec![TreeItem::Bone(1), TreeItem::Bone(1)],
                Some(TreeItem::Bone(1)),
            ),
            (vec![TreeItem::Bone(1)], Some(TreeItem::Bone(2))),
            (vec![TreeItem::Bone(1)], None),
            (vec![], Some(TreeItem::Bone(1))),
        ] {
            assert!(TreeSelection::new(members, primary).is_err());
        }
        let mut selection = TreeSelection::singleton(TreeItem::Slot(0));
        selection.toggle_bone(2, PrimaryPolicy::Keep);
        assert_eq!(selection.single(), Some(&TreeItem::Bone(2)));
    }
}
