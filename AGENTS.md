# Peachy Animator

Build a native, open-source 2D skeletal animation editor with versioned Spine-compatible exports.

## Architecture

- `document`: serializable data, validation, no UI, rendering or filesystem dependencies.
- `engine`: commands, transactional undo/redo, deterministic pose evaluation. Depends on document only.
- `spine`: independent format adapters. Depends on document, never on official runtime code.
- `apps/cli` and `apps/desktop`: filesystem and platform clients of those libraries.
- Follow PhotoCraft's engine-first and command-driven approach, scaled to implemented features.

## Independent implementation

Use public format documentation, general mathematics, our original fixtures and observations of permitted inputs/outputs. Record sources and limitations in `docs/PROVENANCE.md`. Do not inspect, copy, translate, vendor or port proprietary Spine editor/runtime implementation code. The official runtime may be used as an external, separately licensed compatibility oracle. Do not add it to our Cargo dependencies or redistribute it with our application.

Artist assets in the sibling `spine-files-2` project are private local compatibility inputs. Never modify their sources/exports or commit their artwork, atlas pages, skeleton data or screenshots here. Keep local reports outside Git or redact identifying content. Use our original Peach Bot fixture for committed examples and screenshots.

## Verification

Run `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo build --workspace`. Verify interpolation, command atomicity/undo, round trips and external runtime compatibility. The sibling game's Vitest prohibition applies to the game; this project uses Rust's built-in test runner.

Never claim full Spine compatibility. Reject unsupported features on import and preserve the input unchanged. List supported version and feature subset in `docs/COMPATIBILITY.md`. New export fixtures belong here, not in the artist repository.

For headed browser verification, follow the workspace's `ab-headed`/`ab-cdp` isolation rules. Use a unique task session and the approved browser.
