//! Shared curve actions, distinct from exact raw setters and presentation state.
use crate::{
    AnimationKey, Command, CurveChannel, CurveData, CurveTarget, curve_channels_for_targets,
    curve_targets,
};
use peachy_document::{Animation, Curve, Document, IkCurve};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurveSelection {
    pub key: AnimationKey,
    pub channel: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CurvePreset {
    Linear,
    Stepped,
    Bezier,
    Custom([f64; 4]),
}

fn selected_channels(
    document: &Document,
    animation: usize,
    selection: &[CurveSelection],
) -> Result<Vec<CurveChannel>, String> {
    let mut components = vec![];
    let mut seen = BTreeSet::new();
    for selected in selection {
        let key = crate::timeline::canonical_key(document, &selected.key)?;
        let limit = if matches!(key, AnimationKey::SlotColor { .. }) {
            4
        } else {
            1
        };
        if selected.channel >= limit {
            return Err("Curve selection channel is outside its key's channel range".into());
        }
        if seen.insert((key.id(), selected.channel)) {
            components.push(CurveSelection {
                key,
                channel: selected.channel,
            });
        }
    }
    let keys = components.iter().map(|s| s.key.clone()).collect::<Vec<_>>();
    let targets = curve_targets(document, animation, &keys)?;
    let single = keys
        .iter()
        .map(AnimationKey::id)
        .collect::<BTreeSet<_>>()
        .len()
        == 1;
    let channels = curve_channels_for_targets(document, animation, &targets)?;
    let mut result = vec![];
    for selected in components {
        let target = if single {
            targets.first()
        } else {
            targets.iter().find(|t| t.key == selected.key)
        };
        if let Some(target) = target {
            let channel = channels
                .iter()
                .find(|c| c.target == *target && c.channel == selected.channel)
                .ok_or("Selected key has no matching curve channel")?;
            result.push(channel.clone());
        }
    }
    Ok(result)
}

fn surrounding<T>(
    keys: &[T],
    target: &CurveTarget,
    read: fn(&T) -> f32,
) -> Result<Vec<CurveTarget>, String> {
    let index = crate::timeline::exact_index(keys, target.key.time(), read)?;
    if keys
        .get(index + 1)
        .is_none_or(|key| read(key) != target.next_time)
    {
        return Err("Stale curve interval".into());
    }
    let mut result = vec![];
    if index > 0 {
        result.push(CurveTarget {
            key: target.key.at(read(&keys[index - 1])),
            next_time: target.key.time(),
        });
    }
    if let Some(next) = keys.get(index + 2) {
        result.push(CurveTarget {
            key: target.key.at(target.next_time),
            next_time: read(next),
        });
    }
    Ok(result)
}

fn neighbors(clip: &Animation, target: &CurveTarget) -> Result<Vec<CurveTarget>, String> {
    match &target.key {
        AnimationKey::Bone { bone, property, .. } => surrounding(
            &clip
                .tracks
                .iter()
                .find(|t| t.bone == *bone && t.property == *property)
                .ok_or("Unknown scalar track")?
                .keys,
            target,
            |k| k.time,
        ),
        AnimationKey::Ik { constraint, .. } => surrounding(
            &clip
                .ik_tracks
                .iter()
                .find(|t| t.constraint == *constraint)
                .ok_or("Unknown IK track")?
                .keys,
            target,
            |k| k.time,
        ),
        AnimationKey::Deform { attachment, .. } => surrounding(
            &clip
                .deform_tracks
                .iter()
                .find(|t| t.attachment == *attachment)
                .ok_or("Unknown Deform track")?
                .keys,
            target,
            |k| k.time,
        ),
        AnimationKey::SlotColor { slot, .. } => surrounding(
            &clip
                .color_tracks
                .iter()
                .find(|t| t.slot == *slot)
                .ok_or("Unknown color track")?
                .keys,
            target,
            |k| k.time,
        ),
        AnimationKey::SlotAttachment { .. } => Ok(vec![]),
    }
}

/// Sources read or changed by a handle operation, including both neighboring
/// intervals. Clients may freeze these descriptors once when a gesture begins.
pub fn curve_edit_targets(
    document: &Document,
    animation: usize,
    selection: &[CurveSelection],
) -> Result<Vec<CurveTarget>, String> {
    let selected = selected_channels(document, animation, selection)?;
    let clip = document
        .animations
        .get(animation)
        .ok_or("Unknown animation")?;
    let mut seen = BTreeSet::new();
    let mut result = vec![];
    for channel in selected {
        for target in
            std::iter::once(channel.target.clone()).chain(neighbors(clip, &channel.target)?)
        {
            if seen.insert(target.key.id()) {
                result.push(target);
            }
        }
    }
    Ok(result)
}

fn finite_f32(value: f64) -> Result<f32, String> {
    let converted = value as f32;
    if value.is_finite() && converted.is_finite() {
        Ok(converted)
    } else {
        Err("Curve control exceeds finite native coordinates".into())
    }
}

fn standard_values(channel: &CurveChannel) -> [f64; 2] {
    channel.values.map(|value| f64::from(value as f32))
}

fn absolute_handles(
    channel: &CurveChannel,
    curve: CurveData,
) -> Result<Option<[[f64; 2]; 2]>, String> {
    let start = f64::from(channel.target.key.time());
    let duration = f64::from(channel.target.next_time) - start;
    match curve {
        CurveData::Standard(Curve::Bezier(h) | Curve::BezierOffset(h)) => {
            let [left, right] = standard_values(channel);
            let span = if matches!(curve, CurveData::Standard(Curve::BezierOffset(_))) {
                1.
            } else {
                right - left
            };
            Ok(Some([
                [
                    start + duration * f64::from(h[0]),
                    left + span * f64::from(h[1]),
                ],
                [
                    start + duration * f64::from(h[2]),
                    left + span * f64::from(h[3]),
                ],
            ]))
        }
        CurveData::Ik(IkCurve::Bezier { time, mix }) => Ok(Some([
            [start + duration * time[0], f64::from(mix[0])],
            [start + duration * time[1], f64::from(mix[1])],
        ])),
        _ => Ok(None),
    }
}

fn edited_handle(
    channel: &CurveChannel,
    source: CurveData,
    handle: usize,
    point: [f64; 2],
) -> Result<CurveData, String> {
    if handle > 1 || !point.into_iter().all(f64::is_finite) {
        return Err("Curve handle needs index0/1 and finite absolute coordinates".into());
    }
    let start = f64::from(channel.target.key.time());
    let duration = f64::from(channel.target.next_time) - start;
    let x = (point[0] - start) / duration;
    match source {
        CurveData::Standard(Curve::Bezier(mut h)) => {
            let [left, right] = standard_values(channel);
            let span = right - left;
            if span == 0. {
                let mut offsets = [h[0], 0., h[2], 0.];
                offsets[handle * 2] = finite_f32(x)?;
                offsets[handle * 2 + 1] = finite_f32(point[1] - left)?;
                Ok(CurveData::Standard(Curve::BezierOffset(offsets)))
            } else {
                h[handle * 2] = finite_f32(x)?;
                h[handle * 2 + 1] = finite_f32((point[1] - left) / span)?;
                Ok(CurveData::Standard(Curve::Bezier(h)))
            }
        }
        CurveData::Standard(Curve::BezierOffset(mut h)) => {
            let left = standard_values(channel)[0];
            h[handle * 2] = finite_f32(x)?;
            h[handle * 2 + 1] = finite_f32(point[1] - left)?;
            Ok(CurveData::Standard(Curve::BezierOffset(h)))
        }
        CurveData::Ik(IkCurve::Bezier { mut time, mut mix }) => {
            time[handle] = x;
            mix[handle] = finite_f32(point[1])?;
            let curve = IkCurve::Bezier { time, mix };
            curve.validate()?;
            Ok(CurveData::Ik(curve))
        }
        _ => Err("Select a Bezier curve before editing a handle".into()),
    }
}

fn independent_preset(channel: &CurveChannel, preset: CurvePreset) -> Result<CurveData, String> {
    let is_ik = matches!(channel.curve, CurveData::Ik(_));
    match preset {
        CurvePreset::Linear => {
            return Ok(if is_ik {
                CurveData::Ik(IkCurve::Linear)
            } else {
                CurveData::Standard(Curve::Linear)
            });
        }
        CurvePreset::Stepped => {
            return Ok(if is_ik {
                CurveData::Ik(IkCurve::Stepped)
            } else {
                CurveData::Standard(Curve::Stepped)
            });
        }
        _ => {}
    }
    let h = match preset {
        CurvePreset::Custom(h) => h,
        CurvePreset::Bezier => {
            let start = channel.target.key.time();
            let end = channel.target.next_time;
            let third = (end - start) / 3.;
            let span = f64::from(end) - f64::from(start);
            [
                (f64::from(start + third) - f64::from(start)) / span,
                0.,
                (f64::from(end - third) - f64::from(start)) / span,
                1.,
            ]
        }
        _ => unreachable!(),
    };
    if !h.into_iter().all(f64::is_finite) {
        return Err("Preset controls must be finite".into());
    }
    if is_ik {
        let [left, right] = channel.values;
        let curve = IkCurve::Bezier {
            time: [h[0], h[2]],
            mix: [
                finite_f32(left + (right - left) * h[1])?,
                finite_f32(left + (right - left) * h[3])?,
            ],
        };
        curve.validate()?;
        Ok(CurveData::Ik(curve))
    } else {
        let h = [
            finite_f32(h[0])?,
            finite_f32(h[1])?,
            finite_f32(h[2])?,
            finite_f32(h[3])?,
        ];
        Ok(CurveData::Standard(
            if channel.values[0] == channel.values[1] {
                Curve::BezierOffset([h[0], 0., h[2], 0.])
            } else {
                Curve::Bezier(h)
            },
        ))
    }
}

fn commands_for_changes(
    document: &Document,
    animation: usize,
    changes: Vec<(CurveChannel, CurveData)>,
) -> Result<Vec<Command>, String> {
    let mut commands = vec![];
    let mut colors = BTreeMap::new();
    for (channel, curve) in changes {
        let time = channel.target.key.time();
        match (channel.target.key, curve) {
            (AnimationKey::Bone { bone, property, .. }, CurveData::Standard(curve)) => commands
                .push(Command::SetKeyCurve {
                    animation,
                    bone,
                    property,
                    time,
                    curve,
                }),
            (AnimationKey::Ik { constraint, .. }, CurveData::Ik(curve)) => {
                commands.push(Command::SetIkCurve {
                    animation,
                    constraint,
                    time,
                    curve,
                })
            }
            (AnimationKey::Deform { attachment, .. }, CurveData::Standard(curve)) => {
                commands.push(Command::SetDeformCurve {
                    animation,
                    attachment,
                    time,
                    curve,
                })
            }
            (AnimationKey::SlotColor { slot, .. }, CurveData::Standard(curve)) => {
                let curves = colors.entry((slot, time.to_bits())).or_insert_with(|| {
                    document.animations[animation]
                        .color_tracks
                        .iter()
                        .find(|t| t.slot == slot)
                        .unwrap()
                        .keys
                        .iter()
                        .find(|k| k.time == time)
                        .unwrap()
                        .curves
                });
                if curve == Curve::Stepped {
                    *curves = [Curve::Stepped; 4];
                } else {
                    if *curves == [Curve::Stepped; 4] {
                        *curves = [Curve::Linear; 4];
                    }
                    curves[channel.channel] = curve;
                }
            }
            _ => return Err("Curve data does not match its typed target".into()),
        }
    }
    for ((slot, time), curves) in colors {
        commands.push(Command::SetSlotColorCurves {
            animation,
            slot,
            time: f32::from_bits(time),
            curves,
        });
    }
    Ok(commands)
}

fn matched_preset(
    document: &Document,
    animation: usize,
    channels: &[CurveChannel],
    channel: &CurveChannel,
    preset: CurvePreset,
    mut curve: CurveData,
) -> Result<CurveData, String> {
    if !matches!(preset, CurvePreset::Bezier | CurvePreset::Custom(_)) {
        return Ok(curve);
    }
    let clip = &document.animations[animation];
    for neighbor in neighbors(clip, &channel.target)? {
        let source = crate::timeline::channels_for_target(document, clip, &neighbor)?
            .into_iter()
            .find(|c| c.channel == channel.channel)
            .ok_or("Missing neighboring curve channel")?;
        let Some(handles) = absolute_handles(&source, source.curve)? else {
            // Native reset ignores Linear/Stepped outer intervals.
            continue;
        };
        if channels
            .iter()
            .any(|selected| selected.target == neighbor && selected.channel == channel.channel)
        {
            return Err(
                "Preset matching on adjacent selected Bezier intervals is not calibrated; enable Separate"
                    .into(),
            );
        }
        if matches!(preset, CurvePreset::Custom(_)) {
            return Err("Custom preset matching against adjacent handles is not calibrated; enable Separate".into());
        }
        let previous = neighbor.next_time == channel.target.key.time();
        let index = if previous { 0 } else { 1 };
        let outer = handles[if previous { 1 } else { 0 }];
        let node_time = f64::from(if previous {
            channel.target.key.time()
        } else {
            channel.target.next_time
        });
        let source_value = source.values[if previous { 1 } else { 0 }];
        let node_value = channel.values[index];
        let dx = outer[0] - node_time;
        if dx == 0. || (previous && dx > 0.) || (!previous && dx < 0.) {
            return Err(
                "Vertical or outside-time linked matching is not calibrated; enable Separate"
                    .into(),
            );
        }
        let mut point = absolute_handles(channel, curve)?.ok_or("Preset is not Bezier")?[index];
        point[1] = node_value + (point[0] - node_time) * (outer[1] - source_value) / dx;
        let amplitude = (channel.values[1] - channel.values[0]).abs();
        let delta = point[1] - node_value;
        // Each measured limit is centered on that handle's endpoint, not the
        // whole plot's [-1,2] envelope. Flat values keep actual Offset
        // excursions and do not divide by a zero value span.
        if amplitude > 0. && delta.abs() > amplitude {
            let delta = delta.signum() * amplitude;
            point[0] = node_time + delta * dx / (outer[1] - source_value);
            point[1] = node_value + delta;
        }
        curve = edited_handle(channel, curve, index, point)?;
    }
    Ok(curve)
}

pub(crate) fn preset_commands(
    document: &Document,
    animation: usize,
    selection: &[CurveSelection],
    preset: CurvePreset,
    separate: bool,
) -> Result<Vec<Command>, String> {
    if let CurvePreset::Custom(h) = preset
        && !h.into_iter().all(f64::is_finite)
    {
        return Err("Preset controls must be finite".into());
    }
    let channels = selected_channels(document, animation, selection)?;
    let mut changes = vec![];
    for channel in &channels {
        let curve = independent_preset(channel, preset)?;
        let curve = if separate {
            curve
        } else {
            matched_preset(document, animation, &channels, channel, preset, curve)?
        };
        changes.push((channel.clone(), curve));
    }
    commands_for_changes(document, animation, changes)
}

pub(crate) struct HandleRequest<'a> {
    pub animation: usize,
    pub selection: &'a [CurveSelection],
    pub primary: &'a CurveSelection,
    pub handle: usize,
    pub point: [f64; 2],
    pub match_curves: bool,
    pub separate: bool,
}

