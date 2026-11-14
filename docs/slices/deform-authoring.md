# Animate mesh vertices — November 14, 2026

Select an active mesh attachment, stay in Animate, click **Vertices**, and drag
or arrow-nudge one vertex. Weighted and unweighted meshes use the evaluated pose.
With Auto Key off this is a pending pose; **K** keys all pending bone, IK and
deform edits together. The colored **Deform** key in attachment properties keys
only that mesh. Auto Key writes keys directly; **Delete key** removes the current
mesh key. Setup vertices, UVs, weights and bindings remain unchanged.

## Core and history

`Command::SetDeformKey` and `DeleteDeformKey` use native v22 dense offsets, with
no format migration. CLI batches and the standalone MCP schema expose both.
Linked meshes resolve through `Document::deform_target`. Existing fractional
key identities and curves are preserved; new interactive keys follow the frame
grid. Keying refuses a snap that moves the current geometry by more than 0.001
world units. Near-final cursors resolve the same identity for key state and deletion.

Pending offsets live in `PoseEdits`, outside `Document`, with unified Session
history. Saves and exports exclude them. Bone-specific Tree keying remains
bone-only. Time/playback/context changes use the existing pending-pose policy.
Slot renames remap pending targets. Topology/influence-layout changes are refused
while deform keys or pending offsets depend on them, including equal-length
bind/unbind changes that would reinterpret the offset coordinate space.

World translation belongs to the engine. Unweighted deltas use the slot bone's
inverse linear transform. Each positive weighted influence receives its own
inverse-transformed world delta divided by the positive weight total. Repeated
bones, zero entries, nonunit totals and unselected offsets are preserved without
weight normalization. Singular positive contributions, nonfinite results and a
forward residual above 0.001 are rejected atomically.

Drags freeze the target, vertex, animation/time, Auto Key, zoom and full Session
transaction baseline, including pending bone/IK poses. Previews recompute from
that baseline; release commits one Undo step. Escape, focus/context changes,
Undo/Redo and Save/Export cancel an active preview. A canceled or keyed held
pointer cannot restart the edit until release.

## Texture loading

Before the first frame, egui reports a portable texture-side default of 2048.
Startup now seeds the created WGPU device's limit before uploading. PNG header
and decoded dimensions are checked before either normal or solid-ghost texture
is created. Exceeding the device limit reports the image name, dimensions and
limit instead of aborting. Originals are never resized. Reopening uses the same
guarded loader.

## Reference and limits

The public [mesh guide](https://esotericsoftware.com/spine-meshes#Transform-tools)
distinguishes Setup vertex movement from Animate deform keys and describes
individual vertex dragging outside UV/topology edit mode. Key colors and Auto
Key/K reuse the existing keying convention. An installed-editor reference opened
on the original Deform Lab, but exact mesh interaction was not established by
that attempt; the public guide is this slice's behavioral reference.

This is a bounded interaction, not full Spine UI parity. **Vertices** remains an
explicit entry point, with key/delete in attachment properties. Deform Dopesheet
rows, clipboard/retiming/curve UI, multiple vertices, rotation/scale and Animate
soft selection remain pending. Path/clipping vertices and weighted Setup
positions remain read-only. Auto/brush weights and freeform mesh/UV construction
are separate slices.

Private inputs and detailed captures remain in the gitignored local task record.
Committed fixtures/screenshots use original Peach Bot artwork. Integration
results are recorded in `docs/VALIDATION.md`.
