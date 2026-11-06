# Implementation provenance

Started November 5, 2026 for Team Peachy. This is an independently written implementation
from public format facts, general animation mathematics and controlled observations.
The development record is not a legal certification of a formal two-team clean room.

## Architectural model

- [PhotoCraft architecture](https://github.com/storytold/photocraft/blob/main/docs/architecture.md),
  also read from an existing local checkout at commit
  `8a72978549cab915f9868f5ca7091fd52444514b`.
- Adopted ideas: data independent of GUI; commands shared by GUI and automation;
  headless core; explicit dependency layers; deterministic results; verification with
  independent reference inputs/outputs. The source here was written for this project.
- The frontend uses MIT/Apache-licensed egui/eframe and wgpu. Exact dependencies are
  retained in Cargo.lock; their original licenses continue to apply.

## Format and behavior references

- [Spine JSON format](https://esotericsoftware.com/spine-json-format).
- [Spine export guide](https://esotericsoftware.com/spine-export).
- [Spine versioning](https://esotericsoftware.com/spine-versioning).
- [Spine user guide](https://esotericsoftware.com/spine-user-guide).
- Public API declarations for the separately installed Spine 4.3.13 runtime, used to
  construct an external validation harness.
- Exported data from our authorized local artist repository was inspected to identify
  the 4.3 field layout and actual feature requirements. No artist data is bundled here.

No proprietary editor/runtime implementation source, disassembly or decompiler output
was inspected, copied, translated or ported into this project. REA was researched as
context; it was not installed or used to analyze Spine.

## Original fixtures and observed curve behavior

`scripts/make_demo.py` constructs all Peach Bot artwork, rig and animation data from
original geometric primitives using Python's standard library. The resulting source
project, loose PNGs and atlas are reproducible and licensed with this repository.
They do not derive from Esoteric's bundled example characters or private game artwork.

`scripts/check-runtime.mjs` calls a separately supplied runtime through its public API.
The initial exact-cubic evaluator disagreed with that oracle near Bezier endpoints.
For the original ease curve `(0.42,0), (0.58,1)`, a 4-unit translation at normalized
time `1/24` produced approximately `0.03930818` in the oracle, while exact cubic inversion
produced approximately `0.01336670`. A ten-segment parametric polyline yields the
observed value. Our engine independently constructs those ten segments from the cubic
equation and interpolates along them. Sampled fixture agreement is recorded in the
compatibility check; no official curve implementation was read.

Spine and its runtimes retain their separate licenses. External oracle execution is
optional, requires the user's own rights to use that software and does not make it
part of this application's MIT-licensed implementation.

The installed Spine Editor 4.3.26 was also run through its documented import/export
CLI on the original fixture only. Two initial imports revealed default-attachment
and named-skin placeholder collisions. The final fixture puts shared parts in default
and appearances in named skins with unique attachment names. It imports and re-exports
successfully, and its re-exported data passes the same external pose comparison.

## Mesh slice observations

The mesh slice uses the public JSON attachment contract and
[Spine's mesh user guide](https://esotericsoftware.com/spine-meshes). The latter
distinguishes fixed-UV setup vertex movement from UV/topology editing and animated
deform keys. Our first mesh controls implement fixed-UV setup movement only, plus
four-corner region conversion and local centroid subdivision.

Mesh positions use ordinary affine bone transforms. Region conversion uses our
existing region geometry, and subdivision averages the three position/UV pairs.
These algorithms were written independently from general mathematics. No official
triangulation, renderer, mesh evaluator or editor implementation was read.

Public TypeScript API declarations for MeshAttachment, VertexAttachment, Sequence,
SlotPose and TextureRegion were read to call the separately installed runtime oracle.
The oracle compares world vertices, normalized/packed UVs, triangle indices and hull
counts. The installed Spine Editor 4.3.26 imported/re-exported three original meshes,
retaining their mesh data exactly. That also confirmed the nonessential edge values
as coordinate offsets (twice the vertex index). All artwork and rigs remain original
Peach Bot fixtures; no private artist data was copied or modified.

## User interface alignment

The user explicitly requested Spine's visual layout and interaction conventions.
The independently implemented desktop now follows the documented default workspace
and supported default hotkeys, using the public
[UI guide](https://esotericsoftware.com/spine-ui),
[Views](https://esotericsoftware.com/spine-views),
[Tools](https://en.esotericsoftware.com/spine-tools/), and
[shortcut sheet](https://esotericsoftware.com/spine-cheat-sheet).
No proprietary interface source, font, icon or other UI asset was extracted.
The local Spine reference was opened with our original project using its documented
CLI project argument. UI_PARITY.md records acceptance requirements and outstanding
visual/behavioral differences, independently of data-format compatibility.

## Tree and selection behavior

The typed Tree follows public descriptions of
[Tree nodes and filters](https://en.esotericsoftware.com/spine-tree),
[slot visibility](https://en.esotericsoftware.com/spine-slots/),
[skin activation](https://esotericsoftware.com/spine-skins-view), and
[animation activation](https://en.esotericsoftware.com/spine-animations-view).
Node selection and visibility-dot activation are implemented as distinct view actions.
Slot setup attachments, bone lengths and region transforms use our independent engine
commands. Triangle picking uses general signed-area geometry and inverse slot draw
order; no official selection or rendering implementation was read.

The public RegionAttachment declaration from the separately installed runtime was read
to call getOffsets, computeWorldVertices and Sequence UV access in the oracle. The
extended harness identifies region corners by UV rather than assuming vertex order.
All compatibility inputs remain our original Peach Bot artwork and rigs.

## Unkeyed pose behavior

Auto Key, Key Edited (K), green/orange/red key indicators, default paired transform
axes and Undo recovery of a cleared unkeyed pose follow the public
[keying guide](https://esotericsoftware.com/spine-keys), read November 5, 2026.
The implementation uses our own sparse local-property edits and unified Session
snapshots; no editor/runtime implementation source or UI assets were read/copied.
Native input verification uses only original Peach Bot artwork. Export acceptance is
checked separately with installed Spine 4.3.26 and the external runtime.

## Transform tools and axes

Read the public [Tools guide](https://esotericsoftware.com/spine-tools) and
[shortcut sheet](https://esotericsoftware.com/spine-cheat-sheet), and inspected the
published toolbar/axis reference images. A task-owned original project was opened in
installed Spine 4.3.26; its window was verified on screen, but macOS refused capture.
No proprietary screenshot or UI asset entered the repository. Axis math uses ordinary
affine inverses, rotation vectors, projection and vector lengths, independently written
here. Native inputs and external geometry checks use original Peach Bot artwork only.

Selection history and numbered selection groups follow the public Tools guide and
shortcut sheet. Their typed, bounded model was independently written; verification
uses original fixtures, including recalling slots and skins without activating or
editing unrelated entities. No proprietary selection implementation was inspected.

Reverse playback and key navigation use the public shortcut sheet's A/D, W/S and
Q/E conventions. Stepping is independently implemented signed elapsed time and
Euclidean wrapping; nearest-key queries use our document timelines. No official
transport code was read. The bounded nonlooping behavior and navigation scope are
recorded explicitly in UI_PARITY.md rather than claimed as full playback parity.

## Interface iconography

The public UI and Views guide images were inspected for titlebar and menu cues:
open folder, disk, directional history arrows, windows, hierarchy and keyed timelines.
`editor/icons.rs` draws these conventional meanings independently with geometric
primitives. No reference pixels, proprietary icons or fonts are copied or loaded.
Public reference images remain only in the local task evidence directory. Repository
screenshots use our application and original Peach Bot artwork.

## Naming and key selection references — November 5, 2026

The public [Tree guide](https://esotericsoftware.com/spine-tree) documents the
properties Duplicate/Rename buttons and F2/double-click rename dialog conventions.
The [Animations view guide](https://esotericsoftware.com/spine-animations-view)
distinguishes inspection from activation. The [Dopesheet guide](https://esotericsoftware.com/spine-dopesheet)
and [Keys guide](https://esotericsoftware.com/spine-keys) document key selection,
Cmd/Ctrl toggling, deletion and double-click deletion. Their public properties and
Dopesheet row images were visually inspected outside Git. Shared rename/duplicate/
delete drawings were made independently with our vector primitives. No proprietary
editor or runtime implementation was inspected or copied.

Public guides do not establish every mode/activation/focus detail. The implementation
and remaining differences are recorded in UI_PARITY.md and the slice notes. Runtime
checks use the separately licensed runtime only as an external black-box consumer.

## Timeline retiming and clipboard references — November 5, 2026

The public [Dopesheet guide](https://esotericsoftware.com/spine-dopesheet) defines
horizontal key dragging, Shift frame-snap bypass and Cmd/Ctrl+Shift duplication.
The [Keys guide](https://esotericsoftware.com/spine-keys) defines Copy/Cut/Delete/Paste
and pasting supported bone keys onto another bone. Public official forum excerpts
on [key replacement and curves](https://esotericsoftware.com/forum/d/27677-connecting-two-curves-without-breaking-them)
and [frame snapping](https://en.esotericsoftware.com/forum/d/10002-frame-snapping)
informed bounded overwrite/snapping behavior. Exact advanced handle reshaping,
group corner cases and cross-project clipboard preferences remain comparison gates.
No proprietary implementation was read or copied. The clipboard and gesture code
are independently implemented on the existing pure commands and egui shell.


Selection expansion and row tool controls use the public
[Dopesheet guide](https://esotericsoftware.com/spine-dopesheet#Selection) and
[official empty-selection clarification](https://esotericsoftware.com/forum/d/17539-select-bones-keyframes-in-timeline-hotkey).
Selected-row union across several rows remains an explicitly recorded inference.
The footer stability change fixes movement observed in our own native captures;
no proprietary UI image or implementation was copied.


Key-derived duration follows the public [Keys/Timeline guide](https://esotericsoftware.com/spine-keys#Timeline)
and [Animation.duration API contract](https://en.esotericsoftware.com/spine-api-reference#Animation-duration).
The legacy hold migration is defined by our own preceding v2 exporter, captured as
an original golden fixture before changing that code. Installed Spine 4.3.26 is used
only to import/re-export original inputs; the separately supplied runtime is an
external geometry/end oracle. Neither implementation was inspected or copied.

## Weighted meshes and native v4 — November 6, 2026 UTC

The typed influence representation and packed JSON decoder follow the public
[JSON mesh contract](https://esotericsoftware.com/spine-json-format). Bone-local
binding and bone-driven deformation are described in the public
[Weights guide](https://esotericsoftware.com/spine-weights) and
[Mesh guide](https://esotericsoftware.com/spine-meshes). Our evaluator independently
sums each influence's weight times its bone's affine-transformed bind position.
The slot bone is applied only to unweighted slot-local positions. No official
skinning, parser, editor or runtime implementation was inspected, copied or ported.

Native v4 migration extends our own strict decoder: legacy mesh arrays are wrapped
without geometry changes, while the previously captured v2 export golden still
checks materialized duration holds. The adapter's integral-float index support was
prompted by read-only inspection diagnostics and verified with original fixtures;
no private character data was copied into tests.

`scripts/make_demo.py` derives `weighted-bot.peachy`, `weighted-contract.peachy`
and `weighted-repeated.peachy` from our original mesh/artwork. They exercise multiple
bones, reflection/nonuniform scale, more than four influences, zero entries,
repeated bones and raw positive nonunit sums. Committed adapter fixtures are also
original. The runtime harness calls only the separately supplied runtime's public
mesh world-vertex API and compares independently evaluated geometry.

The installed Spine 4.3.26 editor is a black-box import/re-export oracle. Controlled
probes distinguish its repeated-influence rejection from the runtime's acceptance.
The final demo and contract import, but editor output reorders/removes entries and
quantizes some bind coordinates. The original-source comparison fails at 0.001;
reimported editor data passes against that data's external runtime evaluation.
These are separate results, recorded in
[editor acceptance](evidence/weighted-editor-roundtrip.json), not a lossless claim.
The stage causing quantization remains unresolved. Public
[export](https://esotericsoftware.com/spine-export) and
[CLI](https://esotericsoftware.com/spine-command-line-interface) documentation
provided no supported position-precision override. Stable 4.3.26 must not be confused
with the older 4.3.26-beta when interpreting release notes.

Native controls show read-only per-entry influence values alongside our original
artwork. They do not claim Spine's Weights view, Bind or painting UI. Repository
captures show Peachy Animator only; proprietary reference images, native projects,
failed probes and detailed artist reports remain local. The committed artist audit
contains aggregate counters without identifying paths or artwork.

## Initial binding and Weights controls — November 6, 2026 UTC

The public [Weights guide](https://esotericsoftware.com/spine-weights),
[Tools guide](https://esotericsoftware.com/spine-tools),
[Views guide](https://esotericsoftware.com/spine-views), and
[mesh binding tutorial](https://esotericsoftware.com/blog/Mesh-binding-tutorial)
describe mesh-first/bones-first binding, G, retained tool selection, Remove-all and
binding at an Animate pose. The initial single-bone case has the trivial weight of
1 per vertex; it is not an implementation of Spine's Auto algorithm.

Our independent engine evaluates the current mesh with the shared world-vertex
pipeline and inverts the explicitly required target/slot bone to obtain new local
bind positions. Controlled finite-precision regressions prompted the explicit
0.001 forward residual rejection. No proprietary binding, editor, parser or runtime
implementation was inspected or copied. Original fixtures and permitted black-box
input/output comparisons supply the evidence; production artwork remains untouched.

The Weights icon is independently drawn. Public control-image references are linked
in [the slice contract](slices/weight-authoring.md); no proprietary UI artwork is
shipped. There is no successfully captured installed-editor default Weights dock.
The right-side pane is provisional, and the retained-Bind-after-scrub policy is an
explicit current policy rather than an observed Spine-parity claim.

The separately supplied runtime checks each edited result against its own export.
Two original results also pass installed Spine 4.3.26 CLI import/re-export followed
by independent adapter acceptance. Those checks establish neither a lossless
original-data round trip nor the installed editor's interactive Weights behavior.
Our separate native acceptance uses only original Peach Bot artwork. Earlier
weighted-slice reports, including the 140-test checkpoint and the editor-coordinate
quantization finding, retain their original scope.
