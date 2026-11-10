# Skin-required bones and constraints — November 9, 2026

Four game skeletons put bones and constraints inside skins (hair, hats, faces):
such items are active only while a skin that lists them is active. Native v12
stores `Document.skin_scope` (required bones, required constraints, and each
named skin's lists); the Spine adapter reads/writes bone and constraint
`skin: true` and skin `bones` / `ik` / `transform` / `physics` lists.

Activity, observed with the external 4.3.13 runtime:

- A required bone is active when the active skin lists it **or one of its
  descendants** (listing a child activates its parents).
- A constraint is active when it is not required or its skin lists it, and the
  bones it reads and writes are active.
- The default skin (or no skin) activates no required item.

Inactive constraints are skipped in evaluation; inactive bones' slots are not
drawn and their bones are hidden and unpickable. The active skin lives in the
Session as view state (outside history), synced from the shell, so viewport,
inspectors, tools and the CLI agree. The CLI `pose` output reports each bone's
`active` flag, and `scripts/check-runtime.mjs` checks activity per skin.

Refused: path constraints in skin lists, a unified skin `constraints` list,
listing non-required items, and non-required bones under required parents (the
editor never writes those; their runtime result is not modeled).
