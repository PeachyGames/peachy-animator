# Bone and slot renaming

The public [Tree guide](https://esotericsoftware.com/spine-tree) describes a Rename
button at the upper right of item properties, F2, and double-clicking a Tree node to
open a rename dialog. This slice supplies the shared dialog and bone/slot commands.
The integration pass connects F2/Tree double-click and extends the same dialog to
animation names. Setup/Animate mode restrictions were not specified in the guide;
names are editable in both modes without changing setup transforms or keys.

The dialog focuses and selects the current name, confirms with OK/Enter, and cancels
with Cancel/Escape. An invalid name stays in the dialog with the validation error;
no command/history state changes. Editing text does not create document undo entries.
The right-side Properties header contains Rename for bones and slots. F12 target
names are `rename`, `renameName`, `renameConfirm`, and `renameCancel`.

`RenameBone` preserves the bone vector and all index references. `RenameSlot` moves
the slot's attachment maps across every skin while retaining slot order, bone,
placeholder, image paths and attachment contents. Empty/duplicate names and unknown
targets are rejected through the same pure `apply_command` boundary. A successful
rename is one undo entry; no-op renames do not dirty the document. Existing selection
reconciliation removes obsolete name-based attachment references from selection
history and numbered groups before the old slot name can be reused.

Local verification: 50 Rust tests pass (desktop 7, CLI 4, engine 27, I/O 4, adapter 8).
New tests cover index/hierarchy and all-skin preservation, unchanged scene geometry,
native serialization, command serialization, failed-batch atomicity and redo
preservation, selection/group pruning before name reuse, and Spine JSON export/import
references. Formatting, warnings-denied Clippy, workspace build and six-crate architecture checks
also pass. The native schema and Cargo dependencies are unchanged. Native pointer/
keyboard and installed-editor/runtime checks belong to the integration verification;
the local automated tests alone do not establish complete interaction/format parity.

Attachment/skin renaming, multi-rename/search-and-replace, deletion and reparenting
remain outside this slice. Stable entity IDs remain a prerequisite for destructive
structure edits. Public behavior and independently authored original fixtures were
used; no proprietary Spine/runtime implementation was inspected.
