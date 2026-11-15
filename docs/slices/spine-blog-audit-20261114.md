# Official Spine update audit — November 14, 2026

Public sources were checked November 14 after the slot authoring commit
`f345fdd`. The implementation record includes verified export-target, Curves,
global draw-order and November 15 multi-vertex Direct continuations below.
The November 15 update reconciles accepted local evidence without a new web
fetch. This is a backlog and reference record, not a claim of full Spine parity.
Installed-editor observations and native screenshots are recorded
in [UI parity](../UI_PARITY.md) and the linked slice acceptance records.

The [official editor changelog](https://esotericsoftware.com/spine-changelog)
dates **4.3.26 to September 7, 2026** and labels **4.3.27 Unreleased** at the
November 14 source check. Retain the installed 4.3.26 behavioral baseline; listing an unreleased
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

Peachy's current 16-entry inventory contains Metrics and the bounded **Curves**
view, lacks **Problems**, and still labels its bounded color panel **Slot Color**.
The earlier 15-view guide inventory is therefore not the complete inventory
of the installed reference.

| Installed reference | Peachy observation | Remaining parity gate |
| --- | --- | --- |
| Color | Slot Color: Setup/Animate float slot pose, inline chooser, Properties modal, keying and RGBA Graph | Broader Setup targets, separate RGB/alpha and dark tint remain. A label-only rename is insufficient. See [accepted slot scope](slot-authoring.md). |
| Curves | Separate fitted relative-transition pane, exact shared selection, Match/Separate and engine/MCP preset/handle actions | Native/MCP/runtime acceptance passes within the [measured scope](curves-authoring.md). Stored presets, Last chosen, native property groups and several coupled domains remain. |
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
- [Curves and scalar interpolation](curves-authoring.md): exact scalar/IK
  setters, shared typed Graph selection/tangents, fitted relative curves,
  repeated reset/final-key semantics and calibrated Match/Separate actions.
  Native input/persistence and both authored runtime comparisons pass.
- [Global draw-order authoring](draw-order-authoring.md): Tree/key/Reset,
  pending poses, typed discrete timeline and shared MCP raw/semantic commands.
  Native input/fresh reopen, both authored runtime checks and installed 4.3.26
  interchange pass. Independent folder keys remain a separate model slice.
- [Multi-vertex Direct weights](weight-multivertex.md), accepted November 15:
  explicit one-mesh/one-bone selection, numeric/slider/vertical edits, mixed
  blank field, skip-sole assignment, cumulative clamping with frozen ratios,
  ordered sparse origin and raw MCP readback. Actual native history/cancellation,
  pending Save exclusion, fresh reopen/minimum/playback, root-only MCP creation,
  runtime and exact final Editor roundtrip pass. See the
  [scalar evidence](../evidence/weight-multivertex.json) and
  [capture gallery](../evidence/weight-multivertex.html). Auto/Smooth, brushes,
  locks, Update, multiple bones, sparse Deform remapping and Pies/overlay remain.

Native format remains **22**. MCP now exposes **52 strict commands / 12 tools**,
including semantic `editMeshBoneWeights` and read-only `get_mesh_weights`, raw/semantic global
draw-order authoring and complete hidden-slot order readback, value-preserving
`setDeformCurve`, `setSlotColorCurves`, float `insertSlotColorKey` and read-only
typed copying, plus exact `setKeyCurve`/`setIkCurve` and shared
`applyCurvePreset`/`editCurveHandle`. Source at the earlier `f345fdd` lacked
scalar tangents/typed synchronization and Curves; those bounded gaps now have
the linked native/MCP acceptance. Stored preset/default preferences, grouped
properties and several coupled domains remain open.

Two former reference gaps now have installed-editor probes and acceptance:

1. The [current Graph guide](https://esotericsoftware.com/spine-graph) describes
   resetting default handles by clicking Bezier again on an existing Bezier
   selection. Shared Curves/Graph actions now reset it; native tests verify
   repeated reset and preserved values/times.
2. The [4.3 changelog](https://esotericsoftware.com/spine-changelog), 4.3.24
   (September 6, 2026), limits use of the preceding curve to a single selected
   last key. The earlier 4.3.65-beta entry (April 11) introduces that behavior.
   A shared resolver now redirects a single final key to its preceding interval,
   while multiple final selections disable controls and preserve redo. Raw
   commands retain explicit exact-key meaning.

Those bounded semantics are measured; remaining cases are listed in the Curves
contract rather than inferred from the release post or screenshots alone.

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

## Ranked continuation after accepted authoring slices

The order below is a proposed implementation sequence, based on current source
and documented dependencies. It is not a claim that all named features have
been exhaustively observed.

| Rank | Bounded next slice | Shared UI / engine / MCP path and acceptance |
| --- | --- | --- |
| 0: verified interchange prerequisite | Explicit Editor versus Runtime export targets | The shared adapter now rejects confirmed lossy default-name overrides for Editor output and preserves metadata for Runtime output. Native/CLI/MCP and bounded editor/runtime acceptance pass; see [the contract](export-targets.md). Arbitrary conversion and hosted CI remain separate gates. |
| 1: accepted bounded continuation | Selected-owner Curves and scalar interpolation editing | Native/MCP/runtime acceptance covers shared exact setters/actions, typed selection, fitted relative handles, repeated reset, final-key rules and calibrated normal coupling. Stored presets/defaults, grouped properties and uncalibrated coupling remain; see [contract](curves-authoring.md). |
| 2: accepted bounded continuation | Global Animate draw-order editing | The Look around demo's global ordering now has shared pending/key/Reset/typed timeline and MCP authoring. Actual native input/fresh reopen, both authored runtime checks and installed 4.3.26 interchange pass; see [contract](draw-order-authoring.md) and [review](../evidence/draw-order-authoring.html). Per-folder keys, Setup reordering, compact property sizing and cross-layout mapping remain separate slices. |
| 3: accepted weight foundation | Multiple selected vertices, one mesh/bone, Direct mode | Native selection, absolute/relative shared command, raw mixed/uniform MCP query, atomic history/pending guards and ordered sparse insertion now pass native/MCP/runtime/Editor acceptance. See [contract](weight-multivertex.md). Auto/Smooth, brushes, locks, Update, multiple bones, sparse Deform remapping, Pies/overlay and exact styling remain separate work. |
| 4: next major 4.3 rigging gap | One world-space cross-property transform mapping | Existing data and MCP add/set constraint payloads are present, but evaluation refuses translation-to-rotation/scale mappings used by the announcement. Calibrate one mapping, implement it in the engine, expose real mapping controls and evaluate an MCP-authored rig against the external oracle. Local/additive modes remain separate. |
| 5: larger new model | A manually driven slider constraint | Add explicit animation/frame/mix ownership, model validation, shared evaluation, adapter, creation/key controls and strict MCP commands. Detect self-reference/cycles and unsupported timeline domains. A copied pose or an ordinary slot fade is not slider evaluation. Bone-driven sliders extend this slice later. |

The [4.1 release post, July 4, 2022](https://esotericsoftware.com/blog/Spine-4-1-released)
is the primary description of the separate Curves view: relative transition
editing with reusable user presets. It establishes why a scalar value Graph
alone does not cover that view, but does not establish current 4.3 placement.
The [current Weights guide](https://esotericsoftware.com/spine-weights)
documents vertex multi-selection and Direct/brush workflows. The November 15
slice supplies the one-mesh/one-bone Direct selection and semantic editing path;
the legacy raw `setMeshBoneWeight` keeps its stricter policy. Generated Auto/Smooth
and brush weights, locks, Update, multiple selected bones and Pies/overlay
controls remain different missing capabilities.

The installed-editor order controls also exposed an interchange restriction:
changing only sparse B,A to A,B resolves the original import failure, while
consistent C,A,B imports too. This supports consistent per-mesh precedence, not
mandatory numeric sorting. New positive sparse insertion preserves old entries
under an independent precedence extension. Editor export rejects a precedence
cycle before writing; Runtime preserves raw arrays. The exact accepted final
Editor artifact passes 52 source/returned poses at the unchanged 0.001 tolerance.
Native re-export prunes five zero entries and rounds some weights, so the result
does not establish raw-lossless later authoring.

## Broader update backlog

These are feature areas explicitly covered by the
[4.3 announcement](https://esotericsoftware.com/blog/Spine-4.3-released), mapped
to the current implementation and its missing gates. The accepted slices do
not complete this broader objective.

| Public update area | Current gap and required native/MCP work |
| --- | --- |
| Dopesheet-centric synchronization and curve refinements | Shared scalar/Deform/slot selection and bounded Curves/Graph actions pass native/MCP/runtime acceptance. Stored preset library, default curve preferences, native grouping and uncalibrated/generalized handle adjustment remain. |
| Sliders | No native model, evaluation, import/export or creation workflow; add shared commands and strict schemas with independently derived behavior. |
| Transform property mapping and local/world choices | Data is modeled, but evaluation is restricted and mapping editing is pending; do not equate accepted data with working constrained motion. |
| Multiple-mesh tracing and weight workflows | Explicit payload creation and bounded one-mesh/one-bone Direct selection/editing exist through shared native/MCP paths. Native topology generation, multiple-mesh/bone workflows, Auto/Smooth/brush/locks/Update, sparse Deform remapping and Pies/overlay remain. |
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
