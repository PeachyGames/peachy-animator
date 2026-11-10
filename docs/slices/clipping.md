# Clipping — November 10, 2026

Three game skeletons cut attachments with clipping polygons. Native v19 adds
`Attachment::Clipping`: optional name, polygon vertices (slot-local or
weighted), `end` slot, `convex` flag and the editor's nonessential color.

Observed with the external 4.3.13 runtime (public typings; original probes
through its headless `SkeletonRendererCore` and `SkeletonClipping`; no
implementation source read):

- A shown clipping attachment cuts the attachments drawn after its slot,
  through `end` inclusive. With no `end`, or an `end` drawn earlier, it clips to
  the end of drawing.
- **Its own slot never ends it:** with `end` set to the clipping slot itself
  (the female pose does this), everything after it is clipped. A hand-written
  render loop that called `clipEnd` for the clipping slot got this wrong; the
  runtime's own renderer is the oracle.
- A clip that starts while another is active is ignored; a later one starts
  normally after the first ends.
- `convex` clips to the polygon's convex hull; otherwise the polygon may be
  concave. An omitted `convex` is false in the runtime. Inverse clipping is
  refused.
- Installed Spine 4.3.26's JSON import turns `convex` on even when the file says
  false, and fills an omitted `end` with the last slot; the runtime is followed.

Scenes keep each draw's own geometry for editing tools and add `clipped`
geometry for rendering and picking. A concave polygon is split by ear clipping
and each triangle is clipped against each convex piece, with UVs interpolated
from the source triangle. The runtime may decompose differently, so the runtime
check compares covered area and a coverage grid per sample, not vertices.

Deform keys on clipping attachments now import, export and evaluate (see the
[path/deform follow-up](path-constraints.md#deform-keys-on-paths-and-clipping-polygons--november-10-2026)).
The viewport draws their evaluated polygons in the stored editor color and
supports outline selection, including ignored nested clips. Convex clips retain
the authored polygon outline while clipping uses its hull. Vertex editing,
Spine's translucent polygon fill and inverse clipping remain unimplemented.
