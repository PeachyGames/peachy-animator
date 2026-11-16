//! Portable grouped keys and their separate, editor-local identity receipt.
//! No target index, project or animation identity is written to the OS payload.
use super::*;
use peachy_engine::{
    AnimationKey, AnimationKeyGroupClipboard, AnimationKeyGroupTarget, ConstraintIndexRemap,
    KeyOwner, copy_animation_key_group,
};

#[derive(Clone, Debug, PartialEq)]
pub(in crate::editor) struct PasteReceipt {
    animation: usize,
    time: f32,
    accepted_keys: Vec<AnimationKey>,
    accepted_count: usize,
    selected_keys: Vec<AnimationKey>,
    at_document_generation: u64,
}

impl PasteReceipt {
    pub fn new(
        animation: usize,
        time: f32,
        accepted: &[AnimationKey],
        selected: &[AnimationKey],
        generation: u64,
    ) -> Self {
        Self {
            animation,
            time,
            accepted_keys: accepted.to_vec(),
            accepted_count: accepted.len(),
            selected_keys: selected.to_vec(),
            at_document_generation: generation,
        }
    }
    pub fn diagnostic(&self) -> serde_json::Value {
        serde_json::json!({
            "animation":self.animation,"time":self.time,"acceptedKeys":self.accepted_keys,
            "acceptedCount":self.accepted_count,"selectedKeys":self.selected_keys,
            "visibleSelectedCount":self.selected_keys.len(),"selectionPolicy":"preserveExisting",
            "atDocumentGeneration":self.at_document_generation,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::editor) struct GroupClipboard {
    pub clipboard: AnimationKeyGroupClipboard,
    owners: Vec<Option<KeyOwner>>,
    source_animation: usize,
    source_name: String,
}
impl GroupClipboard {
    pub fn copy(
        session: &Session,
        animation: usize,
        keys: &[AnimationKey],
    ) -> Result<Self, String> {
        let copied = copy_animation_key_group(session.document(), animation, keys)?;
        Ok(Self {
            clipboard: copied.clipboard,
            owners: copied.source_owners.into_iter().map(Some).collect(),
            source_animation: animation,
            source_name: session.document().animations[animation].name.clone(),
        })
    }
    pub fn len(&self) -> usize {
        self.clipboard.entries.iter().map(clipboard_len).sum()
    }
    pub fn text(&self) -> String {
        serde_json::json!({
            "peachyAnimatorClipboard":"keys", "version":3, "payload":self.clipboard,
        })
        .to_string()
    }
    pub fn targets(&self) -> Result<Vec<AnimationKeyGroupTarget>, String> {
        if self.owners.len() != self.clipboard.entries.len() {
            return Err(
                "Copied key group has no complete owner receipt. Copy the keys again.".into(),
            );
        }
        self.owners
            .iter()
            .enumerate()
            .map(|(entry, owner)| {
                owner
                    .clone()
                    .map(|target| AnimationKeyGroupTarget { entry, target })
                    .ok_or_else(|| {
                        "A copied key group's original owner was removed. Copy the keys again."
                            .into()
                    })
            })
            .collect()
    }
    pub fn remap(&mut self, mapping: &ConstraintIndexRemap) {
        for owner in &mut self.owners {
            *owner = owner
                .as_ref()
                .and_then(|owner| mapping.map_owner(owner).ok().flatten());
        }
    }
    pub fn diagnostic(&self) -> serde_json::Value {
        serde_json::json!({
            "kind":"group", "version":3, "keys":self.len(), "entryCount":self.clipboard.entries.len(),
            "bindings":self.owners.iter().enumerate().map(|(entry,owner)|serde_json::json!({"entry":entry,"owner":owner})).collect::<Vec<_>>(),
            "bindingsComplete":self.owners.iter().all(Option::is_some),
            "sourceAnimation":{"index":self.source_animation,"name":self.source_name},

        })
    }
}

pub(super) enum CopiedKeys {
    Single(super::leaf::CopiedLeaf),
    Group(GroupClipboard),
}
impl CopiedKeys {
    pub fn len(&self) -> usize {
        match self {
            Self::Single(copied) => clipboard_len(&copied.clipboard),
            Self::Group(keys) => keys.len(),
        }
    }
    pub fn text(&self) -> String {
        match self {
            Self::Single(copied) => copied.source.text().to_owned(),
            Self::Group(keys) => keys.text(),
        }
    }
}
