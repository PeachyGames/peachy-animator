# Compatibility contract

Target: Spine **4.3.x JSON**, with export metadata `4.3.26`. The initial external
oracle is separately installed `@esotericsoftware/spine-core` **4.3.13**.
Compatibility applies to the subset below, not every 4.3 project or future patch.

Static IK is a bounded subset: one or two connected bones, unit positive scale
and zero shear on their chain/ancestors, and a parent for the first bone. Chains
may nest (a toe below a leg) and solve sequentially in constraint order; shared
constrained bones and targets inside another constrained chain are refused.
Two-bone child translation is any constant nonzero setup offset, including Y. Target coincidence and unrepresentable
solved frames fail explicitly; ordinary inner/outer unreachable targets work.
Native9 Setup Mix is any finite value (signed and overshooting Mix observed);
legacy formats keep [0,1]. See the [complete Phase A rules](slices/ik-design.md).
Neutral advanced defaults and `skin:false` are accepted; active advanced features
and skin-required constraints are rejected even at Mix 0. Grouped IK Mix/Positive
timelines import and export ([animated IK](IK_ANIMATION_REVIEW.md)); raw curve
values are stored at f32, an open precision gap ([contract](slices/ik-raw-precision.md)).
Input-only obsolete integer `order` is validated and omitted, never used to sort.

| Area | Supported | Import refuses |
| --- | --- | --- |
| Bones | All five inheritance modes (normal, onlyTranslation, noRotationOrReflection, noScale, noScaleOrReflection) and stepped `inherit` keys; X/Y, rotation, scale X/Y, shear X/Y, length | Legacy `transform` alias, inheritance changes on IK-constrained bones, degenerate zero-scale parents, skin-required bones |
| Attachments | Regions, unweighted meshes and weighted meshes with explicit UVs/topology and bone-local influences | Linked meshes, clipping, paths, sequences, other attachment types |
| Slots | Setup attachment, exact RGBA tint and normal blending in fixed draw order | Dark/two-color tint, other blend modes |
| Skins | Region/mesh maps, explicit attachment names and default-skin fallback | Skin-required bones or constraints |
| Animation | Bone rotation, translation, scale and shear; combined or scalar input timelines; grouped IK Mix/Positive timelines; physics setting and reset timelines, including global (`""`) timelines | Slot, attachment, draw-order, event, transform and path timelines |
| Curves | Linear, stepped, normalized monotone-time cubic handles | Nonfinite or unordered controls; overshooting curves between equal values |
| Constraints | One-/two-bone normal IK with keyed Mix/Positive, two-bone Positive, nested chains in list order, off-axis child offsets; animated targets. Physics constraints as data: every setting, Scale Y mode and Global flag, in unified constraint order | Advanced IK settings, shared-bone or cross-target chains, transformed chains; skin-required physics; transform, path, slider and other constraints |
| Images | Loose PNGs for editing; one untrimmed atlas page for desktop export | Packed-atlas import, rotated/trimmed atlas regions, multipage export |
| Files | `.peachy` v10; in-memory v1–v9 migration; Spine JSON adapter | Native `.spine` projects and binary `.skel`; `skeleton.referenceScale` other than 100 |

Our own project format is JSON and holds the editable rig, skin references and clips.
It is distinct from a runtime export. The importer ignores documented editor-only
metadata such as bone colors/icons and skeleton bounds; it blocks unsupported animation
semantics. It is not a lossless editor-project converter.

CLI import/export creates new files. Desktop exports use a new uniquely named directory.
Desktop saves back up an existing `.peachy` file before atomically replacing it; when
saving elsewhere, image collisions are refused rather than overwritten.

For independently obtained artist exports, run `peachy-cli inspect FILE.json` before
planning a migration. The first local review inspected 11 current exports without
modifying them. All needed features beyond this slice, so no artist migration is claimed.

## Acceptance evidence

