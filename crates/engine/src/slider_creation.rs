//! Semantic native creation, separate from complete raw append authoring.
use peachy_document::{Constraint, Document, SliderConstraint};

fn increment(digits: &str) -> String {
    let mut digits: Vec<_> = digits.bytes().rev().collect();
    let mut carry = true;
    for digit in &mut digits {
        if *digit == b'9' {
            *digit = b'0';
        } else {
            *digit += 1;
            carry = false;
            break;
        }
    }
    if carry {
        digits.push(b'1');
    }
    digits.into_iter().rev().map(char::from).collect()
}

fn name(doc: &Document, stem: &str) -> Result<String, String> {
    let exists = |name: &str| {
        doc.constraints
            .iter()
            .filter_map(Constraint::as_slider)
            .any(|slider| slider.name == name)
    };
    if !exists(stem) {
        return Ok(stem.to_owned());
    }
    let split = stem
        .char_indices()
        .rev()
        .find(|(_, ch)| !ch.is_ascii_digit())
        .map_or(0, |(at, ch)| at + ch.len_utf8());
    let (prefix, suffix) = stem.split_at(split);
    let (prefix, mut suffix) = if suffix.is_empty() {
        (stem, "2".to_owned())
    } else {
        let normalized = suffix.trim_start_matches('0');
        (
            prefix,
            increment(if normalized.is_empty() {
                "0"
            } else {
                normalized
            }),
        )
    };
    let limit = doc
        .constraints
        .len()
        .checked_add(1)
        .ok_or("Constraint count overflow")?;
    for _ in 0..limit {
        let candidate = format!("{prefix}{suffix}");
        if !exists(&candidate) {
            return Ok(candidate);
        }
        suffix = increment(&suffix);
    }
    Err("Unable to choose an unused Slider name".into())
}

pub(crate) fn create(doc: &mut Document, animation: usize) -> Result<(), String> {
    let clip = doc
        .animations
        .get(animation)
        .ok_or("Unknown Slider source animation")?;
    let value = SliderConstraint::new(name(doc, &clip.name)?, animation);
    let shift = |index: &mut usize| -> Result<(), String> {
        *index = index.checked_add(1).ok_or("Constraint index overflow")?;
        Ok(())
    };
    for clip in &mut doc.animations {
        for track in &mut clip.ik_tracks {
            shift(&mut track.constraint)?;
        }
        for track in &mut clip.slider_tracks {
            shift(&mut track.constraint)?;
        }
        for track in &mut clip.path_tracks {
            shift(&mut track.constraint)?;
        }
        for track in &mut clip.physics {
            if let Some(index) = &mut track.constraint {
                shift(index)?;
            }
        }
        for reset in &mut clip.physics_resets {
            if let Some(index) = &mut reset.constraint {
                shift(index)?;
            }
        }
    }
    let required = doc
        .skin_scope
        .required_constraints
        .iter()
        .map(|index| index.checked_add(1).ok_or("Constraint index overflow"))
        .collect::<Result<_, _>>()?;
    doc.skin_scope.required_constraints = required;
    for members in doc.skin_scope.skins.values_mut() {
        for index in &mut members.constraints {
            shift(index)?;
        }
    }
    doc.constraints.insert(0, Constraint::Slider(value));
    Ok(())
}
