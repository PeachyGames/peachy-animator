//! The v2 payload stays portable; only this editor owns its original target receipt.
use super::*;
use peachy_engine::{AnimationKey, ConstraintIndexRemap, KeyOwner, copy_animation_key_group};

#[derive(Clone, Debug, PartialEq)]
pub(in crate::editor) struct LeafSourceReceipt {
    owner: Option<KeyOwner>,
    payload_text: String,
    source_animation: usize,
    source_name: String,
}

pub(super) struct CopiedLeaf {
    pub clipboard: AnimationKeyClipboard,
    pub source: LeafSourceReceipt,
}

impl LeafSourceReceipt {
    pub(super) fn copy(
        session: &Session,
        animation: usize,
        keys: &[AnimationKey],
    ) -> Result<CopiedLeaf, String> {
        // A one-owner group uses the same origin, values and leaf builder as
        // the legacy copy helper, and returns its canonical owner in one read.
        let mut copied = copy_animation_key_group(session.document(), animation, keys)?;
        if copied.clipboard.entries.len() != 1 || copied.source_owners.len() != 1 {
            return Err("Cannot identify one source item for the selected keys.".into());
        }
        let clipboard = copied.clipboard.entries.pop().unwrap();
        let owner = copied.source_owners.pop().unwrap();
        let source = Self {
            owner: Some(owner),
            payload_text: clipboard_text(&clipboard),
            source_animation: animation,
            source_name: session.document().animations[animation].name.clone(),
        };
        Ok(CopiedLeaf { clipboard, source })
    }
    pub fn text(&self) -> &str {
        &self.payload_text
    }
    pub fn has_owner(&self) -> bool {
        self.owner.is_some()
    }
    pub fn target(&self, clipboard: &AnimationKeyClipboard) -> Result<KeyOwner, String> {
        if clipboard_text(clipboard) != self.payload_text {
            return Err(
                "These keys no longer match the keys copied in this project. Copy them again."
                    .into(),
            );
        }
        self.owner.clone().ok_or_else(|| "The original item for these keys was removed. Select a compatible item or copy the keys again.".into())
    }
    pub fn remap(&mut self, mapping: &ConstraintIndexRemap) {
        self.owner = self
            .owner
            .as_ref()
            .and_then(|owner| mapping.map_owner(owner).ok().flatten());
    }
    pub fn diagnostic(&self, clipboard: &AnimationKeyClipboard) -> serde_json::Value {
        serde_json::json!({
            "kind":"single","version":2,"keys":clipboard_len(clipboard),"entryCount":1,
            "bindings":[{"entry":0,"owner":self.owner}],"bindingsComplete":self.owner.is_some(),
            "sourceBindingOwned":clipboard_text(clipboard)==self.payload_text,
            "sourceAnimation":{"index":self.source_animation,"name":self.source_name},
        })
    }
}
