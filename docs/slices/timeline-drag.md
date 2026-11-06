# Dopesheet key retiming

The public [Dopesheet guide](https://esotericsoftware.com/spine-dopesheet) describes
dragging selected keys, Shift to disable frame snapping and Cmd/Ctrl+Shift at the
start of a drag to duplicate. The coordinator inspected these public references;
no editor or runtime implementation was read. This slice independently implements
those controls for the selected bone's supported scalar transform tracks.

Pressing a key captures its exact source identity, the current key selection,
pointer origin, modifier state and chart geometry. If the pressed key was not
selected, the drag acts on that key alone. Otherwise it preserves the selected
group. Duplication is captured at press, so releasing Cmd/Ctrl or Shift before the
movement threshold does not change a copy into a move. Shift's snapping override
uses its current state while moving.

The pressed key's destination snaps to a full frame by default. The same offset
is applied to the group, preserving relative timing, including fractional source
times. Shift allows fractional destinations. The earliest destination is clamped
at zero without changing group spacing. Keys may move past the old duration; the
engine extends the native clip. The chart's captured origin, width and duration
stay fixed until release so a preview cannot rescale keys under the pointer.
Auto-pan/zoom beyond the visible chart remains pending.

`key_drag.rs` holds transient input state and pointer-to-time mapping.
`Session::preview_transaction` evaluates each MoveKeys command against the same
transaction baseline, rather than incrementally moving the prior preview. That
restores overwritten destinations when the pointer moves away and retains exact
source references throughout a gesture. Values and curve payloads stay in the
engine; the desktop does not rebuild keys or maintain a second undo stack.

Release commits one history entry. Escape cancels even while copy modifiers are
held. A context change, focus loss, hidden Dopesheet, Save, Export or Open cancels
the new gesture before continuing, restoring both the document and the prepress
key selection. Active-drag Undo/Redo cancels the gesture and returns without
consuming another history entry. Other document Undo/Redo clears key selection
so an overwritten destination restored by history cannot inherit a moved key's
selection and be accidentally deleted. Bone/Tree selection is unchanged.
Delete also cancels before building key references. Bone nudges are suppressed
while the key gesture is pending or active. Right-button drags do not retime keys.

The playhead stays unchanged during retiming. Pending poses are retained in the
transaction baseline and are never implicitly keyed. Exact installed-editor
playhead/curve-handle adjustment behavior, ripple Shift/Offset, box/range scaling,
multi-bone rows and other timeline types remain separate parity work.

F12 diagnostics retain the existing timeline key/canvas targets and add `keyDrag`:
active flag, animation, bone, exact sources, offset, copy flag and fixed chart
geometry. The coordinator owns native verification and export/runtime evidence.

Native acceptance should cover:

1. Drag one key, then a selected group; confirm snapped destinations and preserved
   values, curves and spacing. Undo and Redo once each.
2. Shift-drag to a fractional destination; press Cmd/Ctrl+Shift, release modifiers
   before moving, and confirm duplication remains enabled.
3. Preview over an occupied destination, then move away; confirm that destination
   reappears. Escape restores the original project and prepress selection.
4. Extend beyond the old last frame; confirm chart geometry stays fixed during the
   gesture and clip duration extends only in the committed document.
5. Cancel with held modifiers, mode/animation/selection/tool/playhead changes,
   hidden view, window focus loss, Save/Export, and active-drag Undo/Redo. Preserve
   an existing redo branch. Delete after completed Undo must not target a restored
   overwritten key through stale selection.
6. Right-drag leaves key times unchanged. A normal click still selects/jumps;
   empty-space primary dragging still scrubs.


Local lane checks pass with the core prerequisite: formatting, warnings-denied
Clippy, workspace build, six-crate architecture check and 72 Rust tests. The two
new pure drag-model tests cover fixed mapping, fractional-key snapping/group clamp,
source selection and cancellation metadata. The lane uses its own target directory;
no Cargo dependency or native schema changed. Native interaction verification is
an integration gate and is not established by these unit tests.
