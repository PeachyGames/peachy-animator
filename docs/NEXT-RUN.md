# Next run: Spine parity work

Written November 10, 2026, at the end of the import-parity push. Read this
first, then `AGENTS.md`. The workspace rules in `../AGENTS.md` and `../CLAUDE.md`
still apply.

## Where things stand

- `main` at `6d67e5f` or later, pushed. Native format **v22**.
- **All 21 shipped game skeletons import and match the external Spine 4.3
  runtime** (`peachy-cli inspect` over
  `../loadofftemp-release-0.51.0/src/assets/spine/**/*.json`, skipping
  `underwear-source`). Per-rig evidence is in `docs/VALIDATION.md`.
- 562 Rust tests pass (one existing ignored), clippy `-D warnings` and the architecture check pass. All 21
  original Peach Bot labs in `examples/peach-bot/` pass `scripts/check-runtime.mjs`.
- One check was still running when the session ended: the female pose's full
  runtime check (clipping coverage on real data; its clip ends on its own slot).
  Rerun it with:

  ```sh
  scripts/verify-game-rig.sh femalepose ../loadofftemp-release-0.51.0/src/assets/spine/female-pose/female_pose.json --full
  ```

  It takes hours (85 skins). Its bones and meshes already matched (3.56M world
  values, max 0.0005). If it fails, the likely suspects are the own-slot clip
  end rule and the 1% coverage-mismatch allowance (`docs/slices/clipping.md`).

  **Overlay follow-up, November 10:** the earlier process was gone and its full
  report empty. A fresh run uses the name `femalepose-overlays-20261110`.
  Check whether it is still running before starting another one. Its overall
  log is `.local/artist/femalepose-overlays-20261110.log`, full report is
  `.local/artist/femalepose-overlays-20261110-runtime.txt`, and private native/
  JSON/hash files use the same prefix. Its immutable CLI snapshot is
  `.local/peachy-cli-femalepose-overlays-20261110`; do not delete it during the
  run. The wrapper removes that snapshot when finished. This run is **pending**,
  not additional validation evidence; check the full report even if the wrapper
  exits zero (the wrapper prints full-check failure without failing its exit code).

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
- Disk: stop below 15 GB free (28 GB at the end of this session). `target/` is
  about 14 GB; don't create a second target directory.
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

1. **Finish the female pose full check** (above) and add its numbers to
   `docs/VALIDATION.md`.
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
