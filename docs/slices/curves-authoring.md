# Curves and scalar interpolation authoring

November 14, 2026; source baseline `0326fc3`. A bounded Curves view and scalar
Graph tangent workflow now share engine actions with MCP. Native v22 remains;
MCP exposes 49 commands / 11 tools. Native interaction, exact saved-state recovery
and actual MCP authoring pass. This is not complete Curves/Graph parity.

Curves occupies its own vertical pane above the full-width animation dock. It
fits finite relative value handles, freezes that scale during a drag, emphasizes
the primary curve, and keeps flat intervals blank without rewriting raw data.
Scalar Graph shares exact Dopesheet selection and adds tangent/numeric controls.
All Graph channel surfaces and Curves use one preview/draft controller.

## Source boundary

The current [Graph guide](https://esotericsoftware.com/spine-graph),
[Keys guide](https://esotericsoftware.com/spine-keys) and
[editor changelog](https://esotericsoftware.com/spine-changelog) were fetched
through the agent-reach web/search route on November 14. A dedicated current
Curves guide was not retrieved; this is not proof that no newer page exists.

Curves-specific evidence also comes from these primary sources:

- [Spine 4.1 release post, July 4, 2022](https://esotericsoftware.com/blog/Spine-4-1-released#Curves-view):
  relative transition editing and reusable curve presets in user settings.
- [Nate, December 24, 2024](https://esotericsoftware.com/forum/d/27891-why-are-there-two-curves-in-the-curve-view):
  selected-curve emphasis, Match, Separate and grouped-property distinctions.
- [Misaki, April 13, 2025](https://en.esotericsoftware.com/forum/d/28441-how-to-use-curves-view-window):
  acknowledges incomplete Curves documentation and points to the release post
  and hover/F1 tooltips. The documented preset-menu placement is version-bound.

Only human staff statements from those discussions are used as behavioral
evidence. Automated forum replies do not establish flat-value behavior or
current controls. No proprietary editor/runtime implementation or vendor
example project was read. All upcoming native probes must use original data.

## Verified installed 4.3.26 observations

The coordinator used an original Peach Bot probe, with the installed editor's
actual import/export baseline retained as
`.local/training-spine-20261114/curves-authoring/import-baseline/reference.json`.
The following observations were checked against native captures and exported
animation data; they are not proposed Peachy behavior inferred from appearance.

- Curves appears in Views only in Animate. The observed vertical pane sits
  between the viewport and Animations/Tree, above the full-width Dopesheet.
  It is not a replacement bottom Graph tab. The existing local
  `spine-curves-scalar-key0.png` shows the normalized handle-only plot, a green
  primary curve and gray baseline. This establishes the measured layout; it
  does not establish every dock/resize arrangement.
- Re-clicking already-selected Bezier resets the first Rotate transition,
  whose endpoints are `(0 s, 0)` and `(0.5 s, 30)`. Absolute JSON controls change
  from `[0.1, 21, 0.4, 9]` to `[0.16666667, 0, 0.3333333, 30]`. All other
  intervals, tracks and clips remain identical. This is the verified
  `bezier-reset-verified/reference.json` result.
- Selecting the single final Rotate key at `1 s` shows its preceding interval.
  Bezier reset changes the key at `0.5 s`, not the unused outgoing curve of
  the key at `1 s`. The measured controls become
  `[0.5952381, 50, 0.8333333, 10]`; the prior interval remains unchanged.
  This is `single-last-reset/reference.json`.
- Selecting both final keys, Rotate `1 s` and Translate X `1 s`, leaves the
  Curves plot blank and curve-type buttons disabled. Clicking Bezier leaves
  the entire animation JSON unchanged. This is `multiple-last/reference.json`.
- The actual Separate tooltip says handles can move independently and that it
  preserves handles before preset application or key paste. Its local capture
  is `spine-curves-tooltip-separate.png`. This adds a concrete preservation
  requirement to the human staff explanation of tangent separation.

The first reset above has horizontal value handles at one-third/two-thirds
time. The single-final reset does not: its first control is coupled to the
incoming tangent. **Do not generalize every reset to flat one-third handles.**
Both reset handles and two single-key live coupling directions are now bounded
by the matrices below; broader Match/Separate behavior remains unverified.

An earlier `bezier-reset` attempt exported the unchanged baseline before the
verified click, after an input guard interrupted the action. It was excluded
and retired, so it is not evidence that re-clicking Bezier does nothing.

### Twelve-case Bezier reset matrix, Separate off

The original bones-only `preset-matrix.json` has twelve sibling Rotate tracks
with a common middle key at `0.5 s` (frame 30 under the native 60 FPS default).
The coordinator selected the twelve middle keys through the Dopesheet overview,
reset Bezier with Separate off, saved, and exported. The source manifest is
`preset-matrix-cases.json`; native before/after data is in
`preset-matrix-baseline/preset-matrix.json` and
`matrix-bezier-off/preset-matrix.json`, with scalar results in
`matrix-bezier-off-observations.json`.

Independent comparison of those actual exports confirms that **all twelve
outer-key sets remain unchanged**. Across the entire animation, only the
middle keys' curve fields change; values, times and other key fields remain
identical. This includes the prior key, right endpoint and optional fourth key.

For the first/outgoing handle, the measured ordinary finite cases support this
reset rule. Let the selected interval run from `(s, a)` to `(e, b)`, and set
`d = e - s`, `delta = b - a`:

1. Start at time `s + d/3`, with value `a`.
2. When the preceding interval is Bezier, use its incoming endpoint tangent to
   set the candidate value at that time. A preceding Linear or Stepped interval
   does not supply that tangent; the reset value stays flat at `a`.
3. For nonzero `delta`, limit the candidate value to
   `[a - abs(delta), a + abs(delta)]`. If clipping occurs, move the handle time
   to preserve the tangent. This bound is centered on the start value, not an
   expansion around both endpoint extrema.
4. When `delta` is zero, the observed case retains its tangent excursion rather
   than clamping both handles onto the equal endpoint value.

Representative absolute native controls, for target `(0.5, 30)` to `(1, 10)`:

| Prior second control | Reset first control | Observation |
| --- | --- | --- |
| `(0.4, 29)` | `(0.6666667, 31.666666)` | Moderate positive tangent. |
| `(0.4, 31)` | `(0.6666667, 28.333334)` | Moderate negative tangent. |
| `(0.4, 9)` | `(0.5952381, 50)` | Clips to start value plus `abs(delta)`. |
| `(0.4, 58)` | `(0.5714285, 10)` | Clips to start value minus `abs(delta)`. |
| Prior starts at `0.3`, second control `(0.46, 29)` | `(0.6666667, 34.16666)` | Unequal prior/target durations retain the measured endpoint tangent. |

Changing the target endpoint to `50` with prior control `(0.4, 9)` still yields
first control `(0.5952381, 50)`, distinguishing the start-centered bound from a
direction-dependent endpoint-range guess.

Following Linear/Stepped intervals leave the second/incoming reset handle flat
at the right endpoint. A moderate following Bezier changes that handle to
`(0.8333333, 8.333333)` while preserving the following key's own controls.

### Four-case incoming reset and single-key live coupling

The original bones-only `incoming-matrix.json` and its cases manifest isolate
the next interval's outgoing tangent. Native baseline and reset exports are
`incoming-baseline/incoming-matrix.json` and
`incoming-bezier-off/incoming-matrix.json`. Independent comparison confirms
that only the four middle keys' curve fields change. All outer keys, key values,
times and other document data remain identical, except Spine's derived
`skeleton.hash`.

For the second/incoming reset handle, the finite measured cases mirror the
outgoing construction around the **end** value: begin at `e - d/3`, use the
following Bezier's outgoing tangent, and for nonzero `delta` clip to
`[b - abs(delta), b + abs(delta)]`, shortening the tangent ray when necessary.
For the target `(0.5, 30)` to `(1, 10)`, the observed controls are:

| Following first control | Reset second control | Observation |
| --- | --- | --- |
| `(1.1, 11)` | `(0.8333333, 8.333333)` | Moderate tangent, no clipping. |
| `(1.1, 31)` | `(0.9047619, -10)` | Clips to end value minus `abs(delta)`. |
| `(1.1, -18)` | `(0.9285714, 30)` | Clips to end value plus `abs(delta)`. |
| Flat target `30` to `30`, following `(1.1, 29)` | `(0.8333333, 31.666666)` | Retains the excursion when `delta` is zero. |

Two subsequent actual single-key drags with Separate off establish live
adjacent coupling. Compare each export with the immediately preceding export,
rather than comparing both drags with the original unreset fixture:

- `incoming-bezier-off` to `incoming-live-single`: the selected first control
  becomes `(0.6666667, 28.409304)`; the prior second control changes from
  `(0.4, 29)` to `(0.3999985, 30.95443)`.
- `incoming-live-single` to `incoming-live-right`: the selected second control
  becomes `(0.8333333, 12.098766)`; the next first control changes from
  `(1.1, 11)` to `(1.099991, 8.740855)`.

The adjoining tangents have the same slope at essentially the neighboring
handle's existing horizontal extent, with the small native X drift recorded
above. In each incremental comparison, only those controls and the derived
hash change. Other tracks, endpoints and controls remain identical. Earlier
live probes whose exports showed no neighboring changes are narrower,
inconclusive cases; they cannot establish a general no-coupling policy.
Separate-on remains unmeasured. The following multi-selection and extreme
probes establish limits without completing those policies' calibration.

The additional `incoming-live-clip` export demonstrates that the reset clamp
must not be reused as a universal live-neighbor rule. A more extreme drag puts
the target first control at `(0.5183616, 10.004745)` and the preceding second
control at `(0.4540301, 80.05983)`, even though that preceding interval's values
are `0` to `30`. This exceeds an anchor-centered one-span bound. It establishes
that the simple live-neighbor clipping hypothesis is wrong; it does not
calibrate the replacement rule.

In `incoming-adjacent-match`, two adjacent keys on `c01` are selected with Match
on. The two outgoing handles share normalized Y `0.2136752`: the first becomes
`(0.1, 6.410256)` and the next becomes `(0.6, 25.726496)`. The shared boundary's
incoming handle also changes from `(0.4, 29)` to `(0.4002706, 34.26194)`.
Independent export comparison confirms the other three tracks stay unchanged.
This demonstrates adjacent-selection interaction, including greater native X
drift than the moderate single-key examples; it is not an exact arithmetic
contract for adjacent Match edits.

The current implementation lane therefore remains bounded to calibrated
presets and ordinary single-key Separate-off live edits. Adjacent Match and
extreme live-transfer cases retain explicit limits until independently
calibrated and accepted. Numerical comparisons must use an appropriate stated
tolerance; these exports do not support a claim of exact native handle bits.

### Flat display is separate from stored curve data

Case `c09_flat_target` has endpoint values `30` and `30`. Its reset controls
remain `[0.6666667, 31.666666, 0.8333333, 28.333334]` in native export, including
the nonzero excursions. The coordinator then selected that one key alone;
`matrix-flat-only.png` shows a completely blank Curves plot. The earlier
twelve-key selection included nonflat curves and cannot establish this display
case by itself.

Therefore hide an unrepresentable relative plot without erasing its raw curve.
A blank Curves display does not imply Linear interpolation, zero handles or
absence of stored Bezier data. This is now direct native evidence rather than
the automated forum claim excluded above.

## Curves differs from Graph

The 4.1 announcement describes relative change to the next key rather than
an absolute value/time canvas. Its one-curve description is older than Nate's
2024 explanation of multiple selected curves with one emphasized primary.
Use the later staff explanation and installed 4.3.26 observations to resolve
that difference; do not implement a single-curve-only view from the old post.

Nate's Curves-specific explanation establishes the following:

- Curves edits interpolation handles and leaves key values unchanged.
- The first selected key's curve is emphasized; additional curves are gray.
  Clicking or dragging another curve makes it the emphasized curve.
- Match applies handle edits to the selected keys represented in the view.
- Separate permits a cusp by breaking tangent continuity through a key.
  It is unrelated to separating properties such as X/Y into different timelines.
- A combined Translate key was described as showing one curve in that version.
  Current channel/group display and edits still need the 4.3 probe.

These facts justify a transition editor with multi-selection and meaningful
Match/Separate behavior. They do not specify exact axes, numerical handle
coordinates, drag transfer math, flat-value representation or today's layout.
The layout and initial reset cases above now have direct native evidence.
Other details must come from the installed reference rather than a renamed
copy of the existing Graph or a preset-library-only panel.

The current Graph guide separately describes value/time editing, grouped
properties with multiple curves, shared Dopesheet key selection, handles and
navigation. Its Automatic, Separate, Flat, Bounce and ease controls are a
reference for Graph; the installed probe must establish which controls appear
in Curves and how their behavior is presented there.

## Confirmed 4.3 changes and unresolved exact behavior

The changelog provides version-specific requirements:

| Version/date | Documented change | Current probe status |
| --- | --- | --- |
| 4.3.65-beta, April 11, 2026 | Last-key curve buttons and Curves display use the preceding transition. | Verified for a single final Rotate key; effective edit is at `0.5 s`. |
| 4.3.24, September 6, 2026 | Preceding-curve treatment is limited to a single selected key. | Verified two-final-row selection disables controls and leaves animation data unchanged. Other mixed/grouped selections remain. |
| 4.3.73-beta, April 27, 2026 | Adds the Last chosen default-curve preference and moves Curves presets out of the view menu. | Inspect current controls and preference scope; the 2025 menu instructions are not the current placement contract. |
| 4.3.79-beta, April 30, 2026 | Curves sidebar animation is noted. | Normal vertical placement is captured; narrow/resized arrangements remain. |

The current Graph guide says a repeated Bezier-button click resets default
handles. The first-interval and coupled single-final controls above are now
measured. Probe a mixed Linear/custom-Bezier selection; do not assume resetting a
mixed selection has the same policy as resetting an all-Bezier selection.
Record source and target curves rather than judging the button highlight alone.

The 4.3 release post describes Last chosen for new-key defaults. The Graph
guide also says interior insertion inherits/splits surrounding curves. The
preference must be tested on a fresh key outside an existing interval as well
as an interior key; it must not be assumed to override curve-preserving
insertion. Preference persistence across views, projects and restart remains
to be established by the native probe.

## Grouped keys and independent channels

The current Keys guide documents grouped X/Y for translate, scale and shear,
and grouped RGB/alpha for slot color, with separate-timeline options. The
Graph guide describes independent Bezier handles for channels in one property.
These are different levels of grouping and must not be conflated with the
Curves Separate tangent control.

Peachy currently stores bone scalar tracks separately; slot RGBA keys have four
curves and enforce grouped Stepped. Its accepted UI has a selected slot channel,
while Deform has one whole-shape transition. Probe a combined Translate key,
independent scalar properties, RGBA with distinct channel shapes, and Deform.
Record which channels are displayed, which become primary, which Match edits,
and which unaffected channels retain their raw handles.

Equal endpoint values are now covered by the scalar matrix/sole-selection
probe above: native Curves leaves the relative plot blank but preserves the
stored excursion. Existing offset Beziers must remain available in raw data
and absolute value Graph editing. Grouped RGBA flat-channel combinations still
need their own scope check; no broad flat-value policy is inferred from the
automated forum answer.

## Implemented shared boundary

The engine owns transition resolution, curve conversion and calibrated actions:

1. `curve_targets` resolves editable transitions from the exact typed selection in a pure engine
   helper. Preserve the distinction between the selected final key and the
   preceding key whose curve is edited under the single-selection rule. Raw
   exact-key APIs should continue to address the key explicitly supplied.
2. `SetKeyCurve` and `SetIkCurve` preserve values at exact existing keys, including
   explicit unused terminal metadata. IK time controls retain f64 precision.
   Curve conversion, Match transfer and tangent-continuity arithmetic stay
   in the engine; do not introduce a second UI solver. Existing Deform and slot
   curve operations remain exact and preserve their offsets/bytes/times.
3. `ApplyCurvePreset` and `EditCurveHandle` expose the same strict operations
   through MCP. The latter takes actual seconds/value coordinates and a resolved
   primary within the complete selection. Match copies only the chosen handle's
   normalized position. Every multi-target candidate is atomic.
4. Scalar Graph/Dopesheet and Curves reconcile shared typed identities. View state
   remains separate from document and pose history. Raw scalar import preserves
   flat Bezier controls and tiny offsets instead of collapsing them to Linear.
5. Frozen-baseline Session previews preserve values, exact times,
   unrelated channels, pending poses and redo; one release/explicit edit makes
   one undo entry. Escape, application focus loss and modal/file/history/context
   changes cancel safely and prevent held-pointer re-entry. Numeric drafts own
   Escape and text undo so they cannot fall through to global deselection.

The measured Separate-off preset matrices preserve neighboring keys, while
the two live single-key drags change the adjacent tangent controls. Coupled
changes must be explicit engine candidates, atomically tested and included in
undo evidence; the reset and live operations have different measured scopes.

Reusable user presets are documented, but their current buttons, naming,
save/delete/overwrite behavior and storage format have not been observed here.
The column's management controls remain disabled. Independent Custom preset
application is available through the engine/MCP; this is not a stored library.

Separate-off adjacent selected Bezier intervals, Custom presets with Bezier
neighbors, and live vertical/outside-time, flat-linked or extreme coupling fail
explicitly. These are calibration limits, not claims that Spine refuses them.
Separate-on permits independent finite edits. Match onto flat destinations,
combined X/Y property groups, complete RGBA group behavior, Last chosen and
persistent user presets remain open. Ordinary live comparisons allow 0.0002
absolute units of observed native angle-conversion drift; Peachy retains exact
stored neighbor horizontal coordinates. Reset goldens allow 0.00002 units.

## Integrated acceptance

Focused checks use two Cargo workers and two test threads: 33 engine curve
checks (including eight shared-action checks), 47 desktop curve checks, three
adapter cases, eight MCP schema checks and six actual stdio curve workflows.
Filters overlap and do not establish a full-suite or hosted CI pass. Formatting,
all-target workspace Clippy, build and seven-crate architecture checks pass.

Actual native input verifies Views placement, shared exact selection, tangent
drag/one-step Undo/Redo, an observed held preview canceled by Escape, repeated
Bezier reset, single-final redirection and disabled multiple-final controls that
preserve redo. Actual typing produces pending rotation 13.5 and Graph handle
value 6.5. Numeric Escape preserves document, pose and selection. Normal live
Separate-off coupling changes only the two intended curves and preserves the
pending pose at a fixed time. Save excludes that pose; a fresh native process
loads the exact saved document with ten images and advances playback without
evaluation errors. Timeline movement intentionally clears pending poses.

Actual MCP authoring starts from `create_project` and one raw original cutout,
not a finished rig: 168 requests, 23 accepted commands and 121 retained pose
samples cover Match, terminal handling, raw setters, channel preservation,
atomic failure, history and Save/reopen/export. Its external runtime comparison
passes 312 world, 320 region and 160 color values at the existing 0.001 geometry
tolerance. Reference exports/captures and final authored projects remain private;
the [review gallery](../evidence/curves-authoring.html) uses original Peachy art.

The canonical [Curves Lab](../../examples/peach-bot/curves-bot.peachy) is byte-identical
to the final native-authored saved project, with all ten referenced images
matching the existing original assets. Its three clips across three skins pass
468 external runtime poses: 42,120 world, 43,680 region, 7,800 mesh, 15,600 UV
and 24,960 color values. Maximum mesh error is 0.00003052 at the existing 0.001
tolerance; UV error is zero. Both native and MCP sources stay unchanged.

## Original native probe matrix

The coordinator owns the installed GUI and exact acceptance. Suggested cases:

| Case | Verified or remaining comparison |
| --- | --- |
| Re-click custom Bezier; mixed curve-type selection | First Rotate reset is verified with other animation data unchanged. Mixed-selection policy and complete history acceptance remain. |
| One final key; penultimate+final; two final keys on different rows | Single-final redirection and disabled two-final-row selection are verified. Penultimate+final/grouped cases remain. |
| Multiple unequal slopes and interval lengths, Match off/on | Primary/gray curves, click-to-primary, exact transfer across selected keys and other channels. |
| Separate off/on, move either tangent | Both reset clipping directions, preserved outer keys and two single-key Separate-off live coupling directions are verified. Separate-on and multi-selection transfer remain. |
| Combined Translate, distinct RGBA channels, Deform | Channel display and scope; property separation remains distinct from tangent separation. |
| Equal endpoints with nonzero offset handles | Scalar sole-selection plot is verified blank; raw excursions survive. Grouped flat channels remain. |
| Fresh new key versus interior insertion under each default setting | Default type, inherited/split curves, Last chosen and preference persistence. |
| Presets collapsed/expanded and narrow view | Current 4.3 controls, tooltips, placement, capture of any preset authoring/storage behavior actually exercised. |

Native v22, current export-target rules, private-art isolation and the hosted CI
gate remain unchanged. The remaining cases above are a continuation backlog;
the bounded acceptance does not establish full Curves/Graph parity.
