# Slot attachment and color keys — November 9, 2026

Eleven game skeletons swap slot images and animate slot color. Native v14 adds
`Animation.attachment_tracks` (stepped; a key without a name hides the slot)
and `Animation.color_tracks` (exact RGBA bytes with one curve per channel).

Observed with installed Spine 4.3.26 and the external 4.3.13 runtime:

- Before an animation's first key a slot shows its Setup attachment and color.
- `rgba` curves hold four Bezier groups (R, G, B, A) of absolute time and
  channel value; they are normalized natively like scalar curves (a flat channel
  becomes linear). Stepped keys are stepped on every channel.
- A color key needs `color` (both the runtime and the editor fail without one).

Keyed attachments and colors travel in `AuthoredPose`, so every scene shows the
swaps and tint; draws carry the color as bytes. The runtime check now compares
each slot's visibility and color (within half a byte) at every sample.

Not yet: authoring slot keys in the editor, Dopesheet rows for them, and the
`rgb`, `alpha`, `rgba2`/`rgb2` (dark tint) and `sequence` slot timelines.
