# Deform timeline and Graph authoring — November 14, 2026

This continuation closes Deform key editing across the shared engine, native
editor and MCP. Native format v22 is unchanged. It is part of the continuing
Spine parity objective; it does not establish all-feature or pixel-level parity.

## Observed reference

Installed Spine **4.3.26** was opened with an original Peach Bot Deform Lab
export, in an owned native window with a 1500×960 client area. Actual screenshots
show blue `Deform: <attachment>` property rows under their owning bones, adjacent
Graph/Dopesheet tabs across the bottom beneath the Tree, and cyan deformation
transitions rising from 0 to 100 before resetting at each key. Deformation is a
transition between mesh poses, not a separate scalar X/Y curve for each vertex.
Reference screenshots stay local; no proprietary implementation was inspected.
The [official update audit](spine-blog-audit-20261114.md) links public sources
and records the larger remaining editor/MCP backlog.

## Shared engine and MCP

`AnimationKey::Deform` and `KeyOwner::Deform` carry an explicit `AttachmentRef`.
Linked attachments with shared timelines resolve to their canonical source;
independent links keep separate keys. Typed identities retain exact native-f32
times. Existing retime/copy/delete commands preserve offsets, outgoing curves,
ordering, key-derived duration and atomic Session history.

`copy_animation_keys` is a read-only MCP tool requiring the current revision,
an animation index and exact typed keys from one owner. It returns a portable
clipboard with relative times and complete values. Deform layout metadata
records unweighted vertex counts or weighted per-vertex influence counts.
A current-editor probe confirms cross-attachment paste between unweighted meshes
and into a retained weighted mesh with one influence per vertex. The engine
supports that equivalent segmentation in both directions. Paste also supports
compatible different clips; equal flat offset length alone does not establish
compatibility. A second native probe leaves a 20-value target key unchanged
when copying a 10-value unweighted key, and rejects equal-length weighted
layouts with different per-vertex segmentation. A same-layout weighted paste
at a new time succeeds as the positive control. Values remain in target-local
vertex/binding axes; no conversion, normalization or topology changes occur.

`setDeformCurve` changes one exact existing key's interpolation without
rebuilding offsets. Invalid references, malformed clipboards, nonfinite handles,
missing exact keys and stale revisions fail without publishing partial edits.
Mesh, path and clipping Deform tracks share these timeline commands. Native
path/clipping vertex editing remains a separate feature.

See [MCP usage](../MCP.md#deform-timeline-editing). Published schemas are strict
and expose Deform owners, keys, clipboards, curve edits and read-only copying.

## Verification and remaining work

The engine's focused timeline checks and actual MCP stdio workflow cover dense
value preservation, exact neighboring times, linked ownership, cross-attachment
paste, malformed layout rejection, atomic history, Save/reopen and source-byte
preservation. A retained MCP-authored original project also passes the external
Spine 4.3 runtime: 28,080 bone values, 5,200 mesh values and 10,400 UV values,
with maximum world error 0.00002654 and mesh error 0.00003052 at the unchanged
0.001 geometry tolerance.

Actual native mouse/key input verifies the full-width dock and Deform row,
retiming a weighted key from 0.75 to 0.9 seconds, tangent dragging with one-step
Undo/Redo, typing 25% into a curve handle, Graph keyboard Copy and Dopesheet
Paste/Delete, Escape cancellation without held-pointer rearming, playback and
Save/reopen in a fresh native process. Every original dense offset is unchanged.
The saved native-authored export also matches the external runtime with the same
counts and maximum errors above. All owned native/reference windows are closed.
See the [before/after review gallery](../evidence/deform-timeline.html) and
[scalar acceptance record](../evidence/deform-timeline.json).

Focused checks pass with two Cargo workers/test threads: engine `timeline`
25 (including 10 Deform checks), desktop Deform 19, scalar Graph 9, views 8 and
layout 1, MCP schema 5 and actual stdio 3. Filters overlap and are not additive
full-suite totals. Formatting, all-target workspace Clippy, workspace build and
architecture pass. Required hosted CI remains unresolved: GitHub lists zero
registered workflows; main integration is still gated.
The native clipboard policy is backed by the focused reference probes above;
arbitrary bone layouts and every other cross-mode combination are not claimed.
All-keyed bone-group hierarchy, arbitrary docking, multi-curve Graph selection,
advanced Offset and complete styling remain broader parity work.
