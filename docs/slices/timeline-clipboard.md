# Dopesheet key clipboard

The public [Keys guide](https://esotericsoftware.com/spine-keys) specifies the
Copy, Cut, Delete and Paste controls, their Cmd/Ctrl shortcuts, and pasting bone
transform keys onto another selected bone. This slice covers the existing scalar
bone timelines only. The coordinator owns direct reference and native checks;
no proprietary Spine editor/runtime implementation was inspected.

The Dopesheet toolbar places independently drawn Copy, Cut, Delete and Paste icons
in that order. Buttons keep the Dopesheet canvas focused after successful actions.
Cmd/Ctrl+C/X/V operate only when the visible Animate-mode canvas has focus. Text
fields, naming dialogs, menus and other controls retain their input; unmodified V
continues to select Translate. Selected slots never become implicit bone targets.

Copy reads exact source keys through the engine and creates no history entry.
Cut captures the same payload, then dispatches one exact-key deletion batch. The
clipboard changes only after deletion succeeds. Paste dispatches the shared
PasteKeys command at the actual playhead time, selects the pasted identities and
keeps the active animation/bone unchanged. Copy-relative timing, original scalar
properties, raw setup-relative values and curves are retained. Destination key
replacement and duration extension belong to the shared engine.

Clipboard data is an owned editor-session buffer. It survives bone/animation
changes and document Undo/Redo, but opening a project clears it. Explicit Copy/Cut
also mirror a compact JSON envelope to the operating-system text clipboard, with
`peachyAnimatorClipboard: "keys"`, version 1 and the owned payload. The envelope has
no paths, source bone IDs or project IDs. This supports the native GUI integration,
which emits Copy/Cut/Paste events rather than ordinary clipboard-key presses.

Native Paste text must exactly match the current session's mirrored envelope.
External text, altered/version-mismatched envelopes and old payloads cannot become
animation keys. The toolbar Paste button uses the session buffer directly, even
when unrelated external text has replaced the OS clipboard. Reset, cancellation
and failed Cut never write the OS clipboard. Parsing another application's or a
previous project's payload is outside this slice.

Pending unkeyed poses are neither copied nor silently keyed/discarded. Cut/Paste
preserve those transient edits through the existing Session command path; the
status explicitly says the unkeyed pose was retained. The viewport may therefore
continue showing that pose over the edited keys until the artist keys or clears it.

F12 exposes `clipboardKeys`, and targets `timelineCopy`, `timelineCut`,
`timelineDelete`, and `timelinePaste`. Native acceptance should verify keyboard and
button paths, copy without dirty/history, cut/paste one-step Undo/Redo, cross-bone
and cross-animation paste, overwritten destination restoration, session reset,
pending-pose preservation, external clipboard rejection, and text-field priority.
Tests here cover the versioned envelope and data boundaries, rejection of stale or
external text, event ordering/focus priority, and preserving the plain V shortcut.

The native format and dependencies are unchanged. Multiple-bone clipboard mapping,
other timeline types, cross-project imports, external clipboard interoperability,
full curve-handle reshaping and complete Spine clipboard preferences remain pending.

Local isolated checks pass on the core implementation/test prerequisites: **75 Rust
tests** (desktop 15, CLI 4, engine 44, I/O 4, adapter 8), formatting,
warnings-denied Clippy, workspace build and the six-crate architecture guard.
Dedicated clipboard events take precedence over matching raw-key fallbacks in a
mixed host event stream, so unknown paste text cannot bypass envelope validation
and Copy/Cut are not duplicated. The coordinator supplies the shared icon drawings
and owns the final merged-source native/runtime verification gate.
