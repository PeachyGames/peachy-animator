# Agent access through MCP

Peachy Animator provides a local **stdio MCP server** named `peachy-mcp`. Agents
can open, inspect, edit, undo, evaluate and save animations without starting the
native window. The server calls the same document, command, Session, checked pose
evaluation and file services as the desktop. It uses the official
[Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk), pinned by Cargo.lock.

Each server process owns an independent headless document and history. It does
**not** attach to a running desktop editor, change its selection, refresh its open
document or consume that window's unsaved changes. Saved projects can be opened in
the native editor. A live desktop bridge and raster PNG previews remain future
work; `evaluate_pose` currently returns checked geometry and transforms.

## Start and connect

Build from the repository root:

```sh
cargo build -p peachy-mcp --locked -j 2
```

Configure an MCP client to launch the resulting executable with an explicit,
absolute workspace directory:

```json
{
  "mcpServers": {
    "peachy-animator": {
      "command": "/absolute/path/peachy-animator/target/debug/peachy-mcp",
      "args": ["--root", "/absolute/path/animation-workspace"]
    }
  }
}
```

The `mcpServers` wrapper is a common client configuration shape; clients with a
different configuration format use the same executable and argument list. Supply
the actual binary location if using `CARGO_TARGET_DIR`. No HTTP port, browser,
display, network connection or API key is required at runtime. Standard output is
reserved for MCP messages; startup errors and help go to standard error.

`--root` is required and must be an existing directory. Use a directory containing
the projects and loose images the agent should work with, plus a place for fresh
outputs. Relative tool paths resolve against this directory, independent of the
client's working directory. Absolute paths are accepted only inside that root.

## Tools

| Tool | Behavior |
| --- | --- |
| `get_document` | Complete native document, current revision, dirty flag, undo/redo availability, source and animation/skin inventory. Before Open it returns revision 0 and no document. |
| `create_project` | New in-memory root-only native document with an empty default skin. Uses an existing asset directory within `--root`; no finished rig is loaded and no file is written. Source is null until Save. |
| `inspect_file` | Read-only native validation or Spine capability report. Does not replace the working document. |
| `open_project` | Load `.peachy` (including supported legacy migrations) or supported Spine 4.3 JSON into a new Session. Failed imports leave the current session intact. |
| `get_command_schema` | Full JSON Schema for the 53 supported flat command variants; the same schema appears inline in `apply_commands` discovery. |
| `apply_commands` | Apply a nonempty flat list through one engine Batch, atomically and as one undo step. No file writes. |
| `copy_animation_keys` | Read-only capture of exact keys from one bone, IK constraint, Deform attachment, slot or global draw order. Slot clipboards can mix attachment and color keys with one relative origin; Deform includes vertex/influence layout. Draw order requires the identical ordered slot-name layout on paste. Requires the current revision. |
| `get_mesh_weights` | Read exact selected weights and a uniform/mixed aggregate through the engine; absent influences of an already bound bone read zero. Requires the current revision and exact attachment identity. |
| `get_transform_mapping` | Read raw-derived Min/Max endpoints of one exact world X→Rotate constraint mapping at the current revision. Undefined SourceMax is null; querying never repairs stored coefficients or changes history. |
| `history` | Undo or redo one complete edit using the shared Session history. |
| `evaluate_pose` | Setup or a named animation at a nonnegative time in seconds, with optional skin. Returns authored locals, IK-applied locals, per-bone `inherits` modes, world matrices, raw `slotAttachments`/`slotColors`, complete back-to-front `drawOrder` including hidden/empty slots, textured vertex/UV/index buffers and bounds. |
| `save_project` | Create a new directory containing `project.peachy` and the required loose `images/`; marks the headless Session saved. |
| `export_project` | New Spine JSON file, or a fresh packed JSON/atlas/PNG directory inside an existing parent. Optional `target` defaults to `spineEditor43`; use `runtime43` to retain independent attachment names/placeholders for runtime consumers. Does not mark the native document saved. |

These 13 tools publish parameter schemas, descriptions and behavior annotations through
MCP `tools/list`. Successful results include both `structuredContent` and matching
serialized text for older clients. Operational failures return `isError: true`
with the current revision; malformed tool arguments use the SDK's protocol error.
Read-only tools do not change session state.