fn live_neighbor_change(
    document: &Document,
    animation: usize,
    channels: &[CurveChannel],
    channel: &CurveChannel,
    handle: usize,
    point: [f64; 2],
) -> Result<Option<(CurveChannel, CurveData)>, String> {
    let clip = &document.animations[animation];
    let neighbor = neighbors(clip, &channel.target)?
        .into_iter()
        .find(|target| {
            if handle == 0 {
                target.next_time == channel.target.key.time()
            } else {
                target.key.time() == channel.target.next_time
            }
        });
    let Some(neighbor) = neighbor else {
        return Ok(None);
    };
    let source = crate::timeline::channels_for_target(document, clip, &neighbor)?
        .into_iter()
        .find(|source| source.channel == channel.channel)
        .ok_or("Missing neighboring curve channel")?;
    let Some(handles) = absolute_handles(&source, source.curve)? else {
        // Linear/Stepped intervals do not have an opposite Bezier handle.
        return Ok(None);
    };
    if channels
        .iter()
        .any(|selected| selected.target == neighbor && selected.channel == channel.channel)
    {
        return Err(
            "Live linked handles on adjacent selected intervals are not calibrated; enable Separate"
                .into(),
        );
    }
    let node_time = f64::from(if handle == 0 {
        channel.target.key.time()
    } else {
        channel.target.next_time
    });
    let node_value = channel.values[handle];
    let opposite = 1 - handle;
    let source_node_value = source.values[opposite];
    let mut outer = handles[opposite];
    let dx = point[0] - node_time;
    let outer_dx = outer[0] - node_time;
    if dx == 0.
        || outer_dx == 0.
        || point[0] < f64::from(channel.target.key.time())
        || point[0] > f64::from(channel.target.next_time)
        || outer[0] < f64::from(neighbor.key.time())
        || outer[0] > f64::from(neighbor.next_time)
    {
        return Err(
            "Vertical or outside-time live linked handles are not calibrated; enable Separate"
                .into(),
        );
    }
    let amplitude = (channel.values[1] - channel.values[0]).abs();
    let outer_amplitude = (source.values[1] - source.values[0]).abs();
    if amplitude == 0. || outer_amplitude == 0. {
        return Err("Live linked flat endpoints are not calibrated; enable Separate".into());
    }
    let delta = point[1] - node_value;
    let outer_delta = outer_dx * delta / dx;
    // This is a calibrated-domain guard, not a native clipping rule. Native
    // extreme live moves can change the opposite horizontal extent too, so
    // preserve neither a guessed clamp nor a guessed fixed-X extrapolation.
    if delta.abs() > amplitude || outer_delta.abs() > outer_amplitude {
        return Err("Extreme live linked-handle motion is not calibrated; enable Separate".into());
    }
    outer[1] = source_node_value + outer_delta;
    let mut changed = edited_handle(&source, source.curve, opposite, outer)?;
    // Ordinary native moves retain the horizontal extent apart from observed
    // small angle-conversion drift. Keep our exact source coordinate rather
    // than add drift or a normalization round trip to every preview frame.
    match (&mut changed, source.curve) {
        (
            CurveData::Standard(Curve::Bezier(h) | Curve::BezierOffset(h)),
            CurveData::Standard(Curve::Bezier(old) | Curve::BezierOffset(old)),
        ) => h[opposite * 2] = old[opposite * 2],
        (
            CurveData::Ik(IkCurve::Bezier { time, .. }),
            CurveData::Ik(IkCurve::Bezier { time: old, .. }),
        ) => {
            time[opposite] = old[opposite];
        }
        _ => unreachable!("A coupled handle retains its Bezier representation"),
    }
    Ok(Some((source, changed)))
}

