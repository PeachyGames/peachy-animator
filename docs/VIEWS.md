# Views audit and implemented scope

The old selector contained only **Tree, Dopesheet and Weights** because those were
the only implemented panels. It was not an installation, license or rendering
problem. The rest of Spine's Views had not been built. Setup also hid the working
Dopesheet, which made the gap more apparent.

On November 6, 2026 the public [user guide inventory](https://en.esotericsoftware.com/spine-user-guide)
listed 15 views. The selector now names every one and gives unsupported entries a
disabled state with a concrete explanation. This makes omissions visible without
presenting empty panels as working tools.

| Spine view | Peachy Animator status |
| --- | --- |
| Animations | New searchable animation list in Animate. Click activates without changing the current bone selection; right-click selects its Tree properties without activation; double-click opens the existing Rename dialog. |
| Audio | Unimplemented: audio data, waveforms and playback. |
| Dopesheet | Existing supported bone-transform keys, curves, selection, clipboard and retiming. |
| Ghosting | Unimplemented: sampled poses and controls. |
| Graph | Unimplemented: editable value graph. Current curve controls belong to Dopesheet. |
| Mesh Tools | Separate view unimplemented. Existing attachment Vertices/subdivision controls cover only the documented mesh subset. |
| Metrics | New counts for bones, constraints, slots, all attachments, visible attachments, visible vertices and triangles. Animate additionally shows native scalar-track and key counts. |
| Outline | New clean secondary display of the current evaluated pose, including pending unkeyed edits and static IK. Fit, right/middle drag pan, wheel zoom, and click to center the main viewport. |
| Playback | Separate view unimplemented. Existing transport is in Dopesheet. |
| Preview | Unimplemented: independent playback, tracks and animation mixing. Outline deliberately follows the current pose. |
| Skins | New searchable named-skin list. Click activates, right-click selects Tree properties, Clear skin uses shared default attachments. Single active skin only. |
| Slot Color | Unimplemented: slot-color model and timelines. |
| Timeline | Separate compact view unimplemented. Use Dopesheet. |
| Tree | Existing typed outline, filters, activation dots and contextual properties. The Slots filter shows draw order; Spine has no separate Slots view in this inventory. |
| Weights | Existing bounded Bind/Remove all/Add binding/Direct workflow; still no brush, Update, locks or multi-vertex editing. |

The four new views and Weights use a resizable tabbed panel in the right rail.
With the Tree closed, they use that rail's full height. Opening an already open
entry focuses its tab. Individual close buttons are present on these views, Tree
and Dopesheet; they reopen from Views. F9 stores/restores all view visibility,
including inactive animation tabs. Restore default workspace closes optional panels
and restores Tree plus the Animate Dopesheet. View changes stay outside document
history. Opening another document retains workspace tabs while clearing searches
and fitting Outline to the new document.

Animations and Dopesheet are unavailable in Setup; an open Animations tab is
retained and returns in Animate. All unsupported views remain discoverable in both
modes. This is a local, explicit availability policy. The live public guide says
Spine's lists differ by mode, but does not enumerate the complete per-mode menu.
An exact installed 4.3 menu comparison remains a gate. Preview in Spine can be
useful during Setup, so it is not incorrectly labeled Animate-only here.

The tabs do not yet provide Spine's free drag-and-drop docking, separate minimized
view icons, per-view settings menus, or saved workspace preferences. Animations
does not yet provide New/folders/deletion; Skins does not provide Rename, creation,
ordered pinned composition or folders. Metrics does not estimate clipping, area,
draw calls or runtime vertex-transform cost. Outline has no ghosting or orange
main-viewport indicator. These are bounded panels, not full view parity.

Behavior references are the public
[Views](https://en.esotericsoftware.com/spine-views),
[UI](https://en.esotericsoftware.com/spine-ui),
[Animations](https://en.esotericsoftware.com/spine-animations-view),
[Skins](https://en.esotericsoftware.com/spine-skins-view),
[Metrics](https://en.esotericsoftware.com/spine-metrics),
[Outline](https://en.esotericsoftware.com/spine-outline) and
[Preview](https://en.esotericsoftware.com/spine-preview) guides. Their live content
is not proof of exact installed Spine 4.3 visuals or behavior. Implementation and
icons are independent original Rust code; no proprietary code or UI assets were
inspected or copied.

Desktop Rust interaction checks cover the full inventory, disabled Setup/unsupported
entries, animation/skin activation versus typed selection, document/history
preservation, close/reopen, F9 and Weights restoration, Outline centering with an
unkeyed pose, mode switches and minimum-window close-control visibility. Native
mouse/keyboard acceptance and capture evidence are recorded separately by the
coordinating task; these egui event checks alone do not claim native visual parity.
