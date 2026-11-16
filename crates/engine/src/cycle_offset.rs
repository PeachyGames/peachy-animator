//! Periodic key movement with original-curve memory, separate from ordinary retiming.
//! Exact selected Bone scalar keys seed whole closed property tracks. Other typed
//! timelines and tracks extending beyond the loop remain explicit extension gates.
use crate::{AnimationKey, curve_value};
use peachy_document::{Curve, Document, Keyframe, Property, Track};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CycleOffsetRange {
    /// Inclusive loop bounds in animation seconds.
    pub start: f32,
    pub end: f32,
}
impl CycleOffsetRange {
    fn duration(self) -> Result<f32, String> {
        let duration = self.end - self.start;
        if !self.start.is_finite()
            || !self.end.is_finite()
            || self.start < 0.
            || !duration.is_finite()
            || duration <= 0.
        {
            return Err("Cycle Offset needs finite nonnegative ordered loop bounds".into());
        }
        Ok(duration)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TrackId {
    animation: usize,
    bone: usize,
    property: Property,
    start: u32,
    end: u32,
}
impl TrackId {
    fn key(self, time: f32) -> AnimationKey {
        AnimationKey::Bone {
            bone: self.bone,
            property: self.property,
            time,
        }
    }
}

#[derive(Clone, Debug)]
struct RequestedSeed {
    track: TrackId,
    source: AnimationKey,
    selectable: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct TrackMemory {
    animation_name: String,
    bone_name: String,
    fps: u32,
    range: CycleOffsetRange,
    // History/transaction snapshots share immutable source buffers rather than
    // duplicating every remembered curve on unrelated edits.
    original: Arc<[Keyframe]>,
    expected: Arc<[Keyframe]>,
    phase: f32,
}

/// Session-only original curves and accepted phases for one active canonical
/// property-track cohort. The map's keys include animation and loop identity.
/// Shrinking keeps surviving origins; adding a track rebases current tracks.
/// Both transitions validate the old context before discarding any origins.
/// Never serialize this into an editable Document.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CycleOffsetMemory {
    tracks: BTreeMap<TrackId, TrackMemory>,
}
impl CycleOffsetMemory {
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }
    /// Explicitly forget origins before recapturing changed affected tracks.
    pub fn clear(&mut self) {
        self.tracks.clear();
    }
}

/// The original first key identifies a typed track; phase is relative to its
/// explicit loop start. This receipt does not promise a native key-selection policy.
#[derive(Clone, Debug, PartialEq)]
pub struct CycleOffsetPhase {
    pub source: AnimationKey,
    pub range: CycleOffsetRange,
    pub phase: f32,
}

/// One frozen source context, usable for many previews against the same baseline.
#[derive(Clone, Debug)]
pub struct CycleOffsetPlan {
    memory: CycleOffsetMemory,
    selected: Vec<TrackId>,
    requested: Vec<RequestedSeed>,
}

/// One requested source seed and its accepted target. This list preserves
/// request order; it is separate from all keys of the affected property tracks.
#[derive(Clone, Debug, PartialEq)]
pub struct CycleOffsetKeyMapping {
    pub source: AnimationKey,
    /// None marks an ignored generated seam. Targets can coincide when original
    /// first/end seeds follow the same accepted anchor.
    pub target: Option<AnimationKey>,
}

#[derive(Clone, Debug)]
pub struct CycleOffsetOutcome {
    pub document: Document,
    pub memory: CycleOffsetMemory,
    /// All current affected key IDs, in first-requested track order, then time.
    pub keys: Vec<AnimationKey>,
    /// The two loop-boundary IDs per affected track; these need not be new keys.
    pub boundary_keys: Vec<AnimationKey>,
    pub phases: Vec<CycleOffsetPhase>,
    /// Only requested seeds, never an instruction to select all affected keys.
    pub mappings: Vec<CycleOffsetKeyMapping>,
}

/// Accepted key/phase and requested-seed metadata, without another Document clone.
#[derive(Clone, Debug, PartialEq)]
pub struct CycleOffsetReceipt {
    pub keys: Vec<AnimationKey>,
    pub boundary_keys: Vec<AnimationKey>,
    pub phases: Vec<CycleOffsetPhase>,
    pub mappings: Vec<CycleOffsetKeyMapping>,
}
impl CycleOffsetOutcome {
    pub fn receipt(&self) -> CycleOffsetReceipt {
        CycleOffsetReceipt {
            keys: self.keys.clone(),
            boundary_keys: self.boundary_keys.clone(),
            phases: self.phases.clone(),
            mappings: self.mappings.clone(),
        }
    }
}

impl CycleOffsetPlan {
    pub fn capture(
        document: &Document,
        animation: usize,
        keys: &[AnimationKey],
        range: CycleOffsetRange,
    ) -> Result<Self, String> {
        Self::capture_with_memory(
            document,
            animation,
            keys,
            range,
            &CycleOffsetMemory::default(),
        )
    }