`export_project.target` accepts only `spineEditor43` or `runtime43`. Editor
export rejects default-skin attachment name overrides and named/default
placeholder collisions, plus contradictory weighted bone ordering, before writing
an output. The error identifies the
unsafe attachment. Runtime export preserves valid names, raw influence arrays, placeholder references
and skin fallback without applying those Editor-only restrictions; all other
supported-feature validation remains. Neither target rewrites the native
document or changes revision/history. A runtime export is not an Editor import
guarantee. Successful responses include `exportTarget` and `targetVersion`.

For example, export runtime data from the current revision into a fresh file:

```json
{
  "expected_revision": 1,
  "kind": "json",
  "path": "exports/runtime.json",
  "target": "runtime43"
}
```

The `exports` directory must already exist inside `--root`. The same target
field applies to `kind: "bundle"`. See [the export-target contract](slices/export-targets.md)
for installed-editor observations and the corresponding native/CLI controls.

## Create from artwork

For fresh rig authoring, place the prepared PNGs under an existing asset
directory's `images/` child inside `--root`, then call `create_project`:

```json
{
  "expected_revision": 0,
  "name": "Tutorial recreation",
  "fps": 30,
  "asset_root": "art-input"
}
```

The result is revision 1 with one neutral `root` bone, an empty `default` skin,
and no slots, constraints or animations. FPS defaults to 30 and must be 1–240.
The asset directory must exist and resolve inside the configured workspace;
creation does not copy or alter it. Invalid arguments, an asset-root escape or a
stale revision leave the current document intact. Replacing a dirty headless
document requires explicit `discard_changes:true`, as with Open.

Use `addBone` in parent-before-child order, then `addSlot` and `addAttachment`
through `apply_commands`. New slots start with `attachment:null`. Attachments
require an existing named skin and slot, and an unoccupied placeholder. No
existing attachment is replaced and no skin is created implicitly. Use
`setSlotAttachment` after insertion to activate the placeholder; the whole list
can be one atomic undo step.

`addAttachment.value` accepts native `region` or `mesh` data. Regions have an
image path, source dimensions and region transform. Meshes may be authored with
an arbitrary explicit hull, UVs and triangulation, using either slot-local
positions or per-vertex bone-local influences. This creates new geometry; it
does not reindex an existing animated mesh. The engine validates topology,
influence references, finite values and the rest of the native contract before
publishing the edit. Path/clipping creation and general topology replacement
remain outside these commands.

For example, a region can be added and displayed on the new root:

```json
{
  "expected_revision": 1,
  "commands": [
    {"command":"addSlot","slot":{"name":"body","bone":0,"attachment":null,"color":[255,255,255,255]}},
    {"command":"addAttachment","attachment":{"skin":"default","slot":"body","placeholder":"body"},"value":{"type":"region","path":"body","width":140,"height":148,"transform":{"x":0,"y":0,"rotation":0,"scale_x":1,"scale_y":1}}},
    {"command":"setSlotAttachment","slot":0,"attachment":"body"},
    {"command":"addAnimation","name":"idle","duration":0}
  ]
}
```

This references `art-input/images/body.png`. Add bone keys with `setKey` or
`setPose`, IK through the existing constraint/key commands, and dense mesh keys
with `setDeformKey`. These commands can author motion without opening/importing
a finished project. `evaluate_pose`, Save and Export use the same engine and
I/O boundary as existing-document edits. Prepared PNG artwork is still required;
the MCP server does not separate Photoshop layers or paint missing artwork.

## Editing workflow

First call `get_document` to discover the current revision. A fresh process starts
at 0. For the original fixture with the repository itself as `--root`:

```json
{"path":"examples/peach-bot/peach-bot.peachy","expected_revision":0}
```

Pass that object to `open_project`. A successful Open returns revision 1. Read
`get_document` for current bone, slot and animation indices, then call
`apply_commands`, for example:

```json
{
  "expected_revision": 1,
  "commands": [
    {"command":"renameBone","bone":0,"name":"agent-root"},
    {"command":"setBoneLength","bone":0,"length":42}
  ]
}
```

The result is revision 2. Both edits form one undo step. A later command failure
rolls back the entire list, including any preceding successful command in that
list. Unknown fields and nested `batch` commands are rejected; the server owns
the outer atomic batch.

