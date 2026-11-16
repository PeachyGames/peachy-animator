//! Direct selection semantics. A held gesture freezes recipient ratios while
//! its per-vertex target values advance incrementally through clamp boundaries.
use crate::{Affine, MeshBindPose};
use peachy_document::{Attachment, AttachmentRef, BoneInfluence, Document, MeshVertices};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase", deny_unknown_fields)]
pub enum MeshWeightEdit {
    Absolute { weight: f32 },
    Relative { deltas: Vec<f32> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshWeightSelection {
    pub attachment: AttachmentRef,
    pub vertices: Vec<usize>,
    pub bone: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VertexWeight {
    pub vertex: usize,
    pub weight: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MeshWeightValue {
    Uniform(f32),
    Mixed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshWeightQuery {
    pub selection: MeshWeightSelection,
    pub weights: Vec<VertexWeight>,
    pub value: MeshWeightValue,
}

fn canonical_selection(
    doc: &Document,
    selection: &MeshWeightSelection,
) -> Result<MeshWeightSelection, String> {
    if selection.bone >= doc.bones.len() {
        return Err("Unknown weight bone".into());
    }
    let (_, mesh) = crate::mesh_binding::mesh_target(doc, &selection.attachment)?;
    if !mesh.is_weighted() {
        return Err("Direct weights require a weighted mesh".into());
    }
    let mut result = selection.clone();
    result.vertices.sort_unstable();
    if result.vertices.is_empty() {
        return Err("Direct weights require at least one selected vertex".into());
    }
    if result.vertices.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("Duplicate selected weight vertex".into());
    }
    if result
        .vertices
        .last()
        .is_some_and(|&vertex| vertex >= mesh.vertex_count())
    {
        return Err("Unknown mesh vertex".into());
    }
    Ok(result)
}

fn query(selection: MeshWeightSelection, weights: Vec<VertexWeight>) -> MeshWeightQuery {
    let first = weights[0].weight;
    let value = if weights.iter().all(|value| value.weight == first) {
        MeshWeightValue::Uniform(first)
    } else {
        MeshWeightValue::Mixed
    };
    MeshWeightQuery {
        selection,
        weights,
        value,
    }
}

/// Raw selected values, without averaging, display rounding or normalization.
/// Documents are validated by Session; selected data is checked independently.
pub fn mesh_bone_weights(
    doc: &Document,
    selection: &MeshWeightSelection,
) -> Result<MeshWeightQuery, String> {
    let selection = canonical_selection(doc, selection)?;
    let (_, mesh) = crate::mesh_binding::mesh_target(doc, &selection.attachment)?;
    let MeshVertices::Weighted { influences } = &mesh.vertices else {
        unreachable!()
    };
    if !influences
        .iter()
        .flatten()
        .any(|entry| entry.bone == selection.bone)
    {
        return Err("Bone is not bound to this mesh; Bind it before adjusting weights".into());
    }
    let mut weights = vec![];
    for &vertex in &selection.vertices {
        let entries = &influences[vertex];
        if entries.iter().any(|entry| {
            entry.bone >= doc.bones.len()
                || !entry.weight.is_finite()
                || !(0.0..=1.0).contains(&entry.weight)
                || entry.position.iter().any(|value| !value.is_finite())
        }) {
            return Err(
                "Selected weight bindings must contain known bones and finite native values".into(),
            );
        }
        let mut bones = std::collections::BTreeSet::new();
        if entries.iter().any(|entry| !bones.insert(entry.bone)) {
            return Err(
                "Direct weight editing is ambiguous for repeated bones on this vertex".into(),
            );
        }
        weights.push(VertexWeight {
            vertex,
            weight: entries
                .iter()
                .find(|entry| entry.bone == selection.bone)
                .map_or(0., |entry| entry.weight),
        });
    }
    Ok(query(selection, weights))
}

#[derive(Clone, Debug)]
struct WeightBasis {
    source: Vec<BoneInfluence>,
    working: Vec<BoneInfluence>,
    missing: bool,
    recipients: f64,
}

#[derive(Debug)]
struct BindingOrder {
    ranks: Result<BTreeMap<usize, usize>, String>,
    // Selected entries are already checked against the frozen/preview payload.
    // Retain only the additional source dependency used to place a new entry.
    unselected_bones: Vec<(usize, Vec<usize>)>,
}

/// Independently derive a linear extension of the mesh's existing sequences.
/// Unconstrained bones retain first-appearance priority; bone indices are never
/// used to sort imported entries. This is only needed for sparse creation.
fn binding_ranks(influences: &[Vec<BoneInfluence>]) -> Result<BTreeMap<usize, usize>, String> {
    let mut first = BTreeMap::new();
    for vertex in influences {
        let mut seen = BTreeSet::new();
        for entry in vertex {
            if !seen.insert(entry.bone) {
                return Err(
                    "Cannot create a missing binding: mesh influence ordering contains repeated bones"
                        .into(),
                );
            }
            let index = first.len();
            first.entry(entry.bone).or_insert(index);
        }
    }
    let mut edges = vec![BTreeSet::new(); first.len()];
    let mut incoming = vec![0; first.len()];
    for vertex in influences {
        for pair in vertex.windows(2) {
            let before = first[&pair[0].bone];
            let after = first[&pair[1].bone];
            if edges[before].insert(after) {
                incoming[after] += 1;
            }
        }
    }
    let mut ready: BTreeSet<_> = incoming
        .iter()
        .enumerate()
        .filter_map(|(index, &count)| (count == 0).then_some(index))
        .collect();
    let mut ordered = vec![];
    while let Some(index) = ready.pop_first() {
        ordered.push(index);
        for &after in &edges[index] {
            incoming[after] -= 1;
            if incoming[after] == 0 {
                ready.insert(after);
            }
        }
    }
    if ordered.len() != first.len() {
        return Err(
            "Cannot create a missing binding: mesh influence ordering is contradictory".into(),
        );
    }
    let mut ranks = vec![0; first.len()];
    for (rank, index) in ordered.into_iter().enumerate() {
        ranks[index] = rank;
    }
    Ok(first
        .into_iter()
        .map(|(bone, index)| (bone, ranks[index]))
        .collect())
}

#[derive(Clone, Debug)]
pub struct MeshWeightGesture {
    selection: MeshWeightSelection,
    pose: MeshBindPose,
    world: Vec<Affine>,
    parents: Vec<Option<usize>>,
    legacy_inherits: Option<Vec<peachy_document::Inherit>>,
    vertex_count: usize,
    basis: Vec<WeightBasis>,
    ordering: Option<Arc<BindingOrder>>,
    targets: Vec<f32>,
    prepared: Vec<Vec<BoneInfluence>>,
}

fn same_entries(left: &[BoneInfluence], right: &[BoneInfluence]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(a, b)| {
            a.bone == b.bone
                && a.position.map(f32::to_bits) == b.position.map(f32::to_bits)
                && a.weight.to_bits() == b.weight.to_bits()
        })
}

impl MeshWeightGesture {
    pub fn new(
        doc: &Document,
        selection: MeshWeightSelection,
        pose: MeshBindPose,
    ) -> Result<Self, String> {
        let query = mesh_bone_weights(doc, &selection)?;
        let world = crate::mesh_binding::bind_world(doc, &pose)?;
        let (_, mesh) = crate::mesh_binding::mesh_target(doc, &query.selection.attachment)?;
        let MeshVertices::Weighted { influences } = &mesh.vertices else {
            unreachable!()
        };
        let ordering = query
            .selection
            .vertices
            .iter()
            .any(|&vertex| {
                !influences[vertex]
                    .iter()
                    .any(|entry| entry.bone == query.selection.bone)
            })
            .then(|| {
                Arc::new(BindingOrder {
                    ranks: binding_ranks(influences),
                    unselected_bones: influences
                        .iter()
                        .enumerate()
                        .filter(|(vertex, _)| {
                            query.selection.vertices.binary_search(vertex).is_err()
                        })
                        .map(|(vertex, entries)| {
                            (vertex, entries.iter().map(|entry| entry.bone).collect())
                        })
                        .collect(),
                })
            });
        let mut basis = vec![];
        for &vertex in &query.selection.vertices {
            let source = influences[vertex].clone();
            let mut working = source.clone();
            let missing = !source
                .iter()
                .any(|entry| entry.bone == query.selection.bone);
            if missing {
                // Observed native Direct semantics: an absent, globally bound
                // influence starts at the bone-local origin, not an inverse bind.
                // Cache placement once; malformed global ordering is rejected
                // only if a later positive edit actually creates the entry.
                let at = ordering
                    .as_ref()
                    .and_then(|ordering| ordering.ranks.as_ref().ok())
                    .and_then(|ranks| {
                        working
                            .iter()
                            .position(|entry| ranks[&entry.bone] > ranks[&query.selection.bone])
                    })
                    .unwrap_or(working.len());
                working.insert(
                    at,
                    BoneInfluence {
                        bone: query.selection.bone,
                        position: [0., 0.],
                        weight: 0.,
                    },
                );
            }
            let (_, recipients) =
                crate::mesh_binding::weight_entry(&working, query.selection.bone)?;
            basis.push(WeightBasis {
                source,
                working,
                missing,
                recipients,
            });
        }
        let prepared = basis.iter().map(|basis| basis.source.clone()).collect();
        Ok(Self {
            selection: query.selection,
            parents: doc.bones.iter().map(|bone| bone.parent).collect(),
            legacy_inherits: pose
                .inherits
                .is_none()
                .then(|| doc.bones.iter().map(|bone| bone.inherit).collect()),
            pose,
            world,
            vertex_count: mesh.vertex_count(),
            basis,
            ordering,
            targets: query.weights.iter().map(|entry| entry.weight).collect(),
            prepared,
        })
    }
    pub fn selection(&self) -> &MeshWeightSelection {
        &self.selection
    }
    pub fn query(&self) -> MeshWeightQuery {
        query(
            self.selection.clone(),
            self.selection
                .vertices
                .iter()
                .zip(&self.targets)
                .map(|(&vertex, &weight)| VertexWeight { vertex, weight })
                .collect(),
        )
    }
    fn prepare(&mut self, index: usize, weight: f32) -> Result<(), String> {
        if self.basis[index].missing && weight == 0. {
            self.targets[index] = 0.;
            self.prepared[index] = self.basis[index].source.clone();
            return Ok(());
        }
        if self.basis[index].missing {
            self.ordering
                .as_ref()
                .expect("Missing bindings capture mesh ordering")
                .ranks
                .as_ref()
                .map_err(Clone::clone)?;
        }
        if self.basis[index].recipients == 0. && weight < 1. {
            // Native skips a sole-positive vertex, while other selected vertices
            // may still change. The legacy single raw setter continues to fail.
            return Ok(());
        }
        let next = crate::mesh_binding::direct_vertex_weights(
            &self.basis[index].working,
            self.selection.bone,
            weight,
        )?;
        if crate::mesh::weighted_world_vertex(&next, &self.world)
            .iter()
            .any(|value| !value.is_finite())
        {
            return Err("Direct weight would produce nonfinite world vertices at this pose".into());
        }
        self.targets[index] = weight;
        self.prepared[index] = next;
        Ok(())
    }
    /// Candidate only: retain this helper until the Session preview succeeds.
    pub fn absolute(&self, weight: f32) -> Result<Self, String> {
        if !weight.is_finite() || !(0.0..=1.0).contains(&weight) {
            return Err("Direct weight must be finite and in 0–1".into());
        }
        let mut candidate = self.clone();
        for index in 0..candidate.targets.len() {
            candidate.prepare(index, weight)?;
        }
        Ok(candidate)
    }
    /// Advance clamped targets while preserving the original recipient basis.
    pub fn relative(&self, delta: f32) -> Result<Self, String> {
        if !delta.is_finite() {
            return Err("Relative weight delta must be finite".into());
        }
        let mut candidate = self.clone();
        if delta == 0. {
            return Ok(candidate);
        }
        for index in 0..candidate.targets.len() {
            let next =
                (f64::from(candidate.targets[index]) + f64::from(delta)).clamp(0., 1.) as f32;
            if next != candidate.targets[index] {
                candidate.prepare(index, next)?;
            }
        }
        Ok(candidate)
    }
    fn matches(&self, doc: &Document, original: bool) -> bool {
        let Ok((_, mesh)) = crate::mesh_binding::mesh_target(doc, &self.selection.attachment)
        else {
            return false;
        };
        let MeshVertices::Weighted { influences } = &mesh.vertices else {
            return false;
        };
        doc.bones.len() == self.pose.locals.len()
            && doc
                .bones
                .iter()
                .map(|bone| bone.parent)
                .eq(self.parents.iter().copied())
            && self.legacy_inherits.as_ref().is_none_or(|modes| {
                doc.bones
                    .iter()
                    .map(|bone| bone.inherit)
                    .eq(modes.iter().copied())
            })
            && mesh.vertex_count() == self.vertex_count
            && self.ordering.as_ref().is_none_or(|ordering| {
                ordering.unselected_bones.iter().all(|(vertex, bones)| {
                    influences.get(*vertex).is_some_and(|entries| {
                        entries
                            .iter()
                            .map(|entry| entry.bone)
                            .eq(bones.iter().copied())
                    })
                })
            })
            && self
                .selection
                .vertices
                .iter()
                .enumerate()
                .all(|(index, &vertex)| {
                    influences.get(vertex).is_some_and(|actual| {
                        same_entries(
                            actual,
                            if original {
                                &self.basis[index].source
                            } else {
                                &self.prepared[index]
                            },
                        )
                    })
                })
    }
    pub fn matches_document(&self, doc: &Document) -> bool {
        self.matches(doc, false)
    }
    pub(crate) fn matches_baseline(&self, doc: &Document) -> bool {
        self.matches(doc, true)
    }
    pub(crate) fn apply(&self, doc: &mut Document) -> Result<(), String> {
        if !self.matches_baseline(doc) {
            return Err("Weight gesture source changed".into());
        }
        let Some(Attachment::Mesh(mesh)) = doc.attachment_mut(&self.selection.attachment) else {
            unreachable!()
        };
        let MeshVertices::Weighted { influences } = &mut mesh.vertices else {
            unreachable!()
        };
        for (index, &vertex) in self.selection.vertices.iter().enumerate() {
            influences[vertex] = self.prepared[index].clone();
        }
        Ok(())
    }
}

pub(crate) fn edit_weights(
    doc: &mut Document,
    selection: MeshWeightSelection,
    edit: MeshWeightEdit,
    pose: MeshBindPose,
) -> Result<(), String> {
    let mut gesture = MeshWeightGesture::new(doc, selection, pose)?;
    match edit {
        MeshWeightEdit::Absolute { weight } => gesture = gesture.absolute(weight)?,
        MeshWeightEdit::Relative { deltas } => {
            if deltas.is_empty() {
                return Err("Relative weight edit needs at least one delta".into());
            }
            for delta in deltas {
                gesture = gesture.relative(delta)?;
            }
        }
    }
    gesture.apply(doc)
}
