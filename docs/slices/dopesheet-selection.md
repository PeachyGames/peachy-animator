# Dopesheet key selection and deletion

The November 5, 2026 public behavior reference is the
[Dopesheet guide](https://esotericsoftware.com/spine-dopesheet) and
[key deletion guide](https://esotericsoftware.com/spine-keys). The parent integration
agent inspected these references and the public property-row images. No editor or
runtime implementation code was inspected.

Clicking a key selects it and jumps to its exact source time. Cmd/Ctrl-click toggles
that key in the selection; Shift is not an additive-selection shortcut. Clicking
empty space clears selection and retains frame-snapped scrubbing. Key selection is
independent of the playhead, so playback and keyboard frame/key navigation do not
select every property at that time. Narrow vertical bars distinguish rotation
(green), translation (blue), and scale (red); selected bars use Peachy's accent.

Delete or Backspace removes selected keys in one shared `Batch` command only while
the Dopesheet canvas has focus. Text editors, other controls, open menus, Setup,
hidden views and error dialogs cannot route those keys to this operation.
Double-clicking a key deletes that exact key. Clicking elsewhere relinquishes the
canvas focus. Existing transport shortcuts still work when the Dopesheet has focus.

`dopesheet_selection.rs` is a pure presentation model; it stores animation, bone,
property and exact source time. Context changes clear the selection; document
changes prune missing keys. Opening another project resets it. Undo restores deleted
data through Session, while removed UI selection stays cleared. Key selection is
never written into native files or exports.

The existing engine `DeleteKey` command now prefers exact source time. When there is
no exact key, the existing epsilon tolerance remains available for one unique match;
multiple nearby candidates cause an error. This prevents one selected fractional
key from removing its valid close neighbor. A failed batch changes neither the
original document nor undo/redo history. No native schema or adapter changed.

The slice deliberately covers the selected bone's five supported scalar transform
tracks. Row hierarchy/summary rows, select-all, box selection, key dragging,
copy/paste, key context menus and preferences remain pending. Fractional source keys
can be selected/deleted exactly; general fractional-frame authoring is still pending.
The root integration agent owns native UI checks and final verification evidence.

F12 diagnostics expose `selectedKeys`, `dopesheetFocused`, `timelineCanvas`, and a
`timelineKey:<Property>:<index>` target for every displayed source key. These support
native acceptance checks for single/additive selection, playhead independence,
focus-gated deletion, double-click and one-step undo without relying on pixel guesses.

Local lane checks pass: formatting, warnings-denied Clippy, workspace build, six-crate
architecture boundaries, and 50 Rust tests (9 desktop, 26 engine, 3 CLI, 4 I/O,
8 adapter). New tests exercise exact fractional selection, context/stale-key pruning,
one-step multi-key undo/redo, close-neighbor preservation and failed-batch atomicity
without consuming an existing redo branch. Native UI verification is an integration
gate, not established by these unit tests.
