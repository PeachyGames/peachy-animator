# Spine workspace and interaction parity

Chris's November 5 requirement is to reproduce Spine's visual layout, controls,
terminology and interaction conventions so a Spine user can use familiar habits.
UI parity is an acceptance criterion for every vertical slice. Match the reference
before adding a different arrangement or shortcut. Retain Peachy Animator's own
identity and independently written implementation and icons.

The behavioral reference is the installed Spine Editor 4.3.26, using our original
Peach Bot export imported into a disposable `.spine` project. Public references:
[UI guide](https://esotericsoftware.com/spine-ui),
[Views guide](https://esotericsoftware.com/spine-views),
[Tools guide](https://en.esotericsoftware.com/spine-tools/), and
[default shortcuts](https://esotericsoftware.com/spine-cheat-sheet).
The baseline is the documented default workspace; user layouts/hotkeys can differ.

## First alignment pass

| Reference convention | Current alignment |
| --- | --- |
| Menu and file/history shortcuts at top left | Compact Peachy menu, Open/Save/Undo/Redo; export in menu and Cmd/Ctrl+E |
| Views selector at top right | Full 15-view public-guide inventory, seven bounded working views, explicit disabled gaps, close/reopen, workspace restore and F9 |
| Viewport left, Tree right | Right-side Tree; item properties below it; resizable panes |
| Setup/Animate at viewport's upper left | Mode switch there; Dopesheet appears in Animate |
| Main transform tools below viewport | Rotate/Translate selectors; numeric rotation, translation and scale |
| Animation views below viewport | Frame-based Dopesheet below main tools, beside the full-height Tree |
| Dense neutral dark UI, orange emphasis | Initial independent styling; detailed metrics/icons remain to align |
| C / V / X tools | Rotate / Translate / Scale; right-click switches the last two tools, right-drag pans |
| D playback; R / F frames | Forward play/stop, next/previous frame; Shift advances ten frames |
| Space / Escape selection | Deselect; Escape first cancels an active mesh drag |
| L / Cmd/Ctrl+L keying | Active supported transform / all supported transforms for selected bone |
| Cmd/Ctrl+R looping | Toggle loop without also stepping a frame |
| Arrows nudge | Active transform in the selected axes; Shift fine steps; explicit setup mesh vertices |

This removes the prototype's conflicting R-to-rotate, F-to-fit, Space-to-play,
arrow-to-step and custom M-to-mesh bindings. Fixed-UV setup vertices remain in
attachment properties. Interpolation labels are Linear, Stepped and Bezier.

## Remaining required work

- Align detailed spacing, fonts, control sizing, tool glyphs and visual states using
  Setup/Animate reference screenshots at matched viewport sizes.
- Extend the typed Tree with multi/range selection, creation and remaining rename
  workflows and drag/drop parenting/draw order. Introduce stable IDs before deletion
  or reparenting. Exact glyphs, spacing and every Tree preference remain to align.
- Complete main tool organization with Shear/Pose/Create, full Weights editing and compensation.
  Rotate/Translate/Scale and the axis selectors now cover the supported transform subset.
- Add movable tabs, docking, saved workspace state and the Graph view. Tree and
  Dopesheet currently provide resizers/visibility controls only.
- Add multiple/box selection and complete view-dependent input focus/navigation
  preferences. Extend Auto Key/unkeyed editing to new keyable types
  as their model and adapters are implemented.
- Match Dopesheet key selection, dragging, clipboard, pan/zoom and selection tools.
- Implement Edit Mesh UV/topology controls with their own reference semantics.
  Fixed-UV setup deformation must retain its distinct behavior.

## Verification gate

Observe the installed-editor interaction or record its public behavior reference,
implement through the shared core, then check the running native application's
placement and real mouse/key behavior. Verify mode changes, selection, gestures,
keying and undo with an original fixture. Record remaining differences here.
Export/runtime checks establish separate data compatibility, not UI parity.

Local reference captures stay outside Git. Repository screenshots contain our own
application and original artwork. No proprietary implementation or UI assets are
extracted or copied into the project.

## Tree and selection slice

The Tree now distinguishes skeleton, bones, slots, skin placeholders, attachments,
skins and animations. Names select their own properties. A skin/animation's visibility
dot activates it separately, following the public
[Tree](https://en.esotericsoftware.com/spine-tree),
[Skins view](https://esotericsoftware.com/spine-skins-view), and
[Animations view](https://en.esotericsoftware.com/spine-animations-view) behavior.
Named skin placeholders show the active attachment; default attachments remain
available through fallback. Show all skin attachments exposes inactive appearances
for inspection, with editing blocked until their skin is active.

Search retains ancestors and reveals matches through collapsed branches. Type filters
include quick Bones/Slots/Attachments views. Slots without bones are projected in
draw order. Attachment-filtered viewport picking selects a slot; a selected slot has
no implicit bone transform target. Branch arrows toggle a node, right-click toggles
its descendants, and expand/collapse-all controls are available.

Bone/slot visibility dots are temporary editor state. Setup attachment dots change
which placeholder the slot shows and are undoable document edits. That distinction
follows the [slots guide](https://en.esotericsoftware.com/spine-slots/) and
[keying guide](https://en.esotericsoftware.com/spine-keys). Attachment timelines in
Animate remain unsupported; these controls do not silently change the setup pose.
On macOS, the app's own standard Hide menu relinquishes Cmd+H so Show All can receive
it; the menu action remains available. No other application's menu is changed.

Setup region transforms, bone length and slot setup attachment use shared commands.
The transform toolbar acts only on the selected bone or an editable setup region.
Mesh vertex controls require selecting the mesh itself. Pointer gestures commit as
one history entry and Escape cancels them. Viewport picking uses triangles in reverse
slot order; transparent pixels are not sampled. Pixel-aware picking, mesh-wide
transforms, multi-selection and full Tree parity remain future work.

## Unkeyed pose and Auto Key slice

The main transform toolbar has **Auto Key** and **Key Edited**; **K** keys edited
properties across all bones, including unselected ones. This is called `Key Edited`
in the current [keying guide](https://esotericsoftware.com/spine-keys). Each supported
transform group has a green (absent), orange (edited), or red (keyed) key button.
Bones with pending edits have an orange Tree key button that keys only that bone.
**L**, **Cmd/Ctrl+L**, and explicit transform buttons key regardless of Auto Key.
Translation and scale key both axes by default; the underlying scalar model and
imported timelines remain intact. Separate-axis authoring preferences remain pending.

With Auto Key off, numeric/canvas/arrow edits change a transient multi-bone pose,
never setup data or animation keys. Gestures still have one-step undo and cancellation.
Moving the timeline clears pending edits; with no redo branch, Undo recovers their
absolute local values at the new frame. Starting playback also clears them. Switching
animation or Setup/Animate clears transient pose history to avoid stale edits crossing
contexts. Save/export exclude pending values; the status bar and close/open discard
dialog distinguish them from saved keys. Toggling Auto Key alone does not key a pose.

This completes bone transform keying for the current supported subset. Slot/deform/
constraint keying, fractional-frame authoring, complete Dopesheet behavior and exact
matched-size reference screenshots remain future parity gates.

## Transform tools and axes slice

The main toolbar now has Rotate (C), Translate (V), Scale (X), and the adjacent
vertical Local/Parent/World axis selectors. Translate and Scale draw independently
implemented red X / green Y handles; the drag target is captured at pointer press.
Scale handles always use local axes, including reflected scale. Parent/local/world
numeric conversion and gesture geometry live in the engine. World scale is displayed
for inspection; focused entry and numeric dragging use local scale, as documented by
Spine. World rotation entry specifies a direction while local animation rotation can
retain full turns. A zero-scale parent blocks inverse world translation/rotation.

Numeric entry commits on Enter/Tab and supports +value, +-value, *factor and /divisor.
Arrow keys now edit the active transform; Shift reduces the step. This corrects the
prototype's translation-only arrow behavior. Rotate gestures retain 15-degree Shift
snapping. Scale and axis edits use the existing Auto Key/unkeyed/undo paths.

The installed reference project opened as a task-owned on-screen Spine 4.3.26 window;
macOS refused its window screenshot. The public Tools toolbar and axes images were
visually inspected instead, and remain outside Git. Their layout places the transform
column next to the vertical axis selectors. Full matched-size visual comparison,
Shear/Pose/Create/Weights, compensation and all transform preferences remain pending.

## Selection history and groups slice

Page Up restores the previous selection and Page Down advances through selection
history. Deselecting is a remembered state; making a new selection branches history.
Cmd/Ctrl+0–9 stores a selection and 0–9 recalls it, following the public Tools guide.
Storing no selection clears that number. These are single-item groups until multiple
selection is implemented. Groups/history belong to the current editor session and
reset on opening a different document; persistent group/workspace preferences remain
pending.

History and groups retain typed items: recalling a slot cannot edit a bone; recalling
a skin inspects it without activating it. Invalid items are pruned after document
history changes. Recalled items use the same Tree reveal, properties and viewport
selection path as ordinary selection. Document undo never navigates selection.
Text fields retain digit input. Clicking a transform number stops playback before
entry; idle playback frames do not allocate numeric-edit state.

## Animation transport slice

A plays backward/stops and D plays forward/stops. Pressing the other direction while
playing switches direction. Independently drawn reverse/forward buttons use the same
transition. Looping wraps in either direction, including reversing from frame zero.

W/S navigate the nearest next/previous distinct key in the active animation's supported
tracks, and stop playback. Q goes to zero; E goes to the actual last key.
Key navigation uses the same
unkeyed-pose discard/recovery path as ordinary timeline movement. Selection and document
data do not change. Text-entry focus retains priority over these shortcuts.

Playback remains bounded to the native clip duration when Loop is off. Spine can play
outside the keyed range; that behavior, loop ranges, playback preferences, selected/
filtered timeline navigation policies and multiple active skeletons remain pending.
Current key navigation searches all supported bone tracks of the active animation.

## Interface icon cues

The titlebar uses independently drawn folder, disk, backward-history and forward-
history icons, with named shortcut tooltips and disabled states. File-menu entries
reuse those icons; Export uses an outward arrow. Views has a divided-window icon
and dropdown corner cue. Every implemented Views menu entry has an icon; Tree and
Dopesheet reuse theirs in their headings. View visibility retains its checked state,
and Hide/Restore views changes both its symbol and label. See
[the iconography contract](ICONOGRAPHY.md) for the shared vocabulary and remaining
coverage. No proprietary UI image or font is bundled.

## Naming and Dopesheet authoring slices

Bone, slot and animation names use the Tree properties Rename action and a modal
name dialog. F2 and double-clicking a supported Tree name open the same dialog,
following the public [Tree guide](https://esotericsoftware.com/spine-tree).
Enter/OK confirms; Escape/Cancel discards the draft. Blank or duplicate names leave
the document and history unchanged and show the error in the dialog. Names can be
changed in either Setup or Animate without altering transform keys. Multi-item,
skin, placeholder and attachment renaming remain pending.

Animation properties provide Duplicate with an explicit new-name dialog. The copy
retains duration, keys and curves and becomes the inspected selection. The active
animation stays unchanged until explicitly activated. This activation policy is a
bounded implementation choice; exact installed-editor duplication focus/activation
behavior has not yet been established. Creation/deletion/folders and full animation
management remain incomplete.

Dopesheet clicks select exact source keys independently of playhead position.
Cmd/Ctrl-click toggles keys; Delete/Backspace requires focus in the visible canvas.
Double-click deletes the clicked key. The toolbar offers the same deletion command
with an independent icon. Rotation/translation/scale keys use narrow green/blue/red
bars, following the public row images. Only the selected bone's supported scalar
tracks are covered; multi-bone rows, overview rows, box selection
and Graph synchronization remain pending. Jump-to-key is on;
configurable selection/scrubbing preferences remain pending.


## Key retiming and clipboard slice

Drag a selected key to move its selected group. An unselected key starts a new
single-key group when dragged. The pressed key's destination snaps to frame time;
other keys retain their relative offsets. Shift disables snapping. Cmd/Ctrl+Shift
captured at press duplicates the group. Source times remain exact, including
fractional keys closer than the usual keying tolerance. Ordinary clicks do not
snap an existing fractional key. Dragging preserves the playhead; a click jumps
to the clicked key. Negative destinations are clamped at the earliest source key.

A preview replaces exact destination collisions. Moving away restores those keys
from the transaction baseline; Escape or context/save/export/focus changes cancel
it. Release commits one undo step. This is bounded scalar-track retiming, informed
by the public [Dopesheet guide](https://esotericsoftware.com/spine-dopesheet#Moving-keys)
and [Keys guide](https://esotericsoftware.com/spine-keys). Timeline zoom/pan,
auto-pan, persistent selection boxes/scaling and full gesture preferences remain
pending. Duration now follows the highest key, including shrinkage when the last
key moves earlier; the display range remains separate.

Copy/Cut/Paste icons have the same session actions as focused Cmd/Ctrl+C/X/V.
Copy stores committed selected keys with relative times, scalar values and curves;
Paste uses the current bone/playhead. Cut deletes as one command before replacing
the clipboard. The buffer survives bone/animation changes and clears on opening a
project. Pending unkeyed poses remain separate and are called out in status text.
Undo/Redo clears key selection to avoid selecting restored collision destinations
by stale time identity.

Explicit Copy/Cut also writes a versioned text envelope to the system clipboard.
Keyboard Paste accepts only this session's unchanged envelope; arbitrary external
text and another application's clipboard edits cannot paste a stale internal buffer.
The toolbar Paste uses the internal buffer. Cross-project/system import and full
Spine clipboard format interoperability remain pending. Text fields, modal dialogs,
hidden views and Setup retain their own shortcut behavior. Interpolation controls
resolve the exact playhead key before considering a unique near match, and refuse
ambiguous near matches.


## Selection expansion and row controls

With Dopesheet focus, Cmd/Ctrl+A expands an existing selection to all keys in its
selected property rows, then to every shown key. With no keys selected it selects
all shown keys immediately. Further presses keep that selection. A held chord does
not advance phases. Manual selection, replacement, context changes and stale-key
pruning reset progression; ordinary frame reconciliation does not. Singleton rows
still consume the first phase even when their selected-key count stays unchanged.
Keys remain scoped to the current bone/animation; no hidden bone keys are included.

This follows the public [selection guide](https://esotericsoftware.com/spine-dopesheet#Selection)
and [official clarification](https://esotericsoftware.com/forum/d/17539-select-bones-keyframes-in-timeline-hotkey).
The selected-row union for a multi-row selection is a conservative implementation
choice pending installed-editor comparison. Overview/all-bone rows remain pending.

Selection and Copy/Cut/Paste/Delete share an ordered focused input stream. Delete
before Select All removes only the earlier selection; the reverse deletes expanded
keys. Text fields, modal dialogs and hidden/Setup views retain their inputs. Plain A
retains backward playback. Releases outside the Dopesheet still reset the held-key
latch. Active previews cancel to their prepress selection before expansion, retaining
the document, unkeyed pose and redo branch. Global history shortcuts remain outside
this stream: same-frame Undo followed by Select All currently leaves selection
empty, a known selection-order gap for a future unified shortcut dispatcher.

Clicking Rotation chooses Rotate, X/Y choose Translate and Scale X/Y choose Scale.
Row labels expose named button roles, shortcut hints and a hover cue. The label
column owns its input separately from the key grid, so label clicks cannot start
scrubbing or retiming. Right-click behavior is unchanged. A permanent single-line
footer keeps row geometry stable when selection changes.


## Animation end and authoring range

The frame counter shows cursor / highest keyed frame. An empty clip or keys only
at frame zero end at zero. The Dopesheet still displays at least one second and
includes the cursor, so it remains usable without reserve keys. This is a bounded
view policy until proper timeline pan/zoom, not a claim of exact Spine zoom behavior.
Keys and the playhead use the positive display extent; retiming freezes that extent
for the gesture even while the animation end changes.

Seeking, frame stepping and unkeyed posing can pass the last key without adding
keys. Keying there extends the animation; moving/deleting the last key shrinks it.
Undo/Redo and cancellation restore keys and therefore the end. The cursor remains
where the artist put it. A frame-zero key can be dragged into positive time.

For zero-length clips, current playback safely stops at zero in either direction,
including Loop. Nonlooping playback remains bounded to the last key. Exact Spine
empty-playback behavior and extended nonlooping transport remain comparison gates;
the installed-editor data round trip establishes file semantics, not those UI rules.

## Weighted inspection slice — November 6, 2026 UTC

Weighted mesh data now reaches the existing renderer and picking/bounds path.
Select the weighted attachment, choose **Vertices**, then click a numbered handle
or use **Vertex** to inspect its bone influences. Each stored entry is shown with
its bone name and a percentage; repeated and zero entries stay separate. Display
rounding does not change stored values. This is an inspection extension to the
existing properties panel, not an implemented Spine Weights view or Weights tool.

Inspection stops playback but preserves Setup/Animate, the current frame and an
unkeyed pose. Handles follow the posed world geometry in Animate. Weighted handle
drags and arrow nudges have no editing effect; local-coordinate fields are absent,
and subdivision is disabled. The same operations are rejected by the shared engine.
Returning to bone tools restores the normal pose/key/history paths. Native save
retains native v4 influences and excludes an unkeyed pose as before.

The properties scroll area retains its requested pane height as content changes.
Short initial bone properties no longer collapse the pane before a longer influence
list is selected. Native
verification exercised posed inspection, edit guards, restored bone tools, keying,
Undo/Redo and two save backups, then unweighted drag/subdivision. The working document
and original clipboard were restored. See [acceptance](evidence/weighted-native.json),
[weighted inspection](weighted-editor.png), and
[unweighted regression](weighted-unweighted-regression.png).

Bind/Unbind, Direct/brush editing, normalized weight changes, locks, Update bindings,
pies/overlay, weighted vertex transforms and full Weights-view layout remain pending.
Their shortcuts, selection state and Setup/Animate conventions need dedicated
acceptance. Runtime coordinate agreement does not establish that UI parity.

## Initial Bind and Weights view — November 6, 2026 UTC

This extends the inspection-only slice above. G selects a genuine Weights tool and
opens a named resizable Weights view; the Views menu and heading share its independent
vector icon. F9 hide/restore includes the view. G after hiding restores the saved
workspace and opens Weights. The right-column placement is provisional: public
control references do not establish the installed editor's exact default dock, and
movable tabs/arbitrary docking remain pending.

The view names the explicit mesh and binding bone. Bind supports mesh-first or
bone-first interaction with one selected target. No edit occurs on mode entry.
A concrete target click dispatches initial binding and ends the attempt; success
or failure never automatically retries. Space/Escape or Bind again exits without
rolling back completed work. Remove all is explicitly complete removal, rather
than implying that only the highlighted bone will be removed.

Mesh, vertex and bone inspection context survives normal transform-tool changes,
mode/animation changes and posing while the active appearance remains valid.
Normal bone edits still use the typed Tree selection. Binding mode itself cancels
on mode/animation changes, Undo/Redo, selection-history navigation, project open and view
hide/restore. Scrubbing can retain mode and captures the live pose on target click;
that policy is not claimed as observed Spine behavior. Inactive appearance or
slot-placeholder changes clear the context instead of redirecting a later edit.

G, Space/Escape and ordinary shortcuts respect text, menu and modal priority.
Weights focus admits C/V/X and Save/Undo. Genuine Weights participates in the last
two tools; temporary Mesh inspection preserves its independent return tracking.
The read-only bone list uses independent color cues shared by the viewport.
Weights percentages do not mutate or normalize imported influence entries.

The completed native run verifies G/C/right-click, retained vertex selection,
F9-to-G reopening, Setup Remove-all/Bind with zero observed current-shape error,
explicit weight-1 data, two verified Save backups plus Undo/Redo, pending Animate
locals/time with Remove-all, and Escape exit. It restores the working document/file
semantically at f32 precision and restores the original system clipboard; the raw
initial copy remains retained separately. See [native acceptance](evidence/binding-native.json)
and [the Weights view](binding-editor.png). Screenshot framing is not a claim of
pixel parity or exact Spine docking.

The functional run used `ca457f5`. The final explanatory-hint-only correction at
`a891007` passed the same 162 Rust tests and format/lint/build/architecture checks,
plus a separate focused native visual check,
with its document/file unchanged. The screenshot shows that final build. See
[final visual acceptance](evidence/binding-visual.json); it is separate from the
full functional run rather than a second full-run claim.

Direct/brush editing, additional bindings, Auto, Update, locks, weight clipboard,
multiple vertex/bone selection, pies/overlay and weighted vertex/topology editing
remain pending. No fake active controls advertise those features.

## Additional Bind and Direct controls — November 6, 2026 UTC

The existing Weights pane extends Bind to a genuinely new bone on an already
weighted mesh. A concrete target click appends that bone at 0% on every vertex;
already-bound bones fail explicitly and end the attempt without retry. Initial
Bind and Remove all keep their prior behavior. The pane retains its independent
icon/swatches and explicit mesh/vertex/bone context.

**Mode: Direct** is a static label for the sole implemented mode. **Weight** has a
percentage slider and numeric entry for one vertex and one existing bound bone.
The remaining positive weights share the remainder proportionally. Sparse or
repeated selected-vertex bindings show an unavailable reason. Imported percentages
are display-rounded without changing raw values merely on focus/Tab/blur; invalid
numeric text alone also leaves them and redo intact. Starting a Weight control
from a normal transform tool activates genuine Weights with coherent tool history.

A gesture freezes target, Setup/Animate mode, clip, actual time and one posed-local
snapshot. Each preview uses the same transaction baseline. Pointer release,
Enter/Tab or ordinary blur commits a valid change as one undo step after frame
actions and pointer-up. Escape, application focus loss, tool/mesh/vertex/bone/skin/
time/mode/clip changes, history, Open/Save/Export and view hide cancel first. Stale
numeric responses and held-pointer movement stay suppressed until release, so a
canceled weight cannot return on mouse-up. Undo/Redo during a preview cancels it
without consuming older history. Weight text retains text undo; owned-field
Cmd/Ctrl+S cancels then saves.

Edits preserve pending bone pose and create no keys. Time/context actions keep
their established pending-pose policy after canceling Direct. Changing weights
may deform a vertex; the UI does not compensate its bind position. The pane is
still provisionally docked in the right column. Exact installed-Spine placement,
vertical viewport weight dragging, multiple vertices/bones, brushes, Auto, locks,
Update, individual removal, pies/overlay and weight clipboard remain pending.

Core/CLI/runtime checks and headless input regressions do not establish native
pointer/text behavior or visual parity. The full native run at `694576a` separately
verifies scratch A=hand.R/B=forearm.R Bind/Add0 with zero observed current-shape
error, selected vertex 4's exact 30%/70% numeric result, Save and Undo/Redo, numeric
Escape and a real toolbar Save click, held-slider Escape/Cmd+S followed by further
movement/release without a ghost edit, and one-step slider history.

In Animate, Direct automatically activates genuine Weights from Rotate. The 45%
edit retains an unkeyed 13-degree rotation, exact pending locals/time and existing
keys; Save excludes that pending pose. The full run restores the working document
and file semantically at f32 precision and restores the original clipboard. Five
Save backups are verified, including the exact initial raw working copy.
See [native acceptance](evidence/direct-native.json).

[Visual review](evidence/direct-visual.json) of the same run verifies loaded
original artwork, independent Weights icon/bone colors and actual 30%/45% data and
controls. [Setup capture](direct-editor.png) and [posed capture](direct-posed.png)
show our editor only. This is neither a separate second full native run nor exact
installed-Spine docking, pixel-equivalence or full interaction-parity acceptance.


## Bone Shear and shared transform input — November 6, 2026 UTC

The fourth transform row provides bone-only **Shear [Z]**, local-policy X/Y entry
and paired keying. Its independent red X/green Y handles follow evaluated bone
axes. Toolbar and Dopesheet dimensions derive from the shared real editor style;
all seven rows and the footer are visible in the reviewed native Animate capture.
Region/slot selection cannot redirect Shear through a cached bone or Weights target.

Numeric and viewport gestures freeze the typed selection, authored locals/parent,
tool/axes, mode/clip/time and Auto Key. They share Session's fixed transaction
baseline and one history stack. Context/history/file actions cancel first; stale
blur and held-pointer input stay suppressed. Numeric text commits on Enter/Tab;
Escape or Save abandons uncommitted text. Tab-focused non-text buttons retain
global Undo/Redo/file shortcuts without stealing text undo or widget arrow input.

The final native run verifies typed Setup values, both handles, Undo/Redo, real Save
clicks and keys, held-pointer cancellation, pending Animate values, paired keying,
Auto Key and persistence. See [native evidence](evidence/shear-native.json),
[Setup capture](shear-editor.png), [posed capture](shear-posed.png), and
[visual review](evidence/shear-visual.json). The native-edited document also passes
packed runtime comparisons with pending rotation excluded from the export.

Exact installed-Spine handle shapes/signs/Shift behavior, empty-space mapping,
World/Parent Shear display and Local translation under explicit shear remain
comparison gates. The installed-editor checks are CLI data round trips. Fresh GUI
Screen Capture preflight was false; no proprietary UI screenshot or interactive
Shear comparison was obtained. Local labeling is a bounded policy rather than a
claim of observed exact display behavior.

## Normal IK Phase A — November 6, 2026 UTC

Constraints have a typed Tree folder and independently drawn cues on affected and
target bones. Setup properties follow the observed public guide: an IK Constraint
heading, linked Parent/Child/Target rows, a target pencil chooser, Positive in the
Options row and a numeric Mix field before its slider. Rename uses the existing
name dialog, F2 and double-click conventions. Selecting a linked target and pressing
V allows the usual viewport translation. Animate displays static settings read-only.

Native OS input verifies typed Mix, held slider cancellation, numeric Escape/Save,
Positive, Rename, chooser cancellation/retargeting and one-step history. Setup
rotation preserves authored winding on directly constrained bones (720 + 10 becomes
730); displayed solved motion does not overwrite authored values. Pending target
poses stay outside Save. Bind/Remove all captures the applied frame once, and
Direct weights retains the pending pose through Save and Undo/Redo.

Public guide image version is unknown and includes older controls. Its layout is
useful reference evidence, not proof of exact installed 4.3 UI parity. Our original
captures are `ik-editor.png`, `ik-posed.png` and `ik-weights.png`; proprietary guide
images are retained outside Git and are not application assets. Exact installed
interaction/visual comparison, partial-Mix World/Parent behavior, constraint
creation, animated settings and advanced controls remain gates. No fake creation
wizard or active unsupported option is shown. See `evidence/ik-visual.json` and the
[UI contract](slices/ik-ui.md).

## Views inventory and auxiliary panels — November 6, 2026 UTC

The selector previously listed only the three implemented views. The new
[Views audit](VIEWS.md) accounts for all 15 public-guide views and explains the
remaining eight disabled entries. Animations, Skins, Metrics and Outline now have
real panels using the existing document, Session and checked evaluated scene.
The right rail retains Tree and its contextual properties, with a resizable tabbed
auxiliary area. Menu entries open/focus views; close controls and F9 allow individual
and global dismissal. Weights shares these tabs without changing its command or
transaction boundaries. All view changes remain presentation state.

Animations click activates, right-click inspects its Tree item and double-click
uses Rename. Skins click activates one named skin, right-click inspects and Clear
skin restores default attachments. These panels intentionally omit unsupported
creation/folders and skin composition. Metrics uses actual stored/visible geometry
counts, plus explicitly named native scalar-track/key counts in Animate. Outline
renders the current pose without bone or selection overlays; pan/zoom are separate
from the main viewport and clicking centers that viewport. It is distinct from
Spine's independent Preview and does not implement playback mixing.

Rust egui interaction checks verify menu coverage and gates, typed selection versus
activation, unchanged document/history, close/reopen, F9/Weights restoration,
pending-pose preservation on Outline clicks and close controls at the minimum
window size. General tab docking, saved layouts, minimized icons, exact installed
4.3 per-mode lists and native visual equivalence remain comparison gates. Public
guides are live/unversioned; they do not establish exact installed pixel parity.
