# Bone inheritance modes — November 9, 2026

Nine of the game's skeletons use Spine's bone inheritance settings (Inherit
Rotation / Scale / Reflection), and two key changes to them in animations. This
slice evaluates all five modes exactly, stores them in native v10, edits the
Setup mode in Properties, and imports/exports both the bone field and the
stepped `inherit` timeline.

## Behavior (independently derived)

Formulas were derived only from black-box runtime observations of original
random rigs (grandparent → parent → child with random rotation, scale incl.
negative, and shear), never from runtime or editor source. `P` is the parent's
world linear part, `L` the child's local matrix, `L₀` the local matrix with
rotation zero.

| Mode | Child world linear part | Max observed error |
| --- | --- | --- |
| normal | `P·L` (unchanged code path) | 2e-7 |
| onlyTranslation | `L` | 1.3e-7 |
| noRotationOrReflection | `R(θ)·diag(|X|, |det P|/|X|)·R(−θ)·L`, θ and |X| from P's X axis | 3e-7 |
| noScaleOrReflection | `R(φ)·L₀`, φ = angle of `P·(cos r, sin r)` | 5e-7 |
| noScale | `R(φ)·diag(1, −1)·L₀` when `det P < 0`, else as above | 5e-7 |

Translation always passes through the full parent (exact in every mode).
Keyed modes are stepped; before an animation's first key the Setup mode applies.
Keys without `inherit` mean normal, as the installed editor and runtime agree.

Degenerate parents are refused with an explicit error rather than guessed: the
runtime switches to a different branch for a parent X axis shorter than about
0.00002, and returns NaN for noScale modes under a zero parent. We evaluate
noRotationOrReflection down to an X-axis length of 0.0001 (verified exact well
below that) and noScale modes for any nonzero mapped direction.

Spine forbids disabling inheritance on IK-constrained bones; such data, keyed or
Setup, is refused on import and fails evaluation. The legacy `transform` alias is
ignored by installed 4.3.26, so a non-normal alias is refused instead of guessed.

## Editor

Bone Properties show **Inherit: Rotation, Scale, Reflection**. Reflection is
enabled only with Rotation on and Scale off, the one combination where it
selects between modes. Each change is one `SetBoneInherit` undo step in Setup.
Animate shows the active mode read-only and notes when it is keyed.
Keyed modes reach every Animate evaluation path through `AuthoredPose`.

Not yet: keying inheritance in the editor, Dopesheet rows for inherit keys, and a
comparison of the checkbox layout with the installed editor's panel.
