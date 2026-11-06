# Character workflow milestones

The goal is to use existing game characters in a familiar Spine-style editor, keep
editing them, save the work, and ship compatible runtime exports. Original Peach Bot
fixtures prove the current bounded loop; production characters are a separate gate.
Do not count a rejected import or a synthetic success as a migrated game character.

| Hurdle | Current evidence | Remaining acceptance gate |
| --- | --- | --- |
| Import our Spine characters | Supported 4.3 JSON opens; strict inspection explains refusals | November 5 re-audit: 0/11 sampled artist exports fully supported. Implement their needed weights, linked meshes, constraints and timelines; import packed art; verify an unchanged full production rig |
| Modify a character | Bone transforms/length, region transforms, slot setup attachments, unweighted setup vertices and subdivision | Rig/image creation, pivots, remaining entity naming, delete/reparent with stable IDs; broader mesh/UV and weight authoring |
| Work on poses | Setup pose and transient Animate poses; numeric/canvas controls, Auto Key, K and undo | Scale/Shear/Pose tools, axes, multi-selection, constraints and weighted geometry |
| Work on animations | Supported bone transforms/curves, pose keying, exact key selection/deletion, group retiming/duplication, Copy/Cut/Paste, animation rename/duplicate and transport | Broader timeline selection, extended playback, Graph, clip creation/deletion, mixing and remaining timeline types |
| Work on slots/features | Typed slots/placeholders/attachments/skins, setup attachment edits, temporary editor visibility | Attachment/color/draw-order animation, slot creation/order/blending, sequences, events, clipping and constraints |
| Export for the game | Fresh 4.3 JSON/atlas/PNG bundles; separately supplied Spine runtime checks; installed editor import/re-export | Representative production-character round trip and actual game-engine load/render/playback, including the features that rig uses |
| Save editable work | Strict `.peachy` v3 saves, v1/v2 migration, reopen, verified backups, atomic replacement and image copying | Broad real-rig round trip as coverage expands; crash recovery/packaging. Unkeyed poses must be keyed first |
| Save native Spine files | Our JSON can be imported into the installed Spine Editor and saved there as `.spine` | Peachy Animator itself does **not** read/write native `.spine` projects or binary `.skel`. These are separate format projects |

The latest artist inventory was read-only: all 11 sampled input hashes match the
initial audit. Identifying paths, artwork and detailed feature reports stay outside
Git. Common blockers include weighted meshes, linked meshes, shear, IK/transform/path/
physics constraints, attachment/deform, slot and draw-order timelines. Fixing one
blocker does not make an entire character importable.

Use two continuing tracks: reference-matched authoring controls, and progressively
broader format/evaluation support driven by a selected real rig. Weights are the next
large compatibility step; tool/axis and timeline work continue alongside that track.
Each feature needs independent core behavior, undo/save checks, native UI verification,
export round trips and external runtime comparisons before its milestone advances.

Current limits are specified in [COMPATIBILITY.md](COMPATIBILITY.md); UI differences
are tracked in [UI_PARITY.md](UI_PARITY.md). Full feature parity is the objective,
not a current claim or a fixed completion estimate.
