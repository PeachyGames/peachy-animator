//! Presentation focus never supplies a pose, clipboard or curve edit target.
use super::dopesheet_selection::TimelineScope;
use super::*;
use peachy_engine::KeyOwner;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum DopesheetFocus {
    #[default]
    All,
    Bones(Vec<usize>),
    Owner(KeyOwner),
}

impl Editor {
    pub(super) fn effective_dopesheet_focus(&self) -> DopesheetFocus {
        let doc = self.session.document();
        if self.selection.len() > 1 {
            return DopesheetFocus::Bones(self.selected_bones());
        }
        let focus = match self.selection.single() {
            None => DopesheetFocus::All,
            Some(TreeItem::DrawOrder) => match &self.dopesheet_focus {
                DopesheetFocus::All | DopesheetFocus::Bones(_) => self.dopesheet_focus.clone(),
                DopesheetFocus::Owner(_) => DopesheetFocus::Owner(KeyOwner::DrawOrder {}),
            },
            Some(TreeItem::Bone(bone)) => DopesheetFocus::Bones(vec![*bone]),
            Some(TreeItem::Slot(slot) | TreeItem::DrawOrderSlot { slot }) => {
                doc.slots.get(*slot).map_or(DopesheetFocus::All, |slot| {
                    DopesheetFocus::Bones(vec![slot.bone])
                })
            }
            Some(TreeItem::Attachment(_)) => self.selected_region_pose_bone().map_or_else(
                || {
                    self.timeline_owner()
                        .map_or(DopesheetFocus::All, DopesheetFocus::Owner)
                },
                |bone| DopesheetFocus::Bones(vec![bone]),
            ),
            _ => self
                .timeline_owner()
                .map_or(DopesheetFocus::All, DopesheetFocus::Owner),
        };
        if matches!(&focus,DopesheetFocus::Bones(bones) if bones.iter().any(|bone| *bone >= doc.bones.len()))
            || matches!(&focus,DopesheetFocus::Owner(KeyOwner::Ik{constraint}|KeyOwner::Slider{constraint}) if *constraint >= doc.constraints.len())
        {
            DopesheetFocus::All
        } else {
            focus
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
            DopesheetFocus::Bones(bones) => {
                let mut owners = Vec::new();
                for bone in bones {
                    owners.push(KeyOwner::Bone { bone });
                    owners.extend(self.compound_slot_owners(bone));
                }
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
            DopesheetFocus::Bones(bones) if bones.len() == 1 => {
                serde_json::json!({"type":"bone","bone":bones[0]})
            }
            DopesheetFocus::Bones(bones) => serde_json::json!({"type":"bones","bones":bones}),
            DopesheetFocus::Owner(owner) => serde_json::json!({"type":"owner","owner":owner}),
        }
    }
}
