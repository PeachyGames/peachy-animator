# Code quality review — November 5, 2026

Reviewed the first-slice commit `7f522ee` before expanding the feature set. The initial
crate separation was useful, but it left domain rules in the desktop and duplicated
file handling. This pass repairs those boundaries and concrete correctness defects.

| Finding | Resolution |
| --- | --- |
| Pose-to-key conversion was desktop-only | Shared `SetPose` command handles setup offsets, scale factors, frame snapping and curve preservation |
| Attachment resolution, geometry and fit bounds lived in the renderer | Pure engine `evaluate_scene` returns resolved region quads, world transforms and common bounds |
| CLI and desktop had separate file publication paths | Shared `peachy-io` handles parsing, safe new files, saves/backups and packed exports; CLI can call the same bundle exporter |
| Native document validation included Spine naming policy | Export-specific restrictions moved into the Spine adapter |
| One large desktop module mixed responsibilities | Separate entrypoint, application coordination, panels and viewport |
| Undoing a selected bone addition left an invalid index | Editor reconciles transient selections after document/history changes |
| NaN deletion time could erase unrelated keys | Commands reject nonfinite/out-of-range deletion times before committing |
| Inspector drags filled undo history with intermediate values | Session gesture transactions group pointer updates into one undo step and preserve redo on cancel |
| JSON import could discard malformed containers/duplicate skins | Typed capability report, shape validation and duplicate-name rejection |
| Native JSON could discard unknown future fields | Documents and commands reject unknown fields |
| No automated protection against dependency erosion | Cargo-metadata allowlist checks production/build dependencies and exercises a deliberately invalid UI-to-core edge |

The functional boundary is `apply_command(&Document, Command) -> Result<Document, String>`.
Temporary mutation inside an owned candidate is an implementation detail; the input,
filesystem, clock and UI are unchanged by that function. Session coordinates its
results with in-memory history. File operations live in the imperative shell, where
dialogs and callers choose when to execute them.

## Verification

- 21 Rust tests pass, including command atomicity, pose conversion, gesture cancellation,
  selection repair, scene fallback/bounds, file publication and malformed-input cases.
- Formatting, Clippy with warnings denied, workspace build and the architecture check pass.
- Real native keyboard events verify key insertion, undo, redo, playback and frame stepping
  in a separate task-owned review window; that window is closed after verification.
- Original fixtures retain 13,770 direct-runtime transform comparisons below 0.001 tolerance.
- The shared packed exporter is also checked with the external runtime. The harness now
  verifies actual PNG dimensions rather than assuming every atlas page is 512×512.
- Exported demo JSON is unchanged from the successful Spine Editor round trip retained
  during the first slice. No proprietary implementation or artist asset was changed.

Hosted CI remains manual-only and has not been run. The dependency check enforces crate
edges, not a formal proof of effect freedom; core functions were also inspected for
filesystem, clock, process and UI calls.

## Expansion sequence

Introduce stable bone/animation IDs before deletion, reparenting or merging; indices
remain sufficient for the append-only operations supported today. Add mesh data to the
native model and evaluated scene before building mesh controls. Version native-format
changes explicitly: unknown data is now refused rather than silently lost.

History still uses bounded full-document snapshots. Structural sharing should follow
measured mesh/keyframe workloads. Inspector keyboard edit grouping, typed recoverable
command errors and command-schema discovery can be added when their next consumers need
them; this review does not claim those capabilities already exist.