pub(crate) fn handle_commands(
    document: &Document,
    request: HandleRequest<'_>,
) -> Result<Vec<Command>, String> {
    let HandleRequest {
        animation,
        selection,
        primary,
        handle,
        point,
        match_curves,
        separate,
    } = request;
    if handle > 1 || !point.into_iter().all(f64::is_finite) {
        return Err("Curve handle needs index0/1 and finite absolute coordinates".into());
    }
    let channels = selected_channels(document, animation, selection)?;
    let primary_key = crate::timeline::canonical_key(document, &primary.key)?;
    let source = channels
        .iter()
        .find(|c| c.target.key == primary_key && c.channel == primary.channel)
        .ok_or("Primary curve is not an effective member of the complete selection")?;
    let normalized = [
        (point[0] - f64::from(source.target.key.time()))
            / (f64::from(source.target.next_time) - f64::from(source.target.key.time())),
        (point[1] - source.values[0]) / (source.values[1] - source.values[0]),
    ];
    if match_curves && !normalized.into_iter().all(f64::is_finite) {
        return Err("Matching a flat primary curve has no calibrated normalized value".into());
    }
    let mut changes = vec![];
    for channel in &channels {
        if !match_curves
            && (channel.target.key != primary_key || channel.channel != primary.channel)
        {
            continue;
        }
        if match_curves && channel.values[0] == channel.values[1] {
            return Err(
                "Matching handles onto flat endpoints is awaiting native calibration".into(),
            );
        }
        let point = if match_curves {
            [
                f64::from(channel.target.key.time())
                    + (f64::from(channel.target.next_time) - f64::from(channel.target.key.time()))
                        * normalized[0],
                channel.values[0] + (channel.values[1] - channel.values[0]) * normalized[1],
            ]
        } else {
            point
        };
        changes.push((
            channel.clone(),
            edited_handle(channel, channel.curve, handle, point)?,
        ));
        if !separate
            && let Some(change) =
                live_neighbor_change(document, animation, &channels, channel, handle, point)?
        {
            changes.push(change);
        }
    }
    commands_for_changes(document, animation, changes)
}
