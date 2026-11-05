# Architecture

The reference model is [PhotoCraft's architecture](https://github.com/storytold/photocraft/blob/main/docs/architecture.md).
The first slice adopts its separation of document, engine, I/O and frontend without
replicating a large application's crate count ahead of working features.

```text
desktop (egui + wgpu)        cli (headless)
          \                  /
           engine      spine adapter
                  \   /
                 document
```

`peachy-document` contains plain serializable types and validation. Hierarchical bones
are topologically ordered; parent indices point backwards. Slots establish draw order.
Skins map slot/attachment names to regions. Transforms use degrees and a Y-up coordinate
system; frontends handle screen conversion. Translation and rotation animation values
offset the setup pose, while scale values multiply it.

`peachy-engine` owns Session, command dispatch and history. A command runs against a
candidate document and commits only after validation. A batch is one atomic edit.
Undo/redo restores document snapshots and branching clears redo. The first slice uses
bounded full snapshots; structural sharing is an optimization for larger documents.
Dirty state compares the current document with the last saved snapshot.

Pose evaluation is stateless: asking for time T always starts with setup transforms,
samples the selected clip and composes local affine matrices through the hierarchy.
Scrubbing backwards never depends on previous playback. Region draw geometry is
derived from those same matrices. The external 4.3 curve observations are documented
in PROVENANCE.md.

`peachy-spine` converts between the supported Spine 4.3 JSON subset and our editable
model. It does not call or contain the official runtime. Capability inspection runs
before import and refuses known unsupported semantics. Export uses scalar timelines,
converts normalized Bezier handles into absolute time/value coordinates, and retains
the authored clip duration with a terminal holding key where necessary.

The desktop owns file dialogs, textures, atlas packing and native persistence. It
renders textured region quads through egui's wgpu backend. Canvas drag previews are
temporary; release commits one command. The inspector and CLI dispatch the same engine
commands. No animation evaluation is reimplemented in the frontend.

## Extension points

1. Add typed mesh/weight data before renderer-specific mesh controls.
2. Add evaluated draw operations to the core when region-only rendering grows.
3. Use stable entity IDs before introducing deletion/reparenting and animation merging.
4. Move filesystem orchestration into a shared I/O crate when CLI bundle export arrives.
5. Add schemas and a command registry/MCP transport around the existing serializable
   commands. The first slice supplies CLI automation, not an MCP server.

The app is native-only today. The data/engine crates avoid OS and UI dependencies,
but WebAssembly compilation and a web frontend have not yet been verified.
