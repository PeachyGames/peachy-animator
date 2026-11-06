# Bounded normal IK provenance

The implementation uses elementary one-/two-link geometry, public documentation
and original public-API observations. No external implementation, function source
or runtime constant was inspected, translated or copied. The separately licensed
oracle is not a dependency or redistributed component.

- [IK guide](https://esotericsoftware.com/spine-ik-constraints): one-/two-bone
  constraints, direct child and target restrictions, mix and bend direction.
- [4.3 API](https://esotericsoftware.com/spine-api-reference.md): authored versus
  applied poses and public skeleton/animation queries.
- [Constraints guide](https://esotericsoftware.com/spine-constraints): ordered
  effects and affected descendants. Dependent lists remain outside this slice.
- The angular basis coefficient is the existing independently observed unit-bone
  calibration documented in `SHEAR_PROVENANCE.md`, reused without another constant.

Original inputs and retained failures are local evidence under
`.local-backups/ik-discovery-20261106T083108Z-084144/`,
`.local-backups/ik-design-20261106T085459Z-d4be93/` and
`.local-backups/ik-core-20261106T092420Z-82a9b1/`. They contain no artist assets.
Oracle package metadata is4.3.13; installed editor data checks are4.3.26. Patch
numbers do not establish shared fix chronology. Editor JSON canonicalization is
separate from interactive editor/tool acceptance.

149 original unit winding/tie inputs establish a single signed turn correction:
desired-minus-authored deltas at or below-180 add360 once; deltas above180
subtract360 once. Multiple-turn modulo normalization is falsified by retained
observations. Raw authored rotations and keys are preserved. Unequal-link inputs
also establish child-origin offset as the first link, independently of the first
bone's display length, and noncoincident inner/outer reach behavior.

The engine supports a deliberately bounded active domain: rooted normal unit axes,
zero ancestor/constrained shear, a constant positive child X/zero Y with a positive
second length, noncoincident targets, and independent affected subtrees. Unsupported
queries return errors, preserving raw native data and pending/history. Mix zero is
exact FK. This is not full Spine IK, transformed-chain or constraint-timeline parity.

Precision checks cover both relevant incoming frames/target origins and solved
f32 frames/tips at an absolute0.001 bound. A retained before-fix original case at
rootX1e8, target(10,10) and zero bone length incorrectly accepted51.34019 degrees;
the public oracle reports44.999999353464446. The corrected query rejects the lossy
target explicitly. Relevant positions are reconstructed in f64 from captured
native-f32 locals, so a tiny lost direction can still be solved correctly rather
than hidden behind an endpoint-only bound. Exactly recoverable large poses and
unrelated large/zero-scale branches remain accepted. No global FK math change,
angle clamp, data rewrite, or blanket coordinate cutoff is used.
