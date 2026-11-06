# Additional binding and single-bone Direct weights

Status: approved bounded core design; engine implementation is authorized. Native
v4 remains unchanged. The existing initial single-bone Bind and Remove-all commands
remain separate operations. This approval does not establish UI parity or resolve
the observation gates listed below.

## Public evidence

The current [Weights guide](https://esotericsoftware.com/spine-weights#Direct)
describes selecting vertices and bones, then setting an exact weight in Direct
mode. Its [adjustment rules](https://esotericsoftware.com/spine-weights#Adjusting-weights)
redistribute the remainder so weights total 100%; a sole 100% influence cannot be
reduced when there are no recipients. These statements do not define ambiguous
zero-recipient or sparse-binding cases.

In [staff explanation of proportional weights](https://esotericsoftware.com/forum/d/16165-lock-a-bones-weights-in-specific-percent/5),
Erika explains that setting the chosen percentage proportionally resizes the
others. The historical discussion of missing locks is not evidence that current
Spine lacks locks; the current guide documents them. Locks are outside this slice.

In [staff explanation of newly added bindings](https://esotericsoftware.com/forum/d/11318-weight-smoothing-not-working-properly/8),
Nate states that an initial single-bone bind gives that bone 100% on every vertex;
subsequently added bones receive 0%. This supports the from-scratch workflow below.
It does not establish how to fill missing entries for a bone already present
elsewhere in an imported sparse mesh.

These are public behavioral references, retrieved through Agent Reach's Exa search
route and official-page retrieval. The post-specific URLs above identify the staff
answers; when the reader cannot resolve a post suffix, the same answers are on the
corresponding thread pages. No proprietary implementation is a reference. Exact
4.3 UI placement, gesture behavior and remaining edge cases need separate native
observation and acceptance.

## Approved serializable commands

Extend the existing strict `Command` enum with:

```rust
AddMeshBoneBinding {
    attachment: AttachmentRef,
    bone: usize,
    pose: MeshBindPose,
}
SetMeshBoneWeight {
    attachment: AttachmentRef,
    vertex: usize,
    bone: usize,
    weight: f32,
    pose: MeshBindPose,
}
```

The JSON tags are `addMeshBoneBinding` and `setMeshBoneWeight`. Existing strict
`MeshBindPose { locals: Vec<Transform> }` supplies an explicit finite local pose
with exactly one transform per document bone. Unknown command, pose and transform
fields remain errors. Both commands address an exact skin/slot/placeholder, without
implicit fallback resolution or a cached bone target.

Approved thin Session wrappers:

```rust
add_mesh_bone_binding(attachment, bone, target: EditTarget) -> Result<(), String>
set_mesh_bone_weight(attachment, vertex, bone, weight, target: EditTarget)
    -> Result<(), String>
```

They reuse the existing pose-snapshot path: Setup uses document setup locals;
Animate uses the actual sampled and pending Session locals at the supplied finite,
nonnegative time, including fractional times and times beyond the current end.
They neither snap the bind pose to frames nor key/discard pending edits.

The Direct pose field is an approved safety boundary, not a documented Spine data
format requirement. Direct's weight arithmetic needs no matrices. The captured
pose lets the pure command reject a newly nonfinite selected world vertex under
our f32 evaluator. It is not used to rebind or compensate positions. This field
and the exact API names are approved for this limited engine implementation.

## Add a genuinely new bone at zero weight

This command requires a weighted mesh and an in-range bone absent from every
influence list in that mesh, including zero-weight entries. Finding it anywhere
is an error, not a request to patch a sparse binding or replace existing entries.
Initial binding of a slot-local mesh still uses `BindMeshToBone`.

At the captured pose, let `W[b]` be the bone's current world matrix and let `P[v]`
be each mesh vertex evaluated by the existing weighted evaluator. Build:

```text
new_position[v] = inverse(W[new_bone]) * P[v]
new_entry[v] = { bone: new_bone, position: new_position[v], weight: 0 }
```

The inverse-position choice is our approved mathematical policy for initializing
a usable zero binding. The staff answer establishes the new zero weight, not an
observation of how Spine stores that new entry's bind position. Likewise the
rejection rules below are approved limits, not claims that Spine rejects every
same input.

Append exactly one new entry to each logical vertex. Preserve all existing entries
bit for bit, including positions, positive/non-unit weights, zeros, repeated bones
and order. Do not sort, merge, prune, normalize or update old bindings. UVs,
triangles, hull, images, slot references, bones and animation keys are unchanged.
The per-vertex vector represents the binding; no separate native binding list or
format migration is introduced.

Reusing the existing checked inverse in `mesh_binding.rs` is mandatory. It already
uses f64 intermediates, checks finite local results and reconstructs their f32
world positions. The existing absolute per-coordinate limit is 0.001, with no
relative or large-coordinate exception. Do not copy that algorithm or introduce
a second tolerance. Refactor only a small shared helper if access requires it.

Only the new bone's inverse is required. The slot matrix and previously bound
bones are evaluated forward; they need no inverse. Reject a singular new bone,
nonfinite required transform/world point, unrepresentable local position or
precision-loss result atomically. Small nonzero scales remain valid. An unrelated
zero scale must not reject the operation.

The zero entry must still pass the checked forward evaluation. The current
evaluator transforms each bind position before multiplication: `0 * infinity` or
`0 * NaN` can contaminate the result. A zero weight is not permission to skip the
new-position validity guard. Existing invalid current world geometry is also an
error, not something Add silently repairs.

At this captured pose, appending a finite zero contribution preserves the current
world geometry, including when old weights have a non-unit total. Acceptance must
compare the full evaluated mesh before and after and verify every old influence
is unchanged. This guarantee concerns the captured pose, not arbitrary future
transforms outside the f32 evaluator's representable range.

## Direct on one logical vertex and one existing bone

This command requires a weighted mesh, a valid logical vertex and exactly one
entry for the chosen bone in that vertex. For this first slice, reject the edit if
any bone occurs more than once in the selected vertex, even if a repeated entry
has zero weight. A bone-based UI cannot disambiguate different bind positions by
silently coalescing them. Repeated entries in other vertices remain untouched and
do not block this independent vertex edit.

The requested `weight` must be finite and in `[0, 1]`; never clamp a malformed
serialized request. Let `j` identify the chosen entry, `t` be that requested f32
value, and `O` be the f64 sum of the other strictly positive weights. No epsilon
decides whether a weight is positive.

For `O > 0`:

```text
weight'[j] = t
weight'[i] = f32(f64(weight[i]) * (1 - f64(t)) / O), for other positive i
weight'[i] = weight[i], for other zero i
```

This proportionally assigns the remaining share while keeping the chosen f32
value. It does not divide by the total including the old chosen weight. Keep bind
positions and entry order unchanged. A positive entry becoming zero remains an
entry; no pruning or implicit unbinding occurs. Existing zero entries, including
their sign bits, are not rewritten merely to recompute a total.

For `O == 0`, allow `t == 1` and retain all other zero entries; reject `t < 1`.
Do not choose an arbitrary zero recipient or divide the remainder equally. That
case remains an observation gate. The artist can instead add weight to another
existing zero entry first, which has positive recipients in the old chosen bone.

After f32 conversion, validate the candidate with the existing native rules and
check the selected vertex's forward world evaluation is finite at the captured
pose. Direct uses no inverse, so a singular transform alone is not an error when
the evaluated point remains finite. Do not apply the 0.001 shape residual test to
Direct: changing weights is allowed to deform the image. Rebinding to suppress
that deformation would implement an unrequested Update operation.

The equation totals one in real arithmetic; stored f32 values can retain a small
rounding error. Do not force bit-exact unity by assigning a residual to a chosen
bone or by a second normalization pass. Tests use known weight vectors, preserved
ratios and the resulting total's rounding error. Underflow may turn a rescaled
positive value into zero, but its entry remains present.

Explicit Direct on a vertex with a non-unit imported total intentionally rescales
that vertex's other positive weights. Import, display, Add-binding-zero and
unselected vertices do not normalize anything. An explicit Set request may perform
this normalization even when the chosen value was already the requested value.
An equal resulting Document is a history no-op under the existing Session equality
rules; no new bitwise history comparison is proposed.

## Required from-scratch workflow

1. Initial `BindMeshToBone` binds a slot-local mesh to A: each vertex has A=1.
2. `AddMeshBoneBinding` adds genuinely new B: each has A=1, B=0; geometry is unchanged.
3. On one selected logical vertex, `SetMeshBoneWeight` sets B=0.3. A's positive
   weight becomes 0.7 and B becomes 0.3; other vertices still contain A=1, B=0.
4. Pose A or B to test the result; the stored weights remain global mesh setup
   data. Changing weights creates no animation keys.

For a normalized mesh at the pose used to add B, both bones initially agree on the
vertex position within the shared binding tolerance. This is a mathematical
consequence of the approved bind-position calculation, not evidence for every
Spine edge case. Non-unit imported totals and already different old bind positions
can move when Direct explicitly normalizes the selected vertex.

## History, selection and implementation boundaries

- Apply only to a validated candidate and publish once. Failure preserves the
  document, saved baseline, pending poses, undo and redo, including through Batch.
- Session dispatch keeps current `PoseEdits`. A weight slider uses the existing
  fixed-base transaction preview and one captured pose throughout the gesture.
  Preview failure retains the previous valid preview; Cancel restores the original
  raw payload and redo branch. Commit is one undo step.
- Unselected vertices and unrelated attachments/skins are unchanged. Only the
  selected vertex's chosen weight and proportionally rescaled positive recipients
  are Direct edit targets. Pose snapshots are command inputs, never native fields.
- Logical vertex indices stay stable because these operations do not change
  topology. Reconcile transient weight selection and gestures after context/history
  changes; do not redirect a missing attachment or bone to another entity.
- Add can preserve imported repeated entries while appending a new distinct bone.
  Direct rejects an ambiguous selected vertex and leaves the file intact. A future
  explicit cleanup command would require a separate loss/preservation contract.

Reject malformed/missing attachment references, region or slot-local input for
these weighted operations, unknown bones/vertices, invalid poses and weights, a
bone already present anywhere during Add, missing selected-vertex bone entries,
and ambiguous repeated selected-vertex entries. Validate before any accepted
no-op. No command implicitly activates skins, resolves a different fallback,
changes bind positions during Direct or changes the native v4 schema.

## Acceptance and observation gates

Required implementation tests:

- A=1 -> Add B=0 -> Direct B=0.3 yields A=0.7/B=0.3 on exactly one vertex; every
  other vertex and old bind position remains unchanged.
- Add new zero leaves evaluated geometry unchanged under Setup, exact fractional
  Animate and pending poses, including reflected/nonuniform parents and raw
  non-unit/zero/repeated old entries. Undo restores the exact original payload.
- Direct A/B/C=0.2/0.3/0.5 -> C=0.6 yields A=0.16/B=0.24/C=0.6. Cover requested
  endpoints 0 and 1, positive subnormals and f32 rounding without residual repair.
- Only the new bone inverse is needed by Add. Keep singular-new rejection, valid
  small-scale binding and exactly recoverable large poses; reject precision loss,
  overflowing forward zero contributions and nonfinite candidate Direct geometry.
- Preserve all unselected raw weights, positions, order and other skins. Reject
  already-present Add bones, sparse selected-vertex targets, duplicate ambiguity,
  zero-recipient reductions, bad payloads and partially successful batches atomically.
- Repeated previews use the fixed base rather than cumulative rescaling. Cancel,
  no-op, failed requests, Undo/Redo and saved state preserve pending poses and redo.
- Roundtrip new zero bindings and edited weights through native v4 and the adapter.
  CLI, external runtime and installed-editor/native checks establish separate facts.

Sparse existing-bone filling, zero-recipient redistribution, locks, simultaneous
multi-bone Direct, multi-vertex UI behavior, per-bone Remove and Update bindings are
observation gates. Auto, brush painting, Smooth, Prune, Weld, Swap and deform keys
remain outside this bounded slice. Do not infer an equal-share fallback, silently
normalize imports or advertise those tools from the two approved commands.
