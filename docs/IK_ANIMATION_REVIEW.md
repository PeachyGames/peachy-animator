# Animated IK review checkpoint

This is a review build of the native7 slice. It adds animated Mix/Positive,
grouped IK Dopesheet keys, typed retiming/clipboard actions and shared bone/IK
pending poses with atomic keying/history. The original Animation Lab has eleven
clips, including IK-only, linear, stepped, Bezier, signed-Mix and target motion.

```sh
cargo run -p peachy-animator -- examples/peach-bot/ik-animation-bot.peachy
```

Switch to Animate, select an IK constraint, and use Mix/Positive, Auto Key or
Key Edited. D plays/stops; the animation picker offers the different examples.
Project saves use `.peachy` native7. Older native6 builds cannot open these saves.

![Original Animation Lab in the native editor](ik-animation-review.png)

The exact implementation checkpoint `c82d160` passes 344 Rust tests, formatting,
warnings-denied Clippy and the workspace build. Architecture checks pass. Ten
original packed fixtures pass the separately licensed public runtime comparison
for every clip/skin: 479,232 bone values, 472,160 region coordinates, 75,400 mesh
coordinates and 4,992 IK pose groups. Inputs remain unchanged. Real OS input
verifies typed pending Mix, Undo/Redo and an unchanged document/file before an
intentional review handoff. The complete prepared native acceptance script has
not been run; this smoke check does not establish full interaction parity.

One material compatibility issue remains: importing noncanonical raw IK curve
numbers can lose precision in the current f32 key/control model. Original large
cancellation cases produce visible pose differences. The export-time rounding
bug is corrected and covered by eight regressions, but the broader raw import
correction is still planned in [the precision contract](slices/ik-raw-precision.md).
This checkpoint is therefore not accepted as completed animated-IK compatibility.
IK Graph/curve authoring, exact installed-editor row/shortcut comparison and
broader constraints remain separate gaps. Full production-rig support stays 0/11.

Evidence: `evidence/ik-animation-review-checks.json`,
`ik-animation-review-runtime.json`, `ik-animation-review-native.json` and
`ik-animation-raw-precision.json`. All committed captures/data use original art.
