# Grouped timeline clipboard — November 15, 2026

**Bounded source/MCP/runtime and native acceptance verified**, including minimum
layout and native Save/fresh-process reopen on the unchanged Desktop binary.
The earlier `da1a1da` code push was explicitly partial at the user's request;
this follow-up closes its two native gates. Current
inventory is **64 commands /19 tools**, native v23 unchanged. This extends the
verified [ordinary multi-owner checkpoint](multi-owner-timeline.md) at `b45d18d`;
it does not implement Shift/Offset, general Tree multi-selection or a complete
Spine clipboard workflow.

## Installed-reference contract

Owned Spine 4.3.26 observations use the same original six-bone/two-region fixture,
one blue PNG and 60 FPS. In `clipboard`, driver Translate Y has Frames 5/25 with
values 3/14; follower Rotate has Frames 12/35 with values 11/−7. Copying those four
keys and pasting at Frame 50 uses one earliest-key origin, Frame 5:

| Destination context | Driver values 3/14 | Follower values 11/−7 |
| --- | --- | --- |
| Original owners | driver Frames 50/70 | follower Frames 57/80 |
| One different selected bone, source_parent | original driver Frames 50/70 | original follower Frames 57/80 |
| Two other parents selected in the measured order | source_parent Frames 50/70 | follower_parent Frames 57/80 |

The one-target case does not collapse a two-owner group onto one selected bone.
The two-parent result establishes this particular count/order mapping, not an
automatic general target-selection algorithm. A valid original-owner reselect
probe shows follower first/driver second in the Dopesheet, yet all three exported
animation dictionaries equal the original-owner golden: matching original-owner
identity wins over blindly reversing the map.
Cross-animation paste retains the same nominal timing and values.

The accepted `four-source-keys-paste50-cut` probe pastes at 50 and then cuts. Its
export retains only new driver 50/70 and follower 57/80 keys: the four original
selected source keys were removed. A separate leaf positive control agrees.
**Paste preserves the original selection** in these native cases; accepted
destination IDs are not an instruction to select pasted keys. Earlier unstable
selection attempts are excluded. Full exports and licensed native captures stay
in private `timeline/multi-owner/group-clipboard-reference/`.

