//! Mesh deform authoring. World drags change offsets, never setup geometry or weights.
use crate::{Affine, AuthoredPose, Command, checked_pose_from_authored, deform_keyframe_at};
use peachy_document::{
    Attachment, AttachmentRef, Curve, DeformKey, DeformTrack, Document, Mesh, MeshVertices,
};

pub(crate) fn mesh_target<'a>(
    doc: &'a Document,
    attachment: &AttachmentRef,
) -> Result<(&'a Mesh, AttachmentRef, usize), String> {
    let Some(Attachment::Mesh(mesh)) = doc.attachment(attachment) else {
        return Err("Deform authoring needs an existing mesh attachment".into());
    };
    let slot = doc
        .slots
        .iter()
        .find(|slot| slot.name == attachment.slot)
        .ok_or("Unknown attachment slot")?;
    Ok((mesh, doc.deform_target(attachment), slot.bone))
}

pub(crate) fn validate_offsets(
    doc: &Document,
    attachment: &AttachmentRef,
    offsets: &[f32],
) -> Result<AttachmentRef, String> {
    let (mesh, canonical, _) = mesh_target(doc, attachment)?;
    if offsets.len() != mesh.deform_len() || offsets.iter().any(|value| !value.is_finite()) {
        return Err(format!(
            "Deform pose needs {} finite offsets",
            mesh.deform_len()
        ));
    }
    Ok(canonical)
}

/// Slot names can change while slot indices remain stable. No current command
/// removes/reorders slots; future such commands need stable entity identities.
pub(crate) fn remapped_attachment(
    before: &Document,
    after: &Document,
    attachment: &AttachmentRef,
) -> Result<AttachmentRef, String> {
    let index = before
        .slots
        .iter()
        .position(|slot| slot.name == attachment.slot)
        .ok_or("Unknown deform slot")?;
    let mut next = attachment.clone();
    next.slot = after
        .slots
        .get(index)
        .ok_or("Deform slot was removed")?
        .name
        .clone();
    Ok(next)
}

pub(crate) fn check_layout_transition(
    before: &Document,
    after: &Document,
    attachment: &AttachmentRef,
) -> Result<AttachmentRef, String> {
    let next = remapped_attachment(before, after, attachment)?;
    let previous = before
        .attachment(attachment)
        .and_then(Attachment::vertices)
        .ok_or("Unknown deform attachment")?;
    let current = after
        .attachment(&next)
        .and_then(Attachment::vertices)
        .ok_or("Deform attachment was removed")?;
    let same = match (previous, current) {
        (MeshVertices::SlotLocal { positions: a }, MeshVertices::SlotLocal { positions: b }) => {
            a.len() == b.len()
        }
        (MeshVertices::Weighted { influences: a }, MeshVertices::Weighted { influences: b }) => {
            a.len() == b.len()
                && a.iter().zip(b).all(|(a, b)| {
                    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.bone == b.bone)
                })
        }
        _ => false,
    };
    if !same {
        return Err("Mesh topology or binding layout cannot change while deform keys or pending deform offsets refer to it".into());
    }
    Ok(next)
}

pub(crate) fn offsets(
    doc: &Document,
    pose: &AuthoredPose,
    attachment: &AttachmentRef,
) -> Result<Vec<f32>, String> {
    let (mesh, canonical, _) = mesh_target(doc, attachment)?;
    Ok(pose
        .deforms
        .iter()
        .find(|(key, _)| *key == canonical)
        .map(|(_, values)| values.clone())
        .unwrap_or_else(|| vec![0.; mesh.deform_len()]))
}

pub(crate) fn geometry(
    doc: &Document,
    authored: &AuthoredPose,
    attachment: &AttachmentRef,
) -> Result<Vec<[f32; 2]>, String> {
    let (mesh, _, slot_bone) = mesh_target(doc, attachment)?;
    let evaluated = checked_pose_from_authored(doc, authored)?;
    let vertices = crate::deformed_mesh_world_vertices(
        mesh,
        slot_bone,
        evaluated.world(),
        &offsets(doc, authored, attachment)?,
    );
    if vertices.iter().flatten().any(|value| !value.is_finite()) {
        return Err("Deform pose produces nonfinite world geometry".into());
    }
    Ok(vertices)
}

