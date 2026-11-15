# Next run: Spine parity work

Written November 10, 2026, at the end of the import-parity push. Read this
first, then `AGENTS.md`. The workspace rules in `../AGENTS.md` and `../CLAUDE.md`
still apply.

## November 15 workspace divider continuation

The canvas now reserves the nearest right pane's actual egui resize grab band
from input, using a stable canvas owner and a separate input rectangle. Full
painting/Fit/projection and the 320-pixel minimum viewport budget are unchanged;
already owned drags continue outside that rectangle, and all-hidden right panes
restore full input. Native Tree grabs at −2/0/+2 logical border offsets preserve
document/history/selection. Properties resizing exposes all mapping fields and
Options, and held Mix/Escape still works after resizing. The public full-table
capture is updated. Five focused tests additionally cover Weights-only/Curves-only
boundaries, pending/history preservation, released-size persistence within the
session and an owned Move across the reserved strip. These pane variants have
headless coverage only; no new native or cross-restart docking acceptance follows.

The affected checks pass: 5 pane, 26 Transform, 40 weight, 52 curve and 26 Deform,
plus formatting, all-target Clippy, workspace build and the architecture check.
A pre-existing Deform test clicked raw key metadata below the Properties clip
on both the mapping checkpoint and floating-scrollbar control. Key/Delete
capture targets now include only visible intersections; the real-event test
wheels Properties until its button is visible and verifies clip containment.
Shared key commands are unchanged. The final native unweighted replay passes:
actual Arrow Right, Key/Undo preserving the pending pose, Properties shrinking
with pending pose/redo intact, absent offscreen key/Delete captures, an 80-pixel
wheel revealing Key, and Key/Delete restoring the baseline document. Artwork is
unchanged and the owned window is closed. Weighted/unweighted cases pass focused
event checks; no new native weighted-mesh claim follows. All current source and
native gates for this bounded workspace follow-up pass.

