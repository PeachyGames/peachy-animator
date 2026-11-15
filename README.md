# Peachy Animator

The current engine/MCP interface provides **63 commands / 17 tools**, with
**native v23 unchanged**. The Slider continuation adds semantic creation,
bone-source assignment/clear, explicit property-range snapshots and own-turn
evaluation queries. In Setup, select an animation in the Tree and use
**New → Slider**; creation prepends and returns shared constraint-index effects.

Native creation/Undo/Redo, manual Frame 45 → source-bone assignment with Time-only
key removal, ScaleX → Rotate → ScaleX, and all four range fields pass. Tested
edits include reversed source 8..1 and target Frames 30..90. Local/clear preserves
the manual baseline; changing Mix to 25% captures the observed 1.5-second
fallback. Open retains the remembered ScaleX range, and Save cancels an owned
draft while preserving accepted data. At 1120×740, the four endpoints remain
reachable and an endpoint edit/Undo passes. Final Fit/reopen has one image and
empty history; the public project stays unchanged. A separate native held-range
check changes Source Min **10→13**, then Escape restores the exact document and
redo while the pointer remains held. Continued movement stays blocked until
release; a fresh drag commits with exact one-step Undo/Redo. See the paired
[changed preview](docs/evidence/slider-range-drag.png) and
[Escape restoration](docs/evidence/slider-range-escape.png).

The actual **141-request / 15-pose-query** MCP workflow produces the original
[Slider Source Lab](examples/peach-bot/slider-source-bot.peachy).
External runtime and installed-editor returned geometry each pass **104 poses**
at the unchanged **0.001** tolerance. Strict installed authored equality is
**false**: hidden manual Time is lost, and curve/range representations and the
project name change. This is not lossless interchange or full Slider parity.
Focused checks pass: **63 engine, 33 desktop, 16 schema and 6 stdio** cases, plus
formatting, all-target Clippy, workspace build and the seven-crate architecture
check. See the [source controls](docs/evidence/slider-source-editor.png) and
[minimum layout](docs/evidence/slider-source-minimum.png).

The earlier manual Slider checkpoint used **58 commands / 14 tools**: Frame/Mix,
pending poses, Auto Key/K, typed timelines and Graph shared the engine and MCP.
Its actual 84-request MCP run creates the original
[Slider Lab](examples/peach-bot/slider-bot.peachy) from one PNG and a blank project.
Native editing/history/cancellation, minimum layout and fresh public reopening
pass; 104 exported poses match the external runtime at 0.001. Installed-editor
geometry also passes, with lossy authored curve conversion. That retained lab and
its evidence describe the 58/14 checkpoint. Physics/events through Slider,
extreme precision and broader authoring acceptance remain open; see the
[Slider contract](docs/slices/slider-constraints.md) and
[review gallery](docs/evidence/slider-constraints.html).

The earlier feature checkpoints below retain their recorded version/counts.

World X→Rotate transform mapping now has shared Min/Max commands and exact
readback through MCP: **53 commands / 13 tools**, with native v22 unchanged.
The original six-bone rig is recreated through 122 actual stdio requests, passes
52 runtime poses and returns unchanged geometry and authored data through
installed Spine 4.3.26. Actual native input, cancellation/history, minimum layout
and fresh reopen/playback pass. Native and MCP save byte-identical data; see
[the review gallery](docs/evidence/transform-mapping.html),
[measured scope](docs/slices/transform-mapping-authoring.md) and
[original Mapping Lab](examples/peach-bot/transform-mapping-bot.peachy).
Local/additive modes, other cross mappings and transform timelines remain open.

Multi-vertex Direct weights now supports additive/marquee selection, mixed-value
display, exact assignment and cumulative viewport gestures in a dedicated Weights
pane. MCP shares the engine command and raw uniform/mixed query: **53 commands /
13 tools**, with native v22 unchanged. Native and MCP authoring save the same
original project; fresh native reopen/playback, external runtime and installed
4.3.26 interchange pass. See the [review gallery](docs/evidence/weight-multivertex.html),
[measured contract](docs/slices/weight-multivertex.md) and
[Multi-vertex Weights Lab](examples/peach-bot/multi-weights-bot.peachy).
Auto/Smooth/brush tools, locks, Update and multiple selected bones remain open.

