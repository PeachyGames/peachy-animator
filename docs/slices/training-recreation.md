# Tutorial recreation through MCP — November 14, 2026

**Verification scope:** the original studies and native playback were verified
November 14, 2026, baseline `a173c3e1229ab3a49115c94cbd14ede33cf89f40`.
A fresh v23 MCP/runtime replay on November 15 local / November 16 UTC at
`18bd0e44c46a6c1ee55f83e1daab210d47c94d5c` now passes as detailed below;
it does not add fresh native playback. Later bounded Deform/Graph/Curves, multi-vertex
Direct and clipboard acceptance is recorded in its own slices; those features
must not be treated as wholly absent because this earlier study bypassed their UI.

Fresh MCP authoring can build a rig and original animation studies from prepared
artwork without loading a finished skeleton. At that checkpoint, `create_project`
started a native v22 document containing one identity root and an empty default skin. `addBone`,
`addSlot`, `addAttachment`, `setSlotAttachment`, `addAnimation`, `setKey` and
`setDeformKey` then author the document through `apply_commands`. This is a
separate acceptance path from opening an existing native or Spine export.
See [MCP](../MCP.md) for strict schemas, revision guards, asset-root confinement
and Save behavior.

`addSlot` appends an empty slot on an existing bone. `addAttachment` inserts a
region or mesh into an existing skin and slot; occupied placeholders are
rejected. Mesh payloads carry independently authored geometry, UVs, triangles
and influences. These commands do not introduce a native-format migration.

## Current v23 replay — November 15 local / November 16 UTC

Checked at 2026-11-16 04:48:34 UTC on HEAD `18bd0e4`, with **64 commands /19 tools**.
Both studies were rebuilt from a blank root and the prepared images through
**7,033 construction commands** (4,063 Halloween /2,970 Wind), using **308 actual
stdio requests** (188/120; initialization notifications excluded). No finished
rig was imported as an authoring input. The complete documents equal their
historical baselines after Peachy's shared v22→v23 CLI decoder migration;
history, Save/reopen and empty reopen history are exact. Original inputs and
their source hashes remain unchanged.

There are **271 evaluation calls**: 252 samples at 12 FPS, 10 additional endpoint
queries, two finite Setup queries, two Halloween Deform-isolation queries and
five comparisons against the old native witnesses. Sampling includes frame zero;
these are not 271 unique/interior poses. All five old-native world, authored-local
and applied-local arrays compare exactly. Loop seams remain zero; the five Wind
case checks and bones-fixed Halloween Deform isolation pass again.

Fresh `runtime43` JSON exports, the previously verified atlas/texture with source
hashes preserved, and the current CLI pass **260 external-runtime 4.3.13 poses**
(156 Halloween /104 Wind) at unchanged **0.001** tolerance. Maximum world/mesh
errors remain **0.0004515261 /0.000732421875** for Halloween and
**0.0002205513 /0.0003662109375** for Wind. MCP SHA256 is `0f659abb…9d112`;
CLI is `a414e845…18ab6`. Both owned MCP children are closed.

This is a fresh headless authoring/geometry check against the earlier native
witnesses, not a new native playback or raster review. It installs no dependency,
renders no movie, performs no complete installed-editor interchange, establishes
no live GUI bridge and does not remeasure Setup-bind residuals. The November 14
loaded-art/playback and Setup-bind evidence below remains historical. See the
[redacted replay record](../evidence/training-recreation.json).

## Reference coverage

All four supplied video timelines were sampled every 30 seconds, with a final
frame near each video's end: **569 frames in 31 contact sheets**, all inspected.
The action ledger contains 47 timestamped chapters across approximately four
hours and 42 minutes. This is full-timeline sampled coverage, not continuous
watching or a click-by-click transcript. Targeted original-resolution frames
clarify selected tools and settings.

Full audio decoding found no instructional narration. The three Halloween
tracks are silent; Wind is silent until its final 16.4-second narrative showcase.
That outro's speech was not reliably transcribed, so low-confidence automatic
text was rejected. Its visible subtitles were recorded separately as on-screen
text with sample-bounded timings.

## Four-group acceptance map

The four supplied groups produced two authored animation studies, not four exact
tutorial reproductions. Part2 and Part3 share the Halloween evidence below.
The table combines historical November 14 native playback with the fresh v23
MCP/runtime replay above; it does not claim fresh native playback.

