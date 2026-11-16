//! Referenced Slider timeline domains applied to one live effective state.
//! Math is independently calibrated by original domain-02, holdout-03 and
//! other-slider-additive and sequence-04 public-API reports. No recursive
//! sampling or solving.
use crate::ik::AppliedState;
use crate::slider_sampling::{scalar, value_between};
use peachy_document::{
    Animation, Attachment, AttachmentRef, ColorTrack, Curve, DeformTrack, Document, PathProperty,
    SliderProperty,
};

/// Source families not yet calibrated must not disappear from a reference.
/// Physics/events remain data-only elsewhere; this does not implement their
/// application, physics simulation or event delivery through a Slider.
pub(crate) fn validate_reference(doc: &Document, clip: &Animation) -> Result<(), String> {
    let pending = [
        (
            "physics",
            clip.physics.iter().any(|track| !track.keys.is_empty()),
        ),
        (
            "physics reset",
            clip.physics_resets
                .iter()
                .any(|track| !track.times.is_empty()),
        ),
        ("event", !clip.events.is_empty()),
    ];
    if let Some((domain, _)) = pending.into_iter().find(|(_, present)| *present) {
        return Err(format!(
            "referenced animation {}: {domain} timeline application awaits calibration",
            clip.name
        ));
    }
    for track in clip
        .deform_tracks
        .iter()
        .filter(|track| !track.keys.is_empty())
    {
        match doc.attachment(&track.attachment) {
            Some(Attachment::Mesh(mesh)) if mesh.link.is_none() => {}
            _ => {
                return Err(format!(
                    "referenced animation {}: linked, path or clipping Deform application awaits calibration",
                    clip.name
                ));
            }
        }
    }
    Ok(())
}

fn stored(value: f64, domain: &str) -> Result<f32, String> {
    let native = value as f32;
    if !value.is_finite() || !native.is_finite() {
        return Err(format!(
            "{domain}: applied value is outside the finite native range"
        ));
    }
    Ok(native)
}

fn wide(value: f64, domain: &str) -> Result<f64, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{domain}: applied value is nonfinite"))
    }
}

fn color(track: &ColorTrack, time: f64) -> Option<[f64; 4]> {
    let index = track
        .keys
        .iter()
        .rposition(|key| f64::from(key.time) <= time)?;
    let left = &track.keys[index];
    let channels = |bytes: [u8; 4]| bytes.map(|byte| f32::from(byte) / 255.);
    let a = channels(left.color);
    match track.keys.get(index + 1) {
        Some(right) => {
            let b = channels(right.color);
            let progress =
                (time - f64::from(left.time)) / (f64::from(right.time) - f64::from(left.time));
            Some(std::array::from_fn(|channel| {
                value_between(left.curves[channel], progress, a[channel], b[channel])
            }))
        }
        None => Some(a.map(f64::from)),
    }
}

fn deform(track: &DeformTrack, time: f64) -> Option<Vec<f64>> {
    let index = track
        .keys
        .iter()
        .rposition(|key| f64::from(key.time) <= time)?;
    let left = &track.keys[index];
    match track.keys.get(index + 1) {
        Some(right) => {
            let progress =
                (time - f64::from(left.time)) / (f64::from(right.time) - f64::from(left.time));
            let progress = match left.curve {
                Curve::Linear => progress,
                Curve::Stepped => 0.,
                other => f64::from(crate::curve_progress(other, progress as f32)),
            };
            Some(
                left.offsets
                    .iter()
                    .zip(&right.offsets)
                    .map(|(&a, &b)| f64::from(a) + (f64::from(b) - f64::from(a)) * progress)
                    .collect(),
            )
        }
        None => Some(left.offsets.iter().copied().map(f64::from).collect()),
    }
}

fn active_slot(doc: &Document, state: &AppliedState, slot: usize) -> bool {
    state.active.bones[doc.slots[slot].bone]
}

