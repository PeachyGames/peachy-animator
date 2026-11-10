# Draw order keys — November 10, 2026

Eight game skeletons key draw order. Native v16 adds `Animation.draw_order`:
stepped keys holding Spine's offset lists exactly as written.

Black-box observations (original probes; no implementation source read):

- Each listed slot moves to its setup index plus its offset; unlisted slots
  fill the remaining positions in setup order. This matched the external 4.3.13
  runtime for 301 random encodings of a 9-slot skeleton, including a key
  without offsets, which restores setup order.
- Offsets out of setup order or listing a slot twice make the runtime's reader
  throw. Colliding, out-of-range or fractional offsets leave empty draw
  positions. Import refuses all of these.
- Installed Spine 4.3.26 keeps every order but re-encodes offsets minimally
  (slot count minus the longest run of slots that keep their relative order, in
  81 of 81 keys) and drops zero offsets it does not need. Peachy keeps offsets
  as written, so it round-trips both editor output and hand-written lists.
- Before an animation's first key, draw order is the setup order.

The scene draws slots in the sampled order, so every view, ghosting and
picking (topmost first) follow it. The runtime check now compares our draw
sequence with the runtime's applied draw order at every sample.

Not yet: authoring draw order keys, Dopesheet rows for them, draw order
folders (`DrawOrderFolderTimeline`), and constraint-modified draw order.
