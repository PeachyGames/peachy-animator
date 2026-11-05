# Architecture

The reference model is PhotoCraft's engine-first architecture. This project keeps
its document, commands, deterministic evaluation and format facts separate from
platform effects, with six crates sized around implemented responsibilities.

```text
Desktop / CLI -> engine -> document
Desktop / CLI -> io -> spine -> document
CLI -> spine (capability inspection)
```

`peachy-document` contains plain serializable data and general invariants. Bones are
topologically ordered; parent indices point backwards. Slots establish draw order;
skins resolve region references with default fallback. Transforms use degrees and a
Y-up coordinate system. Translation/rotation keys offset setup, while scale multiplies
it. Native data rejects unknown fields so future features cannot silently disappear.

`peachy-engine::apply_command(&Document, Command)` is a pure document transition. It
returns a validated candidate or an error without changing the input. `SetPose` owns
setup-to-key conversion, frame snapping and curve preservation, shared by interactive
and automated clients. Session adds bounded snapshot history, dirty state and gesture
transactions around that transition. Cancelled gestures preserve the prior redo branch.

Evaluation starts from setup on every call. `evaluate_scene` resolves skins and returns
world transforms, textured region quads in draw order and their common bounds. Its drag
preview is transient input rather than a document edit. Rendering and fit-to-view use
those same vertices; frontends never rebuild attachment geometry. The external 4.3
curve observations are recorded in PROVENANCE.md.

`peachy-spine` converts the supported 4.3 JSON subset. It owns Spine-specific naming
and placeholder restrictions, separate from native invariants. Inspection returns a
typed report; malformed containers and duplicate skins are rejected before decoding
could discard data. It contains no official runtime implementation or I/O calls.

`peachy-io` is the shared imperative shell for decoding/loading, verified backups,
atomic project replacement, complete new-file publication, image copying and packed
exports. It depends on document/spine, not the engine or a UI toolkit. The CLI calls
the pure transition for batches and uses this same file service. Its model-only
import/apply outputs still require loose images beside the project; desktop Save copies
images when saving to another directory. Both frontends use the same bundle exporter.

The desktop owns dialogs, texture upload, camera and pointer coordinates, labels and
platform events. Its entrypoint parses arguments and starts the app. `editor/mod.rs`
coordinates application state, `panels.rs` renders controls, and `viewport.rs` translates
pointer gestures and paints the evaluated scene. View selections are repaired after
history/document changes. Inspector pointer drags and curve sliders use Session
transactions rather than creating history entries for every intermediate value.

## Guardrails and expansion

`scripts/check-architecture.py` checks actual production/build dependency edges against
an explicit allowlist, including a negative probe for an engine-to-UI dependency. The
core was also reviewed for standard-library filesystem/clock/process calls; the graph
check alone cannot establish their absence. See [the quality review](CODE_QUALITY.md).

1. Extend typed native data and evaluated draw operations before building mesh controls.
2. Introduce stable entity IDs before deletion/reparenting and animation merging.
3. Extend the shared I/O shell for formats/destinations; keep dialogs in the frontend.
4. Add schemas and MCP around the existing serializable commands when needed.
5. Measure snapshot memory with real mesh/keyframe workloads before adding structural sharing.

The app is native-only today. Core crates have no OS/UI dependencies, but WebAssembly
compilation, a web frontend and additional platform builds have not been verified.
