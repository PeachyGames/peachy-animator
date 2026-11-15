# Multi-vertex Direct weights — November 15, 2026

**Verified:** shared engine/MCP, installed-reference calibration, actual native
Peachy authoring, fresh public-project reopen and installed-editor roundtrip.
See the [scalar evidence](../evidence/weight-multivertex.json),
[native capture gallery](../evidence/weight-multivertex.html) and
[original public lab](../../examples/peach-bot/multi-weights-bot.peachy).

This extends the earlier [single-vertex Direct slice](weight-direct.md) to an
explicit set of vertices and one bound bone. Native format **v22 is unchanged**.
The existing strict raw setter remains available with its previous behavior.

## Reference and measured scope

The current official [Weights guide](https://esotericsoftware.com/spine-weights)
was retrieved through agent-reach on November 15. It describes Direct numeric
and viewport editing, vertex selection, proportional remainder and asterisks
for differing selected weights. It does not specify drag-start hit testing,
Shift/Alt arithmetic or a universal drag threshold.

Owned installed Spine **4.3.26** windows used an original six-vertex mesh, three
influence bones and one committed original geometric PNG. A second fresh input
actually omits zero influences and includes a posed animation. No artist asset,
vendor project or proprietary implementation was inspected. Private inputs,
captures, outputs and the independent **14-case** record are under
`.local/training-spine-20261114/weights-multivertex/`.

Verified observations include:

- A selected vertex shows its raw per-bone percentages. Different selected
  values show bone-list asterisks and a **blank Weight field**, without an
  invented average. Select All and box selection are captured separately.
- With A selected, numeric `50` assigns A=`0.5` to each eligible selected vertex.
  Other positive weights keep their relative proportions. All six selected
  vertices can participate while the sole-positive A=`1` vertex stays unchanged.
- A valid sole-positive `100→50` attempt leaves the exported weight at `1`.
  After a separately committed pair edit to `100`, a new `50` attempt likewise
  leaves both at `1`. Export equality alone does not establish native dirty,
  redo or history no-op behavior.
- A held drag can reach `100`, then reverse to A=`0.85714` on both selected
  vertices while restoring their original, different B:C proportions. One
  native Undo returns the exact baseline export. This differs from starting a
  fresh edit after committing `100`.
- Existing bind positions and unselected vertices stay unchanged. Observed
  native weight serialization rounds some results to five decimals; that is
  not a reason to quantize our raw f32 values on display or export.

Earlier trials with the intended bone unselected are explicitly excluded from
the evidence. Native input delivery, active bone and resulting fields were
verified before treating the replacement cases as valid.

## Absolute and cumulative relative edits

The new shared command is:

```rust
EditMeshBoneWeights { attachment, vertices, bone, edit, pose }
```

Its wire tag is `editMeshBoneWeights`. `edit` is either
`{"mode":"absolute","weight":0.5}` or
`{"mode":"relative","deltas":[2.0,-0.14285715]}`. Absolute weight is finite
and within `0..1`; relative steps are a nonempty list of finite signed values.
Both forms address one exact attachment, distinct valid vertex indices and one
bone already bound somewhere in that mesh. Invalid selections, ambiguous
repeated-bone entries and invalid poses fail atomically.

For an absolute edit, each eligible vertex gets the requested selected-bone
weight; the remaining mass is distributed proportionally among its other
positive weights. Existing zero entries, bind positions and ordering remain.
When there are no positive recipients and the requested weight is below `1`,
the semantic operation skips that vertex while continuing eligible vertices.
The legacy `SetMeshBoneWeight` / `setMeshBoneWeight` still rejects that case and
does not acquire implicit sparse-binding behavior.

A relative gesture freezes the original recipient weights/positions but
advances each selected target incrementally, clamping after **each** step.
Thus `[2, -1/7]` saturates two different A weights at `1` and then lowers both
to approximately `6/7`, using each vertex's original B:C ratio. Applying one
net delta to the original A values would produce different behavior. A fresh
command starts a fresh basis; it cannot recover positive recipients zeroed by
an already committed `100` edit.

The desktop derives signed relative steps from vertical pointer movement at
**1/350 normalized weight per coordinate unit**, after activation. The measured
axis/event paths establish an activation gate beyond `10` units: the first
crossing event establishes the origin and contributes no weight delta. Fine
2-unit events over 40 units yield `28/350`; coarse events over 80 yield
`68/350`. A horizontal activation followed by 40 vertical units yields
`40/350`. Fine/coarse here describes event spacing, not a keyboard modifier.
The native calibration does not establish exact diagonal/Euclidean threshold,
cross-display scaling or Shift/Alt rules; those remain bounded limits.

Numeric/slider entry uses absolute targets. Pointer previews reuse one frozen
Session transaction, while the engine retains cumulative clamp state and
recipient ratios. Release commits one Undo entry; cancellation restores the
baseline and redo. Text focus, modal/menu/context changes, Save/history and
held-pointer cancellation use the shared transaction lifecycle. Actual native
acceptance checks the concrete cases below.

## Sparse influences and layout guards

The raw query reports `0` when the chosen globally bound bone is absent at a
selected vertex. It does not insert data. A positive semantic edit can add that
missing influence at **bone-local `[0,0]`**. No new entry is materialized for a
missing influence that remains at zero.

This origin rule is directly observed in fresh sparse inputs in **Setup and
Animate**. In contrast, the original input explicitly supplies zero-weight A
at `[27,172]`; assigning it positive weight retains `[27,172]`, even though an
intermediate native JSON export omitted the zero entry. These are separate
cases. Sparse insertion is not an inverse bind or a world-shape preservation
operation; the visible vertex can move.

The new entry follows a consistent mesh binding order, preserving every old
entry's relative order and position. An independent topological extension uses
existing per-vertex precedence, with first-appearance ties. Positive sparse
creation rejects repeated or contradictory ordering; queries, zero/no-op input
and existing-binding edits remain available. The gesture caches this ordering
and checks its unselected bone-sequence dependency before previewing.

Installed-editor controls established why this matters: appending the new A
after B caused an import failure; changing only B,A to A,B resolved it and all
52 returned poses passed at the unchanged 0.001 tolerance. A consistently
nonnumeric C,A,B input also imports, so skeleton-index sorting is not required
for the measured fixtures. Existing raw arrays are never sorted. Editor export
rejects cross-vertex precedence cycles before writing; Runtime export retains
raw arrays. First-occurrence analysis keeps legacy repeated/zero data unchanged.
Adding a globally unbound bone still requires Bind.

An added influence changes weighted Deform layout. Sparse insertion is rejected
if it would invalidate stored Deform tracks or pending Deform edits. The shared
command/Session guards preserve document, pending pose and history on failure;
there is no implicit offset padding, Deform remapping or rebinding. Ordinary
weight changes that keep the layout do not add this restriction.

## Pose capture and animated inheritance

`MeshBindPose` captures **applied locals and their inheritance modes together**.
For MCP, take `appliedLocals` and `inherits` from the same `evaluate_pose` result;
do not substitute authored `locals` when constraints or inheritance have already
changed the applied frame. Binding reconstructs checked FK without solving
constraints a second time.

`pose.inherits` is an optional array for legacy clients. Omission retains Setup
inheritance modes; a supplied array must contain one known mode per document
bone. Explicit `null`, unknown modes, wrong lengths and unknown fields are
rejected. This command-payload extension does not change persistent v22 data.
The actual stdio inheritance regression captures keyed modes and verifies
preserved geometry across the existing binding workflow.

## Actual native Peachy acceptance

Owned native windows verified typed `50` and history against resulting document
data. Held relative editing keeps a mixed field blank, clamps at `1`, reverses
to approximately `6/7` with the original recipient ratios, and commits one Undo.
Escape restores the baseline and blocks a canceled held pointer until release;
marquee/Cmd-add cancellation has the same held-input guard. All-six absolute
editing skips the sole-positive vertex, and sparse positive assignment inserts
the ordered `[0,0]` influence. A pending `13.5`-degree Animate pose is excluded
from Save.

The corrected native and MCP outputs are byte-identical to the public lab,
SHA256 `87214e61dc6410d8f9f0306deacd7b19b2a658a9353bd2b15b9f2be606540807`.
A fresh open of that public file has no history, shows the expected mixed blank
field, and retains selection when Cmd is released before mouse-up. Weights and
Curves are both visible, with Weights on the left and a full-width Dopesheet.
At **1120×740**, the viewport remains **320** units wide and the checked controls
are not clipped. Actual playback advances and loops (`0.5052248→0.0475444`), then
stops at zero. The saved source remains unchanged and all owned GUI instances
are closed. These are native input/data checks; no browser-render claim is made.

## MCP query and verification

MCP now exposes **52 commands / 12 tools**. The new read-only tool is
`get_mesh_weights`, with flat arguments
`{expected_revision, skin, slot, placeholder, vertices, bone}`. It returns the
resolved attachment, sorted vertices, raw `{vertex, weight}` values and either
`{kind:"mixed"}` or `{kind:"uniform", weight}`. It never averages, rounds,
normalizes, mutates history or advances revision. Invalid/stale requests fail
without changing the session.

The command schema and engine decoder are strict at command, edit and pose
levels. Real stdio tests cover mixed/uniform readback, invalid/stale read-only
queries, all-six absolute editing, cumulative clamp/reversal, exact Undo/Redo,
committed `100→50`, invalid atomic batches with redo alive and Save/reopen.
The coordinator reports **11 MCP schema/unit and 15 protocol tests passing**,
including **13 actual stdio** cases. Focused engine weight tests total **10**;
adapter order and legacy weighted groups each have **4** checks. Desktop weight
checks total **39**. These filters overlap with broader verification.

A separate actual MCP recipe starts from `create_project` and only the original
PNG. It authors four bones, two weighted meshes (explicit-zero and sparse) and
an `orbit` clip through **90 requests**, then saves/reopens and exports Runtime
4.3 JSON/bundle. It checks raw queries, all-six skip-sole behavior, recipient
ratios, sparse origin and invalid atomic selections; **51** MCP poses are
retained. The finished native fixture is not a recipe input; the PNG hash stays
unchanged.

The external, separately licensed Spine 4.3 oracle passes the MCP-authored
`orbit` clip/default skin at **52** samples:

| Comparison | Values | Maximum error |
| --- | --- | --- |
| Bone world transforms | 1,248 | `7.967635085037728e-6` |
| Weighted mesh coordinates | 1,248 | `3.0517578125e-5` |
| UV comparisons | 2,496 | `0` |

These are data/geometry checks, not raster or native Peachy input acceptance.
The final recipe was recreated after the ordering fix under `mcp-ordered/`.
Private `mcp-ordered/mcp-report.json`, `mcp-ordered/mcp-runtime.json`, requests
and the independent observation record retain exact hashes and limits. Its
saved project SHA256 is
`87214e61dc6410d8f9f0306deacd7b19b2a658a9353bd2b15b9f2be606540807`.
The corrected native recreation and fresh public-project acceptance use these
same saved bytes; the native and MCP studies remain separate input paths.

## Editor guard and exact final interchange

A separate **9-request actual MCP** consumer study verifies that the legacy
contradictory B,A case fails Editor export before creating an output and without
changing session state. Runtime43 retains its exact raw output. Newly ordered
data exports successfully for Editor. The guard identifies the attachment and
unresolved bones rather than sorting or merging source arrays. It is an Editor
consumer restriction, not a new native-format validation ban.

The exact `final-editor-accepted.json` artifact (SHA256
`cddfcdb42cac1b97bbd495a92843dd3669a828141082f719f43d49e168698b9c`)
was copied with only `skeleton.images` changed for image availability, imported
into a fresh installed Spine **4.3.26** project, re-exported and imported through
our CLI. Compared with the exact final ordered MCP project at **52** poses:

| Round-trip comparison | Values | Maximum error |
| --- | --- | --- |
| Bone world transforms | 1,248 | `0` |
| Weighted mesh coordinates | 1,248 | `0.0001678466796875` |
| Local UV values | 1,248 | `0` |

All **52** draw orders, clip key times/values/curves, UVs, triangles and hulls
match. World-geometry tolerance remains **0.001**. All immutable source hashes
are unchanged. This final named artifact is verified separately from the
earlier diagnostic A/B control.

Native re-export prunes **5** zero entries and rounds effective weights by at
most `4.289999999990135e-6`. Surviving bind positions are exact, with no surviving
entry reorders for this final source. Pruned zero bind positions are lost in
JSON, so later authoring can differ; sampled geometry success is not raw-lossless
interchange. Exact command/hash reports are private under
`installed-interchange/final-ordered/`; licensed CLI logs remain private.

Brush painting, locks, Auto, Smooth, Update bindings, multi-bone editing and
broader weight clipboard operations are not implemented by this slice. Pies and
overlay controls, exact styling, arbitrary docking and all modifier behavior
remain open. Full Spine parity is not claimed.