pub(crate) fn check_key_continuity(
    doc: &Document,
    before: &AuthoredPose,
    next: &Document,
    after: &AuthoredPose,
    attachments: &[AttachmentRef],
) -> Result<(), String> {
    for attachment in attachments {
        let before = geometry(doc, before, attachment)?;
        let after = geometry(next, after, attachment)?;
        if before
            .iter()
            .flatten()
            .zip(after.iter().flatten())
            .any(|(a, b)| (f64::from(*a) - f64::from(*b)).abs() > 0.001)
        {
            return Err("Keying deform would move vertices beyond coordinate precision (0.001); keep the pending pose or use a representable key time".into());
        }
    }
    Ok(())
}

pub(crate) fn set_key(
    doc: &mut Document,
    animation: usize,
    attachment: &AttachmentRef,
    key: DeformKey,
) -> Result<(), String> {
    let canonical = validate_offsets(doc, attachment, &key.offsets)?;
    if !key.time.is_finite() || key.time < 0. || !key.curve.handles_valid() {
        return Err("Deform key needs finite nonnegative time and finite curve handles".into());
    }
    let clip = doc
        .animations
        .get_mut(animation)
        .ok_or("Unknown animation")?;
    let index = match clip
        .deform_tracks
        .iter()
        .position(|track| track.attachment == canonical)
    {
        Some(index) => index,
        None => {
            clip.deform_tracks.push(DeformTrack {
                attachment: canonical,
                keys: vec![],
            });
            clip.deform_tracks.len() - 1
        }
    };
    let keys = &mut clip.deform_tracks[index].keys;
    // The command carries a concrete source identity. Closely spaced imported
    // keys must not be merged just because the UI also accepts nearby cursors.
    if let Some(index) = keys.iter().position(|existing| existing.time == key.time) {
        keys[index] = key;
    } else {
        keys.push(key);
    }
    keys.sort_by(|a, b| a.time.total_cmp(&b.time));
    Ok(())
}

pub(crate) fn delete_key(
    doc: &mut Document,
    animation: usize,
    attachment: &AttachmentRef,
    time: f32,
) -> Result<(), String> {
    let (_, canonical, _) = mesh_target(doc, attachment)?;
    let clip = doc
        .animations
        .get_mut(animation)
        .ok_or("Unknown animation")?;
    if !time.is_finite() || time < 0. {
        return Err("Delete time must be finite and nonnegative".into());
    }
    // Match the key-state query: a nearby cursor can identify the final
    // imported key even when it lies just past the clip's key-derived end.
    // A valid cursor with no matching source identity is a no-op.
    if let Some(track) = clip
        .deform_tracks
        .iter_mut()
        .find(|track| track.attachment == canonical)
    {
        let identity = deform_keyframe_at(&track.keys, time)?.map(|key| key.time);
        if let Some(time) = identity {
            track.keys.retain(|key| key.time != time);
        }
    }
    clip.deform_tracks.retain(|track| !track.keys.is_empty());
    Ok(())
}

pub(crate) fn key_command(
    doc: &Document,
    animation: usize,
    time: f32,
    attachment: AttachmentRef,
    offsets: Vec<f32>,
) -> Result<Command, String> {
    let canonical = validate_offsets(doc, &attachment, &offsets)?;
    let clip = doc.animations.get(animation).ok_or("Unknown animation")?;
    let snapped = crate::commands::snapped_time(doc.fps, time)?;
    let keys = clip
        .deform_tracks
        .iter()
        .find(|track| track.attachment == canonical)
        .map_or(&[][..], |track| track.keys.as_slice());
    let existing = match deform_keyframe_at(keys, time)? {
        Some(key) => Some(key),
        None => deform_keyframe_at(keys, snapped)?,
    };
    Ok(Command::SetDeformKey {
        animation,
        attachment: canonical,
        key: DeformKey {
            time: existing.map_or(snapped, |key| key.time),
            curve: existing.map_or(Curve::Linear, |key| key.curve),
            offsets,
        },
    })
}