Global Animate draw order now supports Tree movement/dragging, Reset, pending
poses, Auto Key/K and typed Dopesheet clipboard/retiming. MCP uses the same engine
for raw/semantic order keys and typed timeline operations. There are 53 strict
commands / 13 tools; native v22 is unchanged.
See the [measured scope](docs/slices/draw-order-authoring.md) and open
[the original Draw Order Authoring Lab](examples/peach-bot/draw-order-authoring-bot.peachy).
Native input, fresh-process reopen, both authored runtime checks and installed
4.3.26 interchange pass. See the [review gallery](docs/evidence/draw-order-authoring.html).

Curves now has a separate relative-transition view, fitted handles, Match/Separate
controls and shared Dopesheet selection. Scalar Graph adds tangent and numeric
editing. Exact scalar/IK curve setters and shared preset/handle actions are also
available through MCP: 53 strict commands / 13 tools, with native v22 unchanged.
Native input, persistence and both authored runtime comparisons pass. Preset
library/default preferences and uncalibrated coupling cases remain explicit gaps.
See the [review gallery](docs/evidence/curves-authoring.html) and
[measured contract](docs/slices/curves-authoring.md).
Open [the original three-clip Curves Lab](examples/peach-bot/curves-bot.peachy)
to inspect the saved scalar, RGBA and weighted Deform transitions.

Slot attachment swaps and RGBA fades now support Animate editing, pending poses,
Auto Key and K through the same engine and MCP. Native Properties opens a Color
dialog; typed Attach/RGBA rows and a four-channel Graph provide key, clipboard
and curve editing. New color keys subdivide existing curves using independently
calibrated Spine behavior. See the [review gallery](docs/evidence/slot-authoring.html)
and [slot contract](docs/slices/slot-authoring.md). MCP exposes 53 strict commands
and raw slot poses, including hidden slots; native format remains v22.

Export now offers explicit Spine Editor 4.3 and Runtime 4.3 targets in the native
app, CLI and MCP. Editor export rejects unsafe default attachment names before
writing and contradictory per-vertex bone ordering; runtime export preserves raw
influence arrays, independent names and placeholder references.
See [the export contract](docs/slices/export-targets.md).

Deform keys now have named Dopesheet rows, exact retiming and clipboard editing,
and a frame-based Graph with 0–100% transitions and editable tangents. Graph and
Dopesheet share bottom tabs beneath the Tree. MCP exposes the same typed key
operations, a read-only `copy_animation_keys` tool, and `setDeformCurve`.
See [the Deform timeline contract](docs/slices/deform-timeline.md) and
[the current Spine update audit](docs/slices/spine-blog-audit-20261114.md).

Animate mesh vertices now supports weighted/unweighted single-vertex dragging and
nudging, Auto Key, pending poses, K, explicit Deform key/delete and undo. See the
[authoring contract](docs/slices/deform-authoring.md) and
[native screenshot](docs/deform-authoring-editor.png). The original
`examples/peach-bot/deform-authoring-bot.peachy` lab and its adjacent command JSON
exercise weighted and unweighted deform-key creation through the CLI.
Large loose images load at original resolution within the actual GPU limit;
oversized images report an error. Freeform meshes and brush weights remain pending.

This branch adds animated IK (keyed Mix/Positive) on native9 projects. See the
[review checkpoint](docs/IK_ANIMATION_REVIEW.md) for controls, verification and
the remaining raw-curve import precision issue. Interactive review is still pending.

A native 2D skeletal animation editor from Team Peachy, with an independent Rust
core and Spine-compatible exports. The current slices include Spine-style naming,
Dopesheet selection/retiming/clipboard actions, key-derived animation duration, and
weighted mesh import, bone-driven preview, binding, multi-vertex Direct weights,
bone shear authoring, and bounded one-/two-bone inverse kinematics (IK).
This is still a bounded subset of Spine.

![Original IK Lab in the native editor](docs/ik-editor.png)

## Try it

Install Rust 1.95 or newer, including `rustfmt` and `clippy`, then run:

