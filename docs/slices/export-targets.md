# Explicit Spine Editor and Runtime export targets

Verified November 14, 2026, following original installed-editor probes at
`f345fdd`. **Native, CLI, actual stdio MCP, filesystem and bounded runtime
acceptance pass.** See the [review gallery](../evidence/export-targets.html)
and [verification record](../evidence/export-targets.json). The accepted targets
do not establish arbitrary Spine Editor round trips or pixel equivalence.

## Problem and observed reference

Spine JSON distinguishes a skin's slot/placeholder lookup key, an attachment's
optional actual `name`, and its image `path`. The
[public JSON format guide](https://esotericsoftware.com/spine-json-format)
describes those distinct roles and default-skin fallback. Some details on that
page are from older formats, so the installed 4.3.26 tests below determine the
bounded interchange policy.

The coordinator imported independently authored minimal JSON into installed
Spine 4.3.26, then exported it again without editing it. The original fixtures
contain their own small region or mesh data; no proprietary implementation or
artist artwork was inspected or modified. Private inputs and scalar results
are under `.local/training-spine-20261114/names-contract/`, principally
`native-observations.json`.

| Original case | Observed installed 4.3.26 import/export |
| --- | --- |
| Default region: placeholder `ph`, explicit actual name `ph` | Setup attachment and visible/hide/visible attachment keys retain `ph`. |
| Default region: placeholder `ph`, actual name omitted | Same successful identity/key preservation as the equal-name control. |
| Default region: placeholder `ph`, actual name `pretty` | Attachment is exported under `pretty`; Setup attachment becomes unset. Previously visible `ph` keys become unnamed/hide entries, while their times remain. |
| Named-skin region: placeholder `ph`, actual name `pretty` | The named skin retains placeholder `ph` and actual name `pretty`; Setup and attachment keys still address `ph`. |
| Valid default plain mesh: actual name equals placeholder | Setup/attachment keys and the placeholder used by Deform remain intact. |
| Valid default plain mesh: actual name overrides placeholder | The attachment is moved to `pretty`, Setup and visible attachment key names are cleared, and the Deform reference is moved to `pretty`. |
| Default source mesh override with a linked mesh | The same source-name/key loss occurs; the linked source reference is rewritten to `pretty`. |

Both import and export exited successfully for these controls. One initial
`default-mesh-override` input used an invalid linked `parent` field and failed
import; it is excluded from successful behavioral evidence. The later valid
plain-mesh and linked-mesh controls are the relevant comparisons. Successful
CLI exit alone is therefore insufficient:
the resulting attachment references and visible keys must also be compared.

## Target contract

The selected policy preserves the native document and runtime metadata. It
does not silently delete actual names, rename placeholders, rewrite animation
keys, duplicate artwork or flatten skins to make Editor import succeed.

| Target | Intended consumer | Bounded export policy |
| --- | --- | --- |
| `SpineEditor43` | A new project imported into Spine Editor 4.3 | Default target. Validate the supported document plus known Editor interchange restrictions before output. Reject explicit default-skin names that differ from their placeholders, and retain the existing named-skin/default-placeholder collision guard. Name-equal/implicit defaults and noncolliding named-skin overrides remain eligible. |
| `Runtime43` | A compatible Spine 4.3 runtime consumer | Validate the supported document/adapter semantics while preserving independent actual names, placeholder keys, image paths, slot keys, Deform references and linked source metadata. Do not apply Editor-only naming restrictions. |

Runtime output is not a guarantee of safe import into Spine Editor. Editor
output is not a lossless `.spine` project converter or proof of every Editor
behavior. The existing feature/version subset and all other adapter guards
still apply. Neither target modifies `Document`, pending poses, history or the
native saved baseline.

The name-override guard applies to supported attachment name metadata. Direct
new native evidence covers regions and the valid plain/linked mesh controls
above; other attachment-specific name edge cases are not claimed as native
equivalence. The collision rule is separate from a global name-uniqueness ban:
reused actual names across slots or named skins remain valid within the native
model and supported runtime contract.

## Shared integration boundary

The current implementation surface uses adapter `ExportTarget::{SpineEditor43,
Runtime43}` and target-aware `export_json_for` / `export_bundle_for` functions.
Existing convenience functions retain the Editor default. The adapter owns
the naming decision; native UI, CLI, confined I/O and MCP must call the same
policy rather than keeping independent copies of it.

- Native export exposes the consumer choice before writing output. Its wording
  explains the consumers and reports actionable naming errors. Each new chooser
  defaults to Editor; Runtime is an explicit choice. Changing this export choice
  is view state, not an edit to the rig.
- CLI `export` and `bundle` accept `--target spine-editor43|runtime43`; omission
  keeps the Editor default. Unknown targets fail without writing an output.
- MCP `export_project` accepts strict `target: "spineEditor43" | "runtime43"`
  with the same default, for JSON and packed bundles. Its response identifies
  the selected `exportTarget`. Existing revision checks, confined paths and
  no-overwrite behavior remain in force; export does not advance revision or
  publish a game. This is an export-tool option; the existing 45 commands / 11
  tools remain unchanged.
- Validate before allocating the final file/bundle directory. A rejected Editor
  export leaves no partial destination, and source images/documents remain
  unchanged. Runtime export must not mark the editable project saved.

## Verified acceptance

Actual CLI JSON/bundle and stdio MCP exports pass the explicit/default target,
invalid-argument, no-partial-output and identity-preservation checks. The MCP
run leaves document, revision, saved baseline and undo/redo state unchanged;
all 11 checked source hashes are unchanged. Runtime JSON and bundle files from
CLI are identical to MCP output. Private reports are `cli-export-report.json`
and `mcp-export-report.json` in the local names-contract record.

Actual native foreground input verifies a typed pending rotation of `13.5`,
chooser Cancel/Escape, blocked K/frame shortcuts, Editor rejection without
output, and a successful explicit Runtime bundle for the same source. The
complete document, pending pose, saved baseline and redo branch survive; the
pending rotation is excluded from the bundle. All three bundle files are
byte-identical to CLI/MCP output. The folder path and Open confirmation used
verified task-owned Cocoa accessibility controls after keyboard delivery failed
safely. Native input coverage combines foreground mouse/keys and the owned
Cocoa controls. The owned editor process was closed and its
recorded PID verified exited. See `native-export-report.json`.

Four adapter-produced Editor controls were imported/re-exported successfully
by installed 4.3.26: equal-name default region, implicit-name default region,
noncolliding named-skin override, and equal-name default plain mesh. Setup and
swap lookup identities survive in all four; the mesh's Deform reference also
survives. Region cases have no Deform timeline. This is identity/key acceptance,
not a full `.spine` or image-size conversion claim. The scalar comparison is in
`editor-target-acceptance.json`.

Runtime output passes the external Spine 4.3 oracle:

- Slot Lab, three clips/all three skins: 42,120 bone values, 49,032 region
  values and 24,516 color values. Maximum bone/region errors are approximately
  `2.6534e-5` / `3.0518e-5` at the unchanged `0.001` geometry tolerance.
- An original minimal mesh case combines a colliding named-skin/default lookup
  key, independent actual-name overrides, Deform and a linked mesh. Its 168
  mesh coordinates, 336 UV values and 224 region values have zero error. Its
  624 bone values differ by at most `2.84e-16`.

These results are in `runtime-auto-report.json` and `runtime-mesh-report.json`.
They establish supported runtime semantics for the tested inputs; they do not
turn Runtime output into safe Editor input.

Focused checks pass; filters overlap and must not be added as a full-suite
total:

| Check | Passing cases |
| --- | ---: |
| Adapter export targets | 5 |
| I/O export target | 1 |
| Targeted actual stdio MCP | 1 |
| Existing MCP protocol | 7 |
| Desktop export chooser | 6 |
| Desktop Color | 17 |
| Linked mesh | 2 |

Formatting, all-targets Clippy, workspace build and architecture boundaries for
all seven crates pass. The public gallery
contains three original native screenshots; private scalar reports and source
fixtures remain in the task record.

No broad migration of existing private artist files, removal of name metadata,
new attachment model, new native format, or relaxation of unrelated import
validation is part of this slice. Hosted CI has not passed and remains a separate
integration gate. Upcoming curve reset/final-key semantics are tracked in
[the blog continuation audit](spine-blog-audit-20261114.md).
