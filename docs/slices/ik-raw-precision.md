# IK raw values and evaluated frames

Status: internally approved correction to the unaccepted native7 slice,
November 6, 2026. Implementation and combined acceptance remain pending.
This supersedes the numeric representation and IK clipboard timing paragraphs
of `ik-animation-design.md`; its other architectural and UI requirements remain.

Original public-API observations expose two different precisions. Setup Mix stays
f64. Timeline frame times and frame Mix are f32, but Bezier construction consumes
the original f64 key times, key Mix and both control coordinates before storing
interior polyline points as f32. Linear/stepped/exact-key evaluation uses the f32
frames. Bezier interpolation anchors its first/last segment to those same frame
values, while its nine interior points come from raw f64 coordinates.

The independent 16-model experiment uses 48 original inputs and 528 observations.
Only the model retaining all four raw coordinate axes matches every observation
exactly. Setup controls separately retain .37 and 100000001 exactly. Earlier
compiled examples demonstrated visible loss from rounding control Y or key Mix,
and eight native/export comparisons demonstrated loss from rounding written X.
The separately licensed public runtime API was executed; no implementation was
read. Evidence stays outside Git under the root integration's `raw-semantics-root-01`
and the independent `ik-export-time-fidelity-20261106T115357Z-3d95ef` records.

Use f64 for `IkSetup.mix`, `IkKeyframe.time`, `IkKeyframe.mix`, and both axes of
`IkCurve::Bezier`. Keep normalized f64 time handles relative to the raw key span.
Add explicit `IkKeyframe::frame_time() -> f32` and `frame_mix() -> f32` helpers.
Bone tracks and transforms remain unchanged. Require finite raw values, finite
nonnegative frame times and strictly increasing distinct frame identities. Never
merge raw times that collapse into one f32 frame. Sampling reports any genuinely
nonfinite frame value or polyline result explicitly.

The sampler uses raw key/control coordinates to construct points, then replaces
the two polyline endpoints with canonical frame time/Mix. Setup bypasses frame
rounding. The existing winding and absolute .001 solved-frame/tip checks remain.
Export writes raw keys and reconstructed f64 absolute handles without a narrowing
cast. Import preserves those raw values and checks raw handle bounds first.
Retain the old native6 typed-f32 Mix range check; migration promotes that old
typed f32 value to f64 rather than reinterpreting the legacy spelling as new data.

`Animation::duration`, `AnimationKey::Ik`, selection, navigation and inspector
lookup use canonical f32 frame identities. Exact SetIkKey replacement addresses
that identity while storing the explicitly supplied raw key payload. Existing
fractional key authoring resolves its actual identity before FPS snapping and
retains its raw time. New interactive keys retain the established FPS policy.
Interactive Mix edits remain explicitly typed f32 edits, promoted to f64 storage;
passive display must preserve wider imported values. Keying still checks the
actual current frame before and after storing canonical authored key values.

For explicit timeline movement, calculate the destination from the canonical
frame and the existing f32 offset. Preserve raw time for a zero move; otherwise
store that deliberately chosen destination frame as f64. Relative Bezier handles
stay unchanged. This gives the command and UI the same exact destination IDs.

IK clipboard data becomes `Ik { origin: f32, keys: Vec<IkKeyframe> }`, retaining
the complete raw source keys. Origin is the minimum canonical source frame.
Validate that anchor, raw data, unique canonical identities and all curve fields.
Paste translates canonical source frames by `target - origin` through the same
movement helper. A paste at the same origin preserves raw times; an explicit
translation adopts its chosen destination frames. Bone clipboard data and the
legacy bone command wrappers keep their existing semantics. Version the OS
envelope explicitly and continue requiring the current editor's exact mirror.

Before accepting this correction, rerun original raw-axis, canonical, held-out,
nearby-frame, migration/export-golden, retiming, clipboard and key-continuity
checks, then the full workspace, packed-runtime and real native-input gates.
F64 storage alone is not acceptance evidence. Do not hide loss by clamping,
discarding raw data, increasing tolerances or rejecting ordinary noncanonical
numbers simply to fit the previous f32 model.