```sh
cargo run -p peachy-animator
```

The editor opens our original **Peach Bot** character: 15 bones, 14 slots, two
animations and peach/mint skins. Its artwork is generated from original geometric
shapes; no game assets or third-party example characters are included.
The demo uses the named `peach` or `mint` appearance in a consuming runtime;
`default` contains the shared parts that those skins inherit.

For the mesh demo, open **Mesh Lab.command**, or run:

```sh
cargo run -p peachy-animator -- examples/peach-bot/mesh-bot.peachy
```

For the weighted mesh demo:

```sh
cargo run -p peachy-animator -- examples/peach-bot/weighted-bot.peachy
```

The original Shear Lab adds reflected/nonuniform parents, nonzero setup shear and
linear, stepped and Bezier shear clips across region and mesh artwork:

```sh
cargo run -p peachy-animator -- examples/peach-bot/shear-bot.peachy
```

Select a bone and press **Z** for Shear. Both numeric axes and explicit handles are
available; exact installed-Spine empty-space/Shift behavior and World/Parent display
remain comparison gates. See [the current UI scope](docs/slices/shear-ui.md).

The original IK Lab adds two targets, one-/two-bone constraints and target animation:

```sh
cargo run -p peachy-animator -- examples/peach-bot/ik-bot.peachy
```

Select a constraint in the Tree's **Constraints** folder. Setup properties expose
Mix, two-bone Positive, linked bones, target replacement and Rename. Move a target
to pose its chain; authored bone values remain separate from solved values.
Constraint settings are static in this slice. See [the IK contract](docs/slices/ik-design.md)
for its chain, transform and precision limits.

The original Physics Lab adds three physics constraints with keyed settings,
global wind keys and reset keys:

```sh
cargo run -p peachy-animator -- examples/peach-bot/physics-bot.peachy
```

Select a physics constraint in the Tree's **Constraints** folder to edit its
Setup settings and Global flags. Physics is saved and exported for the game
runtime, but the editor preview does not simulate it yet. See
[physics data](docs/slices/physics-data.md).

The original Inherit Lab sets every bone inheritance mode, scales, shears and
mirrors the body, and keys mode changes:

```sh
cargo run -p peachy-animator -- examples/peach-bot/inherit-bot.peachy
```

Select a bone to see its **Inherit** Rotation/Scale/Reflection checkboxes.

The original Transform Lab mirrors one hand's movement, aims the head along an
arm and spins a foot with a large rotation offset:

```sh
cargo run -p peachy-animator -- examples/peach-bot/transform-bot.peachy
```

The original Skin Lab has a badge bone, slot and IK aim that exist only in the
mint skin; switch skins in the Tree to see them appear:

```sh
cargo run -p peachy-animator -- examples/peach-bot/skin-bot.peachy
```

The original Deform Lab keys the shapes of the weighted hand and the peach
body mesh (Animate, `wave`):

```sh
cargo run -p peachy-animator -- examples/peach-bot/deform-bot.peachy
```

The original Slot Lab swaps the left hand's image, hides it, and keys head and
body colors (Animate, `wave`):

```sh
cargo run -p peachy-animator -- examples/peach-bot/slot-bot.peachy
```

The original Sequence Lab flips the head through a four-frame sequence and
draws hands and body with additive, screen and multiply blending (Animate,
`wave`):

```sh
cargo run -p peachy-animator -- examples/peach-bot/sequence-bot.peachy
```

The original IK Scale Lab runs IK on mirrored and scaled arms, a stretchy
keyed-scale leg and a stretchy head (Animate, `wave`):

```sh
cargo run -p peachy-animator -- examples/peach-bot/ik-scale-bot.peachy
```

The original Path Lab moves the left hand along a rail past both ends and walks
the right leg chain along a weighted path (Animate, `wave`). The viewport draws
both evaluated curves; select a curve to inspect its knots and handles:

```sh
cargo run -p peachy-animator -- examples/peach-bot/path-bot.peachy
```

The original Clip Lab cuts the body, left arm and head to a concave polygon and
the right arm to a convex hull (Animate, `wave`). Colored polygon outlines show
the evaluated clipping attachments. These guides can be selected but not edited:

