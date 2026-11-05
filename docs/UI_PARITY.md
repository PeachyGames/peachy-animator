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
| Views selector at top right | Tree/Dopesheet visibility, workspace restore, F9 hide/restore |
| Viewport left, Tree right | Right-side Tree; item properties below it; resizable panes |
| Setup/Animate at viewport's upper left | Mode switch there; Dopesheet appears in Animate |
| Main transform tools below viewport | Rotate/Translate selectors; numeric rotation, translation and scale |
| Animation views below viewport | Frame-based Dopesheet below main tools, beside the full-height Tree |
| Dense neutral dark UI, orange emphasis | Initial independent styling; detailed metrics/icons remain to align |
| C / V tools | Rotate / Translate; right-click switches the last two tools, right-drag pans |
| D playback; R / F frames | Forward play/stop, next/previous frame; Shift advances ten frames |
| Space / Escape selection | Deselect; Escape first cancels an active mesh drag |
| L / Cmd/Ctrl+L keying | Active supported transform / all supported transforms for selected bone |
| Cmd/Ctrl+R looping | Toggle loop without also stepping a frame |
| Arrows nudge | Selected bone's parent-space translation or selected setup mesh vertex |

This removes the prototype's conflicting R-to-rotate, F-to-fit, Space-to-play,
arrow-to-step and custom M-to-mesh bindings. Fixed-UV setup vertices remain in
attachment properties. Interpolation labels are Linear, Stepped and Bezier.

## Remaining required work

- Align detailed spacing, fonts, control sizing, tool glyphs and visual states using
  Setup/Animate reference screenshots at matched viewport sizes.
- Extend the typed Tree with multi/range selection, selection history, rename/create
  workflows and drag/drop parenting/draw order. Introduce stable IDs before deletion
  or reparenting. Exact glyphs, spacing and every Tree preference remain to align.
- Match main tool organization, Scale/Shear/Pose/Create tools and axis selectors.
  Numeric scale controls currently have no canvas Scale tool.
- Add movable tabs, docking, saved workspace state and the Graph view. Tree and
  Dopesheet currently provide resizers/visibility controls only.
- Add selection history, multiple/box selection, reverse play, key navigation and
  view-dependent input focus. Extend Auto Key/unkeyed editing to new keyable types
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
