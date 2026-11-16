# Leaf clipboard source and pose targets — November 15, 2026

**Bounded native owner-routing, actual MCP and final source checks06
pass.** Native v23 and **64 commands /19 tools** are unchanged. This continuation addresses the
single-owner fallback gap from the accepted [grouped clipboard](grouped-clipboard.md)
checkpoint. The new leaf replay uses Desktop5825; the parent Desktop87ee group
minimum-window proof remains separate. See the [review gallery](../evidence/leaf-clipboard.html)
and [scalar evidence](../evidence/leaf-clipboard.json).

## Measured installed-reference cases

Owned Spine 4.3.26 input uses the same original six-bone/two-region/one-PNG fixture
at 60 FPS. The copied leaf contains driver Translate Y at Frames 5/25, values 3/14.
Paste at Frame 50 produces two keys at Frames 50/70, retaining values 3/14:

| Accepted reference context | Destination |
| --- | --- |
| No selected bone / animation context | Original driver, Bone 2 |
| Explicit different bone | source_parent, Bone 1 |
| Different animation with no bone selected | Original driver, Bone 2, in `retime` |
| Active slot's pose-bone context | follower, Bone 4 |
| Actual region attachment's pose-bone context | follower, Bone 4 |

These are Bone Y keys. Slot/Region selection does not turn this payload into
attachment or RGBA keys. The active slot/region uses its parent pose bone in
the measured cases. Native Slot and Region Dopesheets show the follower Bone's
rows; unrelated driver keys must be pruned behind a Region context.
Combined source checks enforce that parent-Bone scope/pruning. An inactive
Region retains an explicit read-only scope rather than falling back to all owners;
Graph fallbacks also require editability. Inactive/pending/constraint-remap
behavior has headless coverage; this slice adds no physical native pass for it.

The visible Peachy Animate Slots-filter row is `TreeItem::DrawOrderSlot`, not
`Slot`. Its Dopesheet currently keeps **global DrawOrder scope**, not merely
Slot-only scope. The parent-Bone routing fix now includes that actual row type,
with an actual filter/row-event regression and passed native replay. This fixes
the Bone leaf paste target; native Slot/Bone compound scope remains open.

`leaf-attachment-paste` was actually a Bone:tip selection. It is a valid extra
bone-target control, **not Region evidence**; the true Region case is
`leaf-true-region-paste`. An earlier cross-animation attempt also retained a bone
after an empty-viewport click. That attempt is excluded; Escape cleared the bone
before the accepted cross-animation case.

Each accepted case undoes exactly. Reference animations/bones and the complete
export SHA256 restore to baseline:
`7e8191a5aea88ee03cbd18f281aa07a3684f1dd37561401d41b7c288f3d4004b`.
The owned reference process is closed; original 933×728 bounds, Setup mode and
Bone/Slot filters are restored. Private records remain under
`timeline/multi-owner/leaf-reference-report.json` and `group-clipboard-reference/`.
No licensed captures or artist artwork are published.

## Accepted source contract

The existing portable `AnimationKeyClipboard` and version 2 OS envelope remain
unchanged. A process-local `LeafSourceReceipt` separately records the canonical
source owner, the exact owned payload text and copy-time source animation
index/name. The receipt is not serialized into the native document or portable
payload. No format migration or new MCP command/tool is introduced.

For the measured Bone payload, a genuine explicit Bone target overrides the
source. Active Slot, visible Animate DrawOrderSlot and true Region pose targets
resolve their parent Bone, subject to existing activity/read-only checks. With no explicit typed target,
including no-owner/animation context, paste uses the receipt's canonical source
owner. The destination remains the active animation/cursor; source animation
is diagnostic context, not a command to paste back into the old clip.

Existing non-Bone leaf kinds retain explicit kind/layout validation. This slice
does not define a combined Slot/Bone-family Dopesheet or the complete Region
pose-control/picking workflow. Slot compound-family scope and broader Region
controls remain separate gaps.

Canonical ownership is captured through the shared copy path, including linked
Deform aliases. Constraint-index effects remap owner receipts alongside editor
state; removal produces a tombstone rather than redirecting fallback to another
index or cached bone. Without an available canonical source, implicit fallback
refuses. Explicit compatible targets still use the shared paste validator.

Successful Open/New clears the old project clipboard/receipt. Failed Copy, Paste,
Open or New must preserve the existing document/history/redo and prior valid
receipt as appropriate; they must not rebind unmatched payload text to an old
owner. External/system text is not sufficient to recreate a source receipt.
Final checks04 and the five-case native replay cover the added visible-Slot
routing. Successful native reopen also verifies the cleared receipt cannot be
revived by old OS clipboard text.

