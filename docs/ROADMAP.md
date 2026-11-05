# Next slices

The first slice proves a complete bounded workflow: pose an original rig, add keys,
scrub/play, edit curves, undo/redo, save/reopen and export to a separately supplied
Spine runtime. Subsequent work should grow that working loop.

1. **Spine UI parity.** Match Spine's layout, controls, terminology and conventions.
   The workspace and shortcut alignment is followed by a typed Tree, contextual
   selection, filters, activation dots and viewport picking. Next complete unkeyed/
   Auto Key behavior, tool/axis controls, Graph/view tabs and multi-selection in
   [UI parity](UI_PARITY.md) before expanding the authoring UI.

2. **Weights and broader mesh editing.** Unweighted mesh import/export, rendering,
   region conversion, fixed-UV setup vertex transforms and triangle subdivision now
   work. Next add typed bone influences and deterministic weighted evaluation, then
   binding/weight editing. Follow with UV/hull editing and controlled triangulation.
   These remain major gaps for our existing characters. Refuse unsupported imports
   without changing their source files.
3. **Artwork and rig creation.** Import layered/loose images, create attachments,
   edit pivots and draw order, rename/delete/reparent with stable IDs. Add robust
   atlas import so artist exports can be reviewed without loose originals.
4. **Constraint parity.** Begin with one- and two-bone IK, then transform/path
   constraints and physics. Specify each through controlled fixtures and compare
   observed outputs at boundaries as well as ordinary poses.
5. **Animation editing.** Key selection, copy/paste, retiming, editable clip lengths,
   animation creation UI, ghosting, and timeline/curve editor refinement. Group
   inspector keyboard edits into one undo transaction. Pointer drags in the inspector
   and canvas already group their updates.
6. **Agent interfaces.** Publish a discoverable command registry and schemas, then
   MCP/JSON-RPC around the same dispatcher. `SetPose` already centralizes setup-to-key
   conversion for every client. Add batch screenshot and render commands.
7. **Distribution.** Package a macOS application with bundled demo resources; verify
   Windows/Linux and WASM builds. Add additional Spine export targets deliberately,
   each with a recorded compatibility matrix.

[The parity matrix](PARITY.md) maps the current subset to Spine behavior and records
the verification gates for expanding it.

Do not let native `.spine` decoding or a new runtime ecosystem block the editor.
The near-term target remains independently authored, version-specific exports used
with the existing, separately licensed Spine runtimes.