The [official Dopesheet guide](https://esotericsoftware.com/spine-dopesheet),
[Keys guide](https://esotericsoftware.com/spine-keys) and
[changelog](https://esotericsoftware.com/spine-changelog), retrieved November 15,
provide selection/clipboard terminology. Exact mapping and retained selection
above come from black-box observations, not proprietary implementation.

## Shared data and editing boundary

`AnimationKeyGroupClipboard { entries }` contains existing Bone, IK, Slider,
Deform, Slot and DrawOrder leaf payloads. Entries follow the first canonical
source owner's appearance in the selected IDs. Every key is relative to the
same earliest selected time. At least one key across the group is zero;
individual entries may start later, including the first entry.

`copy_animation_key_group` returns that portable clipboard and a separate
`sourceOwners` receipt. Owner indices, source animation and project identity are
not embedded into the portable entries. Desktop source-owner bindings remain
editor-session state; the group OS envelope is version 3 while the established
version 2 single-owner envelope and old leaf commands remain unchanged.

`PasteAnimationKeyGroup { animation, time, clipboard, targets }` requires a complete
explicit map of `{ entry, target }`. Every entry is mapped exactly once; targets
must have distinct canonical owners and compatible kinds/layouts. Reordered maps
do not change the entry identity. No implicit current target or selection-order
guess is used by the engine/MCP API.

Shared preflight validates the complete group before mutation, including linked
Deform aliases, draw-order slot layout, time/value/curve finiteness and the
existing attachment-name skipping policy. A late invalid entry rolls back the
entire command/batch and preserves redo. Missing named attachment keys may skip;
hide/color keys retain their existing semantics and the global origin never
reanchors around surviving keys. All-skipped input preserves document/pending/
history; successful MCP `apply_commands` still advances its revision under the
existing protocol. Desktop skin/read-only-owner rules and shared Deform-layout
guards remain unchanged; grouping claims no new inactive-owner support.

The new group boundary requires objects at every object level, including leaf
key records, nested attachment references, layouts, targets and draw-order
offsets. Property and curve tokens follow their published string/object forms;
unknown/missing/null/wrong primitive forms and duplicate fields reject. Old
single-owner wire decoding remains separate. Raw duplicate-field regressions
run before JSON value materialization; actual SDK envelope tests make no claim
that transport parsing preserves duplicate fields.

## MCP checkpoint

`copy_animation_key_group` requires `expected_revision`, animation index and exact
typed source `keys`. It is read-only: no document, history, pending pose,
preferences or revision change. The response is `{ revision, clipboard,
sourceOwners }`. Apply returned entries through `pasteAnimationKeyGroup` with
explicit targets at the current revision; use existing mixed Delete/history/save
operations. The [MCP contract](../MCP.md) provides the exact wire examples.

Actual `group-mcp-authoring-02` passes **57 requests**, starting from a blank root,
the same **25 construction commands** and one original PNG. It proves a nonzero
first entry with the global zero only in the second, nine negative atomic cases,
same-owner/two-different-parent/cross-animation nominal frames and values matching
the accepted installed exports, exact Undo/Redo and Save/reopen, empty reopen
history and 12 retained evaluated poses. The owned child is closed; input image
bytes are unchanged.

The accepted saved SHA256 is
`d833624039bbb2596aa8d0a0434607faf4807b24291a86decb6a60575566fbb3`;
MCP binary SHA256 is
`0f659abb43c80d6dd4c502c35e91284848a9851947e13c682afdef290879d112`.
The public lab with those exact saved bytes is
[group-clipboard-bot.peachy](../../examples/peach-bot/group-clipboard-bot.peachy);
see the [review](../evidence/grouped-clipboard.html),
[scalar evidence](../evidence/grouped-clipboard.json) and
[explicit portable-paste example](../../examples/peach-bot/group-clipboard-paste-commands.json).
Four new MCP unit/two actual-stdio tests and final affected source/build checks
pass. Durable check06 also passes engine group 7/timeline 28 (overlapping) and
repeats the MCP cases on unchanged source. Bounded native authoring, minimum-window
and persistence acceptance now pass.

Separately, runtime 4.3.13 comparisons pass 104 poses for other-target output and
104 for cross-animation output: 208 total at unchanged 0.001 tolerance. Combined
checks cover 7,488 world values and 3,328 region coordinates, with world maxima
1.200913195020803e−5 /1.1076360095785276e−5 respectively and region maximum
1.52587890625e−5 for both. The fixtures contain no mesh/UV comparisons. CLI SHA256 is
`a414e845f0500c562e4ffaa579da64df8ca6a3e6379a6a2127aa7d9c6c618ab6`.
These separate geometry comparisons are not the 12 MCP pose queries, native input
acceptance or pixel comparisons. Final check05 passes Dopesheet 29, grouped
clipboard 13, leaf 15, fmt/all-target Clippy/workspace build and seven-crate
architecture. Check04's exact pointer assertion is preserved; a repaint barrier
refreshes stale marker geometry after two Undo operations change the extent.
Final Desktop SHA256 is
`87ee7d48cc92337937eef0e1cf44da517f986c1c3d2914f469df2d80339dee59`.
Final integrated workspace build07 after focused tests verifies Desktop/MCP/CLI
hashes matching the accepted check05/run02/runtime reports, without source changes.

## Native authoring results and remaining gates

The Desktop group path retains original source-owner bindings separately from
portable data and targets those original owners in the same editor session.
Final Desktop `87ee7d48…dee59` now passes actual four-key Copy into two entries.
The independent Cmd-toggle check retains its resulting five selected keys, and
moving the ruler to Frame 50 preserves selected key IDs/primary.

Paste creates driver Frames 50/70 and follower Frames 57/80 while retaining the four original
selected IDs and primary. The normal document-generation increment is not a
selection change. One Undo/Redo restores the complete expected documents; Undo
continues to clear transient key selection under the established policy. An
independent fresh Paste→Cut removes only the original four keys, leaves the four
incoming keys and restores the baseline after two Undo operations.

With another Tree bone selected, group Paste still addresses original bones 2/4
and leaves that Tree selection unchanged. With no shown selected keys, Cut/Delete
do not edit. One Undo recovers the baseline exactly. Cross-animation paste into
animation 1 at Frame 50 retains source-animation 0 bindings, produces the four
global-frame destinations, leaves clip 0 unchanged and undoes in one step.
Accepted incoming IDs are diagnostics rather than automatic selection changes.

These native authoring/history results pass separately from MCP/runtime checks.
A separate version 2 leaf control verifies Copy 2, original selection retained on
Paste, Cut removing originals and exact two-Undo recovery. This does not resolve
the no-Tree fallback gap below.

At historical `da1a1da`, the user chose **finish code checks and push first**.
Native authoring was accepted and its baseline restored/owned processes closed;
minimum-window and Save/fresh reopen were explicitly partial at that checkpoint.

The later follow-up uses the exact same Desktop
`87ee7d48cc92337937eef0e1cf44da517f986c1c3d2914f469df2d80339dee59`, without a source
change or rebuild. It opens baseline88e, copies four keys, seeks Frame 50 and
pastes to eight keys in animation 0. At 1120×740 every key rectangle lies inside
RowsClip, with the full original PNG loaded/fitted and an inspected OS-window
capture. Actual native CmdS leaves the document clean.

A fresh process reopens the **complete typed Document exactly equal** to the
preclose F12 document, with empty Undo/Redo/clipboard, one image and an inspected
OS-window capture. The public
[native saved result](../../examples/peach-bot/group-clipboard-native-result.peachy)
has SHA256 `194418f07951345927cbff588a9ba867a8b957713cfc16330200493d82cf0ae3`.
That case pastes into `clipboard` animation 0; the unchanged public MCPd833 case
pastes into `retime` animation 1. They are separate accepted results, not a
native-versus-MCP byte-equality comparison.

Saved-file versus preclose diagnostic JSON has 12 numeric spelling differences;
every differing value has identical f32 bits. Fresh native reload establishes
typed-data equality. **No raw JSON byte equality is claimed.** Both follow-up
owned processes and all earlier owned Spine/MCP children are verified closed.
These results close the bounded minimum/persistence gates without claiming
general Tree target mapping, pixel parity or the broader feature backlog.
The native pending baseline is empty; nonempty pending preservation has focused
event/core coverage only.

The code-push checkpoint retires **121 enumerated disposable task files** under
the existing `group-cleanup.json` lifecycle record, including generated bundles,
scratch, superseded/duplicate MCP outputs and redundant captures/helpers.
The native-completion follow-up retires **27 more enumerated task files** in
`group-native-completion-cleanup.json`. Its private saved result first matches
the new public native1944 project byte-for-byte; duplicate scratch, persistence
captures and logs then retire. Six accepted F12 JSON witnesses and two OS-window
proofs/metadata, selected earlier states and MCP02 recipes/requests/scalars remain
for active PR review until integration or supersession. Original reference goldens
remain for next Tree/Offset calibration. Both native-control helpers and the
evidence builder are explicitly reassigned to active leaf-clipboard implementation/
native QA, retiring after that unit is accepted or superseded. This retention
does not imply leaf implementation acceptance; grouped minimum/persistence gates
are complete and all owned processes are closed. Shared Cargo target supports
the ongoing goal. Logical cleanup counts are not reclaimed-space measurements.

General Tree multi-selection is absent, so native two-other-bone target mapping
is not implemented; explicit MCP maps can reproduce that reference outcome.
One-owner leaf copy from all-owner context without a Tree owner still lacks a
source receipt for fallback paste. This is a concrete follow-up, not a completed
native clipboard flow. Cross-project/system clipboard interoperability, arbitrary
mapping and read-only/inactive owner behavior remain bounded. Shift/Offset,
Lock/Sync, unsupported editable timeline families and the prior Slider lossy
interchange/precision failures remain open.
