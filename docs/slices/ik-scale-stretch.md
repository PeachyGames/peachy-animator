# IK with scaled, mirrored and stretched bones — November 10, 2026

Four game skeletons put IK on bones that are scaled (keyed uniform 0.978),
mirrored (thigh scale X −1) or stretched (`stretch: true`). Native v21 adds
`IkConstraint.stretch`; the solver now accepts constrained bones with
equal-magnitude X/Y scale and no shear (a mirrored first bone is allowed; a
child scales uniformly and positively; keyed X and Y scale must match).

Fitted black-box against the external 4.3.13 runtime with original random rigs
(no implementation source read):

- **Scaled chains.** The child origin is the child's local offset scaled by
  the first bone's scale; the child's reach is its length × the first bone's
  |X scale| × the child's scale. The child's local rotation undoes the first
  bone's (diagonal) scale. Uniform, mirrored and mirrored-with-scaled-child
  chains match to 7e-6. A mirrored chain's tip need not land on the target; the
  runtime does the same.
- **One bone.** The bone's world X axis points at the target, so a mirrored
  bone turns 180° further.
- **Stretch.** Out of reach, the first bone's local X scale is multiplied by
  distance / reach (two bones: a + b; one bone: length × |X scale|), blended by
  mix like the rotation (1 + (factor − 1) × mix).
- **Stretch flattens the child.** With stretch on, the runtime solves as if the
  child had no Y offset, and an active stretch IK sets the child's applied Y to
  0 (not blended). Off-axis children otherwise keep their offset.

`stretch` is written to JSON only when set, so existing exports are unchanged.
Not yet: `compress`, `uniform`, softness, keyed stretch, nonuniform or sheared
constrained bones, a mirrored child.
