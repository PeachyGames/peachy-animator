# Native6 before IK animation

These nine original native6 files were retained at commit
`13e5c33e4522c41322ac682c1ef20e4811534177` and verified byte-identical
against approved implementation base `2d396e8dcfd76a7b98d9ffbef2a3df992a4221fc`.
All nine have zero IK animation timelines; IK Lab already has two static setup
constraints, which must remain unchanged.

The JSON files are actual complete CLI exports, each reproduced twice with
identical bytes before native7. The independently built baseline CLI SHA-256 is
`ae378d95a10cd98de9e91d29fd6529287ba3de904b4506a3fbed2c8d3391a9a1`.
Native7 migration may add format metadata and empty per-animation ik_tracks only.
Full Spine export bytes must remain identical; empty animation .ik is omitted.

These data and artwork references are original Team Peachy fixtures, generated
independently. No private artist input or official implementation is included.
Captured November 6, 2026 UTC; local source manifests are retained under
`.local-backups/ik-animation-formats-20261106T102700Z-ca7a25/`.
