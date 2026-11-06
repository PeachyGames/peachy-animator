# Shear desktop integration

Status: in progress. Paired bone controls, Z routing and seven scalar timeline rows
are implemented. The shared frozen transform gesture lifecycle and native reference
comparison remain acceptance work. This checkpoint does not claim Shear interaction
parity or a complete vertical slice.

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
