# Bone shear: proposed next vertical slice

Status: design for review, not approved implementation or a parity claim. Prepared November 6, 2026 from clean baseline `f4b6ce72bd455866ca1d019b90594211f6fdd54b`. No application, Cargo, fixture, or public documentation edits accompany this proposal. The Direct desktop work integrating after this baseline must retain its cancellation and pending-pose guarantees.

## Scope and evidence

The proposed slice adds bone setup shear, animated scalar shear channels, the familiar paired Shear controls, and compatible Spine 4.3 interchange. It includes evaluation, existing transform-tool interactions, weighted geometry, native migration, history, timeline operations and native acceptance. A matrix-only implementation does not complete this slice.

Public sources retrieved through AgentReach's Exa search/reader on November 6:

1. [Bones user guide](https://esotericsoftware.com/spine-bones): a bone transform includes shear; normal parent transforms affect descendants; per-animation separation of X/Y is available.
2. [Tools user guide](https://esotericsoftware.com/spine-tools): default Shear shortcut **Z**; X/Y handles change the angle between the axes. The main transform controls support numeric entry, relative operators and keyboard adjustments. Shear values are stored in Local axes. World rotation describes the actual X-axis direction; displayed World scale is a positive combined magnitude, while scale entry edits Local values.
3. [Keys user guide](https://esotericsoftware.com/spine-keys): shear keys are relative to setup; default shear keying includes both channels. Auto Key and pending-pose rules apply to transforms. Separate X/Y is an optional per-animation feature.
4. [Runtime skeletons guide](https://esotericsoftware.com/spine-runtime-skeletons): local bone transforms produce a parent-composed affine world transform whose columns describe the X/Y axes. The same world transform affects attached geometry.
5. [4.3 public API reference](https://esotericsoftware.com/spine-api-reference): `BonePose` exposes local `shearX`/`shearY`, world-axis rotations and positive world-scale magnitudes. `ShearTimeline`, `ShearXTimeline` and `ShearYTimeline` exist. Converting a world matrix back to locals can produce equivalent rather than identical values because decomposition is ambiguous.
6. [JSON format guide](https://esotericsoftware.com/spine-json-format): setup bone `shearX`/`shearY` default to zero; paired animation `shear` values are setup-relative. Region attachment transforms list position, rotation and scale, with no independent region shear channel.

These are public descriptions and API contracts. No linked editor/runtime implementation is an input. The JSON guide still illustrates older field spellings, including rotation `angle`, whereas our accepted 4.3 adapter uses scalar `value` timelines. Do not treat this page as a complete version-pinned 4.3 schema: separate shear spelling and paired curve layout require the black-box 4.3 acceptance gate below. The API page identifies itself as 4.3; the general user guides are not patch-version pinned.

The previous request's Y shortcut was an unverified assumption, subsequently corrected by the coordinator. Use sourced default **Z**, with modified Cmd/Ctrl+Z and Cmd/Ctrl+Shift+Z history routing taking priority. Do not introduce Y as another Shear mapping.

## Native v5 and command contract

Keep existing `Transform` as the public **bone and pose** value; extend it with `shear_x: f32` and `shear_y: f32`. Both default to zero in `Default`, are serialized as those snake_case fields like existing `scale_x`, and must be finite. Native v5 requires both fields; do not add blanket serde defaults that conceal malformed current-format documents. Preserve all authored finite angles, including negative and multi-turn values; do not normalize stored shear to 0–360.

Add `Property::ShearX` / `Property::ShearY`, serialized as `shearX` / `shearY`, with neutral zero and ordinary `read` / `write`. Append them to `Property::ALL` after the existing five properties to preserve established ordering. Scalar key values are offsets from setup shear, not absolute angles and not tangent coefficients. Interpolate scalar values using the existing Linear/Stepped/normalized Bezier rules without shortest-angle wrapping. Before the first key the offset is zero; after the last key it holds. These are the proposed additive semantics supported by the setup-relative public contract; multi-turn and sign examples still enter external acceptance.

`transform_group` expands either shear component into the pair, just as it does translation and scale. `SetKey` and low-level `SetPose.properties` retain independent scalar operation; normal toolbar keying, Auto Key, K/Key Edited, L and Key All include the pair. `PoseEdits` stores absolute posed shear values and the existing Session key conversion subtracts setup. No new command or undo stack is necessary for shear itself:

```rust
// Existing APIs, now with seven-field bone Transform and seven properties.
Command::SetSetup { bone, transform }
Command::SetPose { bone, target, transform, properties }
Command::SetKey { animation, bone, property: Property::ShearX, key }
Session::edit_pose(bone, target, transform, properties, auto_key)
Session::key_pose(animation, time, bone, properties)
```

`MeshBindPose.locals` consequently carries the same seven fields. New command payloads and original CLI acceptance generators must include them, ideally by reading decoded setup/Session snapshots. Old five-field serialized bone commands are rejected clearly rather than interpreted using an undocumented command migration. Native project migration is explicit and separate from command deserialization. Preserve strict unknown-field rejection and finite-value validation for commands, clipboard payloads and every bone pose.

This scalar addition changes neither bone order nor topology. Existing bone indices remain stable; it does not require inventing entity IDs. Deletion/reparenting still retains its existing identity prerequisite.

### Keep region transforms representable

Recommended boundary: add strict `RegionTransform { x, y, rotation, scale_x, scale_y }` and change `Region.transform` plus `Command::SetRegionTransform.transform` to that type. Its serialized shape is exactly today's five-field region transform, so native v5 regions need no extra fields. Give it finite validation and `Default` independently of bone shear.

Provide explicit `From<RegionTransform> for Transform` with zero shear for shared tool display/evaluation, and checked `TryFrom<Transform> for RegionTransform` that rejects either nonzero shear instead of dropping it. Update region command construction through that checked conversion. A region selection must not enable the Shear row, its shortcut edit or viewport shear gesture. It can continue to inherit shear from its slot bone; that is fully representable by bone matrices.

Split adapter helpers into bone and region transform readers/writers. Adding `shearX` to a shared JSON transform writer would otherwise accidentally write unsupported region fields. Keep importing region shear rejected, including a supposedly harmless zero-valued unknown field, under our current region format subset.

An alternative seven-field shared Transform with adapter rejection is smaller initially, but allows native region shear that the UI or exporter may later mishandle. Omitting it only in the writer would silently lose geometry. A five-field region type makes the supported boundary reviewable and is preferred for this slice. This is our representation policy, not a claim that Spine's UI cannot provide other attachment editing workflows.

## Affine evaluation: independent mathematics to verify

Proposed parameterization, inferred from angle-controlled bone axes rather than copied implementation:

```text
r  = rotation in degrees
hx = shear_x in degrees; hy = shear_y in degrees
u  = r + hx
v  = r + 90 + hy
A  = [ scale_x*cos(u)   scale_y*cos(v) ]
     [ scale_x*sin(u)   scale_y*sin(v) ]
M  = [ A  (x,y) ]
world(child) = world(parent) * M(child)
```

Thus X-only shear changes the first basis direction, Y-only shear changes the second, and equal shear on both axes rotates both directions without changing their mutual angle. Column lengths remain the absolute local scales. This differs from inserting a generic tangent shear matrix. Signed scales reflect the corresponding columns. The determinant is `scale_x * scale_y * cos(hx - hy)`: nonzero scales alone do not ensure invertibility.

This is a **proposed mathematical contract pending original-fixture runtime/editor comparison**, not a documented exact equation or a claim of verified Spine shear signs. At zero shear preserve the present affine construction and existing output tolerances; avoid changing all legacy numerics merely to add this feature. Sum/reduce angles for trigonometry using finite intermediates (f64 where needed) so two finite authored f32 values cannot overflow on addition. Reduction is for trigonometry only; do not rewrite the stored angles or keys. Check very large finite angle behavior and equivalent cardinal/parallel axes against the external oracle before choosing numerical details.

`local_pose`, `PoseEdits.locals`, `world_pose` and `scene_from_locals` remain the sole evaluation path. Region, slot-local mesh and weighted mesh rendering, picking, bounds and CLI diagnostics must consume that same result. No renderer-only skew, inverse-bind rewrite or extra slot transform is introduced. Non-normal inheritance modes remain rejected; supporting shear does not implement inheritance checkboxes, constraints or skin-required bones.

### Existing transforms must continue to work on a sheared bone

These assumptions in `crates/engine/src/tools.rs` need deliberate changes and tests, not extra UI math:

- `transform_values` currently derives World rotation from the world X column and World scale from column lengths. Those remain appropriate observable quantities; keep local shear values intact and do not decompose them from the matrix.
- World rotation entry currently inverse-maps the desired direction, adjusts for negative local X scale, and assigns it as `rotation`. With shear it must preserve `shear_x` and solve for rotation after accounting for that offset. For example, desired parent-space X direction theta implies `rotation = theta - shear_x`, with the sign branch for negative X scale and the nearest-turn adjustment to the existing rotation. An unchanged displayed value stays an exact no-op. A zero X column has no meaningful direction; reject the operation rather than guess.
- Rotate drags should change only rotation, preserving both shear components, scales and translation. Parent-inverse angle differences can still produce a rotation delta for the actual pointing direction. Preserve current multi-turn and Setup rotation policies.
- `transform_axes` currently generates an orthogonal pair from `rotation`, assuming a rigid local frame. Scale and Shear handle drawing should instead follow the relevant evaluated basis directions. Negative scale and reflected parents must preserve handle labels and drag signs. Do not render a fabricated perpendicular Y axis on a skewed bone.
- Axis-restricted Scale currently uses dot products against normalized axes. With an oblique basis, projecting onto X can include a Y contribution. Derive axis coefficients through the required basis inverse (or prove a handle-specific equivalent), then change only the requested scale. Preserve shear and rotation. Reject an undefined required conversion atomically; do not snap the bone back to a decomposed orthogonal transform.
- Local translation display/set currently rotates by `rotation` alone. Whether Spine's Local translation frame includes explicit shear/scale signs needs observation. Proposed consistent geometry uses its actual unit directions and the corresponding inverse, while Parent/World translation keep their existing meaning. This proposed change must be approved after the observation gate; a guessed frame must not silently redefine current tool behavior.

Do not create a generic world-matrix decomposition to solve these edits: rotation versus equal shear, signed scales and complete turns are not uniquely recoverable. Carry the existing local parameters and change only the requested property. Required inverses and undefined zero-axis directions must fail without changing document, pending pose or history. Numeric Parent translation and Local shear need no inverse and should remain usable on a flattened bone.

## Shear tool and UI contract

Add `TransformTool::Shear` and desktop `Tool::Shear`, with properties `[ShearX, ShearY]`. Place paired numeric X/Y fields and the existing key-state button in the main transform toolbar, after Scale and before ancillary tools; preserve the established workspace and reuse independent icons/accessible controls. Show `Shear [Z]` and wire Z only after modified history shortcuts and text/modal priority. Do not implement shear as the old Y/redo shortcut.

Use shared `set_transform_value` for numeric edits, relative input operators, fine/coarse nudges and Session dispatch. Proposed initial numeric policy is Local shear values even when another axes mode is selected, clearly indicated like Local scale entry. The Tools guide says shear is stored locally but does not specify all display/handle behavior in each axes mode; exact World/Parent display, axis highlighting and negative-scale behavior remain native observation gates. Do not relabel a guessed world-shear decomposition as parity.

Proposed single-handle gesture math: map start/end pointer rays about the bone origin through the parent inverse, compute the angle delta, and add it to only the selected shear component. Keep rotation, other shear and scale values fixed. Capture explicit typed bone, initial locals, parent matrix, active animation/time and axis at gesture start. Preview from the same transaction base, with one Undo on completion; Escape, context/history changes, Save/Open/Export, menu/modal/focus changes cancel first and suppress the held pointer until release. Do not write pose reconstruction or a second transform model in the UI.

Public docs establish axis handles but not the full empty-space drag mapping, angular sensitivity, center-handle behavior, Shift snapping, zero-axis behavior, or which axis mode changes shear handles. Observe those using our original fixtures before finalizing `TransformGesture` semantics. Until then the above is an explicit candidate, not evidence that it matches Spine. Both complete numeric authoring and tested real handle interaction are required before closing the slice; an isolated matrix prototype is insufficient.

Reuse the current Session model for Animate: Auto Key off retains unkeyed shear; K and the paired button key it; scrubbing/context changes follow existing pose policy; Save/Export exclude it. Auto Key includes both shear channels without clearing unrelated pending properties. The Direct weight transaction must cancel before a Shear gesture begins, and the reverse must also be safe. Selecting slots/skins/attachments must never redirect a shear edit through a cached bone or retained Weights context. Right-click tool return and Mesh inspection return must recognize Shear as a real transform tool.

The dopesheet needs room for seven scalar rows: add labels, tool mapping, colors, hit targets and keyboard focus without clipping the final row at the current minimum panel height. Normal toolbar keying remains paired; native scalar tracks and imported separated timelines remain editable. This slice does not pretend to implement Spine's Separate X/Y checkbox or its grouped-row presentation; those are explicit remaining presentation gaps unless separately approved.

## Interchange and migration

Keep export target `4.3.26`; native format 5 and Spine's format version are independent. Import setup bone shear and allow `shear` paired timelines plus proposed 4.3 scalar `shearx` / `sheary` timelines, with omitted components defaulting to zero. Paired import splits into our independent scalar tracks, using the existing per-channel Bezier conversion. Export scalar shear tracks as `shearx` / `sheary` with `time`, `value` and existing curve encoding, once those spellings/layout are confirmed by an original installed-editor export and the public 4.3 API. Do not broaden version support based on an older JSON-guide example.

Update `inspect` numeric fields and supported timeline lists together with actual decoding. Validate unknown fields, malformed/truncated paired curves, conflicting paired+scalar timelines, duplicate tracks, unsupported equal-value curve overshoot and non-normal inheritance. Unsupported input is rejected unchanged, not partially imported or flattened. Preserve times, values and supported curve payloads through roundtrip. Existing zero-shear export objects must remain exactly equal to captured old goldens: omit setup shear fields when zero, and emit no new timelines without actual keys. A neutral authored shear key is still real data and must not be omitted; it can determine animation duration and mixing behavior.

Migration pipeline:

1. Preserve v1 attachment tagging, v2 duration capture and v3 vertex-array wrapping exactly, including all existing validation and legacy hold semantics.
2. Change the v3 step to set `format = 4` explicitly, rather than jumping straight to the latest `FORMAT_VERSION` as it does today.
3. For formats 1–4, reject fields/properties that were not valid in that legacy schema, including bone `shear_x` / `shear_y` and `shearX` / `shearY` tracks. Do not let the new v5 decoder accidentally legalize smuggled legacy data. Preserve strict validation of all surrounding fields and all skins.
4. A narrow v4→v5 step inserts zero shear into each valid bone setup and sets `format = 5`. Regions keep their existing five-field representation; weighted bind positions, weights, zero/repeated entries and ordering are unchanged. Decode/validate the strict current schema before materializing captured v2 holds; holds remain the same visible keys as before.
5. Save only native5. Future native formats, malformed current data and invalid legacy payloads still fail. No native `.spine` authoring-file writer is implied.

Convert all seven committed `.peachy` example files by adding only bone setup zeros and the format bump. Keep pristine original native bytes and pre-change JSON export goldens for comparison. Update fixture generators and schema assertions so a future regeneration does not erase the new fields. Do not alter private artist inputs or their exports. Re-run the existing weighted/Bind/Direct CLI acceptance runners against v5, including strict pose payloads and retained source-byte manifests.

## Complete shared paths and acceptance

| Boundary | Required work and meaningful evidence |
| --- | --- |
| Document / engine | Seven finite bone fields, strict five-field regions, scalar offsets and default paired key groups; exact command no-ops and atomic failures. Setup changes offset all animation poses consistently. |
| Affine / scene | Root and parented nonzero shear, X versus Y, equal shear, rotation and reflected/nonuniform scale. Region, unweighted and weighted geometry, bounds and picking agree with the same evaluated world columns. Compare original known points, not only our own forward/inverse functions. |
| Existing tools | World rotation with nonzero shearX; unchanged display no drift; Scale on skewed bases; Local/Parent/World translation frame; negative and zero scales. Editing one component preserves all unrelated authored values and complete turns. |
| Keying / history | Auto Key pair, pending shear K/paired keying, unrelated pending pose preserved; one-gesture Undo, redo after cancellation, fixed-base overwrite restoration, active Direct/Shear mutual cancellation. Save excludes unkeyed shear. |
| Timeline / clipboard | Seven rows visible at minimum size; scalar ShearX/Y identities through select-all phases, exact close fractional keys, move/copy/paste/cut/delete/undo across bones and clips. No rotation/shear remapping. Clipboard envelope remains version1 if its extensible property enum is the sole change; old builds reject unknown properties safely. |
| Binding / Direct | Bind/Add0/Unbind under sheared reflected and nonuniform bone poses preserve current coordinates within the existing absolute0.001 bound. Direct deforms through the same frozen sheared pose. Exact evaluated singular required matrices reject; unrelated singular bones do not. Preserve nonunit/repeated/zero data under inspection and unsupported edits. |
| Numeric precision | Small nonzero scales stay supported; finite-angle sums and required inverse intermediates cannot create NaN. Near-collinear sheared bases are assessed by actual finite reconstruction/residual, not a new blanket determinant epsilon. Keep the earlier large-coordinate precision rejection/regression. |
| Native / adapter / CLI | v1–v4→v5 and strict native5 roundtrips, legacy JSON goldens unchanged, malformed/unknown fields rejected; CLI serialized SetPose/SetKey and mesh pose snapshots; loose JSON plus packed atlas export/load and original source bytes unchanged. Shear-only clips exercise empty/frame0/positive end and playback duration. |
| External / native | Original 4.3 editor setup/animation shear import/re-export; runtime world matrices/vertices at keys and intermediate times, including Bezier. Real Z selection, both handles, typed values/Enter/Tab, paired key controls, pending-pose status, slider/context/Save cancellation and one Undo in the task-owned native window. |

Existing binding uses its checked f64 inverse and f32 forward reconstruction with absolute0.001 coordinate residual. Reuse it unchanged where a shear operation requires binding; do not add an epsilon merely because a mathematically parallel basis is represented by tiny trigonometric residuals. Define and test actual evaluated singularity/cardinal handling deliberately. A tiny nonzero determinant may be usable or may fail the existing shape residual; neither case justifies rejecting unrelated bones. Tool conversion currently has an older `Affine::inverse_point` epsilon policy: do not silently equate that with the stricter binding contract or claim small-scale tool parity without tests.

Run required workspace fmt/tests/Clippy/build/architecture checks on the merged implementation, then the external/installed/native gates. Existing separate-X/Y presentation, other inheritance modes, constraints, full rotation mixing and unverified gesture conventions remain clearly reported gaps. No success from a matrix test alone closes a UI/format boundary.

## Decisions before implementation

The representation, property/key grouping, region boundary and zero-preserving migration can be reviewed now. Before claiming full Shear interaction parity, the coordinator must approve observations for: shear sign/axis conventions and interpolation; actual 4.3 scalar/paired timeline encoding; Local translation/Scale handle behavior under explicit shear; numeric axes display; and empty-space/Shift/handle gestures. Keep any unobserved behavior bounded and named rather than silently adopting a generic skew tool.

Suggested ownership once approved: document/engine plus existing native fixture conversion; adapters/I/O/migration plus immutable export goldens; desktop tool/key/timeline integration; coordinator-owned original fixture, external runtime, installed-editor and native acceptance. This document authorizes no implementation or GUI launch by itself.
