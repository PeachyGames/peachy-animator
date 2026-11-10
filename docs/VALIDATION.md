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

## Additional binding and Direct — November 6, 2026 UTC

This is a separate extension of the historical 140-test weighted and 162-test
binding checkpoints above; those sections retain their original scope. The
combined Direct build at `694576a` passes **184 Rust tests**: desktop **63**, CLI
**4**, engine **87**, I/O **15**, adapter **15**, plus formatting, warnings-denied
workspace Clippy, workspace build and the six-crate architecture guard. The engine
includes **11 Direct tests** alongside the prior **13 binding tests**; the desktop
adds **11 Direct input/lifecycle regressions**. Hosted CI and other platforms are
not checked by this local macOS result.

Core cases cover initial A=1 → Add B=0 → Direct B=0.3/A=0.7, proportional known
vectors, nonunit selected-vertex normalization, signed-zero/order/position and
unselected-repeat preservation, subnormal recipients, retained underflow entries,
singular-but-finite Direct poses, required Add inverse and precision guards,
nonfinite candidates and malformed payloads/Batches. Fixed-base previews, failure,
cancel, one-step Undo/Redo, pending locals and no-op redo preservation are explicit
checks. Native v4 and the adapters are unchanged; normalization occurs only through
an explicit Direct edit of its selected vertex.

The reusable `scripts/check-direct-weights.py` imports the existing Bind harness
helpers for guarded files, command invocations, poses and roundtrips. The accepted
main-CLI run passes **10 success cases**, **19 expected failures**, **471 scene
comparisons** and **51,774 vertex coordinates**, with maximum vertex error
**0.000030517578125** and zero UV error. Optional Cargo execution passes the
**24 Bind/Direct Session tests**. All **19 original fixture files and retained
baselines remain byte-for-byte unchanged**. An existing evidence directory and
output inside original fixtures are refused without changing them.
See [CLI acceptance](evidence/direct-cli.json).

The cases retain both ordinary limb influence orders **[8,7]** and **[7,8]** without
sorting. Add leaves original/nonunit/repeated/zero entries untouched; Direct only
changes its selected vertex. A moved-bone case verifies intentional deformation
without allowing changes to unrelated vertices. Failures check the stated reason,
unchanged input and no published output, including sparse/repeated/zero-recipient
limits, existing-global Add bones, singular/precision/nonfinite cases, invalid
payloads, partially successful Batches and overwrite refusal.

All **eight bundled results** also pass the separately supplied Spine 4.3 runtime
oracle, with **52 samples per clip/skin**, exact ends, held poses and backward
scrubbing. Every result is compared against its own edited native document and
packed JSON/atlas/PNG; Direct is not assumed to preserve its original shape.

| Result | Weighted coordinates | Maximum mesh error |
| --- | ---: | ---: |
| Scratch A1/B0 → B0.3/A0.7 | 3,120 | 0.000030517578125 |
| Scratch moved-bone Direct | 3,120 | 0.000030517578125 |
| Scratch order [8,7] | 3,120 | 0.00006103515625 |
| Scratch order [7,8] | 3,120 | 0.0000400543212890625 |
| Imported weighted-bot Direct | 4,680 | 0.00006103515625 |
| Imported nonunit contract Direct | 3,120 | 0.00006866455078125 |
| Repeated/zero/order-preserving Add | 4,680 | 0.00006103515625 |
| Known proportional Direct vector | 3,120 | 0.000030517578125 |
| Total | **28,080** | **0.00006866455078125** |

The full runtime totals are **252,720 bone values**, **262,080 region coordinates**,
**46,800 mesh coordinates** and **93,600 UV comparisons**. Maximum world error is
**0.00005064427244860781**, maximum region error **0.0000762939453125**, and UV error
is zero. Tolerance remains **0.001**. The accepted-report CLI hash matches before
execution; CLI, native, standalone export, packed JSON, atlas, image, harness and
source-report hashes are unchanged afterward. Two concurrent jobs were used; all
eight synchronous Node children exit with code zero. No runtime implementation was
read and no GUI/source/target writes occur in this runtime check.
See [external-runtime acceptance](evidence/direct-runtime.json).

Installed Spine Editor **4.3.26** CLI imports/re-exports **seven of eight** Direct
results, and our adapter accepts all seven outputs. The repeated-influence Add
fixture fails with `Index -2 out of bounds for length 16` while passing the runtime
oracle. Both ordinary limb orders **[8,7]** and **[7,8]** pass editor interchange;
this does not establish a universal parent-first influence order. No data was
sorted, merged or normalized to obtain those results; every source hash is unchanged.

Canonicalization comparison of the seven editor outputs measures maximum bind-
coordinate change **0.00476837158203125** and positive-weight change
**0.0000044405460357666016** on weighted-bot Direct. Other measured positive weights
are unchanged or within one f32 ULP. Thus the earlier weighted fixtures' positive-
weight preservation is not generalized to Direct outputs. Import/re-export and
adapter acceptance prove neither lossless original-data roundtrips nor interactive
Spine Weights behavior. Original-source shape preservation through this editor
roundtrip is not claimed. The editor removes one to four zero entries per accepted
result; reimport can expose sparse targets that our current authoring guards do not
repair. Native/imported raw entries are not pruned by our codec to imitate that loss.
See [editor interchange and canonicalization](evidence/direct-spine-editor.json).

Headless UI regressions exercise actual egui input frames for numeric previews,
pointer-idle retention, Enter/Tab, rounded nonunit focus/blur, invalid text, text undo
and tool-key priority, application focus loss, real toolbar Save press/release,
owned-field Cmd/Ctrl+S, and held-slider Escape/Save/history followed by further
movement/release. Direct context and one pose snapshot remain frozen; canceled
input cannot return on stale blur or mouse-up. These checks pass in the combined
build; native text/pointer delivery is checked separately below.

The full native run at **`694576a`** passes on its first complete execution. It
starts with an unweighted mesh, binds A=hand.R (bone 8), adds B=forearm.R (bone 7),
and verifies all old influence entries plus a zero B entry on every vertex. Initial
Bind and Add show **zero observed world error** at the captured pose. Vertex **4**
then receives an actual typed **30% B / 70% A** result, with verified Save and
Undo/Redo. Numeric Escape and a real toolbar Save click cancel a live preview.
Held-slider Escape/Cmd+S, continued pointer movement and release produce no ghost
edit or history entry; a committed slider edit is one Undo/Redo step.

Animate Direct **45%** activates genuine Weights from Rotate and preserves the
exact pending **13-degree rotation**, locals and time, with no new keys. Save
excludes that pending pose. The working document/file is restored semantically at
f32 precision, and the original system clipboard is restored. **Five Save backups**
are audited: the first matches the retained initial raw copy exactly, the next
three match the 30% document at f32 precision, and the last matches the 45% document
at f32 precision. This is not a claim that final saved bytes equal the initial raw
copy. All **19 repository source files remain unchanged**. See
[full native acceptance](evidence/direct-native.json).

The [visual review](evidence/direct-visual.json) uses two captures from that same
accepted native run, verifying loaded original artwork, independent Weights
icon/bone colors, actual 30%/45% controls/data and the unkeyed Animate pose. It is
not a second full functional run or an interactive Spine UI comparison. See the
[Setup Direct capture](direct-editor.png) and [posed Direct capture](direct-posed.png).
The cleanup audit confirms **16 recorded native/editor CLI processes exited**;
the eight Node oracle jobs above were separately waited/reaped. Two fresh-launch
cleanup records retain no temporary authorization, keys, command directories or
interactive handoff.

Exact Weights docking, pixel and full interaction parity, brushes, locks,
multi-selection, Update, other platforms, hosted CI and broader production/game-
engine acceptance remain pending. No production inputs are changed or newly
claimed supported: **0/11** remain fully supported, all 11 still require constraints.

