//! Pure presentation navigation. Document history and the active selection stay separate.
use super::selection::TreeItem;
use peachy_document::Document;

const LIMIT: usize = 128;
#[derive(Default)]
pub(super) struct SelectionHistory {
    back: Vec<Option<TreeItem>>,
    forward: Vec<Option<TreeItem>>,
    groups: [Option<TreeItem>; 10],
}
impl SelectionHistory {
    pub fn record(&mut self, previous: &Option<TreeItem>, next: &Option<TreeItem>) {
        if previous != next {
            self.back.push(previous.clone());
            if self.back.len() > LIMIT {
                self.back.remove(0);
            }
            self.forward.clear();
        }
    }
    pub fn previous(&mut self, current: &Option<TreeItem>) -> Option<Option<TreeItem>> {
        let previous = self.back.pop()?;
        self.forward.push(current.clone());
        Some(previous)
    }
    pub fn next(&mut self, current: &Option<TreeItem>) -> Option<Option<TreeItem>> {
        let next = self.forward.pop()?;
        self.back.push(current.clone());
        Some(next)
    }
    pub fn store(&mut self, index: usize, selection: &Option<TreeItem>) {
        if let Some(group) = self.groups.get_mut(index) {
            *group = selection.clone();
        }
    }
    pub fn recall(&self, index: usize) -> Option<TreeItem> {
        self.groups.get(index)?.clone()
    }
    pub fn reconcile(&mut self, doc: &Document) {
        let valid = |item: &Option<TreeItem>| item.as_ref().is_none_or(|item| item.exists(doc));
        self.back.retain(valid);
        self.forward.retain(valid);
        for group in &mut self.groups {
            if !valid(group) {
                *group = None;
            }
        }
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
    fn browsing_selection_preserves_empty_and_typed_targets_and_branches() {
        let mut history = SelectionHistory::default();
        let bone = Some(TreeItem::Bone(1));
        let slot = Some(TreeItem::Slot(0));
        history.record(&bone, &slot);
        history.record(&slot, &None);
        assert_eq!(history.previous(&None), Some(slot.clone()));
        assert_eq!(history.previous(&slot), Some(bone.clone()));
        assert_eq!(history.next(&bone), Some(slot.clone()));
        history.record(&slot, &Some(TreeItem::Skin("mint".into())));
        assert_eq!(history.next(&slot), None);
        history.record(&None, &None); // Repeated selection is not a history entry.
        assert_eq!(history.previous(&None), Some(slot));
    }
    #[test]
    fn groups_are_explicit_single_item_targets_and_invalid_entities_are_pruned() {
        let mut history = SelectionHistory::default();
        let bone = Some(TreeItem::Bone(1));
        let bad = Some(TreeItem::Bone(999));
        history.store(1, &bone);
        history.store(2, &Some(TreeItem::Slot(0)));
        history.store(3, &bad);
        history.record(&bad, &bone);
        history.reconcile(&doc());
        assert_eq!(history.recall(1), bone);
        assert_eq!(history.recall(2), Some(TreeItem::Slot(0)));
        assert_eq!(history.recall(3), None);
        assert_eq!(history.previous(&None), None);
        history.store(1, &None);
        assert_eq!(history.recall(1), None);
        assert_eq!(history.recall(99), None);
    }
    #[test]
    fn navigation_is_bounded_without_touching_the_document() {
        let before = doc();
        let mut history = SelectionHistory::default();
        for i in 0..200 {
            history.record(
                &Some(TreeItem::Bone(i % 2)),
                &Some(TreeItem::Bone((i + 1) % 2)),
            );
        }
        let mut current = None;
        let mut count = 0;
        while let Some(next) = history.previous(&current) {
            current = next;
            count += 1;
        }
        assert_eq!(count, LIMIT);
        assert_eq!(before, doc());
    }
}
