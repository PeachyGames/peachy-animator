# Views audit and implemented scope

Implementation progress: **15 of 15 views implemented**. The latest completed
view is **Audio**. Every named view now has functional controls within the
explicit scope below. Feature-level parity gaps remain listed; this is not a
claim that every Spine feature is supported.

The old selector contained only **Tree, Dopesheet and Weights** because those were
the only implemented panels. It was not an installation, license or rendering
problem. The rest of Spine's Views had not been built. Setup also hid the working
Dopesheet, which made the gap more apparent.

On November 6, 2026 the public [user guide inventory](https://en.esotericsoftware.com/spine-user-guide)
listed 15 views. The selector now names every one and opens a working view. Mode-specific
availability and bounded feature scopes are explicit below.

| Spine view | Peachy Animator status |
| --- | --- |
| Animations | New searchable animation list in Animate. Click activates without changing the current bone selection; right-click selects its Tree properties without activation; double-click opens the existing Rename dialog. |
| Audio | New session-only WAV waveform and independent audition view: Load, Play/Pause/Stop, seek, zoom/pan, Loop, Volume and Mute. Native macOS output uses the validated byte snapshot. Audio event timelines, animation synchronization and audio Save/export remain pending. |
| Dopesheet | Existing supported bone-transform keys, curves, selection, clipboard and retiming. |
| Ghosting | New Animate onion-skin view: fractional Frames and exact Key Frames, Current keyed-pose comparison, per-side colors, Images/Solid display and opacity, Anchor, On top, Loop, per-frame offsets and typed bone Selection with Lock/Refresh. Renders checked region/mesh/IK poses in the main viewport. |
| Graph | New editable scalar graph for the selected bone and seven supported transform properties. Exact-key picker, engine-sampled curves, vertical value dragging, Fit and Zoom Keys. Existing key times/curves stay fixed; tangents, retiming and multiple curves remain pending. |
| Mesh Tools | New soft-selection view with Size, Feather and Hull vertices. Affects neighboring vertices during unweighted Setup vertex drags and arrow nudges, with falloff colors and radius rings. Weighted/Animate transforms and multi-vertex selection remain pending. |
| Metrics | New counts for bones, constraints, slots, all attachments, visible attachments, visible vertices and triangles. Animate additionally shows native scalar-track and key counts. |
| Outline | New clean secondary display of the current evaluated pose, including pending unkeyed edits and static IK. Fit, right/middle drag pan, wheel zoom, and click to center the main viewport. |
| Playback | New Animate panel with session-only Speed (1–200% and percentage presets), shared forward/backward transport and Loop, frame/key navigation and a read-only Timeline FPS. Works with Dopesheet closed. FPS retiming, Stepped and Interpolated overrides remain unimplemented. |
| Preview | New independent keyed-pose animation/skin display, own forward/reverse clock, Loop, Time, Speed, Fit/Bones, pan and zoom. Works during Setup and excludes pending unkeyed edits. Single track; mixing and multiple tracks remain pending. |
| Skins | New searchable named-skin list. Click activates, right-click selects Tree properties, Clear skin uses shared default attachments. Single active skin only. |
| Slot Color | New inline Setup HSV/Hue/Alpha chooser and exact #RRGGBBAA entry for a typed slot or attachment's owner. Persistent native7 RGBA tint reaches all image views. Animate is read-only; color timelines/dark tint remain pending. |
| Timeline | New compact Animate ruler below the viewport. Shared cursor/transport/Loop, exact all-track key markers, decimal frame entry, snapped or Shift-subframe scrubbing, wheel/slider zoom, right-drag pan and Zoom Keys. Works with Dopesheet closed. |
| Tree | Existing typed outline, filters, activation dots and contextual properties. The Slots filter shows draw order; Spine has no separate Slots view in this inventory. |
| Weights | Existing bounded Bind/Remove all/Add binding/Direct workflow; still no brush, Update, locks or multi-vertex editing. |

All 15 views now have bounded implementations; no view remains a disabled placeholder.

Animations, Audio, Ghosting, Graph, Mesh Tools, Metrics, Outline, Playback, Preview, Skins, Slot Color and Weights use a resizable tabbed panel in the right rail.
With the Tree closed, they use that rail's full height. Opening an already open
entry focuses its tab. Individual close buttons are present on these views, Tree
and Dopesheet; they reopen from Views. F9 stores/restores all view visibility,
including inactive animation tabs. Restore default workspace closes optional panels
and restores Tree plus the Animate Dopesheet. Timeline has its own closeable
compact panel below the viewport and main transform toolbar, above Dopesheet when
both are open. View changes stay outside document
history. Opening another document retains workspace tabs while clearing searches
and fitting Outline to the new document.

Animations, Ghosting, Graph, Playback, Timeline and Dopesheet are unavailable in Setup; their open
tabs are retained and return in Animate. Audio and Slot Color remain available in both modes; Slot Color is read-only
in Animate. This is a local, explicit availability policy. The live public guide says
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

Audio loads a **session-only WAV** for waveform inspection and independent
audition. It uses one immutable, size-checked byte snapshot for both waveform
and native playback, never rereading the path in the native player. Supported
input is little-endian RIFF/WAVE, mono/stereo, 8–192 kHz, integer PCM8/16/24/32
or finite float32, up to 64 MiB. Compressed/extensible/big-endian/RF64 inputs and
malformed chunks/alignment are refused. At most 4,096 min/max bins include every
sample across channels, so opposite stereo phases do not cancel visually.

Load WAV, Play/Pause/Stop, seek, waveform zoom/pan, Loop, Volume and Mute act on
that audition alone. The cursor polls actual player time rather than simulating
elapsed playback from UI frames. The macOS backend uses an owned, unnamed
NSSound with its own gain; it never changes system volume or runs an external
player. Other platforms provide waveform inspection with playback explicitly
unavailable. The view never changes the main pose, transport, selection or history.

Hide/inactive/focus/modal actions pause audition. Synchronous file dialogs pause
before opening; cancelled Open retains the clip paused, successful Open stops and
clears it. Replacement errors keep the previous usable clip. New source epochs
and held-pointer guards prevent stale input crossing clips; owned focus prevents
main clipboard/Delete/transport/nudges from firing. Dropping a player stops only
its own sound. Preferences and clip data stay outside Document and Save/export.

This is a bounded waveform/audition view. Spine audio attachment/event timelines,
keys, synchronization to animation, audio export, formats beyond the documented
WAV subset and cross-platform playback remain pending. No fake device, rate or
recording controls are shown. It does not claim full Spine Audio parity.

Slot Color targets only a typed slot or an attachment's owning slot; a bone,
constraint or skeleton selection never becomes an implicit target. Its inline
saturation/value, Hue and Alpha controls edit Setup color, with exact byte feedback
and #RRGGBBAA entry. Animate displays that setup color read-only. Slot tint is
shared across skins, and reaches viewport, Outline and Preview. Ghost Images
multiply slot RGBA; Solid keeps its chosen RGB with slot alpha.

RGBA is stored as four unmultiplied bytes in native7, preserving hidden RGB at zero
alpha. Current fields are required; legacy1–6 migrate to white without modifying
source files. Save/export preserve bytes, and Spine setup interchange accepts
strict6/8-digit hex. Color timelines, dark/two-color tint and other blend modes
remain unsupported. The pending IK proposal now targets native9, after native8 physics data.

A picker drag freezes its typed context and forms one Session Undo step. Hex
applies on Enter/Tab; invalid/no-op/cancelled entry preserves document/redo and
text owns its Undo. File/history/context/tool/mode/skin/hide/focus/modal actions
cancel uncommitted input before continuing and block held re-entry. Pending bone
poses stay unchanged. Hue-only changes on white/gray retain the chosen UI hue
without rewriting RGBA or creating history. The picker never roundtrips native
bytes through premultiplied Color32 simply to display them.

Graph displays the selected bone's existing Rotation, X/Y, Scale X/Y or Shear X/Y
track. Values are native Setup offsets (degrees/units) or scale multipliers, not
pending pose values. The orange playhead shares the main clip/time,
while Graph's selected key is presentation-only and never redirects Dopesheet
clipboard/Delete operations. Missing tracks are read-only; no phantom keys appear.

Choose overlapping/close native keys independently from the exact-key picker,
then use Zoom Keys. Drag a marker vertically to edit its value. The exact time,
curve, source context and chart mapping freeze at press; release commits one
Session history step. Editing pauses main playback without seeking or discarding
pending poses. Escape, history/file/context/tool/view/focus/modal actions cancel,
and a held cancelled pointer cannot restart until release. Unrelated pose edits
cancel the graph first, then apply to the correct Session baseline. Graph focus
keeps main transport, clipboard, Delete and nudges out.

Curves use engine sampling with bounded render work (at most 257 markers and
about 1,024 sampled points); native keys are never merged or deleted for display.
The virtualized picker retains every exact time. Fit and Zoom Keys change only
presentation. Main pose-evaluation errors do not prevent correcting finite key
values. Tangent handles, key creation/retiming, clipboard editing, general pan/zoom,
multiple curves and absolute posed-value display remain pending. Dopesheet keeps
its current curve/timing tools; exact installed Graph parity is not claimed.

Preview chooses its own animation and skin and evaluates committed keys through
the same checked region/mesh/static-IK engine. It does not display pending unkeyed
poses or change the main animation, skin, cursor, transport, selection or history.
This makes it useful while editing Setup, when the main viewport still shows Setup.
If the main transport is playing, it continues on its own clock.

Preview has independent forward/reverse play, Start/End, Loop, decimal Time,
1–200% Speed, a seek slider, Fit and Bones. Right/middle drag pans; wheel zooms.
Its focused controls own transport keys and leave main key clipboard/Delete/nudges
alone; text keeps its own Undo. Escape pauses Preview and releases its focus.
Closing, hiding, changing tabs, losing app focus or opening a modal pauses it;
mode changes retain its availability. Successful Open resets its clip/skin/time
and view, stops playback and replaces input IDs while retaining Speed/Loop/Bones.
Errors stop only Preview and are shown inside its canvas.

The current Preview is one track with instantaneous clip changes. Mixing,
multiple tracks, additive/alpha/hold-previous controls, automatic play-current and
hide-controls settings remain unimplemented. Loop affects the current clip;
it does not claim Spine's next-entry Repeat semantics. Speed range and preview
shortcut routing are bounded local policy. Preferences stay outside Save/export,
and free docking and exact installed-editor parity remain pending.

Mesh Tools uses **Size** as a world-pixel radius around the one explicitly selected
unweighted mesh vertex. Zero affects that vertex alone. **Feather** defines the
outer fraction of the radius used for linear falloff; zero has a hard edge and
100% fades from the center to the boundary. **Hull vertices** excludes neighboring
hull vertices when off, while the explicitly selected vertex still moves. These
falloff and boundary rules are bounded local policy, not an exact installed-editor
measurement. Size currently ranges from 0 to 1,000 world pixels.

Use **Vertices** in the attachment properties or Mesh Tools to enter the existing
Setup vertex tool. Soft selection affects viewport translations and arrow nudges;
the explicit Local X/Y fields still address only the chosen vertex. Influences,
baseline positions, pointer and view scale freeze at drag start. Release commits
one atomic history step; Escape, focus loss, context/tool/mode changes,
file/history actions and hiding views cancel. A cancelled held pointer cannot
restart before release. Geometry stays in the engine, using checked world-space
distances under scaling/shear and an inverse linear transform for pointer motion.
UVs, topology and existing vertex indices stay unchanged.

Selected vertices are cyan, influenced neighbors shade from dark blue to cyan,
and unaffected vertices are gray. Hovering/adjusting Size or Feather shows outer
and inner rings in the viewport. Settings are session-only and retained through
close/F9/Open; Restore default workspace resets Size to zero. Opening the view or
changing its settings does not change a document, selection, history or pending
pose. Weighted meshes remain inspectable and direct users to Weights. Rotate/Scale
soft transforms, multiple selected vertices, deform keys, weighted soft selection
and path/bounding-box/clipping vertices remain unimplemented.

Timeline shares the active animation, time and playback state rather than owning a
second clock. Orange diamonds summarize exact distinct key times across all
supported bone tracks, including unselected bones; an orange end marker follows
the last key and a cyan playhead shows the current position. The markers are
read-only: Timeline never selects, deletes, copies or retimes Dopesheet keys.

Click or drag to seek whole frames; Shift seeks subframes. A scrub freezes its
mapping at press. With Loop on, dragging past the end wraps only when the gesture
started before the end; starting past it remains available for beyond-end
posing. Decimal **Frame** entry applies on Enter/Tab. Escape or clicking away
cancels uncommitted text; focus/no-op/invalid text does not round an existing
cursor or discard a pose. Starting entry pauses playback, and an actual seek uses
Session's existing pending-pose discard/recovery policy. Undo can recover the pose
at the new frame, with the established document-selection history behavior.

Wheel zoom, right-drag pan, Zoom and Zoom Keys change only the ruler. Zoom Keys
fits committed keys with an authoring margin; an empty clip keeps a positive
one-second view. The minimum zoom span is one frame. Frame entry reveals its new
cursor, including finite extreme times, without adding reserve keys. Context,
file/history actions, hiding views, focus loss and modal errors cancel active
input; a cancelled held pointer cannot restart before release. Text Undo keeps
priority over document Undo. Timeline visibility participates in close/reopen,
mode switches and F9; Open retains visibility and refits the new document.

Timeline's range is independent of the current Dopesheet range. Free docking,
shared pan/zoom across views, persisted workspace/range preferences and full
installed-editor equivalence remain pending. The existing bounded transport still
stops at the last key when Loop is off; its Loop setting is session-wide, not a
new per-animation serialized preference. No document format or export data changed.

Behavior references are the public
[Views](https://en.esotericsoftware.com/spine-views),
[UI](https://en.esotericsoftware.com/spine-ui),
[Animations](https://en.esotericsoftware.com/spine-animations-view),
[Audio](https://en.esotericsoftware.com/spine-audio-view),
[Skins](https://en.esotericsoftware.com/spine-skins-view),
[Slot Color](https://en.esotericsoftware.com/spine-slot-color),
[Metrics](https://en.esotericsoftware.com/spine-metrics),
[Ghosting](https://en.esotericsoftware.com/spine-ghosting),
[Graph](https://en.esotericsoftware.com/spine-graph),
[Mesh Tools](https://en.esotericsoftware.com/spine-mesh-tools),
[Outline](https://en.esotericsoftware.com/spine-outline),
[Playback](https://en.esotericsoftware.com/spine-playback),
[Timeline](https://en.esotericsoftware.com/spine-timeline),
[Keys / Timeline](https://en.esotericsoftware.com/spine-keys#Timeline) and
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

Timeline model, real editor input, pending-pose/history and native acceptance
checks are recorded in [Timeline evidence](evidence/timeline-view.json).

Mesh Tools geometry, editor interaction checks and native acceptance limits are
recorded in [Mesh Tools evidence](evidence/mesh-tools.json).

Independent Preview rendering, interaction checks and native acceptance limits
are recorded in [Preview evidence](evidence/preview-view.json).

Graph value-editing, exact-key/lifecycle checks and native rendering are recorded
in [Graph evidence](evidence/graph-view.json).

Slot Color model/interchange, exact-byte input, renderer and native checks are
recorded in [Slot Color evidence](evidence/slot-color-view.json).

WAV decoding, independent Audio controls and native backend/render checks are
recorded in [Audio evidence](evidence/audio-view.json).