```sh
cargo run -p peachy-animator -- examples/peach-bot/clip-bot.peachy
```

The original Transform Map Lab remaps a mirrored hand with offsets and scales
and clamps the left foot to a box (Animate, `wave`):

```sh
cargo run -p peachy-animator -- examples/peach-bot/transform-map-bot.peachy
```

The original Linked Mesh Lab swaps the right hand to linked meshes that follow
or ignore its deform keys, and links the mint body to the peach body across
skins (Animate, `wave`; switch skins):

```sh
cargo run -p peachy-animator -- examples/peach-bot/linked-mesh-bot.peachy
```

The original Tint Lab tints attachments on top of keyed slot colors and keys a
curve that overshoots between equal values (Animate, `wave`):

```sh
cargo run -p peachy-animator -- examples/peach-bot/tint-bot.peachy
```

The original Draw Order Lab moves the left arm in front, sends the head to the
back and then restores setup order (Animate, `wave`):

```sh
cargo run -p peachy-animator -- examples/peach-bot/draw-order-bot.peachy
```

For either mesh fixture, select its hand.R attachment and **Vertices** to inspect the selected
vertex's bone influences. Weighted inspection works in Setup and Animate, including
an unkeyed pose. It stops playback and preserves the current pose and mode. Bone
transforms and animation move the weighted artwork. Press **G** for the Weights
view to bind bones or adjust one vertex's weights. Weighted vertex-position and
topology editing remain pending.

File/history shortcuts use compact icons with named shortcut tooltips. The Views
menu and view headings share the same visual cues; see the
[iconography contract](docs/ICONOGRAPHY.md).

Select a bone, slot or animation and use **F2**, double-click its Tree name, or
click the properties **Rename** icon. Animation properties also offer **Duplicate**;
the copy is selected for inspection while the active animation stays unchanged.
Click Dopesheet keys to select them, **Cmd/Ctrl-click** to toggle several, then
press **Delete** while the canvas has focus or click its Delete icon. Double-click
a key to delete it. Drag selected keys to retime them together; **Shift** disables
frame snapping and **Cmd/Ctrl+Shift-drag** duplicates them. **Cmd/Ctrl+C/X/V**
copies/cuts/pastes focused Dopesheet keys. Paste targets the selected bone at the
playhead, retaining relative timing and curves. The matching toolbar icons share
these actions. **Cmd/Ctrl+A** expands selection to its rows, then all shown keys;
with no selection it selects all shown keys immediately. Click a property row label
to select its Rotate/Translate/Scale tool. Escape cancels a live drag; each
committed edit is one undo step.

The workspace follows Spine’s default arrangement: viewport left, Tree and item
properties right, transform controls below the viewport, and Dopesheet below those
in Animate mode. UI parity is an ongoing acceptance requirement; see the
[UI parity contract](docs/UI_PARITY.md) for remaining differences.

**Views** also opens Animations, Ghosting and Playback (in Animate), Skins, Metrics and Outline. These
share a resizable tabbed panel in the right rail. Outline displays the current
pose without editing handles; Animations and Skins switch the active preview
without changing the saved document. Playback adds a shared preview-speed slider,
percentage presets and transport that works with Dopesheet closed. Its Timeline
FPS display is read-only; FPS retiming and interpolation overrides are pending.
Ghosting adds before/after onion skins, a keyed **Current** comparison, tint and
solid-color display, frame/key sampling, offsets and selection controls. Enable it
in its panel or with **Ghosts** above the viewport.

![Ghosting in the native editor](docs/ghosting-editor.png)

All **15 Views** now have functional bounded implementations; [VIEWS.md](docs/VIEWS.md)
records each scope and remaining parity gaps.

**Views → Audio** loads a session-only WAV waveform with independent audition,
seek, zoom/pan, Loop, Volume and Mute. Playback uses native macOS output; other
platforms support waveform inspection. Audio events, synchronization and export
remain pending; audition data does not become part of the project.

**Views → Graph** shares bottom tabs with Dopesheet. For a selected bone it
shows keyed transform values with vertical key editing. A selected Deform
attachment shows all its transitions on a frame axis with 0–100% progress,
exact-key selection, curve presets, numeric handles and tangent dragging.
Multiple-owner/mixed Graph curves and general docking remain pending.

