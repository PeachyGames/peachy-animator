# Next slices

The first slice proves a complete bounded workflow: pose an original rig, add keys,
scrub/play, edit curves, undo/redo, save/reopen and export to a separately supplied
Spine runtime. Subsequent work should grow that working loop.

1. **Spine UI parity.** Match Spine's layout, controls, terminology and conventions.
   The workspace and shortcut alignment is followed by a typed Tree, contextual
   selection, filters, activation dots, viewport picking and unkeyed/Auto Key posing.
   Rotate/Translate/Scale and axis controls now follow the public reference.
   Selection history, single-item groups, reverse playback and key navigation are also available.
   Key retiming/duplication and focused clipboard actions are available. Next extend
   box selection, then Graph/view tabs and
   multi-selection in [UI parity](UI_PARITY.md).

2. **Weight authoring and broader mesh editing.** Unweighted editing and weighted
   import/export, deterministic bone-driven evaluation and read-only influence
   inspection now work through native v4. Initial one-bone Bind and Remove-all
   have shared commands, a G Weights view/tool and native acceptance. Additional
   zero bindings and one-vertex/bone Direct changes now have explicit shared
   commands and percentage controls. Preserve their separate validation gates and
   provisional dock status. Next establish Update bindings, individual removal,
   multi-vertex/bone selection and locks through controlled reference observations.
   Sparse/repeated editing, zero-recipient alternatives and brush behavior remain
   bounded gaps, rather than reasons to normalize or merge imported data.
   Follow with UV/hull editing and controlled triangulation. Linked meshes, deform
   timelines and constraints remain major gaps for existing characters; weighted
   support alone leaves 0/11 sampled production exports fully supported.
   Use the [workflow milestones](WORKFLOWS.md) and private asset inventory to select
   the smallest representative production character and track its missing features. Refuse unsupported imports
   without changing their source files.
3. **Artwork and rig creation.** Import layered/loose images, create attachments,
   edit pivots and draw order, rename/delete/reparent with stable IDs. Add robust
   atlas import so artist exports can be reviewed without loose originals.
4. **Constraint parity.** Begin with one- and two-bone IK, then transform/path
   constraints and physics. Specify each through controlled fixtures and compare
   observed outputs at boundaries as well as ordinary poses.
5. **Animation editing.** Exact selection, copy/paste and group retiming now work.
   [Key-derived duration](slices/key-derived-duration.md), empty clips and beyond-end
   authoring are now implemented. Next add broader selection, animation creation UI,
   extended playback, ghosting and timeline/curve editor refinement. Group
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