    pub fn capture_with_memory(
        document: &Document,
        animation: usize,
        keys: &[AnimationKey],
        range: CycleOffsetRange,
        memory: &CycleOffsetMemory,
    ) -> Result<Self, String> {
        let requested_keys = classify_requested(document, animation, keys, range, memory)?;
        let clip = document
            .animations
            .get(animation)
            .ok_or("Unknown animation for Cycle Offset")?;
        if keys.is_empty() {
            return Err("Select at least one existing key for Cycle Offset".into());
        }
        let mut selected = Vec::new();
        let mut requested = BTreeSet::new();
        for seed in &requested_keys {
            if seed.selectable && requested.insert(seed.track) {
                selected.push(seed.track);
            }
        }
        // Only actual anchors seed a property. Ignored generated boundaries do
        // not shrink/reset the active context when they are the entire request.
        if selected.is_empty() {
            return Ok(Self {
                memory: memory.clone(),
                selected,
                requested: requested_keys,
            });
        }
        let expands_context = selected.iter().any(|id| !memory.tracks.contains_key(id));
        let mut next = if expands_context {
            CycleOffsetMemory::default()
        } else {
            let mut retained = memory.clone();
            retained.tracks.retain(|id, _| requested.contains(id));
            retained
        };
        for id in &selected {
            let track = track(document, *id)?;
            if !next.tracks.contains_key(id) {
                let first = track
                    .keys
                    .first()
                    .ok_or("Cycle Offset needs a nonempty track")?;
                let last = track
                    .keys
                    .last()
                    .ok_or("Cycle Offset needs a nonempty track")?;
                if track.keys.len() < 2
                    || time_id(first.time) != id.start
                    || time_id(last.time) != id.end
                {
                    return Err("Cycle Offset needs explicit first/end keys at the loop bounds; tracks extending outside those bounds remain unsupported".into());
                }
                if first.value != last.value {
                    return Err("Cycle Offset first and last values must match".into());
                }
                let original: Arc<[Keyframe]> = track.keys.clone().into();
                next.tracks.insert(
                    *id,
                    TrackMemory {
                        animation_name: clip.name.clone(),
                        bone_name: document
                            .bones
                            .get(id.bone)
                            .ok_or("Unknown Cycle Offset bone")?
                            .name
                            .clone(),
                        fps: document.fps,
                        range,
                        original: original.clone(),
                        expected: original,
                        phase: 0.,
                    },
                );
            }
        }
        Ok(Self {
            memory: next,
            selected,
            requested: requested_keys,
        })
    }

    /// Recompute from remembered originals at each track's accepted phase plus
    /// this delta in animation seconds. The supplied current affected tracks must match the capture.
    /// A returned outcome is only a candidate; accepting it is the shell's job.
    pub fn advance(&self, document: &Document, delta: f32) -> Result<CycleOffsetOutcome, String> {
        document.validate()?;
        if !delta.is_finite() {
            return Err("Cycle Offset delta must be finite".into());
        }
        for (id, remembered) in &self.memory.tracks {
            validate_expected(document, *id, remembered)?;
        }
        let mut candidate = document.clone();
        let mut memory = self.memory.clone();
        let mut keys = Vec::new();
        let mut boundary_keys = Vec::new();
        let mut phases = Vec::new();
        for id in &self.selected {
            let entry = memory
                .tracks
                .get_mut(id)
                .ok_or("Missing Cycle Offset track memory")?;
            let phase = advance_phase(entry.phase, delta, entry.range.duration()?)?;
            let shifted = if phase.to_bits() == entry.phase.to_bits() {
                entry.expected.to_vec()
            } else if phase == 0. {
                // Full return to the source phase restores its raw curve/time bits.
                entry.original.to_vec()
            } else {
                offset_scalar(&entry.original, entry.range, phase)?
            };
            let target = candidate.animations[id.animation]
                .tracks
                .iter_mut()
                .find(|track| track.bone == id.bone && track.property == id.property)
                .ok_or("Cycle Offset target track disappeared")?;
            target.keys = shifted.clone();
            keys.extend(shifted.iter().map(|key| id.key(key.time)));
            boundary_keys.extend([
                id.key(shifted[0].time),
                id.key(shifted[shifted.len() - 1].time),
            ]);
            phases.push(CycleOffsetPhase {
                source: id.key(entry.original[0].time),
                range: entry.range,
                phase,
            });
            entry.phase = phase;
            entry.expected = shifted.into();
        }
        candidate.validate()?;
        let mappings = self.map_requested_keys(&memory)?;
        Ok(CycleOffsetOutcome {
            document: candidate,
            memory,
            keys,
            boundary_keys,
            phases,
            mappings,
        })
    }

