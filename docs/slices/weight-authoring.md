# Initial mesh binding and Weights view

This slice supports binding an unweighted active mesh to exactly one bone and
removing all bindings. The engine preserves the mesh's current world shape in
Setup or Animate, including a pending unkeyed pose. These operations edit mesh
setup data; they do not key, discard, or serialize the transient bone pose.

## Public references and conventions

- [Weights view](https://esotericsoftware.com/spine-weights): mesh-first and
  bones-first binding, a bone list, Remove, and saved per-tool selection.
- [Tools](https://esotericsoftware.com/spine-tools): Weights uses G; C/V/X select
  transform tools, and right-click switches the last two tools.
- [Views](https://esotericsoftware.com/spine-views): named views, resizing, Views
  visibility, and F9 hide/restore.
- Public control images: [Weights view](https://esotericsoftware.com/img/spine-user-guide/weights/view.png)
  and [Weights tool](https://esotericsoftware.com/img/spine-user-guide/weights/weights-tool.png).

The references were read with Agent Reach's Exa backend. No proprietary editor or
runtime implementation was inspected. Icons are independent vector drawings.
No successful installed-editor capture establishes the exact default Weights
dock. The resizable pane in our existing right-side column is provisional;
movable tabs, arbitrary docking, and exact default placement remain gaps.

## Supported interaction

G opens the Weights view and selects the real Weights tool. The view also has a
Views entry. Its explicit mesh, vertex, and binding-bone context survives normal
bone selection and transform-tool changes. Normal transform commands continue
to address only the typed Tree selection. Temporary Mesh inspection retains its
separate return-tool behavior.

Select an active unweighted mesh, click Bind, then click one bone in the Tree or
viewport. Alternatively select one bone first, click Bind, then click an active
unweighted mesh. Entering Bind mode does not dispatch a command. Escape, Space,
or clicking Bind again exits without rolling back completed work. Each concrete
attempt exits on success or failure, with no automatic retry. A successful
first binding gives every vertex one influence of weight 1.

Mode changes, animation activation, document or selection history, project open, and view hide/restore
cancel transient Bind mode while retaining valid Weights selection. A time scrub
may retain Bind mode; the concrete target click captures the then-current pose.
This scrub policy is explicit and has not been verified as exact Spine behavior.

Remove all removes every bound bone and preserves the current mesh shape. It is
deliberately labeled as a complete removal, rather than implying that the
selected bone alone is removed. Both actions pause playback, cancel active
gesture previews, finalize a prior numeric transaction, and use Session's
explicit Setup or Animate bind-pose APIs. Singular transforms or excessive
coordinate residuals reject atomically in the engine.

Context reconciliation clears stale targets, inactive appearances, and meshes
whose slot placeholder is no longer active. Active-skin default fallback resolves
to an explicit attachment reference. No cached binding bone redirects a normal
pose edit. Opening another project resets Weights context.

## Limits and native acceptance

The bone list and vertex handles inspect weights. Direct adjustment, Auto,
Update, adding bones to an already weighted mesh, normalization, painting,
locks, partial removal, weight clipboard, and multiple vertex/bone selection
are not implemented and have no fake active controls. Weight percentages are
read-only; repeated influence entries and bind positions remain unchanged.

Root's native acceptance should check initial binding, Remove all, current-shape
preservation, Undo/Redo, save/reopen, pending Animate pose/time, inactive-skin
rejection, text/modal shortcut priority, and G/transform/right-click history.
Export/runtime compatibility and UI observation are separate acceptance gates.
Headless shell checks do not establish exact UI placement or mouse parity.

F12 records `weightsContext` with mesh, nullable vertex/bone, bindMode,
worldVertices, handles, and visibility. Targets include weightsTool,
weightsBind, weightsRemoveAll, viewWeights, weightsPanel, weightsBone:index,
and weightsVertex:index. World vertices and handles remain available when the
normal Tree selection is a bone.
