# Linked meshes — November 10, 2026

Four game skeletons reuse a mesh's shape with another image (`linkedmesh`).
The public runtime typings say a linked mesh shares its source mesh's bones,
vertices, region UVs, triangles, hull, edges, width and height, but has its own
name and path (image).

Black-box observations (original probes; no implementation source read):

- 4.3 JSON names the source with `source`. It is looked up in the same slot of
  `skin`; an omitted `skin` means the default skin, never the linked mesh's own.
- An omitted `timelines` means the source's deform keys apply (the runtime
  reports the source as the timeline attachment); `false` means they do not.
- Color is the linked mesh's own, white when omitted (not the source's).
- Installed Spine 4.3.26 writes `skin` only for non-default sources, writes
  `timelines` only when false, and adds the source's width and height.

Native v18 keeps a linked mesh as a `Mesh` with a copy of the source geometry
plus `link` (source skin, source placeholder, `timelines`). Every consumer
(drawing, picking, weights, export) sees an ordinary mesh. Validation requires
the copy to equal the source. After every command, source geometry edits are
copied into their links, and edits to a link alone are refused ("edit the
source mesh"). A deform track naming a linked mesh that follows its source is
refused, because the runtime would ignore it. Export writes `linkedmesh` again
without geometry.

Not yet: creating or unlinking linked meshes in the editor, and chains (a linked
mesh as a source).
