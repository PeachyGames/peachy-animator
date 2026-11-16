# Cycle Offset engine foundation — November 16, 2026

**Engine foundation verified; complete Offset authoring remains open.** This
checkpoint adds no document-format change, persistent `Command`, I/O, desktop
Offset control or MCP Offset tool. Ordinary retiming remains a separate operation.
The overall Spine-parity goal stays active.

## Shared boundary

The independent [planner](../../crates/engine/src/cycle_offset.rs) exposes
`CycleOffsetPlan::capture(document, animation, keys, range)` and
`capture_with_memory(..., memory)`. `CycleOffsetRange { start, end }` and the
`advance(document, delta)` delta use animation seconds. Advancing produces a full
candidate `CycleOffsetOutcome { document, memory, keys, boundary_keys, phases }`;
the pure planner publishes nothing. Keys follow first-requested track order,
then time. Boundary identities are not a native selection-policy instruction.

`CycleOffsetMemory` is opaque, transient memory per typed track and loop, with
independent accepted phases and remembered original curves. Repeated edits
recompute from those originals rather than accumulating generated seam keys.
Unselected tracks retain their memory; reselection alone does not reset it.
Immutable source buffers are shared across history snapshots. No memory is
serialized into the saved Document.

[Session](../../crates/engine/src/session.rs) provides `offset_animation_keys`
and `preview_offset_animation_keys` with the same animation/keys/range/delta
inputs, returning an accepted `CycleOffsetReceipt`. Previews use the transaction's
fixed Document, pending pose and memory baseline. Candidate validation and pending
reconciliation complete before publication. Commit makes one history step;
cancel restores the baseline. Undo/Redo snapshots restore pending state and
original-curve memory together. Failed calls retain the last valid preview and
redo. Cache-only capture and zero/full-period no-ops create no authoring history
or dirty state and preserve redo. `cycle_offset_memory` is read-only;
`reset_cycle_offset_memory` explicitly forgets origins and refuses during a
transaction.

## Current scope and refusal

Every selected identity must be an exact existing Bone scalar key. Each affected
track must be selected completely, have at least two keys, exactly fill the
finite nonnegative ordered loop bounds, and have equal first/end values.
Partial key selections, other typed families, outside-range keys, duplicate or
unknown identities, nonfinite inputs and collapsed/unrepresentable times refuse
atomically. These are temporary feature boundaries, not claims about every
native Offset case.

The current affected keys must match the memory's expected times, values and
curve-handle bits, with the captured animation/Bone names and FPS unchanged.
An intervening affected edit/context change is explicitly stale; it never
silently becomes a new origin. Callers must explicitly forget and recapture.
Unrelated edits remain intact. Returning to phase zero restores original raw
keys/curves. Other phases wrap original points and generate loop-boundary values
and split curves through the existing shared sampler/subdivision helper.

## Reference observations and checks

Installed Spine 4.3.26 black-box probes use an original four-clip, two-track
fixture at 60 FPS. The first intended eight-key selection was not established:
its export moved only four driver keys. A later explicit marquee proves all
eight source keys selected and both tracks shifted. Qualified results are:

| Total movement | Driver keys / seam value | Follower keys / seam value |
| --- | --- | --- |
| +12 frames | 5 / 11.195514 | 5 / −16.49734 |
| +18, without reselection | 5 / −5.19033 | 5 / −24.695436 |
| +24, after deselect/reselect of all current keys | 4 / −16 | 5 / −4.999988 |

Original curves survive that reselection sequence. Three Undo operations restore
all four clips exactly. These observations do not establish partial-selection or
intervening-edit memory behavior.

Eight focused [engine tests](../../crates/engine/src/cycle_offset_tests.rs) pass.
The retained native values/control points use a qualified **0.0001** approximation
bound. Sampling and the independently calibrated subdivision are approximate;
exact cubic inversion is not claimed. Separate mathematical tests cover
Linear/Stepped periodic sampling, negative/full-period moves and a nonzero range
that exactly fills its track; they do not close those native gates. Tests also
cover per-track phases, stale raw bits, collisions, pending/history, fixed-base
previews, cancellation and redo-preserving no-ops. Existing history/preview/remap
regressions, all-target Clippy, workspace build and architecture7 pass; no full
test suite or hosted-CI result is claimed.

Remaining gates include native Stepped/negative/full-period/nonzero-loop cases,
partial selections and intervening edits, broader typed timelines, and complete
shared UI/MCP integration and acceptance. The
[ordinary timeline](multi-owner-timeline.md) and
[grouped clipboard](grouped-clipboard.md) checkpoints do not substitute for them.
Only our code, ordinary mathematics and original input/output observations were
used; proprietary editor/runtime implementation was not inspected.
