# IK animation: Phase B proposal

Status: internally approved for staged implementation, November 6, 2026. Native6
Phase A is the accepted baseline. This contract does not claim accepted native7,
compiled sampling parity or interactive UI parity; those gates remain below.

## Scope and evidence

Add animation and pending authoring for the two existing IK settings: Mix and
two-bone Positive bend. Keep normal one-/two-bone IK and the Phase A evaluated
transform/dependency domain. Softness, compress, stretch, Scale Y modes,
constraint dependency solving, transformed chains and other constraint kinds
remain separate slices. Do not add unused fields for those future features.

The [IK guide](https://esotericsoftware.com/spine-ik-constraints),
[keys guide](https://esotericsoftware.com/spine-keys) and
[4.3 API reference](https://esotericsoftware.com/spine-api-reference.md) establish
keyable constraint settings, authored versus applied poses and constraint pose
values. The older [JSON guide](https://esotericsoftware.com/spine-json-format)
does not establish the current unified setup constraint schema. The exact 4.3
schema/default/curve facts below come from original public-API observations.

Local evidence is retained under
`.local-backups/ik-animation-design-20261106T102719Z-47b79c/`. It contains 61
original JSON files with unchanged SHA-256 hashes, scripts, public-API outputs,
failed arithmetic candidates and a frozen sampling report. These are independent
stimuli, not private artist data. The oracle is the separately licensed npm
`@esotericsoftware/spine-core` **4.3.13**, using original **4.3.26** JSON. No
implementation file, function source or runtime constant was read. Editor import,
reexport and interactive UI remain separate future gates.

Observed facts:

- Setup remains ordered root `constraints`; animation uses
  `animations.<clip>.ik.<constraintName>: [frames]`.
- Before the first key, use setup. Every frame independently defaults omitted
  Mix to 1 and Positive to true. At the exact key those complete values apply;
  omitted fields do not inherit setup or the previous frame. Hold the last key.
- Mix interpolates; Positive holds the left frame until the exact next key,
  including a linear segment. A stepped segment holds both settings.
- Negative Mix and Mix greater than 1 affect both one- and two-bone IK. Zero
  disables solving. A curve can leave 0–1 even when its endpoints stay inside it.
- Equal endpoint Mix values can have a nonconstant curve. Endpoints .5/.5 and
  controls .9/.9 produce approximately .8 at the middle. A normalized value-delta
  bone `Curve` would lose this observed motion.
- A Bezier frame has eight absolute controls: Mix four, then softness four.
  Truncated four-control data produces NaN softness. Nonzero softness controls
  can introduce unsupported softness even with zero softness endpoints.
- Both time handles can lie within their segment while being crossed. Those
  observed curves work; requiring first handle <= second would overrestrict them.
- Source times can collapse after conversion to f32. Reject that collision rather
  than epsilon-merging or silently replacing a source key.

## Native7 data and migration

Keep bone `Property`, `Track`, `Keyframe` and `Curve` unchanged. Add one required
collection to each `Animation`:

```rust
pub struct Animation {
    // Existing fields unchanged.
    pub ik_tracks: Vec<IkTrack>,
}

#[serde(deny_unknown_fields)]
pub struct IkTrack {
    pub constraint: usize,
    pub keys: Vec<IkKeyframe>,
}

#[serde(deny_unknown_fields)]
pub struct IkKeyframe {
    pub time: f32,
    pub mix: f32,
    pub bend_positive: bool,
    pub curve: IkCurve,
}

#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum IkCurve {
    Linear,
    Stepped,
    Bezier { time: [f64; 2], mix: [f32; 2] },
}
```

Follow the existing derive conventions; all shown native fields are required.
Bezier time controls are normalized to the segment, individually finite within
0–1. Mix controls are absolute finite f32 values. Fixed arrays require exactly
two values. Native7 `IkSetup.mix` and stored key Mix accept all finite f32 values.
There is no normalization, range clamp, hidden enabled flag or softness field.

Time handles need f64 storage even though key times remain f32. In original
inputs with keys .25→1, source f32 absolute handle times .4/.775 require normalized
ratios approximately .20000000794728598/.6999999682108561. f32 ratios change
observed samples; f64 ratios reproduce all 102 samples from six additional
originals exactly. This must survive native JSON serialization/deserialization
and Spine absolute-time reconstruction, not only an in-memory test.

Validate known constraint references even for empty tracks, one IK track per
constraint, strictly increasing finite nonnegative times, finite settings/controls
and exact variant fields. Constraints remain append-only; rename retains indexed
track references. Deletion/reordering still requires stable identity or a complete
remapping contract. Native validation retains finite query-invalid authored data.

`Animation::duration()` takes the maximum last bone or IK key time, defaulting to
zero. Empty tracks, an IK-only clip, a single key at zero, tiny fractional clips
and removal/movement of the last IK key use this same rule. Do not add padding,
duration floors, hold keys or reserved-duration state.

Migration must capture the original version. Reject any per-animation `ik_tracks`
presence in formats 1–6, including empty/null, before inserting fields. Keep the
existing legacy root-constraint and shear smuggling guards. Preserve explicit
1→2→3→4→5→6 steps; only literal 6→7 adds empty `ik_tracks` to valid animation
objects. Null/malformed animations must error rather than become objects.

Before relaxing Mix in the latest model, enforce native6's original typed-f32
finite 0–1 setup-Mix contract on migrated data. Do not newly reject an old input
whose source f64 rounded into the valid f32 interval under the native6 decoder.
Latest strict validation still precedes existing v2 hold materialization. The nine
current native6 fixtures change only format and required empty collections; all
existing setup constraints, artwork, keys and complete Spine export bytes stay
unchanged. The formats lane has retained all nine export-byte baselines.

## Sampling and precision

Use a separate pure IK sampler. Absolute Mix avoids a value-span division and
preserves equal-endpoint excursions. Return a transient `IkPose` with `mix: f64`
and `bend_positive: bool`; stored and explicitly edited Mix remain f32. Do not
prematurely round an interpolated Mix before the checked solve.

Observed ten-segment behavior is not an ideal cubic solved at the query time.
The retained independent experiment compares direct Bernstein/Horner, polynomial
and forward-difference constructions, f32/f64 arithmetic, update grouping,
interior-point storage and final rounding. Direct idealized cubic evaluation
incorrectly yields zero for a large cancellation case whose actual Mix is
approximately -7.999997664749159e28.

The matching independently derived model uses standard cubic finite differences:
derive first/second/third scaled derivatives with .3/.06/.006, initialize the
first position difference from velocity + acceleration/2 + jerk*k, then advance
the three differences in f64. The coefficient k was inferred from one original
cancellation observation, then rounded to eight decimal places: **.16666667**.
It was not read from the runtime. Store the nine interior time/Mix point pairs as
f32; keep the supplied endpoints. Interpolate between the bracketing stored points
in f64. Reconstruct absolute time handles in f64 **before** constructing points;
normalizing the query instead changes observable shifted-time behavior.

Calibration comprises 98 samples from 14 inputs: 93 exact matches and maximum
remaining difference 1.11e-16; both large cancellation midpoints match exactly.
Separate held-out evidence has 116 samples across 16 inputs, amplitudes 1→2e38
and a shifted time origin 100000: all match exactly. The later six ratio-control
inputs add 102 exact samples with f64 ratios. Failed candidates and the normalized
time model's .0168932 shifted-time error remain retained. Multiple arithmetic
groupings are observationally indistinguishable here; this is a selected independent
model, not a claim about the runtime's source or a proof for every finite input.

Linear interpolation uses f64 from finite stored endpoints. Stepped and exact
endpoint handling remain explicit. No epsilon test may flatten a small span or
equal endpoints. If point generation/interpolation actually becomes nonfinite or
cannot resolve a required time interval, return a precision error rather than
inventing zero, clamping or silently using an unrelated segment. Absolute-time
point collapse at extreme granularity remains an explicit test/observation gate.

Pass the sampled f64 Mix into the existing normal IK mathematics. Active means
`mix != 0`, including negative values; exactly zero retains FK behavior. Preserve
the existing winding rule and .001 absolute frame/tip forward-reconstruction
guard when solved locals become f32. Large values alone are not an error.
Held-out cancellation inputs with amplitudes 1e8/1e12 produce meaningful poses;
larger examples sometimes cannot retain their output frame after f32 local-angle
rounding. Reject only the actual failed finite/precision condition, with its
constraint and reason. Never widen tolerance, modulo authored angles or clamp Mix.

## Complete authored-pose boundary

A vector of bone locals cannot represent IK-only animation. Introduce an owned
engine capsule with private fields and read-only accessors:

```rust
pub struct AuthoredPose { /* Vec<Transform>, Vec<IkPose> */ }
pub struct IkPose { pub mix: f64, pub bend_positive: bool }

pub fn sample_authored_pose(
    doc: &Document, animation: Option<usize>, time: f32,
) -> Result<AuthoredPose, String>;
pub fn checked_pose_from_authored(
    doc: &Document, authored: &AuthoredPose,
) -> Result<EvaluatedPose, String>;
pub fn checked_scene_from_authored(
    doc: &Document, skin: Option<usize>, authored: &AuthoredPose,
) -> Result<Scene, String>;

Session::authored_pose(EditTarget) -> Result<AuthoredPose, String>;
Session::evaluated_pose(EditTarget) -> Result<EvaluatedPose, String>;
```

`sample_authored_pose` handles Setup or clip sampling. Session overlays both bone
and IK pending values into that same capsule before solving. Validate vector
lengths, settings and finite intermediates centrally. `EvaluatedPose` retains its
authored/applied/world accessors and exposes the authored IK values actually used.
Scene/CLI diagnostics add `ikPoses` alongside `locals` and `appliedLocals`.

Remove or explicitly rename the ambiguous locals-only checked helpers so Animate
callers cannot compile while silently restoring setup IK. An explicit
`checked_setup_pose_from_locals` helper may remain for deliberate Setup-only
testing. `world_pose` remains the explicit FK primitive. No boolean flag or empty
vector means "guess the current IK state".

Migrate CLI pose, Session checked queries, canvas/picking/bounds, selected-pose
queries, transform capsules and weight diagnostics. Geometry uses applied locals
and checked world frames; inspector/key values use authored data. A UI frame may
reuse one evaluated snapshot for its exact document/context. After any command,
pending edit, time/mode/clip or history change, query again; do not maintain an
independent persistent solver cache or reconstruct constraints in panels.

`mesh_bind_pose` still solves the complete current pose once and captures only
applied locals. Bind/Unbind/Add/Direct commands reconstruct that FK snapshot
without solving constraints again. Do not add live IK settings to MeshBindPose.

## Pending authoring, keys and history

Extend existing `PoseEdits` within the single WorkingState/history snapshot with
typed per-property IK overrides. Use separate `IkPoseEdit::Mix { value: f32 }`
and `IkPoseEdit::BendPositive { value: bool }` inputs. Do not retain an entire
sampled IK group when only Mix was edited: the unedited bend must continue to come
from the sampled frame. Duplicate property edits in one payload reject.

Proposed Session API:

```rust
edit_ik_pose(constraint, target, edits: &[IkPoseEdit], auto_key) -> Result<(), String>;
preview_edit_ik_pose(constraint, target, edits: &[IkPoseEdit], auto_key)
    -> Result<(), String>;
key_ik_pose(animation, time, constraint) -> Result<(), String>;
```

Setup editing uses the existing SetIkConstraint command with only requested setup
values changed. Animate with Auto Key off adds pending overrides. Auto Key on or
an explicit IK key snapshots the complete current authored Mix/Positive group,
writes one grouped frame and clears only that constraint's keyed pending group.
Preserve an existing exact frame's outgoing IkCurve; use Linear for a genuinely
new frame. Native/low-level explicit key times stay exact. Session key authoring
uses the existing frame-snapping policy deliberately; changing bone/IK fractional
interactive snapping together is a separate parity decision.

Add strict `SetIkKey { animation, constraint, key: IkKeyframe }` and
`DeleteIkKey { animation, constraint, time }` commands. Reuse exact-first /
unique-near lookup for inspector query/edit paths, including ambiguity errors;
selection operations below always require exact identities. A pure
`ik_keyframe_at` query mirrors the bone helper. Explicitly converting sampled f64
Mix to a stored f32 key must be finite and preserve the current solved frame,
origin and tip within the existing absolute .001 bound against the pre-key
authored pose. A finite cast alone can hide a visible jump. If the checked
reference/candidate cannot be evaluated or the actual difference exceeds that
bound, return an explicit key-conversion precision/domain error with document,
all pending values, transaction baseline and history unchanged. No magnitude
ban, zero replacement, clamp or wider tolerance is permitted.

The current global `key_edited(animation,time,None)` keys all bone **and IK**
pending edits in one candidate state/undo transition. Its existing `Some(bone)`
form remains bone-only; the IK Tree uses `key_ik_pose`. Any key failure preserves
both kinds of pending edits, document, saved baseline and redo. Save/export still
exclude all pending values. Timeline/context discard and recovery apply to both
kinds together, including history snapshots.

Preview methods always start from the transaction's full WorkingState base,
including preexisting bone/IK pending edits. A failed preview leaves the previous
valid preview and original base untouched. Cancel restores exact data and redo;
commit creates one Undo. Setup constraint creation/property UI must report current
checked-domain failures clearly. Query-invalid native data and pending data remain
correctable and must not be discarded, normalized or silently shown as a solved
pose; follow the same checked-query reporting boundary as bone authoring.

An explicit Key action during a live valid field preview must not cancel then
sample the old base. Keep the transaction open, key the current authored preview,
then commit once. Invalid/unparsed text blocks the action with a visible error;
never key a stale last-valid display value. Save/Open/context actions still cancel.
Suppress canceled held-pointer responses through release. This is a proposed
coherent action policy, with real native interaction still to verify.

The shell needs explicit Pristine/Valid/Invalid field state; a `changed` response
or retained last-valid number does not establish that the current text parses.
Key intent may wait until the owned field has processed its text/blur/release,
while the same Session transaction remains open. Passive display, focus and blur
must retain wide sampled Mix and imported finite values outside the slider range.
Only an explicit valid author/key action performs checked f64-to-f32 conversion.
Do not normalize an imported value by rendering the existing bounded slider.

## Typed Dopesheet and clipboard reuse

IK Mix/Positive is one grouped key identity, not fictitious bone scalar properties.
Introduce a strict tagged `AnimationKey` enum:

```rust
Bone { bone: usize, property: Property, time: f32 }
Ik   { constraint: usize, time: f32 }
```

Use a matching `KeyOwner::{Bone { bone }, Ik { constraint }}` for selection/context
and paste targets. Bone index 0 and constraint index 0 are different owners;
switching between them clears selection and expansion phase. Keep existing bone
API types as thin compatibility wrappers around the same internal resolver and
timing algorithm, not a second copy of retiming logic.

Add `copy_animation_keys`, `MoveAnimationKeys`, `DeleteAnimationKeys` and
`PasteAnimationKeys` using typed identities/targets. The tagged clipboard is either
`Bone { keys: Vec<ClipboardKey> }` or `Ik { keys: Vec<IkKeyframe> }`; copied times
are relative and anchored at zero. Copy supports one owner at a time, rejects
mixed/multiple owners and owns its payload. Same-kind cross-constraint paste keeps
absolute settings and curve data. Wrong-kind paste rejects rather than remapping.

Reuse exact source snapshots, simultaneous moves, exact destination replacement,
collapse detection, finite nonnegative target checks, fixed-base previews and
one-step undo. Preserve curves and both grouped settings. Nearby fractional keys
never epsilon-merge. Clipboard payloads receive full validation even when they
come from hand-crafted serialized commands. The OS envelope writes an explicit
new version/kind; a strict old bone-only envelope can be translated into the new
Bone variant. Unknown/malformed payloads never fall back to cached unrelated keys.

Reuse current action ordering, shortcut latches, text/modal priority, drag
cancellation and chart/extent behavior. A separate IK context must not redirect a
bone tool or unrelated Tree selection through a cached index. The public guide
supports grouped IK keying, but exact current Dopesheet row labels and IK-specific
L/Cmd-L behavior remain observation gates; do not invent independent Mix/Bend rows
or an active transform-tool shortcut mapping.

## Adapter and potential-activity policy

Decode each source frame's independent defaults; accept only supported known
settings. Read all eight Bezier coordinates before conversion. Unsupported
softness endpoints and softness Y controls must be exactly source-numeric zero,
including values that would underflow to f32 zero. Both time-control pairs must be
finite and within the source segment; no ordering requirement. Compression and
stretch must remain typed false/absent. Preserve the existing strict unknown-field,
alias and setup-feature gates even when Mix is zero.

Import nonfinal curves into f64 normalized time and absolute f32 Mix controls
with explicit representability checks. Do not reuse bone delta/epsilon curve
conversion. A source final-key Bezier has no segment for normalization: retain
the existing bounded import refusal after clear diagnostics. Raw native outgoing
curves on final keys remain legal/inert; export omits them. This does not claim
raw final-curve metadata roundtripping. Export every IK key's absolute time, Mix
and Positive; nonfinal Bezier writes eight controls with reconstructed absolute
Mix times plus neutral softness controls. Omit empty animation `.ik` so all nine
old export bytes remain unchanged.

Preflight Setup and each clip separately; do not union unrelated clips into fake
dependency conflicts. Setup is potentially active when setup Mix != 0. In a clip,
an absent/empty track inherits setup; a first key after zero retains a setup
interval; a key at zero overrides it for that clip. Any reachable nonzero key or
nonfinal absolute Bezier Mix control makes the constraint potentially active,
including negative values and zero-endpoint excursions. Ignore inert final curves.

For potentially active constraints, retain the current exact supported transform
restrictions and require relevant bone scale/shear/child-offset tracks to remain
identically neutral. Within each clip, potentially active constraints must have
independent subtrees/targets. This deliberately refuses some observed never-coactive
sharing and setup-scale cancellation by constant animation. Those are documented
later domain extensions, not silently solved by sampling a few poses. The dynamic
checked solver inspects actual sampled/pending values and can evaluate native
times with only supported active constraints. Export preflight does not promise
every intermediate numeric pose is representable.

## Acceptance before a parity claim

- Strict native7/command/clipboard serde, legacy typed range and presence guards;
  all nine empty-IK migration/export byte goldens; no artist inputs changed.
- Nondefault setup, omitted reset defaults, fractional first key, adjacent-close
  keys, exact bend boundaries, linear/stepped/Bezier holds and IK-only duration.
- Equal-endpoint excursion, negative/overshooting Mix, absolute-time quantization,
  .25→1 ratio serde/Spine roundtrip, large linear and cancellation originals;
  actual frame precision errors remain explicit at the unchanged .001 bound.
- Full sampled/pending capsule through CLI, rendering, target tools and weighted
  Bind/Direct; half-mix never solved twice. IK-only clips must differ despite
  identical bone locals. Invalid pose errors preserve authored/pending data.
- Mixed bone/IK pending, grouped Auto Key, explicit key during a valid preview,
  invalid text, global atomic K, one Undo/cancel/redo and pending-excluding saves.
- Exact grouped move/copy/cut/delete/paste, cross-constraint semantics, wrong-kind
  rejection, overwritten-key restoration, typed context reset and source hashes.
- Setup0→animated activation, negative activation, zero-endpoint nonzero controls,
  independent clips, conservative dependency refusal and strict advanced fields.
- Separate installed-editor JSON, licensed runtime, actual native control/layout
  and keyboard/gesture checks on original fixtures. Passing the data gates alone
  does not establish interactive Spine parity or full production-rig support.