To evaluate, call `evaluate_pose`:

```json
{"animation":"wave","time":0.3,"skin":"peach"}
```

Omit `animation` for Setup, `time` for zero and `skin` for `default`. Bone-only
documents with no skins are supported. Bind commands require a complete
**applied-local** snapshot from `appliedLocals` together with its sampled
`inherits` array. Use `{"locals": evaluated.appliedLocals, "inherits": evaluated.inherits}`
for the command's `pose`; this captures the checked IK result and animated
inheritance once. Do not substitute authored `locals` for this snapshot.
Legacy poses omitting `inherits` still use document Setup modes. A supplied
array must have one mode per bone; null, unknown modes and wrong lengths fail
without applying the batch. Transform commands
require seven fields: `x`, `y`, `rotation`, `scale_x`, `scale_y`, `shear_x`,
`shear_y`. SetKey values use the native setup-relative convention; scale keys
multiply setup. Shared engine validation and unsupported-feature gates apply.

`slotAttachments` and `slotColors` are indexed by the document's slot list,
including hidden slots. Attachment entries are placeholder names or null.
Colors are finite, unmultiplied float RGBA in normalized channel units, before
attachment tint, scene display conversion or byte rounding; finite Bezier
overshoot may exceed 0–1. Reading them never quantizes or writes a key. Use these
raw channels when keying a sampled slot, rather than a textured draw's tint.

Save with `save_project`:

```json
{"expected_revision":2,"directory":"agent-edits"}
```

The directory must not already exist, and its parent must exist. The result points
to `agent-edits/project.peachy`; copied images make it usable when opened in the
desktop. Save advances revision to 3. Export a packed runtime bundle with:

```json
{"expected_revision":3,"kind":"bundle","path":"."}
```

The response identifies the new timestamped bundle directory. JSON-only export
uses `kind:"json"` and a fresh `.json` output path instead.

## Deform timeline editing

Deform keys now use the same typed timeline identities as bone and IK keys:

```json
{"type":"deform","attachment":{"skin":"default","slot":"hand.R","placeholder":"hand.R"},"time":0.6}
```

Use exact times returned by `get_document`, including fractional values. Pass
identities to `copy_animation_keys` with `expected_revision`, the animation index
and `keys`. Copy preserves revision/history and returns a typed `clipboard` with
relative times. Deform layouts distinguish unweighted vertex counts from weighted
per-vertex influence counts; paste validates compatible offset segmentation.
`pasteAnimationKeys` accepts a Deform target on another compatible attachment
or clip. Linked attachments resolve to their shared timeline owner.

The same identities work with `moveAnimationKeys` (`copy:false` for retiming,
`copy:true` for duplication) and `deleteAnimationKeys`. Those commands preserve
complete dense offsets and curves, and apply atomically. `setDeformCurve` takes
`animation`, `attachment`, exact `time`, and `curve`; it changes interpolation
without replacing the key's geometry. All existing curve shapes remain native
values. Clipboard compatibility is a documented engine policy, not a claim that
every installed-editor weighted binding combination has been verified.

## Shared interval curve actions

`applyCurvePreset` and `editCurveHandle` use the same engine actions as Curves
and Graph. Both take an `animation` and the complete `selection`, an array of
`{key:<typed AnimationKey>,channel:<index>}` entries. Scalar, IK and Deform use
channel 0; slot color uses R/G/B/A indices 0–3. Discrete attachment keys count
toward the selection but have no curve. Keep the complete selection when
editing only its primary curve: filtering it can change final-key resolution.

A single selected final key addresses its preceding interval. Multiple selected
keys skip final keys. An empty effective selection makes no history entry and
preserves redo. Raw setters below instead address the exact supplied key.

`applyCurvePreset` takes `preset` (`"linear"`, `"stepped"`, `"bezier"`, or
`{"custom":[x1,y1,x2,y2]}`) and `separate`. Custom controls use relative
time/value coordinates. Reapplying Bezier resets its handles. Separate controls
whether adjoining explicit Bezier angles affect preset application.

