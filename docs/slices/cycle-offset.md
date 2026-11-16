# Cycle Offset authoring — November 16, 2026

**Bounded native/engine/MCP Offset checkpoint verified; full parity remains
open.** Native v23 and 64 persistent commands are unchanged; the current
25-tool server exposes `offset_animation_keys` and `reset_cycle_offset_memory`
through the shared Session boundary. See the [MCP contract](../MCP.md). These
operations add no document format, persistent `Command` or engine I/O. Desktop
integration passes the closed Bone scalar scope below on Desktop `dc60e423…b2998`.
Ordinary retiming remains separate, and the overall Spine-parity goal stays
active. See the [native UI evidence](../evidence/cycle-offset-ui.json) and
[earlier foundation evidence](../evidence/cycle-offset.json).

## Shared boundary

The independent [planner](../../crates/engine/src/cycle_offset.rs) exposes
`CycleOffsetPlan::capture(document, animation, keys, range)` and
`capture_with_memory(..., memory)`. `CycleOffsetRange { start, end }` and the
`advance(document, delta)` delta use animation seconds. Advancing produces a full
candidate `CycleOffsetOutcome { document, memory, keys, boundary_keys, phases, mappings }`;
the pure planner publishes nothing. Keys follow first-requested track order,
then time. Boundary identities are not a native selection-policy instruction.

Outcome and receipt `mappings` contain only requested seeds, in request order:
`CycleOffsetKeyMapping { source, target: Option<AnimationKey> }`. Targets follow
original-anchor lineage at the accepted phase, with the same checked wrapping
used for stored keys. They do not add delta to an already rounded current time.
Original phase-zero first/end seeds may map to the same accepted anchor. `None`
means an ignored generated seam; it is not a request to select all affected keys.
The desktop maps selection/primary through these records rather than through
`keys` or `boundary_keys`.

`Session::cycle_offset_selectable_keys(animation, keys, range)` and the pure
engine query return actual-anchor IDs in input order, without a solve, mutation
or revision change. The desktop uses this shared mask for hit testing. Empty
queries are allowed; exact-ID, duplicate, unsupported-family and stale checks
remain shared. Known generated seams are excluded; an uncached track starts
fresh from its explicit current raw keys. No departed eligibility registry is
retained.

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

Every requested identity must be an exact existing Bone scalar key. One or more
actual-anchor seeds select their whole property tracks; multiple seeds on one
property still offset it once. Known generated seams do not seed a track and map
to `None`, even in a mixed request. An all-generated request succeeds without
changing Document, pending values, memory, authoring history or redo; its affected
keys/boundaries/phases are empty. The MCP successful-mutator revision still
advances. Each affected track must have at least two keys, exactly
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

Additional no-history controls distinguish actual anchors from generated seam
keys. Original first0 and terminal60, tested separately, both follow the Frame6
anchor after +6; Backspace without reselection removes that anchor. Clicking a
generated zero after driver-only +6 and attempting +6 leaves the Document
unchanged. A nonzero closing end is ignored even when a genuine anchor occupies
zero; that genuine zero anchor can move. Returning to phase zero restores original
raw keys and terminal selectability. Offset Off/On recaptures current data:
another +6 then carries the previous seam into the interior. Attempts involving
Delete/Undo or an unselected key are not used to infer generated-key policy.

All sixteen focused [engine tests](../../crates/engine/src/cycle_offset_tests.rs)
pass, including requested-order mappings, f32 association, endpoint collisions,
the shared mask, fake-only no-ops, real zero/phase return and explicit recapture.
Native seven-step checks retain exact Document/memory snapshots through seven
Undo/Redo operations.
The retained native values/control points use a qualified **0.0001** approximation
bound. Sampling and the independently calibrated subdivision are approximate;
exact cubic inversion is not claimed. Separate mathematical tests cover
Linear/Stepped periodic sampling, negative/full-period moves and a nonzero range
that exactly fills its track; they do not close those native gates. Tests also
cover cohort-relative phases, stale raw bits, collisions, pending/history, fixed-base
previews, cancellation and redo-preserving no-ops. Existing history/preview/remap
regressions pass. Current combined checks02/03 pass six MCP unit/two actual-stdio
tests, Dopesheet43, key-drag4, clipboard45, Graph48, workspace16
and pane-resize5 (overlapping filters), formatting, all-target Clippy, workspace
build and architecture7. Final desktop Offset coverage passes **15 tests**, after
the fast-release correction; the earlier 12-case count is historical. No full
suite or hosted-CI result is claimed.

