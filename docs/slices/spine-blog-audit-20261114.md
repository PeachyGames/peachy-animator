# Official Spine update audit — November 14, 2026

This audit compares public feature descriptions with the Peachy Animator
baseline at `cc985a4`. It is a backlog and reference record, not an acceptance
claim for subsequent implementation. Installed-editor observations and native
screenshots are recorded separately in [UI parity](../UI_PARITY.md).

The [official editor changelog](https://esotericsoftware.com/spine-changelog)
dates **4.3.26 to September 7, 2026** and labels **4.3.27 Unreleased** at this
check. Retain the installed 4.3.26 behavioral baseline; listing an unreleased
version does not authorize changing the installed reference.

## Installed Views inventory

The coordinator's November 14 capture of the actual **Spine 4.3.26** Views
menu was inspected directly. The private evidence is
`timeline/spine-views-menu.png` in the existing local tutorial task record;
it is not embedded or committed here.

The menu has **17 entries**: Color, Mesh Tools, Skins, Weights, Animations,
Curves, Dopesheet, Ghosting, Graph, Playback, Timeline, Audio, Metrics, Outline,
Preview, Problems and Tree. Metrics is visibly present between Audio and
Outline; it must not be counted as a Peachy-only view.

Peachy's current 15-entry inventory contains Metrics but lacks **Curves** and
**Problems**, and still labels its bounded color panel **Slot Color**.
The earlier 15-view guide inventory is therefore not the complete inventory
of the installed reference.

| Installed reference | Peachy observation | Remaining parity gate |
| --- | --- | --- |
| Color | Slot Color, limited to slot setup color and an attachment's owning slot | Compare actual target selection and color operations, then implement the broader behavior and matching terminology. A label-only rename is insufficient. |
| Curves | No separate view | Observe its normalized transition editing, presets, selection and relationship with Graph; expose persistent semantic edits through the engine/MCP where applicable. |
| Problems | No view | Observe issue discovery, navigation and supported fixes; any document-changing fixes need shared commands, undo and MCP access. |
| Metrics | Present in both | Verify content and behavior separately; presence alone does not prove parity. |

These are additional full-feature gaps, separate from the current Deform
Dopesheet/Graph slice. This menu capture proves inventory and labels, not
complete behavior, exact layout metrics, or interaction parity for each view.

## Immediate authoring gap

The [4.3 release announcement](https://esotericsoftware.com/blog/Spine-4.3-released)
highlights Dopesheet-to-Graph synchronization and curve editing improvements.
The established [Dopesheet guide](https://esotericsoftware.com/spine-dopesheet)
describes bone-grouped property rows, selection, timing edits, clipboard actions
and interpolation indicators. Those expectations apply to deformation keys too;
deform rows are not being presented as a newly introduced 4.3 feature.

At the baseline, native mesh deformation and MCP `setDeformKey` /
`deleteDeformKey` exist, but general timeline identities and clipboards in
`crates/engine/src/timeline.rs` support Bone and IK only. The matching MCP
`moveAnimationKeys`, `deleteAnimationKeys` and `pasteAnimationKeys` schemas
therefore cannot express Deform entries. Native Deform Dopesheet/curve work is
explicitly pending in [the current authoring contract](deform-authoring.md).

The next parity work should close the complete workflow across native UI,
engine commands and published MCP schemas:

- Discover and select the active attachment's Deform row, including its skin
  and linked-mesh timeline owner, without falling back to an unrelated bone.
- Select, copy/cut/paste, move/duplicate and delete exact keys, preserving dense
  geometry, interpolation and fractional time identities. Rejected operations
  must leave document, revision and history unchanged.
- Edit a selected deformation transition's interpolation without rewriting its
  vertex offsets, and retain the shared pending-pose and gesture lifecycle.
- Verify save/reopen and runtime output after native edits and actual MCP
  requests. A schema declaration or a successful import alone is insufficient.

### Cross-attachment clipboard behavior

The current [Keys guide, Clipboard buttons](https://esotericsoftware.com/spine-keys#Clipboard-buttons)
explicitly permits deform keys to be pasted to another attachment selected
before Paste. A same-attachment-only clipboard policy would therefore narrow
the requested parity.

Older official support responses describe copying between meshes with matching
structure and reject incompatible vertex counts:
[Copy keyframes Meshed Texture, January 2017](https://en.esotericsoftware.com/forum/d/7807-copy-keyframes-meshed-texture)
and [Copying keys, September 2017](https://uk.esotericsoftware.com/forum/d/9314-copying-keys).
These establish the intended cross-attachment workflow, but do not establish
current weighted-influence compatibility rules. Probe the installed editor for
weighted/unweighted mode, influence layout, source/target bindings and linked
timeline sharing. Do not infer an exact 4.3 rejection policy from legacy advice.

### Deform curves are transition curves

An [official developer explanation, November 2020](https://esotericsoftware.com/forum/d/14932-deformation-keys-and-the-graph-editor)
describes deformation curves as controlling the transition between whole-mesh
poses rather than displaying an X/Y scalar curve for each vertex. The
[mesh transform guide](https://esotericsoftware.com/spine-meshes#Transform-tools)
also states that deformation keys store vertex positions; rotation and scale
operations are not separate keyed transform channels.

Treat a normalized transition display as a candidate to confirm against the
installed editor. The public explanation does not establish the exact current
Graph axes, row labels, handle placement or selection rules. Do not invent a
per-vertex scalar Graph workflow and call it Spine parity.

## Broader update backlog

These are feature areas explicitly covered by the
[4.3 announcement](https://esotericsoftware.com/blog/Spine-4.3-released), mapped
to the baseline implementation and its missing gates. Implementing the first
row above does not complete this broader objective.

| Public update area | Baseline gap and required native/MCP work |
| --- | --- |
| Dopesheet-centric synchronization and curve refinements | Native Graph is a bounded scalar editor; shared selection, retiming/handle behavior and default curve preferences need observation and implementation. MCP must expose the same semantic edits. |
| Sliders | No native model, evaluation, import/export or creation workflow; add shared commands and strict schemas with independently derived behavior. |
| Transform property mapping and local/world choices | Data is modeled, but evaluation is restricted and mapping editing is pending; do not equate accepted data with working constrained motion. |
| Multiple-mesh tracing and weight workflows | Explicit payload creation exists through MCP; native topology generation, multi-selection and Auto/brush workflows remain. |
| Draw order folder keys | Current keyed draw order is separate; folders need a model, adapter, evaluation and authoring surface. |
| Inverse clipping | Existing clipping is bounded and refuses inverse behavior; add geometry/evaluation plus controls and automation commands. |
| Linked meshes across slots | Source sharing and timeline ownership need current-editor probes; creation/unlink authoring remains pending. |
| Volume-preserving IK scaling | Current IK stretch does not establish this scaling mode; physics is still data-only. |
| Problems, sharing/export and skin/PSD workflows | Warnings, project packaging, HTML sharing, skin operations and PSD refresh must be assessed as actual user flows, not inferred from JSON export. |
| Numeric expressions and refined controls | Current numeric operators are a subset; richer expressions, item constants/functions and exact UI names/states need comparison. |

One concrete UI naming check comes from the changelog's 4.3.73-beta entry:
the former Slot Color view became Color and gained broader setup targets.
The installed menu confirms `Color`, while Peachy still displays `Slot Color`.
The target behavior remains a separate probe before changing terminology alone.

## Source conflicts and scope

The mesh guide still states that linked meshes share a slot. The 4.3 release
announcement explicitly allows a different slot. For this conflict, use the
installed 4.3 editor and the version-specific announcement rather than imposing
the older restriction from the general guide.

Other 2026 blog posts cover runtime integration, including
[Phaser Mesh2D, July 14](https://en.esotericsoftware.com/blog/phaser-mesh2d-in-spine-phaser-v4)
and [Godot native 3D, September 11](https://us.esotericsoftware.com/blog/spine-godot-native-3d).
Track these separately from native editor interaction parity. They are not proof
that Peachy Animator implements an equivalent integration or needs to embed a
proprietary runtime.

Only public descriptions, original project code and permitted editor/runtime
observations inform this work. No proprietary implementation, bundled example
art or private artist data is included here. Exact visual comparisons remain a
separate requirement from numerical compatibility.
