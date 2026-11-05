//! Pure outline projection, shared by the Tree widget and its diagnostics.
use super::selection::{Folder, TreeItem};
use peachy_document::{AttachmentRef, Document};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Filters {
    pub bones: bool,
    pub slots: bool,
    pub attachments: bool,
    pub skins: bool,
    pub animations: bool,
}
impl Default for Filters {
    fn default() -> Self {
        Self {
            bones: true,
            slots: true,
            attachments: true,
            skins: true,
            animations: true,
        }
    }
}

#[derive(Default)]
pub struct TreeState {
    pub query: String,
    pub filters: Filters,
    pub all_skin_attachments: bool,
    pub collapsed: BTreeSet<String>,
    pub reveal: Option<String>,
}

pub struct Entry {
    pub item: TreeItem,
    pub label: String,
    pub parent: Option<usize>,
    pub depth: usize,
    pub matched: bool,
}

fn add(entries: &mut Vec<Entry>, item: TreeItem, parent: Option<usize>, doc: &Document) -> usize {
    let index = entries.len();
    let depth = parent.map_or(0, |p| entries[p].depth + 1);
    entries.push(Entry {
        label: item.label(doc),
        item,
        parent,
        depth,
        matched: true,
    });
    index
}

fn slot_entries(
    entries: &mut Vec<Entry>,
    doc: &Document,
    slot: usize,
    parent: usize,
    skin: &str,
    options: &TreeState,
) {
    let parent = if options.filters.slots {
        add(entries, TreeItem::Slot(slot), Some(parent), doc)
    } else {
        parent
    };
    if !options.filters.attachments {
        return;
    }
    let name = &doc.slots[slot].name;
    let placeholders: BTreeSet<_> = doc
        .skins
        .values()
        .filter_map(|s| s.get(name))
        .flat_map(|a| a.keys())
        .collect();
    for placeholder in placeholders {
        let has_skin = doc.skins.iter().any(|(n, s)| {
            n != "default" && s.get(name).is_some_and(|a| a.contains_key(placeholder))
        });
        let parent = if has_skin {
            add(
                entries,
                TreeItem::Placeholder {
                    slot,
                    name: placeholder.clone(),
                },
                Some(parent),
                doc,
            )
        } else {
            parent
        };
        if options.all_skin_attachments {
            for (skin, s) in &doc.skins {
                if s.get(name).is_some_and(|a| a.contains_key(placeholder)) {
                    add(
                        entries,
                        TreeItem::Attachment(AttachmentRef {
                            skin: skin.clone(),
                            slot: name.clone(),
                            placeholder: placeholder.clone(),
                        }),
                        Some(parent),
                        doc,
                    );
                }
            }
        } else if let Some((key, _)) = doc.resolve_attachment(skin, slot, placeholder) {
            add(entries, TreeItem::Attachment(key), Some(parent), doc);
        }
    }
}

pub fn project(doc: &Document, skin: &str, setup: bool, options: &TreeState) -> Vec<Entry> {
    let mut entries = vec![];
    let root = add(&mut entries, TreeItem::Skeleton, None, doc);
    if options.filters.bones {
        let mut pending: Vec<_> = doc
            .bones
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, b)| b.parent.is_none())
            .map(|(i, _)| (i, root))
            .collect();
        while let Some((bone, parent)) = pending.pop() {
            let node = add(&mut entries, TreeItem::Bone(bone), Some(parent), doc);
            for (slot, _) in doc.slots.iter().enumerate().filter(|(_, s)| s.bone == bone) {
                slot_entries(&mut entries, doc, slot, node, skin, options);
            }
            for (child, _) in doc
                .bones
                .iter()
                .enumerate()
                .rev()
                .filter(|(_, b)| b.parent == Some(bone))
            {
                pending.push((child, node));
            }
        }
    } else if options.filters.slots || options.filters.attachments {
        let group = add(
            &mut entries,
            TreeItem::Folder(Folder::Slots),
            Some(root),
            doc,
        );
        for slot in 0..doc.slots.len() {
            slot_entries(&mut entries, doc, slot, group, skin, options);
        }
    }
    if options.filters.skins {
        let group = add(
            &mut entries,
            TreeItem::Folder(Folder::Skins),
            Some(root),
            doc,
        );
        for skin in doc.skins.keys().filter(|s| s.as_str() != "default") {
            add(&mut entries, TreeItem::Skin(skin.clone()), Some(group), doc);
        }
    }
    if options.filters.animations && !setup {
        let group = add(
            &mut entries,
            TreeItem::Folder(Folder::Animations),
            Some(root),
            doc,
        );
        for i in 0..doc.animations.len() {
            add(&mut entries, TreeItem::Animation(i), Some(group), doc);
        }
    }
    let query = options.query.trim().to_lowercase();
    if !query.is_empty() {
        for entry in &mut entries {
            entry.matched = entry.label.to_lowercase().contains(&query);
        }
        for i in (0..entries.len()).rev() {
            if entries[i].matched
                && let Some(parent) = entries[i].parent
            {
                entries[parent].matched = true;
            }
        }
    }
    entries
}

