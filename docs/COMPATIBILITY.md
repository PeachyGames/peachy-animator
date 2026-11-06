# Compatibility contract

Target: Spine **4.3.x JSON**, with export metadata `4.3.26`. The initial external
oracle is separately installed `@esotericsoftware/spine-core` **4.3.13**.
Compatibility applies to the subset below, not every 4.3 project or future patch.

| Area | Supported | Import refuses |
| --- | --- | --- |
| Bones | Normal inheritance; X/Y, rotation, scale X/Y, length | Shear, other inheritance modes, skin-required bones |
| Attachments | Regions, unweighted meshes and weighted meshes with explicit UVs/topology and bone-local influences | Linked meshes, clipping, paths, sequences, other attachment types |
| Slots | Setup attachment and normal blending in fixed draw order | Tint, dark tint, other blend modes |
| Skins | Region/mesh maps, explicit attachment names and default-skin fallback | Skin-required bones or constraints |
| Animation | Bone rotation, translation and scale; combined or scalar input timelines | Slot, attachment, draw-order, event or constraint timelines |
| Curves | Linear, stepped, normalized monotone-time cubic handles | Nonfinite or unordered controls; overshooting curves between equal values |
| Constraints | None | IK, transform, path, physics and other constraints |
| Images | Loose PNGs for editing; one untrimmed atlas page for desktop export | Packed-atlas import, rotated/trimmed atlas regions, multipage export |
| Files | `.peachy` v4; in-memory v1/v2/v3 migration; Spine JSON adapter | Native `.spine` projects and binary `.skel` |

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

The original fixture also imports into a new native project with Spine Editor
4.3.26, re-exports from that editor, and imports back into Peachy Animator. Its
re-exported animations retain the same sampled transform agreement. Appearance
attachments use unique names, and skin placeholders do not collide with direct
attachments in the default skin. Select `peach` or `mint` for a complete demo character.

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
stored duration in favor of actual hold keys. The current v4 representation adds
tagged mesh vertices and migrates v1/v2/v3 in memory. Explicit Save writes v4 and
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
That authoring slice retained native v2; the current native format is v4 and the
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
