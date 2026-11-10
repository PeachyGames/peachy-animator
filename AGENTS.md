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

Native format v6 requires ordered `Document.constraints`, with strict static one-/two-bone
`Constraint::Ik` entries. It retains tagged `Attachment::Region` / `Attachment::Mesh` and key-derived
animation ends. Bone/pose `Transform` requires seven finite fields, including
`shear_x`/`shear_y`; `RegionTransform` retains five fields and rejects shear.
`MeshVertices` distinguishes slot-bone-local positions from per-vertex lists of
bone-local `BoneInfluence` values. The I/O decoder chains v1 region tags, v2
duration capture, v3 mesh-array wrapping and v4 zero-shear insertion into strict
v5 validation, then v5→v6 inserts empty constraints before materializing prior exported holds. Formats 1–5 reject
root constraints even when null or empty. Legacy formats reject
new shear fields/tracks even when zero/empty; current commands require the new
bone payload rather than hiding missing fields with serde defaults. Validate influence references
in every skin, including zero-weight entries and inactive attachments. Preserve
influence order, repeated bones, zero entries and positive nonunit totals;
imports/exports must not silently normalize, merge, sort or prune them.

Native v8 adds `Constraint::Physics` (bone, influences, Scale Y mode, limit, FPS,
seven keyable settings and Global flags) and required per-animation `physics` /
`physics_resets` lists; v7→v8 inserts empty lists, and formats 1–7 refuse physics
data. Physics is data only: evaluation must equal the runtime's `Physics.none`
until a calibrated simulation slice lands. Keep observed Spine defaults
(inertia 0.5, damping 0.85) in `peachy_document::PHYSICS_DEFAULT_SETUP`; export
writes every field explicitly. Never let an IK command replace a physics
constraint or the reverse.

Native v10 adds required `Bone.inherit` and `Animation.inherit_tracks` (after
native9 IK tracks). Inheritance math lives in `engine::pose::inherit_linear`;
the normal mode keeps the original f32 composition. Keyed modes travel in
`AuthoredPose.inherits`; never evaluate Animate poses with Setup modes. IK
chain bones must inherit normally. See docs/slices/inherit-modes.md.

Native v11 adds `Constraint::Transform` with complete 4.3 data; only the
`TransformConstraint::supported_subset` (world; identity Rotate; X/Y with
offsets, scale, clamp) evaluates.
Constraints run in list order inside `ik_solver::apply`; transform results are
written back as applied locals (`transform_constraint::local_from_world`). IK
chain ancestors may be transformed; constrained IK bones still need unit axes.

Native v12 adds `Document.skin_scope`. Evaluate through `AuthoredPose::with_skin`
(Session holds the active skin as view state; the shell syncs it every frame).
Inactive constraints never run; inactive bones' slots are not drawn.

Native v13 adds `Animation.deform_tracks` with dense offsets (per vertex, or per
weighted influence). Sampled deforms travel in `AuthoredPose`; scenes apply them
via `deformed_mesh_world_vertices`. Keep slot renames updating deform refs.

Native v18 adds `Mesh.link`: a linked mesh stores a copy of its source's
geometry. `apply_command` propagates source edits (`propagate_linked_geometry`)
and validation refuses diverging copies; resolve deform keys through
`Document::deform_target`.

Native v17 adds `Region.color`/`Mesh.color` (draws use slot × attachment color)
and `Curve::BezierOffset` (value handles as offsets; sample values with
`curve_value`, not `curve_progress`). Bezier handles only need to be finite.
Data sliders must use `SliderClamping::Edits` so viewing never rewrites data.

Native v16 adds `Animation.draw_order` (Spine offsets as written; scenes draw
in `EvaluatedPose.draw_order`, never slot-list order, in Animate).

Native v15 adds `Document.events`, `Document.reference_scale` and
`Animation.events` (data only; key overrides stay `Option`). Constraint names
are unique per type, not globally.

