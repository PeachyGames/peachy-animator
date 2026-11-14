# Next run: Spine parity work

Written November 10, 2026, at the end of the import-parity push. Read this
first, then `AGENTS.md`. The workspace rules in `../AGENTS.md` and `../CLAUDE.md`
still apply.

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

Next useful continuation: deform Dopesheet rows/curves and multi-vertex editing,
then broader mesh/Auto/brush weights and multi-bone cycle Offset workflows.
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
