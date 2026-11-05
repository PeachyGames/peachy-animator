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
skins resolve tagged region/mesh attachments with default fallback. Transforms use degrees and a
Y-up coordinate system. Translation/rotation keys offset setup, while scale multiplies
it. Native data rejects unknown fields so future features cannot silently disappear.

`peachy-engine::apply_command(&Document, Command)` is a pure document transition. It
returns a validated candidate or an error without changing the input. `SetPose` owns
setup-to-key conversion, frame snapping and curve preservation, shared by interactive
and automated clients. Session adds bounded snapshot history, dirty state and gesture
transactions around that transition. Cancelled gestures preserve the prior redo branch.

Evaluation starts from setup on every call. `evaluate_scene` resolves skins and returns
world transforms, textured vertex/UV/index buffers in draw order and their common bounds. Its optional
preview remains available to pure callers; the desktop now uses cancellable command
transactions for both bone and region pointer gestures. Rendering and fit-to-view use
those same vertices; frontends never rebuild attachment geometry. The external 4.3
curve observations are recorded in PROVENANCE.md.

Mesh vertices live in slot-bone local coordinates; UVs use a top-left origin.
Region conversion bakes its attachment transform into four local vertices, using
the same geometry function as region rendering. Subdivision inserts a triangle
centroid and interpolates its UV; it preserves the hull and existing vertex indices.
Both operations and setup vertex movement are shared serializable commands. The
canvas converts pointer motion to bone-local coordinates and dispatches a Session
transaction. Mesh rendering, bounds and CLI geometry all use `evaluate_scene`.

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
The decoder explicitly migrates native v1 region data to v2 tagged attachments
before strict deserialization. It does not weaken unknown-field validation.

The desktop owns dialogs, texture upload, camera and pointer coordinates, labels and
platform events. Its entrypoint parses arguments and starts the app. `editor/mod.rs`
coordinates application state, `panels.rs` renders controls, `shortcuts.rs` maps
Spine conventions to existing operations, and `viewport.rs` translates
pointer gestures and paints the evaluated scene. `mesh.rs` coordinates attachment
selection and controls. View selections, including vertex and triangle indices, are repaired after
history/document changes. Inspector pointer drags and curve sliders use Session
transactions rather than creating history entries for every intermediate value.

## Guardrails and expansion

`scripts/check-architecture.py` checks actual production/build dependency edges against
an explicit allowlist, including a negative probe for an engine-to-UI dependency. The
core was also reviewed for standard-library filesystem/clock/process calls; the graph
check alone cannot establish their absence. See [the quality review](CODE_QUALITY.md).

1. Extend typed native data and evaluated draw operations before building weights or deform controls.
2. Introduce stable entity IDs before deletion/reparenting and animation merging.
3. Extend the shared I/O shell for formats/destinations; keep dialogs in the frontend.
4. Add schemas and MCP around the existing serializable commands when needed.
5. Measure snapshot memory with real mesh/keyframe workloads before adding structural sharing.

The app is native-only today. Core crates have no OS/UI dependencies, but WebAssembly
compilation, a web frontend and additional platform builds have not been verified.

## User interface contract

The user requires Spine's visual layout, controls and conventions. The native shell
must follow [UI parity](UI_PARITY.md), independently of export/runtime compatibility.
UI layout and shortcuts stay in the desktop, while pose/key changes still use shared
commands. Deselecting clears transient view selection without editing the document. Tree/properties are
on the right; the viewport, main transform toolbar and Animate-mode Dopesheet occupy
the left. UI presentation changes do not change the file/engine adapters.

## Typed Tree selection

`editor/selection.rs` holds pure presentation identities. The Tree and viewport use
one `Option<TreeItem>`; there is no remembered bone index behind a selected slot,
skin or attachment. Reconciliation clears invalid identities after document changes
instead of redirecting a later edit. Active animation/skin and temporary visibility
are separate view state and do not enter document history.

`tree_model.rs` projects the document into an outline without egui, file or GPU calls.
It resolves placeholders through the document's shared `resolve_attachment` method,
retains search ancestors, and projects slot-only views in draw order. `tree.rs`
paints that outline and sends selection/activation/visibility actions; `properties.rs`
chooses controls for the selected entity. No frontend directly mutates the document.

`SetBoneLength`, `SetRegionTransform` and `SetSlotAttachment` extend the same pure
command boundary used by the CLI. Slot attachment names are validated against that
slot's available placeholders. The engine's triangle picking and attachment bounds
are shared geometry queries, also used after temporary viewport visibility filtering.
The small macOS application-menu adjustment lives in `apps/desktop/src/platform.rs`,
using safe AppKit bindings; those dependencies are allowed only in the desktop shell.

## Transient animation authoring

`engine/authoring.rs` holds sparse absolute local `PoseEdits`, transform grouping,
key-state queries and posed-local evaluation. These are engine state, never fields in
`Document`, native files, or exports. `Session` snapshots document plus transient pose
in a single bounded history; its saved baseline/dirty comparison includes only the
document. Clients query pending edits separately. No secondary UI undo stack exists.

`Session::edit_pose` validates both keyed and unkeyed edits through the same pure
command boundary. `key_pose` explicitly keys supported properties; `key_edited`
constructs one atomic batch across edited bones, preserving setup-relative conversion
and existing curves. Failure changes neither document, pending pose nor history.
Transactions cover both kinds of state. Timeline discard can be recovered with Undo
at the new frame if no redo branch exists; context switches clear stale pose snapshots
without erasing document history. Timeline/mode/playback transitions are shell events.

The desktop's `animation.rs` coordinates those events and reports errors. Both numeric
fields and the viewport query Session's posed locals. `scene_from_locals` feeds those
locals through the existing shared geometry pipeline; rendering/picking/bounds do not
reimplement pose rules. The CLI's document evaluator intentionally ignores transient
poses, just like save/export. No runtime or platform dependency entered the core.
