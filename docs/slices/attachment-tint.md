# Attachment tint and Spine curve edge cases — November 10, 2026

Four game skeletons tint an attachment (`icing_02`, `3835f5ff`). Native v17
stores `color` on regions and meshes: exact RGBA bytes, omitted when white.
Draws carry slot color × attachment color, as Spine renderers tint, so every
view shows it.

Observed with installed Spine 4.3.26 and the external 4.3.13 runtime (original
probes): attachment colors follow the setup-slot color contract. Six digits are
opaque, an omitted color is white, and the editor writes eight lowercase
digits.

The idle-female rig, the first to open after this, exposed two curve cases the
native model refused:

- **Overshoot between equal values.** A Bezier between two keys with the same
  value can still bulge (`[0.6, 0, 0.733, 1.9, …]` between zero translations).
  Normalized value handles cannot express it, so native v17 adds
  `Curve::BezierOffset`: normalized time handles, with value handles stored as
  offsets from the start key's value. It is used only where normalizing is
  impossible (bone, physics and color keys) and evaluated in the value domain
  with the same ten-segment polyline.
- **Time handles outside the key interval.** Game exports place handles up to
  9% outside it. The runtime evaluates any finite handles through the same
  polyline as ours (an original fixture with handles from −0.9 to 1.9 matches),
  so Bezier handles now only need to be finite.

The curve panel no longer rewrites keys it only displays. It used to swap
crossed time handles and let egui clamp out-of-range values, both of which
dispatched an edit; its sliders now use `SliderClamping::Edits`.

Not yet: editing attachment color in the editor.
