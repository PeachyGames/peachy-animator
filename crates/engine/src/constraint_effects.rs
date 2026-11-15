//! Computed identity changes for structural constraint edits and history.
use crate::{AnimationKey, KeyOwner};
use serde::Serialize;

#[derive(Clone, Debug)]
pub(crate) enum PoseOperation {
    Remap(EditEffects),
    ClearSliderTime { constraint: usize },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintIndexRemap {
    old_to_new: Vec<Option<usize>>,
    new_len: usize,
}

impl ConstraintIndexRemap {
    pub fn old_to_new(&self) -> &[Option<usize>] {
        &self.old_to_new
    }
    pub fn new_len(&self) -> usize {
        self.new_len
    }
    pub fn map_index(&self, old: usize) -> Result<Option<usize>, String> {
        self.old_to_new
            .get(old)
            .copied()
            .ok_or_else(|| "Constraint reference is outside the remap source".into())
    }
    pub fn map_owner(&self, owner: &KeyOwner) -> Result<Option<KeyOwner>, String> {
        Ok(match owner {
            KeyOwner::Ik { constraint } => self
                .map_index(*constraint)?
                .map(|constraint| KeyOwner::Ik { constraint }),
            KeyOwner::Slider { constraint } => self
                .map_index(*constraint)?
                .map(|constraint| KeyOwner::Slider { constraint }),
            _ => Some(owner.clone()),
        })
    }
    pub fn map_key(&self, key: &AnimationKey) -> Result<Option<AnimationKey>, String> {
        let mut next = key.clone();
        match &mut next {
            AnimationKey::Ik { constraint, .. } | AnimationKey::Slider { constraint, .. } => {
                let Some(mapped) = self.map_index(*constraint)? else {
                    return Ok(None);
                };
                *constraint = mapped;
            }
            _ => {}
        }
        Ok(Some(next))
    }
    pub(crate) fn insertion(old_len: usize, at: usize) -> Result<Self, String> {
        if at > old_len {
            return Err("Constraint insertion is outside its source layout".into());
        }
        let new_len = old_len.checked_add(1).ok_or("Constraint count overflow")?;
        Ok(Self {
            old_to_new: (0..old_len)
                .map(|index| Some(if index >= at { index + 1 } else { index }))
                .collect(),
            new_len,
        })
    }
    pub(crate) fn inverse(&self) -> Self {
        let mut old_to_new = vec![None; self.new_len];
        for (old, new) in self.old_to_new.iter().enumerate() {
            if let Some(new) = new {
                old_to_new[*new] = Some(old);
            }
        }
        Self {
            old_to_new,
            new_len: self.old_to_new.len(),
        }
    }
    fn then(&self, next: &Self) -> Result<Self, String> {
        if self.new_len != next.old_to_new.len() {
            return Err("Constraint remaps do not share an intermediate layout".into());
        }
        let old_to_new = self
            .old_to_new
            .iter()
            .map(|index| match index {
                Some(index) => next.map_index(*index),
                None => Ok(None),
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            old_to_new,
            new_len: next.new_len,
        })
    }
    fn is_identity(&self) -> bool {
        self.new_len == self.old_to_new.len()
            && self
                .old_to_new
                .iter()
                .enumerate()
                .all(|(index, value)| *value == Some(index))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditEffects {
    pub constraint_indices: Option<ConstraintIndexRemap>,
}

impl EditEffects {
    pub(crate) fn insertion(old_len: usize, at: usize) -> Result<Self, String> {
        Ok(Self {
            constraint_indices: Some(ConstraintIndexRemap::insertion(old_len, at)?),
        })
    }
    pub(crate) fn then(&self, next: &Self) -> Result<Self, String> {
        let constraint_indices = match (&self.constraint_indices, &next.constraint_indices) {
            (Some(before), Some(after)) => Some(before.then(after)?),
            (Some(map), None) | (None, Some(map)) => Some(map.clone()),
            (None, None) => None,
        }
        .filter(|map| !map.is_identity());
        Ok(Self { constraint_indices })
    }
    pub(crate) fn inverse(&self) -> Self {
        Self {
            constraint_indices: self
                .constraint_indices
                .as_ref()
                .map(ConstraintIndexRemap::inverse),
        }
    }
}