**Next feature action: calibrate manually driven Slider constraints from native
controls and public runtime behavior, or the remaining bounded UI workflows in
the [blog backlog](slices/spine-blog-audit-20261114.md).** Mapping checkpoint
`54936e4` is already pushed with the draft PR updated; the coordinator handles
this follow-up's cleanup, commit and publication. There is no native-format or
MCP-schema change. Reference/native windows used for these checks are closed.
See [UI parity](UI_PARITY.md#workspace-divider-input--november-15-2026).

## November 15 Transform mapping continuation

World X→Rotate now passes shared core/MCP, actual native acceptance and both
external-runtime/installed-editor gates. Native v22 stays, with **53 commands /
13 tools**. See [scope](slices/transform-mapping-authoring.md),
[gallery](evidence/transform-mapping.html) and [scalar evidence](evidence/transform-mapping.json).

Native typing, repair/reversal, Clamp/Mix, held cancellation/input blocking,
Undo/Redo, Offset extrapolation, owned/pending Save, zero-scale blank/recovery,
minimum 1120×740 controls and fresh public reopen/playback pass. Native and the
122-request root-only MCP recipe save byte-identical
`examples/peach-bot/transform-mapping-bot.peachy` (SHA256
`62a637db493f5abf41bd197c4cdb5db03c83120d95acae6e4fd24f285204757d`).
The native coalesced edge-drag exposed floating scrollbar overlap; a reserved
solid outer scrollbar fixes it, with a whole-workspace regression and native
replay. The regression settles egui's scrollbar animation before freezing geometry.
Temporary diagnostics are removed. The workspace continuation above now resolves
the former exact-edge resize gap; exact styling and general docking remain open.

The 103-case matrix records 101 runtime passes at unchanged 0.001 tolerance,
one overflow refusal and one measured consumer-precision refusal. Installed
Spine points down for the paired biased/unbiased −180 controls; its JS runtime
points up for the biased source. Do not relabel this refusal as a runtime pass.
Published f32 poses remain; the private wide FK never solves constraints again.
Exact final 04 installed interchange passes 52 poses with zero world/region/UV error
and exact raw authored sections, including mappings/keys/attachment dimensions.
At the mapping checkpoint, 26 Transform, 39 weight and 51 curve cases, prior core/MCP/adapter checks,
fmt, all-target Clippy, workspace build and seven-crate architecture pass.

All task-owned reference/native windows are closed. Public original lab/captures
and scalar evidence are committed with this slice. Private calibration sources,
selected licensed captures, final 04 requests/recipes/reports and consumer outputs
remain only for active PR review, until integrated or superseded. Task-owned
superseded outputs, temporary native helpers and redundant raw diagnostics retire
at this checkpoint. The shared Cargo target remains required by ongoing parity
work; no extra target/worktree/repository copy was created. PR1 remains draft;
required hosted CI still gates main integration.

Continue the [blog backlog](slices/spine-blog-audit-20261114.md): manually driven
Slider constraints need native/public-runtime calibration before a new model;
other mappings, creation/reconnection, stored Curves defaults/presets,
Auto/Smooth/brush weights and locks/Update also remain. Problems, broader Color,
Pies/overlay and exact styling/docking remain open. The canceled female-pose full
verification stays canceled.

## November 15 multi-vertex Direct continuation

Multi-vertex Direct weights now has a dedicated native pane, additive/marquee
selection, mixed blank/asterisk display, absolute assignment and cumulative
relative gestures. The shared engine command and read-only `get_mesh_weights`
bring MCP to **52 commands / 12 tools**; native **v22** is unchanged. Applied
binding snapshots now include sampled inheritance modes. Legacy omitted modes
retain Setup reconstruction. See [scope](slices/weight-multivertex.md),
[gallery](evidence/weight-multivertex.html) and [evidence](evidence/weight-multivertex.json).

Actual native input/history/cancellation, pending Save exclusion, fast Cmd release,
fresh public reopen, playback and 1120×740 layout pass. Native and root-only MCP
authoring save byte-identical `examples/peach-bot/multi-weights-bot.peachy`
(SHA256 `87214e61dc6410d8f9f0306deacd7b19b2a658a9353bd2b15b9f2be606540807`).
Ninety actual stdio requests recreate the lab from one original PNG; both the
external runtime and installed 4.3.26 roundtrip pass at unchanged 0.001 tolerance.
An additional 141-request study verifies five captured inheritance modes;
its keyed modes are imported fixture data, not new inheritance-key authoring.
Focused two-worker checks, formatting, all-target Clippy, workspace build and
seven-crate architecture check pass. Hosted CI still gates main; keep PR1 draft.

Sparse creation inserts only a new `[0,0]` entry in consistent mesh binding order.
Existing raw entries are untouched. Installed controls isolated an import failure
for cyclic cross-vertex precedence; Editor export now rejects it before writes,
while Runtime export preserves the arrays. Deform-layout remapping remains a
separate requirement for sparse creation with keyed/pending Deform data.

Continue the [blog backlog](slices/spine-blog-audit-20261114.md): stored Curves
presets/default choices, transform mappings/sliders, Auto/Smooth/brush weights,
locks/Update and multiple selected bones. Pies/overlay controls, Problems, broader
Color and exact styling/docking remain open. The canceled female-pose full check
stays canceled.

All owned native/reference processes are closed. Private original calibration
inputs, selected captures, scalar reports, actual MCP requests/recipes and final
runtime/interchange outputs remain in `weights-multivertex/` until active review
is integrated or superseded. Temporary input helpers, invalid input diagnostics
and reconstructible Save backups retire at the verified checkpoint. The shared
Cargo target remains required by ongoing parity work; no extra checkout or target
was created.

## November 14 global draw-order continuation

Global Animate draw order now has Tree movement/dragging, Reset, pending poses,
Auto Key/K and typed Dopesheet retime/clipboard/Delete. Shared raw and semantic
commands plus complete hidden-slot readback bring MCP to **51 commands / 11
tools**; native v22 stays. Actual native input, cancellation/history, fresh
canonical reopen/playback, both authored runtime checks and installed 4.3.26
interchange pass. Focused two-worker checks, formatting, all-target Clippy,
workspace build and architecture pass. See [review](evidence/draw-order-authoring.html),
[scope](slices/draw-order-authoring.md) and [evidence](evidence/draw-order-authoring.json).

The exact native eight-key lab is
`examples/peach-bot/draw-order-authoring-bot.peachy`. Its four original PNGs
also drive the separate root-only actual MCP study: 343 requests, two clips,
265 poses, 32 hidden-slot samples. No finished rig is a recipe input. Raw
imported encodings survive; newly encoded minimal ties are ours. Peachy Undo
preserves pending poses, an explicit measured history difference from Spine.

Folders, Setup reordering, cross-layout clipboard mapping and compact native
property sizing remain. Continue the blog backlog with stored curve presets
and default choices, multi-vertex weights, transform mappings and sliders.
Observe native behavior before adding each shared engine/MCP boundary. Problems,
broader Color and arbitrary docking remain open. Hosted CI still gates main;
keep the PR draft. The canceled female-pose full check remains canceled.

All owned native/reference processes are closed. Private original inputs,
MCP recipes/requests, authored exports and selected review witnesses remain in
the existing `draw-order-authoring/` record until review/CI evidence is superseded.
Temporary input helpers, raw step captures and superseded Save backups retire
at the verified checkpoint. The shared Cargo target remains required by ongoing
parity work; no extra target, worktree or repository copy was created.

## November 14 Curves continuation

Curves now has a separate fitted relative-transition pane and shares exact
Dopesheet selection with Scalar Graph. Scalar tangents/numerics, raw scalar/IK
curve setters and shared preset/handle actions bring MCP to 49 commands / 11
tools; native v22 stays. Installed 4.3.26 reset/live matrices, actual native
typing/cancel/history/persistence and both authored external runtime checks pass.
Focused two-worker checks, formatting, Clippy, build and architecture pass.
See [review](evidence/curves-authoring.html), [scope](slices/curves-authoring.md)
and [scalar evidence](evidence/curves-authoring.json).

Stored preset management is disabled. Last chosen, native property grouping,
flat Match destinations and several Separate-off coupled domains remain open;
unsupported operations fail explicitly. The current Views count is 16 of the
installed editor's 17; Problems and broader Color remain. Full parity is not
claimed. Required hosted CI still gates main; keep the feature PR draft.

Global Animate draw-order editing is covered by the continuation above. The
4.3 Look around demo uses it alongside completed slot swaps/fades. Per-folder
keys need a later identity/model slice. Continue
Curves library/defaults, multi-vertex weights, cross-property mappings and sliders
as documented in [the official audit](slices/spine-blog-audit-20261114.md).

Owned Curves native/reference processes are closed. Final original projects,
reports and selected captures remain in `curves-authoring/` until review or
supersession; temporary helpers/raw captures retire at the verified checkpoint.
Shared Cargo target remains required by continuing parity work; no second target
or checkout was created. The canceled female-pose full check remains canceled.

## November 14 export-target continuation

Native, CLI and MCP export now share explicit Spine Editor 4.3 / Runtime 4.3
consumer targets. Editor is the default and rejects unsafe default name overrides
before writing; Runtime preserves independent names, placeholders, skin fallback,
slot keys, Deform and linked references. Native v22 and 45 commands/11 tools stay.
Actual native pending/history/cancel/rejection/export, CLI and stdio MCP checks
pass. Three bundle files are identical across native/MCP/CLI. Four accepted Editor
controls survive installed 4.3.26 import/re-export; region and minimal linked-mesh
Runtime fixtures pass the external oracle. Focused checks, formatting, Clippy,
build and architecture pass. See [gallery](evidence/export-targets.html),
[contract](slices/export-targets.md) and [scalar evidence](evidence/export-targets.json).

The previous naming gap is bounded by an explicit consumer policy; this is not
silent normalization or a complete .spine converter. All owned GUI is closed.
Private original inputs, reports, selected captures and final bundles remain in
`.local/training-spine-20261114/names-contract/` until review or supersession.
Task-owned raw captures, Cocoa/input diagnostics, invalid probes and redundant
CLI outputs retire at closure; shared target remains required by ongoing parity.

The reset/final-key probes and bounded scalar Curves workflow are covered by the
latest continuation above. Continue its recorded authoring gaps, multi-vertex
Direct/Auto/brush weights, cross-property transform mapping and sliders.
Use [the refreshed official blog audit](slices/spine-blog-audit-20261114.md).
Hosted CI still gates main; keep the feature PR draft. The canceled female-pose
full check remains canceled.

## November 14 slot authoring continuation

Slot swaps/hiding and RGBA fades now have native/MCP authoring, shared float
pending poses, Auto Key/K, exact typed keys/clipboard and four-channel Graph
editing. Properties matches the observed Set slot color dialog and adjacent key.
Native v22 stays; MCP has 45 strict commands/11 tools and raw hidden-slot arrays.
New color keys share the calibrated split command with MCP; 104 independently
observed native split parameters match, including a nonzero-origin holdout.

Actual native modal/Graph/keyboard/history/pending Save/reopen/playback checks
pass; both authored three-clip exports match the external runtime. Focused checks,
formatting, Clippy, build and architecture pass. See
[review gallery](evidence/slot-authoring.html), [contract](slices/slot-authoring.md)
and [scalar evidence](evidence/slot-authoring.json). Private reference comparisons,
authored projects and MCP logs remain in
`.local/training-spine-20261114/slot-authoring/` until review. Temporary helpers,
invalid probes, redundant captures and verified duplicate Save backups retire at
closure. The shared Cargo target remains needed for continuing parity work.

The default-skin name-override gap found here is now handled by the explicit
Editor/Runtime export contract above, preserving valid native/runtime metadata.
Then continue multi-vertex/Auto/brush tools, all-keyed row hierarchy/mixed Graph,
Offset, broader Color and current blog features. Byte sampling ties/extreme-time
fallback remain explicit limits. Hosted workflow registration still returns zero;
keep the feature PR draft and main gated on required hosted CI. Do not restart
the canceled female-pose full check.

## November 14 Deform timeline continuation

Deform Dopesheet rows, exact retime/copy/delete and typed clipboards now share
engine commands with native controls and MCP. Graph edits normalized 0–100
pose-transition curves with presets, draggable tangents and numeric percentages.
Graph/Dopesheet tabs span the bottom beneath Tree, matching observed 4.3.26
placement. Native v22 remains unchanged; MCP now has 11 tools / 41 commands,
including read-only `copy_animation_keys` and exact `setDeformCurve`.

Integrated native mouse/key, clipboard/history, Escape, playback and Save/reopen
acceptance passes; both native- and MCP-authored projects match the external
runtime. Local focused checks, formatting, Clippy, build and architecture pass.
See [review gallery](evidence/deform-timeline.html),
[contract](slices/deform-timeline.md) and
[official update backlog](slices/spine-blog-audit-20261114.md). The actual installed
Views menu has 17 entries, not 15; Curves/Problems and broader Color remain open.
Owned native/reference processes are closed. Private comparison/projects/MCP
logs remain in `.local/training-spine-20261114/timeline/`; redundant F12 captures
and helpers retire after the verified commit. The shared Cargo target is retained
for continuing parity development; no second target/worktree was created.

Continue through bounded slices: Animate slot attachment/RGBA color authoring
(image swaps/fades used in the 4.3 blog), then multi-vertex/Auto/brush tools,
all-keyed bone hierarchy/mixed Graph and Offset. Hosted CI still lists no registered
workflows as of this check; keep feature PR draft and do not integrate main until
the required hosted check passes. Female-pose full verification remains canceled.

## November 14 authoring update

Final private tutorial review: `.local/training-spine-20261114/recreation/index.html`.
The timestamped visual action transcript and full audio-coverage records are
linked there. Both recreated projects were built through actual MCP calls from
raw cutouts and the empty source layout; the finished rig is excluded from
recipe inputs. Halloween has 79 bones, 27 meshes, one IK constraint and three
clips; Wind has five explicitly qualified technique cases, seven weighted
meshes and two clips. Both pass the external runtime checks. See
[training recreation](slices/training-recreation.md).

Prepared on `feat/training-deform-authoring`; **main integration awaits hosted
CI**. The manual dispatch returns HTTP 404 (`check.yml` not found on the default
branch), while the contents API confirms the file exists on remote main and
Actions reports zero registered workflows/runs. Repository Actions is enabled;
no settings were changed. Do not describe this as a hosted CI pass or silently
waive the required gate.

Startup image loading now uses the actual WGPU device texture limit. The private
training example that aborted above 2048 pixels now opens with all original images;
both imported clips were observed playing in the native editor. No source artwork
was changed. Local assessment/implementation evidence is in
`.local/training-spine-20261114/`.

Single-vertex weighted/unweighted **Animate deform authoring** is implemented:
Vertices retains the current mode; drag/nudge, pending offsets, Auto Key, K,
contextual Deform key/delete, undo/redo and guarded fixed-baseline previews share
the engine. Native v22 remains. See [the contract](slices/deform-authoring.md)
and [original-fixture evidence](evidence/deform-authoring.json).

Fresh tutorial outcomes are now built through actual stdio MCP calls from a
root-only `create_project`, using raw cutouts and the source Photoshop layout.
The finished artist rig is excluded from recipe inputs. Shared commands append
slots and region/weighted mesh attachments; single-page packed export supports
4096×4096. See [scope and evidence](slices/training-recreation.md).

Audio coverage of all four videos is complete. Instructional portions are silent;
visual action transcripts use 569 frames across 31 contact sheets. The Wind
showcase's last 16.4 seconds contain speech whose wording remains unverified;
visible English subtitles are transcribed. Wind source artwork was not supplied,
so its five technique cases use explicit proxies. Do not call this an exact
character recreation or full tutorial workflow/UI parity.

Deform Dopesheet rows/curves are covered by the continuation above. Remaining
mesh work includes multi-vertex editing, Auto/brush weights and multi-bone cycle Offset workflows.
The tutorials' wind motion is hand-keyed; no wind rig was supplied. Physics
simulation remains separate. The female-pose full-check cancellation below
still applies. Manual CI requires an explicit focused test-name filter and
caps Cargo/test workers at two.

## Where things stand

- `main` at `6d67e5f` or later, pushed. Native format **v22**.
- **All 21 shipped game skeletons import and match the external Spine 4.3
  runtime** (`peachy-cli inspect` over
  `../loadofftemp-release-0.51.0/src/assets/spine/**/*.json`, skipping
  `underwear-source`). Per-rig evidence is in `docs/VALIDATION.md`.
- 562 Rust tests pass (one existing ignored), clippy `-D warnings` and the architecture check pass. All 21
  original Peach Bot labs in `examples/peach-bot/` pass `scripts/check-runtime.mjs`.
- **Female-pose full verification: stopped at Chris's explicit request on
  November 10, 2026. Do not restart it unless he explicitly requests it in the
  current session.** The wrapper, full-check process and CLI worker were stopped
  and verified exited. There is no continuing background verification.

  The interrupted run was `femalepose-overlays-20261110`. Partial evidence stays
  local: `.local/artist/femalepose-overlays-20261110.log`,
  `.local/artist/femalepose-overlays-20261110-runtime.txt`, and matching native/
  JSON/hash files. Its full report was empty at cancellation; this is **not a
  full-check pass**. Stop/process evidence is recorded in
  `.local/overlays-20261110/female-run.json` and `stop-request-processes.json`.
  The stopped run's disposable CLI snapshot has been removed.

  Earlier bone/mesh checks matched (3.56M world values, max 0.0005). The rerun's
  original-vs-re-export phase also passed before cancellation. Full clipping
  coverage across all 85 skins remains unverified; see `docs/slices/clipping.md`.

- **Viewport guides completed:** evaluated path/clipping outlines, active-skin
  and deform handling, picking, selected vertices/handles, path end directions,
  hidden-slot handling and image-free Fit. Both original labs pass actual native
  selection, typed search, hide/show and frame-step checks. See
  `docs/evidence/attachment-overlays.html` and `docs/UI_PARITY.md`. Vertex editing,
  clipping fill and exact styling remain gaps. Reference/native windows were closed;
  local detailed evidence is `.local/overlays-20261110/`. The existing shared
  `target/` was reused; no extra build directory was created.

## Ground rules that bit or nearly bit

- **Never read or port Spine editor or runtime source.** Derive behaviour from
  public docs, `.d.ts` typings and black-box runs of the runtime and the
  installed editor. Record what you observed in the slice doc and
  `docs/PROVENANCE.md`. Stack traces that point into runtime files are fine to
  see; don't open the files.
- **Game art is read-only.** Never modify or commit anything in `spine-files-2`
  or `../loadofftemp-release-0.51.0/src/assets/spine`. Keep copies and reports
  under `.local/artist/` (gitignored) and check SHA-256 before and after.
- Commits use the trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
  This repo's workflow: feature branch, fast-forward `main`, `git push origin main`.
  (The game repo's "no Co-Authored-By" rule does not apply here.)
- Disk: run `python3 /Users/chris/.local/bin/codex-disk-check --path "$PWD" --json`
  and include `--expected-write-gb` before a large write. Keep 15 decimal GB of
  verified available capacity after the write; report raw free and native macOS
  available capacity separately. Follow the helper's verified/raw-free fallback
  policy; a low `df` alone is not a block. Reuse the existing Cargo target.
- Tests: `-j 2` and `-- --test-threads=2`. Vitest is banned for the game only.

## Tooling

| What | Command or path |
| --- | --- |
| cargo (no system cargo) | `../.local-tools/peachy-animator/cargo` |
| Architecture check | `PATH=../.local-tools/peachy-animator/rustup-home/toolchains/stable-aarch64-apple-darwin/bin:$PATH CARGO_HOME=../.local-tools/peachy-animator/cargo-home python3 scripts/check-architecture.py` |
| Runtime oracle (spine-core 4.3.13) | `../loadofftemp-release-0.51.0/node_modules/@esotericsoftware/spine-core/dist/index.js` |
| Headless renderer (final triangles, for clipping/blend checks) | `new SkeletonRendererCore().render(skeleton)` from the same package |
| Spine editor CLI (4.3.26) | import: `/Applications/Spine.app/Contents/MacOS/Spine -u 4.3.26 -i in.json -o p.spine -r`; export: `-i p.spine -o out/ -e json` |
| Fixture vs runtime | `node scripts/check-runtime.mjs <runtime> <export.json> <atlas> <source.peachy>`; set `PEACHY_CLI=<snapshot>` for long runs |
| Game rig vs runtime | `scripts/verify-game-rig.sh <name> <skeleton.json> [--full]` (wraps `check-rig-skins.mjs` and `check-rig-reexport.mjs`) |
| Desktop screenshot | `target/debug/peachy-animator FILE --capture OUT.png` (kill the process after the PNG appears) |

## How each slice was done (repeat this)

1. **Survey** the game data for the blocker (`peachy-cli inspect` messages,
   then a short Python pass over the JSON for the fields actually used).
2. **Probe the runtime black-box** with tiny original skeletons built in a Node
   script; read results only through public API. Fit a model in JS against
   random cases until it matches to about 1e-5. Bisect thresholds when results
   jump.
3. **Probe the editor** with a CLI round trip to learn what it writes (it often
   omits defaults and sometimes changes data, e.g. clipping `convex`).
4. **Implement**: document model (bump `FORMAT_VERSION`, add an io migration
   plus legacy guard, a `native-vN-pre-*` fixture, textual example upgrades),
   Spine adapter (read, write, inspect refusals), engine, desktop matches.
5. **Original lab fixture** in `examples/peach-bot/` that exercises every
   branch; pass `check-runtime.mjs`; confirm the feature actually engages
   (compare against a copy with it removed).
6. **Game rigs**: `scripts/verify-game-rig.sh`, then audit all 21.
7. **Docs**: `docs/slices/<slice>.md`, COMPATIBILITY, PARITY, VALIDATION,
   PROVENANCE, AGENTS.md note, README lab line. Then commit and push.

## Pitfalls seen this session

- Adding a struct field: either give it `#[serde(default, skip_serializing_if)]`
  or bump the format with a migration. Literal sites fail to compile, which is
  good; a script that inserts the field everywhere also inserted it into `..`
  match patterns three times, so check patterns after bulk edits.
- egui sliders default to `SliderClamping::Always`, which rewrites imported
  values just by displaying them. Data sliders must use `SliderClamping::Edits`.
- Long background checks call the CLI per sample. Rebuilding or deleting the
  binary mid-run breaks them: use `PEACHY_CLI` snapshots and remove a snapshot
  only after its run ends (one run was lost this way).
- f32 vs f64: export values like `0.2` come back as `0.20000000298`; compare
  with `json!(0.2f32)` or a tolerance.
- The exporter refuses a non-default skin placeholder that shares a name with a
  default-skin attachment (an editor rule). Fixtures must use distinct names.
- `check-runtime.mjs` samples past the clip end; anything time-dependent past
  the last key (sequences) must follow the runtime's end clamp.
- Bash `cwd` resets between calls; `cd` into `peachy-animator` in the same call
  as any git or cargo command.

## Next tasks, in suggested order

1. **Female-pose full check: deferred by Chris's request.** Do not run or
   restart it without a new explicit request. Continue with other work below.
2. **Viewport overlays: done** in the November 10 follow-up. Scene guides live
   in `crates/engine/src/scene.rs`; `path_cubics` shares solver/viewport knot
   interpretation; painting/picking live in
   `apps/desktop/src/editor/viewport_overlays.rs`. Vertex authoring is still task 3.
3. **Authoring for imported-only features** (data, import/export and
   evaluation exist; the editor can't create or edit them yet): draw order
   keys, slot attachment and color keys (Dopesheet rows), events (an Events
   view and keys), attachment tint, sequences and blend modes (properties
   panel), transform mapping editing (the panel only displays mappings), path
   and clipping vertex editing, linked mesh create/unlink, IK stretch toggle.
   Every edit must go through `peachy_engine::Command` so undo and validation
   work. Follow the memory rules: UI explanations in tooltips, name features
   by what they do. **UI parity is an acceptance criterion** (`AGENTS.md`,
   "Spine user interface parity"): observe where the installed Spine 4.3 puts
   each control, match placement, terminology and interaction, record it in
   `docs/UI_PARITY.md`, and verify real mouse and keyboard behaviour, not only
   screenshots.
4. **Exact screen and multiply compositing** in the viewport. Today screen is
   drawn additively and multiply as normal (`scene_view::draw_tint`). Needs an
   egui paint callback or offscreen pass.
5. **Physics simulation preview.** Physics is data only; re-export checks run
   the runtime's simulation, but the editor shows the unsimulated pose.
6. **Open reviews**: Chris's interactive review of animated IK
   (`docs/IK_ANIMATION_REVIEW.md`) and the raw IK-curve precision gap
   (`docs/slices/ik-raw-precision.md`).
7. **Remaining refusals** (no shipped rig needs them; fit black-box like the
   rest): closed paths, chain/chainScale path rotation, proportional spacing,
   path mix keys, IK compress/uniform/softness and keyed stretch, nonuniform or
   sheared IK bones, inverse clipping, dark tint (`rgba2`/`rgb2`), `rgb` and
   `alpha` slot timelines, draw order folders, linked-mesh chains,
   local/additive transform constraints and cross-property mappings, slider
   constraints.
8. **Native click-through checks** of the newer panels (only screenshots and
   egui tests so far).

## Native format chain

v8 physics data · v9 animated IK · v10 inherit modes · v11 transform constraints
· v12 skin scope · v13 deform keys · v14 slot attachment/color keys · v15 events
and reference scale · v16 draw order · v17 attachment tint and offset curves ·
v18 linked meshes · v19 clipping · v20 path constraints · v21 IK stretch ·
v22 image sequences and blend modes. Migrations live in `crates/io/src/lib.rs`.
