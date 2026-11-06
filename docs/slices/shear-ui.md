# Shear desktop integration

Status: bounded authoring/data slice implemented and native-verified on November 6,
2026. Paired controls, Z routing, seven rows and shared fixed-baseline previews pass
native, runtime and installed-editor data checks. Full Shear interaction parity
remains in progress: empty-space mapping, Shift, exact handle cues and World/Parent
display still need installed-reference observation. See [validation](../VALIDATION.md#bone-shear-native-v5-and-shared-transform-input--november-6-2026-utc).

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


## Native acceptance

The final source `775c7e3` passes 242 tests and the full root checks. A task-owned
native run verifies 23.5/-17.25 Setup input with Enter/Tab, two-step Undo/Redo, both
axis handles as one history step, held-pointer Escape/Save suppression, numeric
Escape/toolbar/owned-field Save, pending Animate shear, paired keying and Auto Key.
Transform text updates on commit; numeric Escape/Save abandons uncommitted text.
The test does not claim model updates while typing before Enter/Tab.

Global file/history actions work over a non-text button focused by Tab, while text
undo and popup/modal priority remain. Both native behavior and actual-context
regressions cover this distinction. The restored working file matches the original
at f32 precision; seven backups are verified, including exact initial raw bytes.
The original clipboard is restored and the task-owned editor is closed.

[Setup](../shear-editor.png) and [Animate](../shear-posed.png) show original artwork,
local-policy Shear values and the fully visible seven-row chart/footer. This is
visual review of our editor, not an installed-Spine pixel comparison.
