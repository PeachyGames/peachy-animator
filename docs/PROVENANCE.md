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