**Views → Preview** plays a keyed animation and skin on an independent clock while
you edit Setup or another pose. It has its own transport, speed, pan/zoom and Fit;
pending unkeyed edits stay in the main editor. Mixing and multiple tracks remain pending.

**Views → Mesh Tools** provides Size, Feather and Hull vertices for soft selection
while dragging or nudging unweighted Setup vertices. Neighbors use a linear falloff;
a drag is one Undo step. Weighted soft selection and Rotate/Scale remain pending.

**Views → Timeline** opens a compact ruler below the viewport, with playback,
frame/key navigation, decimal frame entry, key markers, scrubbing, zoom and pan.
It shares the current animation and works with Dopesheet closed.

The menu lists the remaining Spine views
with an explanation of each missing feature. See [the complete inventory](docs/VIEWS.md).

For an unweighted mesh in Setup, expand the hand.R bone and slot, select its
mesh attachment, then choose **Vertices** in the attachment properties and drag a numbered vertex. The **Mesh** checkbox converts a region
to a four-corner mesh; existing mesh triangles can be split at their centers.
Setup vertex movement deforms the image while UVs stay fixed; each canvas
drag is one undo step, and Escape cancels the current drag.
This corresponds to Spine's setup vertex transforms. UV editing, freeform hull
creation and animated deform keys are not implemented yet.
Weighted vertex dragging, nudging, local-coordinate fields and subdivision are
blocked; selecting them never converts or normalizes their data.

![Editor iconography and Views menu](docs/icons-views-menu.png)

1. Switch to **Animate** at the viewport's upper left; press **D** to play forward/stop, **A** backward/stop.
2. Select a bone or region attachment in the viewport or Tree; **C** rotates, **V**
   translates, and **X** scales. Attachment transforms are available in Setup.
3. Use the transform fields below the viewport. **L** keys the active transform;
   **Cmd/Ctrl+L** keys all supported transforms for the selected bone. With **Auto Key**
   off, pose first and press **K** to key every edited bone.
4. Scrub the Dopesheet; **W/S** jumps to next/previous key, **Q/E** to start/last key.
   Use **R/F** for next/previous frame and **Shift+R/F** for ten.
5. Undo/redo with **Cmd/Ctrl+Z** / **Cmd/Ctrl+Shift+Z**. Space deselects.
6. Save a `.peachy` project and use **Peachy → Export** or **Cmd/Ctrl+E**.

**Animate** edits create keys while **Auto Key** is on. With it off, orange key
buttons mark unkeyed edits; click a transform's key button, a bone's orange Tree
button, or press **K** (Key Edited). Green means no key; red means keyed at this frame.
Translate and scale use paired X/Y keys by default. Moving the timeline or starting
playback clears unkeyed edits. With no redo branch, Undo recovers the pose at the
new frame. Switching animation or mode clears the pending pose.

Save/export include setup data and animation keys; **unkeyed poses are excluded**.
The status bar calls out a pending pose even if the document itself is saved.
**Setup** edits change the underlying rig.
Animation length follows the highest key and shrinks when it moves or is deleted.
You can seek and pose beyond the last key; keying there extends the animation.
Empty clips and frame-zero-only clips both end at zero while retaining different
key data. The timeline keeps a positive display range independently of that end.
Local/Parent/World select transform coordinates. Numeric entry accepts relative
`+value`, `+-value`, `*factor` and `/divisor` on Enter/Tab. World scale entry uses
local values. Arrows nudge the active transform; Shift uses smaller steps; the **Fit** button fits the scene. Scroll zooms,
right-drag pans, and right-click switches the last two tools. **F9** hides/restores views.
**F12** captures the editor and a diagnostic project snapshot to a local temporary
capture folder. The app currently runs from its source checkout; an installer and
bundled-resource packaging are later work.

## Bind a mesh

Select an active unweighted mesh attachment, press **G** to open the Weights tool
and view, then click **Bind** and click one bone in the Tree or viewport. You can
also select one bone first, click Bind, then click an active unweighted mesh.
The first binding gives every vertex one influence at 100%. Entering Bind mode
alone makes no document edit; Space/Escape exits without rolling back completed work.

