# Next slices

The first slice proves a complete bounded workflow: pose an original rig, add keys,
scrub/play, edit curves, undo/redo, save/reopen and export to a separately supplied
Spine runtime. Subsequent work should grow that working loop.

1. **Meshes and weights.** Add typed unweighted meshes, triangulation and UVs; render
   them; then add bone weights and weight painting. This is the largest immediate
   gap for our existing characters. Preserve unsupported data until it is supported.
2. **Artwork and rig creation.** Import layered/loose images, create attachments,
   edit pivots and draw order, rename/delete/reparent with stable IDs. Add robust
   atlas import so artist exports can be reviewed without loose originals.
3. **Constraint parity.** Begin with one- and two-bone IK, then transform/path
   constraints and physics. Specify each through controlled fixtures and compare
   observed outputs at boundaries as well as ordinary poses.
4. **Animation editing.** Key selection, copy/paste, retiming, editable clip lengths,
   animation creation UI, ghosting, and timeline/curve editor refinement. Group
   inspector drag changes into one undo transaction, as canvas drags already are.
5. **Agent interfaces.** Publish a discoverable command registry and schemas, then
   MCP/JSON-RPC around the same dispatcher. Add batch screenshot and render commands.
6. **Distribution.** Package a macOS application with bundled demo resources; verify
   Windows/Linux and WASM builds. Add additional Spine export targets deliberately,
   each with a recorded compatibility matrix.

Do not let native `.spine` decoding or a new runtime ecosystem block the editor.
The near-term target remains independently authored, version-specific exports used
with the existing, separately licensed Spine runtimes.
