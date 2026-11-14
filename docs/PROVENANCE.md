# Implementation provenance

## Animate mesh vertices and texture startup — November 14, 2026

The public [mesh transform tools guide](https://esotericsoftware.com/spine-meshes#Transform-tools)
provides the Setup/Animate and vertex-drag distinction. Dense offsets reuse the
already verified native deform representation. Weighted translation is our own
inverse-affine calculation, checked by a forward world-coordinate residual;
no proprietary implementation was read. Runtime checks use the separately
installed runtime only through public APIs. Native reference/interaction captures
and private training inputs remain local; no third-party artwork is committed.
Texture handling was derived from the locally installed MIT-licensed egui/eframe
API and the actual WGPU device limit.

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

## MCP and Views work — November 6, 2026

The agent interface uses the independently maintained official
[Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk) and the public
[MCP tools specification](https://modelcontextprotocol.io/specification/2025-11-25/server/tools).
Its protocol and schema dependencies remain confined to `apps/mcp`.

PhotoCraft's local README and automation manifests were inspected to confirm its
headless-engine and optional desktop-bridge architecture. Peachy Animator's MCP
tools are written independently around its existing Session and shared I/O, rather
than copying PhotoCraft's application implementation.

The Views audit uses the public [Spine Views guide](https://en.esotericsoftware.com/spine-views),
[user interface guide](https://en.esotericsoftware.com/spine-ui) and named view pages.
It separates documented panel names and behavior from exact installed-editor mode
lists and docking behavior, which require further interactive comparison. New
panels and vector icons are independently implemented. See [the view inventory](VIEWS.md).

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

## Additional binding and Direct references — November 6, 2026 UTC

The public [Weights Direct guide](https://esotericsoftware.com/spine-weights#Direct)
describes exact weights for selected vertices/bones and a Weight control.
[Erika's staff explanation](https://esotericsoftware.com/forum/d/16165-lock-a-bones-weights-in-specific-percent/5)
supports proportionally resizing the remaining weights when one percentage is
changed. [Nate's staff explanation](https://esotericsoftware.com/forum/d/11318-weight-smoothing-not-working-properly/8)
supports initial single-bone 100% binding and subsequently added bones at 0%.
Those references were retrieved through Agent Reach's official-page/Exa routes
and are linked in the [approved design](slices/weight-direct-design.md). A historical
discussion of missing locks is not evidence that current Spine lacks locks.

Our Add command's inverse-current-world initialization for the new zero entry is
an approved mathematical policy; the public zero-weight convention does not
establish Spine's exact new bind position. Direct's independently written formula
uses a wider sum for the other positive weights, then f32 storage. Its explicit
selected-vertex normalization is separate from unchanged import/inspection/Add
and unselected raw data. The engine intentionally permits Direct deformation.

Rejecting ambiguous repeated selected-vertex bones, sparse missing targets and
sole-positive reductions without recipients defines a bounded first authoring
slice. These are not observations of Spine's complete edge behavior. No duplicate
aggregation, sorting, pruning, equal fallback or silent normalization was inferred.
Public references do not resolve locks, multi-selection, Update or brush semantics
for this implementation.

The independently written Direct controls reuse the existing Weights pane, vector
icon, bone colors and retained explicit selection. Frozen-pose fixed-base previews
and cancellation safety reuse our shared Session boundary. No editor/runtime
implementation or proprietary UI assets were inspected or copied. Original Peach
Bot derivatives are the only acceptance inputs; artist files remain untouched.

Separate CLI, external-runtime, installed-editor and native-interaction results
belong in the current validation section. Neither runtime agreement nor successful
editor import establishes exact Weights docking, visual/interaction parity or
lossless editor re-export.

Black-box installed Spine 4.3.26 CLI probes import/re-export seven of eight Direct
results, including both ordinary [8,7] and [7,8] limb orders. A repeated-influence
Add result fails editor import despite runtime acceptance; no universal order rule
is inferred from the older controlled probe. The seven accepted editor outputs
show bind-coordinate quantization up to 0.00476837158203125 and positive-weight
changes up to 0.0000044405460357666016 on the Direct weighted-bot input. This new
result limits the older fixtures' positive-weight-preservation finding to those
fixtures. Adapter acceptance of changed output is not a lossless-original-data
claim. Every source input remains unchanged; no interactive Spine Weights UI was
observed by these CLI probes.

The full native run on `694576a` uses only task-owned original artwork and confirms
typed 30% and 45% controls, real pointer/text cancellation, pending Animate locals,
history and Save restoration. Its two reviewed captures show our independently
written controls and artwork, not extracted proprietary assets. See
[native acceptance](evidence/direct-native.json) and
[visual review](evidence/direct-visual.json). These observations of our editor
remain separate from interactive Spine comparison and exact docking/pixel parity.


## Independent bone shear — November 6, 2026 UTC

The [shear design](slices/shear-design.md) and
[engine observation record](../crates/engine/SHEAR_PROVENANCE.md) separate public
contracts, independently measured data and unobserved UI behavior. Sources are the
official Bones/Tools/Keys/JSON/4.3 API guides, staff guidance linked by the design,
and public-API execution of original synthetic JSON in the separately installed
Spine runtime 4.3.13. No editor/runtime implementation or constants were inspected.

Angular basis math uses general affine composition. A coefficient was independently
calibrated from four unit-root outputs, selected before held-out tests, and checked
against observed finite matrices. Failed ordinary-Pi/modulo/f32-sum candidates are
retained locally. This establishes bounded numerical behavior, not an assertion
about proprietary implementation. Native v5, migration, region/bone separation,
wide finite arithmetic and guarded scene evaluation are independently authored.

The installed editor 4.3.26 imported and re-exported all 17 original shear cases,
including an original Peach Bot reference project; the subsequent Shear Lab gate
adds an eighteenth accepted input. Inputs remained unchanged.
This was a public command-line data check; no interactive Spine UI or screenshots
were obtained. Screen Capture preflight was false inside the verified fresh GUI
login, with no permission prompt or settings changes. Exact Shear display,
empty-space/Shift behavior and handle cues remain comparison gates. Our current
local-value policy and independently drawn handles are explicitly bounded.


The final independently authored desktop slice is native-verified with original
Shear Lab input. Its reviewed captures contain our artwork and independent vector
controls. Fixed-baseline input, focus routing and checked scene/grid arithmetic
reuse our existing Session and shell boundaries. The native-edited saved document
also passes the separately licensed runtime oracle. No editor/runtime implementation,
source map, constant or proprietary UI asset was inspected, copied or redistributed.
The [UI scope](slices/shear-ui.md) retains the unobserved reference gates.

## Independent normal IK — November 6, 2026 UTC

The Phase A solver combines general planar IK/affine mathematics with original
input/output observations executed through the separately licensed public Spine
4.3 runtime API. Winding and tie behavior, applied transforms, reach and precision
boundaries are recorded in `crates/engine/IK_PROVENANCE.md`. No proprietary editor
or runtime implementation, source maps, function text or internal constants were
read, copied or translated. The runtime is not a Cargo dependency or distributed
artifact. Installed-editor CLI import/re-export is a distinct data gate.

Official public guide images supply independently observed UI ordering: Constraints
folder, annotations, IK Constraint heading, linked Parent/Child/Target, Options
Positive, numeric Mix before slider. Their version is unknown and older controls
are visible. Image URLs and limitations appear in `evidence/ik-visual.json`; those
images are neither committed nor reused as app assets. The UI and vector drawings
are independently authored. Native screenshots contain only our original Peach Bot.
Exact installed 4.3 visual/interaction parity remains unverified.

Native v6, authored/applied evaluation, frame-aware gestures, strict migrations and
format rejection are our independently designed boundaries. Binding reuses one
applied pose snapshot without solving twice. Public runtime controls justify accepting
neutral `skin:false` while skin-required constraints remain unsupported. Empty
constraint lists preserve all eight earlier export goldens. See the approved
[contract](slices/ik-design.md), [UI scope](slices/ik-ui.md) and final validation.


## Independent Playback view — November 6, 2026

The official public [Playback guide](https://en.esotericsoftware.com/spine-playback)
informed the Timeline FPS/Speed ordering, percentage presets and temporary,
project-wide preview-speed semantics. It also identifies the still-missing FPS
retiming, Stepped and Interpolated features. The live page is not an installed
4.3 comparison. Our 1–200% bounds, chosen presets and added shared transport are
bounded local policy. The UI, play/slider icon and speed adaptation are original
Rust code over our existing engine transport; no proprietary implementation,
reference images or UI assets were copied into the application. Native project
format and exported animation timing are unchanged.


## Independent Ghosting — November 6, 2026

The official [Ghosting guide](https://en.esotericsoftware.com/spine-ghosting)
supplies the public onion-skin workflow and control vocabulary. Our pure sampler,
checked-scene presentation, transient settings, figure icon and alpha-mask display
are original Rust code. Sampling math uses general frame/time arithmetic; whole-key
stride, 64-pose capacity and coincident-loop-endpoint behavior are explicitly
bounded local contracts. Exact native time identities preserve nearby distinct
keys and identify anchored lattice positions without a general authoring epsilon.

The existing exporter and evaluator are checked against the separately licensed
Spine 4.3.13 runtime through its public API, using only our original Peach Bot,
weighted and IK fixtures. No runtime/editor implementation is read or redistributed.
The guide's live content is not evidence of exact installed 4.3 UI parity. Native
captures show our own original artwork. Motion Vectors and Silhouette/Xray outlines
are not approximated or presented as implemented modes.


## Independent compact Timeline — November 6, 2026

The official [Timeline](https://en.esotericsoftware.com/spine-timeline) and
[Keys / Timeline](https://en.esotericsoftware.com/spine-keys#Timeline) guides
inform the placement and ruler workflow. The range/map/tick math, exact key-time
aggregation, transient input ownership, frame field and vector icons are original
Rust implementation. General f64 view arithmetic and native f32 endpoint
identities prevent nonfinite or invisible cursors without changing authored keys.

No proprietary implementation or visual asset was inspected/copied, and no new
runtime dependency, document format or export field was introduced. The existing
separately licensed public runtime API remains an external geometry/export oracle.
The live public guides do not prove exact installed 4.3 interaction or pixel parity.
The screenshot uses only our original Peach Bot artwork.


### Mesh Tools soft selection — November 6, 2026

Behavioral reference: the public [Mesh Tools view guide](https://en.esotericsoftware.com/spine-mesh-tools),
read November 6, 2026, describes Size, Feather, Hull vertices and viewport falloff
cues. Our pure engine geometry, atomic command composition and native egui controls
are original implementations. World-distance calculations and inverse affine
translation use general mathematics. The linear outer-falloff formula, one-point
selection, exact radius endpoints and bounded slider range are explicit local
policies. No proprietary source or UI assets were inspected or copied. The
external public-API oracle remains separately installed and unredistributed.


### Independent Preview — November 6, 2026

The public [Preview guide](https://en.esotericsoftware.com/spine-preview), read
November 6, describes a separate runtime-like animation display useful during Setup,
with pan/zoom, speed, tracks and mixing. Our bounded single-track view reuses the
original checked evaluator and transport, with independently written egui controls,
projection math and icon. It does not implement track mixing or copy runtime/editor
implementation or UI assets. Loop-current behavior, percentage limits and shortcut
ownership are explicit local policies. All captures use original artwork.


### Scalar Graph view — November 6, 2026

Reference: public [Graph guide](https://en.esotericsoftware.com/spine-graph).
The original implementation uses existing engine scalar sampling and SetKey
transactions; native f64 chart mapping, bounded display sampling, exact-key
selection and icons are independent code. It edits only existing key values,
not times/tangents. Setup-offset/multiplier display is an explicit bounded scope,
not a claim of every installed-editor Graph convention. No proprietary code or
UI assets are inspected or copied.


### Setup Slot Color — November 6, 2026

The public [Slot Color](https://en.esotericsoftware.com/spine-slot-color),
[Slots](https://en.esotericsoftware.com/spine-slots) and
[JSON format](https://en.esotericsoftware.com/spine-json-format) guides establish
selected-slot/attachment-owned color and setup RGB/RGBA hex. Original public-API
stimuli confirm the6/8-digit defaults and every byte per channel against the
separately installed4.3.13 runtime. The adapter deliberately rejects malformed
hex/null rather than copying runtime permissiveness. No runtime source is read.

Native7 schema/migration/commands, slot propagation, HSV chooser/gesture lifecycle,
renderer tint and icons are independent code. General HSV mathematics uses public
egui Hsva conversions. The compact custom drawing avoids global numeric-color
style mutation or copied assets. Native bytes remain unmultiplied; premultiplication
occurs only at rendering. The separate future IK contract is explicitly renumbered
to native8 without changing its approved semantic requirements or gates. (November 9:
native8 went to physics data; the IK contract now targets native9.)


### WAV waveform and independent Audio audition — November 6, 2026

The public [Audio guide](https://en.esotericsoftware.com/spine-audio-view) describes
keyed audio events, which remain outside this bounded session auditioner. RIFF
chunk parsing, integer/float PCM normalization and bounded channel extrema use
independent standard-format code. Native output calls public NSSound methods
through the already used, separately maintained objc2 AppKit bindings, following
Apple's public [NSSound API](https://developer.apple.com/documentation/appkit/nssound)
and [transport guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/Sound/Tasks/PlayingAudioData.html).
Native observation additionally found that NSSound can keep `isPlaying` true
while paused; the adapter combines that value with a successful pause state and
checks the stopped native clock. No proprietary Spine implementation, private audio or copied icons are used. Tests
and captures generate an original PCM tone; native state verification sets only
that owned sound's volume to zero, never system volume. No listening-quality or
complete Spine Audio interchange/parity claim is made.


### Physics constraint data — November 9, 2026

The public [physics constraints guide](https://esotericsoftware.com/spine-physics-constraints)
describes each setting qualitatively; the public JSON format page predates physics
and the unified 4.3 `constraints` array. Field names, omitted values and key
defaults were therefore observed from original probe files: installed Spine
4.3.26 import/re-export through its public CLI, and the separately installed
4.3.13 runtime's public data API (`PhysicsConstraintData`, its `setupPose` and
timeline frames), read as a black box. Public TypeScript declarations named the
`ScaleYMode` values. No runtime or editor implementation source was read; no
simulation behavior is derived or claimed. Data model, adapter, commands, UI and
the pendulum icon are independent code.


### Nested IK chains, child offsets and attachment names — November 9, 2026

Sequential solving and the offset correction are elementary triangle geometry,
checked only by black-box comparison with the external runtime on original
fixtures. The attachment-name rule was corrected from installed-editor CLI probes
with original minimal JSON. No implementation source was read.


### Bone inheritance modes — November 9, 2026

The public [bones guide](https://esotericsoftware.com/spine-bones) describes the
Inherit checkboxes qualitatively. Exact formulas were fitted to black-box
observations of the external runtime's public API for original random rigs:
candidate models were tested numerically and kept only when they reproduced
every sample. The degenerate-parent switch was located the same way and is
refused rather than modeled. Editor CLI probes with original JSON established
the field/key format. No runtime or editor implementation source was read.


### Transform constraints and IK ancestors — November 9, 2026

Public TypeScript declarations named the 4.3 data fields. Defaults came from
original probes through the installed editor CLI and the runtime's public data
API. Evaluation rules were fitted to black-box runtime observations of original
random rigs; the IK ancestor relaxation rests on the same kind of comparison. No
runtime or editor implementation source was read.


### Events, reference scale, constraint name scope and crossed curves — November 10, 2026

Event fields and defaults came from an original probe through the installed
editor CLI (import/re-export) and the runtime's public data API. Per-type
constraint names and crossed Bezier handles were observed in a shipped game
export and confirmed by black-box runtime comparison of original fixtures. No
runtime or editor implementation source was read.


### Draw order keys — November 10, 2026

The public JSON format page defines offsets relative to the setup draw order
but not how unlisted slots are placed. That rule, and the malformed cases, were
established by black-box comparison of original random probes against the
runtime's public `Skeleton.drawOrder` and through installed-editor round trips.
No runtime or editor implementation source was read.


### Attachment tint and curve edge cases — November 10, 2026

The attachment color contract came from an original probe through the installed
editor CLI and the runtime's public attachment `color`. Offset and
out-of-interval curves were compared black-box against the runtime with original
fixtures; the shipped game export only showed that they occur. No runtime or
editor implementation source was read. Reading egui's own (MIT) source to
confirm `SliderClamping` defaults is ordinary dependency inspection.


### Linked meshes — November 10, 2026

Shared fields come from the public `MeshAttachment`/`Attachment` typings.
Source lookup, the `timelines` default and color came from original probes
through the runtime's public API (`getSourceMesh`, `timelineAttachment`,
`computeWorldVertices`), and the field layout from installed-editor round trips.
No runtime or editor implementation source was read.


### Transform remapping — November 10, 2026

The mapping rule was fitted to black-box runtime outputs for original probe
rigs; field names were already observed in the transform constraint slice. No
runtime or editor implementation source was read.


### Clipping — November 10, 2026

Semantics come from the public `ClippingAttachment`/`SkeletonClipping`/
`SkeletonRendererCore` typings and black-box probes of the runtime's headless
renderer with original rigs (rendered area and coverage). The polygon
decomposition is our own (ear clipping, Sutherland–Hodgman); it is compared by
coverage, not by matching the runtime's vertices. No runtime or editor
implementation source was read.


### Path constraints — November 10, 2026

Field names and defaults came from the public `PathConstraintData`/
`PathAttachment` typings and the runtime's public data API. The evaluation rule
(4-segment distances, 10-segment in-curve table, end extension, tangent
rotation, mixes) was fitted to black-box runtime outputs on original random
paths. No runtime or editor implementation source was read.


### IK scale and stretch — November 10, 2026

Rules were fitted to black-box runtime outputs on original random rigs and
probes (stretch factor, mix blending, ignored child Y offset, mirrored bones).
No runtime or editor implementation source was read.


### Image sequences and blend modes — November 10, 2026

Field names, defaults and frame selection came from the public `Sequence`/
`SequenceTimeline`/`AttachmentLoader` typings and black-box probes through the
runtime's public API (`resolveIndex`, `getPath`, `sequenceIndex`). Sequence Lab
frames are copies of original Peach Bot art. No runtime or editor
implementation source was read.


### Path and clipping viewport guides — November 10, 2026

Visual references: the public [Paths](https://esotericsoftware.com/spine-paths)
and [Clipping](https://esotericsoftware.com/spine-clipping) guides, plus actual
mouse selection in task-owned installed Spine 4.3.26 windows opened on our
original Path/Clip Labs. Orange paths, cyan selection points and attachment
editor colors were observed directly; the fallback clipping color was also
checked through `new ClippingAttachment(...).color` in external 4.3.13. Runtime
`PathAttachment.color` defaults to white, so the editor's observed orange is the
viewport fallback instead. These are black-box/public API observations.

World guide vertices reuse our existing weighted/deform evaluation. Cubic knot
interpretation is shared with our independently derived constraint solver;
painting uses egui's MIT-licensed cubic Bezier primitive. No proprietary editor
or runtime implementation, icons or UI assets were inspected/copied. Committed
screenshots show original Peach Bot artwork in our application only. Installed
reference captures and native-input attempt logs remain local.
