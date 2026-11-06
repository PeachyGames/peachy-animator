# Peachy Animator

Build a native, open-source 2D skeletal animation editor with versioned Spine-compatible exports.

## Spine user interface parity

The user explicitly requires Spine's visual layout, controls, terminology and interaction conventions, alongside compatible animation data. This is a familiar replacement for Spine users. Do not invent a different workspace or shortcut scheme. Match the observed Spine 4.3 default workspace: viewport on the left, Tree with item properties on the right, Setup/Animate switch at the viewport's upper left, main transform toolbar below the viewport, and animation views below it. Use the installed editor and public user guide as behavioral/visual references; write all implementation and icons independently. Keep Peachy Animator's own identity.

UI parity is part of every feature's acceptance criteria. Record observations, implemented behavior and remaining differences in `docs/UI_PARITY.md`. Verify placement and real mouse/keyboard behavior. Do not claim pixel or interaction parity from export/runtime checks. Prioritize correcting the prototype's workspace and conflicting shortcuts before expanding weights or other features.

## Architecture

- `document`: serializable data, validation, no UI, rendering or filesystem dependencies.
- `engine`: commands, transactional undo/redo, deterministic pose evaluation. Depends on document only.
- `spine`: independent format adapters. Depends on document, never on official runtime code.
- `io`: shared imperative shell for file decoding, safe writes, image copying and packed exports. Depends on document/spine; no UI dependency.
- `apps/cli` and `apps/desktop`: platform clients; keep pose/key and region geometry rules in the engine.
- Follow PhotoCraft's engine-first and command-driven approach, scaled to implemented features.

## Independent implementation

Use public format documentation, general mathematics, our original fixtures and observations of permitted inputs/outputs. Record sources and limitations in `docs/PROVENANCE.md`. Do not inspect, copy, translate, vendor or port proprietary Spine editor/runtime implementation code. The official runtime may be used as an external, separately licensed compatibility oracle. Do not add it to our Cargo dependencies or redistribute it with our application.

Artist assets in the sibling `spine-files-2` project are private local compatibility inputs. Never modify their sources/exports or commit their artwork, atlas pages, skeleton data or screenshots here. Keep local reports outside Git or redact identifying content. Use our original Peach Bot fixture for committed examples and screenshots.

## Verification

Run `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo build --workspace`. Verify interpolation, command atomicity/undo, round trips and external runtime compatibility. The sibling game's Vitest prohibition applies to the game; this project uses Rust's built-in test runner.
Run `python3 scripts/check-architecture.py` too. Its allowlist checks real Cargo dependencies; update the rules explicitly when deliberately changing the layers. It does not prove arbitrary standard-library I/O cannot be introduced, so review core functions for side effects as well.

`apply_command(&Document, Command)` is the pure editing boundary. `Session` wraps it with in-memory history; pointer gestures use begin/commit/cancel transactions. Native documents and commands reject unknown fields. Export-target restrictions belong to the adapter, not native document validation. Reconcile transient editor selections after history/document changes. Introduce stable entity IDs before deletion/reparenting.

Native format v4 uses tagged `Attachment::Region` / `Attachment::Mesh` and derives
animation ends from keys. `MeshVertices` distinguishes slot-bone-local positions
from per-vertex lists of bone-local `BoneInfluence` values. The I/O decoder chains
v1 region tags, v2 duration capture and v3 mesh-array wrapping into strict v4
validation before materializing prior exported holds. Validate influence references
in every skin, including zero-weight entries and inactive attachments. Preserve
influence order, repeated bones, zero entries and positive nonunit totals;
imports/exports must not silently normalize, merge, sort or prune them.

Mesh topology, UVs, binding data and weighted geometry belong in document/engine.
`evaluate_scene` emits shared vertex/UV/index buffers for rendering, bounds and CLI
diagnostics. Mesh commands address an explicit skin/slot/placeholder; resolve default
fallback before building a command. Existing unweighted vertex indices remain stable
under centroid subdivision. Weighted meshes currently expose read-only influence
inspection; the shell and engine reject slot-local vertex edits and subdivision on
them. Binding/weight editing requires explicit commands and separate acceptance.
Add stable vertex identity before removal/reindexing or cross-topology animation
references, and remap all influences before any future bone reordering/deletion.

Track concrete Spine behavior and gaps in `docs/PARITY.md`. Check new geometry against the external runtime and exported data against the installed editor using original fixtures before claiming parity for a feature.

Never claim full Spine compatibility. Reject unsupported features on import and preserve the input unchanged. List supported version and feature subset in `docs/COMPATIBILITY.md`. New export fixtures belong here, not in the artist repository.

For headed browser verification, follow the workspace's `ab-headed`/`ab-cdp` isolation rules. Use a unique task session and the approved browser.

The Tree and viewport share one typed `Option<TreeItem>` presentation selection. A slot, skin, animation or attachment must never use a cached bone as an edit target. Selecting a name and activating its visibility dot are separate actions. Skin/animation activation and temporary bone/slot visibility stay outside document history; setup attachment visibility is an explicit `SetSlotAttachment` command. Reconcile invalid selections by clearing them, not redirecting an edit to another entity. Keep outline/search/filter rules pure in `tree_model.rs`, controls in the shell, and picking/bounds in the engine.

Unkeyed animation poses belong to `engine::PoseEdits`, outside `Document`. Session
snapshots the document and transient pose together; its saved baseline/dirty flag
compares only document data. Query pending edits separately for UI/discard notices.
Use `Session::edit_pose`, `key_pose` and `key_edited` for interactive bone authoring;
never implement key conversion, pending-pose rules or a second undo stack in the UI.
Both numeric controls and viewport geometry must use Session's posed locals.
Timeline movement/playback clears pending edits; context switches clear stale pose
history. Save/export must never silently serialize or key a transient pose.

## Iconography

Use recognizable visual cues alongside the established Spine controls: folder for
Open, disk for Save, directional history arrows, window/pane icons for Views, and
an icon for each implemented view entry. Use the shared independent vector drawings
in `editor/icons.rs`; do not substitute Unicode/emoji or copy proprietary UI assets.
Reuse a view's icon in its menu and heading. Compact icon buttons need accessible
names, shortcut tooltips and visible enabled/disabled/hover states. Keep text labels
in menus. Preserve command and visibility behavior when replacing a text control.

## Parallel development

The user has authorized coordinated subagent development toward Spine parity.
Continue through tested, committed vertical slices within the active task; a
completed commit is a progress checkpoint, not a request for another "continue".
Choose the next useful gap from docs/PARITY.md and docs/UI_PARITY.md. Ask only for
missing decisions or genuine blockers. Report actual session/Goal limits clearly;
repository instructions alone do not activate automatic continuation across turns.
Use bounded slices in isolated worktrees when parallel work helps; the coordinating
agent owns integration, shared conventions and final native/compatibility checks.
Agree on APIs and ownership before editing shared boundaries. Keep each lane's
Cargo target directory separate: shared workspace package artifacts can be reused
incorrectly across different worktrees despite Cargo's write lock. Rebuild and
verify the combined source after integration rather than treating lane results as
evidence for the merged application. Keep GUI launches/focus under one coordinator.

Native acceptance must verify actual typed values and resulting document state.
Requested focus, highlighted text and a synthesized key event alone do not prove
input reached a field. Keep native OS input confined to the verified task-owned
foreground window, preserve other applications and remove temporary diagnostics.
