//! Exact persistent global draw-order keys and independent permutation encoding.
use peachy_document::{Animation, Document, DrawOrderKey, DrawOrderOffset};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawOrderPlacement {
    Behind,
    InFront,
}

/// Move one stable slot identity through a complete back-to-front permutation.
/// Positive deltas move toward the front; either boundary clamps to a no-op.
pub fn move_draw_order_slot(
    order: &[usize],
    slot: usize,
    delta: i32,
) -> Result<Vec<usize>, String> {
    validate_order(order.len(), order)?;
    let source = order
        .iter()
        .position(|&value| value == slot)
        .ok_or("Unknown draw order slot")?;
    let destination = if delta >= 0 {
        source.saturating_add(delta as usize).min(order.len() - 1)
    } else {
        source.saturating_sub(delta.unsigned_abs() as usize)
    };
    let mut result = order.to_vec();
    result.remove(source);
    result.insert(destination, slot);
    Ok(result)
}

/// Place one stable slot directly behind or in front of another stable slot.
/// Coordinates and the reversed Tree presentation never enter this operation.
pub fn place_draw_order_slot(
    order: &[usize],
    slot: usize,
    anchor: usize,
    placement: DrawOrderPlacement,
) -> Result<Vec<usize>, String> {
    validate_order(order.len(), order)?;
    let source = order
        .iter()
        .position(|&value| value == slot)
        .ok_or("Unknown draw order slot")?;
    if !order.contains(&anchor) {
        return Err("Unknown draw order anchor slot".into());
    }
    let mut result = order.to_vec();
    if slot != anchor {
        result.remove(source);
        let anchor_position = result.iter().position(|&value| value == anchor).unwrap();
        let destination = anchor_position + usize::from(placement == DrawOrderPlacement::InFront);
        result.insert(destination, slot);
    }
    Ok(result)
}

fn checked_time(time: f32) -> Result<(), String> {
    if !time.is_finite() || time < 0. {
        return Err("Draw order key time must be finite and nonnegative".into());
    }
    Ok(())
}

/// Exact imported identity first, otherwise one unambiguous nearby key.
pub fn draw_order_key_time(
    doc: &Document,
    animation: usize,
    time: f32,
) -> Result<Option<f32>, String> {
    checked_time(time)?;
    let keys = &doc
        .animations
        .get(animation)
        .ok_or("Unknown animation")?
        .draw_order;
    crate::commands::key_index_by_time(keys, time, |key| key.time)
        .map(|index| index.map(|index| keys[index].time))
}

pub(crate) fn key_command(
    doc: &Document,
    animation: usize,
    time: f32,
    order: &[usize],
) -> Result<crate::Command, String> {
    validate_order(doc.slots.len(), order)?;
    let snapped = crate::commands::snapped_time(doc.fps, time)?;
    let existing = match draw_order_key_time(doc, animation, time)? {
        Some(exact) => Some(exact),
        None => draw_order_key_time(doc, animation, snapped)?,
    };
    Ok(crate::Command::SetDrawOrder {
        animation,
        time: existing.unwrap_or(snapped),
        order: order.to_vec(),
    })
}

pub(crate) fn validate_order(slot_count: usize, order: &[usize]) -> Result<(), String> {
    if order.len() != slot_count {
        return Err("Draw order needs every slot exactly once".into());
    }
    let mut seen = vec![false; slot_count];
    for &slot in order {
        let entry = seen
            .get_mut(slot)
            .ok_or("Draw order contains an unknown slot")?;
        if *entry {
            return Err("Draw order needs every slot exactly once".into());
        }
        *entry = true;
    }
    Ok(())
}

pub(crate) fn key_from_order(
    slot_count: usize,
    time: f32,
    order: &[usize],
) -> Result<DrawOrderKey, String> {
    checked_time(time)?;
    validate_order(slot_count, order)?;

    // Unlisted slots fill free positions in setup order. Keeping a longest
    // increasing subsequence therefore minimizes the number of listed slots.
    // Patience sorting stores subsequence tails and predecessor indices. The
    // deterministic tie choice is ours, not a claim about the native editor.
    let mut tails: Vec<usize> = vec![];
    let mut previous = vec![None; slot_count];
    for (index, &slot) in order.iter().enumerate() {
        let length = tails.partition_point(|&tail| order[tail] < slot);
        if length > 0 {
            previous[index] = Some(tails[length - 1]);
        }
        if length == tails.len() {
            tails.push(index);
        } else {
            tails[length] = index;
        }
    }
    let mut kept = vec![false; slot_count];
    let mut cursor = tails.last().copied();
    while let Some(index) = cursor {
        kept[order[index]] = true;
        cursor = previous[index];
    }
    let mut positions = vec![0; slot_count];
    for (position, &slot) in order.iter().enumerate() {
        positions[slot] = position;
    }
    let mut offsets = vec![];
    for (slot, &position) in positions.iter().enumerate() {
        if !kept[slot] {
            let from = i64::try_from(slot).map_err(|_| "Too many draw order slots")?;
            let to = i64::try_from(position).map_err(|_| "Too many draw order slots")?;
            let offset = i32::try_from(to - from)
                .map_err(|_| "Draw order displacement exceeds the stored offset range")?;
            offsets.push(DrawOrderOffset { slot, offset });
        }
    }
    Ok(DrawOrderKey { time, offsets })
}

pub(crate) fn insert_key(clip: &mut Animation, key: DrawOrderKey) {
    if let Some(existing) = clip.draw_order.iter_mut().find(|old| old.time == key.time) {
        *existing = key;
    } else {
        clip.draw_order.push(key);
    }
    clip.draw_order.sort_by(|a, b| a.time.total_cmp(&b.time));
}

pub(crate) fn set_key(
    doc: &mut Document,
    animation: usize,
    key: DrawOrderKey,
) -> Result<(), String> {
    checked_time(key.time)?;
    key.order(doc.slots.len())?;
    let clip = doc
        .animations
        .get_mut(animation)
        .ok_or("Unknown animation")?;
    insert_key(clip, key);
    Ok(())
}

pub(crate) fn set_order(
    doc: &mut Document,
    animation: usize,
    time: f32,
    order: &[usize],
) -> Result<(), String> {
    checked_time(time)?;
    validate_order(doc.slots.len(), order)?;
    let clip = doc.animations.get(animation).ok_or("Unknown animation")?;
    let existing = clip.draw_order.iter().find(|key| key.time == time);
    if let Some(existing) = existing
        && existing.order(doc.slots.len())? == order
    {
        // Semantic re-keying never normalizes an untouched raw representation.
        return Ok(());
    }
    let key = key_from_order(
        doc.slots.len(),
        existing.map_or(time, |key| key.time),
        order,
    )?;
    insert_key(&mut doc.animations[animation], key);
    Ok(())
}
