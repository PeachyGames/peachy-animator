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
- Complete the typed Tree for bones, slots, attachments, skins and animations,
  with reference selection/filter/visibility behavior and contextual properties.
- Match main tool organization, Scale/Shear/Pose/Create tools and axis selectors.
  Numeric scale controls currently have no canvas Scale tool.
- Add movable tabs, docking, saved workspace state and the Graph view. Tree and
  Dopesheet currently provide resizers/visibility controls only.
- Add Auto Key off/unkeyed poses, Key Changed (K), selection history, multiple/box
  selection, reverse play, key navigation and view-dependent input focus. Auto Key
  currently remains enabled; implemented shortcuts cover the supported subset.
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