## Shared context and pose-preview preparation — November 6, 2026 UTC

Native startup and every headless interaction fixture now use the same editor
context configuration. The extracted theme, font, spacing and keyboard-zoom
settings are unchanged; the existing interaction assertions needed no adjustment.
This makes future toolbar and timeline geometry checks use the native metrics.

`Session::preview_edit_pose` evaluates from a transaction's original document and
pending pose through the same pure transition as ordinary `edit_pose`. Six new
regressions verify Setup and Animate, Auto Key on/off, multiple pending bones,
fractional time and existing key snapping, overwritten/nearby curve restoration,
late failure, saved state, redo, cancellation and one-step commit. It requires an
active transaction and publishes only a successful candidate. Native v4 is unchanged;
this API does not itself add Shear controls or change the existing pose policy.

The integrated preparation passes **190 tests**: desktop **63**, CLI **4**, engine
**93**, I/O **15**, adapter **15**, plus formatting, warnings-denied Clippy, build
and architecture checks. The Direct native run and captures above remain scoped to
`694576a`; these mechanical context and engine-preview changes are not presented as
another native run. Shear model, migration, controls and their acceptance remain
the next work.


## Bone shear, native v5 and shared transform input — November 6, 2026 UTC

Final implementation source `775c7e3` passes **242 Rust tests** (81 desktop,
115 engine, 5 CLI, 19 I/O and 22 Spine adapter), formatting, warnings-denied
workspace/all-target Clippy, workspace build and the architecture guard. Checks
ran on the combined root source with its own Cargo target. Hosted CI and other
platforms have not been run.

Native v5 requires finite bone shear X/Y, keeps five-field region transforms and
strictly migrates v1–v4 without modifying load inputs. All seven original
zero-shear full export goldens remain byte-identical. Curve regressions preserve
finite ±2e38 spans and reject truly unrepresentable controls/evaluations before
JSON output; normal f32 results and the ten-segment policy remain. Checked scenes
guard drawing/picking/diagnostics. A bounded grid iteration fixes non-advancing
large-coordinate loops.

| Independent gate | Verified result |
| --- | --- |
| Compiled synthetic runtime | 30 original cases, 11,544 world values; max error 0.000002944 |
| Installed Spine 4.3.26 | 18 original imports/reexports pass, including Shear Lab; independent adapter accepts outputs |
| Editor-reexported synthetic runtime | 16 cases, 6,552 world values; max error 0.000003955 |
| Eight packed original fixtures | 224,640 bone values, 239,200 region coordinates, 33,800 mesh coordinates including 15,600 weighted, 67,600 UV values |
| Packed maximum errors | World 0.000048119; region/mesh 0.000061036; UV 0, at unchanged 0.001 tolerance |
| Original Shear Lab | All five clips and three skins; 13,000 mesh coordinates including 7,800 weighted |
| Native-edited packed document | All five clips: 70,200 bone values, 13,000 mesh coordinates including 7,800 weighted; max mesh error 0.000061036 |
| Bind/Remove-all CLI regression | 271 scene comparisons / 29,726 vertex coordinates; max vertex difference 0.000015259 |
| Direct CLI regression | 471 scene comparisons / 51,774 vertex coordinates; max vertex difference 0.000007630 |
| Artist audit | 11 inputs unchanged, zero new inspection issues and zero shear/weighted refusals; 0/11 fully supported |

The final compiled CLI remained unchanged across the native checks. Runtime
package **4.3.13** and installed editor/JSON target **4.3.26** are recorded
separately. Their patch numbers are not comparable release counters.

Native OS input verifies exact 23.5/-17.25 Setup values via Enter/Tab, both axis
handles, one-gesture history, held-pointer Escape/Save suppression, numeric
Escape/toolbar/owned-field Save, pending Animate shear, paired keying, Auto Key and
Save exclusion of a pending 13-degree rotation. Text entry updates on commit; no
live typed-model-preview claim is made. Global history also works after Tab
focuses an axes button, while actual-context tests preserve text/modal priority.
Seven Save backups are audited: the first matches exact initial bytes and all
match captured documents at f32 precision. The working document/file is restored
at f32 precision and the original clipboard restored. All 62 recorded task-owned
native/Spine command PIDs have exited; temporary authorization/command directories
are removed and no private key was written. Node jobs were separately waited/reaped.

Reports: [synthetic runtime](evidence/shear-runtime-synthetic.json),
[packed runtime](evidence/shear-runtime-packed.json),
[editor interchange](evidence/shear-spine-editor.json),
[native input](evidence/shear-native.json),
[native-edited runtime](evidence/shear-native-edited-runtime.json),
[Bind regression](evidence/shear-bind-regression.json),
[Direct regression](evidence/shear-direct-regression.json),
[artist summary](evidence/shear-artist-summary.json), and
[visual review](evidence/shear-visual.json). Only original artwork/data is committed.

This accepts the bounded data/authoring implementation. Exact Shear UI comparison,
empty-space/Shift behavior, World/Parent display, broader inheritance, constraints
and full production-rig compatibility remain open. Screen Capture was denied in
the fresh GUI context; installed-editor CLI checks do not establish those UI
behaviors. All sampled artist rigs still require physics; many also need IK,
transform/path constraints and unsupported animation types.

## Normal IK Phase A — November 6, 2026 UTC

Integrated implementation `5e4ea7e` passes **288 Rust tests**: 95 desktop, 133 engine,
7 CLI, 23 I/O and 30 adapter. Format, all-target Clippy with warnings denied,
workspace build and dependency architecture checks pass on that exact source.
`evidence/ik-checks.json` pins the source and both accepted binary hashes.

Native v6 requires ordered static IK constraints and migrates formats 1–5 without
changing their source bytes. All eight previous examples change only format 5→6
and an empty constraints list; their complete exported JSON remains byte-identical.
All 12 original artwork/atlas files remain byte-identical. The ninth fixture, IK
Lab, adds two target bones and independent one-/two-bone chains to the existing
weighted artwork. Five clips animate targets; the clip named “ik negative” is
negative target motion, not animated bend. See `evidence/ik-source-baselines.json`.

Independent original controls exercise winding, 180-degree ties, both bends,
ordinary unreachable targets, partial Mix and precision boundaries. Of 157 original
sources, 154 accepted poses match the separately installed public runtime API at
absolute 0.001 tolerance (maximum world error about 5.73e-7); three expected query
errors cover coincidence and lossy relevant target coordinates. Raw failures before
the targeted f64 precision fix remain retained locally. No runtime implementation
was inspected. `evidence/ik-runtime-synthetic.json` and engine provenance record scope.

The final integrated CLI packs all nine original fixtures and compares every clip
and all three skins at 52 times each, including exact ends, held tails and backwards
zero. It checks 304,200 bone values, 312,000 region coordinates, 46,800 mesh coordinates
(23,400 weighted) and 93,600 UV coordinates. Maximum world error is 4.82e-5;
region/mesh error is at most 6.11e-5 and UV error is zero. Geometry tolerance stays
0.001. Inputs remain unchanged. `evidence/ik-runtime-packed.json` records per-case results.

Native acceptance is **composite**, not one uninterrupted successful script. The
first run passes Mix text/slider history and cancellation, Positive, shared Rename,
target chooser/link/translation, raw Setup winding, read-only Animate settings,
pending-pose Save exclusion and posed Unbind/Bind with zero observed shape error.
It then fails because the driver tries to reveal a viewport vertex by scrolling
the Weights pane. Its edited task copy and failure are retained. A corrected focused
run starts from that actual authored save, uses Fit and the viewport vertex, and
passes Direct 35%, one-step Undo/Redo, exact pending-pose preservation, Save exclusion
and baseline restoration. No app changes separate these runs. Both clipboards are
restored; owned editors, two editor CLI processes and the isolated reference browser
are verified exited. Six plus two Save backups are readable, first backups match raw
baselines, and all eight match captured documents at f32 precision. See
`evidence/ik-native.json`; private inputs and unrelated windows were untouched.

