# Image sequences and blend modes — November 10, 2026

The bonfire, the last game skeleton that did not import, plays a 96-frame
flame sequence and draws a glow with the screen blend mode. Native v22 adds
`SequenceDef` on regions and meshes (count, start, digits, setup frame),
`Animation.sequence_tracks` (mode, index, delay keys) and `Slot.blend`.

Observed with the external 4.3.13 runtime's public API (original probes; no
implementation source read):

- Frame paths are the base path plus (start + index), zero-padded to `digits`.
  Defaults: start 1, digits 0, setup frame (`setup`) 0.
- Frames advance by ⌊elapsed ÷ delay + 1e-5⌋ from the key's index. `once`
  clamps at the last frame, `loop` wraps, `pingpong` reflects over
  2·(count − 1), and the reverse modes map frame f to count − 1 − f. `hold`
  shows the index. Before the first key the setup frame shows.
- A key without `delay` carries the previous key's delay (0 for the first key);
  `mode` (default hold) and `index` (default 0) do not carry.
- Non-looping playback stops clip time at the clip's end, so held poses past the
  end keep the last frame (bone keys hold anyway).

Draws carry the frame's image path and the slot's blend mode, and the desktop
loads every frame image. egui composites premultiplied colors: additive draws
exactly (zero-alpha premultiplied tint), screen is drawn additively (slightly
brighter than true screen), and multiply falls back to normal blending.

Not yet: authoring sequences and blend modes in the editor, exact screen and
multiply compositing in the viewport.
