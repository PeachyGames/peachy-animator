# Peachy Animator

Build a native, open-source 2D skeletal animation editor with versioned Spine-compatible exports.

## Architecture

- `document`: serializable data, validation, no UI, rendering or filesystem dependencies.
- `engine`: commands, transactional undo/redo, deterministic pose evaluation. Depends on document only.
- `spine`: independent format adapters. Depends on document, never on official runtime code.
- `io`: shared imperative shell for file decoding, safe writes, image copying and packed exports. Depends on document/spine; no UI dependency.
- `apps/cli` and `apps/desktop`: platform clients; keep pose/key and region geometry rules in the engine.
- Follow PhotoCraft's engine-first and command-driven approach, scaled to implemented features.

## Independent implementation

Use public format documentation, general mathematics, our original fixtures and observations of permitted inputs/outputs. Record sources and limitations in `docs/PROVENANCE.md`. Do not inspect, copy, translate, vendor or port proprietary Spine editor/runtime implementation code. The official runtime may be used as an external, separately licensed compatibility oracle. Do not add it to our Cargo dependencies or redistribute it with our application.

Artist assets in the sibling `spine-files-2` project are private local compatibility inputs. Never modify their sources/exports or commit their artwork, atlas pages, skeleton data or screenshots here. Keep local reports outside Git or redact identifying content. Use our original Peach Bot fixture for committed examples and screenshots.

## Verification

Run `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo build --workspace`. Verify interpolation, command atomicity/undo, round trips and external runtime compatibility. The sibling game's Vitest prohibition applies to the game; this project uses Rust's built-in test runner.
Run `python3 scripts/check-architecture.py` too. Its allowlist checks real Cargo dependencies; update the rules explicitly when deliberately changing the layers. It does not prove arbitrary standard-library I/O cannot be introduced, so review core functions for side effects as well.

`apply_command(&Document, Command)` is the pure editing boundary. `Session` wraps it with in-memory history; pointer gestures use begin/commit/cancel transactions. Native documents and commands reject unknown fields. Export-target restrictions belong to the adapter, not native document validation. Reconcile transient editor selections after history/document changes. Introduce stable entity IDs before deletion/reparenting.

Native format v2 uses tagged `Attachment::Region` / `Attachment::Mesh`; the I/O decoder migrates v1 regions explicitly before strict deserialization. Mesh topology, UVs and slot-bone-local setup vertices belong in document/engine. `evaluate_scene` emits shared vertex/UV/index buffers for rendering, bounds and CLI diagnostics. Mesh commands address an explicit skin/slot/placeholder; resolve default fallback before building a command. Existing vertex indices remain stable under centroid subdivision. Add stable vertex identity before removal/reindexing or cross-topology animation references.

Track concrete Spine behavior and gaps in `docs/PARITY.md`. Check new geometry against the external runtime and exported data against the installed editor using original fixtures before claiming parity for a feature.

Never claim full Spine compatibility. Reject unsupported features on import and preserve the input unchanged. List supported version and feature subset in `docs/COMPATIBILITY.md`. New export fixtures belong here, not in the artist repository.

For headed browser verification, follow the workspace's `ab-headed`/`ab-cdp` isolation rules. Use a unique task session and the approved browser.