pub fn row_visible(entries: &[Entry], index: usize, state: &TreeState) -> bool {
    if !entries[index].matched {
        return false;
    }
    if !state.query.trim().is_empty() {
        return true;
    }
    let mut parent = entries[index].parent;
    while let Some(p) = parent {
        if state.collapsed.contains(&entries[p].item.key()) {
            return false;
        }
        parent = entries[p].parent;
    }
    true
}

impl TreeState {
    pub fn for_document(doc: &Document, skin: &str) -> Self {
        let mut state = Self::default();
        for e in project(doc, skin, false, &state) {
            if matches!(e.item, TreeItem::Slot(_) | TreeItem::Placeholder { .. }) {
                state.collapsed.insert(e.item.key());
            }
        }
        state
    }
    pub fn reveal(&mut self, doc: &Document, skin: &str, setup: bool, item: &TreeItem) {
        let entries = project(doc, skin, setup, self);
        if let Some(index) = entries.iter().position(|e| &e.item == item) {
            let mut parent = entries[index].parent;
            while let Some(p) = parent {
                self.collapsed.remove(&entries[p].item.key());
                parent = entries[p].parent;
            }
        }
        self.reveal = Some(item.key());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn doc() -> Document {
        serde_json::from_str(include_str!(
            "../../../../examples/peach-bot/mesh-bot.peachy"
        ))
        .unwrap()
    }
    #[test]
    fn skin_placeholders_show_active_appearance_and_default_fallback_without_duplicates() {
        let d = doc();
        let state = TreeState::default();
        let entries = project(&d, "mint", false, &state);
        let attachments: Vec<_> = entries.iter().filter_map(|e| e.item.attachment()).collect();
        assert_eq!(attachments.len(), 14);
        assert!(
            attachments
                .iter()
                .any(|k| k.skin == "mint" && k.slot == "body")
        );
        assert!(
            attachments
                .iter()
                .any(|k| k.skin == "default" && k.slot == "hand.R")
        );
        assert!(!attachments.iter().any(|k| k.skin == "peach"));
        assert!(
            entries
                .iter()
                .any(|e| matches!(&e.item,TreeItem::Placeholder{name,..} if name=="body"))
        );
        let all = project(
            &d,
            "mint",
            false,
            &TreeState {
                all_skin_attachments: true,
                ..Default::default()
            },
        );
        assert_eq!(
            all.iter()
                .filter(|e| matches!(e.item, TreeItem::Attachment(_)))
                .count(),
            16
        );
        let mut no_default = d;
        no_default.skins.remove("default");
        assert!(super::super::selection::active_skin_exists(
            &no_default,
            "default"
        ));
        let cleared = project(&no_default, "default", false, &state);
        assert!(
            !cleared
                .iter()
                .any(|e| matches!(e.item, TreeItem::Attachment(_)))
        );
        assert!(
            cleared
                .iter()
                .any(|e| matches!(e.item, TreeItem::Placeholder { .. }))
        );
    }
    #[test]
    fn slots_filter_uses_draw_order_and_search_retains_ancestors_across_collapsed_branches() {
        let d = doc();
        let mut state = TreeState::for_document(&d, "peach");
        state.filters = Filters {
            bones: false,
            slots: true,
            attachments: false,
            skins: false,
            animations: false,
        };
        let rows = project(&d, "peach", true, &state);
        let slots: Vec<_> = rows
            .iter()
            .filter_map(|e| {
                if let TreeItem::Slot(i) = e.item {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(slots, (0..d.slots.len()).collect::<Vec<_>>());
        state.filters = Filters::default();
        state.query = "head.peach".into();
        let rows = project(&d, "peach", true, &state);
        let visible: Vec<_> = rows
            .iter()
            .enumerate()
            .filter(|(i, _)| row_visible(&rows, *i, &state))
            .map(|(_, e)| &e.item)
            .collect();
        assert!(visible.contains(&&TreeItem::Skeleton));
        assert!(visible.iter().any(|e| matches!(e, TreeItem::Slot(_))));
        assert!(
            visible
                .iter()
                .any(|e| matches!(e,TreeItem::Attachment(k) if k.slot=="head" && k.skin=="peach"))
        );
        assert!(
            !visible
                .iter()
                .any(|e| matches!(e,TreeItem::Attachment(k) if k.slot=="body"))
        );
    }
    #[test]
    fn selection_reconciliation_clears_stale_entities_instead_of_redirecting_an_edit() {
        let mut d = doc();
        let last = d.bones.len();
        d.bones.push(peachy_document::Bone {
            name: "temporary".into(),
            parent: Some(0),
            length: 10.,
            setup: Default::default(),
        });
        let mut selected = Some(TreeItem::Bone(last));
        d.bones.pop();
        super::super::selection::reconcile(&mut selected, &d);
        assert!(selected.is_none());
        let slot = TreeItem::Slot(0);
        assert!(slot.bone().is_none());
        assert!(slot.attachment().is_none());
        let mut selected = Some(TreeItem::Skin("mint".into()));
        super::super::selection::reconcile(&mut selected, &d);
        assert!(selected.is_some());
        d.skins.remove("mint");
        super::super::selection::reconcile(&mut selected, &d);
        assert!(selected.is_none());
    }
}
