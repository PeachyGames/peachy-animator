//! Source transitions and process-view range memory, never native document data.
use peachy_document::{Constraint, Document, SliderConstraint, SliderProperty, TransformProperty};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SliderSourceRange {
    pub min: f32,
    pub max: f32,
}
impl<'de> Deserialize<'de> for SliderSourceRange {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> serde::de::Visitor<'de> for ObjectVisitor {
            type Value = SliderSourceRange;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a Slider source range object with min and max")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                map: A,
            ) -> Result<Self::Value, A::Error> {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Fields {
                    min: f32,
                    max: f32,
                }
                let fields =
                    Fields::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
                Ok(SliderSourceRange {
                    min: fields.min,
                    max: fields.max,
                })
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}
impl SliderSourceRange {
    pub(crate) fn validate(self) -> Result<(), String> {
        if !self.min.is_finite() || !self.max.is_finite() || self.min == self.max {
            return Err("Slider source range needs distinct finite endpoints".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct SliderSourcePreferences {
    ranges: BTreeMap<TransformProperty, SliderSourceRange>,
}
impl SliderSourcePreferences {
    fn choice(&self, property: TransformProperty) -> SliderSourceRange {
        self.ranges
            .get(&property)
            .copied()
            .unwrap_or(match property {
                TransformProperty::ScaleX | TransformProperty::ScaleY => {
                    SliderSourceRange { min: 1., max: 2. }
                }
                _ => SliderSourceRange { min: 0., max: 100. },
            })
    }
    fn remember(&mut self, memo: SourceMemo) {
        self.ranges.insert(memo.property, memo.range);
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SourceMemo {
    pub property: TransformProperty,
    pub range: SliderSourceRange,
}
pub(crate) fn memo(slider: &SliderConstraint) -> Option<SourceMemo> {
    if slider.bone.is_none() || slider.scale == 0. {
        return None;
    }
    let property = slider.property?;
    let max = (f64::from(slider.from)
        + (f64::from(slider.max) - f64::from(slider.to)) / f64::from(slider.scale))
        as f32;
    let range = SliderSourceRange {
        min: slider.from,
        max,
    };
    range.validate().ok()?;
    Some(SourceMemo { property, range })
}

#[derive(Clone, Debug)]
pub(crate) enum SourceViewOperation {
    Remap(crate::EditEffects),
    Focus {
        constraint: usize,
        memo: Option<SourceMemo>,
    },
    Observe {
        constraint: usize,
        memo: Option<SourceMemo>,
    },
    Remember(SourceMemo),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SourceView {
    pub preferences: SliderSourcePreferences,
    pub focus: Option<usize>,
}
impl SourceView {
    pub fn preferences_for_transfer(&self, doc: &Document) -> SliderSourcePreferences {
        let mut preferences = self.preferences.clone();
        if let Some(current) = self
            .focus
            .and_then(|index| doc.constraints.get(index))
            .and_then(Constraint::as_slider)
            .and_then(memo)
        {
            preferences.remember(current);
        }
        preferences
    }
    pub fn focus(&mut self, doc: &Document, next: Option<usize>) -> Result<(), String> {
        if let Some(index) = next {
            doc.constraints
                .get(index)
                .and_then(Constraint::as_slider)
                .ok_or("Unknown Slider source context")?;
        }
        if next == self.focus {
            return Ok(());
        }
        if let Some(old) = self
            .focus
            .and_then(|index| doc.constraints.get(index))
            .and_then(Constraint::as_slider)
            .and_then(memo)
        {
            self.preferences.remember(old);
        }
        self.focus = next;
        Ok(())
    }
    pub fn choice(&self, doc: &Document, property: TransformProperty) -> SliderSourceRange {
        self.preferences_for_transfer(doc).choice(property)
    }
    pub fn remap(&mut self, effects: &crate::EditEffects) -> Result<(), String> {
        if let (Some(map), Some(index)) = (&effects.constraint_indices, self.focus) {
            self.focus = map.map_index(index)?;
        }
        Ok(())
    }
    pub fn apply(
        &self,
        doc: &Document,
        operations: &[SourceViewOperation],
    ) -> Result<Self, String> {
        let mut next = self.clone();
        let mut current = next
            .focus
            .and_then(|index| doc.constraints.get(index))
            .and_then(Constraint::as_slider)
            .and_then(memo);
        for operation in operations {
            match operation {
                SourceViewOperation::Remap(effects) => next.remap(effects)?,
                SourceViewOperation::Focus { constraint, memo } => {
                    if next.focus != Some(*constraint)
                        && let Some(current) = current
                    {
                        next.preferences.remember(current);
                    }
                    next.focus = Some(*constraint);
                    current = *memo;
                }
                SourceViewOperation::Observe { constraint, memo }
                    if next.focus == Some(*constraint) =>
                {
                    current = *memo
                }
                SourceViewOperation::Observe { .. } => {}
                SourceViewOperation::Remember(memo) => next.preferences.remember(*memo),
            }
        }
        Ok(next)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SourceChange {
    pub before: Option<SourceMemo>,
    pub after: Option<SourceMemo>,
    pub assigned: bool,
}
pub(crate) fn set_source(
    doc: &mut Document,
    constraint: usize,
    bone: Option<usize>,
) -> Result<Option<SourceChange>, String> {
    if bone.is_some_and(|index| index >= doc.bones.len()) {
        return Err("Unknown Slider source bone".into());
    }
    let current = doc
        .constraints
        .get(constraint)
        .and_then(Constraint::as_slider)
        .ok_or("Unknown Slider constraint")?;
    if current.bone == bone {
        return Ok(None);
    }
    let before = memo(current);
    let mut next = current.clone();
    next.bone = bone;
    let mut after = None;
    if bone.is_some() && next.property.is_none() {
        let duration = doc
            .animations
            .get(next.animation)
            .ok_or("Unknown Slider reference animation")?
            .duration();
        next.property = Some(TransformProperty::Rotate);
        next.local = true;
        next.from = 0.;
        next.to = 0.;
        next.max = duration;
        next.scale = (f64::from(duration) / 100.) as f32;
        after = Some(SourceMemo {
            property: TransformProperty::Rotate,
            range: SliderSourceRange { min: 0., max: 100. },
        });
    }
    next.validate(&doc.bones)?;
    let after = after.or_else(|| memo(&next));
    if bone.is_some() {
        for clip in &mut doc.animations {
            clip.slider_tracks.retain(|track| {
                track.constraint != constraint || track.property != SliderProperty::Time
            });
        }
    }
    doc.constraints[constraint] = Constraint::Slider(next);
    Ok(Some(SourceChange {
        before,
        after,
        assigned: bone.is_some(),
    }))
}
pub(crate) fn set_property(
    doc: &mut Document,
    constraint: usize,
    property: TransformProperty,
    range: SliderSourceRange,
) -> Result<Option<SourceChange>, String> {
    range.validate()?;
    let current = doc
        .constraints
        .get(constraint)
        .and_then(Constraint::as_slider)
        .ok_or("Unknown Slider constraint")?;
    if current.bone.is_none() {
        return Err("Slider source property requires a source bone".into());
    }
    if current.property == Some(property) {
        return Ok(None);
    }
    let before = memo(current);
    let mut next = current.clone();
    let scale = (f64::from(current.max) - f64::from(current.to))
        / (f64::from(range.max) - f64::from(range.min));
    let encoded = scale as f32;
    if !scale.is_finite() || !encoded.is_finite() || (scale != 0. && encoded == 0.) {
        return Err("Slider source range exceeds native coordinate precision".into());
    }
    next.property = Some(property);
    next.from = range.min;
    next.scale = encoded;
    next.validate(&doc.bones)?;
    doc.constraints[constraint] = Constraint::Slider(next);
    Ok(Some(SourceChange {
        before,
        after: Some(SourceMemo { property, range }),
        assigned: false,
    }))
}