fn inverse_delta(frame: Affine, delta: [f64; 2]) -> Result<[f64; 2], String> {
    let [a, b, c, d] = [frame.a, frame.b, frame.c, frame.d].map(f64::from);
    let determinant = a * d - b * c;
    if !determinant.is_finite() || determinant == 0. {
        return Err("Deform translation needs invertible contributing bone transforms".into());
    }
    let result = [
        (d * delta[0] - b * delta[1]) / determinant,
        (a * delta[1] - c * delta[0]) / determinant,
    ];
    if !result.into_iter().all(f64::is_finite) {
        return Err("Deform translation exceeds the finite coordinate range".into());
    }
    Ok(result)
}

/// Preserve all unselected offsets and all binding data. Positive weighted
/// influences each receive the same world displacement divided by their total.
/// This supports repeated bones and nonunit totals without normalization.
pub(crate) fn translated_vertex(
    doc: &Document,
    authored: &AuthoredPose,
    attachment: &AttachmentRef,
    vertex: usize,
    delta: [f32; 2],
) -> Result<Vec<f32>, String> {
    if !delta.into_iter().all(f32::is_finite) {
        return Err("Deform world delta must be finite".into());
    }
    let (mesh, _, slot_bone) = mesh_target(doc, attachment)?;
    if vertex >= mesh.vertex_count() {
        return Err("Unknown mesh vertex".into());
    }
    let evaluated = checked_pose_from_authored(doc, authored)?;
    let mut values = offsets(doc, authored, attachment)?;
    validate_offsets(doc, attachment, &values)?;
    let before = crate::deformed_mesh_world_vertices(mesh, slot_bone, evaluated.world(), &values);
    let mut add = |index: usize, local: [f64; 2]| -> Result<(), String> {
        for axis in 0..2 {
            if local[axis] != 0. {
                values[index + axis] = (f64::from(values[index + axis]) + local[axis]) as f32;
            }
            if !values[index + axis].is_finite() {
                return Err("Deform offset exceeds the finite coordinate range".into());
            }
        }
        Ok(())
    };
    match &mesh.vertices {
        MeshVertices::SlotLocal { .. } => {
            add(
                vertex * 2,
                inverse_delta(evaluated.world()[slot_bone], delta.map(f64::from))?,
            )?;
        }
        MeshVertices::Weighted { influences } => {
            let entries = &influences[vertex];
            let total: f64 = entries.iter().map(|entry| f64::from(entry.weight)).sum();
            if !total.is_finite() || total <= 0. {
                return Err("Deform translation needs a positive finite influence total".into());
            }
            let mut index = influences[..vertex].iter().map(Vec::len).sum::<usize>() * 2;
            for entry in entries {
                if entry.weight > 0. {
                    let local = inverse_delta(
                        evaluated.world()[entry.bone],
                        delta.map(|value| f64::from(value) / total),
                    )?;
                    add(index, local)?;
                }
                index += 2;
            }
        }
    }
    let after = crate::deformed_mesh_world_vertices(mesh, slot_bone, evaluated.world(), &values);
    if before
        .iter()
        .chain(&after)
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err("Deform translation produces nonfinite world geometry".into());
    }
    for axis in 0..2 {
        let expected = f64::from(before[vertex][axis]) + f64::from(delta[axis]);
        if (f64::from(after[vertex][axis]) - expected).abs() > 0.001 {
            return Err(
                "Deform translation cannot preserve the requested world position within 0.001"
                    .into(),
            );
        }
    }
    Ok(values)
}
