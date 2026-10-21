# Input context — 2026-10-21

Automatic observation now records a bounded input context alongside captured
frames. Desktop and MCP share the same data and lifecycle. This supplies another
measurement for future action/effect investigations; it does not learn controls,
attribute consequences, generate input or authorize an edit.

## What is measured

A dedicated Rust worker polls Windows virtual-key states with a nominal 16ms
sleep between polls. It holds a query-only handle to the original process,
verifies its creation identity, and checks that the selected window still
belongs to that process. It calls `GetAsyncKeyState` only when that exact window
is foreground. A second window/PID/focus check discards the entire key set if
focus changes during the scan. This is best-effort scoping, not an atomic focus
and input snapshot; a switch away and back between checks can be missed.

The implementation uses only the current-state high bit. Microsoft's
[GetAsyncKeyState documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getasynckeystate)
explains why the shared low bit is unsuitable for reconstructing presses, why
zero can also indicate an unavailable state, and why mouse buttons describe
physical buttons. Reversebolt does not translate virtual-key codes into text.
It records left/right modifiers separately and does not record other windows'
titles, identities or key sets in these polls.

These measurements do not establish that the game received or acted on a key.
Short taps and transitions between polls can be missed. No cursor motion,
mouse-wheel, controller or audio measurement is added in this version. The
existing visual model still receives images; it does not consume this context
or infer gameplay meanings from it yet.

## Ownership, timing and limits

The first capture of a job starts its sampler. Its cancellation flag is shared
with the job's existing operation owner, so pause, stop, failure, takeover and
completion revoke collection. A fresh resumed job starts a new generation.
The target's drop joins the worker; process exit or lost window identity ends
the loop. Restart reconstructs no sampler or active target. Manual captures do
not start one. Optional sampler startup failure is recorded in shared history
without suppressing the ordinary image/memory capture.

Poll begin/end bounds use the same investigation-local monotonic clock as image
and memory operations. A frame's optional `input_context` selects complete polls
between the previous saved image's end and its own image-capture end. The first
frame starts at its capture begin. A window records the requested boundaries,
monitor start, generation, retained-history boundary, omitted poll count and
largest unsampled interval including the edges. Empty intervals stay unknown.

The worker retains at most 4,096 polls; each frame retains at most the latest 256
polls in its requested interval. Older ring data are represented by
`retained_from_us`; per-window omissions are counted separately. Long capture
delays therefore cannot silently claim continuous input coverage. Reads have no
side effects, and complete poll validation precedes buffer mutation.

Input context is part of `FrameRef`, so existing capture/sequence MCP tools carry
the same object the desktop sees. Historical image evidence files retain it;
active-target APIs are still invalid after restart. The Inspect view shows a
short, explicitly historical capture summary. The main view remains named
control cards, with a small sampling notice while a job runs.

## Verification

Three core tests reject out-of-scope key sets and malformed chronology,
preserve focus gaps without inventing releases/idle, and expose ring/window
truncation. A controller test verifies that completion revokes its passive
operation owner. Existing pause/cancellation and restart checks remain in the
workspace suite.

The native test uses two owned WinForms processes in Anode, with no game input
or screenshots. The fixture confirms actual delivered `KeyDown` events while
Reversebolt independently polls the OS. Selecting the source starts observation
automatically. D is observed; after pause/resume, E delivered to the other window
is excluded and polls mark it outside the selected window. After cancellation,
F is not collected. A fresh job observes G, then completion prevents H from
being collected. Captures preserve common clocks and bounds, MCP sequence reads
preserve the contexts, and saved evidence contains the same objects.

Early harness failures are retained: the first script failed Python syntax
checking before launch; the next could not obtain foreground focus; the third
hit an unsupported PowerShell type alias before input injection. The harness now
uses Anode's window focus operation and explicit UInt16 conversion. The first
complete native pass is `test-results/input-context-v1/native-03/report.json`.
These were test-harness failures, not successful observations or product claims.

The published-build pass at `test-results/input-context-v1/published-01/report.json`
also passed. UI Automation verified the Inspect expander, historical capture
label, zero-sample unavailable state after completion, and its limitation text,
all onscreen without screenshots. Restart retained the byte-identical archived
input evidence and reconstructed no active target. The final archive has 368
polls: 97 selected-foreground and 271 outside-selected polls. E, F and H are
absent from every retained key set. These counts describe this run, not a
guaranteed sampling rate.

All 114 workspace tests, strict Clippy, formatting and the published 29-tool
practice/MCP smoke pass. Preview `0.1.0-20261021-040557` is running on the user's
desktop with its eight history entries and previous local-model setting intact.
Owned test processes were closed and the Anode lease released. The
[compact evidence](../../research/input-context/evidence/native-v1.json) records
source/binary/raw-artifact hashes and the preserved earlier failures.

To repeat the native test, acquire an Anode lease and run
`scripts/input-context-smoke.py --out NEW_DIRECTORY --bin BUILD_DIRECTORY` with
`seat_exec` in that child session. Supply its actual session and parent-session
IDs as `REVERSEBOLT_TEST_SESSION` and `REVERSEBOLT_PARENT_SESSION`, plus the
calling agent's `ANODE_AGENT_ID` and `ANODE_LEASE_TOKEN` for the owned-window focus
operations. The test refuses the parent session. `--visible --keep-ui` leaves
the owned desktop at a checkpoint for accessibility inspection; creating
`close-ui` in that test directory continues the restart check and cleanup.
It never replaces an existing result directory.

Next: use sampled input context in controlled contrasts, including cases where
the same key has different effects or no effect. Temporal association alone must
not turn a card into a named gameplay control.
