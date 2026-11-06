# First-slice validation — November 5, 2026

Verified locally on macOS with Rust 1.99.1 and the locked dependencies:

| Check | Result |
| --- | --- |
| Formatting | `cargo fmt --all -- --check` passes |
| Rust tests | 11 pass across engine, adapters, CLI and file persistence |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` passes |
| Build | `cargo build --workspace` passes |
| Native interaction | Real key events verify key insertion, undo, redo, play/pause and frame stepping |
| Original fixture runtime | 9,180 sampled transform comparisons pass |
| Synthetic transform contract | 4,590 comparisons pass, including held keys, scale and full-turn rotation |
| Spine Editor interchange | 4.3.26 imports our JSON into a new native project, re-exports it, and our importer accepts that export |
| Re-exported runtime data | 9,180 comparisons pass after the Spine Editor round trip |
| Artist input preservation | 11 existing exports inspected without source changes; all report unsupported features |

Machine-readable public reports under `docs/evidence/` use only original fixtures.
The screenshot in `docs/editor.png` shows the running native app. The artist inputs,
browser profile, authentication state and local operating logs are excluded from Git.

The native smoke check restores the original document after exercising history.
Full save/reopen, backup verification, image copying and atlas export are also covered
by filesystem checks. A new project save uses a temporary file and atomic replacement;
an existing project receives a timestamped, verified backup first.

Hosted CI has not run. Its workflow is intentionally manual-only. Windows/Linux,
WebAssembly, native installers, pixel-by-pixel render equivalence and production artist
asset migration have not been verified or claimed.

## Mesh slice — November 5, 2026

The completed mesh slice passes 28 Rust tests, formatting, warnings-denied Clippy,
workspace build and the six-crate architecture guard. Checks include preserved
region geometry under reflection/scale, local subdivision and UV interpolation,
explicit fallback attachment addressing, invalid batch atomicity, gesture undo,
v1 migration, mesh save/reopen/backup/bundle export, serialized CLI edits and rejected
malformed or unsupported mesh semantics.

| Compatibility check | Result |
| --- | --- |
| Mesh demo, all three skins, two clips | 5,100 mesh coordinates; max error 0.000038147; 10,200 UV values with zero observed error |
| Reflected/nonuniform parent and animated scale contract | 2,550 mesh coordinates; max error 0.0000610352; 5,100 UV values with zero observed error |
| Original region fixtures, now all skins | 41,310 bone matrix/position comparisons pass |
| Mesh fixtures' bone transforms | 41,310 further comparisons pass |
| Installed Spine Editor 4.3.26 | Imports/re-exports all three meshes; vertex, UV, triangle, hull, edge and size data unchanged; our importer accepts output |
| Editor re-export in external runtime | 5,100 mesh coordinates, 10,200 UV values and 27,540 bone values pass |
| Shared packed bundle in external runtime | 5,100 mesh coordinates, 10,200 UV values and 27,540 bone values pass |
| Native mouse and keyboard events | Region conversion, subdivision, vertex drag with fixed UVs, single-step drag undo, redo and full document restoration pass |

These samples use 51 times per clip per skin, including endpoints and backwards
scrubbing. The harness asserts native meshes remain mesh attachments in the external
runtime, compares triangle order and hull count, and verifies the actual atlas PNG
size. Transform/vertex tolerance is 0.001; UV tolerance is 0.000001. These are finite
geometry checks, not a pixel-equivalence test.

Public mesh reports and the native interaction summary are under `docs/evidence/mesh-*`.
The original-only screenshot `docs/mesh-editor.png` shows an edited hand vertex. Local
logs and the installed editor's generated `.spine` project stay outside the repository.
The first native mouse attempt used direct-PID events without window routing and did
not click the button; the successful check used normal on-screen mouse routing in the
verified task-owned foreground window. Both attempts retained evidence and closed
only their own native test windows. No private artwork or existing user window changed.

Weights, deform animation, linked meshes, UV/hull editing, production artist migration,
pixel rendering equivalence, other operating systems and hosted CI remain unverified.

## UI alignment — November 5, 2026

The user made Spine's layout, controls and conventions an explicit requirement.
The initial alignment uses the public 4.3 UI/views/tools documentation and default
shortcut sheet. A task-owned instance of installed Spine 4.3.26 opened our original
reference project, but macOS refused its window screenshot (`could not create image
from window`). Exact visual comparison with that installed window remains pending;
this pass does not claim pixel equivalence. The reference process was closed.

Native mouse/key checks in our own task-owned review window verify:

- Tree and properties on the right, transform toolbar below the viewport, Setup/Animate
  at upper left, and Dopesheet below main tools only in Animate.
- C/V tool selection, D play/stop, R/F frame stepping, Shift ten-frame stepping,
  Cmd/Ctrl+R loop toggle without stepping, Space deselection, Cmd/Ctrl+L keying,
  arrows nudging instead of stepping, and F9 view hide/restore.
- Setup mesh vertex dragging preserves UVs and remains one undo step.
- History restores the original document after checks; no fixture is saved or changed.

All 28 existing Rust checks, formatting, warnings-denied Clippy, build and the six-crate
architecture guard pass. Domain and format crates are unchanged, so the previous
mesh/runtime results remain the data compatibility baseline. The native interaction
summary is `docs/evidence/ui-native.json`; current Setup/Animate screenshots are in
`docs/workspace-setup.png` and `docs/workspace-animate.png`. Earlier screenshots and
records describe earlier slices. The review window is closed after checking; a clean
interactive native handoff is retained separately.

Detailed styling, Graph/docking, full Tree entity types, all tool axes and selection
semantics, unkeyed poses and other controls remain required work in UI_PARITY.md.

## Tree and contextual selection — November 5, 2026

32 Rust tests pass, together with formatting, warnings-denied Clippy, workspace build
and the six-crate dependency guard. New checks cover placeholder/fallback projection,
slot filtering in draw order, search ancestors, stale selection clearing, triangle
picking under reflection, visibility bounds, isolated contextual edits and invalid
batch atomicity. The previous cache-clamping selection check is replaced by tests
that require invalid typed selections to clear instead of changing their edit target.

A task-owned native window verifies real mouse/key interactions for Tree search and
selection, setup region nudging/dragging with one-step undo, attachment picking, slot
selection when attachments are filtered, disabled bone edits for a selected slot,
temporary slot hiding, Cmd+H restoration, skin inspection versus activation,
inactive-skin edit refusal, setup attachment visibility, animation inspection versus
activation, playback and deselection. The original document is restored at the end.

The native working copy is saved and checked through four verified Save backups.
Region and slot edits are retained as separate original-art test projects; the CLI
reopens/exports them. Saved data and diagnostics are compared at the native model's
f32 precision, rather than requiring identical decimal spellings of the same value.
The first comparison exposed that harness distinction. A later native run exposed
macOS's standard Cmd+H Hide menu conflict; the app now relinquishes that menu shortcut
through safe, app-local AppKit calls. The completed mouse run verifies exact-PID focus
before every click so other open editor windows cannot receive the interaction.

| Runtime input, all three skins and two clips | Region values | Mesh values | Bone values |
| --- | --- | --- | --- |
| Unmodified mesh fixture | 28,560 | 5,100 | 27,540 |
| GUI-edited region project | 28,560 | 5,100 | 27,540 |
| GUI-edited slot visibility project | 26,928 | 5,100 | 27,540 |
| Spine Editor 4.3.26 re-export of GUI edits | 28,560 | 5,100 | 27,540 |
| Shared packed export of GUI edits | 28,560 | 5,100 | 27,540 |

Maximum region error is 0.000030518 units, mesh error 0.000038147, with a 0.001
acceptance tolerance. Local/packed mesh UV agreement retains zero observed error.
The installed editor imports/re-exports the GUI-edited project, and our importer
accepts that output. The oracle now asserts native regions remain actual region
attachments and compares their world corners by UV identity through public APIs.

Public original-only reports are `docs/evidence/tree-*`. Current screenshots are
`docs/tree-editor.png` and `docs/tree-animate.png`. Local working projects, native
logs, the installed editor's `.spine` output and recovery evidence remain outside Git.
The test windows are closed; a clean native handoff is retained separately.

Full visual parity, multi/range selection, alpha-aware picking, all Tree preferences,
rename/reparent/delete, mesh-wide transforms, unkeyed poses, cross-platform builds
and hosted CI remain outside this slice. Native format and export target are unchanged.

## Unkeyed pose and Auto Key slice — November 5, 2026

The expanded workspace passes **38 Rust tests**: desktop 3, CLI integration 3,
engine 20, I/O 4 and format adapter 8. Formatting, warnings-denied Clippy, workspace
build and all six dependency-layer checks pass locally. The changed core modules
were reviewed for filesystem/process/clock/GUI effects; none were introduced.
Hosted CI was not run.

Six new engine scenarios cover transient multi-bone poses, paired key groups,
setup-relative reflected scale, atomic keying failure, unified undo/redo, gesture
cancellation with redo preservation, saved-state separation, manual versus automatic
keying, return-to-sampled values and frame-change pose recovery. Native format remains
v2; no unkeyed state enters the serialized document.

Real native mouse and keyboard verification passes on the final build:

- Auto Key off preserves document/setup/key data while canvas, numeric and arrow
  input changes the evaluated pose; Save excludes pending values.
- Frame movement drops edits; Undo restores them at the new frame. K keying is one
  undoable transition, and Redo restores committed keys.
- Transform buttons/L key explicitly with Auto Key off. Tree keys consume one bone;
  K consumes edits on unselected bones too. Auto Key on keys changes immediately.
- A canvas pose gesture has one undo entry. Playback and clip/mode switches clear
  pending edits. Native save output matches the diagnostic document after f32
  normalization. The final screenshot shows independently drawn colored indicators.

The final GUI-edited native file was saved, exported, imported into installed Spine
Editor **4.3.26** as a fresh `.spine`, re-exported and reimported into our native model.
External runtime **4.3.13** comparisons pass for the GUI export, editor re-export and
shared packed bundle. Each case checks all three demo skins and both clips at 51
sample times per skin: **27,540 bone values**, **28,560 region coordinates**,
**5,100 mesh coordinates**, and **10,200 UV values**. Maximum sampled geometry error
is **0.00006103515625** units, below 0.001; UV error is zero. These are finite fixture
checks, not complete format, pixel or game-engine parity.

Committed evidence contains original fixtures only:
[keying-native.json](evidence/keying-native.json),
[edited runtime](evidence/keying-runtime-edited.json),
[editor round trip](evidence/keying-runtime-editor-roundtrip.json),
[packed runtime](evidence/keying-runtime-packed.json), and
[keying-editor.png](keying-editor.png).
Detailed native captures, working copies, verified backups, Spine logs and cleanup
records stay in the task's timestamped workspace backup directory. Both task-owned
review processes were closed; earlier intentional user handoffs were left alone.

A fresh read-only audit of the same 11 private artist exports finds **0/11** fully
supported; all input hashes match the initial audit. Weighted meshes occur in ten of
those reports, alongside linked meshes, constraints and unsupported timelines.
No artist file was edited or copied into Git. [Workflow milestones](WORKFLOWS.md)
track the remaining production-character and actual-game acceptance gates.

## Transform tools and axes slice — November 5, 2026

**41 Rust tests** pass, including new coordinate round trips under rotated/reflected/
nonuniform parents, restricted translation/scaling, singular parents, relative numeric
operators and full-turn keying. Unchanged display entry is an exact transform no-op.
Formatting, Clippy with warnings denied, workspace build and architecture checks pass.

Native mouse/keyboard verification passes on an original rig with a reflected,
nonuniform, rotated root: Scale X shortcut, active-tool arrows, .001 fine scale steps,
X-only handles, one-step gesture undo, Local/Parent/World view state, World scale using
local entry, *2 exactly once, World translation entry/handles, +360 animation entry,
Auto Key off, explicit scale keys, World rotation and saved native data.

Native QA caught and fixed handle picking after the drag threshold, double parsing of
relative expressions during focus loss, and unchanged World entry causing float drift
after Undo. A focus check also safely aborted a review without sending mouse input to
another process. All attempt records stay outside Git. The final review passes exact
setup-document restoration. Numeric widget state is scoped to item, axes, mode and frame.

Installed Spine 4.3.26 imported/re-exported the saved GUI edit. The external 4.3.13
runtime checks the GUI export and editor re-export across three skins/two clips,
including 27,540 bone values, 28,560 region coordinates and 5,100 mesh coordinates
per case, with the existing 0.001 tolerance. Original-only evidence:
[tools-native.json](evidence/tools-native.json),
[GUI runtime](evidence/tools-runtime-edited.json),
[editor runtime](evidence/tools-runtime-editor-roundtrip.json), and
[tools-editor.png](tools-editor.png).

The separate installed-editor reference window was on screen, but macOS refused its
capture. Public toolbar/axes images were inspected; pixel parity remains unverified.

## Selection history and groups slice — November 5, 2026

The workspace passes **44 Rust tests**: desktop 6, CLI integration 3, engine 23,
I/O 4 and adapter 8. Three new presentation-model scenarios cover selection history
branching/deselection, typed numbered groups, invalid references and bounded storage.
Formatting, warnings-denied Clippy, workspace build and the six-crate architecture
check pass locally. No document schema, export adapter or core dependency changed.

Final native verification passes Page Up/Down, remembered deselection, numbered
storage/recall (including 0 without UI zoom), bone/slot/skin/attachment identity,
read-only slot behavior, skin inspection without activation, independent document
Undo, empty group clearing and digit entry in text fields. It also verifies that
clicking a number stops playback and relative *2 entry applies once. The final
original document is restored exactly and is not dirty. See
[selection-native.json](evidence/selection-native.json) and
[selection-editor.png](selection-editor.png).

Input state now stays stable during playback and changes identity on deliberate
frame changes. Idle frames do not allocate edit state; groups reset on opening a
new document. Framework keyboard zoom is disabled locally so numbered group shortcuts
belong to the editor. Selection groups currently hold one typed item and are not
persisted in project files.

Review runs stopped on failed exact-PID foreground checks. The final run activates
and verifies the owned foreground PID through native process APIs, checks its normal
window, calibrates pointer coordinates against its F12 diagnostics before clicking,
and asserts each expected target type. Task-owned processes and temporary login
credentials are cleaned up; other applications and retained user handoffs are preserved.

## Animation transport slice — November 5, 2026

The workspace passes **46 Rust tests** (desktop 6, CLI 3, engine 25, I/O 4, adapter 8).
New core tests cover forward/backward wrapping, exact boundaries, multi-loop elapsed
time, invalid inputs, finite large-step results, duplicate key times, strict nearest-
key queries and last-key versus reserved clip duration. Formatting, warnings-denied
Clippy, workspace build and architecture checks pass. Changed core modules were
reviewed for filesystem/process/clock/GUI effects; none were introduced.

Final native verification passes W/S and Q/E, A/D, switching direction while playing,
reverse looping from zero, both nonlooping boundaries and reverse/stop buttons. Its
original rig reserves extra clip time after the last key, confirming E uses the key
rather than duration. Key navigation clears unkeyed edits, and Undo restores the pose
at the new key position. The document remains exactly unchanged and is not dirty.
See [transport-native.json](evidence/transport-native.json) and
[transport-editor.png](transport-editor.png).

Early review attempts stopped during hover calibration/startup window detection;
no clicks were sent before successful owned-window/foreground/pointer verification.
The final review process and temporary launch authorization were cleaned up. No
private artwork, game code, format schema or export adapter changed in this slice.
The geometry/export evidence from the transform slice remains applicable; this slice
changes transient transport state and uses the same deterministic pose evaluator.
Bounded nonlooping playback and current navigation scope remain explicit UI parity gaps.

## Interface iconography slice — November 5, 2026

All **46 existing Rust tests** pass. Formatting, warnings-denied Clippy, workspace
build and the six-crate architecture check pass locally. No domain model, engine,
format adapter, dependency or artist asset changed. No redundant icon-coordinate
unit tests were added for this presentation change.

Native verification on an original Peach Bot working copy confirms:

- Disabled Undo/Redo remain inert; enabled icons invoke the correct history actions.
- Toolbar Save and file-menu Save write the expected native document, preserve two
  verified backups, and restore the original project at the end.
- Views icons accompany every implemented entry. Setup availability, Tree/Dopesheet
  visibility, default workspace restore, and hide/restore continue to work.
- Native checkboxes retain view selection/keyboard semantics. Tree and Dopesheet
  headings reuse their menu icons. The same folder/disk drawings appear in the
  toolbar and file menu.
- A separate final visual review confirms the named disabled Undo tooltip, file
  menu, Views menu and dropdown cue on the final build. Only original artwork and
  our UI appear in committed captures.

An early custom visibility-row prototype failed native checks and was replaced by
an icon beside the existing native checkbox. Temporary traces and the prototype
state logic were removed. Review tooling waits for the initial rendered capture
before taking control coordinates; early startup/focus/calibration failures remain
in local evidence. All clicks require the owned foreground PID, normal window and
calibrated pointer coordinates.

Evidence:
[icons-native.json](evidence/icons-native.json),
[icons-visual.json](evidence/icons-visual.json),
[Views menu](icons-views-menu.png),
[file menu](icons-file-menu.png),
[disabled tooltip](icons-tooltip.png), and
[editor](icons-editor.png).

Public Spine titlebar/Views images informed the conventional visual cues; reference
images stay outside Git and no proprietary image/font is bundled. The new vectors
use a 16-point grid and egui's scaling. Exact pixel matching, other display scales,
other platforms and screen-reader operation were not verified. Accessible names and
native checkbox roles are attached in code. Hosted CI was not run.

## Parallel authoring slices — November 5, 2026

Three implementation agents worked in isolated worktrees on Dopesheet selection,
bone/slot renaming and animation rename/duplicate. The coordinator reviewed the
combined source, integrated shared dialogs/icons/focus rules and verified it.
Each lane ultimately used its own Cargo target directory. The coordinator cleaned
all six workspace packages and rebuilt the integrated source; lane results are not
used as a substitute for this check. After the Tree interaction fix, all **58 Rust
tests** pass (desktop 10, CLI 4, engine 32, I/O 4, adapter 8), along with formatting,
warnings-denied Clippy, workspace build and architecture checks. New core functions
were reviewed for filesystem, process, clock and GUI effects; none were introduced.

Tests cover names and all-skin references, unchanged evaluated geometry, exact owned
animation copies, native/Spine serialization, invalid-batch atomicity, preservation
of redo, stale typed selection/group pruning, exact fractional-key selection and
close-neighbor deletion safety. No schema or dependency changes were needed.

The integrated CLI applied five commands to the original Peach Bot: rename a bone,
rename a slot, rename an animation, duplicate it, then delete a key from the copy.
Native validation and a Spine JSON export/import/reexport preserve the result.
Packed export succeeds. An initial harness attempt omitted the required existing
bundle parent; the corrected final check retained its own output directory.
The artist sources remain unchanged.

The resulting packed export was loaded by the separately licensed external Spine
4.3 runtime across three animations and three skins. **41,310** world-component and
**48,960** region-coordinate comparisons pass at tolerance 0.001. Maximum errors
were 0.000027367 and 0.000045777 respectively. This fixture contains regions; mesh
naming preservation is covered by the engine scene tests, and prior mesh geometry
oracle evidence remains applicable. See [runtime evidence](evidence/authoring-runtime.json)
and [CLI evidence](evidence/authoring-interop.json).

Native review exposed the Tree auto-scroll moving a clicked row before its second
click. Tree-origin selection now leaves the visible row in place; viewport/history
selection still uses the reveal path. Several later runner attempts exposed unreliable synthetic text/Select All input;
a focused probe established that the app received the unchanged draft. The final
runner uses ordinary OS keyboard events for literal input, explicit field focus,
bounded plain-key clearing and rendered-state synchronization. Function-key captures
remain targeted to the owned PID. Each action group starts with owned foreground/
normal-window verification; all mouse coordinates are calibrated. Temporary traces
were removed. Failed attempts and captures remain in local task evidence.

The bounded scope and remaining UI differences are recorded in UI_PARITY.md and
docs/slices/. Exact installed-editor duplication autoactivation, other display
scales/platforms and screen-reader operation remain unverified. Native `.spine` and
binary `.skel` support are unchanged and remain unavailable. Hosted CI was not run.


Native acceptance confirms bone/slot/animation naming, inline validation and cancel,
animation copying without implicit activation, exact key selection independent of
the playhead, Cmd/Ctrl toggling, focused deletion, toolbar deletion, double-click key
deletion and one-step undo/redo. Two verified native save backups are retained; the
working document and saved file were restored. See
[feature acceptance](evidence/authoring-native.json). The long run stopped at its last
Tree double-click assertion; a separate focused probe with an explicit pointer move
and two ordinary clicks confirms that Rename opens and the row stays in place. See
[Tree evidence](evidence/authoring-tree.json). Acceptance is recorded by feature;
there was no claim that the long automation script passed uninterrupted. Some Tree
selection steps needed a bounded retry after verified activation, so exact first-
click behavior across host focus transitions remains a manual comparison gate.

Captures: [editor](authoring-editor.png), [selected keys](authoring-keys.png),
[Rename](authoring-rename.png), and [Duplicate](authoring-duplicate.png).
All review processes and temporary login authorizations were cleaned up; earlier
user windows were preserved. No new interactive handoff remains.


## Dopesheet retiming and clipboard — November 5, 2026

Three lanes implemented pure timeline commands, key dragging and clipboard routing
in isolated worktrees with separate Cargo targets. The integrated source passes
**78 Rust tests** (desktop 18, CLI 4, engine 44, I/O 4, adapter 8), formatting,
warnings-denied Clippy, workspace build and the six-crate dependency guard.
No native schema or dependency change was required. Core transitions were reviewed
for platform/I/O effects; the OS clipboard remains in the desktop shell.

New cases cover simultaneous moves, overlap/collision replacement, exact fractional
neighbors, curve/value preservation, malformed clipboard/offset rejection, atomic
batches, baseline preview rollback, redo preservation and session clipboard routing.
SetKey, SetPose curve lookup, DeleteKey and curve-inspector lookup prefer exact keys;
ambiguous near-key edits fail instead of targeting an arbitrary neighbor.

The CLI applies six edits to our original fixture, including moving groups, extending
native duration, duplicate/paste and editing one of two very close fractional keys.
Native validation, packed export and export/import comparison pass: **174 pose
samples**, maximum round-trip error 8.3819032e-9. Source input bytes are unchanged.
The external separately licensed Spine 4.3 runtime compares **27,540 world components**
and **32,640 region coordinates** across two clips and three skins. Maximum errors
are 0.000031082 and 0.000061036 at tolerance 0.001. This fixture has no meshes; prior
mesh geometry evidence remains the baseline. See
[CLI evidence](evidence/timeline-interop.json) and
[runtime evidence](evidence/timeline-runtime.json).

Native feature checks use only original Peach Bot copies in verified task-owned
foreground windows. Collision previews restore overwritten keys when moved away,
Escape restores the baseline, group retiming is one undo/redo step and Shift keeps
fractional times. A longer run stopped on foreground loss and refused further
input. The clipboard run confirms native Copy/Paste events, toolbar Cut/Paste,
Undo selection clearing, inert Delete after Undo, cross-bone paste, foreign-text
rejection, correct saved documents and original clipboard restoration.

Acceptance is recorded by feature, not as an uninterrupted long-run pass. Splitting
the clipboard run removed one save but initially retained a three-backup assertion;
its feature assertions passed before that harness error. An independent file audit
verifies the actual two readable backups and restored original file. Failed attempts
and recovery logs remain outside Git. See
[drag checks](evidence/timeline-drag-native.json),
[clipboard checks](evidence/timeline-clipboard-native.json) and
[save audit](evidence/timeline-save-audit.json).

Captures: [clipboard selection](timeline-clipboard.png) and
[editor](timeline-editor.png). Full selection boxes, Select All, all-bone rows,
Graph, advanced curve reshaping, shrinking duration, extended nonlooping playback,
other platforms and hosted CI remain unverified or pending.

The extra native run for committed fractional-key clicks, modifier duplication and
Save during a preview stopped safely during initial pointer calibration after
foreground loss. Those additional native comparisons remain pending; pure command/
gesture tests and source review are not presented as native interaction evidence.
All nine task-owned review processes have exited. Nine cleanup records confirm no
temporary authorization, private-key file or temporary command directory remains.
No new interactive handoff is retained; existing user windows were preserved.


## Selection expansion and row controls — November 5, 2026

The integrated selection slice passes **93 Rust tests** (desktop 33, CLI 4, engine
44, I/O 4, adapter 8), formatting, warnings-denied Clippy, workspace build and the
six-crate architecture guard. New checks cover explicit expansion phases, singleton
and multi-row selections, exact close fractional keys, context/stale pruning,
Delete/Undo scope, repeat/release handling, action order, text/modal priority,
window focus loss and cancellation of a live preview to its original selection,
document, pending pose and redo branch.

Review caught a destructive event-order issue before integration: Delete followed
by Select All in one input batch used to expand first. Deletion now belongs to the
ordered timeline stream, with tests for both event orders. Global history shortcuts
remain outside it; mixed same-frame Undo/Select All ordering is a known selection
result gap, documented in UI_PARITY.md.

A complete native run on an original-only disposable nine-key fixture passes:
1 → 4 → 9 → 9 selection counts; matching Copy counts; all three row tool mappings;
unchanged document, selected bone/keys and playhead; stable row geometry from empty
through expanded selection; expanded deletion/one-step Undo; empty-selection Select
All without consuming redo; and Cmd+A in a text field without timeline expansion or
backward playback. The working document and complete original system clipboard are
restored. Clipboard preservation keeps all item flavors in memory and only restores
from a complete flavor snapshot verified as this run's explicit copy; a newer
external copy is preserved.

See [native acceptance](evidence/timeline-selection-native.json) and
[expanded selection](timeline-selection-editor.png). The task-owned review process and
fresh-login authorization were cleaned up; no interactive handoff remains.
The engine and adapters are unchanged, so the preceding timeline runtime/round-trip
evidence remains applicable. This slice does not claim all-bone timelines, marquee,
Graph, pixel parity, other platforms or hosted CI.

## Key-derived duration and native v3 — November 5, 2026

The integrated source passes **115 Rust tests** (desktop 37, CLI 4, engine 54,
I/O 10, adapter 10), formatting, warnings-denied Clippy, workspace build and all
six architecture checks. New semantic tests cover shrinking across tracks/bones,
Undo/Redo and cancelled collision previews, pending poses and redo preservation,
empty/frame-zero/tiny clips, beyond-end authoring, large finite cursor arithmetic,
strict v3 decoding, and safe last-key deletion queries. Shared desktop fixtures
replace the duplicate clipboard-only constructor.

Native v3 removes stored duration. Migration golden data was captured from our
unchanged v2 exporter before modifying it. Tests compare the entire migrated
export with that original golden, including reserved holds, all-empty clips,
empty root tracks, final unused Bezier normalization and Stepped retention.
Malformed legacy bounds, schema/curve/order errors and stale v3 duration fields
are rejected. Load leaves source bytes unchanged; explicit Save keeps an exact
verified backup. The 4 → 6 → delete 6 → 4 case survives save/reopen.

The CLI performs ten commands on our original mesh fixture, producing empty,
frame-zero, tiny and explicit-hold clips, plus a moved key on a shorter track.
Native validation, packed export and import/re-export pass **333 pose samples**,
maximum round-trip error 4.1611888e-9. Both original inputs and the legacy fixture
remain byte-for-byte unchanged. The native v3 output contains no duration fields.
See [CLI acceptance](evidence/duration-interop.json).

The external Spine 4.3 runtime checks exact clip ends and geometry across six
clips, three skins and 52 times per clip/skin, including the end plus one second
and backwards sample order. **84,240** world components, **87,360** region
coordinates, **15,600** mesh coordinates and **31,200** UV values pass. Maximum
world/region/mesh errors are 0.000027367, 0.000030518 and 0.000038147 at tolerance
0.001; UV error is zero. A further **3,744** world components pass for legacy
migration cases, maximum error 0.000001206. See
[runtime acceptance](evidence/duration-runtime.json) and
[legacy runtime acceptance](evidence/duration-legacy-runtime.json).

Installed Spine Editor **4.3.26** imports the original duration fixture into a new
`.spine` project and re-exports it. Our importer accepts the result; empty clips
stay empty, frame-zero keys remain, and tiny/ordinary clip ends are retained.
The same external geometry/end checks pass on the editor's re-export. This is a
black-box CLI/data check; it does not establish installed-editor UI/playback parity.
The `.spine` project and raw CLI logs stay outside Git. See
[editor round-trip acceptance](evidence/duration-spine-editor.json).

Two complete native runs verify real pointer/keyboard behavior:

- Empty playback remains finite with Loop on/off. Seeking and posing beyond the
  end do not add keys or dirty the document. Save excludes the pending pose;
  explicit keying extends the end, and Undo/Redo restores it without moving the
  cursor. Three native save backups are retained.
- Last-key retiming shrinks the end while chart geometry stays frozen. Escape
  restores it; Save during preview cancels before writing. Modifier duplication
  and fractional-key clicks pass the native checks left pending in the earlier
  timeline batch. Delete shrinks to zero without clamping the cursor; history
  restores the expected key/end. Two further save backups are retained.

Both native working documents/files and original system clipboards are restored.
Task-owned review/CLI processes exited and all temporary recovery authorizations
were removed; existing user windows were preserved. No interactive handoff remains.
See [empty authoring](evidence/duration-empty-native.json),
[retiming](evidence/duration-retime-native.json),
[keyed pose](duration-keyed.png) and [shortened clip](duration-retimed.png).

Nonlooping playback is still bounded to the last key; exact Spine empty-playback
UI behavior, extended transport, timeline pan/zoom/ranges, animation creation UI,
other platforms and hosted CI remain pending. Weighted meshes, constraints and the
other production import blockers remain separate work; native v3 does not claim
that an existing artist character is now fully supported.

## Weighted meshes and native v4 — November 6, 2026 UTC

The weighted-slice baseline at `4687dd7` passes **140 Rust tests**: desktop **43**,
CLI **4**, engine **63**, I/O **15**, and adapter **15**. Formatting, warnings-denied workspace Clippy,
workspace build and the six-crate architecture guard pass. This records the weighted
import/evaluation/inspection slice; Bind and weight-authoring work is not included.
Earlier dated results above remain the acceptance record for their own slices.

Semantic checks cover more than four influences, repeated bones, zero weights,
positive subnormal and rounded nonunit totals, inactive-skin bone references,
strict tagged vertex modes, reflected/nonuniform/posed geometry, atomic weighted
edit rejection and preserved pose/history/redo. Packed counts and bone indices
accept mathematically integral JSON float tokens; fractional, negative, overflow,
truncated and trailing cases stay rejected. Exact integer maxima and the exclusive
floating index bound have separate checks.

Native v1/v2/v3 migration reaches v4 through strict validation before adding legacy
holds. The old v2 exporter golden still matches exactly. Legacy v3 mesh loading
leaves source bytes unchanged; explicit save backs up the original, writes tagged
vertices, and reopens correctly, including weighted data. Unknown fields and wrong
version-specific vertex shapes remain errors.

The CLI applies four commands, validates native v4, preserves influences through
bone/slot rename and JSON round trips, and compares **168 poses** with maximum
round-trip error **4.1611887e-9**. The legacy v3 export golden matches exactly and
both source inputs remain unchanged. See [CLI acceptance](evidence/weighted-interop.json).

Three original inputs are compared against the separately supplied Spine 4.3
runtime at 52 times per clip/skin, including ends, beyond-end holds and reverse
sample order:

| Original fixture | Weighted coordinate comparisons | Maximum mesh error |
| --- | ---: | ---: |
| Weighted demo | 3,120 | 0.000061035156 |
| Weighted transform contract | 1,560 | 0.000068664551 |
| Repeated-influence contract | 3,120 | 0.000061035156 |
| Total | **7,800** | **0.000068664551** |

The world-coordinate tolerance remains **0.001**; UV error is zero. The harness
also compares region/bone geometry, triangle order and hulls. Fixture hashes match
the reports and sources remain unchanged. See [demo](evidence/weighted-runtime.json),
[transform contract](evidence/weighted-contract-runtime.json), and
[repeated entries](evidence/weighted-repeated-runtime.json). The separate
[edited-data report](evidence/weighted-edited-runtime.json) verifies the renamed
native document against its own export.

Installed Spine Editor **4.3.26** imports and re-exports the demo and transform
contract, and our importer accepts both. Positive weights and nonunit sums survive
at f32 precision, but the editor drops a zero entry, reorders influences and changes
local bind coordinates by up to **0.0047683716**. The external oracle comparison
against the **original pre-editor native data fails** at the unchanged 0.001 tolerance
(observed error **0.0012054443**). Importing the actual editor output into a new
native document and evaluating that data against the runtime passes. This is not
lossless original-data round-trip acceptance. The stage responsible for coordinate
quantization is unproven and no documented precision override was found.

Controlled repeated/order probes also distinguish runtime acceptance from stricter
editor input behavior. The adapter continues preserving order, repeats, zeros and
raw totals; no silent merge/prune/normalization was introduced to make a probe pass.
See [editor round-trip acceptance](evidence/weighted-editor-roundtrip.json).

The accepted native check verifies posed weighted inspection and finite handles,
read-only drag/nudge/numeric/subdivision guards, then restored bone tools, keying,
Undo/Redo and **two** native-save backups. Unweighted setup dragging, subdivision
and history still pass. The original working document and system clipboard are
restored. See [native acceptance](evidence/weighted-native.json),
[weighted inspection](weighted-editor.png) and
[unweighted regression](weighted-unweighted-regression.png).

The read-only production audit covers the same **11** inputs, all unchanged from
the prior audit. Weighted-mesh refusal reports fall **10 → 0** with no new inspection
issues after the integral-float fix. Fully supported characters remain **0/11**;
all 11 still report constraints. Identifying inputs, artwork and detailed diagnostics
remain outside Git. See [redacted audit](evidence/weighted-artist-summary.json).

Cleanup verifies 13 task cleanup records: all **28** task-owned native/CLI processes
have exited, temporary authorizations/private-key files and temporary command
folders are absent, and no interactive handoff remains. Failed attempts remain in
local task evidence rather than being counted as accepted runs.

Weight authoring, linked/deform meshes, constraints, full Weights UI, actual
production-character/game-engine acceptance, other platforms and hosted CI remain
pending. These finite samples establish bounded data/evaluation and native inspection
behavior, not full Spine compatibility or pixel equivalence.

## Initial Bind and Remove-all — November 6, 2026 UTC

This checkpoint extends the weighted import/evaluation/inspection slice above; its
recorded **140-test** result remains unchanged. The combined binding slice passes
**162 Rust tests**: desktop **52**, CLI **4**, engine **76**, I/O **15**, adapter
**15**. Formatting, warnings-denied workspace Clippy, workspace build and the
six-crate architecture guard pass. The engine total includes **13 binding tests**.

Initial Bind creates one weight-1 influence per vertex. Remove-all preserves the
current captured shape through slot-local positions. Checks cover reflected and
nonuniform transforms, pending posed locals/time, exact history restoration,
required inverse selection, invalid/no-op pose validation, singular transforms,
finite-coordinate precision loss and command/batch atomicity. Residuals greater
than **0.001** reject; the tolerance was not widened and no normalization was added.

Malformed intermediate Batch hierarchies, including an invalid parent added before
Bind or Unbind, are rejected before world-pose evaluation rather than panicking.
These cases extend the existing atomicity test without changing the 162-test total
or the accepted valid binding geometry.
Hardening commit `5bf006c` passes all 162 tests and the required format, Clippy,
build and architecture checks again. It affects invalid intermediate hierarchies;
the recorded valid native, CLI and runtime checks remain applicable, without a
claim that the full native workflow was rerun after this guard.

[CLI acceptance](evidence/binding-cli.json) passes **271 scene comparisons** and
**29,726 vertex coordinates**, with maximum vertex error **0.000030517578125**
and zero UV error. Its nine expected rejections include singular Bind/Unbind,
already-weighted initial Bind, invalid pose/no-op/bone-batch cases, two finite
precision-loss cases and output overwrite refusal. **19 source files, plus retained
baselines, remain byte-for-byte unchanged.** CLI files are new outputs; desktop Save/backup
and transient Session behavior are separate checks.

Three original edited results are compared with the separately supplied Spine 4.3
runtime across 52 samples per clip/skin, including zero ends and held poses:

| Result | Bone values | Region coordinates | Mesh coordinates | Weighted coordinates | UV values | Maximum mesh error |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Setup-bound | 28,080 | 29,120 | 5,200 | 3,120 | 10,400 | 0.000030517578125 |
| Posed-bound | 28,080 | 29,120 | 5,200 | 3,120 | 10,400 | 0.000030517578125 |
| Removed bindings | 28,080 | 29,120 | 5,200 | 0 | 10,400 | 0.000057220458984375 |
| Total | **84,240** | **87,360** | **15,600** | **6,240** | **31,200** | **0.000057220458984375** |

The world-coordinate tolerance remains **0.001**, and UV error is zero. Each result
is compared against its own edited export, not against a former binding's behavior
at every animation pose. See [Setup](evidence/binding-setup-runtime.json),
[posed](evidence/binding-posed-runtime.json), and
[removed](evidence/binding-removed-runtime.json).

Installed Spine Editor **4.3.26** imports/re-exports the Setup-bound and
removed-contract inputs; our adapter accepts both outputs and both source hashes
remain unchanged. This is CLI interchange acceptance, not lossless original-data
comparison, interactive Weights observation or native `.spine` support in our app.
See [editor interchange](evidence/binding-spine-editor.json).

The separate full native run passes G/C/right-click and vertex retention, F9-to-G
reopening, Setup Remove-all/Bind with zero observed shape error, typed weight-1
influences, two verified Save backups and Undo/Redo. Animate Remove-all preserves
pending locals and time with zero observed shape error; Escape exits Bind untouched.
The first backup matches the initial working-copy bytes exactly; the second matches
the bound document at f32 precision.
The original system clipboard is restored. The working document/file is restored
semantically at f32 precision through canonical Save; this is not a claim that its
final raw bytes equal the initial raw copy. That initial copy is retained exactly,
and the 19 repository input/baseline files remain unchanged. The owned process is
closed and its temporary authorization removed. See [native acceptance](evidence/binding-native.json)
and [Weights capture](binding-editor.png).

The full functional native run used `ca457f5`. Visual-check build `a891007` corrects
an explanatory properties hint; all 162 Rust tests and format/lint/build/architecture
checks pass again, and a separate
[focused visual run](evidence/binding-visual.json) verifies fully loaded original
artwork, the G tool/view, independent icon and bone colors, a 100% row, and the
corrected hint without changing its document/file. The screenshot is from this
visual-check build. This focused check is not counted as a second full functional run.

The Bind checkpoint's three cleanup records confirm all six task-owned processes
have exited, with no interactive handoff remaining. Earlier weighted-slice cleanup
counts above retain their separate historical scope.

Exact installed-editor docking, Direct/brush editing, additional bindings, Auto,
Update, broader production-character/game-engine acceptance, other platforms and
hosted CI remain pending. The new authoring commands do not change the prior
**0/11** fully supported production-character result or remove constraint blockers.
