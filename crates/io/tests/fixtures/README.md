# Original duration migration fixture

`legacy-v2-duration.peachy` is an original, minimal native v2 fixture, authored for
this migration. It contains reserved duration, partial tracks, unused final
Bezier and Stepped curves, empty root and nonroot tracks, and a tiny clip.

`legacy-v2-export.json` was captured by calling this project's own
`peachy_spine::export_json` at commit `a2395e9` before changing the document schema
or adapter. Migration tests compare the complete JSON value against that golden.
No Spine implementation or artist data was used to create either fixture.
