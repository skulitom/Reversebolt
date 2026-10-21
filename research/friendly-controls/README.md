# Named controls, numeric steppers and Freeze

Unidentified OCR placeholders and unusable names are omitted from cards. Runtime
field names are made readable, with short provisional descriptions; technical
namespace/generic signatures stay out of the card. These descriptions are not
verified gameplay explanations. Numbers use direct exact entry and +/- controls.
A slider requires known small domain bounds; current raw runtime readings have no
such bounds. Booleans retain their On/Off control. Every card shows Freeze;
supported runtime numbers and booleans can hold, while other observations remain
disabled. Undo remains available for direct edits.

Freeze uses the shared engine and first-class MCP action. It periodically reads
the path and only writes on drift, retaining the existing typed bounds and write
guards. It pins the first fresh field/reference chain, rejects overlapping holds,
stops on reference changes/refusal/uncertainty, and never resumes after restart.
Every write intent and outcome is saved. Unknown outcomes fence further writes.
Stopping waits for current work and leaves the value in place. Checks yield to
queued commands and investigation jobs; agent holds pause during human control.
Eight simultaneous holds and 128 activation records are supported. This is not an
instantaneous/per-frame lock, and address reuse/downstream effects remain limits.

Validation:

- Workspace tests passed, followed by the four final freeze core checks, strict
  Clippy, formatting and release builds. Core/worker checks cover no-op reads,
  fresh drift values, moved objects, overlap, restart/unknown outcomes, stop during
  a check, and rejection of public calls to the private scheduler action.
- Native v1 failed an evaluator assumption: replacing a root changed the chain
  but retained the same boxed scalar. The test now reads the current value instead
  of assuming 3.5. The failed evidence and cleanup are retained.
- Native v2 (product `64ab681`) passed numeric/boolean re-holding, stopping,
  replay/conflict, replacement and restart, plus eight native UI actions: Freeze
  on, increment while frozen, Freeze off, decrement, exact entry, Undo, boolean
  toggle and Undo. Independent host byte reads confirmed entry/restoration.
  Its 60 helper operations had 126 balanced pauses and all helpers unloaded.
  One layout screenshot exposed technical type text; the final presentation fix
  removes those signatures.
- Final native v3 (`d4e2b0d`) additionally rejected competing aliases before any
  write. Five hold records, 17 helper operations and 40 balanced pauses passed;
  maximum pause was 83 microseconds, with zero helper heap operations while
  paused. This run used no UI actions; v2 supplies the native interaction evidence.
  All owned test processes exited. The final frozen 45-tool smoke passed.
- Preview `0.1.0-20261021-191551` matches the four final frozen product binaries,
  is running on the desktop, and preserves all 44 history entries and the selected
  local model. Anode was released.

Local evidence: `test-results/friendly-controls-v1`, `friendly-controls-v2`,
`friendly-controls-v3`, and `smoke-746e292f`. See [protocol](PROTOCOL.md) and
[results](results.json). No game screenshots or gameplay input were needed for
these control tests. The separate Manifold Garden experiment supports one narrow
real-game effect; it does not validate automatic naming or game generalization.
