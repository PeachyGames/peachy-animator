//! Pure presentation navigation. Document history and the active selection stay separate.
#[cfg(test)]
use super::selection::TreeItem;
use super::selection::TreeSelection;
use peachy_document::Document;

const LIMIT: usize = 128;
#[derive(Default)]
pub(super) struct SelectionHistory {
    back: Vec<TreeSelection>,
    forward: Vec<TreeSelection>,
    groups: [Option<TreeSelection>; 10],
}
impl SelectionHistory {
    pub fn record(&mut self, previous: &TreeSelection, next: &TreeSelection) {
        if previous != next {
            self.back.push(previous.clone());
            if self.back.len() > LIMIT {
                self.back.remove(0);
            }
            self.forward.clear();
        }
    }
    pub fn previous(&mut self, current: &TreeSelection) -> Option<TreeSelection> {
        let previous = self.back.pop()?;
        self.forward.push(current.clone());
        Some(previous)
    }
    pub fn next(&mut self, current: &TreeSelection) -> Option<TreeSelection> {
        let next = self.forward.pop()?;
        self.back.push(current.clone());
        Some(next)
    }
    pub fn store(&mut self, index: usize, selection: &TreeSelection) {
        if let Some(group) = self.groups.get_mut(index) {
            *group = (!selection.is_empty()).then(|| selection.clone());
        }
    }
    pub fn recall(&self, index: usize) -> Option<TreeSelection> {
        self.groups.get(index)?.clone()
    }
    pub fn reconcile(&mut self, doc: &Document) {
        let valid = |selection: &mut TreeSelection| {
            let empty = selection.is_empty();
            selection.reconcile(doc);
            empty || !selection.is_empty()
        };
        self.back.retain_mut(valid);
        self.forward.retain_mut(valid);
        for group in &mut self.groups {
            if group.as_mut().is_some_and(|selection| !valid(selection)) {
                *group = None;
            }
        }
    }