## Bounded Peachy native acceptance

Candidate02 passes actual OS input on Desktop
`dc60e423a84a6994d3bd026b6333cf3b030c364b5a611f3059b4146c3a7b2998`:
fast Offset and ordinary retime, mixed two-track movement with one Undo, only
mapped-seed selection, generated-seam hit rejection, explicit/automatic bounds
and Off/On recapture producing six keys. A pending **X25 on the same Bone** stays
live while timing changes; Escape and Save cancel an owned changed preview and
block continued held movement. Save excludes the pending value. At 1120×740,
Offset, Loop Start and End controls are accepted with manual Fit and a filtered
Tree; visibility of every Tree row is not claimed. Actual edited Save and fresh
reopen restore the exact Document with empty history, pending pose, Offset cache
and editor clipboard, plus one PNG. All owned processes are closed.
Native nonzero-loop wrapping remains a separate gate from the accepted bounds
controls.

The [native saved result](../../examples/peach-bot/cycle-offset-native-result.peachy)
has SHA256
`68126b9a56801e5be6a2e6b6ca641dfcf4145832bbddbe6f348d3e7239ed42a4`.
A separate **15-request actual MCP replay** constructs from a blank root through
97 commands, then applies the measured UI f32 deltas rather than rounded frame
labels. Complete Document, authored/applied/world poses including the pending
value, and reopened native-file data match exactly. MCP Save is byte-identical
to the native file. This is independent headless replay, not a live GUI bridge.

That exact native result passes **364 external-runtime poses** across seven
clips at **0.001** with Spine 4.3.13: maximum world error
**1.547993635142575e−5**, Region error **1.52587890625e−5**. Inputs and CLI remain
unchanged. This original Region fixture adds no mesh or private-asset coverage.

## Earlier protocol and foundation proofs

The final MCP metadata/reset study uses binary `ee0bebd0…a38f4` and **32 requests**
across two owned clients: three observer requests open only our public baseline
to obtain typed expected data; the independent author uses 29 requests and
constructs from a blank root through **97 commands**. No finished rig is an
authoring input. Original-boundary collisions, generated-seam `None` mappings,
fake-only no-op state, mixed mappings, native-toggle-equivalent reset and exact
Save/reopen pass. Both clients close, and inputs/binary remain unchanged. Its
saved result is SHA256
`050f5abd1ef2f98a8c9f9eb95a769a6109b526d71bfc9f8c588020bcfd3272ab`.
This is a separate protocol/source proof; the later native acceptance above
belongs to the distinct `68126` result.

The earlier foundation run04 on MCP `6e09bc36…94383` passed **276 requests**, **97 construction
commands** (57 public-lab commands plus 40 original cycle-clip commands) and
**11 atomic negatives**. It reproduces all seven native Linear stages and checks
pending/memory history, stale/reset recovery, no-op redo preservation and exact
Save/reopen. Inputs and binary are unchanged and the owned child is closed.
That earlier saved result has SHA256
`0316f8487a35701171c2f18b27c483d9a5b5beb47854534aaf03dca4c4cf3d73`.
At that checkpoint, all seven clips passed 52 CLI/external-runtime samples each,
**364 poses** at unchanged **0.001**: maximum world error
1.4148056052931679e−5 and Region error 1.52587890625e−5. This geometry comparison
does not accept desktop Offset interaction or the unmeasured native operations.
Its 364-pose proof covers `0316`, not the new `050f` saved result; no fresh runtime
comparison of `050f` is claimed here.

Remaining gates include native Stepped/negative/full-period/nonzero-loop cases,
broader selection combinations and intervening edits, broader typed timelines,
Shift/ripple/Adjust, editable Current, full toolbar styling and general
marquee/stacked Graph workflows. Canonical start targeting when one anchor
coincides with both boundaries follows engine wrapping; full native 0/end
unmerge selection is not directly accepted. Full history/context equivalence,
the live desktop MCP bridge and full Spine parity also remain open. The
[ordinary timeline](multi-owner-timeline.md) and
[grouped clipboard](grouped-clipboard.md) checkpoints do not substitute for them.
Only our code, ordinary mathematics and original input/output observations were
used; proprietary editor/runtime implementation was not inspected.
