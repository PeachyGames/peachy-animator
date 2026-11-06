# Exact timeline retiming and clipboard core

The public [Dopesheet guide](https://esotericsoftware.com/spine-dopesheet) describes
moving selected keys and modifier-based duplication. The [Keys guide](https://esotericsoftware.com/spine-keys)
describes copy, cut and paste, including pasting supported bone keys to another bone.
The coordinator supplied observations from the official developer discussion on
[dropping a key onto another key](https://esotericsoftware.com/forum/d/27677-connecting-two-curves-without-breaking-them).
These are public behavior references; no proprietary implementation was inspected.

`TimelineKey` identifies an exact source time, bone and scalar property. `MoveKeys`
captures every source before changing the candidate, so overlapping selected ranges
cannot erase a source that has yet to move. It moves or copies simultaneously and
replaces exact destination keys. Keys near a destination remain separate. Values and
normalized curves travel with the source key. Native duration grows to include the
destination and preserves any previously reserved duration; it never shrinks.

`copy_keys` returns an owned, sorted `KeyClipboard`, with the earliest key at relative
time zero. The current clipboard supports one source bone and preserves the property
for each entry. `PasteKeys` targets an explicit bone/animation and offsets relative
times at the supplied playhead time. Values retain the document's setup-relative
meaning; no absolute-pose conversion or rotation-to-translation remapping occurs.
Hand-built clipboard payloads must have a zero anchor, finite nonnegative times,
finite values, valid normalized curves and unique property/time identities. Keys in each
destination track are sorted during insertion, including unordered valid payloads.

Missing sources, duplicate identities, negative/nonfinite destinations, invalid
payloads and destinations that collapse distinct times due to floating-point
precision are rejected atomically. No epsilon merging is used in MoveKeys/PasteKeys.
SetKey, SetPose curve lookup and DeleteKey share exact-first resolution; tolerance
is available only for one unambiguous nearby key. This keeps subsequent ordinary
edits from changing the wrong close neighbor. SetPose's existing FPS snapping policy
is unchanged; general fractional-frame pose authoring remains pending.

`Session::preview_transaction` applies a command to the fixed transaction baseline,
including its transient pose. Success replaces only the preview; failure leaves the
last valid preview unchanged. Repeated previews restore overwritten destinations
from the baseline when a key moves away. Commit creates one history entry; cancel
restores the baseline and its existing redo branch. There is no second history stack
or OS/GUI dependency in these operations. Native schema and dependencies are unchanged.

The bounded curve model preserves source payloads. Spine's advanced curve-boundary
and handle-reshaping rules while moving/pasting are not established by these tests
and remain a parity gap. The coordinator owns GUI gestures, clipboard state, native
interaction checks, exported data checks and the external runtime oracle.


Local lane verification passes **70 Rust tests** (44 engine, 10 desktop, 4 CLI,
4 I/O, 8 adapter), formatting, warnings-denied Clippy, workspace build and all six
architecture boundaries. The twelve new tests cover exact/close identities, grouped
moves across properties/bones, overlap, exact replacement, owned relative copying,
cross-bone/clip paste, invalid payloads, floating-point collapse/overflow, atomic
failure and redo, baseline previews, cancellation, one-step history and later
SetKey/SetPose neighbor safety. All builds used this worktree's isolated target.
The initial Clippy run requested collapsing a nested DeleteKey conditional; the
final run includes that style correction. Source core functions contain no file,
process, clock, GUI or runtime-library effects.
