# Global Animate draw-order authoring

November 14, 2026; continuation from `0326fc3` and the Curves slice.
Combined engine, native UI and MCP acceptance passes within the scope below.
Actual native input, fresh-process persistence, both authored runtime checks and
installed Spine 4.3.26 interchange pass. Native v22 remains unchanged; MCP has
51 commands / 11 tools. See [the review gallery](../evidence/draw-order-authoring.html)
and [scalar evidence](../evidence/draw-order-authoring.json). Hosted CI still
gates main; this is not full Spine or pixel parity.

## Scope and sources

This slice authors the skeleton's global draw order in Animate. Existing
[draw-order data/evaluation](draw-order.md) already imports, preserves, samples
and exports global stepped keys. Native v22 already contains that data; no
persistent format change is required.

Primary public references, retrieved through agent-reach on November 14:

- [Slots guide](https://esotericsoftware.com/spine-slots): Tree draw order,
  dragging, keyboard movement and slots-only filtering.
- [Keys guide](https://esotericsoftware.com/spine-keys): keying the current
  order beside the Draw Order node.
- [Spine 4.3 release post](https://esotericsoftware.com/blog/Spine-4.3-released):
  the Look around example uses draw-order changes alongside image swaps;
  independently keyed draw-order folders are a broader feature.

Installed-reference evidence uses only our original four-slot cutout fixture
under `.local/training-spine-20261114/draw-order-authoring/`. Its `task.json`
records ownership, source hashes and retention; `native-observations.json`
indexes the measured behavior. No proprietary editor/runtime implementation or
vendor artwork was inspected. Private native captures and exported probes are
reference inputs, not committed application screenshots.

## Verified installed layout and controls

`reference-draw-order-selected.png` shows an actual Animate workspace:

- The Tree has a selectable **Draw order** category. Its slot children list
  the frontmost slot first. The key control sits to the left of the category.
  Slot selection identity survives reordering.
- Selecting the category shows **Draw order** Properties with **Reset**.
  The captured New control is disabled; folders are outside this slice.
- The Dopesheet has a **Draw order** hierarchy and a **Default** child row for
  discrete global keys. It is not an interpolated scalar Graph/Curves channel.

Stored/evaluated permutations run **back to front**, opposite the Tree's visual
top-to-bottom presentation. Do not reverse the engine array to imitate the Tree.

The original fixture's setup indices are red `0`, green `1`, blue `2`, gold `3`.
Its key at `0.5 s` moves red by `+3`, producing `[1, 2, 3, 0]`; the explicit
empty-offset key at `1.5 s` restores `[0, 1, 2, 3]`. Before the first key, setup
order applies. Installed CLI and UI baselines have identical animation data.

## Verified native editing behavior

The following are observations of the installed reference, not extrapolations
from public shortcut descriptions. Export names refer to directories under the
private evidence root above. Comparisons concern animation data; Spine's
derived skeleton hash is not an authored-order assertion.

| Action | Observed result and evidence |
| --- | --- |
| Press the physical equals/plus key, macOS keycode 24 | Moves the selected slot one place toward the front. Minus, keycode 27, moves one place toward the back. |
| Shift plus/minus | Requests five positions and clamps at the order bounds. The four-slot probes reach front/setup order without wrapping. `shift-plus-clamp` keys red `+3`; `shift-minus-clamp` keys an explicit empty-offset reset at `1.0166667 s`. |
| Auto Key off at an unkeyed time | A visible order change remains pending. `plus-unkeyed-save` has exactly the baseline animation data after Save/export. |
| Explicitly key that order | `keyed-zero` adds a time-zero key with red offset `+1`. Its missing JSON `time` field denotes zero. |
| Auto Key off at an existing key | A visible edit at `0.5 s` still remains pending. `existing-off-save` equals `keyed-zero`; merely editing on a keyed time does not persist the replacement. |
| Enable Auto Key with an existing pending order | Flushes it immediately. `autokey-enabled-pending` changes the `0.5 s` red offset from `+3` to `+2`, without another movement. A subsequent Auto Key edit changes that key to `+1` in `autokey-existing-edit`. |
| Enable Auto Key with mixed pending edits | At `1 s`, `mixed-autokey-enabled` adds root Rotate `15` and the pending draw-order key together. One native Undo removes both persistent keys; `mixed-autokey-undo` equals `autokey-existing-edit`. |
| Move farther past a boundary | The measured plus action produces no animation change and preserves redo. `boundary-plus-noop` equals `shift-plus-undo`; subsequent `boundary-redo` equals `shift-plus-clamp`. |
| Reset an already-setup order | No animation change: `setup-reset-noop` equals `shift-minus-clamp`. This establishes this no-op case, not every Reset/history policy. |
| Drag gold from frontmost to below red | `drag-to-back` adds a key at `1.05 s`, gold offset `-3`. One Undo restores the prior animation: `drag-undo` equals `shift-minus-clamp`. |

Independent comparison of the actual exported pairs confirms the equalities
above. The native record also confirms all original input hashes are unchanged.
These cases do not establish every repeat-key, multi-selection, drag-cancel,
Reset-from-nonsetup, cross-document paste or focus-loss behavior.

## History and pending-pose distinction

The native mixed Auto Key test establishes a single persistent data Undo. Its
post-Undo visuals re-evaluate the keyed pose: root Rotate returns to zero and
order comes from the remaining keys. The pre-enable pending pose is not visibly
retained in that observation.

Peachy's established `WorkingState` history snapshots document data and pending
pose together. Its intended mixed-flush Undo restores the pre-enable pending
pose as well as removing the persisted keys. That preserves pending work, and
is an explicit difference from this native visual observation. Verify atomic
data Undo and pending-state preservation separately; do not claim exact native
history parity from matching exported animation JSON.

Pending order must stay outside the saved/exported document and its dirty
baseline, while remaining queryable as an unsaved pose edit. Keying removes the
relevant pending order without discarding unrelated bone, IK, slot or Deform
edits. Enabling Auto Key must flush the applicable mixed pending edits through
one shared Session candidate. A boundary/no-op movement must preserve redo.

## Shared engine and MCP contract

The current boundary has two distinct persistent commands:

- `SetDrawOrderKey { animation, key }` accepts exact validated raw offset data.
  Existing imported encodings, including valid redundant zero entries, remain
  first-class data.
- `SetDrawOrder { animation, time, order }` accepts a complete back-to-front
  permutation of every slot. Duplicates, omissions and out-of-range indices
  fail atomically. New/replaced semantic keys use the independent minimal
  encoder described below. Re-keying an existing same-order key preserves its
  original raw representation.

Session reads combine sampled and pending order. Animate edits, frozen-baseline
previews and explicit keying share the existing `WorkingState` transaction and
history boundary. Setup order remains readable; Setup reordering is deferred.
Explicit keying can create a hold/reset at an absent time even when no pending
movement exists. A pose movement that changes nothing is a separate no-op.

The strict typed singleton owner is `KeyOwner::DrawOrder {}`; an exact key is
`AnimationKey::DrawOrder { time }`. It must never alias a selected slot or cached
bone. Move/delete/copy/paste use the shared typed-key path and retain raw offsets
and relative times. The current clipboard carries ordered slot names and raw
keys; paste requires the same ordered slot-name layout. That is Peachy's
explicit bounded policy, not verified native cross-document clipboard parity.

MCP publishes the same commands as `setDrawOrderKey` and `setDrawOrder`, the
`drawOrder` owner/key/clipboard branches, and the complete `drawOrder` array
from `evaluate_pose`. The array includes hidden slots and slots without a
textured attachment. Scene geometry alone cannot reconstruct it. Evaluation
remains read-only and samples the separate headless document; it does not
inspect a desktop pending pose.

Strict schema/decoder agreement, finite nonnegative time, signed integer raw
offsets, exact typed identities, revision checks and all-or-nothing command
batches remain acceptance requirements. Failure must preserve document,
revision and history. Global stepped keys must not become interpolated curves.

## Raw preservation and independent minimal encoding

The adapter keeps valid offset entries as supplied, in setup-slot order. Each
listed slot moves to `setup_index + offset`; unlisted slots fill remaining
positions in setup order. Validation rejects unsorted/repeated slots, colliding
or out-of-range destinations and invalid key times.

An empty-offset key is a meaningful explicit reset. Export keeps the key and
omits its empty `offsets` field; reimport reconstructs the empty list. Raw-entry
preservation does not imply byte-identical JSON field spelling. Deleting that
reset changes later sampled behavior and must not be treated as deleting nothing.

For a newly authored permutation, keeping a longest increasing subsequence of
setup indices minimizes the number of explicitly moved slots. Peachy's encoder
uses independent patience-sorting mathematics, then emits the other slots in
setup order with their target displacement. Its deterministic choice among
equal-length subsequences is our own. A different equally minimal list can have
the same evaluated order; exact native tie choice is not claimed. Untouched raw
keys are never globally normalized to this encoding.

## Verification status and remaining scope

Final focused checks pass with two workers/test threads: engine draw order
**20**, legacy timeline **26**, desktop draw order **9**, Curves **48**,
clipboard **19**, Tree **4**, MCP schema **9** and actual stdio **10**.
Filters overlap. Formatting, all-target Clippy, workspace build and the
seven-crate architecture check pass. Curves footer controls fit their column;
actual Tree widget IDs stay stable across reorder and Undo. These checks do not
replace the separately recorded native and runtime acceptance below.

### Verified actual MCP authoring and runtime checks

The retained `mcp-authoring.py` imports the earlier slot study's stdio client
and uses the freshly released MCP executable. It starts with `create_project`
and the four original PNGs, then authors one root, four slots/regions and two
clips. It never imports the finished reference JSON; that fifth original input
is hashed only to prove it remains unchanged.

The successful attempt verifies **51 commands / 11 tools**, **343 protocol
requests**, Save/reopen and explicit Runtime 4.3 JSON/bundle exports:

- `orders` has a raw nonminimal key at `0.125 s` with redundant zero entries,
  semantic order `[1, 2, 3, 0]` at `0.5 s`, and setup reset at `1.5 s`. Red is
  hidden at `0.75 s` and shown at `1.25 s` without losing its full-order entry.
- Copying the two semantic order keys into `copied-orders` at `0.25 s` preserves
  their relative times and the explicit reset, ending at `1.25 s`.
- Exact global copy/retime/Delete/Undo/Redo, coincident slot-key isolation, raw
  zero preservation and same-order semantic no-op with redo alive pass.
- Eight rejected wrong-layout, invalid-order/raw-offset/time, strict-owner and
  stale-revision requests preserve the complete document, revision and history.
- **265** full pose samples, including Setup and **32** hidden-slot samples,
  retain complete order and finite readback. All **5** original input hashes
  remain unchanged; the owned server exits successfully.

The external, separately licensed Spine 4.3 oracle checks both clips and their
only skin, `default`, from the exported bundle/JSON:

| Check | Verified comparisons | Maximum error |
| --- | --- | --- |
| Standard `check-runtime` | 104 pose/visible-order samples; 624 world values; 3,200 region values; 1,600 color channels | World `2.832769475542897e-16`; regions `0`. |
| Independent public-API comparison against every retained MCP pose | 265 complete-order arrays / 1,060 slot indices, including hidden slots; 265 visible orders | Exact discrete order match. |
| Geometry/color at those same MCP times | 1,590 world values; 8,224 region values; 4,240 raw color channels | World `2.832769475542897e-16`; regions and colors `0`. |

These are geometry/data checks, not raster or native-input acceptance. Private
`mcp-report.json`, `mcp-authored/runtime-report.json`,
`mcp-authored/runtime-order-report.json` and
`mcp-requests/attempt-3/artifact-index.json` retain exact totals, hashes and input
provenance. Request logs and raw samples remain private. The first two authoring
harness failures and the direct-check API/corner-mapping corrections are
recorded honestly; only the successful final study is acceptance evidence.

### Verified native-authored runtime and Editor interchange

The coordinator's `native-acceptance.json` records actual Peachy authoring:
pending Save exclusion at absent/existing keys; typed Rotate `15`; one mixed
Auto Key Undo/Redo with pending preservation; Shift clamps and boundary/Reset
no-op redo; nonsetup Reset; selected/unselected-source drags; stable slot
identity; Escape/held-pointer/outside-release cancellation and row reentry;
typed copy/paste/retime/Delete Undo/Redo; playback and exact-f32 Save. That is a
verified interaction record. A fresh final-workspace-build process opens the
byte-identical public example with all eight keys, four images, no history,
no pending pose and working playback. All three owned Peachy processes and the
reference/CLI processes are closed; other tasks' browser sessions were untouched.

The stable final `native-start.peachy` has **8** global keys, including the
explicit setup reset at `1.75 s`, plus root Rotate `15` at `1 s`. Its SHA256 is
`7dd9e844eb4b4fbb743be210472d930603ae07115d73d9feb12367a8d1f20149`.
Runtime 4.3 JSON/bundle and Editor-target exports leave this project and all
five original inputs unchanged.

- External `check-runtime` passes `calibration` and its only skin, `default`,
  at **52** sampled poses/visible-order checks: **312** world values, maximum
  error `1.25668286887759e-8`; **1,664** region values, maximum error
  `7.62939453125e-6`; **832** color comparisons.
- Installed Spine **4.3.26** CLI imports the Editor export into a fresh `.spine`
  project and re-exports JSON. All **8** global key times and effective orders
  match; an independent **23**-time order comparison includes key boundaries
  and held intervals. Root setup rotation and the single Rotate `15` key at
  `1 s` retain their semantics.
- One equally minimal raw offset spelling changes in that Editor round trip.
  It is semantically equivalent and does not establish our encoder's native
  tie choice. Runtime export preserves the source's exact supplied raw lists.

Private `native-runtime-report.json` and `native-runtime-artifact-index.json`
retain counts, hashes, CLI commands/process exits and the semantic comparison.
An initial restricted Spine CLI launch aborted before creating output; its log
and failed report are retained. Approved unsandboxed CLI import/export then
complete successfully. These are data operations with no GUI input. Both CLI
leases were released after completion. Final native captures use only Peachy
and our original geometric artwork; installed-editor captures stay private.

The combined checks cover all four-slot permutations, retained raw zeros,
reset copy/retime/delete, hidden-slot order, coincident key isolation, atomic
failures, pending Save exclusion, mixed flush history, boundary no-op redo and
actual native drag/cancellation. Native Save is compared at exact f32 identity;
display/export decimal spelling alone is not a data mismatch.

Draw-order folders and their independent timelines, Setup slot reordering and
identity remapping, constraint-modified draw order, broader native clipboard
mapping, compact Tree/property sizing and exact history parity remain deferred. Existing export-target rules,
private-art isolation and the required hosted CI gate remain unchanged. No full
Spine compatibility or pixel-parity claim follows from this bounded slice.
