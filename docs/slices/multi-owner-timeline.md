# Multi-owner timeline — November 15, 2026

**Bounded ordinary multi-owner native/MCP/runtime checkpoint verified;
group clipboard and Shift/Offset remain open.** This
record separates reference results from implementation results. Current inventory
is **63 commands /18 tools**, with native v23 unchanged. Sync, general Tree
multi-selection and group clipboard/Shift/Offset remain outside acceptance.

## Reference and tutorial scope

The coordinator used installed **Spine 4.3.26** on an original six-bone,
two-region fixture with one original blue PNG, at **60 FPS**. Separate `retime`,
`clipboard` and `offset` animations isolate the operations. Licensed native
screenshots and full exports remain in the existing private tutorial task;
no reference artwork, interface assets or implementation source are published.

The retained Wind visual transcript shows grouped hair/head-cloth controls and
Dopesheet Offset at 10:00–21:30, grouped upper-garment timing at 21:30–25:30,
and multiple long-cloth controls at 32:00–39:30. Its local evidence is
`visual-transcripts/visual-action-transcript.txt`, lines 135–148, in the existing
training record. The [earlier recreation study](training-recreation.md) achieved
those mechanisms through explicit MCP keys; this slice addresses the demonstrated
multi-owner editing workflow, without claiming exact source timing or artwork.