For a weighted mesh, **Remove all** removes every bound bone while preserving its
current shape. These operations capture the current Setup or Animate pose, pause
playback, and preserve the frame and unkeyed bone edits. Save stores the resulting
mesh data; key the bone pose separately if it must survive save/reopen or playback.
The Weights mesh/vertex selection survives switching to bone transform tools and
back with G. Normal bone edits still target the normal Tree selection.

For an already weighted mesh, **Bind** adds a genuinely new bone at 0% on every
vertex, preserving the old entries and current shape. A bone already present
anywhere in the mesh, including at 0%, is rejected; this does not repair sparse
imported bindings.

Select one vertex and one bound bone in Weights. **Mode: Direct** exposes a
**Weight** slider and percentage field. Starting with A=100%, Bind B=0%, then set
B to 30: only that vertex becomes A=70%, B=30%. Other positive weights share the
remainder proportionally; bind positions and influence order stay unchanged.
An explicit edit can normalize an imported nonunit total on that vertex and may
deform it. Import, inspection, focusing a rounded value, and adding a zero binding
do not normalize the data.

Release the slider or use Enter/Tab to commit one undo step. Escape or Undo/Redo
cancels an active preview; Save/Export and context changes cancel before continuing.
Direct captures one pose for the gesture and preserves pending bone edits without
keying them. Save excludes that unkeyed pose as before. Ambiguous repeated bones,
missing selected-vertex bindings and reducing the sole positive weight without a
recipient are guarded limits of this slice.

