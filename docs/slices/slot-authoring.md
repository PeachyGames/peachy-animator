# Slot attachment and RGBA authoring — reference contract

Updated November 14, 2026. Slot pose/key commands, native controls, typed
Dopesheet keys, RGBA Graph authoring and MCP are implemented on the feature
branch. **Native interaction acceptance, the insertion holdout and the final
45-command MCP run pass.** These checks establish the bounded slot workflow
below; broader Color-view and extreme-precision parity remain separate. See the
[review gallery](../evidence/slot-authoring.html),
[verification record](../evidence/slot-authoring.json) and earlier
[slot data/evaluation contract](slot-keys.md).

## Public behavior reference

The [4.3 release blog](https://esotericsoftware.com/blog/Spine-4.3-released)
demonstrates image swaps and shading fades in its slider examples. Real slot
authoring is a prerequisite for creating those kinds of animation, even before
slider constraints are implemented.

The [current Keys guide](https://esotericsoftware.com/spine-keys) establishes:

- Attachment choices are keyed from the slot's Tree key button; the attachment
  visibility control chooses the visible appearance.
- The slot color key button is beside its Tree color control. RGB and alpha
  are keyed together by default; Separate enables independent RGB/alpha rows.
- Auto Key records keyable changes. K, Key Edited, records changed properties.
- Green, orange and red key buttons indicate absent, pending and keyed states.
- Moving time clears pending poses; Undo can recover them at the new time when
  there was no redo branch. Key Shown uses Cmd/Ctrl+Shift+L.

The [Slots guide](https://esotericsoftware.com/spine-slots) distinguishes two
different forms of hiding: bone/slot visibility dots reduce editor clutter;
attachment choices are animation data. It also distinguishes animated slot
tint from the attachment's separate, non-keyable tint. Its color button shows
alpha on the left and opaque color on the right. A fade can reach zero alpha
before an attachment key hides the image; zero alpha alone does not remove its
drawing cost.

The [Attachments guide](https://esotericsoftware.com/spine-attachments)
likewise assigns visible-attachment selection to the slot. One slot shows one
attachment or none; changing appearance must not alter an unrelated bone or
be confused with a temporary slot visibility toggle.

### Color view scope and placement

The [published Slot Color guide](https://esotericsoftware.com/spine-slot-color)
describes an inline chooser that avoids opening a dialog, targets the owning
slot when an attachment is selected, and exposes HSV/RGB/alpha controls as its
size allows. However, that page still uses the older name and slot-only scope.

The [4.3 changelog](https://esotericsoftware.com/spine-changelog) records the
rename to Color and broader Setup targets. The actual installed 4.3.26 menu
also says Color; see [the update audit](spine-blog-audit-20261114.md).
Installed Spine 4.3.26 observations on November 14 establish the following
slot controls:

- Properties opens a modal Color chooser from the slot swatch, with an adjacent
  key button. The inline Color view is a separate control surface.
- Timeline labels are `Attach: <slot>` and `RGBA: <slot>`.
- The grouped RGBA Graph overlays red R, green G, blue B and purple A value
  curves on a frame axis. Its value axis is percent, normally 0–100; these are
  channel values, rather than the normalized transition progress used by Deform.

The current Peachy slice implements slot Properties color/key controls, the
inline slot chooser, slot rows and selected-slot RGBA Graph editing. Broader
Setup targets and full Color-view scope remain unverified and unimplemented;
renaming a slot-only chooser does not complete that view. Private reference
captures include `spine-color-picker.png` and `spine-slot-graph.png` under
`.local/training-spine-20261114/slot-authoring/`.

### Clipboard and missing attachment names

The [Keys clipboard reference](https://esotericsoftware.com/spine-keys#Clipboard-buttons)
defines Cmd/Ctrl+C, X and V, Delete, and cross-slot pasting of attachment/color
keys after selecting the destination. An
[October 2024 staff reply](https://esotericsoftware.com/forum/d/26995-can-i-copy-attchment-key-frame-and-paste-it-to-the-other)
reported that cross-slot attachment-key paste was not available then. The
installed 4.3.26 tests provide a current, bounded reference for that older reply.

The native positive control copied the known `hand.L` attachment key back to
its owning slot at 2.2 seconds and successfully exported the new key. Pasting
that key to `hand.R`, which lacked the `hand.L` placeholder, left the entire
exported JSON unchanged. This establishes a skipped unavailable named key,
rather than a failed clipboard operation in the positive control. Evidence is
in the private `paste-known-positive/` and `clipboard-missing-export/` exports
and their corresponding screenshots.

The shared engine now skips pasted named attachment keys when no skin contains
that placeholder on the target slot. Hidden-attachment keys and color keys
remain eligible and retain the clipboard's common time origin. That all-skins
lookup is the implemented policy; the native probe did not exhaust every skin
combination. Native data validation and import continue to permit nonempty
unresolved attachment names. Paste policy is not a new import restriction and
does not create artwork or infer a name mapping.

Peachy's native editor selects only the identities accepted by the shared engine
paste resolver. A skipped attachment key does not select an existing target
key at the same time; a subsequent Delete cannot remove that unrelated key.
Mixed paste retains eligible color/hide keys without shifting their origin.

### Measured precision and insertion limits

The [JSON format reference](https://esotericsoftware.com/spine-json-format)
describes RGBA strings as pairs of hex digits. It contains legacy timeline
syntax, so this is a precision hint rather than a complete 4.3 authoring
specification. A byte-oriented export does not prove that the editor rounds
an interpolated pose before keying or stores only byte precision internally.

Explicit native G-field entries of both `128.5` and `128.9` displayed `128`.
These two entries show truncation for that numeric entry path, rather than
ties-to-even rounding. The captures are `spine-color-fractional-entry.png` and
`spine-color-128point9.png`. This observation does not make sampled float
values, displayed fields and stored byte keys interchangeable. Peachy's real
native Properties dialog also accepts `128.9` as `128`; Cancel restores the
whole prior state. View-only fields and OK preserve untouched raw channels.
Exact sampled-color quantization ties are not exhaustively characterized by
these field-entry tests.

A fresh original 16-case native insertion probe used Key Shown with keys at
`[0, 0.4, 0.8]` seconds. Its 52 channel parameters matched an independent
300-step f32 calculation using absolute-coordinate finite differences. This
was followed by an independent held-out insertion at 0.6 seconds on the
nonzero-origin interval created by the first insertion. All 104 inferred
split indices matched across calibration and holdout. This establishes the
measured cases rather than every floating-point or curve-time configuration.
Private inputs, exports, analysis and captures use the `curve-insert-*` names
in the same local task directory.

The slot adapter now preserves flat-endpoint Bezier value offsets and their
time handles; three focused adapter tests pass. This change is confined to slot
timelines. A whole-group flat-Bezier-to-Stepped optimization was already
present in an import-only native control; it was not caused by key insertion.
Peachy's insertion therefore retains the raw curve kinds. A separate native
4.3 JSON import test with an original overridden fist name changed the
placeholder and cleared its key. That interchange gap remains open.

## Current Peachy model and shared boundary

Current native data has `AttachmentKey { time, attachment: Option<String> }`
and `ColorKey { time, color: [u8; 4], curves: [Curve; 4] }`. Attachment keys
hold a placeholder name, with `None` meaning hidden. Before a first key,
Setup applies; the last key holds. RGBA interpolation has one curve per
channel; existing validation requires all four channels to be Stepped together.
Setup setters remain separate from the following shared animation commands:

- `SetSlotAttachmentKey { animation, slot, key }` writes an explicit attachment
  key, including `None` to hide.
- `SetSlotColorKey { animation, slot, key }` writes an explicit full key with
  byte RGBA and all four curves. This is the raw key replacement boundary.
- `SetSlotColorCurves { animation, slot, time, curves: [Curve; 4] }` changes
  curves at an exact existing key while preserving RGBA bytes and time.
- `InsertSlotColorKey { animation, slot, time, color: [f32; 4] }` accepts an
  authored normalized float color. An exact existing key retains all four
  curves; an interior new key subdivides the neighboring transition before
  publication. Finite/representable validation and rollback stay in the engine.
  Native insertion calibration and the nonzero-origin holdout pass as above.

The native format remains v22. Sampled and pending colors retain unmultiplied
float channels; persistent key encoding is explicit. Existing fractional key
identities and their per-channel curves survive value edits. Interactive new
keys use the frame grid; raw commands retain their explicit exact times.

Typed timeline identities use `KeyOwner::Slot { slot }`, with distinct
`AnimationKey::SlotAttachment` and `AnimationKey::SlotColor` variants.
Both may exist on one slot at the same time; retiming and clipboard identity
retain that distinction. One slot clipboard can contain both row kinds
with a common relative-time origin. Cross-slot behavior must preserve copied
values according to the skip policy above; invalid commands/batches roll back.

MCP now publishes 45 strict shared commands, including the camelCase forms of
the four slot commands above and typed key/clipboard schemas. The read-only
`copy_animation_keys` tool returns the same clipboard as the native editor.
`evaluate_pose` returns `slotAttachments` and raw unmultiplied float
`slotColors`, including hidden slots, before attachment tint and display byte
rounding. Mutation, revision, validation and history stay in the shared engine;
save/reopen and export use the existing confined I/O boundary.

## Pending poses and native input

Transient attachment and color overrides live in `PoseEdits`, with Session
history. Missing override and explicitly hidden attachment are different
states. Sample/pending slot color is available independently of a draw:
draw color already includes attachment tint and may disappear when hidden.

Auto Key off leaves Document and Setup unchanged. Explicit property keying
keys only that property; global K keys all supported pending domains atomically.
Save/export exclude pending state. Timeline, playback and context changes retain
the shared clearing/recovery policy. Merely viewing, focusing or blurring a
rounded color field must not quantize values or consume redo.

Freeze concrete slot, selection, active skin, mode, animation, time, Auto Key,
pose snapshot and input owner during previews. Release commits one undo step;
Escape, focus/history/file/context/view changes cancel and block held re-entry.
Tree attachment controls must not become aliases for the editor-only slot dot.

RGBA Graph editing selects an exact color key and channel, displays all four
channel curves, and offers Linear/Stepped/Bezier, draggable tangents and finite
numeric handle values in percent. Sampling uses the engine's `curve_value`.
Flat-endpoint overshoot uses value-offset Beziers. Stepped applies to all four
channels; leaving grouped Stepped resets the other channels to Linear and
changes the chosen channel explicitly. Other channel curves, values and times
remain unchanged. A virtual exact-key picker and bounded interval rendering
preserve dense data without drawing every key. Copy/Cut/Paste/Delete use the
typed shared selection: color-only selection stays color-only, and a mixed
attachment/color selection retains both.

## Verified native and MCP workflow

Real native controls passed the following checks on original Peach Bot art:

- Properties modal channel entry `128.9` encodes `128`; Cancel restores the
  complete prior document/pending state. Pending attachment/color changes stay
  out of Save, and mixed attachment plus Color K commits in one undo step.
- RGBA Graph numeric entry authors an absolute R handle value of 80 percent.
  Tangent drag changes only the intended curve; one Undo/Redo restores the
  complete before/after documents. Numeric-field Escape retains slot, exact
  key and channel. Escape cancels a held tangent without held-pointer re-entry.
- Graph Copy and Dopesheet Paste/Delete affect color keys without aliasing
  coincident attachment keys, with clipboard Undo/Redo verified.
- Save and fresh-process reopen preserve the final document; the fresh process
  has no inherited undo stack. Actual playback advances without modifying the
  document. All owned reference/editor processes are closed.

The private `native-pending-report.json` and `native-graph-report.json` record
these results. The final native export also passed the external runtime check
in `native-runtime-final.json`.

The final actual stdio MCP run exposed 11 tools / 45 commands and authored,
saved and reopened the three-clip project. It exercised curve-preserving
insertion, mixed clipboard across clips, strict/atomic failures and Undo/Redo;
hidden RGB and per-channel curves survived. It returned 81 raw slot samples
and verified that all 15 source files retained their hashes. Its export passed
the external Spine 4.3 runtime across all three skins and clips: 42,120 bone
values, 49,032 region values and 24,516 color values, with maximum bone and
region errors `2.6534e-5` and `3.0518e-5`. The final native export produced the
same comparison counts and maximum errors. Private evidence is
`mcp-authoring-final-report.json`, `mcp-runtime-final.json` and
`native-runtime-final.json`.

Final focused checks pass; filters overlap and their counts must not be added:

| Check | Passing cases |
| --- | ---: |
| Engine slot | 37 |
| Desktop slot | 37 |
| Desktop Graph | 33 |
| Desktop clipboard | 15 |
| Numeric-field Escape | 2 |
| MCP schema | 6 |
| Actual stdio MCP | 4 |
| Spine slot-key adapter | 3 |

Formatting, all-targets Clippy, workspace build and architecture checks pass.

## Remaining limits

The insertion scan is calibrated to the measured ordinary f32 cases. Extreme
or adjacent times that overflow or collapse that scan use a stable subdivision
fallback; that fallback is finite and tested, but native equivalence outside
the calibrated range is not claimed. Numeric field truncation does not settle
every sampled-color quantization tie, and the persistent key model remains
byte RGBA while sampled/pending values are floats.

The native name-override import gap above remains open. Separate RGB/alpha
timelines, dark tint, full Color-view target coverage, sliders and general
multi-owner clipboard work remain distinct parity gates. This slot workflow
does not complete those features.

The feature remains subject to the repository's hosted CI gate before main
integration. Local verification is not a hosted CI pass; see
[the current continuation record](../NEXT-RUN.md) for the unregistered-workflow
condition and required integration gate.
