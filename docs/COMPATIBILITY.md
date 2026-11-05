# Compatibility contract — mesh slice

Target: Spine **4.3.x JSON**, with export metadata `4.3.26`. The initial external
oracle is separately installed `@esotericsoftware/spine-core` **4.3.13**.
Compatibility applies to the subset below, not every 4.3 project or future patch.

| Area | Supported | Import refuses |
| --- | --- | --- |
| Bones | Normal inheritance; X/Y, rotation, scale X/Y, length | Shear, other inheritance modes, skin-required bones |
| Attachments | Regions and unweighted meshes with explicit UVs, triangles, hulls and setup vertices | Weighted/linked meshes, clipping, paths, sequences, other attachment types |
| Slots | Setup attachment and normal blending in fixed draw order | Tint, dark tint, other blend modes |
| Skins | Region/mesh maps, explicit attachment names and default-skin fallback | Skin-required bones or constraints |
| Animation | Bone rotation, translation and scale; combined or scalar input timelines | Slot, attachment, draw-order, event or constraint timelines |
| Curves | Linear, stepped, normalized monotone-time cubic handles | Nonfinite or unordered controls; overshooting curves between equal values |
| Constraints | None | IK, transform, path, physics and other constraints |
| Images | Loose PNGs for editing; one untrimmed atlas page for desktop export | Packed-atlas import, rotated/trimmed atlas regions, multipage export |
| Files | `.peachy` v2; in-memory v1 migration; Spine JSON adapter | Native `.spine` projects and binary `.skel` |

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

Imported meshes must be unweighted, triangulated, have matching coordinate/UV pairs,
UVs within 0–1, a hull count of at least three, valid nonduplicate triangle indices,
and every vertex referenced by a triangle. Nonessential image dimensions and edge
pairs are preserved when supplied. Edge entries use coordinate offsets, twice the
vertex index, as confirmed by the installed editor's round trip. Runtime-only meshes
may omit dimensions/edges. Missing hulls are refused. Tint other than white, linked
meshes, sequences, weights and all attachment/deform timelines remain blocked.

Native v2 tags each attachment as `region` or `mesh`. The loader migrates v1 regions
in memory, then applies the strict v2 schema. Save writes v2, preserving a verified
backup of any previous native file. Unknown native fields/future versions are refused.

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
setup attachments are shared commands, used by both frontends. Native format stays v2;
export remains the same bounded Spine 4.3 JSON target.

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
Native format remains v2 and Spine target remains 4.3.x JSON. Unkeyed poses are not
serialized or exported. Save is not a substitute for pressing K to preserve a pose.
The UI distinguishes unkeyed edits from document dirty state and warns on discard.

A fresh read-only audit of the same 11 artist exports still finds none fully supported;
all source hashes are unchanged. This slice changes authoring, not the accepted
production feature set. See [workflow milestones](WORKFLOWS.md) for every hurdle,
including the distinction between editable `.peachy`, Spine runtime JSON/atlas, and
unsupported native `.spine`/binary `.skel` files.