`editCurveHandle` takes `primary` (a resolved left-interval key/channel from
the complete selection), `handle` (0 or 1), `point:[seconds,value]`,
`match_curves`, and `separate`. Values are actual scalar/Mix values, normalized
RGBA, or normalized Deform progress. Match copies the selected handle's relative
position across effective curves and preserves each other handle. A flat
primary has no relative value coordinate, so matched editing refuses it.
All targets are planned atomically; an unsupported or invalid target leaves
the document and history unchanged. Native values, times and pending poses are
preserved. Separate-off Bezier reset matches adjoining explicit Bezier angles,
shortening an excessive handle along its ray within its own endpoint's value
span; it leaves outer curves unchanged. Normal Separate-off live edits adjust
the adjoining tangent and retain its exact horizontal coordinate, within the
documented native comparison tolerance. These are different actions.

Uncalibrated Separate-off adjacent selected intervals, Custom presets with
Bezier neighbors, and live vertical/outside-time, flat-linked or extreme
coupling fail explicitly. Separate-on permits independent finite handle edits.
Matched edits onto flat destinations remain unsupported. User preset storage,
Last chosen preferences and combined native property groups remain open. See
[the measured scope](slices/curves-authoring.md).

```json
{
  "commands": [
    {"command":"applyCurvePreset","animation":0,"selection":[{"key":{"type":"bone","bone":8,"property":"rotation","time":0},"channel":0}],"preset":"bezier","separate":true},
    {"command":"editCurveHandle","animation":0,"selection":[{"key":{"type":"bone","bone":8,"property":"rotation","time":0},"channel":0}],"primary":{"key":{"type":"bone","bone":8,"property":"rotation","time":0},"channel":0},"handle":0,"point":[0.1,6],"match_curves":false,"separate":true}
  ]
}
```

## Scalar and IK curve-only commands

`setKeyCurve` takes `animation`, `bone`, `property`, an exact existing `time`,
and `curve`. It changes only that scalar key's outgoing interpolation, retaining
its raw value and time, sibling properties, pending pose and history semantics.
All seven bone properties and native Linear, Stepped, Bezier and BezierOffset
shapes are accepted. Missing exact identities fail atomically; there is no
nearest-key upsert or implicit final-key redirection.

`setIkCurve` similarly takes `animation`, `constraint`, exact `time` and typed
`IkCurve`. Its Bezier payload retains f64 normalized time handles and absolute
Mix controls; it does not cast them to scalar f32 controls or change Mix/Positive.
Raw terminal curve metadata is explicit in both commands. Interactive interval
selection and preset policies are separate from these exact setters.

```json
{
  "expected_revision": 1,
  "commands": [
    {"command":"setKeyCurve","animation":0,"bone":8,"property":"rotation","time":0.5,"curve":{"bezier":[0.25,0,0.75,1]}},
    {"command":"setIkCurve","animation":0,"constraint":0,"time":0.5,"curve":{"type":"bezier","time":[0.16666666666666669,0.8333333333333333],"mix":[0.25,1.25]}}
  ]
}
```

Both commands use the same atomic command/history boundary as native controls.
The example requires both exact keys to exist; a failed second command restores
the complete batch. Native v22 stays unchanged; the current tool inventory is above.

## Slot attachment and RGBA authoring

Four persistent commands author animation slots through the shared engine:

| Command | Payload and behavior |
| --- | --- |
| `setSlotAttachmentKey` | `animation`, `slot`, `key:{time,attachment}`. Creates or replaces the exact key; null hides the slot. |
| `setSlotColorKey` | `animation`, `slot`, `key:{time,color,curves}`. Stores four RGBA bytes and all four explicit outgoing curves, without splitting neighboring curves. |
| `setSlotColorCurves` | `animation`, `slot`, exact `time`, `curves`. Changes only the existing key's four curves, preserving its bytes and identity; an absent exact key fails. |
| `insertSlotColorKey` | `animation`, `slot`, `time`, `color`. Accepts four precise float RGBA channels, splits an interior interval's curves, then encodes the new key once at native byte precision. An existing exact key retains its curves. |

Evaluation uses Setup before each channel's first key and holds its last key
after the track ends. Attachment swaps are discrete; null is an explicit hide.

