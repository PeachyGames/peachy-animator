# Path constraints — November 10, 2026

Five game skeletons move bones along paths. Native v20 adds `Attachment::Path`
(composite Bezier: in handle, point and out handle per knot; `closed`,
`constant_speed`, setup `lengths`, editor color), `Constraint::Path` (bones,
slot, position/spacing/rotate modes, offset rotation, position, spacing and
mixes) and `Animation.path_tracks` (keyed position and spacing).

Defaults observed through the external 4.3.13 runtime's public data API:
open, constant-speed paths; percent position, length spacing, tangent rotation,
mixes of 1; an omitted spacing key value is 0.

Evaluation was fitted black-box against the runtime with original random paths
(no implementation source read):

- **Two length measures.** Distances along the path (the total for percent
  values, which curve a distance falls in) use a **4-segment** polyline length
  per curve. Inside a curve, the fraction travelled maps to the curve parameter
  through a **10-segment** arc-length table, linear in the parameter within a
  segment. A single curve in percent mode fits the 10-segment table alone,
  which hid the coarse measure until fixed positions and several curves were
  tested; the combined rule matches to 1e-5.
- With constant speed the setup `lengths` are not used.
- Position: percent of the total, or fixed units. Spacing for each later bone:
  the previous bone's length plus spacing (length), spacing (fixed), or spacing
  times the total (percent).
- Beyond either end, bones continue in a straight line along the end handle
  (`out₀ − p₀` before the start, `p_last − in_last` after the end), at that
  handle's angle.
- Tangent rotation turns the bone's world frame toward the curve direction plus
  the offset rotation, by mixRotate; X/Y move toward the point by mixX/mixY.
  Bones apply in list order, rebuilding world frames after each, so chains see
  their constrained parents.

A path constraint whose slot is not showing a path attachment leaves its bones
alone. Path attachments draw nothing.

Refused: closed paths, paths without constant speed, chain and chain-scale
rotation, proportional spacing, path mix timelines, and (until the follow-up)
deform keys on paths. Editing paths and drawing them in the viewport are not
done yet.
