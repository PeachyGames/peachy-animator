//! Semantic Setup edits, distinct from complete raw constraint replacement.
use crate::{SliderEligibility, SliderPoseEdit};
use peachy_document::{Constraint, Document, SliderProperty};

pub(crate) fn set_pose(
    doc: &mut Document,
    constraint: usize,
    edits: &[SliderPoseEdit],
    skin: &str,
) -> Result<(), String> {
    crate::slider_authoring::validate_edits(edits)?;
    if !doc.skins.contains_key(skin) {
        return Err("Unknown Slider Setup skin".into());
    }
    let slider = doc
        .constraints
        .get(constraint)
        .and_then(Constraint::as_slider)
        .ok_or("Unknown Slider constraint")?;
    for edit in edits {
        if let SliderPoseEdit::Time { value } = edit {
            if slider.bone.is_some() {
                return Err("Slider Frame is driven by its source bone".into());
            }
            if *value < 0. {
                return Err("Slider Setup Frame must be nonnegative".into());
            }
        }
    }
    let changed: Vec<_> = edits
        .iter()
        .copied()
        .filter(|edit| edit.value() != edit.property().read(slider.setup))
        .collect();
    if changed.is_empty() {
        return Ok(());
    }
    let mut next = slider.clone();
    let capture = slider.bone.is_some()
        && changed
            .iter()
            .any(|edit| edit.property() == SliderProperty::Mix);
    if capture {
        // A preceding raw command in the same atomic batch may have left an
        // invalid intermediate document. Validate before any indexed solve.
        doc.validate()?;
        let authored = crate::sample_authored_pose_with_skin(doc, None, 0., skin)?;
        let evaluated = crate::checked_pose_from_authored(doc, &authored)?;
        let turn = evaluated
            .slider_evaluations()
            .get(constraint)
            .copied()
            .flatten()
            .ok_or("Missing Slider own-turn evaluation")?;
        if turn.eligibility == SliderEligibility::Eligible
            && let Some(time) = turn.sample_time
        {
            let stored = time as f32;
            if !time.is_finite() || !stored.is_finite() {
                return Err(
                    "Slider captured Setup Frame is outside the finite native range".into(),
                );
            }
            next.setup.time = stored;
        }
        // Zero incoming Mix/inactive source/constraint retain their existing
        // hidden baseline. No forced zero-Mix source calculation or second pass.
    }
    for edit in changed {
        match edit {
            SliderPoseEdit::Time { value } => next.setup.time = value,
            SliderPoseEdit::Mix { value } => next.setup.mix = value,
        }
    }
    doc.constraints[constraint] = Constraint::Slider(next);
    Ok(())
}