    fn map_requested_keys(
        &self,
        accepted: &CycleOffsetMemory,
    ) -> Result<Vec<CycleOffsetKeyMapping>, String> {
        let mut lineages = BTreeMap::new();
        for id in &self.selected {
            let before = &self.memory.tracks[id];
            let after = &accepted.tracks[id];
            lineages.insert(
                *id,
                TrackLineage {
                    sources: anchor_lineage(before)?,
                    targets: anchor_times(after)?,
                    source_keys: before
                        .expected
                        .iter()
                        .map(|key| (time_id(key.time), key.time))
                        .collect(),
                    accepted_keys: after.expected.iter().map(|key| time_id(key.time)).collect(),
                },
            );
        }
        self.requested
            .iter()
            .map(|seed| {
                if !seed.selectable {
                    return Ok(CycleOffsetKeyMapping {
                        source: seed.source.clone(),
                        target: None,
                    });
                }
                let id = &seed.track;
                let source = &seed.source;
                let AnimationKey::Bone { time, .. } = source else {
                    return Err("Cycle Offset requested lineage must be a Bone key".into());
                };
                let before = &self.memory.tracks[id];
                let after = &accepted.tracks[id];
                let lineage = &lineages[id];
                let target_time = if before.phase.to_bits() == after.phase.to_bits() {
                    // Cache-only/full-period no-ops keep the exact current identity,
                    // including distinct selectable original first/end keys.
                    lineage
                        .source_keys
                        .get(&time_id(*time))
                        .copied()
                        .ok_or("Cycle Offset requested identity disappeared")?
                } else {
                    let anchor = lineage
                        .sources
                        .get(&time_id(*time))
                        .ok_or("Cycle Offset selectable source anchor disappeared")?;
                    lineage.targets[*anchor]
                };
                if !lineage.accepted_keys.contains(&time_id(target_time)) {
                    return Err("Cycle Offset accepted anchor identity disappeared".into());
                }
                Ok(CycleOffsetKeyMapping {
                    source: source.clone(),
                    target: Some(id.key(target_time)),
                })
            })
            .collect()
    }
}

/// Exact selectable-anchor subset in request order, without mutation or a pose
/// solve. Known generated seams are excluded; uncached tracks start from their
/// explicit raw keys. Empty queries are allowed; non-Bone/unknown/duplicate IDs
/// and stale remembered data fail under the same checks as Offset capture.
pub fn cycle_offset_selectable_keys(
    document: &Document,
    animation: usize,
    keys: &[AnimationKey],
    range: CycleOffsetRange,
    memory: &CycleOffsetMemory,
) -> Result<Vec<AnimationKey>, String> {
    Ok(
        classify_requested(document, animation, keys, range, memory)?
            .into_iter()
            .filter(|seed| seed.selectable)
            .map(|seed| seed.source)
            .collect(),
    )
}

struct SelectableSources {
    existing: BTreeSet<u32>,
    anchors: Option<BTreeSet<u32>>,
}

fn classify_requested(
    document: &Document,
    animation: usize,
    keys: &[AnimationKey],
    range: CycleOffsetRange,
    memory: &CycleOffsetMemory,
) -> Result<Vec<RequestedSeed>, String> {
    document.validate()?;
    range.duration()?;
    document
        .animations
        .get(animation)
        .ok_or("Unknown animation for Cycle Offset")?;
    // Context transitions cannot discard evidence of an intervening stale edit,
    // even when a requested source itself is a generated, ignored boundary.
    for (id, remembered) in &memory.tracks {
        validate_expected(document, *id, remembered)?;
    }
    let mut facts: BTreeMap<TrackId, SelectableSources> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut requested = Vec::with_capacity(keys.len());
    for source in keys {
        let AnimationKey::Bone {
            bone,
            property,
            time,
        } = source
        else {
            return Err("Cycle Offset currently supports whole Bone scalar tracks only; other selected timeline families remain unsupported".into());
        };
        if !time.is_finite() || *time < 0. {
            return Err("Cycle Offset source identities must have finite nonnegative times".into());
        }
        let id = TrackId {
            animation,
            bone: *bone,
            property: *property,
            start: time_id(range.start),
            end: time_id(range.end),
        };
        let fact = match facts.entry(id) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                let current = track(document, id)?;
                let anchors = memory
                    .tracks
                    .get(&id)
                    .map(|remembered| {
                        anchor_lineage(remembered).map(|anchors| anchors.into_keys().collect())
                    })
                    .transpose()?;
                entry.insert(SelectableSources {
                    existing: current.keys.iter().map(|key| time_id(key.time)).collect(),
                    anchors,
                })
            }
        };
        let identity = time_id(*time);
        if !fact.existing.contains(&identity) {
            return Err("Unknown exact Cycle Offset source key".into());
        }
        if !seen.insert((id, identity)) {
            return Err("Duplicate Cycle Offset source key".into());
        }
        let selectable = fact
            .anchors
            .as_ref()
            .is_none_or(|anchors| anchors.contains(&identity));
        requested.push(RequestedSeed {
            track: id,
            source: source.clone(),
            selectable,
        });
    }
    Ok(requested)
}

