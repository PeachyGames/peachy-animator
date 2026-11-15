# World X-to-Rotate Transform mapping — November 15, 2026

**Verified bounded continuation:** shared engine/MCP, actual native Peachy
authoring and fresh public reopen, original reference controls, the expanded
runtime matrix and exact installed-editor interchange. See the
[capture gallery](../evidence/transform-mapping.html),
[scalar evidence](../evidence/transform-mapping.json) and
[original public lab](../../examples/peach-bot/transform-mapping-bot.peachy).
Native format **22** is unchanged; MCP exposes **53 commands / 13 tools**.

This extends the [existing world Transform subset](transform-constraints.md)
with one source property, **Translate X**, mapped to one constrained property,
**Rotate**. It does not implement every mapping described in the 4.3 release.

## References and control placement

The official [4.3 release post](https://esotericsoftware.com/blog/Spine-4.3-released)
describes translation driving rotation/scale, clamping and independently chosen
world/local spaces for source and constrained bones. The
[Transform guide](https://esotericsoftware.com/spine-transform-constraints) and
[JSON-format page](https://esotericsoftware.com/spine-json-format), retrieved
November 15 through agent-reach, still describe older Local/Relative/Target
controls and JSON fields. They do not establish current mapping defaults,
numeric limits or the new panel layout. The
[changelog](https://esotericsoftware.com/spine-changelog), 4.3.56-beta on March 30,
records world as the new default; imported explicit values are not proof of
newly created constraint defaults.

Actual installed **Spine 4.3.26** controls use Source and Targets rows, expandable
Offset and Mix groups, a Properties table and Options. The table has six rows:
Rotate, Translate X/Y, Scale X/Y and Shear Y. Source Min/Max appears on the left,
target Min/Max on the right, with colored nodes and a connecting curve. Only
mapped Rotate has a Mix control in the original X-to-Rotate input. Options
include local source/target, Additive and Clamp; Properties Match is present but
uncalibrated.

The current Peachy surface follows the six-row mapping presentation and uses the
shared range operation for the supported X-to-Rotate endpoints. Other mapping
domains remain bounded by the existing evaluation policy. An expanded native
Offset capture shows **Translate X/Y numeric fields and sliders** for this
cross mapping, without a Rotate offset row. Measured pointer positions fit a
nominal **−50..50 span over 107 coordinate units**, centered at X = 1247.5 for
the first slider and 1418.5 for the second. Editing **extrapolates beyond that
span**: first-slider positions 1298, 1340 and 1170 export X = 47.2, 86.45 and
−72.43; second-slider position 1490 exports Y = 66.82. These observations show
no hard endpoint clamp for the measured paths. Two-decimal native serialization
does not establish internal quantization. Private filenames containing
`clamped` describe the initial hypothesis, not the observed behavior.

Reference inputs contain six original bones, two regions and the existing
original `blue_cutout.png`. Image dimensions are **128×128**, with region scales
carrying the intended display dimensions. Earlier import resizing of explicit
24×24/120×24 dimensions is recorded privately and is not presented as lossless
interchange. Licensed reference captures and detailed logs stay private.

## Shared range API

The semantic command is:

```rust
EditTransformMappingRange {
    constraint, from, to, field, value: f64
}
```

Its strict wire tag is `editTransformMappingRange`; `from:"x"` and `to:"rotate"`
identify the supported mapping. `field` is one of `sourceMin`, `sourceMax`,
`targetMin` or `targetMax`. The command addresses an existing unique mapping,
preserves unrelated properties, offsets, mixes, keys and attachments, and uses
the usual atomic `apply_commands` batch/history boundary.

`transform_mapping_range` returns raw-derived f64 endpoints. The stored native
model remains a source offset and target offset/max/scale, all f32. With
nonzero scale, source maximum is recovered as:

```text
sourceMax = sourceMin + (targetMax - targetMin) / scale
```

An endpoint edit preserves its partners, subject to the measured equal-endpoint
repair: when either endpoint makes a group's Min equal Max, the group's Max
becomes **Min + 100**. Reversed ranges remain reversed. Nine independently
inspected native capture/export cases establish this repair in both groups,
whether editing Min or Max. The observed 100 span is a repair for this mapping,
not a generic default, a universal Rotate range or a 360-degree rule.

Exact readback submitted unchanged preserves raw data and redo. Querying or
focusing a value never repairs or normalizes imported data. At raw scale zero,
`sourceMax` is **null**, because native data does not encode that endpoint.
An exact known-field no-op stays a no-op. A SourceMax edit can resolve the range
when the target span is nonzero; other undefined-range mutations fail. Zero
target span, nonfinite/out-of-range storage, underflowed scale and collapsed
stored intervals are rejected atomically rather than assigned invented values.

`edited_transform_mapping_range` advances a pure constraint candidate using the
same transition as the command. Native previews can therefore retain successive
repaired endpoints while publishing one Session transaction. The complete raw
`SetTransformConstraint` remains available with its existing data semantics;
semantic endpoint repair is not applied to imports or raw setters.

Actual reference toolbar input verifies one Undo restoring the previous range,
one Redo restoring the changed range and an explicit Clamp click. Separate native
Peachy input verifies its transaction/cancellation behavior below.

## Evaluated behavior and precision boundary

The measured world-space mapping takes X from the source frame applied to the
constraint's Translate X/Y offset point. It maps the value using source offset,
target offset and scale, then clamps between target endpoints in either order
when Clamp is enabled. The result drives constrained rotation in degrees through
Rotate mix. Constraint Rotate offset is retained as data but does not affect
this cross-property path; the established identity Rotate path keeps its prior
behavior.

Angular difference receives one correction only when strictly below **−180**
or above **180**. Exact −180 does not take the identity-Rotate correction branch.
Large-angle calibration establishes conversion of the difference to radians
before applying mix. The applied result is written back into locals, so child
transforms and later constraints consume the same evaluated pose.

A paired original native control exposes a real consumer difference. Both
installed-editor captures point the follower downward at half mix, with driver
X = −180 and source-parent Y respectively 0 and 120. The inputs differ only in
that Y value. In the biased case, the external JS runtime computes source world
X = `−180.0000027846124` and points the follower upward. This case is **not a
runtime pass**.

The shared engine preserves its published f32 FK. A wider FK diagnostic checks
whether the angular branch changes with coordinate precision, using the current
applied locals/inheritance without running constraint solvers again. If the
native/editor and runtime branches cannot be shared, checked evaluation refuses
that pose explicitly. It does not silently switch consumers, widen the published
pose or relax geometry tolerance. A mapped angle outside finite f32 is also
refused, even if subsequent trigonometry could conceal the overflow.

The bounded domain remains world source/target, nonadditive, one X-to-Rotate
destination and one writer per constrained property. Local/additive modes,
other cross-property destinations, one-to-many mapping and duplicate Rotate
writers remain unsupported by the adapter/evaluator. Raw finite native data is
distinct from that evaluation capability; native document validation is not
replaced with a blanket format ban.

## MCP and measured verification

The revision-checked read-only tool is `get_transform_mapping`, with flat input:

```json
{"expected_revision":1,"constraint":0,"from":"x","to":"rotate"}
```

It returns `{revision, constraint, from, to, range}`, where `range` contains
camelCase endpoints and an optional/null source maximum. It changes no document,
history or revision and does not evaluate a pending desktop pose. MCP is an
independent headless session, not a bridge to an open native window.

Focused engine Transform checks total **22**; MCP schema/unit and protocol groups
total **12** and **17**, with CLI/MCP builds passing. Desktop filters pass **26**
Transform checks (including **12** constraint-control checks), **40** weight,
**52** curve and **26** Deform checks; **5** pane checks cover the bounded resize
continuation. These filters overlap. Formatting, all-target Clippy,
workspace build and architecture checks pass. Strict schema and actual
stdio tests cover endpoint authoring, unrelated-data
preservation, exact no-op/redo, Save/reopen, stale/unsupported/ambiguous queries,
unknown/null/overflow input and atomic rollback. Four adapter regressions cover
accepted offsets/scale/mix/clamp, raw zero/negative coefficients, unchanged
unsupported modes and duplicate-writer guards through both export targets.

The fresh **`actual-mcp-04`** recipe uses **122 actual stdio requests**, starting
from a root-only project and one original PNG. It authors the bones, regions,
constraint and calibration keys through commands, reproduces the measured range
sequence and retains **51** MCP poses. It verifies exact no-op redo, four atomic
rejections, stale read-only rejection and Save/reopen. Its final native project
SHA256 is:

```text
62a637db493f5abf41bd197c4cdb5db03c83120d95acae6e4fd24f285204757d
```

The fresh report records MCP executable SHA256 `aa1c1e9a…76875e5c6e` and CLI
`4cfd17ea…325c3bcd3`; complete hashes remain in the retained execution report.

External runtime checks of that authored calibration/default skin use **52**
samples at the unchanged **0.001** geometry tolerance:

| Comparison | Values | Maximum error |
| --- | --- | --- |
| Bone world transforms | 1,872 | `1.6592621648214845e-5` |
| Region coordinates | 832 | `1.52587890625e-5` |

The report also checks 52 draw orders, 104 images and 416 color values. It does
not report region-UV comparisons; zero UV comparisons are not UV acceptance.

The expanded original matrix accounts separately for all **103** selected cases:

- **101 evaluated** cases: 3,648 world values, maximum error
  `1.611465881978802e-5`, at unchanged 0.001 tolerance.
- **1 explicit shared-consumer precision refusal** at the biased half-turn.
- **1 explicit mapped-angle overflow refusal** outside finite f32.

Neither refusal is counted as a runtime comparison/pass. The matrix has no
unexpected failures after the precision policy and multiplication-order fix;
it does not establish complete native Transform parity.

## Actual native Peachy acceptance

Actual typed input reproduces the endpoint repair/reversal sequence, preserves
unrelated authored sections, and sets Clamp on with Rotate mix **0.5**. The
saved `native-authored/project.peachy` is byte-identical to the final MCP source,
with the canonical SHA256 above. The independent native and MCP input paths
therefore reach the same saved data for this sequence.

Held Mix gestures commit one Undo/Redo; Escape cancels and blocks re-entry while
the pointer remains held, including secondary-button input. The rebuilt final
application also accepts the former coalesced right-edge case, previewing Mix
approximately **0.92** before Escape restores the baseline. Offset X numeric
`12.5`/Undo and a rail drag beyond nominal 50 are verified; cancellation restores
the original value. Owned Mix-field Cmd+S cancels its numeric draft, and a stale
Enter does not revive it. A pending root Rotate **13.5** stays outside Save.

A fresh raw-zero-scale fixture shows SourceMax **blank**. Focusing it and pressing
Enter untouched preserves document and redo. Typing **0**, equal to SourceMin,
repairs SourceMax to a displayed **100** through the same engine operation;
raw-derived f64 readback is approximately `100.00000223517424` after f32 storage.
This is a measured edit, not an invented value on display or an asserted native
default.

The mapping-authoring native executable SHA256 is
`9048ecf435ec6abf840698ad2d930a21016f3faac353179076fb366ac56794ec`.
A fresh public-project open has **one image**, no history and unchanged saved
bytes. At **1120×740**, actual SourceMin, TargetMax, Clamp and Offset Y edits are
reachable through the Properties/table scrolling; each Undo restores the exact
canonical document. The two minimum captures expose source and target sides
separately rather than claiming all columns fit simultaneously.

Actual playback advances from `0.2441903` to `1.0249436` with **Loop enabled**,
then stops at zero with Q. A loop wrap was not observed in this run. All owned
GUI instances are closed. This first mapping run did not establish Tree-pane
resizing; the separate workspace continuation below does.

## Bounded workspace resize continuation

The subsequent native executable
`a8cfabddb25112361a3ffb3464e88e0266d34914873f82be38fb9132fc39c7f6`
verifies actual Tree-divider starts at its center and **−2 / +2** offsets. Each
start resizes the viewport/Tree while preserving document, history and selection.
Properties-divider resizing exposes the full six-row mapping table, and a held
Mix edit followed by Escape restores the canonical document after resizing.
The [full editor capture](../evidence/transform-mapping-editor.png) records this
later workspace state; the minimum captures retain the earlier mapping run.
The exact owned process **41530** is closed and the canonical project remains
unchanged.

This resolves the earlier Tree/Properties resize gap within the tested bounds.
The five automated pane checks do not establish native Weights/Curves divider
behavior; those dividers remain unverified with actual native input.

A separate native run on the same `a8cfabdd` build verifies an original converted
**unweighted mesh**. ArrowRight creates a pending Deform pose; Key then Undo
restores that pending pose. Shrinking Properties preserves pending edits and redo,
and offscreen Key/Delete hit targets are absent. One actual **80-pixel wheel
event** exposes the Key target at Y `565..586`; clicking Key creates the track,
and Delete restores the baseline document. The source file stays unchanged and
owned process **42475** is closed. Weighted and unweighted paths have **26**
focused event checks; this native run covers the unweighted fixture only.

Cross-restart docking and arbitrary workspace behavior are not claimed.

## Exact installed-editor interchange

The exact `actual-mcp-04/editor.json` artifact imports/re-exports through installed
Spine **4.3.26**, with only image-directory metadata changed in a fresh copy.
Our CLI imports the returned JSON and compares the exact canonical MCP source
at **52** poses, retaining the **0.001** geometry tolerance:

| Round-trip comparison | Values | Maximum error |
| --- | --- | --- |
| Bone world transforms | 1,872 | `0` |
| Region coordinates | 832 | `0` |
| UV values | 832 | `0` |

All **416** color values, **52** orders and **104** image identities match. Both
slots are visible in this exact fixture, so attachment sequence covers its full
order; this CLI report does not claim hidden-slot readback.

All raw authored sections, including key times/values/curves, constraint mappings,
region dimensions and transforms, are exact after re-import. The native project
name becomes `input`; JSON default fields are omitted and skeleton/image metadata
is canonicalized. This is not byte identity. Immutable sources and both executable
hashes remain unchanged. The private `installed-interchange-04/report.json` has
SHA256 `bcac0d57b1124c57d57372d24cbcde2ce81494df0968b080f87652a42dac3df2`.
Licensed CLI logs remain private; no broad lossless-conversion claim follows.

## Remaining scope

Properties Match, mapping connection creation/removal, general source/target changes,
remaining cross-property/local/additive domains,
Transform timeline authoring and full styling/docking remain separate work.
Native Weights/Curves divider behavior is not established by the Tree/Properties
resize continuation. Native Deform resize/scroll/key coverage is unweighted;
weighted coverage here is from focused event checks. Cross-restart docking
remains open; loop wrap was not observed during the mapping native run.
Slider behavior beyond the measured paths remains bounded.
No full Spine, pixel or browser-render equivalence is claimed. Hosted CI remains
a separate integration gate.