Native v14 adds `Animation.attachment_tracks` / `color_tracks`. Keyed slot
attachments and colors travel in `AuthoredPose` (`slot_attachments`,
`slot_colors`); scenes resolve and tint from them, never from Setup in Animate.

IK evaluation uses one checked engine solve. `EvaluatedPose` distinguishes authored
locals, applied locals and world transforms; `Session::local_pose` stays authored.
Rendering, picking, bounds and tools use the evaluated frame. The old evaluation
names are fallible aliases; `world_pose` is explicitly forward kinematics only.
Binding captures applied locals once and reconstructs FK, never applying IK twice.
Viewport deltas use `drag_transform_in_frame` and update authored values without
baking solved transforms. Preserve raw Setup rotation winding on directly constrained
bones, including zero Mix. Native6 Mix is [0,1]; unsupported chains/poses fail explicitly
under the approved `docs/slices/ik-design.md` rules and absolute 0.001 world residual.
Append-only core/CLI constraint creation is supported. Creation UI needs the reference
selection workflow; deletion/reordering require identity/remapping contracts. IK keys
and advanced settings need explicit models. Do not approximate them or add a UI-only solver.

Mesh topology, UVs, binding data and weighted geometry belong in document/engine.
`evaluate_scene` emits shared vertex/UV/index buffers for rendering, bounds and CLI
diagnostics. Mesh commands address an explicit skin/slot/placeholder; resolve default
fallback before building a command. Existing unweighted vertex indices remain stable
under centroid subdivision. Weighted meshes expose influence inspection and
explicit weight commands; the shell and engine reject slot-local vertex edits and
subdivision on them. Initial one-bone Bind and Remove-all use `Session::bind_mesh_to_bone` and
`Session::unbind_mesh` with an explicit Setup/Animate target. The engine captures
posed locals, inverts only the required target/slot bone and rejects nonfinite,
singular or greater-than-0.001 forward world-coordinate residuals atomically.
`AddMeshBoneBinding` appends a genuinely new bone at zero weight to every vertex,
preserving all old entries. Reuse the same checked inverse and 0.001 residual
guard; even a zero contribution must evaluate finitely. `SetMeshBoneWeight`
changes one unambiguous vertex/bone and proportionally redistributes its other
positive weights. Only that explicit Direct edit normalizes that vertex; it may
deform geometry and uses no inverse or shape-preservation residual. Zero entries,
positions, order and unselected vertices stay unchanged. Reject repeated bones on
the selected vertex, sparse missing targets, existing-global-bone Add and a
sole-positive reduction without recipients; these are bounded policies, not
observations of every Spine edge case. Do not silently normalize imports, key or
discard pending poses, merge bindings or rebuild geometry in the UI. Brush editing,
locks, multiple selected vertices/bones and Update need separate commands/gates.
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

Weights retains an explicit mesh/vertex/bone context across transform-tool changes;
it must never become an implicit target for a normal bone edit. Reconcile active
skin/default fallback, the slot's active placeholder and stale references before
Bind/Remove-all. Transient Bind mode ends on concrete success/failure, mode or
animation changes, Undo/Redo, selection-history navigation and view hide/restore. A retained
mode after time scrubbing captures the live pose only on the concrete target click.
Opening another document resets the context. Keep genuine Weights tool history
separate from temporary Mesh inspection's return-tool tracking.

Direct previews freeze the explicit mesh/vertex/bone, mode/clip/actual time and one
pose snapshot, then use Session's fixed transaction baseline. Completion is one
undo step after frame actions and pointer release. Escape, application focus loss,
tool/context/history/Open/Save/Export/view-hide actions cancel before continuing;
block stale numeric responses and a canceled held pointer until release. Text undo
retains priority; owned Weight-field Save cancels then saves. Rounded display-only
focus/blur and invalid numeric text alone must preserve raw values and redo. Pending poses
are preserved by weight edits and excluded from saves. Keep these lifecycle rules
in the shell, weight arithmetic and validation in the engine.

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