The four curve entries are ordered R, G, B, A. Linear and Bezier types can differ
per channel; Stepped must apply to all four. Bezier handles must be finite;
`bezierOffset` retains value offsets for flat endpoints and overshoot. When
changing a value with `setSlotColorKey`, supply the existing four curves to
preserve them. This explicit setter retains the complete supplied payload;
interactive new-key insertion and MCP `insertSlotColorKey` share the separate
curve-splitting command.

For example, after opening `examples/peach-bot/slot-bot.peachy` at revision 1,
create a fade with a hide key and insert its sampled midpoint in one undo step:

```json
{
  "expected_revision": 1,
  "commands": [
    {"command":"setSlotAttachmentKey","animation":1,"slot":8,"key":{"time":0.75,"attachment":null}},
    {"command":"setSlotColorKey","animation":1,"slot":8,"key":{"time":0.25,"color":[255,128,64,255],"curves":[{"bezier":[0.25,0,0.75,1]},"linear","linear","linear"]}},
    {"command":"setSlotColorKey","animation":1,"slot":8,"key":{"time":0.75,"color":[0,128,64,0],"curves":["linear","linear","linear","linear"]}},
    {"command":"insertSlotColorKey","animation":1,"slot":8,"time":0.5,"color":[0.5,0.5019608,0.2509804,0.5]}
  ]
}
```

Commands reject unknown fields. Times are finite nonnegative seconds stored as
f32, not frame numbers. Reuse exact times returned by the document/copy tools;
do not round fractional identities or infer them from a displayed frame. These
persistent commands do not apply the desktop's interactive frame snapping.

A slot owner is `{"type":"slot","slot":8}`. Typed identities distinguish
`slotAttachment` from `slotColor`, including coincident keys. For a mixed copy:

```json
{
  "expected_revision": 2,
  "animation": 1,
  "keys": [
    {"type":"slotColor","slot":8,"time":0.5},
    {"type":"slotAttachment","slot":8,"time":0.75}
  ]
}
```

Pass this to read-only `copy_animation_keys`. Its clipboard has
`type:"slot"`, `attachment_keys` and `color_keys`; both arrays share the earliest
copied time as zero. This example returns color time 0 and attachment time 0.25,
with complete color values/curves. `pasteAnimationKeys` takes that clipboard,
an animation index, a typed slot `target` and a destination `time` in seconds.
The same identities support exact `moveAnimationKeys` and `deleteAnimationKeys`.

Pasting a named attachment checks the target slot's placeholder names across
all skins. An unavailable name skips only that incoming attachment key; hide
keys and valid color keys still paste with the original shared timing origin.
An existing destination key at a skipped identity remains untouched. All-skipped
paste leaves document/history unchanged, though a successful `apply_commands`
still advances the MCP revision. Explicit attachment-key setters and imports
continue to retain nonempty names that do not currently resolve.

Interior Bezier insertion uses an independently derived 300-step absolute-time
scan calibrated against 104 native 4.3.26 channel parameters, including a separate
nonzero-origin holdout. Subdivision retains precise float controls before rebasing
them onto stored byte endpoints. Stepped intervals inherit Stepped. Extreme or
adjacent f32 times that cannot support the scan use stable exact inversion;
that fallback is outside the calibrated native range. Persistent color keys
remain bytes in native v22: explicit keying can incur at most half a byte per
channel, while unrepresentable colors fail atomically. Existing sampling
arithmetic may differ from the installed editor by one encoded byte at
half-byte ties; this is not a claim of lossless subbyte key storage or complete
native UI acceptance.

## Global draw-order authoring

`setDrawOrder` accepts every slot index exactly once, **back to front**, including
hidden slots and slots without attachments. For a four-slot document, this moves
slot 0 to the front at half a second and restores setup order at 1.5 seconds:

```json
{
  "expected_revision": 1,
  "commands": [
    {"command":"setDrawOrder","animation":0,"time":0.5,"order":[1,2,3,0]},
    {"command":"setDrawOrder","animation":0,"time":1.5,"order":[0,1,2,3]}
  ]
}
```