/// Apply only started tracks to the live state. The caller freezes this
/// Slider's query and performs the one ordered constraint pass. Returned bones
/// changed inheritance and need to join the caller's single FK refresh.
pub(crate) fn apply(
    doc: &Document,
    clip: &Animation,
    time: f64,
    mix: f64,
    additive: bool,
    state: &mut AppliedState,
) -> Result<Vec<usize>, String> {
    validate_reference(doc, clip)?;
    if !time.is_finite() || !mix.is_finite() {
        return Err("Slider domain application requires finite time and Mix".into());
    }
    if mix == 0. {
        return Ok(vec![]);
    }

    // Stage writes first: an unsupported target or nonfinite result never
    // exposes a partial domain application to the caller.
    let mut attachments = vec![];
    let mut colors = vec![];
    let mut inherits = vec![];
    let mut ik = vec![];
    let mut paths = vec![];
    let mut sliders = vec![];
    let mut deforms: Vec<(AttachmentRef, Vec<f32>)> = vec![];
    let mut sequences: Vec<(AttachmentRef, u32)> = vec![];
    for track in &clip.attachment_tracks {
        let Some(key) = track.keys.iter().rfind(|key| f64::from(key.time) <= time) else {
            continue;
        };
        if !active_slot(doc, state, track.slot) {
            continue;
        }
        if state.slot_attachments[track.slot] != key.attachment
            && state
                .deforms
                .iter()
                .any(|(key, _)| key.slot == doc.slots[track.slot].name)
        {
            return Err("attachment switching with incoming Deform awaits calibration".into());
        }
        attachments.push((track.slot, key.attachment.clone()));
    }
    for track in &clip.color_tracks {
        if !active_slot(doc, state, track.slot) {
            continue;
        }
        let Some(sample) = color(track, time) else {
            continue;
        };
        let current = state.slot_colors[track.slot];
        let mut value = [0.; 4];
        for channel in 0..4 {
            let current = f64::from(current[channel]);
            // Slot color ignores Additive; observed clamping is part of the
            // effective value, without quantizing back to authored RGBA bytes.
            let blended = wide(current + (sample[channel] - current) * mix, "slot color")?;
            value[channel] = stored(blended.clamp(0., 1.), "slot color")?;
        }
        colors.push((track.slot, value));
    }
    for track in &clip.inherit_tracks {
        let Some(key) = track.keys.iter().rfind(|key| f64::from(key.time) <= time) else {
            continue;
        };
        if !state.active.bones[track.bone] {
            continue;
        }
        if state.inherits[track.bone] != key.inherit {
            inherits.push((track.bone, key.inherit));
        }
    }
    let order = clip
        .draw_order
        .iter()
        .rfind(|key| f64::from(key.time) <= time)
        .map(|key| key.order(doc.slots.len()))
        .transpose()?;
    for track in &clip.ik_tracks {
        if track
            .keys
            .first()
            .is_none_or(|key| time < f64::from(key.time))
        {
            continue;
        }
        if !state.active.constraints[track.constraint] {
            continue;
        }
        let setup = doc.constraints[track.constraint]
            .as_ik()
            .ok_or("IK timeline has the wrong owner")?
            .setup;
        let sample = crate::ik_sampling::sample_ik_at_time(track, setup, time)?;
        let current = state.ik[track.constraint];
        ik.push((
            track.constraint,
            crate::IkPose {
                mix: wide(current.mix + (sample.mix - current.mix) * mix, "IK Mix")?,
                bend_positive: sample.bend_positive,
            },
        ));
    }
    for track in &clip.path_tracks {
        if !state.active.constraints[track.constraint] {
            continue;
        }
        let Some(sample) = scalar(&track.keys, time) else {
            continue;
        };
        let current = match track.property {
            PathProperty::Position => state.paths[track.constraint].position,
            PathProperty::Spacing => state.paths[track.constraint].spacing,
        };
        let current = f64::from(current);
        let delta = if additive && track.property == PathProperty::Position {
            sample
        } else {
            sample - current
        };
        paths.push((
            track.constraint,
            track.property,
            stored(current + delta * mix, "Path setting")?,
        ));
    }
    for track in &clip.slider_tracks {
        if !state.active.constraints[track.constraint] {
            continue;
        }
        let Some(sample) = scalar(&track.keys, time) else {
            continue;
        };
        let current = state.sliders[track.constraint].value(track.property);
        let delta = if additive { sample } else { sample - current };
        sliders.push((
            track.constraint,
            track.property,
            wide(current + delta * mix, "Slider setting")?,
        ));
    }
    for track in &clip.deform_tracks {
        let slot = doc
            .slots
            .iter()
            .position(|slot| slot.name == track.attachment.slot)
            .ok_or("Deform timeline has an unknown slot")?;
        if !active_slot(doc, state, slot) {
            continue;
        }
        let Some(sample) = deform(track, time) else {
            continue;
        };
        let shown = attachments
            .iter()
            .find(|(index, _)| *index == slot)
            .map_or(&state.slot_attachments[slot], |(_, shown)| shown);
        let resolved = shown
            .as_deref()
            .and_then(|name| doc.resolve_attachment(&state.skin, slot, name));
        let Some((key, Attachment::Mesh(mesh))) = resolved else {
            return Err(
                "Deform with a hidden or different shown attachment awaits calibration".into(),
            );
        };
        if mesh.link.is_some() || doc.deform_target(&key) != doc.deform_target(&track.attachment) {
            return Err(
                "Deform with a linked or different shown attachment awaits calibration".into(),
            );
        }
        let target = doc.deform_target(&track.attachment);
        let current = state
            .deforms
            .iter()
            .find(|(key, _)| *key == target)
            .map(|(_, offsets)| offsets);
        if current.is_some_and(|offsets| offsets.len() != sample.len()) {
            return Err("incoming Deform does not match its target layout".into());
        }
        let values = sample
            .into_iter()
            .enumerate()
            .map(|(index, sample)| {
                let current = current.map_or(0., |values| f64::from(values[index]));
                let delta = if additive { sample } else { sample - current };
                stored(current + delta * mix, "Deform")
            })
            .collect::<Result<Vec<_>, _>>()?;
        deforms.push((target, values));
    }
    for track in &clip.sequence_tracks {
        let slot = doc
            .slots
            .iter()
            .position(|slot| slot.name == track.attachment.slot)
            .ok_or("Sequence timeline has an unknown slot")?;
        if !active_slot(doc, state, slot) {
            continue;
        }
        let Some(key) = track.keys.iter().rfind(|key| f64::from(key.time) <= time) else {
            continue;
        };
        let shown = attachments
            .iter()
            .find(|(index, _)| *index == slot)
            .map_or(&state.slot_attachments[slot], |(_, shown)| shown);
        let resolved = shown
            .as_deref()
            .and_then(|name| doc.resolve_attachment(&state.skin, slot, name));
        let Some((shown_key, attachment)) = resolved else {
            return Err("Sequence with a hidden shown attachment awaits calibration".into());
        };
        if shown_key != track.attachment {
            return Err("Sequence with a different shown attachment awaits calibration".into());
        }
        let sequence = attachment
            .sequence()
            .ok_or("Sequence timeline target is not sequenced")?;
        // The shared checked helper keeps the derived query wide and refuses
        // unsupported huge phase rather than saturating or overflowing it.
        let frame = key.frame_at_time_checked(time, sequence.count)?;
        sequences.push((shown_key, frame));
    }

    for (slot, value) in attachments {
        state.slot_attachments[slot] = value;
    }
    for (slot, value) in colors {
        state.slot_colors[slot] = value;
    }
    let changed = inherits.iter().map(|(bone, _)| *bone).collect();
    for (bone, value) in inherits {
        state.inherits[bone] = value;
    }
    if let Some(order) = order {
        state.draw_order = order;
    }
    for (constraint, value) in ik {
        state.ik[constraint] = value;
    }
    for (constraint, property, value) in paths {
        match property {
            PathProperty::Position => state.paths[constraint].position = value,
            PathProperty::Spacing => state.paths[constraint].spacing = value,
        }
    }
    for (constraint, property, value) in sliders {
        match property {
            SliderProperty::Time => state.sliders[constraint].time = value,
            SliderProperty::Mix => state.sliders[constraint].mix = value,
        }
    }
    for (key, offsets) in deforms {
        match state.deforms.iter_mut().find(|(target, _)| *target == key) {
            Some((_, existing)) => *existing = offsets,
            None => state.deforms.push((key, offsets)),
        }
    }
    for (key, frame) in sequences {
        match state
            .sequence_frames
            .iter_mut()
            .find(|(target, _)| *target == key)
        {
            Some((_, existing)) => *existing = frame,
            None => state.sequence_frames.push((key, frame)),
        }
    }
    Ok(changed)
}

#[cfg(test)]
#[path = "slider_domains_tests.rs"]
mod tests;
