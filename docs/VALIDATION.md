# First-slice validation — November 5, 2026

Verified locally on macOS with Rust 1.99.1 and the locked dependencies:

| Check | Result |
| --- | --- |
| Formatting | `cargo fmt --all -- --check` passes |
| Rust tests | 11 pass across engine, adapters, CLI and file persistence |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` passes |
| Build | `cargo build --workspace` passes |
| Native interaction | Real key events verify key insertion, undo, redo, play/pause and frame stepping |
| Original fixture runtime | 9,180 sampled transform comparisons pass |
| Synthetic transform contract | 4,590 comparisons pass, including held keys, scale and full-turn rotation |
| Spine Editor interchange | 4.3.26 imports our JSON into a new native project, re-exports it, and our importer accepts that export |
| Re-exported runtime data | 9,180 comparisons pass after the Spine Editor round trip |
| Artist input preservation | 11 existing exports inspected without source changes; all report unsupported features |

Machine-readable public reports under `docs/evidence/` use only original fixtures.
The screenshot in `docs/editor.png` shows the running native app. The artist inputs,
browser profile, authentication state and local operating logs are excluded from Git.

The native smoke check restores the original document after exercising history.
Full save/reopen, backup verification, image copying and atlas export are also covered
by filesystem checks. A new project save uses a temporary file and atomic replacement;
an existing project receives a timestamped, verified backup first.

Hosted CI has not run. Its workflow is intentionally manual-only. Windows/Linux,
WebAssembly, native installers, pixel-by-pixel render equivalence and production artist
asset migration have not been verified or claimed.

## Mesh slice — November 5, 2026

The completed mesh slice passes 28 Rust tests, formatting, warnings-denied Clippy,
workspace build and the six-crate architecture guard. Checks include preserved
region geometry under reflection/scale, local subdivision and UV interpolation,
explicit fallback attachment addressing, invalid batch atomicity, gesture undo,
v1 migration, mesh save/reopen/backup/bundle export, serialized CLI edits and rejected
malformed or unsupported mesh semantics.

| Compatibility check | Result |
| --- | --- |
| Mesh demo, all three skins, two clips | 5,100 mesh coordinates; max error 0.000038147; 10,200 UV values with zero observed error |
| Reflected/nonuniform parent and animated scale contract | 2,550 mesh coordinates; max error 0.0000610352; 5,100 UV values with zero observed error |
| Original region fixtures, now all skins | 41,310 bone matrix/position comparisons pass |
| Mesh fixtures' bone transforms | 41,310 further comparisons pass |
| Installed Spine Editor 4.3.26 | Imports/re-exports all three meshes; vertex, UV, triangle, hull, edge and size data unchanged; our importer accepts output |
| Editor re-export in external runtime | 5,100 mesh coordinates, 10,200 UV values and 27,540 bone values pass |
| Shared packed bundle in external runtime | 5,100 mesh coordinates, 10,200 UV values and 27,540 bone values pass |
| Native mouse and keyboard events | Region conversion, subdivision, vertex drag with fixed UVs, single-step drag undo, redo and full document restoration pass |

These samples use 51 times per clip per skin, including endpoints and backwards
scrubbing. The harness asserts native meshes remain mesh attachments in the external
runtime, compares triangle order and hull count, and verifies the actual atlas PNG
size. Transform/vertex tolerance is 0.001; UV tolerance is 0.000001. These are finite
geometry checks, not a pixel-equivalence test.

Public mesh reports and the native interaction summary are under `docs/evidence/mesh-*`.
The original-only screenshot `docs/mesh-editor.png` shows an edited hand vertex. Local
logs and the installed editor's generated `.spine` project stay outside the repository.
The first native mouse attempt used direct-PID events without window routing and did
not click the button; the successful check used normal on-screen mouse routing in the
verified task-owned foreground window. Both attempts retained evidence and closed
only their own native test windows. No private artwork or existing user window changed.

Weights, deform animation, linked meshes, UV/hull editing, production artist migration,
pixel rendering equivalence, other operating systems and hosted CI remain unverified.

## UI alignment — November 5, 2026

The user made Spine's layout, controls and conventions an explicit requirement.
The initial alignment uses the public 4.3 UI/views/tools documentation and default
shortcut sheet. A task-owned instance of installed Spine 4.3.26 opened our original
reference project, but macOS refused its window screenshot (`could not create image
from window`). Exact visual comparison with that installed window remains pending;
this pass does not claim pixel equivalence. The reference process was closed.

Native mouse/key checks in our own task-owned review window verify:

- Tree and properties on the right, transform toolbar below the viewport, Setup/Animate
  at upper left, and Dopesheet below main tools only in Animate.
- C/V tool selection, D play/stop, R/F frame stepping, Shift ten-frame stepping,
  Cmd/Ctrl+R loop toggle without stepping, Space deselection, Cmd/Ctrl+L keying,
  arrows nudging instead of stepping, and F9 view hide/restore.
- Setup mesh vertex dragging preserves UVs and remains one undo step.
- History restores the original document after checks; no fixture is saved or changed.

All 28 existing Rust checks, formatting, warnings-denied Clippy, build and the six-crate
architecture guard pass. Domain and format crates are unchanged, so the previous
mesh/runtime results remain the data compatibility baseline. The native interaction
summary is `docs/evidence/ui-native.json`; current Setup/Animate screenshots are in
`docs/workspace-setup.png` and `docs/workspace-animate.png`. Earlier screenshots and
records describe earlier slices. The review window is closed after checking; a clean
interactive native handoff is retained separately.

Detailed styling, Graph/docking, full Tree entity types, all tool axes and selection
semantics, unkeyed poses and other controls remain required work in UI_PARITY.md.
