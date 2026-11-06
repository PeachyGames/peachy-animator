# Shear desktop integration

Status: in progress. Paired bone controls, Z routing, seven scalar timeline rows and
shared numeric/viewport transform previews are implemented. Installed-reference,
native and interoperability acceptance remain work. In particular, unsupported
empty-space mapping remains a comparison gate. This does not claim complete Shear
interaction parity or a complete vertical slice.

The strict seven-field bone transform and five-field region transform are supplied
by the core. Region values are promoted for ordinary transform display and checked
when constructing a region command. Shear fields, nudges and gestures require a
typed Bone selection; retained Weights context never redirects them.

Shear X/Y values currently display and edit local angles in every axes mode, labeled
Local. This is an explicit bounded policy. The public guide establishes local
storage but does not settle the installed editor's World/Parent Shear display.
Numeric entry reuses the existing relative operators and Session pose/key boundary.
Normal Shear keying and Auto Key use the core's paired group; scalar tracks remain
individually editable. Setup shear is not normalized into a rotation range.

The toolbar height derives from four transform rows and the ancillary column using
actual shared editor style. The Dopesheet chart derives from Property::ALL, and its
minimum/default panel height includes controls, chart, footer and frame spacing.
A real-style headless clip test checks all seven labels, footer and numeric rows
at the native minimum window size. This does not establish native visual parity.

Sources: [Tools guide](https://esotericsoftware.com/spine-tools),
[Keys guide](https://esotericsoftware.com/spine-keys),
[staff arrow-axis guidance](https://esotericsoftware.com/forum/d/24202-shortcuts-and-macros-and-newby-questions).
The default shortcut is Z, after modified Undo/Redo and text/modal priority.
Left/Right selects the X component; Up/Down selects Y. Existing editor nudge steps
are reused; exact Shear step amounts have not been compared with installed Spine.

The documented presence of empty-space dragging is established. Its exact mapping,
Shear handle sign/shape, Shift behavior, zero axes and center gestures remain
comparison gates. The core rejects unsupported axisNone instead of choosing X.
Tool-math errors are visible; no UI inverse or world-shear decomposition is added.
Reference research lives outside Git and used official guides/staff/tutorial text,
without proprietary implementation or copied UI assets.


## Frozen transform gestures

Numeric entry and viewport tools share one transform capsule: exact typed item,
initial local pose, parent matrix, actual mode/clip/time, tool, axes, Auto Key and
properties. Bone previews use Session::preview_edit_pose from its fixed transaction
baseline, preserving unrelated pending properties and overwritten keys. Setup
region previews use checked RegionTransform conversion and command previews.
The shell performs no inverse or transform decomposition.

Both Shear handles use the core's evaluated directions and single-axis gesture
math. The independent red X and green Y cues are selectable, and degenerate zero
directions do not draw fake handles. Unsupported empty-space input reports the
core error without choosing an axis or silently dropping a failed result.

Numeric Enter/Tab and pointer release settle after all frame actions, as one Undo
entry. Escape, history, context/tool/axes/Auto Key changes, Save/Open/Export,
application focus loss and menus/modals cancel before further actions. Canceled
held pointers and stale numeric responses cannot restart the edit. The Direct
weight and transform owners cancel each other before capturing a new baseline.
Keyboard text undo retains priority; owned numeric Save cancels before writing.
Focused numeric text retains precision; passive rounding or invalid text alone
does not create a pose/history change.

F12 reports transformGesture (frozen item, initialPose, parent, tool, axes,
mode/clip/time/Auto Key, properties, input and error) separately from current
values. Targets include shearTool, value:ShearX/Y, key:ShearX,
timelineRow:ShearX/Y, gizmo:X/Y, gizmoOrigin, mainToolsClip and dopesheetClip.
The diagnostic shearDisplayPolicy explicitly records the bounded local policy.


## Finite scene boundary

Canvas drawing/picking and Weights world-coordinate queries use the engine's
checked scene evaluator with Session's pending locals. A nonrepresentable pose
shows a viewport/status error, skips geometry and handles, and preserves the
original document, selection, authoring time and pending pose. F12 evaluationError
and Weights diagnostic evaluationError expose the failure. Local finite inputs are
not replaced with zero or clamped into a drawable pose.

Camera/geometry projection is also checked before drawing and picking. Ordinary
finite arithmetic is retained, with wider fallback for representable intermediate
overflow. Decorative grid lines use a count derived from viewport width/height
and a wider phase calculation. This avoids a nonadvancing float-addition loop for
large finite camera offsets without changing world geometry or authoring state.
