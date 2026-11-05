//! Pure presentation identities. A selected slot is never an implicit bone edit.
use peachy_document::{AttachmentRef, Document};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Folder {
    Skins,
    Animations,
    Slots,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeItem {
    Skeleton,
    Folder(Folder),
    Bone(usize),
    Slot(usize),
    Placeholder { slot: usize, name: String },
    Attachment(AttachmentRef),
    Skin(String),
    Animation(usize),
}

impl TreeItem {
    pub fn key(&self) -> String {
        match self {
            Self::Skeleton => "skeleton".into(),
            Self::Folder(f) => format!("folder:{f:?}"),
            Self::Bone(i) => format!("bone:{i}"),
            Self::Slot(i) => format!("slot:{i}"),
            Self::Placeholder { slot, name } => format!("placeholder:{}", json!([slot, name])),
            Self::Attachment(k) => format!("attachment:{}", json!([k.skin, k.slot, k.placeholder])),
            Self::Skin(s) => format!("skin:{s}"),
            Self::Animation(i) => format!("animation:{i}"),
        }
    }
    pub fn exists(&self, doc: &Document) -> bool {
        match self {
            Self::Skeleton | Self::Folder(_) => true,
            Self::Bone(i) => *i < doc.bones.len(),
            Self::Slot(i) => *i < doc.slots.len(),
            Self::Placeholder { slot, name } => doc.slots.get(*slot).is_some_and(|s| {
                doc.skins
                    .values()
                    .any(|skin| skin.get(&s.name).is_some_and(|a| a.contains_key(name)))
            }),
            Self::Attachment(k) => doc.attachment(k).is_some(),
            Self::Skin(s) => doc.skins.contains_key(s),
            Self::Animation(i) => *i < doc.animations.len(),
        }
    }
    pub fn label(&self, doc: &Document) -> String {
        match self {
            Self::Skeleton => doc.name.clone(),
            Self::Folder(f) => format!("{f:?}"),
            Self::Bone(i) => doc.bones.get(*i).map_or("?", |b| b.name.as_str()).into(),
            Self::Slot(i) => doc.slots.get(*i).map_or("?", |s| s.name.as_str()).into(),
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
            Self::Slot(i) | Self::Placeholder { slot: i, .. } => Some(*i),
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
            Self::Placeholder { slot, name } => {
                json!({"type":"placeholder","slot":slot,"name":name})
            }
            Self::Attachment(k) => json!({"type":"attachment","key":k}),
            Self::Skin(s) => json!({"type":"skin","name":s}),
            Self::Animation(i) => json!({"type":"animation","index":i}),
        }
    }
}

pub fn reconcile(selection: &mut Option<TreeItem>, doc: &Document) {
    if selection.as_ref().is_some_and(|s| !s.exists(doc)) {
        *selection = None;
    }
}

/// "default" also represents no named skin when no default attachments exist.
pub fn active_skin_exists(doc: &Document, skin: &str) -> bool {
    skin == "default" || doc.skins.contains_key(skin)
}