The rig captured after those actual native IK/target/Direct edits is independently
packed and compared for all five clips and three skins. Its 79,560 bone values,
72,800 region coordinates, 13,000 mesh coordinates and 26,000 UV coordinates pass
with pending target translation excluded. `evidence/ik-native-edited-runtime.json`
pins that gate. Original IK Lab also imports/re-exports through installed Spine
Editor 4.3.26 and reimports into our adapter; `evidence/ik-spine-editor.json` records
this separate CLI interchange result. Editor re-export can reorder metadata and
quantize numbers; arbitrary lossless project conversion is not claimed.

Public guide images were observed in an isolated approved headed browser. Their
application version is unknown. They support the folder/property/annotation ordering
in `evidence/ik-visual.json`, but not exact installed-editor visual/interaction parity.
Screen Capture permission remains unavailable; no permission settings were changed.
IK timelines, creation/deletion/reordering, advanced or dependent/transformed chains,
physics, transform/path constraints and representative game-engine acceptance remain
open. The private inventory still has 0/11 fully supported rigs; full parity is an
ongoing objective, not this checkpoint's claim.

The final private audit uses the same integrated CLI and verifies all 11 source
hashes against the previous Shear baseline. Weighted/shear inspection issues remain
zero. It finds 33 IK, 224 physics, eight transform and three path entries; whole-rig
acceptance stays **0/11**. Of 33 IK entries, 30 lack advanced-field blockers, but
required-bone/domain validation still applies: only six pass isolated minimal
required-bone diagnostic projections. No private rig is filtered for actual import.
All 85 inspection invocations are waited, read-only and retain identifying reports
outside Git. The redacted report is `evidence/ik-artist-summary.json`.

