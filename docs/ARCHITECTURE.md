# Architecture

The reference model is PhotoCraft's engine-first architecture. This project keeps
its document, commands, deterministic evaluation and format facts separate from
platform effects, with seven crates sized around implemented responsibilities.

```text
Desktop / CLI / MCP -> engine -> document
Desktop / CLI / MCP -> io -> spine -> document
CLI / MCP -> spine (capability inspection)
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

Evaluation samples authored state with the selected skin before pending overlays,
then uses one ordered effective state for bone/inheritance, constraint and slot
domains. Inactive host slot/Slider tracks skip before explicit pending overrides.
Slider samples referenced tracks into that state without recursively solving a
second skeleton. `checked_scene_from_evaluated` consumes the resulting checked
pose, so geometry/readback do not repeat the constraint solve. Authored locals
and Slider Time/Mix remain separate from their applied values.
`evaluate_scene` resolves skins and returns
world transforms, textured vertex/UV/index buffers in draw order and their common bounds. Its optional
preview remains available to pure callers; the desktop now uses cancellable command
transactions for both bone and region pointer gestures. Rendering and fit-to-view use
those same vertices; frontends never rebuild attachment geometry. The external 4.3
curve observations are recorded in PROVENANCE.md.

Native v4 `MeshVertices` distinguishes `SlotLocal { positions }` from
`Weighted { influences }`. Each weighted vertex stores an ordered list of
`BoneInfluence { bone, position, weight }`; positions are local to the referenced
bone. UVs use a top-left origin. All influence references are validated against
the document, including inactive skins and zero-weight entries. Finite weights
must be in 0–1 with a positive total; raw nonunit totals, order and repeated bones
remain valid and are preserved.

`mesh_world_vertices` applies the slot bone to unweighted positions. For weighted
vertices it sums `weight * world[bone].point(bind_position)` for every influence,
without applying the slot transform again or renormalizing. `scene_from_locals`
feeds both keyed and transient posed bones through this shared evaluator. Rendering,
triangle picking, fit/bounds and CLI output consume its `AttachmentDraw` vertices.

Region conversion bakes its attachment transform into four slot-local vertices.
Unweighted subdivision inserts a triangle centroid and interpolates its UV while
preserving the hull and existing indices. Both operations and setup vertex movement
are shared serializable commands. Weighted `SetMeshVertex` and subdivision fail
atomically, preserving pending pose/history/redo. Initial Bind and Remove-all use
the shared commands described below. Additional zero bindings and single-vertex
Direct use separate explicit commands. Brushes and Update remain pending. The
desktop shows influence values and posed handles without implementing weighted
vertex-position editing or inverse-bind math in the UI.

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
The decoder chains v1 region tagging, v2 legacy duration capture and v3 mesh-array
wrapping into native v4. It rejects object-shaped legacy mesh vertices and raw-array
v4 vertices. Complete strict schema, geometry, curve/order and legacy time-bound
validation happens before v2 hold normalization. Native v3/v4 stale duration fields
remain errors; migration does not use the Spine adapter or weaken unknown-field
validation.

Native v23 adds strict Slider constraints and per-animation `slider_tracks`.
The decoder migrates v22 by adding empty lists in memory; legacy payloads reject
injected new fields. Frozen example bytes remain unchanged. A test-only pure
fixture builder upgrades known historical payloads for engine/adapter tests;
production migration stays in I/O and the engine still depends only on document.

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

1. Extend explicit Bind/Remove-all/Add/Direct with Update, brush and deform
   commands; retain imported values until an accepted edit changes them.
2. Introduce stable entity IDs before deletion/reparenting and animation merging.
3. Extend the shared I/O shell for formats/destinations; keep dialogs in the frontend.
4. Extend MCP's discoverable schemas around the existing serializable commands;
   keep transport, filesystem confinement and client state in the MCP shell.
5. Measure snapshot memory with real mesh/keyframe workloads before adding structural sharing.

The app is native-only today. Core crates have no OS/UI dependencies, but WebAssembly
compilation, a web frontend and additional platform builds have not been verified.

## Headless MCP client

`apps/mcp` is a separate stdio client of the same document, Session, checked scene
evaluator and shared file services. The official Rust MCP SDK owns protocol
negotiation, tool discovery and transport. Tool schemas and file access policy stay
in that shell; the core has no MCP or schema-library dependency.

Each process owns one independent editing session and an explicit filesystem root.
Revision checks prevent edits based on an outdated observation. Commands are
dispatched as one atomic batch with the engine's existing undo/redo history; loading
another project must explicitly account for unsaved changes. New-output save/export
operations preserve source projects, and referenced image files must also remain
inside the configured root. See [the interface and its limits](MCP.md).

This headless process does not control an already-open desktop window. A future
desktop bridge needs explicit session ownership and conflict rules. Evaluated
geometry is available without a window; image previews and GUI screenshot tools
remain separate work.

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

## Transform tools

`engine/tools.rs` owns coordinate display/conversion, axis directions, transform
gestures and relative numeric parsing. UI code supplies world points and an explicit
axis lock; there are no screen, pointer or egui types in this module. The desktop
snapshots the parent frame and pressed handle when a gesture starts. World edits
convert through the actual parent affine transform, including reflection and
nonuniform scale, then go through Session's existing editing/keying boundary.

`editor/tools.rs` is the shell adapter. Axis choice is transient view state. Numeric
entry commits on focus loss/Enter rather than dispatching intermediate keystrokes.
World scale is presentation data; the value passed for editing remains local.

## Selection navigation

`editor/selection_history.rs` is a pure, bounded presentation model for typed previous/
next selections and ten single-item groups. It has no egui, filesystem or document
mutation calls. The active `Option<TreeItem>` remains authoritative; navigation
restores it through the same selection effects path. Invalid targets are pruned after
document changes, and groups/history reset when opening another document. They do
not enter Session, document history, project saves or exports. Keyboard focus prevents
number entry from invoking group shortcuts.

## Animation transport

`engine/playback.rs` owns deterministic forward/reverse stepping, bounded loop
wrapping and nearest/last-key queries. The desktop supplies elapsed time, duration
and direction, and applies the returned playhead/playing state. The engine reads no
clock and mutates no document. Transport buttons and hotkeys share the same shell
transition; manual key navigation goes through `set_time`, preserving transient pose
discard and Undo recovery. Playback state is not serialized into runtime exports.

## Interface icons

`editor/icons.rs` owns independent vector drawings and the small reusable controls
that place them. egui button atoms reserve their layout rectangles, preserving full
button hit areas, focus and disabled behavior. File/history buttons have explicit
accessible names and shortcut tooltips; view visibility rows expose checkbox state.
The same view icon is used in its menu and heading. This is presentation-only code:
no document, history, file-service, adapter or engine dependency changes are involved.

## Naming and timeline selection

Bone and slot renaming are pure commands; slot renaming moves the slot's map entry
in every skin while preserving indices, draw order, attachment names and image paths.
Animation renaming preserves its index; duplication appends a fully owned clip copy.
Validation and Session provide atomic failure and a single undo entry. No schema or
format version changes are involved. Stable IDs are still required before general
entity deletion or reparenting.

The desktop shares one name-dialog renderer with explicit pending targets and
transient draft/error state. Confirmation dispatches a command; cancellation has no
document effect. Global editing shortcuts do not run behind the modal. The Tree and
selection history reconcile name-based references after a rename.

`dopesheet_selection.rs` is a pure presentation model, separate from playhead and
bone selection. It identifies supported keys by animation, bone, property and exact
source time. Context changes clear it and document changes prune missing keys.
`dopesheet.rs` owns layout, hit testing and focus, and dispatches a shared Batch of
DeleteKey commands. Exact source matches take precedence over the command's
single-match time tolerance; ambiguous near matches fail atomically. No screen or
platform types enter the engine.

## Key retiming and clipboard

`engine/timeline.rs` owns exact key references, simultaneous moves/copies, relative
clipboard payloads and paste transitions. Sources are captured before any removals;
exact destination keys are replaced while close neighbors remain separate. Every
transition is a serializable command through the existing pure validation boundary.
The engine uses no OS clipboard, event, clock or file APIs.

`engine/timeline_inventory.rs` provides authored inventory and exact keys for
explicit typed owners, shared by the desktop and revision-checked MCP
`get_animation_keys`. Empty owners request inventory only. The full inventory
and scoped result expose unsupported timeline descriptors; these are not editable
IDs. Queries neither solve nor depend on skin, visibility, pending poses or UI
selection. Canonical linked Deform aliases cannot duplicate an owner.

`Session::preview_transaction` recomputes from a fixed document/pose baseline.
Moving away from an overwritten key restores it during preview. Cancellation keeps
the existing redo branch; commit creates one history entry. The desktop captures
press-time source keys, modifiers and chart geometry, and cancels a key preview
before context changes or Save/Export/Open. Completed Undo/Redo clears key selection
so a restored destination cannot inherit a moved key's time-based selection.

`engine/group_clipboard.rs` defines portable group entries and strict new nested
decoders; `timeline.rs` shares existing leaf arithmetic for one global origin and
complete explicit target maps. Preflight covers all entries before mutation and
canonical target aliases cannot collide. Copy's `sourceOwners` receipt stays
separate from portable data. Existing single-owner codecs/commands are unchanged.

`timeline_clipboard.rs` is the imperative clipboard shell. Its session-owned buffer
contains key data, never assets or file paths. Explicit successful Copy/Cut mirrors a
versioned JSON envelope through egui's native clipboard output. Dedicated clipboard
input events take precedence over raw-key fallbacks and only run for chart focus.
Keyboard Paste requires the current envelope; text fields keep their input. The
system clipboard is not an engine dependency or native-project field.
The new editor-local grouped path uses a version 3 OS envelope and separate
original-owner bindings; leaf version 2 remains unchanged. Paste keeps existing
selection and records accepted IDs as diagnostics, not automatic selection.
Final checks05/06 and bounded grouped/leaf-v2 native acceptance pass, including
the unchanged-binary minimum-window and Save/fresh-process follow-up. Typed
Document recovery is exact; diagnostic/file decimal spelling is not raw-byte
equality. The [native result](../examples/peach-bot/group-clipboard-native-result.peachy)
is distinct from the MCP clip1 case. Owned processes are closed. General Tree
multi-target mapping remains absent; the later leaf receipt contract follows.
Actual MCP run02 and separate 208-pose runtime geometry checks validate the shared
data path; they do not close native input acceptance. The exact pointer regression
passes after a test repaint barrier refreshes post-Undo geometry.

`timeline_leaf_clipboard.rs` adds a process-local canonical `LeafSourceReceipt`
bound to exact owned payload text and copy-time animation diagnostics. The v2
portable envelope is unchanged. Bone keys use explicit Bone/active Slot or true
Region parent-Bone targets; otherwise no-owner/Animation fallback uses the owned
source. Constraint effects remap/tombstone the receipt, and successful Open/New
clears it. Foreign OS text cannot recreate ownership; failed operations preserve
the prior valid clipboard/state. Source animation does not dictate destination clip.
Final checks04 and five native routing cases pass; native Undo and same-case Save
byte equality to MCP5d432/fresh exact typed reopen pass, while MCP/headless checks
cover Undo/Redo and inactive/pending/remap guards. No native Redo or new runtime
check is claimed. Slot global DrawOrder presentation and broader Region controls/
styling remain gaps. See [leaf scope](slices/leaf-clipboard.md) and
[scalar evidence](evidence/leaf-clipboard.json).


The pure `KeySelection` model keeps an explicit Rows/Shown expansion phase separate
from key count. It reconciles exact identities before expanding and excludes keys
outside the shown owner set/rows/clip. With no selected editable Tree owner,
bounded all-owner rows use the shared authored inventory; a typed selection
retains its explicit primary owner for Graph dispatch. Read-only descriptors and
inaccessible Deform owners cannot silently join an editable group. The desktop's
ordered action stream combines Select All, clipboard and deletion. A release-aware input latch suppresses
held repeats; passive observation after shortcut routing handles releases while a
text field/modal owns input without consuming that control's events. Selection
expansion never dispatches a document command or moves the playhead.
Bounded two-bone scalar native acceptance verifies selection/retiming, one-step
history, held cancellation, primary-owner Graph routing, minimum layout and
Save/reopen. Cancellation preserves the exact selected set/primary and redo,
blocking continued held movement until release. The native pending baseline is
empty; nonempty pending preservation has headless coverage only. At that earlier
checkpoint mixed Copy explicitly refused. The grouped continuation above extends
that boundary; Shift/Offset, Lock/Sync and general Tree multi-selection remain
separate capabilities.


## Key-derived duration

`Animation::duration()`, introduced in native v3 and retained in v4, reads the last
key of each validated ordered track and returns their maximum or zero. There is no stored cache to update during edits,
Undo or baseline previews. SetKey/SetPose and transient authoring accept finite
nonnegative times beyond that end. DeleteKey is a source query and retains its
upper-bound guard to prevent a near-match query beyond the end deleting a last key.
The compatible AddAnimation duration argument becomes an explicit visible hold.

The I/O decoder chains v1 region tagging into strict v2 duration migration. It
validates the complete schema, curves/order and legacy time bounds before adding
the prior exporter's hold keys. It does not use an export/import round trip or
weaken target-independent native validation. Source bytes stay unchanged until
explicit Save; the existing verified backup/atomic replacement path is reused.

The desktop separates `animation_end` from a positive `display_extent`, which
includes the cursor. Fixed drag geometry uses the latter; playback uses the former.
Frame and screen/time arithmetic use wider intermediates to avoid overflow for
finite large cursors. Empty clips do not enter division/remainder by zero. The
external oracle independently checks exported runtime ends, including zero, and
samples held poses after the end as well as ordinary and backward sample order.

## Initial Bind and Remove-all

`BindMeshToBone` and `UnbindMesh` are serializable pure transitions carrying an
explicit `MeshBindPose` of local bone transforms. Initial Bind accepts a slot-local
mesh and produces one weight-1 influence per vertex. Remove-all evaluates every
existing influence, including repeats and nonunit totals, then stores the current
vertices in slot-bone-local space. UVs, topology, hull, image metadata and bone
transforms do not change. Later animation behavior follows the new bindings; shape
preservation applies at the captured pose, not every former animation pose.

The engine inverts only the bone required by the operation: the target bone for
Bind or the slot bone for Remove-all. Wider intermediates avoid avoidable inversion
loss. A zero determinant, nonfinite result or forward world-coordinate residual
above 0.001 rejects the candidate atomically. The guard does not normalize weights
or reject unrelated zero-scale bones merely because they do not have an inverse.

`Session::bind_mesh_to_bone` and `Session::unbind_mesh` build that snapshot from an
explicit Setup/Animate target, including pending posed locals. They preserve the
transient pose and use the existing document-plus-pose history. The shell cancels
gesture previews, finalizes prior numeric work and pauses playback before a concrete
edit. It implements no binding geometry and does not key or discard the pose.

`editor/weights.rs` owns pure retained selection and the imperative view/input shell.
Its mesh/vertex/bone context survives normal bone posing, while normal transforms
still use `TreeItem::Bone`. Binding requires a concrete target click after entering
mode. Active-skin fallback and actual slot-placeholder resolution guard the explicit
attachment reference; invalid contexts clear rather than redirect. Undo/Redo,
selection-history navigation, mode/animation changes and hiding the view cancel transient Bind
mode. Scrubbing may retain it and uses the pose at the subsequent target click.

The real G tool participates in ordinary last-two-tool switching. Temporary Mesh
inspection keeps its independent return tracking. Weights is a named resizable
right-column view with shared independent icon cues and visibility state. This is
provisional placement, not an implemented arbitrary docking system.

## Additional bindings and Direct weights

Native v4 is unchanged. `AddMeshBoneBinding` and `SetMeshBoneWeight` carry an exact
attachment reference and `MeshBindPose`. Thin Session methods capture Setup locals
or actual Animate locals, including pending edits at the supplied time. Pose
snapshots are command inputs, never saved mesh fields. Neither operation keys or
discards pending bone edits.

Add requires a new bone absent from every vertex, including zero entries. It
evaluates the current weighted point, uses the existing checked inverse for the
new bone and appends its bone-local position at weight zero. Every old entry stays
unchanged, including signed zeros, repeated bones, nonunit sums and order. Only
the new inverse is required. The shared finite and absolute 0.001 forward-residual
checks also apply to a zero entry: transforming it before multiplication must not
produce infinity or NaN. The initialized position is our approved mathematical
policy; the public zero-weight convention does not establish Spine's exact storage
choice. Add does not repair a bone present on only some vertices.

Direct addresses one vertex and one existing bone. For requested f32 weight `t`
and f64 sum `O` of the other strictly positive weights, keep the chosen value `t`
and store each other positive value as `f32(f64(old) * (1 - f64(t)) / O)`.
Existing zero entries and all bind positions/order are untouched. There is no
second normalization or residual assignment after f32 rounding. An explicit edit
can normalize an imported nonunit selected vertex even if its chosen value was
unchanged. Import, display, Add and unselected vertices do not normalize anything.

When `O` is zero, only `t == 1` is allowed. Repeated bones anywhere on the selected
vertex are ambiguous and reject; repeats elsewhere remain valid and untouched.
Missing selected-vertex targets and out-of-range/nonfinite payloads reject. These
are bounded authoring policies, not a relaxation of native data preservation or a
claim that Spine rejects the same inputs. A genuinely unchanged result retains
the existing Session no-op behavior and redo branch.

Direct uses forward evaluation to reject a newly nonfinite selected point at the
captured pose. It requires no inverse, so a singular transform alone need not
reject. It intentionally permits deformation and never runs Add's 0.001 shape
residual guard or compensates positions. Atomic candidate validation and Batch
publication preserve document, pending pose, saved state and history on failure.

The Weight control freezes attachment, vertex, bone, Setup/Animate, animation,
actual time and one pose snapshot. Every preview starts from Session's transaction
baseline, avoiding cumulative redistribution. A failed preview retains the last
valid preview. Pointer release, Enter/Tab and ordinary blur settle one undo step
after frame actions and pointer-up; a context or Save action can cancel first.
Escape, application focus loss, tool/selection/context/history/Open/Save/Export
and view-hide actions restore the baseline and suppress stale field responses or
held-pointer movement until release. Numeric text owns text undo; owned-field
Save cancels then saves. No new history stack or preview math lives in the UI.

See the [approved command contract](slices/weight-direct-design.md). Multi-vertex/
multi-bone Direct, brushes, locks, individual removal, sparse repair and Update
remain separate model, interaction and acceptance work.


## Bone shear, native v5 and checked evaluation

`Transform` is the seven-field bone/pose value. `RegionTransform` preserves the
five-field attachment boundary; promotion inserts zero shear and checked demotion
rejects it. Shear X/Y are scalar setup-relative angular properties with paired
interactive keying. Affine evaluation uses independently observed angular basis
math, preserving authored angles rather than decomposing matrices. Engine tool
math owns checked inverse directions, World rotation and oblique Scale coordinates.

The adapter owns absolute/normalized curve conversion and target range errors.
The engine keeps the ten-segment curve policy and widens overflowing interpolation
intermediates. `checked_scene_from_locals` and `checked_evaluate_scene` return an
error before shells serialize, draw or pick nonfinite data; the original evaluation
names now delegate through fallible aliases. Shells use the checked
boundary. Bounds centers likewise widen only
when the ordinary intermediate overflows. All these functions remain pure.

Desktop numeric and viewport transforms share a frozen gesture capsule. It retains
typed selection, initial pose/parent, tool/axes, mode/clip/time, Auto Key and edited
properties. Bone previews use `Session::preview_edit_pose`; region previews use a
checked command. Direct and transform input cancel each other before capture.
Pointer-up and frame settlement produce one history step; context/history/file
actions cancel and suppress stale blur/held-pointer input. File operations remain
in the shell. Toolbar and seven-channel chart sizes derive from shared style.

## Static IK and native v6

The document owns ordered, tagged `Constraint::Ik` data and validates references;
the engine owns the bounded solver and command validation. The adapter recognizes
4.3's unified setup `constraints` array and translates names to native references.
Native v6 requires the list. The decoder rejects it in formats 1–5 before adding an
empty list during migration; no serde default conceals missing current data.

`EvaluatedPose` encapsulates authored locals, applied locals and solved world
transforms. `Session::local_pose` remains authored; checked evaluation builds one
applied frame for geometry, picking, bounds and presentation. Constraint changes
use the ordinary pure command boundary and Session transactions. Unsupported
dynamic poses return explicit errors without rewriting valid stored data.

The independent solver widens only relevant FK/IK inputs and checks final f32
origins/tips against the absolute 0.001 world residual. It preserves observed raw
rotation winding and performs one angular correction rather than modulo reduction.
`drag_transform_in_frame` derives gestures from displayed applied coordinates and
adds their delta to authored values. No client bakes the solved pose into keys.

Binding snapshots applied locals once, then reconstructs FK from that snapshot;
it never reruns IK over an already solved chain. Pending bone edits, saved-state
comparison and history retain their existing Session ownership. Constraint Mix,
Positive, Rename and target replacement reuse transactions, typed Tree selection
and the shared name dialog. Static settings are read-only in Animate. Animated
settings need an explicit pose/track model before extending these boundaries.
