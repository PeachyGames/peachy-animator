# Agent access through MCP

Peachy Animator provides a local **stdio MCP server** named `peachy-mcp`. Agents
can open, inspect, edit, undo, evaluate and save animations without starting the
native window. The server calls the same document, command, Session, checked pose
evaluation and file services as the desktop. It uses the official
[Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk), pinned by Cargo.lock.

Each server process owns an independent headless document and history. It does
**not** attach to a running desktop editor, change its selection, refresh its open
document or consume that window's unsaved changes. Saved projects can be opened in
the native editor. A live desktop bridge and raster PNG previews remain future
work; `evaluate_pose` currently returns checked geometry and transforms.

## Start and connect

Build from the repository root:

```sh
cargo build -p peachy-mcp --locked
```

Configure an MCP client to launch the resulting executable with an explicit,
absolute workspace directory:

```json
{
  "mcpServers": {
    "peachy-animator": {
      "command": "/absolute/path/peachy-animator/target/debug/peachy-mcp",
      "args": ["--root", "/absolute/path/animation-workspace"]
    }
  }
}
```

The `mcpServers` wrapper is a common client configuration shape; clients with a
different configuration format use the same executable and argument list. Supply
the actual binary location if using `CARGO_TARGET_DIR`. No HTTP port, browser,
display, network connection or API key is required at runtime. Standard output is
reserved for MCP messages; startup errors and help go to standard error.

`--root` is required and must be an existing directory. Use a directory containing
the projects and loose images the agent should work with, plus a place for fresh
outputs. Relative tool paths resolve against this directory, independent of the
client's working directory. Absolute paths are accepted only inside that root.

## Tools

| Tool | Behavior |
| --- | --- |
| `get_document` | Complete native document, current revision, dirty flag, undo/redo availability, source and animation/skin inventory. Before Open it returns revision 0 and no document. |
| `inspect_file` | Read-only native validation or Spine capability report. Does not replace the working document. |
| `open_project` | Load `.peachy` (including supported legacy migrations) or supported Spine 4.3 JSON into a new Session. Failed imports leave the current session intact. |
| `get_command_schema` | Full JSON Schema for the 24 supported flat command variants; the same schema appears inline in `apply_commands` discovery. |
| `apply_commands` | Apply a nonempty flat list through one engine Batch, atomically and as one undo step. No file writes. |
| `history` | Undo or redo one complete edit using the shared Session history. |
| `evaluate_pose` | Setup or a named animation at a nonnegative time in seconds, with optional skin. Returns authored locals, IK-applied locals, world matrices, textured vertex/UV/index buffers and bounds. |
| `save_project` | Create a new directory containing `project.peachy` and the required loose `images/`; marks the headless Session saved. |
| `export_project` | New Spine JSON file, or a fresh packed JSON/atlas/PNG directory inside an existing parent. Does not mark the native document saved. |

All tools publish parameter schemas, descriptions and behavior annotations through
MCP `tools/list`. Successful results include both `structuredContent` and matching
serialized text for older clients. Operational failures return `isError: true`
with the current revision; malformed tool arguments use the SDK's protocol error.
Read-only tools do not change session state.

## Editing workflow

First call `get_document` to discover the current revision. A fresh process starts
at 0. For the original fixture with the repository itself as `--root`:

```json
{"path":"examples/peach-bot/peach-bot.peachy","expected_revision":0}
```

Pass that object to `open_project`. A successful Open returns revision 1. Read
`get_document` for current bone, slot and animation indices, then call
`apply_commands`, for example:

```json
{
  "expected_revision": 1,
  "commands": [
    {"command":"renameBone","bone":0,"name":"agent-root"},
    {"command":"setBoneLength","bone":0,"length":42}
  ]
}
```

The result is revision 2. Both edits form one undo step. A later command failure
rolls back the entire list, including any preceding successful command in that
list. Unknown fields and nested `batch` commands are rejected; the server owns
the outer atomic batch.

To evaluate, call `evaluate_pose`:

```json
{"animation":"wave","time":0.3,"skin":"peach"}
```

Omit `animation` for Setup, `time` for zero and `skin` for `default`. Bone-only
documents with no skins are supported. Bind commands require a complete
**applied-local** snapshot from `appliedLocals`; this captures the checked IK result
once. Do not substitute authored `locals` for this snapshot. Transform commands
require seven fields: `x`, `y`, `rotation`, `scale_x`, `scale_y`, `shear_x`,
`shear_y`. SetKey values use the native setup-relative convention; scale keys
multiply setup. Shared engine validation and unsupported-feature gates apply.

Save with `save_project`:

```json
{"expected_revision":2,"directory":"agent-edits"}
```

The directory must not already exist, and its parent must exist. The result points
to `agent-edits/project.peachy`; copied images make it usable when opened in the
desktop. Save advances revision to 3. Export a packed runtime bundle with:

```json
{"expected_revision":3,"kind":"bundle","path":"."}
```

The response identifies the new timestamped bundle directory. JSON-only export
uses `kind:"json"` and a fresh `.json` output path instead.

## History and file boundaries

Open, Apply, Undo, Redo and Save advance a monotonic revision. Each of those tools,
and Export, requires `expected_revision`; a stale value fails without side effects.
Re-read state before retrying. Revisions belong to one server process and reset
when it exits. Concurrent requests serialize around the same Session; independent
server processes do not share documents or history.

Opening another document refuses to discard unsaved edits unless the call
explicitly supplies `discard_changes:true`. Successful Save marks the saved native
document clean; Export preserves the native dirty baseline. Unsupported imports,
invalid edits and failed output operations preserve the Session and revision.

Primary file paths, output parent directories and every loose image used by Save
or bundle export are canonicalized and checked against `--root`. Parent traversal,
symlinks resolving outside the root, existing outputs and dangling output symlinks
are rejected. JSON-only export does not read images. Loading and inspecting a
project do not require all images to exist; operations that need images fail
explicitly if any are missing or outside the root. Artist inputs are never
overwritten. Save creates its own fresh directory, copies through `peachy-io`, and
removes that owned directory if saving fails. No arbitrary code or shell tool is
exposed.

These are application-level checks for a cooperating local filesystem, not an OS
sandbox against another process changing filesystem paths during an operation.
Keep independent jobs in separate output directories. The server does not publish
games, issue ownership keys, modify account settings or broaden the currently
supported Spine feature subset.

## Verification

```sh
cargo test -p peachy-mcp --locked
cargo clippy -p peachy-mcp --all-targets --locked -- -D warnings
python3 scripts/check-architecture.py
```

The Rust protocol tests launch the actual binary and connect with the official SDK
client over stdio. They cover initialization/discovery, inline command schemas,
matching structured/text results, atomic batch failure, stale revisions, dirty
Open refusal, undo/redo, evaluated geometry, native/Spine import failures, fresh
Save with image copying, JSON and packed exports, source-byte preservation, and
primary/output/image symlink escapes. A schema test decodes and reserializes every
published flat command shape using strict engine serde. Exact native GUI behavior
and external runtime compatibility remain covered by their separate project
acceptance checks; MCP does not establish additional Spine feature parity.
