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
| Ghosting | New Animate onion-skin view: fractional Frames and exact Key Frames, Current keyed-pose comparison, per-side colors, Images/Solid display and opacity, Anchor, On top, Loop, per-frame offsets and typed bone Selection with Lock/Refresh. Renders checked region/mesh/IK poses in the main viewport. |
| Graph | Unimplemented: editable value graph. Current curve controls belong to Dopesheet. |
| Mesh Tools | Separate view unimplemented. Existing attachment Vertices/subdivision controls cover only the documented mesh subset. |
| Metrics | New counts for bones, constraints, slots, all attachments, visible attachments, visible vertices and triangles. Animate additionally shows native scalar-track and key counts. |
| Outline | New clean secondary display of the current evaluated pose, including pending unkeyed edits and static IK. Fit, right/middle drag pan, wheel zoom, and click to center the main viewport. |
| Playback | New Animate panel with session-only Speed (1–200% and percentage presets), shared forward/backward transport and Loop, frame/key navigation and a read-only Timeline FPS. Works with Dopesheet closed. FPS retiming, Stepped and Interpolated overrides remain unimplemented. |
| Preview | Unimplemented: independent playback, tracks and animation mixing. Outline deliberately follows the current pose. |
| Skins | New searchable named-skin list. Click activates, right-click selects Tree properties, Clear skin uses shared default attachments. Single active skin only. |
| Slot Color | Unimplemented: slot-color model and timelines. |
| Timeline | Separate compact view unimplemented. Use Dopesheet. |
| Tree | Existing typed outline, filters, activation dots and contextual properties. The Slots filter shows draw order; Spine has no separate Slots view in this inventory. |
| Weights | Existing bounded Bind/Remove all/Add binding/Direct workflow; still no brush, Update, locks or multi-vertex editing. |

Six views remain unimplemented: **Audio, Graph, Mesh Tools, Preview, Slot Color and Timeline**.

Animations, Ghosting, Metrics, Outline, Playback, Skins and Weights use a resizable tabbed panel in the right rail.
With the Tree closed, they use that rail's full height. Opening an already open
entry focuses its tab. Individual close buttons are present on these views, Tree
and Dopesheet; they reopen from Views. F9 stores/restores all view visibility,
including inactive animation tabs. Restore default workspace closes optional panels
and restores Tree plus the Animate Dopesheet. View changes stay outside document
history. Opening another document retains workspace tabs while clearing searches
and fitting Outline to the new document.

Animations, Ghosting, Playback and Dopesheet are unavailable in Setup; their open
tabs are retained and return in Animate. All unsupported views remain discoverable in both
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

Playback speed changes elapsed playback time in either direction through the same
engine transport used by Dopesheet and A/D. It does not rescale keys, change project
FPS, alter frame/key navigation or create document history. Speed and Loop are
shared across animations and retained through tab/mode changes, F9 and Open; a
new editor session starts at 100%. Mode changes and Open still stop playback. Starting playback and
seeking retain the existing unkeyed-pose discard policy. Speed changes alone
preserve the pending pose. Timeline FPS is displayed without an editable control:
changing FPS correctly needs a separate atomic, frame-preserving retiming command.
Stepped and Interpolated preview overrides are not presented as working settings.
The 1–200% range and preset choices are bounded local policy, not measured
installed-Spine limits.

Ghosting samples committed animation data at the document's FPS without applying
pending unkeyed edits to the ghosts. **Current** therefore compares a keyed pose
with the live unkeyed pose. **Frames** uses before/after reaches and fractional
steps; **Anchor** fixes that sampling grid to frame zero. **Key Frames** samples
exact distinct native key times, with an integer Step selecting every Nth nearest
key on each side. Its key-step and coincident loop-endpoint policies are explicitly
bounded local choices; exact installed behavior remains a comparison gate.

Enable Ghosting in its panel or with **Ghosts** above the viewport. Closing the
panel or using F9 keeps the enabled preview; Setup suppresses it, and Restore
default workspace turns it off. **Loop** applies only while transport Loop is also
on. X/Y offsets are pixels per frame using each sample's signed unwrapped distance,
so they retain direction across loop boundaries. **Selection** includes only slots
on the selected bone; with no selected bone it includes all slots. Lock freezes
that one-bone filter and Refresh captures the current typed selection. Open clears
that lock while preserving appearance preferences for the editor session.

Ghosts never participate in picking, Fit, bounds, keying, document history or
Save/export. They use the checked scene evaluator for regions, weighted meshes
and static IK. Solid display preserves the PNG alpha through an independently
derived white mask; original image pixels and files are unchanged. A request over
64 poses or a failed sampled evaluation shows an error and suppresses the ghost
layer, leaving the current pose usable. Current controls use reaches up to 60
frames and steps down to 0.25 frames. Motion Vectors and Silhouette/Xray outlines
remain pending; the implemented Images and Solid modes are functional rendering.

Behavior references are the public
[Views](https://en.esotericsoftware.com/spine-views),
[UI](https://en.esotericsoftware.com/spine-ui),
[Animations](https://en.esotericsoftware.com/spine-animations-view),
[Skins](https://en.esotericsoftware.com/spine-skins-view),
[Metrics](https://en.esotericsoftware.com/spine-metrics),
[Ghosting](https://en.esotericsoftware.com/spine-ghosting),
[Outline](https://en.esotericsoftware.com/spine-outline),
[Playback](https://en.esotericsoftware.com/spine-playback) and
[Preview](https://en.esotericsoftware.com/spine-preview) guides. Their live content
is not proof of exact installed Spine 4.3 visuals or behavior. Implementation and
icons are independent original Rust code; no proprietary code or UI assets were
inspected or copied.

Desktop Rust interaction checks cover the full inventory, disabled Setup/unsupported
entries, animation/skin activation versus typed selection, document/history
preservation, close/reopen, F9 and Weights restoration, Outline centering with an
unkeyed pose, mode switches and minimum-window close-control visibility. Playback adds typed percentage-entry, shared transport, speed and minimum-panel
visibility checks. Its [render and native-input evidence](evidence/playback.json)
separates diagnostic rendering from the incomplete native mouse/keyboard attempts;
these egui event checks alone do not claim native visual parity.

Ghosting sampling and full-editor controls, including pending-pose comparisons,
weighted/IK geometry and selection/workspace lifecycle, are covered separately in
[Ghosting evidence](evidence/ghosting.json).
