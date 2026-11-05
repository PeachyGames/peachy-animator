# Compatibility contract — slice 1

Target: Spine **4.3.x JSON**, with export metadata `4.3.26`. The initial external
oracle is separately installed `@esotericsoftware/spine-core` **4.3.13**.
Compatibility applies to the subset below, not every 4.3 project or future patch.

| Area | Supported | Import refuses |
| --- | --- | --- |
| Bones | Normal inheritance; X/Y, rotation, scale X/Y, length | Shear, other inheritance modes, skin-required bones |
| Attachments | Static rectangular regions with transforms and image paths | Meshes, linked meshes, clipping, paths, sequences, other attachment types |
| Slots | Setup attachment and normal blending in fixed draw order | Tint, dark tint, other blend modes |
| Skins | Region maps, explicit attachment names and default-skin fallback | Skin-required bones or constraints |
| Animation | Bone rotation, translation and scale; combined or scalar input timelines | Slot, attachment, draw-order, event or constraint timelines |
| Curves | Linear, stepped, normalized monotone-time cubic handles | Nonfinite or unordered controls; overshooting curves between equal values |
| Constraints | None | IK, transform, path, physics and other constraints |
| Images | Loose PNGs for editing; one untrimmed atlas page for desktop export | Packed-atlas import, rotated/trimmed atlas regions, multipage export |
| Files | `.peachy` v1; Spine JSON adapter | Native `.spine` projects and binary `.skel` |

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
