# Official Spine update audit — November 14, 2026

Rechecked November 14 after the slot authoring commit, against Peachy Animator
`f345fdd` plus the verified export-target continuation below. This is a current
backlog and reference record, not a claim of full Spine parity.
Installed-editor observations and native screenshots are recorded
in [UI parity](../UI_PARITY.md) and the linked slice acceptance records.

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
| Color | Slot Color: Setup/Animate float slot pose, inline chooser, Properties modal, keying and RGBA Graph | Broader Setup targets, separate RGB/alpha and dark tint remain. A label-only rename is insufficient. See [accepted slot scope](slot-authoring.md). |
| Curves | No separate view | Observe its normalized transition editing, presets, selection and relationship with Graph; expose persistent semantic edits through the engine/MCP where applicable. |
| Problems | No view | Observe issue discovery, navigation and supported fixes; any document-changing fixes need shared commands, undo and MCP access. |
| Metrics | Present in both | Verify content and behavior separately; presence alone does not prove parity. |

These are additional full-feature gaps, separate from the accepted Deform and
slot slices. This menu capture proves inventory and labels, not
complete behavior, exact layout metrics, or interaction parity for each view.

## Completed slices and remaining curve semantics

The [4.3 release announcement](https://esotericsoftware.com/blog/Spine-4.3-released)
highlights Dopesheet-to-Graph synchronization and curve editing improvements.
The established [Dopesheet guide](https://esotericsoftware.com/spine-dopesheet)
describes bone-grouped property rows, selection, timing edits, clipboard actions
and interpolation indicators. Those expectations apply to deformation keys too;
deform rows are not being presented as a newly introduced 4.3 feature.

The former immediate gaps are now closed within their documented bounds:

- [Deform timeline/Graph](deform-timeline.md): typed attachment ownership,
  exact retime/copy/delete, layout-aware clipboards, normalized transition
  curves, tangent/numeric authoring and actual MCP/native/runtime acceptance.
- [Slot authoring](slot-authoring.md): attachment swaps/hiding, float pending
  RGBA poses, Auto Key/K, exact typed mixed clipboard, calibrated color-key
  insertion and four value curves, with native/MCP/runtime acceptance.
- [Explicit export targets](export-targets.md): Editor rejects confirmed unsafe
  default-name overrides before writing; Runtime preserves lookup/name metadata.
  Actual CLI/MCP/native exports, four accepted Editor import controls and the
  region/mesh/linked runtime checks pass. Hosted CI remains a separate gate.

MCP now exposes 45 strict commands / 11 tools, including value-preserving
`setDeformCurve`, `setSlotColorCurves`, float `insertSlotColorKey` and read-only
typed copying. This does not complete all Graph/Curves semantics. Source
inspection at `f345fdd` shows scalar Graph edits key values but has no tangent
controls; its selection remains separate from the typed Dopesheet selection.
There is no Curves view or scalar curve-only command: MCP can replace a full
`setKey` payload, which is different from a semantic edit that preserves value
and time.

Two concrete reference cases need installed-editor probes next:

1. The [current Graph guide](https://esotericsoftware.com/spine-graph) describes
   resetting default handles by clicking Bezier again on an existing Bezier
   selection. Current slot/Deform preset handlers ignore an already-selected
   curve type, so this reset gesture is absent.
2. The [4.3 changelog](https://esotericsoftware.com/spine-changelog), 4.3.24
   (September 6, 2026), limits use of the preceding curve to a single selected
   last key. The earlier 4.3.65-beta entry (April 11) introduces that behavior.
   Current slot/Deform controls edit the selected final key's unused outgoing
   curve. Probe exact last-key and multi-selection behavior before changing the
   shared resolver; raw commands should retain explicit exact-key meaning.

These are open interaction semantics, not retrospective failures of the
specific accepted tangent/value workflows. No new native probe was performed
for this audit refresh.

### Cross-attachment clipboard behavior

The current [Keys guide, Clipboard buttons](https://esotericsoftware.com/spine-keys#Clipboard-buttons)
explicitly permits deform keys to be pasted to another attachment selected
before Paste. A same-attachment-only clipboard policy would therefore narrow
the requested parity.

Older official support responses describe copying between meshes with matching
structure and reject incompatible vertex counts:
[Copy keyframes Meshed Texture, January 2017](https://en.esotericsoftware.com/forum/d/7807-copy-keyframes-meshed-texture)
and [Copying keys, September 2017](https://uk.esotericsoftware.com/forum/d/9314-copying-keys).
These establish the intended cross-attachment workflow but not a complete 4.3
layout policy. Subsequent controlled native probes now confirm compatible
unweighted paste, the unweighted/one-influence weighted bridge, matching
weighted segmentation, and rejection of incompatible counts or segmentation.
The engine implements that bounded policy without changing local offsets or
bindings; [the Deform contract](deform-timeline.md) records the evidence.
Arbitrary topology and every cross-mode combination are not claimed.

### Deform curves are transition curves

An [official developer explanation, November 2020](https://esotericsoftware.com/forum/d/14932-deformation-keys-and-the-graph-editor)
describes deformation curves as controlling the transition between whole-mesh
poses rather than displaying an X/Y scalar curve for each vertex. The
[mesh transform guide](https://esotericsoftware.com/spine-meshes#Transform-tools)
also states that deformation keys store vertex positions; rotation and scale
operations are not separate keyed transform channels.

The installed 4.3.26 probes subsequently confirmed a full-time cyan 0–100
transition display that resets each interval, with blue attachment-owned rows.
Those observations support the current Deform Graph. They do not establish a
per-vertex scalar Graph workflow.

## Ranked continuation after slot authoring

The order below is a proposed implementation sequence, based on current source
and documented dependencies. It is not a claim that all named features have
been exhaustively observed.

| Rank | Bounded next slice | Shared UI / engine / MCP path and acceptance |
| --- | --- | --- |
| 0: verified interchange prerequisite | Explicit Editor versus Runtime export targets | The shared adapter now rejects confirmed lossy default-name overrides for Editor output and preserves metadata for Runtime output. Native/CLI/MCP and bounded editor/runtime acceptance pass; see [the contract](export-targets.md). Arbitrary conversion and hosted CI remain separate gates. |
| 1: next authoring feature | Selected-owner Curves and scalar interpolation editing | Probe the two cases above, then add normalized transition authoring for exact scalar keys through a curve-only engine command and strict MCP schema. Reuse guarded Session previews. Wire Curves discovery and Dopesheet selection; preserve values, fractional times, pending poses and redo. Probe flat-endpoint offset curves before defining their normalized display. |
| 2: weight workflow foundation | Multiple selected vertices, one mesh/bone, Direct mode | Replace the retained single vertex with explicit selection and native box/additive/select-all controls. Apply one frozen, atomic Batch through existing per-vertex weight commands; MCP already accepts explicit batches. Preserve untouched influences/positions and pending poses. Auto/brush algorithms, locks and multi-mesh painting require later shared operations. |
| 3: major 4.3 rigging gap | One world-space cross-property transform mapping | Existing data and MCP add/set constraint payloads are present, but evaluation refuses translation-to-rotation/scale mappings used by the announcement. Calibrate one mapping, implement it in the engine, expose real mapping controls and evaluate an MCP-authored rig against the external oracle. Local/additive modes remain separate. |
| 4: larger new model | A manually driven slider constraint | Add explicit animation/frame/mix ownership, model validation, shared evaluation, adapter, creation/key controls and strict MCP commands. Detect self-reference/cycles and unsupported timeline domains. A copied pose or an ordinary slot fade is not slider evaluation. Bone-driven sliders extend this slice later. |

The [4.1 release post, July 4, 2022](https://esotericsoftware.com/blog/Spine-4-1-released)
is the primary description of the separate Curves view: relative transition
editing with reusable user presets. It establishes why a scalar value Graph
alone does not cover that view, but does not establish current 4.3 placement.
The [current Weights guide](https://esotericsoftware.com/spine-weights)
documents vertex multi-selection and Direct/brush workflows. Current source has
one retained vertex and `setMeshBoneWeight`. Explicit MCP batches already edit
multiple vertices; native selection and generated Auto/brush weights remain
different missing capabilities.

## Broader update backlog

These are feature areas explicitly covered by the
[4.3 announcement](https://esotericsoftware.com/blog/Spine-4.3-released), mapped
to the current implementation and its missing gates. The accepted slices do
not complete this broader objective.

| Public update area | Current gap and required native/MCP work |
| --- | --- |
| Dopesheet-centric synchronization and curve refinements | Deform/slot selection and curve commands work; scalar shared selection/tangents, reset/final-key semantics, Curves presets, default curve preferences and generalized handle adjustment remain. |
| Sliders | No native model, evaluation, import/export or creation workflow; add shared commands and strict schemas with independently derived behavior. |
| Transform property mapping and local/world choices | Data is modeled, but evaluation is restricted and mapping editing is pending; do not equate accepted data with working constrained motion. |
| Multiple-mesh tracing and weight workflows | Explicit payload creation exists through MCP; native topology generation, multi-selection and Auto/brush workflows remain. |
| Draw order folder keys | Current keyed draw order is separate; folders need a model, adapter, evaluation and authoring surface. |
| Inverse clipping | Existing clipping is bounded and refuses inverse behavior; add geometry/evaluation plus controls and automation commands. |
| Linked meshes across slots | Current links resolve source skin/name within the same slot. Cross-slot source identity, adapter and evaluation remain; raw MCP `addAttachment` can already create valid same-slot link payloads, while dedicated native create/unlink controls are absent. |
| Volume-preserving IK scaling | Current IK stretch does not establish this scaling mode; physics is still data-only. |
| Problems, sharing/export and skin/PSD workflows | Warnings, project packaging, HTML sharing, skin operations and PSD refresh must be assessed as actual user flows, not inferred from JSON export. |
| Numeric expressions and refined controls | Current numeric operators are a subset; richer expressions, item constants/functions and exact UI names/states need comparison. |

One concrete UI naming check comes from the changelog's 4.3.73-beta entry:
the former Slot Color view became Color and gained broader setup targets.
The installed menu confirms `Color`, while Peachy still displays `Slot Color`.
The target behavior remains a separate probe before changing terminology alone.

## Recent tutorial and search coverage

The official [Skins for equipped items — Spine Tips #11, October 20, 2026](https://esotericsoftware.com/blog/Skins-for-equipped-items-Spine-Tips-11)
adds a relevant workflow target after the 4.3 announcement: item-specific skin
bones/constraints/attachments, item animation layered over locomotion, and
Transform/Physics follow-through. Current skin-scope evaluation and slot swaps
cover part of the ingredients. Preview is single-track, skin composition/
authoring is bounded, and editor physics remains data-only; the complete
equipped-item outcome has not been recreated or verified.

This refresh read the official announcement, current changelog, Graph/Weights
guides and the identified tutorial posts through the agent-reach web/search
route. The blog index reader returned its category shell, so targeted official
search was used to discover individual posts. This is not an exhaustive claim
that no newer post exists. Linked videos/project downloads were not fetched or
transcribed in this refresh; their full workflow coverage remains unassessed.

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