Official sources retrieved November 15 provide terminology:
[Dopesheet selection](https://esotericsoftware.com/spine-dopesheet#Selection),
[clipboard/Shift/Offset](https://esotericsoftware.com/spine-keys#Manipulating-keys),
the [4.3 release](https://esotericsoftware.com/blog/Spine-4.3-released) and
[changelog](https://esotericsoftware.com/spine-changelog).
Public Shift/Offset descriptions are not substitutes for native calibration.

## Ordinary mixed-owner retime

In animation `retime`, the native Dopesheet displays `follower` Rotate and
`driver` Translate Y together. The valid selected set contains driver Frame 40
and follower Frames 5/20. An ordinary drag adds **5 frames** to those three keys.

| Track | Baseline frames | Frames after ordinary move |
| --- | --- | --- |
| driver Translate Y | 0, 10, **40**, 60 | 0, 10, **45**, 60 |
| follower Rotate | **5**, **20**, 45, 60 | **10**, **25**, 45, 60 |

Only the selected key times change. Their values remain unchanged: driver −4,
follower 0/35. Unselected keys, other properties and other animations remain
unchanged. The moved driver key and untouched follower key can both occupy
Frame 45 because their typed owners differ. This linear-key probe does not
establish Bezier-handle retiming or overwrite/collision rules within one owner.

One actual Undo restores **all animation data exactly** to the baseline. The
retained baseline and Undo JSON exports are also byte-identical in this case.

Evidence under private `timeline/multi-owner/`:

- `native-baseline/group-input.json` and `native-baseline-record.json`.
- `retime-mixed-plus5/group-input.json` and its record; screenshots
  `mixed-three-keys-selected.png` / `mixed-three-keys-moved5.png`.
- `retime-undo/group-input.json` and its record; `retime-after-undo.png`.

## Clipboard uses one earliest-key origin

In animation `clipboard`, driver Translate Y starts at Frames 5/25 with values
3/14; follower Rotate starts at Frames 12/35 with values 11/−7. Copying the selected
keys from both bones and pasting at **Frame 50**, onto the same original bones,
produces:

| Track | Original frames retained | New pasted frames |
| --- | --- | --- |
| driver Translate Y | 5, 25 | **50, 70** |
| follower Rotate | 12, 35 | **57, 80** |

The origin is the earliest key across the complete copied set: Frame 5.
Every copied key receives the same 45-frame translation. Each owner's internal
timing, the seven-frame difference between the first driver/follower keys, values
and property identities survive. The result is not independently anchored at 50
for each bone. The `retime` and `offset` animations remain unchanged.

Evidence: `clipboard-paste50/group-input.json`, its record,
`clipboard-all-selected.png` and `clipboard-paste-at50.png` in that private unit.
This establishes same-animation/same-bone identity paste only. Mapping to different
bones, changed selection order/count, incompatible properties, mixed non-bone
owners and cross-skeleton behavior remain outside this native probe.

## Shift: one selected property ripples on its own track

The clean, paused `shift-clean-driver-plus5` probe selects driver Translate Y at
Frame 10, with both driver/follower rows shown. Moving it +5 with Shift produces
driver Frames **0, 15, 45, 65**, from 0/10/40/60. Driver Frame 0 and all follower
Rotate keys/values remain unchanged. This establishes following-key ripple on
the selected property for this case, not a global shift of every shown owner.
Multi-owner Shift, other properties on the same bone, filters and modifier
precedence remain uncalibrated. No Shift implementation acceptance follows.

## Offset: wrapping and repeated movement without reselection

The `offset` animation has matching endpoint values at Frames 0/60. Driver
Translate Y starts at 0/20/40/60 with values 0/30/−10/0 and a Bezier first interval;
follower Rotate starts at 0/15/45/60 with values 0/−20/40/0. Both owners participate.

| Accepted reference step | Driver frames / endpoint value | Follower frames / endpoint value |
| --- | --- | --- |
| `offset-plus10`: +10 frames | 0, 10, 30, 50, 60 / **−5.000002** at 0/60 | 0, 10, 25, 55, 60 / **26.66667** at 0/60 |
| `offset-repeat-plus5`: +5 again, no selection change, total +15 | 0, 15, 35, 55, 60 / **−7.500001** at 0/60 | 0, 15, 30, 60 / **40** at 0/60 |

These are native exported decimal values; frame labels are nominal 60-FPS values.
The driver first interval's absolute Bezier controls change from
`[0.08333333,24,0.25,6]` to `[0.25,24,0.4166667,6]` after +10, then
`[0.3333333,24,0.5,6]` at total +15. The moved control times shift while their
Y values stay 24/6. This is not a calibrated Bezier seam-splitting algorithm.

The second move leaves **five driver keys and four follower keys**; old inserted
seam keys do not accumulate as new interior keys in this unchanged-selection
case. The result establishes remembered origins for this sequence without
establishing a universal selection-reset/cache policy.

The later `offset-reselect-plus5` export is retained as an unresolved observation:
driver has four keys at 0/20/40/60, while follower has five at 0/5/20/35/60. Their
endpoint values are −10 and 29.999996 respectively. This asymmetry does not justify
assuming that Cmd+A/reclick resets every track's original-key memory identically.
Exact per-track selection/memory behavior still needs discrimination.

Evidence is the corresponding `offset-plus10`, `offset-repeat-plus5` and
`offset-reselect-plus5` export folders/records in private `timeline/multi-owner/`.
Group Offset implementation and its native/MCP acceptance remain pending.

Three subsequent actual Undo operations restore all animation data exactly to
the pre-Offset export. The owned reference window is closed, with original bounds
and Setup mode restored. This closes the calibration session, not Offset's
implementation gate.

## Actual MCP ordinary-move study

Bounded all-owner Dopesheet rows, typed key selection and ordinary mixed movement
are implemented using the shared authored inventory. Existing single-owner
surfaces retain their own contracts. Read-only timeline descriptors do not become
editable IDs; unsupported groups cannot be called complete. Native acceptance
below covers the original two-bone scalar case, not every mixed domain.

The coordinator's final `mcp-authoring-03` run passes **41 requests**, including
**25 authored commands**, starting with one identity root and one original PNG.
Its ordinary mixed move matches the measured native frames/values. Five invalid
inventory queries preserve read-only state; Undo/Redo and Save/reopen are exact,
12 evaluated poses are retained, source inputs are unchanged and the owned child
is closed. Actual discovery verifies 63 commands/18 tools.

The public [Multi-owner Lab](../../examples/peach-bot/multi-owner-bot.peachy) has
SHA256 `88e6b67fdc058b4acb8bbf618dd1ebeb98fdbb3c765f24d301aebdf3d0c5c2cf`.
Its adjacent [25-command recipe](../../examples/peach-bot/multi-owner-authoring-commands.json)
creates the original six bones/two regions from a blank root and the one PNG;
it does not import a finished rig.

Baseline and moved exports each pass 104 external-runtime poses at unchanged
0.001 tolerance: 3,744 world values, 1,664 region coordinates, 832 color values,
104 full-order checks and 208 image checks each. Maximum world errors are
1.200913195020803e−5 and 1.167843909399835e−5 respectively; region maximum is
1.52587890625e−5 in both. These compare authored ordinary retiming, not Shift/Offset
or raster output. See [review](../evidence/multi-owner-timeline.html) and
[scalar evidence](../evidence/multi-owner-timeline.json).

Focused checks pass: six engine, two generation, 16 schema, three query and one
actual-stdio case. Final MCP binary SHA256 is
`93268b50871aa3a21fe68ef24c3bfbc98dce6f23dfc3199b580361300feeba1d`.
The headless report does not attest native UI; separate native evidence follows.
Group clipboard/Shift/Offset remain unaccepted features. Reference observations
above must not be presented as implemented controls or completed MCP features.

## Actual Peachy ordinary-move acceptance

Final Desktop `e1251a01…c70d1ee` passes actual Cmd-selection of three scalar keys
across bones 2/4 while Tree holds the animation. A +5 displayed-frame drag matches
the shared command using its accepted raw delta **0.083333320915699 seconds**;
one Undo restores the baseline and one Redo restores the moved document. The
displayed frame step is not an assertion that the raw delta equals exact 5/60.

A changed held preview followed by Escape restores the document, selected key set,
primary key and redo. Continued pointer movement while held stays blocked; release
and a fresh gesture work. The native baseline has **empty pending edits**.
Nonempty pending-pose preservation is covered by headless tests, not this native
replay. Cmd+A selects all eight represented keys; Delete/Undo recovers them exactly
and leaves the other clip unchanged.

Graph retains bone 4 while Tree holds the animation. Switching Y/Rotation retains
that owner; it does not manufacture an unkeyed channel. Clicking the driver owner
header prunes the shown rows. Space clear removes selection and Graph context
instead of falling back to a cached bone. At 1120×740 all eight key targets are
visible. Mixed-owner Copy explicitly refuses and preserves document, selected
keys/primary, redo and the internal clipboard.

Native Save and a fresh process reopen the exact full edited document with empty
history and one image. The task-owned retimed file SHA256 is
`8c2ba3768e5f925f895d4348dfa09a56e496eca673eba2c779fe2d4f8dc2510d`.
It is separate from the unchanged public baseline SHA256 above. Private witnesses
remain under `timeline/multi-owner/peachy/`; public artwork is original. The native
report passes, all owned native/MCP processes are closed, and reference bounds/
Setup mode are restored.

Final Desktop filters pass Dopesheet 28, Graph 46, Curves 51, Slot 42, Slider 34,
Deform 26, key-drag 4 and clipboard 15. These overlap. Final formatting,
all-target Clippy, workspace build and seven-crate architecture checks pass.

The checkpoint retires 79 disposable task files, including generated runtime
bundles, scratch projects, superseded MCP outputs, redundant native pictures and
closed-native input helpers/bytecode. Public examples preserve the canonical
project and original PNG. Selected final requests, recipes, scalars and exact
native-state/window witnesses remain for active PR review until integration or
supersession; original reference fixtures/exports remain for the next clipboard/
Offset studies. The continuing goal still needs the shared Cargo target.
No unrelated paths were removed and no reclaimed-capacity amount is claimed.

## Excluded attempts and remaining gates

The earlier unchanged `shift-driver-plus5` attempt had unstable input/playback
preconditions and is excluded. Reverse playback was visible during diagnosis,
but its causal role is not established. A mixed Shift attempt registered −10
rather than the requested +5 because its pointer baseline was not controlled; it
does not establish the requested-delta contract. A valid mixed Shift probe is
separate work.
An exact foreground guard stopped a focus interruption before any other-app
input; an interrupted attempt is not accepted operation evidence.

Ordinary movement now has separate native, actual MCP and runtime evidence.
The **next slice is group clipboard with explicit owner mapping and one shared
origin**, through both native controls and actual MCP requests, before Shift/Offset.
Existing one-owner clipboard behavior and mixed-Copy refusal do not complete it.

Mixed Shift ripple scope, Offset memory resets/nonzero loop bounds, Sync modes,
filter/lock behavior, fractional timing and general Bezier handling remain
calibration gates. Ordinary retiming must not be presented as Offset: the latter
includes distinct boundary/key-generation operations. No full Spine workflow,
pixel or runtime parity claim follows from these finite observations.