The IK checkpoint passes 288 integrated Rust tests and the required format,
Clippy, build and architecture checks. Nine original packed fixtures agree with
the separately installed runtime across all clips/skins within 0.001 world units;
the rig actually edited in the native UI passes the same gate. Original IK Lab
imports/re-exports through Spine Editor 4.3.26. These are separate data, runtime and
native UI gates; exact installed-editor visual/interaction parity remains open.
See [current validation](VALIDATION.md#normal-ik-phase-a--november-6-2026-utc).

The original fixture also imports into a new native project with Spine Editor
4.3.26, re-exports from that editor, and imports back into Peachy Animator. Its
re-exported animations retain the same sampled transform agreement. Skin
placeholders must not collide with direct attachments in the default skin.
Attachment names may repeat across skins and slots (corrected November 9, 2026
after installed-editor probes; shipped game exports reuse names). Select `peach` or `mint` for a complete demo character.

- Rust checks cover interpolation, transform composition, validation, command atomicity,
  undo/redo, curve conversion and file save/reopen/backup/export.
- The original demo's two animations are compared at 51 times each against the external
  runtime: 9,180 scalar matrix/position comparisons. The largest initial observed error
  was below 0.00003; the check enforces a 0.001 tolerance.
- A separate synthetic contract adds nonuniform parent scale, animated X/Y and scale,
  stepped interpolation and a 400-degree rotation: 4,590 more comparisons, initially
  below 0.00005. Its reproducible source is `examples/peach-bot/contract-cases.peachy`;
  pass that project as the fourth positional argument to `scripts/check-runtime.mjs`, following
  the runtime module, its exported JSON and the demo atlas.
- This establishes sampled transform agreement for those fixtures. It does not establish
  pixel equivalence, all combinations of features, or support for unimplemented features.

Fresh exported JSON, runtime reports, original-demo screenshots and local native
interaction evidence are retained during development. Private artist inputs and their
reports stay outside the repository.

## Mesh slice

Region conversion preserves its name, path, image size, setup transform, UV mapping
and draw order by baking the attachment transform into four slot-bone-local vertices.
Setup vertex motion deforms the image with fixed UVs. A triangle may be subdivided
at its centroid; UVs interpolate at that point, and existing indices and hull stay
unchanged. The UI shows triangle edges and numbered handles. These are deliberately
bounded operations, not a replacement for all of Spine's Edit Mesh tools.

Unweighted meshes must be triangulated and have matching coordinate/UV pairs,
UVs within 0–1, a hull count of at least three, valid nonduplicate triangle indices,
and every vertex referenced by a triangle. Nonessential image dimensions and edge
pairs are preserved when supplied. Edge entries use coordinate offsets, twice the
vertex index, as confirmed by the installed editor's round trip. Runtime-only meshes
may omit dimensions/edges. Missing hulls are refused. Tint other than white, linked
meshes, sequences and all attachment/deform timelines remain blocked. Weighted
mesh import and read-only inspection are described in the current slice below.

The unweighted slice introduced native v2 attachment tags; v3 subsequently removed
stored duration in favor of actual hold keys. The v4 representation added tagged mesh vertices. The current v5 representation
adds required bone shear fields and migrates v1/v2/v3/v4 in memory. Explicit Save writes v5 and
preserves a verified backup. Unknown native fields/future versions are refused.

The external harness now checks all three fixture skins. Across mesh demo and reflected,
nonuniform-scale contract fixtures it checks 7,650 world coordinates and 15,300 local/
packed UV values. Maximum mesh error was 0.0000610352 units (tolerance 0.001), with
zero observed UV error. It also checks triangle order and hull count. Separate checks
pass after the installed Spine Editor 4.3.26 round trip and the shared packed exporter.
The editor retained all three meshes' vertices, UVs, triangles, hulls and edges exactly.
These checks establish sampled geometry/format agreement; they do not establish pixel
rendering equivalence, weight/deform behavior or complete Spine parity.

## Tree editing slice

The native Tree and viewport now share typed selection for bones, slots, placeholders,
attachments, skins and animations. Setup region transforms, bone lengths and slot
setup attachments are shared commands, used by both frontends. At that slice the
native format was v2; export remains the same bounded Spine 4.3 JSON target.

Skin/animation activation, search, type filtering and temporary visibility are editor
state and do not change exported data. Attachment visibility in Setup changes the
slot's saved setup attachment and supports undo. Attachment timelines in Animate remain
unsupported. Inactive appearances can be inspected, while GUI editing requires their
skin to be active. CLI commands address explicit attachment keys and can edit any skin.

Viewport picking uses triangulated geometry and slot draw order; it does not sample
image alpha. Only selected bones and editable setup regions expose transform controls;
slots have no implicit bone transform target. Full Tree/selection parity and other
interaction limits remain in UI_PARITY.md.

The external harness now compares region world corners too, identifying corner pairs
by their UVs through the official public RegionAttachment API. The unmodified mesh
fixture passes 28,560 region coordinates (maximum error 0.000030518), alongside its
existing 5,100 mesh and 27,540 bone values across all three skins and two clips.

## Unkeyed pose and Auto Key slice

Auto Key off changes a transient engine pose; explicit keying converts it into the
same supported transform timelines. Default authoring keys translation/scale axes
as pairs; imported scalar tracks and the low-level command API remain supported.
That authoring slice retained native v2; the current native format is v5 and the
Spine target remains 4.3.x JSON. Unkeyed poses are not
serialized or exported. Save is not a substitute for pressing K to preserve a pose.
The UI distinguishes unkeyed edits from document dirty state and warns on discard.

A fresh read-only audit of the same 11 artist exports still finds none fully supported;
all source hashes are unchanged. This slice changes authoring, not the accepted
production feature set. See [workflow milestones](WORKFLOWS.md) for every hurdle,
including the distinction between editable `.peachy`, Spine runtime JSON/atlas, and
unsupported native `.spine`/binary `.skel` files.


## Key-derived duration and native v3

Animation duration is the highest authored key time. Empty and frame-zero-only
clips both have duration zero; their key/property coverage remains distinct.
Import preserves tiny positive durations without a frame-length floor. Export
writes actual keys only, with no synthetic padding or empty-animation root key.
Manual/automatic keying, paste and retiming extend or shrink the derived end.

Native v3 stores no duration field; stale duration fields are rejected. Legacy v2
clips materialize every hold the old exporter emitted, including its neutral root
hold for all-empty clips. A formerly final unused Bezier is normalized to Linear
when a hold is appended, matching the previous exporter; Stepped is retained.
This preserves prior exported behavior, rather than promising identical unused
native curve metadata. v1 regions follow the same duration migration after tagging.
Loading never rewrites a file. Explicit Save retains a byte-exact verified backup.

`AddAnimation { name, duration }` remains a compatible command: zero creates an
empty clip and a positive value creates a visible neutral root hold. The editor's
New animation UI remains pending. Nonlooping playback remains bounded to the last
key; extended playback, loop ranges and exact installed-editor empty-playback UI
behavior remain parity gaps. Native `.spine`/binary `.skel` support is unchanged.

## Weighted meshes and native v4 — November 6, 2026 UTC

Native v4 tags mesh vertices as `slotLocal` positions or `weighted` per-vertex
influence lists. Each influence stores a global bone index, a bone-local bind
position and a weight. Validation checks all skins, including inactive attachments
and zero-weight references. Coordinates/weights must be finite, weights must be
within 0–1, and each vertex needs a positive total. More than four influences,
repeated bone entries, zero entries and nonunit positive totals are supported;
the adapter preserves their order and values without normalization or pruning.

Spine JSON weighted arrays use a count followed by bone/X/Y/weight groups for
each UV vertex. Counts and bone indices accept integer tokens and mathematically
integral JSON floats such as `1.0`; fractional, negative, overflowing, truncated
or trailing data is refused. Float-to-index bounds are strictly below `2^usize::BITS`.
All meshes retain the topology/UV/hull restrictions above. Linked meshes and
attachment/deform/constraint timelines remain unsupported.

The loader migrates v1 region tags, captures and validates v2 duration, and wraps
v2/v3 mesh arrays as slot-local vertices. Complete native schema/geometry/curve and
legacy time-bound validation happens before adding v2 holds. Object-shaped legacy
vertices, raw-array v4 vertices and stale v3/v4 duration fields are rejected.
Load is read-only; explicit Save preserves verified original bytes before writing v4.

The desktop's existing **Vertices** action selects posed weighted handles and shows
each influence for the selected vertex in Setup or Animate. It preserves mode and
pending pose and stops playback. Dragging/nudging weighted vertices, local-position
entry and subdivision are guarded in both shell and engine. Bind/Unbind, Direct/
brush weight editing, locks, Update bindings and weighted topology editing are
future authoring work. Supported bone transforms still drive weighted artwork.

Three original runtime fixtures pass **7,800 weighted world-coordinate comparisons**
at the unchanged **0.001** tolerance; maximum observed mesh error is
**0.000068664551**. Native inspection, bone pose/key/history/save, two backups, and
unweighted drag/subdivision regressions pass. See [current validation](VALIDATION.md#weighted-meshes-and-native-v4--november-6-2026-utc).

Installed Spine **4.3.26** imports and re-exports the demo and transform contract.
Its output drops a zero influence, reorders entries, and preserves positive weights
and raw nonunit totals at f32 precision. It also changes local bind coordinates by
up to **0.0047683716**. Comparing that output against the pre-editor native source
fails the unchanged 0.001 geometry tolerance (observed error **0.0012054443**).
Importing the actual editor output and comparing that document against its runtime
consumer passes. This proves agreement for the changed output, not a lossless
round trip from the original source. Import versus export quantization is not
isolated; no documented precision override was found.

A controlled repeated-bone input is accepted by the runtime but rejected by the
editor. Sorting or zero pruning alone did not repair that case; merging its duplicate
with identical bind positions allowed import. That observation does not justify
merging arbitrary influence data. The codec retains repeated entries; the separate
runtime fixture keeps this boundary visible. See [editor acceptance](evidence/weighted-editor-roundtrip.json).

The read-only artist audit removes weighted-mesh refusals from **10 to 0**, with no
new inspection issues and all **11** source inputs unchanged. **0/11** are fully
supported: all 11 still report constraints, with other unsupported features varying
by character. No production-character migration is claimed. See
[redacted inventory](evidence/weighted-artist-summary.json).

## Initial Bind and Remove-all — November 6, 2026 UTC

This extends the weighted import/inspection baseline above. Native format remains
v4 and export remains the bounded Spine 4.3 JSON subset. The new commands initially
bind an unweighted mesh to one bone at weight 1 per vertex, or remove all bindings
into slot-local positions. Both preserve the current captured world shape within
0.001 and leave UVs/topology, bone transforms, keys and pending bone pose unchanged.
No normalization or merge/prune of imported influence data is introduced.

Session APIs accept Setup or Animate, including unkeyed posed locals. Only required
inverse transforms are used. Singular, nonfinite and finite-but-imprecise results
with a forward world-coordinate residual above 0.001 fail atomically. Shape
preservation is at that pose; changing bindings intentionally changes how other
bone poses drive the mesh. Saving bindings does not save an unkeyed bone pose.

The desktop exposes G, a Weights view, initial Bind and Remove all. It requires an
active appearance and the slot's active placeholder, resolving default fallback to
an explicit attachment reference. CLI commands can address explicit references in
other skins. Weighted vertex edits/subdivision, additional bindings, Direct/brush
changes, Auto, normalization, locks and Update are not implemented.

Command/native checks and three external runtime cases pass against each edited
result's own export at the unchanged tolerance. Installed Spine 4.3.26 imports and
re-exports two original results and our adapter accepts that output. This is neither
a lossless original-data round trip nor Weights UI/layout acceptance. The separate
native run verifies our implemented controls and current-pose behavior; the right
column dock remains provisional. See [binding validation](VALIDATION.md#initial-bind-and-remove-all--november-6-2026-utc).

## Additional binding and Direct — November 6, 2026 UTC

This extends the two historical weighted/binding checkpoints above without
changing native v4 or the adapter's accepted subset. Bind on an already weighted
mesh can append a genuinely new bone at zero weight on every vertex. Old weights,
positions, zeros, repeated entries and order remain intact; there is no import or
Add-time normalization. A bone already present anywhere, even at zero weight,
rejects rather than filling sparse entries. Its new local position is initialized
from the captured world point through the shared checked inverse, with the same
absolute 0.001 forward residual and finite-zero-contribution guards.

Direct explicitly changes one logical vertex and one uniquely bound bone. Other
positive weights share the remainder proportionally, while positions, zero
entries, order, other vertices and animation keys stay unchanged. Explicit Direct
may normalize that selected vertex's imported nonunit total and deform it. A
display-rounded focus/blur or invalid text does not dispatch that normalization.
Direct needs no inverse or shape-preserving residual; it validates finite candidate
world geometry at the captured pose.

Repeated bones on the selected vertex, a missing selected-vertex target and
reducing a sole positive weight without another positive recipient are rejected
limits. Repeated entries elsewhere remain supported native/import data. These
guards are our bounded authoring policies, not observed behavior for every Spine
edge case; no silent merge, equal-recipient fallback, sort or prune is introduced.

Setup/Animate edits retain actual time and pending poses without keying them.
Preview gestures freeze their explicit target and pose, use the shared baseline
and commit one history step. Escape and context/history/Save/Export boundaries
cancel owned previews before continuing; saves still exclude unkeyed poses.
Weights remains a provisional right-column pane with one selected vertex/bone.
Brushes, locks, Auto, Update, individual removal and weighted vertex/topology
editing remain unsupported.

CLI and external-runtime results are recorded in the latest validation section;
installed-editor interchange and native interaction are separate gates. Installed
Spine 4.3.26 imports/re-exports seven of the eight Direct cases, including both
ordinary [8,7] and [7,8] limb binding orders. The repeated-influence Add fixture
fails that editor's import while passing the runtime; it is retained unchanged.
These observations do not establish a universal parent-first influence order.

For the seven editor outputs, measured bind-coordinate changes reach
0.00476837158203125 and positive-weight changes reach 0.0000044405460357666016
on the Direct weighted-bot case. Other measured positive weights are unchanged or
within one f32 ULP. The independent adapter accepts all seven re-exports, but this
does not prove lossless source-data or shape preservation, or interactive Spine UI
parity. Do not generalize earlier fixtures' positive-weight preservation to every
Direct output or canonicalize our data to repair the failed case.

The seven editor outputs also remove one to four zero entries. Reimported results
can therefore contain a bone on only some vertices. Such data remains valid, but
the current missing-target and existing-global Add guards do not fill those sparse
bindings; import acceptance does not promise every later weight edit is available.

See [CLI](evidence/direct-cli.json), [runtime](evidence/direct-runtime.json), and
[installed-editor interchange](evidence/direct-spine-editor.json). Separate
[native acceptance](evidence/direct-native.json) verifies numeric/slider edits,
one-step history, cancellation before Save, exact pending-pose retention and five
Save backups. [Visual review](evidence/direct-visual.json) verifies loaded original
artwork and the implemented cues; it does not establish exact Spine docking or
pixel equivalence.

Additional weight authoring does not add constraints, linked/deform timelines, binary `.skel`
or native `.spine` support, and does not advance the **0/11** production-import result.


## Bone shear and native v5 — November 6, 2026 UTC

Bone setup shear uses `shearX`/`shearY` in Spine JSON. Animation accepts paired
`shear` with `x`/`y`, or lowercase scalar `shearx`/`sheary` with `value`; values
are setup-relative degrees. Paired/scalar collisions are refused even when a
conflicting track is empty. Scalar Bezier keys require four absolute controls;
paired keys require eight. Export uses scalar tracks and omits zero setup shear,
so all seven retained zero-shear export goldens remain unchanged.

Native v5 requires both finite bone shear fields. Region attachments retain their
five-field transform and reject shear fields. Read-only migration adds zero only
to actual v1–v4 bone transforms after refusing new fields/tracks in those legacy
schemas. Explicit Save writes v5 with the existing verified-backup behavior.
Older binaries cannot open v5. Serialized bone commands also require all seven
fields; they are not project-migration envelopes.

Finite curve spans and controls use wider intermediates only when needed, while
preserving ordinary f32 results and the observed ten-segment interpolation policy.
Controls genuinely outside the finite native range fail explicitly before export.
Checked scene evaluation rejects nonfinite local/world transforms, geometry, UVs
or rendering bounds before the CLI serializes a pose or the desktop draws/picks it.
No clamping or replacement geometry is used. This remains a bounded f32 model.

Data interchange and interactive UI acceptance are distinct. Exact installed
Shear handle/empty-space/Shift behavior and World/Parent numeric display remain
comparison gates; see [desktop scope](slices/shear-ui.md). Successful CLI imports
do not establish those behaviors or full Spine UI parity.


## Setup Slot Color and native7 — November 6, 2026

Every current slot requires `color:[r,g,b,a]`, exactly four integer bytes0–255 in
straight/unmultiplied RGBA. White is `[255,255,255,255]`. Zero alpha preserves
hidden RGB in native files, commands and Spine interchange. Legacy formats1–6
reject any slot.color presence, even white/null, before the explicit6→7 migration
adds white to valid slot objects. Missing/malformed current colors remain errors.
Source files are unchanged on load; explicit Save writes7 with the normal backup.

Spine setup slots accept strict six-digit RGB (opaque alpha) or eight-digit RGBA
ASCII hex, either case. Omission means white; malformed/null values are refused.
Nonwhite colors export as eight lowercase hex digits, including hidden RGB at
zero alpha. White is omitted, so all9 previous original white JSON exports remain
byte-identical. The external4.3.13 public API agrees on1,029 original cases and
4,116 normalized channel comparisons with zero error. No runtime implementation
is read/copied or redistributed.

Tint reaches all native image views. Ghost Images multiply slot RGBA with the
ghost color; Solid keeps its chosen RGB and applies slot alpha. Color affects
image pixels, not geometry, picking/bounds or bone overlays. This is setup tint
only: slot/attachment/color animation timelines, dark/two-color tint, additive
and other blend modes remain refused. Native7 does not add placeholder IK tracks;
the separately approved pending [IK animation contract](slices/ik-animation-design.md)
then targeted native8 (now native9, after physics data) with the same gates. No new private-rig acceptance audit is
claimed. See [Slot Color evidence](evidence/slot-color-view.json).


## Audio audition scope — November 6, 2026

The Audio view adds independent session-only WAV inspection/audition, not an audio
attachment/event model. Project format remains 7. RIFF little-endian mono/stereo
PCM8/16/24/32 and finite float32 at 8–192 kHz are bounded to 64 MiB. Checked metadata,
chunk bounds/padding, frame alignment and byte rate guard both the waveform and
unchanged native playback snapshot. Waveform extrema cover all channels and the
last sample, bounded to 4,096 bins. macOS playback uses NSSound; waveform inspection
is available without that backend on other platforms. No new audio crate/runtime
is redistributed. Audio timelines, synchronization, recorded data, other formats,
cross-platform output and audio Save/export remain unsupported. [Evidence](evidence/audio-view.json)
separates mock UI checks, silent native backend state and native input acceptance.

## Physics constraint data and native v8 — November 9, 2026

Physics constraints import, edit, save and export as **data**. The editor does
not simulate physics: the evaluated pose equals the external runtime with
`Physics.none`, and the Properties panel says so. The game runtime simulates the
exported settings. See the [slice scope](slices/physics-data.md).

| Spine 4.3 field | Native | Omitted value |
| --- | --- | --- |
| `bone` | `bone` index | required |
| `x`, `y`, `rotate`, `scaleX`, `shearX` | `influence` | 0 |
| `scaleY` (`none`/`uniform`/`volume`) | `scale_y_mode` | none |
| `limit`, `fps` | `limit`, `fps` (positive) | 5000, 60 |
| `inertia`, `strength`, `damping`, `mass`, `wind`, `gravity`, `mix` | `setup` (mass > 0, mix ≥ 0) | 0.5, 100, 0.85, 1, 0, 0, 1 |
| `<setting>Global` | `global` flags | false |
| `skin` | refused when true | false |
| `order` | validated, not stored; array order is evaluation order | — |

Defaults were observed, not assumed: inertia **0.5** and damping **0.85** differ
from the values many older references show. Export writes every field explicitly,
so consumers never depend on defaults.

Animation `physics` maps constraint name, or `""` for global timelines, to
`inertia`/`strength`/`damping`/`mass`/`wind`/`gravity`/`mix` keys and time-only
`reset` keys. Omitted key values are 1 for `mix` and 0 otherwise; omitted times
are 0. Values are absolute settings, not offsets. Physics and reset keys extend
the animation end, W/S key navigation and Timeline markers. Keys must reference
a physics constraint; each constraint/setting and reset list appears once.

Native v8 adds required `physics` and `physics_resets` lists to every animation.
v7 and earlier migrate in memory with empty lists and refuse any physics
constraint or physics field. `.peachy` files are rewritten only by an explicit save.

## Bone inheritance modes and native v10 — November 9, 2026

Bone `inherit` and animation bone `inherit` keys (`{time, inherit}`, stepped,
omitted value normal) round-trip. Native v10 adds required `Bone.inherit` and
`Animation.inherit_tracks`; v9 and earlier migrate with normal modes and empty
lists and refuse inheritance fields. Evaluation matches the external runtime;
see [the slice](slices/inherit-modes.md).
