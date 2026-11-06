# Normal IK: Phase A contract

Status: internally approved for implementation, 2026-11-06. Isolated core, format
and desktop lanes are in progress. This contract does not claim accepted IK or
interactive UI parity. The accepted main application remains native v5 until the
combined native6 slice passes its integration gates.

## Scope and evidence

Phase A supports static, normal, one- and two-bone IK constraints, with existing
bone animation driving their targets. Constraint mix and bend direction are setup
settings. Constraint animation, softness, compression, stretch, Scale Y modes,
dependent constraints and transformed chains are later slices. Unsupported input
must fail clearly rather than become a different default IK constraint.

The [IK guide](https://esotericsoftware.com/spine-ik-constraints) establishes the
one-/two-bone use, immediate-child and target restrictions, target selection and
constraint properties. The [constraints guide](https://esotericsoftware.com/spine-constraints)
establishes ordered evaluation, Tree placement and annotations. Its older Uniform
and Essential wording is not a version-pinned 4.3 UI reference. The
[4.3 API](https://esotericsoftware.com/spine-api-reference.md) explicitly separates
authored `pose` from solved `appliedPose`, has one unified constraint collection,
and describes three Scale Y modes.

Original black-box JSON, public runtime API observations and failures are retained
locally under `.local-backups/ik-discovery-20261106T083108Z-084144/` and
`.local-backups/ik-design-20261106T085459Z-d4be93/`. No runtime implementation,
function source, private artist input or editor project internals were read.

The separately licensed npm oracle is `@esotericsoftware/spine-core` **4.3.13**;
JSON/editor observations are **4.3.26**. The
[official npm registry](https://registry.npmjs.org/@esotericsoftware%2Fspine-core)
reports 4.3.13 as latest, published July 24, 2026. Runtime and editor patch numbers
are different schemes; neither patch-number comparison nor publication dates
identify which individual fixes are shared. No dependency installation/change was
performed. The [editor changelog](https://esotericsoftware.com/spine-changelog/archive)
and [staff reply about Essential export](https://esotericsoftware.com/forum/d/30820-4326-essential-binary-export-silently-drops-ik-constraints-json-keeps-them)
also show version-specific IK changes; JSON and interactive behavior remain
separate acceptance gates.

Observed 4.3 data facts:

- Setup uses `constraints: [{"type":"ik",...}]`. Legacy root `ik` parses but
  creates zero constraints. Animation still uses `animations.<clip>.ik.<name>`;
  guessed animation `constraints` objects create no timelines.
- Array order is behavioral. Reversing two half-mix constraints changes the result
  even when numeric `order` fields retain their previous values. Editor 4.3.26
  preserves array order and removes those numeric fields on reexport.
- `scaleY:"uniform"` and `scaleY:"volume"` work. `uniform:true` and
  `scaleYMode:"uniform"` are ignored. Phase A rejects those aliases, rather than
  interpreting them as supported settings.
- IK mix/softness frames use absolute settings and two independent curves. Eight
  absolute controls are mix-four then softness-four. Truncated four-control data
  produces NaN softness in the oracle and is refused by editor import. Phase A
  refuses IK timelines entirely; these facts constrain the later timeline slice.
- The guide's parent-shear limitation disagrees with the original sheared-chain
  observation. Editor reexport retains authored shear. Phase A refuses that
  active transform domain; it never zeros authored shear to manufacture a solve.

Root's installed-editor gate preserved constraints in all ten positive/order/
settings cases, retained IK timelines in both animated cases, and rejected the
truncated-curve control. The legacy-root control lost its nonexistent constraint
as expected. All twelve sources stayed unchanged. Public-API before/after pose
comparisons of the eleven successful reexports match exactly at five times. This
does not measure an interactive editor pose or imply Phase A supports all those
settings/order cases.

## Required native v6 data

Add only required `Document.constraints: Vec<Constraint>`. Leave `Animation`,
bone `Property`, `Track`, bone key clipboard and bone key identity unchanged.

```rust
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Constraint {
    Ik(IkConstraint),
}

#[serde(deny_unknown_fields)]
pub struct IkConstraint {
    pub name: String,
    pub bones: Vec<usize>,
    pub target: usize,
    pub setup: IkSetup,
}

#[serde(deny_unknown_fields)]
pub struct IkSetup {
    pub mix: f32,
    pub bend_positive: bool,
}
```

Use existing derive conventions. Both setup fields are required in native data
and commands; do not use blanket serde defaults. There is no invented `enabled`
flag, stored numeric order, unused future timeline collection, or hidden advanced
setting. Mix zero disables the effect. A later constraint-timeline/schema addition
requires an explicit later native version.

Structural validation requires a unique nonempty constraint name, finite mix in
`[0,1]`, one bone or two distinct bones in immediate parent/child order, valid
references, and a target that is neither a constrained bone nor a descendant of
the first constrained bone. Constraint list order is retained. Native validation
does not rewrite finite bone transforms or bake solved locals into setup/keys.

Constraints are append-only in this slice. Array indices identify them while no
deletion/reordering exists. Stable constraint identity or a complete remapping
contract must precede deletion/reordering and later timeline references. The
existing bone-index identity policy is unchanged.

## Exact evaluated domain

For every active constraint (`mix > 0`), Phase A accepts:

- Normal inheritance, with a real parent for the first constrained bone.
- Unit positive local scales (`scale_x == scale_y == 1`) and zero local shear on
  the constrained bones and all their ancestors. Local translations and finite
  rotations are retained. Positive uniform scale other than one, reflection,
  nonuniform scale, shear and root-constrained chains are separate domains.
- For two bones, positive child-local X equal to its setup value, exactly zero
  child-local Y, and positive
  child bone length. The first link length comes from the child origin offset,
  **not** the first bone's display length. The original child-X7/parent-length99
  case reaches `(7,10)` with a length10 child. A first bone's display length may
  be zero without making that positive link degenerate.
- A target position distinct from the first constrained bone's origin. One-bone
  length zero is accepted when this direction exists. Coincident targets,
  zero-length second links and zero child offsets are explicit temporary errors;
  no arbitrary pole direction or equal-weight fallback is invented.
- Reachable chains and ordinary noncoincident outer/inner unreachable targets.
  Standard geometric reach limiting straightens/folds the links to their nearest
  attainable distance. Original unequal links7/10 stop at17 for target30, and at3
  for target1. This geometric reach limit does not rewrite authored angles.
- Multiple active constraints only when their affected descendant subtrees are
  disjoint and no target is inside another active constraint's affected subtree.
  Shared-bone, ancestor/descendant and cross-target dependency lists are refused.
  Their noncommuting order probes are retained for a later scheduling slice,
  **not** presented as supported Phase A solver cases.

Unrelated FK branches and descendants not themselves constrained keep existing
transform support, including weighted geometry. Do not reject an unrelated zero
scale merely because an IK constraint exists elsewhere. Mix zero skips solving
and preserves exact FK output; structural/source-feature checks still apply.

Checked queries inspect the actual authored pose, including animation and pending
edits. Domain errors return `Err` with the constraint/bone and reason. They never
clamp scales, clear child Y/shear, discard pending values, replace the pose with FK,
or corrupt history. Native save/load retains raw finite authored data even when a
particular pose lies outside this evaluator's current domain. The UI can show an
evaluation error while preserving that data for correction.

The Spine adapter also preflights source features: active constrained/ancestor
scale/shear tracks must be identically neutral; the two-bone child offset must be
constant with positive X/zero Y. Non-neutral or varying unsupported channels are
refused before import. Other bone tracks, including target translation and
ordinary rotation, stay in the existing bone-track model. Numeric representability
remains a checked-query boundary, as it is for large FK/Bezier data; finite stored
keys alone do not promise every intermediate pose is representable.

### Winding and precision

Original unit cases vary authored angles through `±181`, `±359`, `±720` and
`±1080`, with mix `.25/.5/.75`, on one bone and separately the two-bone parent
and child. Eighty-seven inputs match the following independently inferred rule:
compute desired-minus-authored in f64, add360 **once** if the delta is `<= -180`,
otherwise subtract360 **once** if it is `> 180`, then apply the mix. Fifty-one
boundary cases and eleven east-target cases establish the positive180 tie.
Repeated modulo-to-shortest-angle is falsified: it differs in36 comparisons, by
up to540 degrees. For example, authored720/desired90/mix.5 yields about585.

These are input/output mathematical observations, not an inspected algorithm.
Preserve raw authored angles; no normalization/clamp or arbitrary angular maximum
is introduced. Use the existing independently calibrated basis coefficient and
f64 solver intermediates. The tested range is evidence, not a proof for all huge
angles or transformed chains.

Before publishing an applied pose, require finite f32 results and a forward
reconstruction bound of **0.001 absolute per coordinate**, without a relative
exception. Compare the f32 reconstructed affected world frame/origin/tip against
the same solved f64 pose before local rounding, using the incoming parent frame
as the common reference. Check frame components as well as tips so zero-length
bones cannot hide angular loss. No-op/mix-zero paths preserve original bits.
If that representation would move a frame/tip beyond the bound, return a clear
precision error and leave document, pending pose and history unchanged. This
does not claim a universal attachment-error bound for arbitrarily large vertices;
checked scene geometry and existing binding residuals remain separate boundaries.

Applied-local native-f32 FK reconstruction has maximum residual `5.73e-7` over
the149 original winding/tie cases. Failed modulo predictions remain in evidence.
Implementation tests must include accepted ordinary/large exactly recoverable
values and rejected genuinely lossy cases; the guard must not become a blanket
large-coordinate or small-scale ban on unrelated branches.

## Checked pose and Scene API

Keep `local_pose`/`Session::local_pose` authored and `world_pose` an explicitly FK
primitive. Add one pure checked constraint stage:

```rust
pub struct EvaluatedPose { /* private authored, applied and world vectors */ }
impl EvaluatedPose {
    pub fn authored_locals(&self) -> &[Transform];
    pub fn applied_locals(&self) -> &[Transform];
    pub fn world(&self) -> &[Affine];
}
pub fn checked_pose_from_locals(
    doc: &Document,
    authored_locals: Vec<Transform>,
) -> Result<EvaluatedPose, String>;
pub fn checked_evaluate_pose(
    doc: &Document,
    animation: Option<usize>,
    time: f32,
) -> Result<EvaluatedPose, String>;
impl Session {
    pub fn evaluated_pose(&self, target: EditTarget) -> Result<EvaluatedPose, String>;
}
```

Validate local count/finiteness, explicit animation/time queries, required inverses,
the active domain, applied locals and final world matrices. Evaluate once from
authored locals, in list order, into a separate applied vector. Do not solve an
already applied snapshot. With no active constraints, preserve existing FK bits.

`Scene.locals` remains authored for compatibility. Add `Scene.applied_locals`;
`Scene.world` is the solved world vector. `checked_scene_from_locals` obtains one
`EvaluatedPose`, then emits geometry from its already evaluated world. Its private
geometry builder must not invoke the solver a second time. Existing checked
vertex/UV/bounds/fit validation remains in force.

The old unchecked `evaluate`, `evaluate_scene` and `scene_from_locals` must become
fallible aliases of the checked paths, with mechanical caller/test adaptation.
Do not retain an ambiguously named public full-rig API that silently emits FK for
a native6 IK document, or introduce `expect`/panic as an error strategy. Explicit
low-level `world_pose` remains available for genuine FK reconstruction.

Caller migration is part of Phase A:

- Numeric local values, key conversion, pending edits and saved/exported transforms
  read authored locals. Constraint output is never automatically baked.
- `selected_parent` becomes Result-aware and reads evaluated parent world. Gizmo
  origins and rendered axes use the selected bone's applied world directly;
  `parent.then(authored)` is no longer a valid geometry shortcut.
- Canvas rendering, picking, bounds, fit, CLI pose and weight diagnostics consume
  the same checked Scene/evaluated result. Inspector-first errors must surface
  even before the canvas queries that frame.
- CLI `pose` adds explicitly named authored `locals` and solved `appliedLocals`
  fields; the previous output contained world bones and attachments only. Bind/Direct acceptance helpers must request that applied
  snapshot; their old setup/held-key reconstruction is authored. A temporary FK
  snapshot document must not retain enabled constraints and solve it again.
  Until a helper has an explicit constrained-rig path, refuse those inputs
  clearly rather than quietly neutralizing real user constraints.
- No persistent UI pose cache is added. Reuse one immutable evaluated result only
  within a canvas/query pass with unchanged Session state. After dispatch,
  history, context or preview changes, query again. Gesture launch cancels old
  previews first, then freezes authored transform, applied transform and applied
  parent from one new checked baseline. Panels never implement another solver.
- `Session::mesh_bind_pose` captures **applied locals once**, including current
  Animate pending edits. Serialized `MeshBindPose.locals` remains an explicit FK
  snapshot. Bind/Add/Unbind/Direct reconstruct it with `world_pose` and never
  apply constraints again. Preserve the existing inverse/.001 binding guards.
  A half-mix test must expose any accidental second solve.

## Commands and UI boundary

Add strict serializable commands:

```rust
AddIkConstraint { constraint: IkConstraint }
SetIkConstraint { constraint: usize, value: IkConstraint }
```

The second replaces one existing entry without changing its identity/order and
covers name, refs and setup settings. Add appends only. Validate candidate data
before publishing; failure preserves document, pending pose, saved state and
undo/redo. Setup controls use existing dispatch/fixed-base transaction previews,
commit one undo, and cancel exactly. No second history or pending-IK model exists
in this slice. Existing bone pending poses remain separate and excluded from save.

Add typed `TreeItem::Constraint` and a Constraints folder, preserving document
order. A constraint selection must never redirect a normal tool to a cached bone.
Setup can edit supported constraint settings; Animate displays them read-only and
clearly indicates that constraint keys are not supported yet. Target-bone pose/
animation editing remains available through the existing bone authoring boundary.

Creation follows the public `New... > IK Constraint` target-selection convention.
Two-bone creation requires actual parent+child selection; the current singular
Tree selection is an explicit prerequisite/limitation, not permission to invent
a different wizard. New target+constraint creation must use one validated Batch
with the existing AddBone command once parent/naming/target-at-tip conventions
are observed. Existing-target creation needs no implicit bone rewrite.
The IK target chooser and Weights Bind are mutually exclusive transient modes.
Starting either cancels the other's intent and any active transform/Direct preview
before capturing a new target. Open/history/context/focus changes cancel stale
chooser and preview state; text undo and normal file/history shortcuts retain
their existing priorities.

Actual 4.3 Scale Y labels, constrained-axis display/edit restrictions, partial-mix
bone tool behavior, target selection/cancellation and target-at-tip placement are
interactive observation gates. Provide authored and applied API data now, but do
not infer those controls from a matrix prototype. Phase A acceptance can cover
target manipulation and setup constraint properties without claiming all
constrained-bone World/Parent numeric interactions are already equivalent.

## Migration, adapters and acceptance

Native5→6 inserts required `constraints: []`, with no other semantic rewrite.
Versions1–4 retain their existing tag/duration/mesh/shear migration chain, then
take the5→6 step. Reject constraint fields smuggled into versions1–5, even empty.
Keep validation-before-legacy-hold materialization unchanged. Convert all eight
current original fixtures by format+empty-constraint changes only. Preserve their
full existing Spine export bytes by omitting an empty Spine constraint collection.
The Python latest-version helper does not populate new root fields automatically:
demo constructors must add `constraints: []`. Legacy test documents derived from
the latest model must deliberately remove the new field when representing an old
version, rather than accidentally treating smuggled fields as a valid baseline.

Spine4.3 import accepts only unified root IK entries with supported neutral
advanced settings: omitted or softness0, compress/stretchfalse and scaleY`none`.
Reject nondefault advanced values **even at mix0**, unknown/skin-required kinds,
legacy root `ik`, guessed animation `constraints`, ignored setting aliases and
any nonempty animation `ik` object, including named empty timelines. Omitted or
an empty animation `ik` object is harmless but must have the correct type.
Optional `order` is recognized only as finite, nonnegative integral obsolete input
metadata, validated without a lossy numeric cast; never use/sort/export it, and
make no raw-metadata-lossless claim.
Names, references, values and field types remain strict. No unsupported constraint
is filtered out of a mixed list. Data-domain/import failures never overwrite input.

Export preserves list order/names/refs/setup mix/bend, emits no inactive advanced
feature or invented animation frame, and refuses unsupported source features
before writing. This means exact structural/source restrictions, not a second
solver in the adapter or a guarantee inferred from sampling a few times. A target
can cross a coincident origin between keys; dynamic coincidence, precision and
finite-result failures remain explicit checked-query errors. Import/export success
does not certify that every intermediate pose lies in the bounded evaluator's
domain. Native roundtrip preserves raw valid authored data that a query currently
cannot evaluate. Never null, clamp, bake, or silently default an IK feature.

Implementation acceptance requires structural/strict-schema and migration tests;
zero-constraint byte goldens; normal one-/two-bone bends, mix and reach limits;
moving/fractional animated targets; exact winding/ties; disjoint ordered lists;
clear dependent/transformed/degenerate refusals; checked numeric failures;
authored/pending/applied separation; weighted geometry/bounds/picking; applied
Bind snapshot without double solve; fixed-base previews/one undo/cancelled redo;
CLI/native/packed roundtrip; external runtime and installed-editor import/reexport;
and actual native target/property interactions. Retain failures and source hashes.

This remains a bounded constraint slice. Physics, path and transform constraints,
IK timelines/settings and full artist-rig support are separate work. Private
aggregate compatibility findings remain separate from these original fixtures.
