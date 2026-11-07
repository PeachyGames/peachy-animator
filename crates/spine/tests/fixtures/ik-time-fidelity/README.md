# Original IK export-time regressions

These eight original bone-only native fixtures reproduce observable geometry
changes when f64 reconstructed absolute control times are rounded to f32 by the
JSON exporter. Four were produced by real MoveAnimationKeys operations on
source-derived curves. The independent public-API oracle accepted the native
queries; the old exported curves differed by up to1.977 world units.

They were copied byte-for-byte from retained original observations under
`.local-backups/ik-export-time-fidelity-20261106T115357Z-3d95ef/native-export-01/`.
No runtime/editor implementation or private artist data was inspected or copied.
The corresponding f64-X export-only public-API control matched all eight native
Mix values exactly and world matrices within4.88e-7 at the unchanged0.001 bound.

The adapter test checks complete f64 X reconstruction rather than silently
canonicalizing the native curve. Broader raw-source import precision remains a
separate model decision; these regressions do not claim that decision is closed.
