# Animation naming and duplication

This slice adds serializable `RenameAnimation` and `DuplicateAnimation` commands
through the existing pure command boundary and Session history. Renaming changes
only the indexed clip's name. Duplication appends a full owned copy: duration,
tracks, scalar keys, fractional key times and interpolation curves remain exact.
Native document format and Spine adapter rules do not change. Empty names,
duplicate names and unknown animation indices fail without changing the document
or its undo/redo branch.

The selected animation's properties expose Rename and Duplicate actions. Rename
uses the shared name dialog, with Enter/OK to commit and Escape/Cancel to discard.
Duplicate uses the same dialog presentation and requires a new unique name. After
successful duplication, the new clip is selected for inspection; the active clip,
playhead and Setup/Animate mode stay unchanged. Its activation is an explicit action.
Rename is available in both modes and retains the clip's active identity.

The public [Tree guide](https://esotericsoftware.com/spine-tree) documents the
properties actions and the F2/double-click rename dialog; the
[Animations view](https://esotericsoftware.com/spine-animations-view) distinguishes
selection from animation activation. These are behavioral references, not source
code or bundled UI assets. The public guide does not establish whether Spine
automatically activates a duplicated animation; that exact post-duplicate policy
requires a separate installed-editor observation. The current inspection-only
policy is explicit, not a claim of verified Spine behavior.

Four engine tests verify identity/evaluation preservation during rename, no-op
rename history, exact independent cloning with all curve types, native round trips,
undo/redo, strict command decoding and atomic invalid batches. Root integration
adds final native mouse/keyboard verification, export checks and shared validation
records. Creation, deletion, duration editing, animation folders and multi-clip
selection remain outside this slice.