| Training group | Accepted outcome | MCP / native / runtime evidence | Main workflow or fidelity limit |
| --- | --- | --- | --- |
| Halloween Part1 — artwork preparation | Supplied cutouts assembled and preserved unchanged; painting and occlusion repair are external prerequisites. | The Halloween study loads all 37 images natively; source PNG hashes remain unchanged. | Peachy PSD ingestion and raster painting/repair are not demonstrated. |
| Halloween Part2 — rigging | Independently authored 79-bone, 27-mesh rig with blended weights and representative leg IK. | Shared Halloween study: 4,063 MCP authoring commands, exact history/Save/reopen, textured native playback and runtime geometry within 0.001. | Freeform Mesh New/Modify and automatic/brush/Smooth/Prune workflows were bypassed by explicit data; exact source hierarchy/topology/chest controls are unclaimed. |
| Halloween Part3 — animation | Three original clips cover body/head, hat/hair, garment/accessory, limb/broom motion and five local Deform mechanisms. | All three clips play natively without evaluation errors; Deform isolation holds bone transforms fixed; maximum runtime mesh error is approximately 0.0007324. | Multi-bone cycle Offset/ripple workflow and exact artist poses/timing remain unverified; later bounded Graph/Curves and clipboard checks are separate evidence. |
| Wind — hair and clothing | Five measured motion mechanisms using explicitly labeled artwork proxies; two original four-second clips. | 2,970 MCP commands, seven weighted meshes, native playback of both clips/all five panels, zero measured loop/root drift and maximum runtime mesh error approximately 0.0003662. | Original Wind art/rig was absent, preventing exact character comparison; native multi-bone cycle Offset remains a separate workflow gap. |

Later [Curves](curves-authoring.md), [multi-vertex Direct](weight-multivertex.md),
[grouped clipboard](grouped-clipboard.md) and [leaf routing](leaf-clipboard.md)
checkpoints extend the available authoring workflows. Their finite acceptance
does not turn these studies into exact source-animation or complete Spine UI parity.

## Historical November 14 recreation evidence

The private Halloween study begins with the prepared cutouts and one-root image
layout. It independently places controls, constructs meshes and weights, and
authors smooth motion through the actual stdio MCP server. Its only later
`open_project` call reopens its own saved output. Finished tutorial rig topology,
hierarchy and animation keys are not reconstruction inputs.

| Study | Actual completed snapshot | Scope |
| --- | --- | --- |
| Halloween | 79 bones, 37 slots, 27 meshes, one IK constraint; three clips of 6, 4 and 3 seconds | Body/head motion, weighted hat and hair, chest/cowl lift, pack/strap, limbs/broom and five local Deform targets |
| Wind | 48 bones, seven slots/weighted meshes; two 4-second clips, five mechanism cases | Front hair, head cloth, long cloth, upper garment and waist sash, using explicitly labelled art proxies |

The Halloween snapshot has 26 weighted meshes and one unweighted face mesh.
Of 1,127 weighted vertices, 715 blend two positive influences. Setup binding
preserves authored positions within 0.000285 world units. Undo/redo restores the
captured rig/authored document exactly; Save/reopen preserves document equality.
Source PNG hashes remain unchanged. JSON and packed exports both succeed.

Actual MCP pose evaluation produced 156 Halloween and 96 Wind software-preview
frames at 12 FPS. All five clip endpoint loop residuals are zero. Each Wind case
also passes measured mesh-motion, loop, fixed-root and rotation checks in both
clips; upper-garment Y and sash X/Y translation are verified. Every case has
zero measured root drift and loop residual.

The separately licensed Spine 4.3 runtime is used only as an external oracle:

| Final study | Transform comparisons / max error | Mesh comparisons / max error | IK pose comparisons |
| --- | --- | --- | --- |
| Halloween | 73,944 / 0.000452 | 377,832 / 0.000732 | 156 |
| Wind | 29,952 / 0.000221 | 105,248 / 0.000366 | 0 |

Both pass the 0.001 geometry tolerance. Maximum UV errors are below 0.00000003.
These numerical and software-preview checks are separate from native editor
window, texture and control review. Both final saved projects were also opened
in verified normal foreground native windows: Halloween loads 37 images and
plays three clips; Wind loads seven images and plays two. All clips advance
without evaluation errors, including loop wrapping, and the documents remain
unchanged at native f32 precision. The coordinator inspected the fitted full
Halloween character, including hat and boots, and all five Wind panels.

The redacted scalar record is [training-recreation.json](../evidence/training-recreation.json).
Private artwork, native documents, exports, screenshots and detailed protocol
logs stay outside Git.

## Outcome and workflow limits

Weighted bone motion supplies flexible hat, cowl and broom outcomes. The final
Halloween study also authors three local Deform keys per clip for each of the
face, hat, cowl, body and broom. An actual MCP isolation check temporarily removes
bone keys, evaluates two poses and undoes the test: bone-world change is zero,
while the five meshes move by 4.5, 7, 5.00012, 3.80432 and 9 world units.
This establishes local shape changes independently of bone motion.

A fresh animated two-bone leg IK target demonstrates constraint creation and
evaluation. The sampled tutorial shows chest-target constraint entries, but
their exact settings were not established; this representative IK does not
claim the original chest-control arrangement.

Photoshop separation, repainting and PSD preparation are supplied-art
prerequisites. This study does not establish Photoshop tools or native PSD
ingestion. Artwork preparation quality and crop/layout differences also limit
an exact visual source match. Explicit mesh/weight data and authored phase
offsets do not establish
Spine's Auto/brush weights, Graph, Ghosting or multi-row Offset interaction
parity. Exact artist poses, timing, hierarchy and triangulation remain unclaimed.

The visual samples do not establish a blink sequence, smoke effect, standalone
bat-wing keying chapter or standalone lantern-swing chapter. Wind recreations
must remain qualified as mechanism studies until the original character's art
and rig are available; motion proxies cannot establish an exact character match.