These are persistent stepped keys; the Tree displays the reverse order, with
frontmost slots first. A new identity-order key is an explicit reset/hold,
distinct from no key. `setDrawOrderKey` accepts an exact native key such as
`{"time":0.5,"offsets":[{"slot":0,"offset":3}]}`. Offsets are signed integers;
entries must be in setup-slot order and have distinct in-range destinations.
Valid redundant zero entries survive raw authoring, history, clipboard and
persistence. Semantic replacement of a key with the same order preserves that
key's raw encoding. Newly encoded keys minimize moved entries independently;
exact native tie choices are not claimed.

Copy, move and delete use exact identities `{"type":"drawOrder","time":0.5}`.
Paste targets the singleton `{"type":"drawOrder"}`. Its clipboard carries
relative-time raw keys plus ordered slot names; paste requires the identical
ordered slot-name layout and rejects mismatches atomically. No slot or bone
index belongs on the global owner. Global keys have no interpolated Graph or
Curves channels. Read the complete sampled order from `evaluate_pose.drawOrder`;
visible attachment geometry omits hidden/empty slots.

MCP commands persist keys directly in the independent headless session. Native
pending order and Auto Key use the shared Session boundary but are not a live
desktop connection. Setup slot reordering, draw-order folders and cross-layout
clipboard mapping remain outside this slice.

## History and file boundaries

Create, Open, Apply, Undo, Redo and Save advance a monotonic revision. Each of those tools,
and Export, requires `expected_revision`; a stale value fails without side effects.
Re-read state before retrying. Revisions belong to one server process and reset
when it exits. Concurrent requests serialize around the same Session; independent
server processes do not share documents or history.

Opening or creating another document refuses to discard unsaved edits unless the call
explicitly supplies `discard_changes:true`. Successful Save marks the saved native
document clean; Export preserves the native dirty baseline. Unsupported imports,
invalid edits and failed output operations preserve the Session and revision.

Primary file paths, output parent directories and every loose image used by Save
or bundle export are canonicalized and checked against `--root`. Parent traversal,
symlinks resolving outside the root, existing outputs and dangling output symlinks
are rejected. JSON-only export does not read images. Loading and inspecting a
project do not require all images to exist; operations that need images fail
explicitly if any are missing or outside the root. Artist inputs are never
overwritten. Save creates its own fresh directory, copies through `peachy-io`, and
removes that owned directory if saving fails. No arbitrary code or shell tool is
exposed.

These are application-level checks for a cooperating local filesystem, not an OS
sandbox against another process changing filesystem paths during an operation.
Keep independent jobs in separate output directories. The server does not publish
games, issue ownership keys, modify account settings or broaden the currently
supported Spine feature subset.

## Verification

```sh
cargo test -p peachy-mcp --locked -j 2 stdio -- --test-threads=2
cargo test -p peachy-mcp --locked -j 2 schema -- --test-threads=2
cargo clippy -p peachy-mcp --all-targets --locked -- -D warnings
python3 scripts/check-architecture.py
```

The Rust protocol tests launch the actual binary and connect with the official SDK
client over stdio. They cover initialization/discovery, inline command schemas,
matching structured/text results, atomic batch failure, stale revisions, dirty
Open refusal, undo/redo, evaluated geometry, native/Spine import failures, fresh
Save with image copying, JSON and packed exports, source-byte preservation, and
primary/output/image symlink escapes. A schema test decodes and reserializes every
published flat command shape using strict engine serde. Exact native GUI behavior
and external runtime compatibility remain covered by their separate project
acceptance checks; MCP does not establish additional Spine feature parity.

Slot protocol coverage includes raw hidden-slot float evaluation, mixed
attachment/color copy and paste, exact value/curve commands, curve-preserving
insertion, undo/redo, failed-batch rollback and Save/reopen. Shared engine tests
also verify skipped-name collisions cannot select or delete untouched keys.

The fresh-creation protocol case starts with the actual `create_project` tool,
authors a region and a weighted triangulated mesh, adds bone/deform animation,
checks failed-batch rollback and undo/redo, saves with unchanged image copies,
exports JSON and an atlas bundle, and reopens the saved native project. It loads
no finished rig as its starting document. This is a creation capability check;
recreating a particular tutorial remains a separate artifact/visual acceptance
task.


## Multi-vertex Direct weights

`get_mesh_weights` reads one explicit attachment and bound bone at the current
revision. For example:

```json
{"expected_revision":17,"skin":"default","slot":"dense_mesh","placeholder":"dense_mesh","vertices":[3,2],"bone":1}
```

