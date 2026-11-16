//! Presentation focus never supplies a pose, clipboard or curve edit target.
use super::dopesheet_selection::TimelineScope;
use super::*;
use peachy_engine::KeyOwner;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum DopesheetFocus {
    #[default]
    All,
    Bone(usize),
    Owner(KeyOwner),
}

impl Editor {
    pub(super) fn effective_dopesheet_focus(&self) -> DopesheetFocus {
        let doc = self.session.document();
        let focus = match self.selection.as_ref() {
            None => DopesheetFocus::All,
            Some(TreeItem::DrawOrder) => match &self.dopesheet_focus {
                DopesheetFocus::All | DopesheetFocus::Bone(_) => self.dopesheet_focus.clone(),
                DopesheetFocus::Owner(_) => DopesheetFocus::Owner(KeyOwner::DrawOrder {}),
            },
            Some(TreeItem::Bone(bone)) => DopesheetFocus::Bone(*bone),
            Some(TreeItem::Slot(slot) | TreeItem::DrawOrderSlot { slot }) => doc
                .slots
                .get(*slot)
                .map_or(DopesheetFocus::All, |slot| DopesheetFocus::Bone(slot.bone)),
            Some(TreeItem::Attachment(_)) => self.selected_region_pose_bone().map_or_else(
                || {
                    self.timeline_owner()
                        .map_or(DopesheetFocus::All, DopesheetFocus::Owner)
                },
                DopesheetFocus::Bone,
            ),
            _ => self
                .timeline_owner()
                .map_or(DopesheetFocus::All, DopesheetFocus::Owner),
        };
        match focus {
            DopesheetFocus::Bone(bone) if bone >= doc.bones.len() => DopesheetFocus::All,
            DopesheetFocus::Owner(
                KeyOwner::Ik { constraint } | KeyOwner::Slider { constraint },
            ) if constraint >= doc.constraints.len() => DopesheetFocus::All,
            _ => focus,
        }
    }

    pub(super) fn reconcile_dopesheet_focus(&mut self) {
        self.dopesheet_focus = self.effective_dopesheet_focus();
    }

    pub(super) fn compound_slot_owners(&self, bone: usize) -> Vec<KeyOwner> {
        let doc = self.session.document();
        let mut slots: Vec<_> = doc
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.bone == bone)
            .collect();
        // Native Dopesheet siblings use ascending names independently of the
        // front-to-back ordering used by the Draw order Tree branch.
        slots.sort_by(|(ai, a), (bi, b)| a.name.cmp(&b.name).then(ai.cmp(bi)));
        slots
            .into_iter()
            .map(|(slot, _)| KeyOwner::Slot { slot })
            .collect()
    }

    pub(super) fn timeline_scope(&self) -> TimelineScope {
        match self.effective_dopesheet_focus() {
            DopesheetFocus::All => TimelineScope::All,
            DopesheetFocus::Owner(owner) => TimelineScope::Owners(vec![owner]),
            DopesheetFocus::Bone(bone) => {
                let mut owners = vec![KeyOwner::Bone { bone }];
                owners.extend(self.compound_slot_owners(bone));
                owners.push(KeyOwner::DrawOrder {});
                TimelineScope::Owners(owners)
            }
        }
    }

    pub(super) fn graph_owner_in_scope(&self, owner: &KeyOwner) -> bool {
        match self.timeline_scope() {
            TimelineScope::All => true,
            TimelineScope::Owners(owners) => owners.contains(owner),
        }
    }

    pub(super) fn dopesheet_focus_diagnostic(&self) -> serde_json::Value {
        match self.effective_dopesheet_focus() {
            DopesheetFocus::All => serde_json::json!({"type":"all"}),
            DopesheetFocus::Bone(bone) => serde_json::json!({"type":"bone","bone":bone}),
            DopesheetFocus::Owner(owner) => serde_json::json!({"type":"owner","owner":owner}),
        }
    }
}
