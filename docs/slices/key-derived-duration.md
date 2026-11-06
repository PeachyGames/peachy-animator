# Key-derived animation duration

Status: implemented in native v3. Integrated source, native interactions, migration,
installed-editor round trip and external runtime results are recorded in
[VALIDATION.md](../VALIDATION.md#key-derived-duration-and-native-v3--november-5-2026).

Spine defines animation duration by the highest key. Empty animations and clips with
only frame-zero keys therefore have duration zero. Public references:
[Keys/Timeline](https://esotericsoftware.com/spine-keys#Timeline),
[Animation.duration](https://en.esotericsoftware.com/spine-api-reference#Animation-duration).
No proprietary implementation is an implementation reference.

The preceding v2 model stored a positive duration. Import floored it at 1/30 second;
export added implicit last-value holds or a neutral root rotation key. Moving a
last key earlier did not shorten the clip. The v3 change fixes these together with
an explicit migration.

## Contract

- Native v3 removes `Animation.duration`; `duration()` derives the maximum last key
  over validated ordered tracks, or zero. Reject a v3 `duration` field as unknown.
- Preserve actual keys on Spine import, including empty, frame-zero-only and tiny
  positive clips. Export actual keys without synthetic padding or root injection.
- Keep `AddAnimation { name, duration }` as a compatible command: zero creates an
  empty clip; a finite positive value creates a visible neutral root hold at that
  time. New animation UI should default to an empty clip.
- Separate a positive chart display extent from animation end. Finite nonnegative
  authoring time may exceed the last key; inserting a key naturally extends the end.
  Seeking and unkeyed posing must not change animation duration.
- Specify zero-duration looping and nonlooping transport separately. Never divide
  or take a remainder by zero. Exact empty-editor playback requires observation;
  do not infer its UI behavior solely from a runtime API.

## Preserve v2 exported behavior

Materialize the keys that the old exporter already emitted as visible editable
keys. Pad every nonempty track to its old duration when needed. For an all-empty
clip, reuse/create the root rotation track with the old neutral hold. Empty tracks
otherwise remain empty. This preserves exported property coverage and mixing
semantics as closely as the v2 exporter did.

Before appending a hold, normalize a formerly final unused Bezier hint to Linear,
because the old exporter omitted that unused curve. Preserve Stepped. Validate the
legacy duration, all key bounds/order, curves and the entire strict schema before
normalizing anything. Preserve names and animation/track order. Do not implement
migration through the Spine adapter, which has separate target restrictions.
Chain v1 → v2 → v3 explicitly. Loading migrates in memory; only explicit Save writes
the new version, after retaining a verified backup of the old file.

## Acceptance

Compare old v2 exports with migrated v3 exports, including reserved-duration tracks,
all-empty clips, empty root tracks, Stepped and unused Bezier metadata. Reject
malformed legacy durations, out-of-range keys and unknown fields. Cover empty,
frame-zero-only and tiny positive import/export round trips. A migrated 4-second
hold extended to 6 then shortened must return to 4 after save/reopen. Ordinary v3
last-key moves/deletes must shrink the clip; Undo/cancelled previews restore it.
Verify beyond-end keying, zero-duration chart/transport safety and external runtime
geometry using original fixtures. Keep production artist inputs untouched.

Suggested lanes: document/engine API; I/O migration and Spine adapter; desktop chart,
authoring cursor and transport. Coordinate fixtures and version changes before
editing; root owns integrated checks and native/runtime acceptance.