Brush painting, Auto, locks, Update, multiple vertex/bone selection and individual
bone removal remain pending.
The resizable Weights pane uses the existing right column; exact Spine dock
placement is provisional. See the [binding contract](docs/slices/weight-authoring.md),
[Direct command contract](docs/slices/weight-direct-design.md),
[controls and gesture contract](docs/slices/weight-direct.md), and
[validation](docs/VALIDATION.md#additional-binding-and-direct--november-6-2026-utc).

![Initial one-bone binding in the Weights view](docs/binding-editor.png)

![Direct weight entry on the original mesh](docs/direct-editor.png)

## Tree workflow

Select an item's **name** to inspect it. Use a skin or animation's **visibility dot**
to make it active. Activating a skin or animation does not dirty the project. Slot
visibility dots temporarily hide artwork; Cmd/Ctrl+H restores hidden items.
Attachment dots in Setup change the slot's saved setup attachment and support undo.

Page Up/Page Down browse selection history. Cmd/Ctrl+0–9 stores the selected item;
0–9 recalls it. Storing an empty selection clears a group. These session groups
currently hold one item each. Document Undo remains separate from selection history.

Search reveals matching branches. The Bones, Slots and Attachments filters provide
quick views; Slots lists the rig in draw order. When attachments are filtered out,
clicking artwork in the viewport selects its slot. Only the selected entity's
supported controls can edit it. Inactive skin attachments remain inspectable.

## What works

- Hierarchical bones with position, rotation and nonuniform scale.
- Layered region attachments, draw order, skins and default-skin fallback.
- Global Animate draw-order movement, drag placement, pending poses and Auto Key/K,
  with exact global keys and typed Dopesheet copy/paste/retime/delete.
- A typed Tree for bones, slots, skin placeholders, attachments, skins and animations,
  with search, type filters, branch controls, temporary visibility and contextual properties.
- Shared commands for bone length, region setup transforms and a slot’s setup attachment.
- Unweighted meshes with explicit UVs, triangles and hulls, setup vertex editing,
  region conversion and triangle subdivision.
- Weighted meshes with typed bone-local influences, shared posed geometry for
  rendering/picking/bounds, read-only inspection and preserved import/export data.
- A G Weights tool/view, initial one-bone Bind and Remove all at the current pose,
  preserving shape, pending bone pose and time through shared engine commands.
- Additional zero-weight bindings and Direct percentage editing of one vertex and
  one bound bone, with proportional redistribution and transactional previews.
- Deterministic timeline playback, scrubbing, frame stepping and looping.
- A separate compact Timeline with shared transport, fractional frame entry,
  all-track key markers, snapped/subframe seeking, pan and zoom.
- Ghosting with frame/key onion skins, pending-pose comparison, tinted or solid
  rendering, anchor/loop/offset controls and a lockable bone selection filter.
- Scalar keys with linear, stepped and editable Bezier interpolation.
- Auto Key, unkeyed multi-bone posing, Key Edited (K), colored key indicators,
  explicit transform/Tree key buttons and pose-aware transactional history.
- Canvas manipulation, transform controls and adding child bones.
- Shared commands, atomic batch edits and undo/redo.
- Static one-/two-bone normal IK with animated targets, constraint Tree properties,
  and pose-aware binding/transform tools within the supported chain subset.
- JSON project saves with atomic replacement, verified timestamped backups and
  image copying when saving to another directory.
- Spine **4.3.x JSON** import/export for the supported subset. Desktop export
  creates a fresh directory with JSON, an atlas and its PNG page.
- A headless CLI that uses the same document, command and pose engine.

Brush weight editing, multi-vertex/multi-bone weights, Update bindings, linked meshes,
IK timelines and advanced/dependent chains, physics, path/transform constraints,
clipping, slot animation,
events, image sequences, packed-atlas import, native `.spine` files and animation
mixing are not implemented. Unsupported imports are blocked with a feature report;
the source remains untouched. The latest read-only artist audit still finds 0/11
fully supported characters: weighted-mesh refusals are gone, but all 11 require
constraints. Installed-editor re-export can quantize bind coordinates and positive
weights, remove zero entries and reorder influences, so lossless editor round trips
are not claimed.
See [the compatibility contract](docs/COMPATIBILITY.md).

## CLI

```sh
cargo run -p peachy-cli -- validate examples/peach-bot/peach-bot.peachy
cargo run -p peachy-cli -- pose examples/peach-bot/peach-bot.peachy wave 0.3
cargo run -p peachy-cli -- inspect /path/to/skeleton.json
cargo run -p peachy-cli -- import /path/to/skeleton.json new-project.peachy
cargo run -p peachy-cli -- export examples/peach-bot/peach-bot.peachy new-export.json
cargo run -p peachy-cli -- bundle examples/peach-bot/peach-bot.peachy /path/to/export-parent
cargo run -p peachy-cli -- apply input.peachy commands.json edited.peachy
```

CLI outputs must be new files; it refuses to overwrite existing output. The CLI
`export` command writes JSON only. Use the desktop exporter for a packed bundle.
The CLI `bundle` command uses that same shared exporter.
Native projects reference loose images in `images/` beside the project. Native
format v7 requires exact four-byte setup RGBA on every slot, an ordered constraints list, bone shear X/Y and tagged `slotLocal`/`weighted` mesh
vertices. Animation duration follows its last key, including empty clips. The loader
migrates v1–v6 projects in memory, preserving legacy holds/vertices, adding
zero shear, empty constraints and white slot colors where needed. Loading leaves source bytes unchanged. Explicit Save
writes v7 and backs up the previous file; older binaries cannot open v7. Bone
command payloads require all seven transform fields. `pose` accepts an optional
skin name and emits authored locals, applied locals, world transforms and attachment geometry; unsupported or nonfinite evaluation fails
explicitly before JSON output.

## Architecture and verification

Agents can work without opening a window through the standalone MCP server:

```sh
cargo build -p peachy-mcp --locked
./target/debug/peachy-mcp --root /absolute/path/to/animation-workspace
```

It communicates over stdio and exposes **63 commands / 17 tools** for project
inspection, validated command batches, typed key copying, undo/redo, pose
evaluation and new-output saves/exports through the same engine as the desktop.
Each server owns an independent session; it does not
control an existing editor window. See [MCP setup, tools and limits](docs/MCP.md)
for client configuration and a complete workflow.

Slider creation and source editing use distinct shared operations:

- `createSliderConstraint` takes an animation index, chooses the native automatic
  name/defaults and **prepends**. Raw `addSliderConstraint` still appends a complete
  payload. Successful edits and history responses return
  `effects.constraintIndices.oldToNew` and `newLen`; an undone creation maps its
  removed index to `null`. Remap retained constraint/key references and read
  `get_document` again. Within a command batch, indices address each successive
  candidate.
- `setSliderSource` requires `bone`, even when clearing it with `null`. A genuine
  assignment removes that Slider's Time tracks and pending Time while preserving
  Mix. `setSliderSourceProperty` requires the six-choice property and an explicit
  `source_range` snapshot. Complete raw `setSliderConstraint` remains separate
  from these transitions.
- `setSliderSetupPose` edits Setup Time/Mix semantically. A genuine bound Mix
  change captures the prior own-turn sampled Time when available; an exact no-op
  or unavailable sample preserves the hidden baseline.

Example flat command payloads, sent inside revision-checked `apply_commands`:

```json
{"command":"setSliderSource","constraint":0,"bone":null}
```

```json
{"command":"setSliderSourceProperty","constraint":0,"property":"scaleX","source_range":{"min":1,"max":2}}
```

Use `get_slider_source_choices` to obtain the contextual range and complete
command snapshot rather than assuming those example endpoints. The source
properties are `rotate`, `x`, `y`, `scaleX`, `scaleY` and `shearY`.

| Read-only Slider tool | Contract |
| --- | --- |
| `get_slider_mapping` | Raw-derived source and target endpoints. Target values are referenced seconds, not Frames; undefined SourceMax is `null`. No solving, repair or authored-Time change. |
| `get_slider_evaluation` | One Slider's own ordered turn: incoming/applied settings, `sampleTime` and eligibility. A missing sample is `null`, including zero incoming Mix. Later constraints can change final effective settings without changing this snapshot. |
| `get_slider_source_choices` | Six contextual source-range choices and explicit property commands, without focusing, remembering a range, solving or changing document/history/revision. |

These queries require the current revision and exact constraint index. Evaluation
also accepts a host animation name, nonnegative host seconds and skin. The
existing `get_slider_pose` remains an **authored** Time/Mix query;
`evaluate_pose` exposes authored/final applied arrays and own-turn evaluations
from the same checked solve. Use `editSliderMappingRange` for endpoint edits;
its target values also use referenced seconds.

Source-range preferences belong to the server/editor process, outside the native
document and undo history. They survive successful Open and new headless projects
within that process; canceled/failed Open leaves them unchanged. A fresh process
starts with defaults. Reading choices or saving a project does not store those
preferences in `.peachy` data.

The design follows PhotoCraft's engine-first, command-driven architecture:
serializable documents, an independent animation engine, format adapters, and thin
desktop/CLI clients. No PhotoCraft application code was copied. See
[architecture](docs/ARCHITECTURE.md), [implementation provenance](docs/PROVENANCE.md),
and [the next slices](docs/ROADMAP.md).
The [workflow milestones](docs/WORKFLOWS.md) track import, edit, pose, animate, slots,
save and game-export hurdles against the current limitations.
The [Spine parity matrix](docs/PARITY.md) records implemented behavior, evidence,
and the remaining gaps by feature area.

```sh
cargo fmt --all -- --check
python3 scripts/check-architecture.py
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --locked
```

The included GitHub workflow is manual-only. Hosted CI has not been run; the
initial verification was local on macOS.
The [code quality review](docs/CODE_QUALITY.md) records the subsequent boundary
refactor, corrected defects and checks for expanding the editor.

The optional external compatibility check needs Node.js and a **separately licensed**
Spine 4.3 runtime supplied by its absolute module path. It executes that runtime as
an oracle without copying its implementation into this project:

```sh
mkdir -p .local
cargo run -p peachy-cli -- export examples/peach-bot/peach-bot.peachy .local/peach-bot.json
node scripts/check-runtime.mjs /absolute/path/to/spine-core/dist/index.js
```

The official Spine runtimes are not dependencies of this application. Games using
those runtimes retain their applicable Spine licensing requirements. Our MIT license
covers our own source and original fixture; it does not relicense Esoteric Software's
tools or runtimes. Spine is a trademark of Esoteric Software. Team Peachy is independent
of Esoteric Software and the ArtCraft team.
