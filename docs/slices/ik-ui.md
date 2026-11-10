# Normal IK: desktop boundary

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

Animate edits sampled Mix and Positive as pending authored pose values. Auto Key,
the constraint's Tree key button and global K use one complete supported IK group;
the existing bone key paths retain their scopes. The Dopesheet projects one grouped
IK row for the selected constraint, reusing its exact selection, drag, duplicate,
delete, Copy/Cut/Paste and Select All paths. Bone and IK owners have separate typed
identities. Wrong-kind paste reports an error; same-kind cross-constraint paste
keeps absolute Mix, Positive and outgoing curves. The clipboard envelope is version
2 and only the current session's exact mirrored payload authorizes an OS paste.
Ordinary target-bone posing and animation retain their authored bone paths.

Creation, Parent/Child replacement, target-at-tip/empty-space creation, ordering,
deletion and advanced IK settings remain explicit gaps. IK curve data is sampled
and retained through keying and timing edits; an IK-specific curve/Graph authoring
control remains a separate UI gap. The grouped row label is provisional, and
IK-specific L/Cmd-L behavior is not inferred from bone transform shortcuts.
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

An explicit Key request during a valid bone or IK numeric preview settles after
the current widget has parsed its input, then keys while the same Session
transaction is open. Completion creates one Undo step. Keying another item's
scope retains the valid owned authored preview as pending data rather than
silently dropping it. Invalid/unparsed text or a failed preview blocks Key and
Positive with a visible message. A canceled or keyed held pointer cannot restart
the old gesture before release. Direct weight previews are canceled before pose
keying because they edit mesh setup data. This is a coherent ownership policy;
exact installed-Spine preview-to-Key behavior remains a comparison gate.

The Mix number accepts finite stored values outside the slider's 0–100% range.
Untouched focus/blur preserves imported bits and redo. The slider limits actual
slider edits; it does not clamp values merely while displaying them. Sampled IK
Mix stays f64 in the complete authored capsule, and normalized Bezier time handles
remain f64 through copied payloads. Session owns f32 key conversion and the 0.001
current-frame continuity guard; the UI neither normalizes nor bakes a failed pose.
Indicators and navigation inspect the actual cursor and exact existing key before
any authoring grid snap, including adjacent fractional keys. The Dopesheet retains
its established panel height while displaying the selected owner's rows.

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
setup constraint properties. Animated sampling/pending values now enter rendering,
picking, transform capture, Bind and Direct through the complete AuthoredPose;
bone locals alone cannot substitute setup IK during Animate.

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
- [Keys](https://esotericsoftware.com/spine-keys): grouped IK key buttons, Auto Key,
  K and absent/edited/keyed color cues. The supported group here contains Mix and
  Positive; softness/compress/stretch are not active controls.
- [Dopesheet](https://esotericsoftware.com/spine-dopesheet): key timing, item/property
  rows and selection. Its text does not establish an exact current IK row label,
  Graph layout or IK-specific transform-tool shortcut.
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