Paste continues to preserve original selected IDs/primary; accepted incoming
IDs are diagnostics. Undo retains its established transient-selection clearing
policy. Native NoOwner paste verifies the original two selected IDs are retained;
incoming keys remain unselected. Broader primary/guard invariants have headless coverage.

## Actual MCP evidence

`leaf-mcp-authoring-01` passes **71 actual stdio requests**, starting with a blank
root and one original PNG through **25 construction commands**. No finished rig
is imported. Existing `copy_animation_keys`/`pasteAnimationKeys` use explicit
target owners to reproduce the five native reference mappings: original Bone 2,
different Bone 1, cross-animation Bone 2, and Slot/Region pose Bone 4. Each matches
nominal Frames 50/70 and values 3/14, with exact Undo/Redo.

Two negative atomic cases preserve complete state and redo. Copy is read-only;
12 evaluated poses, exact Save/reopen with empty history and unchanged input PNG
are retained, and the owned child is closed. This headless study neither selects
items in Peachy nor bridges a live native window. It does not verify automatic
native target resolution, new runtime compatibility or full Spine parity.

The public [Leaf Clipboard Lab](../../examples/peach-bot/leaf-clipboard-bot.peachy)
has SHA256 `5d432da2d6773ada1c8c0499a29c85207d689420fa817330104acd7efeddde6d`.
Its [single paste command](../../examples/peach-bot/leaf-clipboard-paste-commands.json),
extracted from the last actual MCP request, pastes the unchanged raw leaf payload
to Bone 4 in animation 1 at 50/60 seconds. The command file SHA256 is
`6281979fc6c61f9c6b41054ceb3afb1c8b65d356ee55df9915996c0a4093c881`.
MCP binary SHA256 remains
`0f659abb43c80d6dd4c502c35e91284848a9851947e13c682afdef290879d112`.

## Final source and bounded native acceptance

`leaf-focused-checks-04` passes Leaf 10, Slot 46, Draw-order 9 and legacy clipboard
15, plus formatting, all-target Clippy, workspace build and seven-crate architecture.
Checks03 also pass group 13, Dopesheet 29, Graph 46, Curves 52, Slider 35 and Deform
28. Filters overlap; these are not a full-suite total. Detailed native replay uses Desktop SHA256
`58255481712c76ecd655ffd1fc1d1e917239a5f083ba2b93a690a5cf194a4c19`.
Final checks06 repeat Leaf 10, formatting, all-target Clippy and workspace build
after simplifying five user-facing error strings. Final Desktop SHA256 is
`7edd697b394f786554d0868e45fdae314e225566a5939464076b798914450ba1`.
Success paths/data/routing are unchanged; the new error wording has no separate
native replay claim.
MCP remains `0f659abb…9d112` and CLI remains `a414e845…18ab6`; MCP71 is still the
final executed headless study. No new external-runtime study is claimed.

Actual native replay passes all five contexts in the table, including the visible
Animate `DrawOrderSlot:1` row and true Region. Each complete typed Document equals
its corresponding MCP case. NoOwner Paste retains the original two IDs; direct
Paste→Cut removes originals and two Undo operations restore baseline. Other-Bone,
cross-animation and Slot cases each Undo exactly. Region selection prunes unrelated
Driver keys, retains timeline owner Bone 4 and shows Bone properties. Extra unkeyed
rows, all-seven-row presentation and single-owner-header styling remain differences.
Native Undo is verified here; Undo/Redo is verified by MCP/headless tests.

The final Region/clip1 case, saved with the same baseline name, is byte-identical
to the public MCP lab (`5d432da2…eddde6d`). A fresh process reopens the complete typed
Document exactly, with empty Undo/Redo/clipboard and one image; old OS Paste cannot
revive the source receipt. Three OS screenshots were inspected. Both final owned
Peachy processes and all owned reference/MCP processes are closed. The parent group
checkpoint supplies prior minimum-window evidence; this slice adds no new native
minimum, inactive/pending or constraint-remap claim.

## Lifecycle

`leaf-cleanup.json` records **163 enumerated retired task files**, all verified
absent: duplicate saved scratch, the task-owned Spine clone, redundant captures/
exports/failed-check logs and bytecode. Both reassigned native-control helpers,
the export helper and both evidence builders reached their end condition and
are retired. No reclaimed-space amount is claimed.

Accepted 03/04/06 source reports/logs, 20 selected F12 witnesses and three OS-window
proofs/metadata, the MCP71 recipe/requests/scalars/poses and five reference goldens
with baseline/full source remain for active PR review and next compound Slot/
Bone/Tree/Offset calibration, until integration or supersession. The shared Cargo
target serves the ongoing goal. All owned processes are closed.

General Tree multi-selection, arbitrary target mapping, cross-project clipboard
interoperability, full Slot/Region property scope, Shift/Offset and Lock/Sync
remain outside this bounded contract. Previous Slider precision/interchange and
referenced-domain limitations are unchanged.
