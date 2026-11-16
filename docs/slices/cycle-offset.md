# Cycle Offset engine foundation — November 16, 2026

**Shared engine/MCP checkpoint verified; complete native Offset authoring
remains open.** Native v23 and 64 persistent commands are unchanged; the current
25-tool server exposes `offset_animation_keys` and `reset_cycle_offset_memory`
through the shared Session boundary. See the [MCP contract](../MCP.md). These
operations add no document format, persistent `Command` or engine I/O. Full
desktop Offset control/acceptance remains open. Ordinary retiming is separate,
and the overall Spine-parity goal stays active. Final source checks, actual MCP
run04 and the saved-result runtime comparison pass within the scope below; see
the [scalar evidence](../evidence/cycle-offset.json).

## Shared boundary

The independent [planner](../../crates/engine/src/cycle_offset.rs) exposes
`CycleOffsetPlan::capture(document, animation, keys, range)` and
`capture_with_memory(..., memory)`. `CycleOffsetRange { start, end }` and the
`advance(document, delta)` delta use animation seconds. Advancing produces a full
candidate `CycleOffsetOutcome { document, memory, keys, boundary_keys, phases }`;
the pure planner publishes nothing. Keys follow first-requested track order,
then time. Boundary identities are not a native selection-policy instruction.

`CycleOffsetMemory` is opaque, transient memory for one active canonical set of
property tracks, including animation and loop identity. Seed order/count changes
within that cohort retain its original curves and accepted phases. Repeated edits
recompute from those originals rather than accumulating generated seam keys.
Every context transition validates all old remembered tracks first. Shrinking
retains the surviving originals/phases and drops departed tracks. Adding any
new track, including returning a departed track, captures all selected current
tracks together at phase zero. Departed contexts are not automatically revived.
Phases describe each retained or newly captured origin. History can explicitly
restore an earlier snapshot.
Immutable source buffers are shared across those snapshots. No memory is
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

Every selected identity must be an exact existing Bone scalar key. One or more
selected keys seed their whole property tracks; multiple keys on one property
still offset it once. Each affected track must have at least two keys, exactly
fill the finite nonnegative ordered loop bounds, and have equal first/end values.
Empty selection, other typed families, outside-range tracks, duplicate or unknown
identities, nonfinite inputs and collapsed/unrepresentable times refuse
atomically. This whole-track expansion is specific to Offset; ordinary
`MoveAnimationKeys` continues to address selected keys. Broader track types and
loop ranges remain temporary feature boundaries.

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
all four clips exactly. A separate plain-click control selects only the Linear
driver key at Frame 12/value 32, then offsets +6: the whole driver track becomes
Frames 0/6/18/42/60 with seam value 1.999998; the follower is unchanged. Selecting
all five current driver keys and moving +6 again retains its original source:
five keys at Frames 0/12/24/48/60, seam −4.000004. Exact key subsets therefore
seed whole properties without seed expansion alone resetting the origin.

Changing the affected set after driver-only +6 is different: a marquee of both
current tracks followed by +6 gives six driver keys and five follower keys.
The driver's old seam survives at Frame 6/value 1.999998; its new seam is
−4.000004, while the follower seam is 30.000004. Explicitly positioned input
confirms that expansion; three more moves still affect both tracks. A genuine
Tree-scoped shrink to driver retains its origins: six keys at
Frames 0/6/24/30/42/60 become six at 0/12/30/36/48/60, with the follower unchanged.
Returning both tracks rebases their current data, producing seven driver and
six follower keys, including carried old seams. The complete seven-step native
sequence is driver/both/both/both/both/driver/both, each +6 frames.
Earlier canvas clicks intended as subset changes actually still moved both;
they are not treated as shrink evidence. General context/loop changes and
intervening-edit policies remain broader calibration gates.

All ten focused [engine tests](../../crates/engine/src/cycle_offset_tests.rs) pass,
including native single-key/seven-step and context regressions, with exact
Document/memory snapshots through seven Undo/Redo operations.
The retained native values/control points use a qualified **0.0001** approximation
bound. Sampling and the independently calibrated subdivision are approximate;
exact cubic inversion is not claimed. Separate mathematical tests cover
Linear/Stepped periodic sampling, negative/full-period moves and a nonzero range
that exactly fills its track; they do not close those native gates. Tests also
cover cohort-relative phases, stale raw bits, collisions, pending/history, fixed-base
previews, cancellation and redo-preserving no-ops. Existing history/preview/remap
regressions pass. Final checks04 also pass four MCP unit/two actual-stdio tests,
existing/discovery gates, formatting, all-target Clippy, workspace build and
architecture7 with stable source hashes. No full suite or hosted-CI result is claimed.

Actual run04 on final MCP `6e09bc36…94383` passes **276 requests**, **97 construction
commands** (57 public-lab commands plus 40 original cycle-clip commands) and
**11 atomic negatives**. It reproduces all seven native Linear stages and checks
pending/memory history, stale/reset recovery, no-op redo preservation and exact
Save/reopen. Inputs and binary are unchanged and the owned child is closed.
The canonical saved result has SHA256
`0316f8487a35701171c2f18b27c483d9a5b5beb47854534aaf03dca4c4cf3d73`.
With the current CLI, all seven clips pass 52 external-runtime samples each,
**364 poses** at unchanged **0.001**: maximum world error
1.4148056052931679e−5 and Region error 1.52587890625e−5. This geometry comparison
does not accept desktop Offset interaction or the unmeasured native operations.

Remaining gates include native Stepped/negative/full-period/nonzero-loop cases,
broader selection combinations and intervening edits, broader typed timelines,
and complete desktop integration/native acceptance. The
[ordinary timeline](multi-owner-timeline.md) and
[grouped clipboard](grouped-clipboard.md) checkpoints do not substitute for them.
Only our code, ordinary mathematics and original input/output observations were
used; proprietary editor/runtime implementation was not inspected.