The response sorts vertex indices and returns raw `weights` plus
`value: {"kind":"mixed"}` or `value: {"kind":"uniform","weight":0.5}`.
It never averages, normalizes or edits data. An absent contribution of a bone
already bound elsewhere on the mesh reads zero. Empty/duplicate selections,
unknown identities, repeated ambiguous influences and stale revisions fail.

`editMeshBoneWeights` takes `attachment`, `vertices`, `bone`, `pose` and `edit`.
Capture **both** `appliedLocals` and `inherits` from one `evaluate_pose` result:
pass them as `pose.locals` and `pose.inherits`. The new `edit` object is one of:

```json
{"mode":"absolute","weight":0.5}
```

```json
{"mode":"relative","deltas":[2.0,-0.14285715]}
```

Weights and deltas use 0–1 weight units. Ordered deltas describe **one gesture**:
each step clamps each vertex's target while redistribution uses the original
positive recipient ratios. Thus clamping to one and reversing can restore those
ratios within the gesture. A later command starts from committed data. Absolute
entry assigns the same value to every eligible vertex; sole-positive vertices
without recipients stay unchanged while the others can change. Both forms are
one atomic undo step and preserve bind positions and unselected vertices.

For a truly absent influence of an already bound bone, a positive edit appends
a `(0,0)` bone-local entry in the mesh’s existing binding order, matching the
measured Setup/Animate origin policy.
Existing entries retain their relative order and positions. Sparse insertion uses
an independent topological extension of mesh bone sequences, with first-appearance
ties; repeated or contradictory ordering rejects positive creation atomically.
Existing zero entries retain their positions. Zero/no-change relative input
preserves raw absence and nonunit data. Creating a missing influence on a mesh
with keyed or pending Deform data is rejected until offset-layout remapping is
implemented; explicit existing entries can still be edited. Ambiguous repeated
bones and nonfinite resulting geometry fail atomically. The strict legacy
`setMeshBoneWeight` retains its previous sparse/sole-recipient rejection policy.

Legacy pose payloads that omit `inherits` retain Setup modes. A supplied array
must have one valid mode per bone; `null` is rejected. Native captures and new
MCP recipes include the array so animated inheritance is retained when binding.
See [the measured contract](slices/weight-multivertex.md) for native-reference
calibration, differences and verification limits.

## World X to Rotate mapping

Native v22 is unchanged. The current schema has **53 commands / 13 tools**.
An existing world-space X→Rotate transform constraint can be edited by the same
semantic range operation as the native Min/Max controls:

```json
{"command":"editTransformMappingRange","constraint":0,"from":"x","to":"rotate","field":"sourceMax","value":100}
```

`field` is exactly `sourceMin`, `sourceMax`, `targetMin` or `targetMax`; `value`
is a finite f64 endpoint with checked f32 storage. The operation retains the
other endpoints subject to stored coefficient precision, repairs an edited equal span by setting Max to Min+100, and
retains reversed ranges. Put ordered field edits in one `apply_commands` list
for one atomic history entry. Existing raw `addTransformConstraint` and
`setTransformConstraint` still expose complete native data.

Call `get_transform_mapping` with:

```json
{"expected_revision":1,"constraint":0,"from":"x","to":"rotate"}
```

The response includes `revision`, exact mapping identity and
`range: {sourceMin, sourceMax, targetMin, targetMax}`. Endpoints are derived from
stored offsets, max and scale without rounding or mutation. A zero raw scale
returns `sourceMax: null`; known-field no-ops preserve that data. An explicit
SourceMax edit can resolve it when the target span is nonzero. Other unresolved
edits, collapsed/nonrepresentable storage and ambiguous writers reject atomically.

The evaluated subset is world source/target, nonadditive and one X→Rotate writer;
legacy identity Rotate and same-property X/Y mappings remain supported. Other
cross mappings, local/additive modes and transform timelines remain outside this
slice. `evaluate_pose` reports the observed native/runtime precision conflict at
certain half-turn boundaries explicitly; it does not publish a guessed pose.
The separately calibrated 0.001 comparisons count that refusal separately from
successful runtime evaluations. See [the measured scope and evidence](slices/transform-mapping-authoring.md).