struct TrackLineage {
    sources: BTreeMap<u32, usize>,
    targets: Vec<f32>,
    source_keys: BTreeMap<u32, f32>,
    accepted_keys: BTreeSet<u32>,
}

fn anchor_times(memory: &TrackMemory) -> Result<Vec<f32>, String> {
    memory.original[..memory.original.len() - 1]
        .iter()
        .map(|key| {
            if memory.phase == 0. {
                Ok(key.time)
            } else {
                moved_time(key.time, memory.range, memory.phase)
            }
        })
        .collect()
}

fn anchor_lineage(memory: &TrackMemory) -> Result<BTreeMap<u32, usize>, String> {
    let mut lineage = BTreeMap::new();
    for (index, time) in anchor_times(memory)?.into_iter().enumerate() {
        if lineage.insert(time_id(time), index).is_some() {
            return Err("Cycle Offset original anchor identities collapse".into());
        }
        if memory.phase == 0. && time_id(time) == time_id(memory.range.start) {
            // Original closing keys were independently confirmed to follow the
            // same moved anchor as their matching original first key.
            lineage.insert(time_id(memory.range.end), index);
        }
    }
    Ok(lineage)
}

fn time_id(time: f32) -> u32 {
    if time == 0. { 0 } else { time.to_bits() }
}

fn track(document: &Document, id: TrackId) -> Result<&Track, String> {
    document
        .animations
        .get(id.animation)
        .ok_or("Unknown Cycle Offset animation")?
        .tracks
        .iter()
        .find(|track| track.bone == id.bone && track.property == id.property)
        .ok_or_else(|| "Unknown Cycle Offset source track".into())
}

fn same_keys(left: &[Keyframe], right: &[Keyframe]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(a, b)| {
            a.time.to_bits() == b.time.to_bits()
                && a.value.to_bits() == b.value.to_bits()
                && match (a.curve, b.curve) {
                    (Curve::Linear, Curve::Linear) | (Curve::Stepped, Curve::Stepped) => true,
                    (Curve::Bezier(a), Curve::Bezier(b))
                    | (Curve::BezierOffset(a), Curve::BezierOffset(b)) => {
                        a.map(f32::to_bits) == b.map(f32::to_bits)
                    }
                    _ => false,
                }
        })
}

