# Direct weights: one vertex and one bone

The desktop Weights view now supports Direct percentage entry and a slider for one
explicit vertex and one existing bound bone. Bind can also add a genuinely new
bone to an already weighted mesh, at zero weight on every vertex. Initial Bind of
an unweighted mesh and Remove all retain their previous current-pose behavior.
Native v4 and the export adapters are unchanged by this UI slice.

## Public reference and scope

The official [Weights guide](https://esotericsoftware.com/spine-weights#Direct)
describes Direct mode and its Weight slider, selected vertices and selected bound
bones. Its [adjustment rules](https://esotericsoftware.com/spine-weights#Adjusting-weights)
and the staff explanations linked in [the approved engine design](weight-direct-design.md)
support proportional redistribution, initial one-bone 100% binding and subsequently
added zero-weight bones. The [Tools guide](https://esotericsoftware.com/spine-tools)
and [Views guide](https://esotericsoftware.com/spine-views) supply the existing G,
view visibility and retained per-tool selection conventions. These are public
behavioral references, previously retrieved through Agent Reach's official-page
route; no proprietary source or UI assets were inspected or copied.

The existing resizable right-column Weights view keeps its independent icon, bone
color swatches and explicit mesh/vertex/bone selection. It adds Mode: Direct and
Weight with a percentage slider and numeric entry. Beginning a Weight edit activates the genuine Weights tool and retains its explicit
selection. Direct is the only implemented
mode, so there is no selector advertising inactive modes. This column placement
remains provisional within the current docking system; it is not a verified
installed-Spine default dock or a claim of exact visual parity.

Brush painting, vertical viewport weight dragging, multiple vertices/bones,
Auto, locks, Update bindings, individual bone removal, sparse binding repair and
ambiguous repeated-bone editing remain outside this slice. Those controls are not
presented as active features. Sparse/repeated selected-vertex cases show why the
Weight control is unavailable. A Bind attempt involving any already-bound bone
reports the engine's explicit error and ends Bind mode without retrying.

## Command and gesture boundary

Slot-local Bind still calls `Session::bind_mesh_to_bone`. A weighted mesh instead
calls `Session::add_mesh_bone_binding` after a concrete target click. The UI does
not construct positions or change old influences. The engine checks the current
posed geometry and required inverse, appends a zero binding, and preserves old
weights, positions and ordering.

A Direct gesture freezes the exact resolved attachment reference, vertex, bone,
Setup/Animate mode, clip, actual time and one `Session::mesh_bind_pose` snapshot.
Every value preview sends `SetMeshBoneWeight` through Session's fixed transaction
baseline. The UI only converts percentages to the command's 0..1 input; the
engine owns proportional redistribution, finiteness, rejection and history.
Direct intentionally can move the selected vertex. It does not update bindings or
compensate positions to preserve world shape.

Pointer release, Enter, Tab and ordinary control blur commit a successful edit as
one undo step. Completion waits until all frame actions run and the pointer is
up, so a toolbar/context click can cancel before its release action. An active
numeric edit is exempt from the general pointer-idle transaction finalizer.
Escape, application focus loss, tool/mesh/vertex/bone/skin/time/mode/clip changes,
history, Open, Save, Export and hiding the view cancel an owned preview first.
An Undo/Redo request during a preview cancels it without consuming older history.
Numeric text keeps ordinary text undo and tool-shortcut priority. Cmd/Ctrl+S is an
explicit exception for the owned Weight field: it cancels before saving.

Cancellation suppresses stale changed/blur/Enter responses and a held pointer
until release. A new target has separate widget IDs, so old text cannot apply to
another vertex or bone. Merely focusing and blurring a rounded percentage or
entering invalid numeric text preserves raw imported values and redo. Real numeric
input is constrained to 0..100%; serialized commands remain strictly validated by
the engine. A rejected preview leaves its last valid preview visible, with the
error, until corrected, committed or canceled. No background retry occurs.

Bind and Direct use actual pending Animate locals and time without frame snapping,
implicit keying or pose discard. Direct Undo restores the document and exact
pending pose together. Existing timeline/context actions retain their own explicit
pending-pose policies after canceling the weight preview.

## Automated checks and native recipe

The shell regressions cover the from-scratch A=100% → Bind B=0% → selected-vertex
B=30%/A=70% workflow, unrelated vertices, fixed-pose Animate previews, exact pending
pose/history restoration, failures/no-ops/redo, actual numeric input frames across
pointer idle, Enter/Tab/Escape, application focus loss, real toolbar Save press and
release, numeric Cmd/Ctrl+S, and held slider Escape/Save/history followed by further
movement/release. Rounded nonunit values and invalid text have explicit regressions.
The coordinating agent owns merged checks and native/runtime acceptance; these
headless UI checks alone do not establish native parity.

Native acceptance should use an original task-owned fixture and verify typed values
and resulting document data:

1. G opens Weights. Select a vertex and a bound bone, enter 30 in Weight, then
   Enter. Inspect only that vertex's proportional weights, unchanged bind positions,
   pending pose/time and unaffected vertices. Save, Undo and Redo the actual result.
2. Hold the slider pointer, press Escape or Cmd/Ctrl+S, continue moving and release.
   The canceled value must not return or become a history entry/saved value.
3. Enter a different numeric preview, then Escape. Repeat with a real toolbar Save
   click using separate press/release events. Verify the baseline was saved.
4. Start with an unweighted mesh. Bind one concrete bone A, then Bind a genuinely
   new bone B. All vertices must show A=100%, B=0% with unchanged world shape at
   the captured pose. Direct B=30% changes only the chosen vertex to A=70%, B=30%.
5. In Animate, pose a bone without keying, then edit Direct. Confirm unchanged
   pending locals/time and no new keys; Undo restores the exact previous document
   and pose. Confirm G/C/V/X, right-click tool history and retained selection.
6. Exercise text/modal/menu priority, view hide/restore, active-skin/default-fallback
   reconciliation, already-bound bone errors and application focus loss.

F12 targets: `weightsDirect` (the Direct mode label), `weightsWeight` (the combined
row), `weightsWeightSlider`, `weightsWeightValue` (numeric entry), existing
`weightsBind`, `weightsRemoveAll`, `weightsBone:<index>` and `weightsVertex:<index>`.
The `weightsDirect` diagnostic contains the explicit current target, percentage or
unavailable reason, cancellation blocking state, and any frozen active target,
mode/clip/time/pose/value/error. `weightsContext` continues reporting world vertices
and handles even when the normal typed Tree selection is a Bone.
