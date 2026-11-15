# Slider constraints — November 15, 2026

**Verified bounded manual/MCP checkpoint; full Slider parity remains OPEN.**
Native format **23** and the shared data, authoring, adapter and evaluation paths
are implemented; MCP exposes **58 commands / 14 tools**. Actual MCP recreation,
bounded native editing, fresh public reopen on the final skin-aware build and
external geometry checks pass. Installed interchange passes returned geometry
within tolerance while its strict authored report fails. Two adversarial precision
failures and the feature gaps below remain open.

See the [review gallery](../evidence/slider-constraints.html),
[scalar evidence](../evidence/slider-constraints.json) and
[original Slider Lab](../../examples/peach-bot/slider-bot.peachy).

## References and observed native controls

The official [4.3 release](https://esotericsoftware.com/blog/Spine-4.3-released)
introduces manual and bone-driven animation application. The
[Sliders guide](https://esotericsoftware.com/spine-sliders) supplies creation and
control terminology; the [4.3 API](https://esotericsoftware.com/spine-api-reference#SliderData)
distinguishes stored data, authored pose and applied pose. Sources were retrieved
through agent-reach on November 15. Current wire details and arithmetic come from
original inputs/outputs, not assumptions based on older guide prose.

Permitted installed **Spine 4.3.26** observations use an original six-bone,
two-region rig and one original 128×128 blue cutout. Selecting an animation and
using New → Slider creates the constraint immediately. Frame **30** at 60 FPS
exports setup `time:0.5`. Bone selection replaces the manual Frame row with
property/range controls and a derived Frame readout. The six source choices are
Rotate, Translate X/Y, Scale X/Y and Shear Y; this differs from the seven keyed
bone channels, which also include Shear X.

The original follower/Rotate selection uses Local, source 0..100 and Frame
0..90, exporting `max:1.5` and `scale:0.015`. Source Min 10 exports `from:10`;
target Frame Min 20 exports `to:0.3333333`. These are measured cases, not universal
creation defaults. Mix accepts the observed −50%, 50% and 150% values.

Independent host Time/Mix key buttons produce separate scalar timelines. With
Auto Key off, pending Frame 45/Mix 25% changes stay outside Save until explicitly
keyed. An earlier filename suggesting pending work actually captured Auto Key
on and a saved key; that attempt is classified accordingly, not counted as Save
exclusion evidence. Assigning a bone removes Time timelines while preserving Mix
timelines. Clearing a bone retained the current derived time in the observed
sequence, rather than the earlier manual time; its semantic transition remains
open below.

Native Graph displays Time values as Frames: reference seconds 0.5..0.75 appear
as Frames 30..45, while host seconds 0..0.25 span Frames 0..15. An actual Bezier
export stores `[0.08333333,0.5,0.16666667,0.75]`: absolute host-time and referenced-
time coordinates. A failed Tree/Graph click was discarded through native reload;
its capture is excluded. Licensed reference captures remain private.

## Data, migration and interchange

`Constraint::Slider` stores an animation index, optional source bone/property,
Local/Loop/Additive, independent `from`/`to`/`scale`/`max`, and setup Time/Mix.
Each animation requires `slider_tracks`, with separate absolute Time and Mix
tracks. Source references resolve by animation name before ordered constraints
load; timeline owners resolve by constraint type. Names shared by different
constraint types do not alias. Named skins use the observed `slider` membership
list and a constraint's `skin:true` flag.

The original source-only cases also retain a named skin's Slider list after the
Slider stops being skin-required. This redundant membership is accepted and
preserved as raw metadata; it does not limit that Slider's activity in other
skins. Duplicate/unknown references, unflagged non-Slider membership, unflagged
bones and default-skin membership remain rejected. The narrow validation and
serialization fix passes the focused adapter regression and the affected
101-case/21-case matrix reruns below.

The native v22→v23 decoder inserts empty Slider-track lists **in memory**. Legacy
files reject injected Slider constraints/tracks, including an explicitly supplied
new track field. Current native payloads require complete strict fields. Historical
public `.peachy` examples keep their original bytes; focused tests upgrade frozen
fixtures in memory instead of rewriting them or weakening deserialization.

The Spine wire uses `constraints[].type:"slider"`, an animation name and
`animations.<host>.slider.<constraint>.time|mix`. Omitted setup Time is **0** and
Mix **1**. Omitted key `value` is **1 for both Time and Mix**; an explicit zero
must survive. A source bone requires a property. Inactive range fields without
a bone remain raw data through our adapter, even when the consumer ignores them.
Imports do not normalize those fields or perform editor source-switch actions.

## One authored/effective evaluation

The engine builds an authored capsule once, then applies constraints in list
order to one effective state. Each Slider freezes its current Time/Mix at its
turn and samples only the referenced tracks that have started. Missing or
before-first channels preserve incoming state. It does not evaluate a separate
Setup-filled pose or recursively solve the referenced animation.

Earlier writes affect later constraints. Writing IK/Path settings or another
Slider after it has already executed updates its effective settings without
running it again. Changed bone channels/inheritance join one FK refresh. Authored
state, pending edits and saved data remain separate from evaluated output.

Let `c` be incoming, `s` Setup, `v` the referenced scalar and `a` Mix. The observed
bone rules are:

- Translation/rotation/shear: replacement `c + a*(s + v - c)`; Additive
  `c + a*v`. Rotation retains winding, including ±180 and ±720.
- Scale Additive: `c + a*(v - 1)*s`. Replacement targets `s*v`; full Mix assigns
  that target directly. Otherwise it mixes from `abs(c)*sign(target)`, retaining
  the measured signed-zero behavior. Exact zero Mix preserves incoming values.

Other calibrated domains use these rules:

| Domain | Replacement | Additive |
| --- | --- | --- |
| Slot RGBA | Clamp `c + a*(v-c)` to 0..1, without byte quantization | Same replacement rule |
| Plain/weighted Deform offsets | `c + a*(v-c)` | `c + a*v` |
| IK Mix | `c + a*(v-c)` | Same replacement rule |
| Path position | `c + a*(v-c)` | `c + a*v` |
| Path spacing | `c + a*(v-c)` | Same replacement rule |
| Another Slider's Time/Mix | `c + a*(v-c)` | `c + a*v` |
| Attachment/hide, draw order, inheritance, IK Positive, Sequence | Held key for the tested nonzero signed Mix values; zero preserves incoming | Same discrete selection |

For unweighted meshes, public runtime Deform arrays contain absolute local
vertices; our dense offsets subtract Setup vertices. Weighted arrays contain
per-influence offsets. Mixing happens in offset space. Distinct incoming values
and distinct two-Slider targets resolve the replacement/Additive and ordering
rules; coincident-target probes alone were not treated as proof.

Inactive referenced bones, their slot domains, and inactive IK/Path/Slider
targets skip writes. Global draw order still applies to the complete slot list,
including inactive/hidden slots. An inactive source bone skips the Slider itself.
Skin-aware ordinary-host sampling now also skips keys on inactive slots. Its fix
is verified by the separate 177-case Sequence/inactive-target matrix below;
inactive slot data is compared without exclusions.

## Bone-driven time and Sequence limits

Source properties read the live applied local transform or world frame at this
ordered turn. World scale uses axis magnitudes; world rotation and Shear Y use
the measured axis-angle conventions, including reflection and signed-zero cases.
The source mapping is `(property - from)*scale + to`, in referenced seconds.
Without Loop it has a lower bound of zero; `max` is retained editor range metadata
and is not an evaluation upper cap. With Loop, the bone-derived effective time
uses the observed pre-wrapping convention and sampling applies the final modulo.
Manual effective Time stays raw and uses signed remainder when looping. These
paths never overwrite the authored Time. Bone-driven Loop on a zero-duration
reference is explicitly refused because the observed consumer produces NaN.

Sequence uses the shared seven-mode frame rule with a wide derived query and a
wide key-selection gate. Before the first key it preserves the host's frame;
without a host key it preserves Setup fallback. The three-frame study uses
count 3/start 1/digits 2/setup 1, with host index 2 and reference index 0. Distinct
atlas names alias the **same original texture**; index/image identity is measured,
not raster appearance. Additive does not change the observed discrete selection.

Checked advancing phase is limited to a nonnegative counter through **i32::MAX**.
The external Loop counter at Time 2^30/delay 0.25 produces raw index −1 and Setup
fallback, rather than ordinary unsigned modulo. Unsupported counter phases are
refused; no wrap or saturation policy is invented. Hold bypasses advancing-phase
arithmetic. These refusals are not runtime passes or support for extreme phases.

## Shared authoring and MCP

The current command family includes `addSliderConstraint`, the complete raw
`setSliderConstraint`, `setSliderKey`, semantic `insertSliderKey` and
`setSliderCurve`. Typed Slider Time/Mix keys participate in shared deletion,
retiming, clipboard and curve operations. Raw constraint replacement is distinct
from the still-open state-dependent source-switch/clear actions.

Revision-checked `get_slider_pose` returns authored Time/Mix at a host cursor;
it does not substitute bone-derived time. `evaluate_pose` returns
`authoredSliderPoses` and `appliedSliderPoses` from the same checked solve that
provides world transforms, slot domains and geometry. Arrays use the complete
ordered constraint list, with inactive placeholders for other types. MCP is an
independent headless Session, not control of an already open desktop window.
The retained actual-stdio recipe, bounded native/fresh-reopen results and lossy
installed interchange are recorded below. Broader authoring and compatibility
obligations remain open.

## Evidence at this checkpoint

External observations use separately installed official **runtime 4.3.13** through
its public API. Input/parsed skeleton version **4.3.26** identifies the data, not
the runtime package. No editor/runtime implementation source was inspected.
Original recipes retain immutable input, script and executable hashes privately.

A focused checkpoint passes **9** adapter Slider, **39** engine Slider, **18**
desktop Slider, **14** schema and **2** actual-stdio cases. These are focused
checks, separate from the actual recreation and native interactions below.
Formatting, all-target Clippy, workspace build and architecture checks pass.
Final 170/177/101/21-case reruns complete after the skin-aware host fix.

The validated-cache `domain-02` engine matrix compares **170/170 cases** at the
unchanged **0.001** tolerance, maximum absolute error **4.81771965e−5**. Reconstructed
FK from engine-applied locals has **zero residual**; sources remain unchanged.
This matrix is a completed checkpoint, not acceptance of every later expansion.

An earlier report retained **12 Path applied-local metadata mismatches** while
WORLD already matched. The public
[BonePose validation contract](https://esotericsoftware.com/spine-api-reference#BonePose-validateLocalTransform)
requires local-cache validation after world/constraint edits before reading those
locals. Versioned observer v6 calls that method before each post-world snapshot.
Pre/post WORLD bits, including signed zero, remain exactly equal; no second world
update or solver runs. Correct reads resolve the 12 metadata comparisons without
changing world tolerance. Earlier reports/observers are preserved as history.

Other original observation sets include the 34-case bone study, 101 discriminating
incoming-state holdouts, 21 other-Slider/inactive-source cases, and 177 Sequence/
inactive-target cases. Their observation counts are not a sum of final engine,
MCP or native acceptance passes. The 177-case set contains 113 exact Sequence
index/image checks and 32 inactive-target comparisons against matching zero-Mix
contexts, with the remaining cases supplying active controls.

After the skin-aware host-sampling fix, the retained
`sequence-04/engine-matrix-skin-02.json` compares **177/177 cases**, with **zero
mismatches** at the unchanged **0.001** tolerance and maximum absolute error
**6.848346799870342e−6**. All cases preserve Setup and pending work and repeat
exactly across three evaluations. Engine-applied FK residual is **0**. No Setup
substitution is used; incoming Path settings are represented by exact host keys.
The comparison covers observed metadata, effective Sequence frame and active
world matrices. Inactive runtime world caches are uncomputed; their activity and
local values, all-engine FK consistency and inactive slot data remain checked.
These reports contain no artwork vertices/UV output or event log, so this result
does not establish raster, mesh/region geometry or event-delivery acceptance.
Sources remain unchanged. The report SHA256 is
`41fe617e1edf08fcea59f52f266f01a3dd13df3b46923e732489503a2805e4fd`.

The subsequent reruns keep successful comparisons separate from guards and
unresolved failures:

| Study | Compared | Other outcomes |
| --- | --- | --- |
| Discriminating incoming-state holdouts | 101/101 | None |
| Other-Slider/Additive and inactive-source cases | 21/21 | None |
| Bone-source holdouts | 72 | 3 explicit Loop precision refusals |
| Source Mix / constraint order | 6/6 / 6/6 | None |
| Original bone / nested Slider studies | 34/34 / 3/3 | None |
| Driven Time / driven Mix | 12/12 / 12/12 | None |
| Manual adversarial study | 11 of 13 | 2 raw `1e30` f32 precision mismatches |
| Sequence wide-time study | 11 of 15 | 4 counter-range refusals |
| Sequence integer-boundary study | 8 of 16 | 8 counter-range refusals |

The two manual precision mismatches remain **failures**, not passes or declared
refusals. Geometry tolerance stays **0.001**. Refused counter/Loop cases are also
not counted as runtime passes. Counts describe these individual studies and are
not an aggregate claim that every selected Slider pose evaluates.

## Actual MCP recreation and bounded native editing

The original [Slider Lab](../../examples/peach-bot/slider-bot.peachy) is authored
from a blank project and one original PNG through **84 actual MCP requests**,
retaining **14** pose samples. Final `actual-mcp-02` repeats the same request
count and canonical project SHA256:

```text
f097c00c4542d46b7e2ee846d72d74606c9a6e5fad9bf41ccc1dcdc277a60dc6
```

Actual native Peachy input on that existing rig verifies independent Time/Mix
key controls and K, pending Save exclusion, Undo/Redo and Auto Key flush; Frame
120 beyond the current end; signed Mix; Setup Loop/Additive; owned numeric-draft
Save; and held Escape/focus-loss cancellation. Graph Frame/percent numeric
editing, commit/value cancellation and Copy/Paste/Delete pass. Controls also pass
the **1120×740** interaction checks.

Native edits return to the MCP canonical document. This study edits/restores an
MCP-authored rig; independent native rig creation remains unverified.

The final native build
`47ca392adf56a9881bf54b845656fcc01af390aa7e52fff88b87d2038a12bde8`
opens the public Slider Lab with **one image**, **empty history** and Graph handles
visible. Actual playback reaches **0.224702 seconds**, then stops/resets to **0**.
The document and canonical SHA remain unchanged, and the owned window is closed.
This verifies fresh public reopen on the final skin-aware build; it does not
establish a loop wrap or independent native creation.

The authored rig's separate external geometry check covers **104 poses** at
unchanged **0.001** tolerance. Maximum world error is
**1.838038693335875e−5**; **1,664** region-coordinate values have maximum error
**2.288818359375e−5**. These geometry results do not close the two adversarial
precision failures or raster/styling comparison.

## Installed-editor interchange: geometry passes, authored data changes

Installed **Spine 4.3.26** import/re-export compares **104** returned poses at the
unchanged **0.001** tolerance. **3,744** world and **1,664** region-coordinate values
have maximum error **1.52587890625e−5**; **1,664 UV values are exact**. The report
also checks **832** color values and **104** draw orders.

The retained strict authored report has **passed:false**. Native output writes
Mix tracks before Time tracks and recanonicalizes Beziers. One inserted Time
value changes from `0.61266667` to `0.6126666`; normalized handle drift is about
`6.3e−7`. All **10 non-animation sections** remain exact after reimport.

This is bounded **lossy Editor compatibility within tolerance**. It does not
establish exact authored interchange or lossless conversion. The failed strict
report and separate semantic analysis remain retained, and neither the adapter
nor tolerance is weakened to make the strict report pass.

## Open acceptance and feature obligations

- Native-equivalent source switching, state-dependent clear, and creation that
  preserves the observed prepend/order and identity relationships.
- Physics settings/resets and events through referenced animations; no simulation
  or event-firing claim follows from existing data-only support.
- Linked/path/clipping Deform and attachment switches with incoming Deform;
  hidden/different shown attachment cases remain explicit evaluation guards.
- Remaining Sequence attachment/mismatch cases, larger counter behavior and full
  source/order/cycle/skin coverage beyond the completed matrices.
- Independent native creation and remaining full-feature/matrix gates. Strict
  authored interchange retains the documented failure; the two manual raw `1e30`
  precision mismatches remain open. The verified bounded manual/MCP checkpoint
  does not establish full runtime, lossless conversion, raster or CI parity.

Private licensed captures, account paths and artist data are not included here.
