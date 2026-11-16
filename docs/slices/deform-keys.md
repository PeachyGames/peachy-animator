# Mesh deform keys — November 9, 2026

Thirteen game skeletons key mesh shapes (1,128 deform timelines; 961 on weighted
meshes). Native v13 adds `Animation.deform_tracks`: per attachment (skin, slot,
placeholder) keys with a curve and **dense** offsets. The Spine adapter reads the
sparse JSON (`offset` + `vertices`, missing values zero) and writes it sparse
the way the installed editor does (zeros trimmed into `offset`).

Observed with installed Spine 4.3.26 and the external 4.3.13 runtime:

- Unweighted meshes: two values per vertex, added to the setup positions.
- Weighted meshes: two values per bone influence, added to that influence's
  bone-local position before weighting.
- Bezier curves store absolute times and progress fractions; natively the
  times are normalized like every other curve.
- Before the first key the mesh shows setup vertices; after the last it holds
  the last key; a key without values is zero.

Sampled offsets ride in `AuthoredPose`, so the viewport, Preview, ghosts, CLI
and exports all show deformation. Slot renames keep keys attached. Commands that
change a keyed mesh's vertex or influence count are refused (keys would no
longer fit). The editor round trip drops zero-weight influences and reorders
the rest, moving deform values consistently (geometry within 0.0016).

Single-vertex Animate authoring is available; see
[the authoring contract](deform-authoring.md). Deform Dopesheet rows and
multi-vertex/soft transforms remain pending. Path/clipping deforms import and
evaluate, but their vertices remain read-only. Bounding boxes remain unsupported.