    pub fn remap_constraints(&mut self, mapping: &peachy_engine::ConstraintIndexRemap) {
        for entries in [&mut self.back, &mut self.forward] {
            entries.retain_mut(|selection| {
                let empty = selection.is_empty();
                selection.remap_constraints(mapping);
                empty || !selection.is_empty()
            });
        }
        for group in &mut self.groups {
            if let Some(selection) = group {
                selection.remap_constraints(mapping);
                if selection.is_empty() {
                    *group = None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn doc() -> Document {
        peachy_io::decode_project(include_str!(
            "../../../../examples/peach-bot/mesh-bot.peachy"
        ))
        .unwrap()
    }
    #[test]
    fn ordered_selection_history_and_number_groups_restore_members_and_primary() {
        let first = TreeSelection::new(
            vec![TreeItem::Bone(4), TreeItem::Bone(2)],
            Some(TreeItem::Bone(4)),
        )
        .unwrap();
        let second = TreeSelection::new(
            vec![TreeItem::Bone(2), TreeItem::Bone(4)],
            Some(TreeItem::Bone(4)),
        )
        .unwrap();
        let mut history = SelectionHistory::default();
        history.record(&TreeSelection::default(), &first);
        history.record(&first, &second);
        history.store(3, &first);
        assert_eq!(history.previous(&second), Some(first.clone()));
        assert_eq!(history.next(&first), Some(second));
        assert_eq!(history.recall(3), Some(first));
        history.store(3, &TreeSelection::default());
        assert!(history.recall(3).is_none());
    }
    #[test]
    fn browsing_selection_preserves_empty_and_typed_targets_and_branches() {
        let mut history = SelectionHistory::default();
        let bone = TreeSelection::singleton(TreeItem::Bone(1));
        let slot = TreeSelection::singleton(TreeItem::Slot(0));
        history.record(&bone, &slot);
        history.record(&slot, &TreeSelection::default());
        assert_eq!(
            history.previous(&TreeSelection::default()),
            Some(slot.clone())
        );
        assert_eq!(history.previous(&slot), Some(bone.clone()));
        assert_eq!(history.next(&bone), Some(slot.clone()));
        history.record(
            &slot,
            &TreeSelection::singleton(TreeItem::Skin("mint".into())),
        );
        assert_eq!(history.next(&slot), None);
        history.record(&TreeSelection::default(), &TreeSelection::default()); // Repeated selection is not a history entry.
        assert_eq!(history.previous(&TreeSelection::default()), Some(slot));
    }
    #[test]
    fn groups_are_explicit_single_item_targets_and_invalid_entities_are_pruned() {
        let mut history = SelectionHistory::default();
        let bone = TreeSelection::singleton(TreeItem::Bone(1));
        let bad = TreeSelection::singleton(TreeItem::Bone(999));
        history.store(1, &bone);
        history.store(2, &TreeSelection::singleton(TreeItem::Slot(0)));
        history.store(3, &bad);
        history.record(&bad, &bone);
        history.reconcile(&doc());
        assert_eq!(history.recall(1), Some(bone));
        assert_eq!(
            history.recall(2),
            Some(TreeSelection::singleton(TreeItem::Slot(0)))
        );
        assert_eq!(history.recall(3), None);
        assert_eq!(history.previous(&TreeSelection::default()), None);
        history.store(1, &TreeSelection::default());
        assert_eq!(history.recall(1), None);
        assert_eq!(history.recall(99), None);
    }
    #[test]
    fn navigation_is_bounded_without_touching_the_document() {
        let before = doc();
        let mut history = SelectionHistory::default();
        for i in 0..200 {
            history.record(
                &TreeSelection::singleton(TreeItem::Bone(i % 2)),
                &TreeSelection::singleton(TreeItem::Bone((i + 1) % 2)),
            );
        }
        let mut current = TreeSelection::default();
        let mut count = 0;
        while let Some(next) = history.previous(&current) {
            current = next;
            count += 1;
        }
        assert_eq!(count, LIMIT);
        assert_eq!(before, doc());
    }
}

#[cfg(test)]
mod rename_tests {
    use super::*;
    use peachy_document::AttachmentRef;
    use peachy_engine::{Command, apply_command};

    #[test]
    fn slot_rename_prunes_old_attachment_identity_before_name_reuse() {
        let before: Document = peachy_io::decode_project(include_str!(
            "../../../../examples/peach-bot/mesh-bot.peachy"
        ))
        .unwrap();
        let slot = before.slots.iter().position(|s| s.name == "head").unwrap();
        let attachment = TreeSelection::singleton(TreeItem::Attachment(AttachmentRef {
            skin: "peach".into(),
            slot: "head".into(),
            placeholder: "head".into(),
        }));
        let mut selection = attachment.clone();
        let mut history = SelectionHistory::default();
        history.store(1, &attachment);
        history.record(&attachment, &TreeSelection::singleton(TreeItem::Slot(slot)));
        let after = apply_command(
            &before,
            Command::RenameSlot {
                slot,
                name: "face-slot".into(),
            },
        )
        .unwrap();
        super::super::selection::reconcile(&mut selection, &after);
        history.reconcile(&after);
        assert_eq!(selection, None);
        assert_eq!(history.recall(1), None);
        assert_eq!(
            history.previous(&TreeSelection::singleton(TreeItem::Slot(slot))),
            None
        );
        assert!(TreeItem::Slot(slot).exists(&after));
        // Reusing the name for another slot cannot revive a pruned target/group.
        let another = if slot == 0 { 1 } else { 0 };
        let reused = apply_command(
            &after,
            Command::RenameSlot {
                slot: another,
                name: "head".into(),
            },
        )
        .unwrap();
        history.reconcile(&reused);
        assert_eq!(history.recall(1), None);
        assert_eq!(selection, None);
    }
}