fn validate_expected(document: &Document, id: TrackId, memory: &TrackMemory) -> Result<(), String> {
    let animation = document
        .animations
        .get(id.animation)
        .ok_or("Stale Cycle Offset animation")?;
    let bone = document
        .bones
        .get(id.bone)
        .ok_or("Stale Cycle Offset bone")?;
    if animation.name != memory.animation_name
        || bone.name != memory.bone_name
        || document.fps != memory.fps
        || !same_keys(&track(document, id)?.keys, &memory.expected)
    {
        return Err("Stale Cycle Offset source/context; explicitly forget its memory and recapture before editing".into());
    }
    Ok(())
}

fn advance_phase(phase: f32, delta: f32, duration: f32) -> Result<f32, String> {
    // Reduce the delta before adding: a huge whole-period move must not swallow
    // the already accepted fractional phase in a wide floating-point sum.
    let signed = f64::from(delta) % f64::from(duration);
    if signed == 0. {
        return Ok(phase);
    }
    let reduced = if signed < 0. {
        signed + f64::from(duration)
    } else {
        signed
    };
    let increment = reduced as f32;
    if increment >= duration || increment <= 0. {
        return Err("Cycle Offset phase cannot be represented inside these loop bounds".into());
    }
    let sum = phase + increment;
    let result = if sum.is_finite() {
        sum.rem_euclid(duration)
    } else {
        ((f64::from(phase) + f64::from(increment)) % f64::from(duration)) as f32
    };
    if !result.is_finite() || result < 0. || result >= duration {
        return Err("Cycle Offset phase exceeds finite loop coordinates".into());
    }
    Ok(if result == 0. { 0. } else { result })
}

fn moved_time(time: f32, range: CycleOffsetRange, phase: f32) -> Result<f32, String> {
    let duration = range.duration()?;
    let relative = time - range.start;
    let sum = relative + phase;
    let wrapped = if sum.is_finite() {
        sum.rem_euclid(duration)
    } else {
        ((f64::from(relative) + f64::from(phase)) % f64::from(duration)) as f32
    };
    let time = range.start + wrapped;
    if !time.is_finite()
        || time < range.start
        || time >= range.end
        || (wrapped != 0. && time == range.start)
    {
        return Err("Cycle Offset key cannot be represented inside its loop".into());
    }
    Ok(time)
}

fn offset_scalar(
    original: &[Keyframe],
    range: CycleOffsetRange,
    phase: f32,
) -> Result<Vec<Keyframe>, String> {
    let mut shifted = Vec::with_capacity(original.len() + 1);
    let mut identities = BTreeSet::new();
    let mut at_seam = None;
    for (index, source) in original[..original.len() - 1].iter().enumerate() {
        let time = moved_time(source.time, range, phase)?;
        if !identities.insert(time_id(time)) {
            return Err("Cycle Offset key times collapse".into());
        }
        if time_id(time) == time_id(range.start) {
            at_seam = Some(index);
        }
        shifted.push(Keyframe { time, ..*source });
    }
    let seam = if let Some(index) = at_seam {
        original[index].value
    } else {
        let cut = range.end - phase;
        if !cut.is_finite() || cut <= range.start || cut >= range.end {
            return Err("Cycle Offset seam cannot be represented inside its loop".into());
        }
        let index = original
            .windows(2)
            .position(|pair| pair[0].time < cut && cut < pair[1].time)
            .ok_or("Cycle Offset seam identity is ambiguous after f32 retiming")?;
        let (left, right) = (original[index], original[index + 1]);
        let progress = (cut - left.time) / (right.time - left.time);
        let value = curve_value(left.curve, progress, left.value, right.value);
        if !value.is_finite() {
            return Err("Cycle Offset seam value must be finite".into());
        }
        let (before, after) = crate::color_curve_split::split(
            left.curve,
            [left.time, cut, right.time],
            [left.value, value, right.value],
            value,
        )?;
        shifted[index].curve = before;
        shifted.push(Keyframe {
            time: range.start,
            value,
            curve: after,
        });
        value
    };
    shifted.push(Keyframe {
        time: range.end,
        value: seam,
        curve: Curve::Linear,
    });
    shifted.sort_by(|left, right| left.time.total_cmp(&right.time));
    if shifted.windows(2).any(|pair| pair[0].time >= pair[1].time) {
        return Err("Cycle Offset boundary keys collide".into());
    }
    Ok(shifted)
}
