# Events, reference scale and per-type constraint names — November 9–10, 2026

Four game skeletons define events. Native v15 adds `Document.events` (name,
int, float, string, optional audio path, volume, balance) and
`Animation.events` (timed keys that may override int, float, string, volume
and balance). Events never change the pose; they are data for the game.

Observed with installed Spine 4.3.26 and the external 4.3.13 runtime, using an
original probe:

- The editor writes a definition's defaults as omitted fields (`"bare": {}`)
  and keeps a key's overrides exactly as written.
- How omitted key values resolve differs by consumer (the runtime, for
  example, only resolves volume and balance for events with audio). Keys
  therefore store only what was written (`Option` per field), never a
  resolved copy.
- Event key times extend the animation's duration like any other key.
- The editor sorts simultaneous keys when it re-saves; the runtime fires them
  in file order. Peachy keeps file order.

Native v15 also stores `skeleton.referenceScale` (default 100). It only affects
distance-based physics, which Peachy does not yet simulate, so it is data only.

The doggy-scene female (`baking-doggy/skeleton_ humanoid_female.json`) also
needed two corrections the earlier slices had wrong:

- **Constraint names are scoped by type.** An IK and a physics constraint may
  share a name (`target6`, `target12`), and IK or physics timelines resolve
  names within their own type.
- **Bezier time handles may cross** (`[0.594, …, 0.406, …]` in `icing`). The
  runtime evaluates them normally, so they now only need to lie within the
  key interval.

Not yet: an Events view, authoring events in the editor and Dopesheet event rows.
