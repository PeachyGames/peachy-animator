# Character workflow milestones

The goal is to use existing game characters in a familiar Spine-style editor, keep
editing them, save the work, and ship compatible runtime exports. Original Peach Bot
fixtures prove the current bounded loop; production characters are a separate gate.
Do not count a rejected import or a synthetic success as a migrated game character.

| Hurdle | Current evidence | Remaining acceptance gate |
| --- | --- | --- |
| Import our Spine characters | Supported 4.3 JSON includes weighted meshes; strict inspection explains refusals | November 6 UTC audit: weighted refusals 10→0, but 0/11 artist exports fully supported and all 11 require constraints. Add linked meshes/timelines/constraints and packed art, then verify an unchanged full production rig |
| Modify a character | Bone transforms/length, region transforms, slot setup attachments, unweighted setup vertices/subdivision, initial Bind/Remove-all, additional zero bindings and one-vertex/bone Direct weights | Rig/image creation, pivots, remaining entity naming, delete/reparent with stable IDs; brush/lock/multi-selection weights, Update and broader mesh/UV tools |
| Work on poses | Setup and transient Animate poses, Rotate/Translate/Scale, bounded bone Shear and axes, Auto Key/K/undo; weighted artwork follows posed bones and weights can be edited at that pose | Exact Shear interactions/Pose tool, compensation, multi-selection, constraints and broader weight authoring |
| Work on animations | Supported bone transforms/curves, pose keying, exact key selection/deletion, group retiming/duplication, Copy/Cut/Paste, animation rename/duplicate and transport | Broader timeline selection, extended playback, Graph, clip creation/deletion, mixing and remaining timeline types |
| Work on slots/features | Typed slots/placeholders/attachments/skins, setup attachment edits, temporary editor visibility | Attachment/color/draw-order animation, slot creation/order/blending, sequences, events, clipping and constraints |
| Export for the game | Fresh 4.3 JSON/atlas/PNG bundles; separately supplied Spine runtime checks; installed editor import/re-export | Representative production-character round trip and actual game-engine load/render/playback, including the features that rig uses |
| Save editable work | Strict `.peachy` v5 saves, v1/v2/v3/v4 migration, weighted influence preservation, reopen, verified backups, atomic replacement and image copying | Broad real-rig round trip as coverage expands; crash recovery/packaging. Unkeyed poses must be keyed first |
| Save native Spine files | Our JSON can be imported into the installed Spine Editor and saved there as `.spine` | Peachy Animator itself does **not** read/write native `.spine` projects or binary `.skel`. These are separate format projects |

The latest artist inventory was read-only: all 11 sampled input hashes match the
initial audit. Identifying paths, artwork and detailed feature reports stay outside
Git. Weighted-mesh refusals have fallen from 10 reports to zero, with no new
inspection issues after accepting integral float count/index tokens. Common remaining
blockers include linked meshes, IK/transform/path/physics constraints,
attachment/deform, slot and draw-order timelines. Fixing one blocker does not make
an entire character importable. The newer [shear audit](evidence/shear-artist-summary.json)
keeps all 11 source hashes unchanged, with zero new inspection issues and no shear
or weighted-data refusals. All 11 still require physics; eight also use IK, seven
transform constraints and three paths. Their 4.3 setup data uses a unified
`constraints` array. None is yet fully supported.

Use two continuing tracks: reference-matched authoring controls, and progressively
broader format/evaluation support driven by a selected real rig. Weighted preview
now works, as do initial one-bone binding and Remove-all at the current pose.
Additional zero bindings and single-vertex/bone Direct percentages extend that
loop. Bone shear now extends the native v5 model and bone animation; exact Shear
UI comparison remains open. Brush editing, locks, Update and constraints are
next; these are not implied by the current G Weights tool/view.
Tool/axis and timeline work continue alongside that track.
Each feature needs independent core behavior, undo/save checks, native UI verification,
export round trips and external runtime comparisons before its milestone advances.

Current limits are specified in [COMPATIBILITY.md](COMPATIBILITY.md); UI differences
are tracked in [UI_PARITY.md](UI_PARITY.md). Full feature parity is the objective,
not a current claim or a fixed completion estimate.

## Initial binding checkpoint — November 6, 2026 UTC

An active unweighted mesh can now be bound to one explicitly selected bone in
Setup or Animate. Remove all captures its current shape as slot-local vertices.
G opens the Weights tool/view; inspection context survives posing with normal bone
tools. Save/reopen and export preserve the resulting mesh data. Pending bone poses
still require explicit keying to become saved animation data.

Command, CLI, runtime, installed-editor interchange and separate native acceptance
are recorded in [binding validation](VALIDATION.md#initial-bind-and-remove-all--november-6-2026-utc).
This advances a bounded rig-editing operation, not import coverage: Direct changes,
adding bones to weighted meshes, Update, constraints, full production-rig editing
and actual consuming-game acceptance remain separate hurdles. Native `.spine` and
binary `.skel` decoding/writing remain unsupported.

## Additional binding and Direct checkpoint — November 6, 2026 UTC

The next bounded workflow is unweighted → Bind A=100% → add B=0% → Direct
B=30%/A=70% on one vertex. Bind appends the new zero entry without rewriting old
influences; Direct proportionally redistributes only the explicitly selected
vertex's other positive weights. It can deform the mesh and normalize that
selected imported total. Other vertices, bind positions, order and keys remain
unchanged. Weight edits preserve the actual pending pose, which still requires
keying separately to survive Save as animation.

The CLI/native-format and external-runtime gates measure edited results against
their own exports. Both ordinary limb binding orders are retained without sorting.
Installed-editor import and native pointer/text/history/Save behavior require
their own recorded acceptance: seven of eight editor inputs pass interchange;
the full native run verifies actual 30%/45% edits, cancellation, pending poses,
history and five Save backups. See the latest
[validation](VALIDATION.md#additional-binding-and-direct--november-6-2026-utc)
for those separate results and the editor's precision/repeated-entry limits.
Constraints, full production-character editing and
actual consuming-game acceptance remain unchanged hurdles; **0/11** sampled
production exports are fully supported.