Initial source publication is verified at
[PeachyGames/peachy-animator](https://github.com/PeachyGames/peachy-animator),
checkpoint `52a8446`. Remote and local commit hashes match; hosted CI was not
dispatched. See `evidence/repository-publication.json`. The approved
[next IK animation contract](slices/ik-animation-design.md) is design authorization,
not an implemented native7 or animation-parity claim.

## MCP and Views inventory — November 6, 2026

The combined source passes formatting, all **298 workspace Rust tests** (including
101 desktop tests, one MCP command-schema test and three real subprocess SDK-client
tests), Clippy across all targets with warnings denied, the workspace build and
the seven-crate architecture guard. Hosted CI was not run.

An independent JSON-RPC client also negotiates MCP 2025-11-25 with the built stdio
server and exercises all nine tools: inspection, guarded Open, a two-command edit,
atomic failure, stale revision refusal, dirty-Open refusal, Undo/Redo, pose geometry,
new-directory Save with images, and JSON plus packed export. Its evaluated pose
matches the CLI. Source bytes remain unchanged and the test server exits. Saved
JSON is compared at native f32 precision rather than requiring identical decimal
spelling between structured MCP values and direct file serialization.

The Views tests cover all 15 menu entries and the four new bounded panels, mode
availability, activation versus Tree selection, close/reopen, F9, pending-pose
preservation and workspace retention on Open. A normal task-owned native window
renders the original Peach Bot artwork. Three OS-input attempts stop at foreground
ownership guards before completing the new view workflow; they are not native
interaction acceptance. Each owned editor is closed and temporary local-login
authorization is removed. Native interaction, exact installed-editor mode lists and
pixel/docking parity remain pending. See [the evidence summary](evidence/mcp-views.json),
[MCP scope](MCP.md) and [Views scope](VIEWS.md).


## Playback view — November 6, 2026

The slice adds a bounded Animate Playback panel over the existing engine
transport. The final source passes formatting, all **303 workspace Rust tests**
(including **106 desktop tests**), Clippy across all targets with warnings denied,
the workspace build and the seven-crate architecture guard. Hosted CI was not run. Desktop regression checks cover percentage entry committed by actual
egui text/Enter events, the D shortcut and directional transport, forward/reverse
speed, both nonlooping
boundaries, reverse wrapping, zero-length clips, a stalled-frame cap before speed
scaling, frame/key navigation independent of speed, and unchanged document/history
and pending poses when speed alone changes. Menu availability, close/reopen, F9,
mode retention and minimum-window control visibility use the actual workspace UI.
The render review caught a clipped final navigation row; compacting the transport
row fixed it, and the layout assertion guards the default panel height.

A native render check uses only the original Peach Bot, an actual normal on-screen
window and the native screenshot path. Its isolated diagnostic binary opens
Playback initially so the panel can be reviewed without OS input. That temporary
startup code is removed from the final source and application. This verifies
rendering only: three full native input attempts did not complete, due missed
mouse delivery/window resizing and exact-PID foreground loss. No native typed-speed
or full mouse/keyboard acceptance is claimed. The [capture](playback-editor.png)
and [machine-readable evidence](evidence/playback.json) retain that distinction.
Exact installed Spine comparison, FPS retiming and Stepped/Interpolated overrides
remain pending. Neither export timing nor the native document format changes.

The starting commit is `1eb5a60750338a7fd5e478ddbe2c895fd0bec5b3` with a clean
working tree. Readable baseline, check logs, native attempts and cleanup records
are retained outside Git at
`../.local-backups/peachy-playback-20261106T154658Z-01c317/`.
Rollback: revert the Playback feature commit; consult the retained starting status
and diff before applying changes so unrelated later work is preserved. Task-owned
native processes are closed and temporary local-login authorization is removed.


## Ghosting view — November 6, 2026

The final integrated source passes formatting, **317 workspace Rust tests**
(including **141 engine** and **112 desktop** tests), Clippy across all targets
with warnings denied, the workspace build and the seven-crate architecture guard.
Hosted CI was not run. Eight engine Ghosting tests cover fractional/anchored
sampling, exact close keys, loop endpoints, empty clips, numeric extremes and the
64-pose budget. Six desktop Ghosting checks exercise real egui input, keyed versus
pending Current poses, checked weighted/IK geometry, lock reconciliation,
minimum-dock control visibility, unchanged Fit/selection/history and failure
isolation. The generic Views tests also cover Ghosting's inventory and mode gates.

A native workflow verifies actual typed 1.5-frame steps, 45% opacity and 2.5-pixel
per-frame offsets, Current, Solid display, Anchor/On top/Loop, Selection/Lock/Refresh,
F9, mode suppression, close/reopen, viewport enable and use without Dopesheet.
It uses original artwork and leaves document/history unchanged. This run exposed
an anchored f32 boundary that omitted the furthest preceding pose at native
0.6 seconds. The engine fix recognizes exact native lattice identities rather
than merging nearby times; a new regression protects that case. The final native
recheck verifies all eight expected offsets and preceding ghosts wrapping at zero.
The [evidence](evidence/ghosting.json) distinguishes the full control workflow,
final timing recheck and intermediate harness failures. The [capture](ghosting-editor.png)
shows our native editor and original artwork; it is not installed-Spine parity.

The final CLI's original Peach Bot, weighted and IK exports/evaluation also pass
the external Spine 4.3.13 public-API oracle. Across these finite fixtures there
are 135,720 bone-transform component, 135,200 region-coordinate and 18,200 mesh-coordinate
comparisons (10,920 weighted), plus 36,400 UV comparisons. Maximum region error is
0.000038147 and maximum mesh error is 0.000030518, below the unchanged 0.001
tolerance. These checks validate the geometry used for ghosts, while frame/key
sampling is checked separately. No private artwork or runtime implementation is
copied into this repository, and no runtime is redistributed.

Native verification uses isolated task-owned temporary macOS app bundles and
exact-PID foreground/window guards through the documented serialized fresh local
GUI login. Only owned windows are raised/positioned. Temporary authorization is
removed immediately after authentication and again during cleanup; no private key
is written. All owned editors are closed. No application-startup diagnostic or
special ghosting fixture flag is added to the product.

The readable starting commit/status/diff, engine lane, complete logs, native
scripts and cleanup records are retained at
`../.local-backups/peachy-ghosting-20261106T163038Z-3621ce/`.
The clean baseline is `b5fc141cdfc490a32a04dee029acb1471b318a76`.
Rollback: revert the desktop Ghosting commit and the two engine Ghosting commits
in reverse order; consult the retained baseline and preserve unrelated later work.
Motion Vectors, Silhouette/Xray and exact installed-editor comparison remain gaps.


## Timeline view — November 6, 2026

The final source passes formatting, **334 workspace Rust tests** (including
**129 desktop** and **141 engine** tests), Clippy across all targets with warnings
denied, the workspace build and the seven-crate architecture guard. Hosted CI was
not run. Seven pure model tests cover exact distinct keys, coordinate mapping,
snapping, zoom/pan, empty clips and bounded ticks at extreme times. Ten desktop
Timeline checks exercise actual egui input through the full editor: decimal Frame
entry, Enter/Tab apply, no-op/invalid/cancel preservation, pending-pose discard and
Undo recovery, snapped/subframe/loop scrubs, held-pointer cancellation, focus and
modal guards, file/history/context actions, shared transport, range controls and
minimum-window layout. These are distinct from native OS-input acceptance.

A normal task-owned native window renders the original Peach Bot with Timeline
open and Dopesheet closed. The [capture](timeline-view-editor.png) shows the compact
controls, key markers and cyan playhead within the panel. This rendering-only
check uses an isolated diagnostic build with temporary initial mode/view/time
assignments. The source is restored byte-for-byte and the final application rebuilt
without those assignments. Three separate native mouse attempts could not activate
even the existing Setup/Animate control, so **native mouse/keyboard acceptance is
pending**. No native typed-frame or installed-editor parity claim is made. The
[evidence](evidence/timeline-view.json) records this distinction and binary hashes.

The original Peach Bot also passes the separately installed Spine 4.3.13 public-API
oracle: 28,080 bone-transform component and 33,280 region-coordinate comparisons,
two clips, three skins and 52 samples per clip/skin. Maximum world error is
0.000026438 and maximum region error is 0.000030518, below the unchanged 0.001
tolerance. No runtime implementation or private artwork is inspected or copied;
the runtime remains external and is not redistributed. No native/export format
or engine geometry changes are part of Timeline.

The clean starting commit is `613973fa2e56cd9c4d884f635d2d2e5af5e2aeb4`.
Readable source baseline/status/diff, model worktree, build/check logs, native
attempts, diagnostic capture and final cleanup audit are retained at
`../.local-backups/peachy-timeline-20261106T172510Z-b653c1/`.
Rollback: revert the desktop Timeline feature commit, then model commit `dd00baf`;
consult the retained baseline and preserve unrelated later work. Disk recovery
removed only this task's redundant build artifacts; source, logs and evidence remain.
All four recorded task-owned editor PIDs are verified exited. Temporary localhost
GUI authorization and command directories are removed, no private key was written,
and no browser was launched. Independent session-only ranges, free docking,
per-animation Repeat and exact installed-editor comparison remain gaps.


## Mesh Tools view — November 6, 2026

The final integrated source passes formatting, **352 workspace Rust tests**
(including **150 engine** and **138 desktop** tests), Clippy across all targets
with warnings denied, the workspace build and the seven-crate architecture guard.
Hosted CI was not run. Nine geometry tests cover world-distance falloff, hull
exclusion, hard/feathered endpoints, coincident Size-zero vertices, shear/reflection,
large origins, wide inverse cancellation, malformed/weighted/overflow rejection
and fixed-baseline transactional history. Nine desktop checks cover real egui
settings and vertex input, exact press targeting, release position, zero-return
previews, pending-pose/history preservation, Save, context/focus/modal/projection
cancellation, cancelled held pointers, weighted/inactive gates and minimum layout.

Review caught and fixed returning-to-zero previews, nearby-vertex drag stealing
and a projection-error transaction leak. The initial engine regression incorrectly
expected an empty Undo stack even though its fixture already contained a pose edit;
it now verifies the actual live-dispatch path and that the preexisting pose edit
remains the sole Undo step. Initial failures and final passing logs are retained.

A pure engine harness moves the original Mesh Lab body's center by world [6,-4]
with Size 120/Feather 50%, preserves all UVs/topology/hull/edges and verifies one-step
Undo/Redo and unchanged source bytes. The resulting exported rig passes the
external Spine 4.3.13 public-API oracle over two clips, three skins and 52 times per
clip/skin: 28,080 bone-transform values, 29,120 region coordinates, 5,200 mesh
coordinates and 10,400 UV coordinates. Maximum geometry error is 0.000030518,
below the unchanged 0.001 tolerance; UV error is zero. No runtime implementation
or private artwork is inspected/copied, and no runtime is redistributed.

The [native capture](mesh-tools-editor.png) shows original artwork, the actual
Mesh Tools controls, five influence-colored vertices and both radius rings. Exact
owned-PID foreground and a normal on-screen window are verified. This is a
rendering-only diagnostic build with temporary initial context/view/settings;
the final source is restored byte-for-byte and the application rebuilt without
that setup. Native click/typed-value acceptance is **pending**, following the
same-session Timeline mouse-delivery failures. The capture and egui tests do not
claim installed-editor visual/interaction parity. See [evidence](evidence/mesh-tools.json).

The clean baseline is `8da2dc245ccb89183c51c8e7aa77952145817188`.
Source/status/diff, engine lane, tests/builds, generated original geometry,
runtime report and native cleanup records are at
`../.local-backups/peachy-mesh-tools-20261106T181246Z-f93e45/`.
Rollback: revert the desktop Mesh Tools commit, then `22cdaa9` and `d241b80`,
preserving unrelated later work. The disk filled during a diagnostic compile;
source restoration succeeded. Only this project's regenerable incremental
compiler cache and this lane's disposable target were removed; baselines,
source, logs and retained binaries remain. Later builds disable incremental
caching for space. The owned editor is verified exited, temporary localhost
GUI authorization/command directories are removed, no private key is written,
and no browser is launched. Multi-point, Rotate/Scale, weighted/Animate soft
transforms and exact installed-editor comparison remain gaps.


## Independent Preview — November 6, 2026

The final source passes formatting, **359 workspace Rust tests** (including
**145 desktop** and **150 engine** tests), all-targets Clippy with warnings denied,
the workspace build and seven-crate architecture guard. Hosted CI was not run.
Seven Preview checks exercise independent clip/skin/seek/transport, speed/loop
boundaries, checked weighted/static-IK keyed poses excluding pending edits,
localized evaluation failure/recovery, hide/F9/Setup/focus/Open lifecycle,
actual canvas pan/zoom and shortcut isolation, minimum layout and empty/extreme
native contexts. Main document/pose/selection/clip/time/history are compared.
The checked evaluator, IK solver, sampling, transport and Session sources are
byte-identical to the verified Mesh Tools checkpoint; its external public-API
runtime evidence remains applicable. Projection/rendering are checked separately.

The initial layout clipped the canvas and overflowed Speed. Four compact rows now
leave a 100-pixel canvas in the default dock at minimum window size. A fixture-index error
was corrected to select the actual appended alternate clip. Escape handling also
needed the actual end-frame focus owner because egui clears focus before the
shortcut pass; its regression now confirms main selection stays unchanged.

The [native capture](preview-view-editor.png) shows a checked keyed wave inside
Preview while the main viewport remains in Setup. A normal on-screen window,
exact owned-PID foreground, loaded original artwork and all controls/canvas
inside the panel are verified. This render-only binary differs by one initial
open-Preview call; final source is restored byte-for-byte and the app rebuilt
without that call. Native click/typed-value acceptance remains **pending**;
no installed-editor visual/interaction equivalence is claimed. See
[evidence](evidence/preview-view.json).

The clean integration baseline is `dfa4f25` (full hash retained in the backup).
Source/status/diff, lane files, initial failures and final checks, native scripts,
capture, hashes and cleanup are retained at
`../.local-backups/peachy-preview-20261106T183406Z-589208/`.
Rollback: revert the Preview integration commit, then `322c260` and `1d886d7`,
preserving unrelated later work. The owned editor is verified exited; temporary
local GUI authorization/command directory are removed, no private key is written,
and no browser is launched. Mixing/layering and exact installed-editor parity
remain gaps; no document format or export timing changes are part of Preview.


## Scalar Graph view — November 6, 2026

The final source passes formatting, **368 workspace Rust tests** (including
**154 desktop** and **150 engine** tests), all-targets Clippy with warnings denied,
the workspace build and seven-crate architecture guard. Hosted CI was not run.
Nine Graph regressions cover exact time/curve preservation, one-step value history,
pending poses/redo, held cancellation, close native keys and exact picker/Zoom Keys,
10,000-key display budgets, extreme finite values, missing tracks, typed contexts,
main pose edits, focus isolation and default-dock bounds at1120×740. Sampling and
SetKey use the unchanged verified engine; no format/export changes are introduced.

The actual scroll-area clip now bounds plot height. Key selection/presentation is
separate from Dopesheet clipboard data. End-frame focus ownership handles Escape
without clearing the main bone, and an unrelated pose edit cancels the graph before
changing Session's pending state. Main pose-render errors do not prevent correcting
finite scalar data. Tangent/retiming/key-creation/general-pan-zoom/multi-curve tools
remain pending; no complete installed Graph interaction-parity claim is made.

The [native capture](graph-view-editor.png) shows original artwork, the sampled
curve, eight exact rotation-key markers and shared18-frame playhead, with all
controls/plot inside the default dock. It verifies a normal task-owned foreground
window and loaded artwork. The isolated diagnostic startup only enters Animate,
opens Graph and seeks0.6; final source is restored byte-for-byte and app rebuilt.
Native mouse/keyboard acceptance remains **pending**, separate from egui event
checks and rendering. [Evidence](evidence/graph-view.json) pins these limits/hashes.

The clean integration baseline is `06d2afe` (full hash retained), with source,
status/diff, lane, checks, captured output and cleanup at
`../.local-backups/peachy-graph-20261106T184001Z-4731b0/`.
Rollback: revert the Graph integration commit then `cb2afd0`, preserving unrelated
later work. The owned editor is verified exited, temporary localhost GUI
authorization/command directory are removed, no private key is written, and no
browser is launched.


## Setup Slot Color and native7 — November 6, 2026

The integrated source passes formatting, **390 workspace Rust tests** (including
**165 desktop** and **153 engine** tests), all-targets Clippy with warnings denied,
the workspace build and seven-crate architecture guard. Hosted CI was not run.
Nine chooser checks cover exact RGBA, hidden RGB at alpha0, real picker/hex input,
Hue→SV intent on gray, one-step Undo/Redo, invalid/no-op/cancel/focus behavior,
typed targets, read-only Animate, pending poses, file actions and held input.
Two additional paint-mesh checks verify viewport/Outline/Preview tint and both
ghost modes. The initial test incorrectly expected can_undo=false during a changed
transaction; Session intentionally exposes Undo as cancellation. Its corrected
regression verifies one committed step and a clean baseline after Undo.

Native7 requires exact four-byte setup slot.color. Legacy1–6 reject field presence
before white migration; current missing/malformed fields fail. All9 previous white
Spine JSON exports are byte-identical; all35 legacy fixture files remain unchanged.
An original public-API oracle checks1,029 cases/4,116 normalized channel values with
zero error, including all byte values,6RGB/8RGBA and case handling. Runtime code is
not inspected/copied or redistributed. Static IK behavior remains unchanged; the
pending approved IK animation contract now explicitly targets native8 with the
same gates, preserving slot colors and adding no unused IK fields to native7.

The [native capture](slot-color-view-editor.png) shows exact#FF5078D2 body color,
the inline chooser and original artwork with body/head/palm tint/alpha changes.
The fixture is a task copy authored through real CLI SetSlotColor commands, never
an artist input. The normal owned foreground window and clipped controls are
verified. Temporary startup only selects the body slot/opens the view; final source
is restored byte-for-byte and app rebuilt. This is rendering evidence; native
click/typed-value acceptance remains **pending**. See [evidence](evidence/slot-color-view.json).

The clean integration baseline is `976f858` (full hash retained). Source/status/
diff,9 copied originals/export baselines, legacy hashes, probes, lane/UI source,
all checks and native cleanup are under
`../.local-backups/peachy-slot-color-20261106T190033Z-83a8ab/` and
`../.local-backups/peachy-slot-color-ui-20261106T190400Z-54159a/`.
Rollback: revert the integration commit, then `b5a2d1b`, `6a792cc`, `42289d3` and
`2b657b5`, preserving unrelated work. To use an older binary, restore a pre-Save
native6 backup; do not manually change a native7 version number or discard colors.
Existing source files are unchanged until explicit Save, which retains a backup.
The task-owned editor is verified exited, temporary GUI authorization/command
files are removed, no private key is written, and no browser is launched.


## Audio view and completed view inventory — November 6, 2026

All **15 views** in [VIEWS.md](VIEWS.md) now open functional bounded views;
there are no unimplemented view placeholders. Feature-level limitations and
mode availability remain explicit. Audio completes this inventory with a
session-only WAV waveform and independent audition, rather than an audio
attachment/event timeline model. Project format remains native7.

Final source passes formatting, **404 workspace Rust tests** (including
**173 desktop**, **153 engine** and **10 IO library** tests), all-targets Clippy
with warnings denied, the workspace build and seven-crate architecture guard.
Hosted CI was not run. Eight Audio GUI/mock regressions and six WAV decoder
checks cover actual resulting state, independent transport/preferences, source
replacement, cancellation/focus/modal lifecycle, unsupported playback, bounded
waveform work, malformed input and the minimum dock. An egui wheel-event phase
and waveform arrow-focus lock were corrected before final checks.

The separate, normally ignored native test also **passes** from an authenticated
GUI context. It checks actual native clock advancement, pause stability, repeated
Pause, seek, resume, Loop and Stop with the owned sound's volume set to zero;
system volume is untouched. Its first run caught NSSound reporting `isPlaying`
while successfully paused with a stopped clock. The adapter now combines native
state with its successful pause state, allowing Resume and harmless repeated
Pause. All workspace checks were rerun on that fix. This is native backend-state
acceptance, not an audible listening-quality assessment.

The [native capture](audio-view-editor.png) shows the original Peach Bot and a
generated two-second mono 16 kHz tone loaded in Audio. A normal on-screen window,
exact owned-PID foreground, loaded artwork and all waveform/controls inside the
panel are verified. A later foreground change caused the exact-PID guard to
refuse a redundant pointer-refresh capture; the earlier verified loaded frame
is retained. The trap closed only the owned editor. Temporary startup opens
Audio and loads that original tone without autoplay; final source is restored
byte-for-byte and production rebuilt. Full native mouse/keyboard acceptance
remains **pending**, distinct from egui checks and native backend/render evidence.
See [Audio evidence](evidence/audio-view.json).

The clean integration baseline is `9cd4e11`; Audio source checkpoints are
`6571552`, `58246e6`, `311b707` and native pause correction `3936a37`.
Baseline/status/diff, lane, failed probes, final checks, native artifacts/hashes
and cleanup are retained under
`../.local-backups/peachy-audio-20261106T190207Z-ed0f8a/`.
Rollback: revert this documentation checkpoint, then `3936a37`,
`311b707`, `58246e6` and `6571552` in reverse dependency order, preserving unrelated
work. No native file migration is part of Audio. The exact owned editor PID is
verified exited, scoped localhost authorization and temporary command files are
removed, no private key is written, and no browser is launched. Audio event
keys, animation synchronization, audio Save/export, other formats and
cross-platform output remain explicit gaps.

## Physics constraint data — November 9, 2026

Scope: physics constraints and physics timelines as lossless data in native v8,
with typed Setup editing. **No editor simulation** is claimed; see
[the slice](slices/physics-data.md) and [evidence](evidence/physics-data.json).

| Check | Result |
| --- | --- |
| Format probes | Original probes through installed Spine **4.3.26** import/re-export and the external **4.3.13** public data API establish field names and defaults (inertia 0.5, damping 0.85, strength 100, mass 1, mix 1, limit 5000, FPS 60) and omitted key values (Mix 1, others 0) |
| Original Physics Lab fixture | 3 constraints, 63 settings, 8 timelines and 16 frames equal in the external runtime; 28,080 bone and 33,280 region comparisons with `Physics.none`, max world error 0.000026; clip ends include reset keys (idle 2.75) |
| Installed editor round trip | 4.3.26 imports/re-exports the fixture; our importer accepts it; constraints, setting keys and resets equal at f32 precision; the same runtime checks pass on the re-export |
| Game skeleton audit | 19 shipped exports read-only and unchanged; fully supported 0 → **1**; physics refusals remain only for skin-required physics (2 files) |
| Game rig with simulation | One newly supported game skeleton (6 bones, 2 physics constraints) and its Peachy import→export run side by side in the runtime with `Physics.update`, 60 fps for 6 s: max bone difference 0.00012 and mesh vertex difference 0.00024, while physics itself moves bones up to 16.4 units |
| Rust suite | `cargo fmt --check`, `clippy -D warnings`, architecture check and build pass; **417** tests pass, 1 ignored (two test threads) |

The desktop checks render the real workspace through egui: every physics control
is reachable inside Properties (scrolling as a user would), a typed Strength value
applies once on Enter as one undo step, the Global toggle is a separate step, Animate
controls are read-only, and rename routes to the physics command while keyed
references stay attached. **Native OS mouse/keyboard acceptance in a task-owned
window was not run for this slice**, so no native interaction parity is claimed.

The game-rig comparison exercises private artist data locally only. Its inputs
were verified unchanged by SHA-256; outputs and reports stay under `.local/`.
Region corners are not compared for that rig because its atlas trims whitespace.


## Nested IK chains, child offsets and export names — November 9, 2026

| Check | Result |
| --- | --- |
| Original IK Chain Lab (`ik-chain-bot.peachy`): leg + foot aim, arm + hand aim, off-axis children | 93,600 bone, 72,800 region and 13,000 mesh comparisons against external 4.3.13; max world error 0.000034 |
| Same rig with inner chains listed first | Poses differ from forward order by up to 1.6 units on the foot; runtime matches again (93,600 comparisons) |
| Wide (20, 30) and negative-X (−30, 25) child offsets | Runtime matches (93,600 comparisons each) |
| Installed 4.3.26 attachment-name probes | Reused names across skins, within one skin and in the default skin all import and re-export |
| Game audit | Fully supported 1 → **2** of 19; inputs unchanged |
| Game rig (62 bones, 6 IK incl. nested leg/toe, 16 physics, 40 skins) | Our evaluated pose vs runtime `Physics.none`: 9,300 comparisons, max 0.00042 (long hair-chain tip). Original vs Peachy re-export with `Physics.update`, 40 skins × 240 frames: max bone 0.000092, mesh 0.00024; physics moves bones up to 11.6 units |
| Rust suite | fmt, clippy `-D warnings`, architecture and build pass; **420** tests pass |

Artist inputs and reports stay under `.local/`; only counts are recorded here.


## Animated IK integrated as native9 — November 9, 2026

| Check | Result |
| --- | --- |
| Merge | 16 review commits (`71339e7`…`5002278`) merged onto `a89637b`; format renumbered to native9 |
| Rust suite | fmt, clippy `-D warnings`, build and architecture pass; **480** tests pass, 1 ignored (two threads) |
| External 4.3.13 runtime, 12 original examples | All pass. Animation Lab: 175,032 bone values (max 0.000036), 3,432 IK pose groups (max Mix error 8.3e-17), 160,160 region and 28,600 mesh coordinates. IK Chain Lab 3,900 IK groups; Physics Lab 3 constraints and 16 frames |
| Game audit | 2/19 fully supported, inputs unchanged |
| Not run | Native OS input acceptance; the user's interactive review |

The raw IK-curve import precision correction remains open (see
[ik-raw-precision.md](slices/ik-raw-precision.md)).


## Bone inheritance modes and native v10 — November 9, 2026

| Check | Result |
| --- | --- |
| Formula derivation | 1,360 random runtime observations (shear, negative scales, nested parents); all five modes reproduced within 5e-7 |
| Original Inherit Lab (`inherit-bot.peachy`): every mode, nonuniform/sheared/mirrored parent, keyed mode changes | 28,080 bone and 33,280 region comparisons against external 4.3.13; max world error 0.000036. Resetting all modes to normal moves bones by up to 1.5 matrix units, so the check discriminates |
| Installed editor and runtime format probes | `inherit` field and keys round-trip; omitted key value is normal; Setup mode before the first key; legacy `transform` alias ignored by the editor |
| Game audit | 21 skeletons now (two new survivor-liquids files); fully supported **4**; zero inheritance refusals remain; inputs unchanged |
| Regression | Doggy-scene male rig pose vs runtime unchanged (max 0.00042) after merging animated IK |
| Rust suite | fmt, clippy `-D warnings`, architecture and build pass; **491** tests pass |

Animated IK (native9) was merged into `main` first; inheritance became native10.


## Transform constraints, IK ancestors and native v11 — November 9, 2026

| Check | Result |
| --- | --- |
| Behavior derivation | 300 random runtime rigs (x, y, rotate, combinations; mirrored sources): max error 1e-6 |
| Original Transform Lab (`transform-bot.peachy`): mirrored translate (mix −1), rotate with offset 30 and 250 | 28,080 bone and 33,280 region comparisons against external 4.3.13; max error 0.000045. Removing the constraints moves hand.L by up to 190 units |
| IK under transformed ancestors | Uniform, small/large shear, nonuniform and mirrored ancestors: 1,800 values each, max error 2.8e-5; IK moves bones by a median 27 units |
| Game audit | **6 of 21** fully supported (doggy female2 and doggy-artist back view added); inputs unchanged |
| Doggy female2 (97 bones, 77 skins, 6 IK, 42 physics, 2 transform) | Our pose vs runtime: 29,100 values, max 0.00035. Original vs re-export with `Physics.update`: max bone 0.000079, mesh 0.00024 |
| Doggy-artist back view (30 skins, 26 physics, transform) | Our pose vs runtime: 10,050 values, max 0.00035. With physics: max bone 0.000085 |
| Rust suite | fmt, clippy `-D warnings`, architecture and build pass; **498** tests pass |


## Skin-required bones and constraints, native v12 — November 9, 2026

| Check | Result |
| --- | --- |
| Runtime activity probes | Listing a child activates required parents; constraints need active bones; default/no skin activates none |
| Original Skin Lab (`skin-bot.peachy`): mint-only bone, slot and IK aim; non-required transform on that bone | All three skins: activity agrees; 30,576 bone and 34,112 region comparisons, max 0.000027; 208 inactive bone samples skipped |
| Side-view head (game, 52 skins, 19 skin-required bones, 12 physics) | 26,000 activity checks agree; 95,850 active-bone comparisons, max 0.00004; with `Physics.update` re-export matches the original within 0.00005 |
| Game audit | **7 of 21** fully supported; inputs unchanged |
| Rust suite | fmt, clippy `-D warnings`, architecture and build pass; **504** tests pass |


## Mesh deform keys, native v13 — November 9, 2026

| Check | Result |
| --- | --- |
| Runtime and editor format probes | Unweighted offsets add to setup positions; weighted offsets add per influence; sparse `offset`/`vertices`; absolute-time Bezier with fraction values |
| Original Deform Lab (`deform-bot.peachy`): weighted hand and unweighted body, linear/Bezier/stepped | 5,200 mesh coordinates (3,120 weighted), max 0.000031, plus 28,080 bone values against external 4.3.13; deform moves vertices by up to 8.2 units |
| Installed 4.3.26 round trip | Reimported; deformed geometry within 0.0016 (editor drops a zero-weight influence, reorders influences and their deform values consistently) |
| Front-view unified head (game, 119 skins, 56 bones, 37 constraints, deform keys) | 99,960 activity checks agree; 135,090 bone values max 0.00004; 61,410 deformed mesh coordinates max 0.00006; physics-on re-export within 0.00005 |
| Game audit | **8 of 21** fully supported; inputs unchanged |
| Rust suite | fmt, clippy `-D warnings`, architecture and build pass; **509** tests pass |


## Slot attachment and color keys, native v14 — November 9, 2026

| Check | Result |
| --- | --- |
| Runtime and editor probes | Setup before the first key; nameless key hides; per-channel absolute Bezier; color required on rgba keys |
| Original Slot Lab (`slot-bot.peachy`): swap to a second image, hide, return; per-channel Bezier, stepped and linear color keys | Visibility agrees at every sample; 32,992 region coordinates and 16,496 color channels (within half a byte) against external 4.3.13 |
| Game audit | Slot keys no longer block any skeleton; still **8 of 21** fully supported (remaining rigs have other blockers) |
| Rust suite | fmt, clippy `-D warnings`, architecture and build pass; **514** tests pass |


## Events and reference scale, native v15 — November 10, 2026

| Check | Result |
| --- | --- |
| Editor and runtime probes (original) | Defaults written as omitted fields; key overrides kept as written; event times extend duration |
| Crossed Bezier time handles (original Peach Bot variant) | 28,080 comparisons against external 4.3.13, max error 0.00003 |
| Doggy-scene female (game asset, local only; SHA-256 unchanged) | Imports with events and reference scale 92. Across 73 skins: 107,310 bone-activity checks and 643,860 world values match our CLI, max 0.0015 (the 292 values above 0.001 are all `toe.L`, at the end of a two-bone IK chain at 99.98% reach, where the bend angle is ill-conditioned); 80,558 mesh values, max 0.002. Original vs re-export with physics: 65,700 frames, max bone difference 0.0001, 56 physics constraints |
| Game audit | **9 of 21** skeletons import fully |
| Rust suite | fmt, clippy `-D warnings`, architecture and build pass; **518** tests pass |


## Draw order keys, native v16 — November 10, 2026

| Check | Result |
| --- | --- |
| Runtime probe (original, 9 slots) | Hypothesis matches 301 of 301 random offset encodings; 7 malformed cases throw or corrupt the order and are refused |
| Editor probe | Installed 4.3.26 preserves all 81 key orders and re-encodes minimally |
| Original Draw Order Lab (`draw-order-bot.peachy`): arm to the front, head to the back with a zero offset, reset key, first key after 0 | Our draw sequence equals the runtime's applied draw order at all 312 samples; 28,080 world values, max error 0.00003; editor round trip keeps every order |
| Game audit | Draw order no longer blocks any skeleton; still **9 of 21** (the draw order rigs have other blockers) |
| Rust suite | fmt, clippy `-D warnings`, architecture and build pass; **522** tests pass |


## Attachment tint and curve edge cases, native v17 — November 10, 2026

| Check | Result |
| --- | --- |
| Editor and runtime probes (original) | Attachment colors follow the slot color contract (6 digits opaque, omitted white) |
| Original Tint Lab (`tint-bot.peachy`): tinted region, half-alpha tint under keyed slot colors, tinted swap image, flat overshoot bone key and color channel | 16,496 color channels (slot × attachment) within half a byte, draw order at 312 samples and 28,080 world values (max 0.00003) against external 4.3.13; editor round trip keeps colors and both overshoot curves |
| Out-of-interval time handles (original variant, −0.9 to 1.9) | 28,080 world values, max 0.00003 |
| Oral-artist back (game asset, local only; SHA-256 unchanged) | 30 skins: 106,920 world values (max 0.0002) and 42,024 mesh values (max 0.0002); original vs re-export with physics: 16,200 frames, max bone difference 0.00006, 29 physics constraints |
| Idle female (game asset, local only; SHA-256 unchanged) | 5 offset curves, 10 draw order keys, 1 tint. 106 skins: 431,844 bone-activity checks, 2,591,064 world values (max 0.0004) and 1,043,812 mesh values (max 0.0005); original vs re-export with physics: 267,120 frames, max bone difference 0.00015, 46 physics constraints |
| Runtime check script | Now also runs on packed game atlases: rotated regions are matched by position and packed UVs are compared only on untrimmed, unrotated regions |
| Game audit | **11 of 21** skeletons import fully |
| Rust suite | fmt, clippy `-D warnings`, architecture and build pass; **527** tests pass |


## Linked meshes, native v18 — November 10, 2026

| Check | Result |
| --- | --- |
| Runtime and editor probes (original) | Source in the same slot of `skin` (omitted: default); `timelines` defaults to following the source; own color; editor layout as documented |
| Original Linked Mesh Lab (`linked-mesh-bot.peachy`): weighted hand swapped to a tinted linked mesh that follows its deform keys and to one that ignores them; mint body linked to the deformed peach body across skins | All skins against external 4.3.13: 5,200 mesh coordinates (3,120 weighted), 10,400 UVs, 16,640 color channels, draw order at 312 samples, max mesh error 0.00003; editor round trip keeps every link field |
| Doggy-artist front (game asset, local only; SHA-256 unchanged; 4 links) | 30 skins: 106,920 world values (max 0.0013, the stretched IK toe) and 24,882 mesh values (max 0.003); original vs re-export with physics: 16,200 frames, max bone difference 0.0007, 33 physics constraints |
| Oral female (game asset, local only; SHA-256 unchanged; 2 links) | 60 skins: 317,520 world values (max 0.0002) and 119,718 mesh values (max 0.0004); original vs re-export with physics: 32,400 frames, max bone difference 0.0001, 44 physics constraints |
| Game audit | **13 of 21** skeletons import fully |
| Rust suite | fmt, clippy `-D warnings`, architecture and build pass; **532** tests pass |


## Transform remapping — November 10, 2026

| Check | Result |
| --- | --- |
| Runtime probes (original) | 4 sweeps and 5 clamp/mix edge cases fit `(p − from) × scale + to`, clamped between to offset and max, then mixed |
| Original Transform Map Lab (`transform-map-bot.peachy`) | 28,080 world values (max 0.00006) and 33,280 region coordinates against external 4.3.13; X is inside the clamp range at some samples and clamped at others |
| Cow front (game asset, local only; SHA-256 unchanged) | 30 skins: 22,950 bone-activity checks, 137,700 world values (max 0.0002) and 40,260 mesh values (max 0.0002); original vs re-export with physics: 27,000 frames, max bone difference 0.0001, 18 physics constraints |
| Game audit | **14 of 21** skeletons import fully |
| Rust suite | fmt, clippy `-D warnings`, architecture and build pass; **533** tests pass |


## Clipping, native v19 — November 10, 2026

| Check | Result |
| --- | --- |
| Runtime probes (original) through the headless renderer | Clip range, own-slot `end`, ignored nested clips and the `convex` default as documented in the slice |
| Original Clip Lab (`clip-bot.peachy`): concave polygon on the body through the head, convex-hull clip on the right arm to the end of drawing | At all 312 samples, total clipped area (max difference 0.006) and a 24×24 coverage grid (179,712 points, 0 mismatches) equal the runtime renderer's; 28,080 world values (max 0.00003); editor round trip keeps both polygons (editor sets `convex`, see the slice) |
| Desktop capture | The Clip Lab renders with the concave notch through head and body and the arm trimmed to the hull |
| Game audit | **15 of 21** skeletons import fully (female pose added) |
| Rust suite | fmt, clippy `-D warnings`, architecture and build pass; **538** tests pass |


## Path constraints, native v20 — November 10, 2026

| Check | Result |
| --- | --- |
| Black-box fits (original random paths, external 4.3.13) | 25 single curves: 10-segment table, max 7e-6. Fixed mode: implied curve length = 4-segment polyline (within 0.003). Combined model over multi-curve paths, percent/fixed position, offset rotation, length/fixed/percent spacing for siblings and chains: positions within 1e-5, rotations within 3e-4°. Out-of-range positions follow the end handles exactly |
| Original Path Lab (`path-bot.peachy`): unweighted rail with keyed position past both ends (Bezier key), offset rotation and partial mixes; three-bone leg chain on a weighted path with keyed negative spacing | 28,080 world values (max 0.00003) and 33,280 region coordinates against the runtime; the constraints move the bones by up to ~120 units vs unconstrained. Editor round trip keeps both constraints, both timelines and the path vertices (it recomputes setup `lengths`) |
| Game audit | Path constraints themselves now import; the five path rigs remain blocked by deform keys on path/clipping attachments (next) or IK stretch (cow back). Still **15 of 21** |
| Rust suite | fmt, clippy `-D warnings`, architecture and build pass; **543** tests pass |


## Deform keys on paths and clipping polygons — November 10, 2026

| Check | Result |
| --- | --- |
| Tangent near a curve start (original probes) | Runtime uses the start direction below t = 0.001 (bisected on three curves), the derivative above |
| Path Lab with deform keys on its plain and weighted rails | 28,080 world values (max 0.00003) and 33,280 region coordinates (max 0.0001) against the runtime; the deform moves the hand by more than 1 unit |
| Clip Lab with deform keys on the concave polygon | 312 samples, 179,712 coverage points, 0 mismatches; the deform changes the clipped area (29,793 vs 34,216 at t = 1.2) |
| Game audit | **16 of 21** skeletons import fully (oral male added) |
| Rust suite | fmt, clippy `-D warnings`, architecture and build pass; **545** tests pass |


## Path spacing and bone scale — November 10, 2026

| Check | Result |
| --- | --- |
| Black-box fit (original sibling bones scaled 1.0/1.5/0.6) | Length gaps = (previous length + spacing) × previous world X scale; fixed gaps = spacing × that scale; percent unscaled |
| Oral male path chain at setup | All four `icing_path` bones match the runtime to 0.001 (were up to 5.4 off) |
| Oral male (game asset, local only; SHA-256 unchanged; 1 path constraint, 4 deform tracks incl. the path) | 31 skins: 107,136 world values (max 0.0004) and 31,904 mesh values (max 0.0005) |
| Runtime check script | Its native clip end now counts every keyed kind (deform, slot, path, draw order, events), as `Animation::duration` does; all twelve labs pass |
| Path Lab with a 1.3-scaled chain bone | 28,080 world values (max 0.00006) and 33,280 region coordinates (max 0.0001) |
| Female pose (game asset, local only; SHA-256 unchanged) | 85 skins: 1,085,280 bone-activity checks, 3,560,256 world values (max 0.0005), 1,355,542 mesh values (max 0.0007); original vs re-export with physics: 489,600 frames, max bone difference 0.00014, 33 physics constraints |


## IK with scaled, mirrored and stretched bones, native v21 — November 10, 2026

| Check | Result |
| --- | --- |
| Black-box fits (original random two-bone rigs) | Unit, uniform, mirrored and mirrored-with-scaled-child chains: max 7.5e-6. One bone: X axis aims at the target (mirrored turns 180°). Stretch: factor distance/reach blended by mix; with stretch on, the child's Y offset is ignored and its applied Y becomes 0 |
| Original IK Scale Lab (`ik-scale-bot.peachy`): mirrored arm, (1, −1)-scaled arm with 0.8 child and negative bend, keyed-scale leg with an off-axis child and stretch, one-bone head stretch at mix 0.6 | 35,568 world values (max 0.00004), 33,280 region coordinates and 1,248 IK pose values against the runtime; stretch reaches 2.48× (leg) and 6.1× (head) |
| Oral-artist front, idle male, male pose, cow back (game assets, local only; SHA-256 unchanged) | 31/49/25/31 skins: 83,700/419,832/141,750/156,240 world values (max 0.0004/0.0005/0.0004/0.0002) and 14,534/90,632/59,926/27,378 mesh values (max 0.0005); original vs re-export with physics max bone difference 0.0026/0.0002/0.0003/0.0004 |
| Idle female full runtime check (draw order, color, regions, meshes) | 44,911,776 world values (max 0.0004), 18,121,432 mesh values (max 0.0007), 77,168 draw order checks, 2,292,888 color channels |
| Oral male full runtime check (incl. clipping coverage) | 1,857,024 world values (max 0.0005), 557,570 mesh values, 624 clip checks over 359,424 coverage points (2 edge mismatches), 4,836 draw order checks, 95,264 color channels |
| Game audit | **20 of 21** skeletons import fully; only the bonfire (image sequence, blend mode) remains |
| Rust suite | fmt, clippy `-D warnings`, architecture and build pass; **549** tests pass |


## Image sequences and blend modes, native v22 — November 10, 2026

| Check | Result |
| --- | --- |
| Runtime probes (original) | All seven modes over 22 sample times (table in `sequence_tests.rs`); boundary tolerance 1e-5 frames (bisected); path naming and defaults; delay carry-forward |
| Original Sequence Lab (`sequence-bot.peachy`, frames copied from Peach Bot art): loop → pingpong → once-reverse head frames with a setup frame; additive, screen and multiply slots | 4,160 frame-and-blend checks and 28,080 world values (max 0.00003) against the runtime |
| Bonfire (game asset, local only; SHA-256 unchanged; 96-frame sequence, screen-blended glow mesh) | 156 frame-and-blend checks, 624 world values, 416 mesh and 832 region coordinates match; original vs re-export: max bone difference 1e-6 |
| All 21 labs | Pass the extended runtime check (frame image and blend for every draw) |
| Game audit | **21 of 21** skeletons import fully |
| Rust suite | fmt, clippy `-D warnings`, architecture and build pass; **554** tests pass |
