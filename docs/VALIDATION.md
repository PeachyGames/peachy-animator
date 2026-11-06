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
