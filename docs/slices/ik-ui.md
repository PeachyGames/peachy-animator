# Static normal IK: desktop boundary

This desktop slice supports imported one-/two-bone IK constraints in the existing
Tree and Properties workspace. Constraint rows keep document application order,
have independent vector icons, participate in search/filter/reveal/selection
history, and never act as an implicit bone selection. Bone annotations select the
constraint; active constrained-bone origins have an outlined cue. These are
independent drawings, not copied assets or a claim of exact 4.3 pixels.

F2 and the shared header Rename icon use the existing name dialog and replace
only the selected constraint's name through SetIkConstraint. Empty/duplicate
names fail atomically; refs, order, settings and pending bone poses stay intact.
Opening Rename cancels a target chooser or preview before capturing the name.

In Setup, Mix is a percentage slider/numeric entry and Positive is available for
two-bone constraints. Parent, Child and Target names select their actual bones.
The Target pencil enters an existing-bone chooser; a concrete Tree/viewport bone
click attempts one shared SetIkConstraint command. Errors end that attempt without
retry or redirect. Escape cancels before any command. Other item/empty-space clicks
do not create a new target. The normal selection remains the constraint after
retargeting. Current installed-editor chooser completion/cancellation details
remain an interaction comparison gate.

Animate shows the static setup controls read-only and says constraint keys are not
supported yet. Ordinary target-bone posing and animation use the normal authored
bone paths. No constraint-key button or fake bone timeline is introduced.

Creation, Parent/Child replacement, target-at-tip/empty-space creation, ordering,
deletion, advanced IK settings and constraint animation remain explicit gaps.
The current single-selection Tree cannot perform the documented two-bone creation
workflow; this UI does not invent a second-bone wizard. The core Add command and
CLI can create original accepted fixtures independently of that UI gap.

Mix previews freeze the explicit constraint/base settings and use Session's fixed
transaction baseline. Enter/Tab or ordinary blur settle after all frame actions
and pointer release, producing one Undo. Untouched formatted values and invalid
text do not create an edit. Text undo retains priority. Escape, focus loss,
history, tools/context, Open/Save/Export and view changes cancel before a new
capture, suppressing stale text responses and a canceled held pointer until its
release. Direct, transform and IK previews have one owner; Weights Bind and the IK
target chooser are mutually exclusive. Pending bone locals remain separate from
document settings and are neither keyed nor discarded by constraint edits.

Authored local values feed numeric/key editing. The shell queries one checked
EvaluatedPose for each transform baseline; it retains authored and applied values
separately. Canvas origin/axes use applied world output and the shared engine draw
helper. There is no persistent pose cache, UI solver or applied-pose bake.
Viewport gesture conversion also receives frozen authored/applied locals, actual
world and parent frames through the shared engine helper. It projects along the
displayed frame while applying deltas to authored values; this establishes
internal control coherence, not exact installed-Spine partial-mix semantics.
Setup local rotation fields and edits preserve raw turns for directly referenced
IK bones, including a currently zero-mix constraint. Raw winding affects partial
IK: authored 720 with a north target at half mix evaluates to 585; relative +10 must
retain 730 and evaluate to 590. Ordinary FK and region Setup range conventions
remain unchanged. This authored-pose policy keeps command, preview and display
consistent; the exact installed-Spine Setup field convention remains an
observation gate rather than an inferred normalization rule.
A valid native edit that lies outside the current evaluator domain remains authored data
and displays a checked evaluation error; it is not silently replaced with FK.
Exact constrained-bone World/Parent/constrained-axis editing remains a reference
observation gate. Phase A acceptance centers on target manipulation and supported
setup constraint properties.

Headless interaction fixtures render Editor::workspace_ui, the same body used by
eframe, with the shared native context style. Native GUI/editor/runtime acceptance
is coordinator-owned and separate from these tests.

## Public references

Retrieved November 6, 2026 through Agent Reach/Exa:

- [IK constraints](https://esotericsoftware.com/spine-ik-constraints): New… → IK
  Constraint; one/two-bone setup; Parent/Child/Target links and pencils; Mix and
  Positive terminology. The guide is undated and not pinned to 4.3.26.
- [Constraints](https://esotericsoftware.com/spine-constraints): Constraints Tree
  node, application order, hollow bone cues and clickable Tree annotations.
- [Tree](https://esotericsoftware.com/spine-tree): selected-item properties and
  selection/navigation conventions.
- [4.3 API](https://esotericsoftware.com/spine-api-reference#Bone): authored and
  applied pose distinction.
- [4.3.73-beta changelog](https://esotericsoftware.com/spine-changelog#v4-3-73-beta):
  April 27, 2026 replacement of the old Uniform setting by scaleY/Volume. The old
  guide does not establish the current visible widget/label. Advanced modes are
  not exposed in this slice.

The coordinator retrieved the official guide PNGs in an isolated reference
browser after the text reader refused their content type. We inspected the
[one-bone properties](https://esotericsoftware.com/img/spine-user-guide/ik/properties1.png),
[two-bone properties](https://esotericsoftware.com/img/spine-user-guide/ik/properties2.png),
[creation](https://esotericsoftware.com/img/spine-user-guide/ik/setup.png),
[Tree](https://esotericsoftware.com/img/spine-user-guide/constraints/tree.png) and
[annotations](https://esotericsoftware.com/img/spine-user-guide/constraints/annotations.png).
They establish the IK Constraint: name header, aligned bone-reference rows with
cues, an Options row containing Positive for two bones, and Mix numeric entry
before its slider. The independent controls follow those organization cues.
The images have unknown application versions and show the obsolete Uniform
checkbox. They do not establish current 4.3 pixels or advanced settings. No
reference assets were copied into Git. Exact current styling and chooser behavior
remain visual/reference gates; no extra pane or creation wizard was inferred.

## Native diagnostics

Unmodified C/V/X/Z/G tool actions are routed outside text, popup and modal
ownership, including after a Target link or normal button has focus. They retain
event order and exact modifiers; file/history modified shortcuts run first. The
focused widget keeps arrows, Enter, Space and Tab. Tool actions cancel a held
preview before switching, including selecting the current tool again. Numeric
text retains tool letters as text and keeps its own undo.

This is a deliberate inference from the globally listed defaults in the
[Tools guide](https://esotericsoftware.com/spine-tools) and text-field ownership;
it is not an observed claim of exact installed-Spine normal-button focus behavior.
That comparison remains a gate. A historical human staff discussion of
[text-field focus](https://esotericsoftware.com/forum/d/11231-editor-focusing-on-rotate-field-after-mode-switch)
also distinguishes text traversal from viewport hotkeys, but is not 4.3 evidence.

Targets: constraint:<index>, folder:Constraints, ikParent/ikChild/ikTarget,
ikTargetChoose, ikMixSlider, ikMixValue, ikPositive, and
ikAnnotation:<bone>:<constraint>. F12 ik reports selection, target-picker and Mix
gesture state. evaluatedPose reports authoredLocals/appliedLocals/world or its
checked error. transformGesture also retains applied launch baselines.
