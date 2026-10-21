# Explicit native trial edits

Date: 2026-10-21. This is an experimental editing path for observed screen
numbers. It does not identify gameplay authority, prove a slider range, or
establish independent-game compatibility.

The [compact evidence record](../../research/live-trial-edits/evidence/native-v1.json)
retains successful and failed run summaries, source/raw-evidence checksums and
the published binary hashes. Full local receipts and images are under
`test-results/trial-edit-v1/` and are excluded from Git.

## What is implemented

An OCR card with a current prospective prediction check can offer a nearby
requested value. The desktop supports dragging or typing that value, reviewing
the proposed edit, explicitly trying it, checking the visible result and
restoring the original bytes. Evidence remains behind Inspect. MCP uses the
same serialized engine and journal through `prepare_change`, `apply_change`,
`observe_change` and `restore_change`. Connection and automatic discovery remain
read-only; preparing an edit never authorizes or performs it.

Preparation retains every non-contradicted interpretation from the frozen check.
Every retained interpretation must have sufficient fresh evidence, agree with
the current bytes, and propose exactly the same normalized byte action for the
requested value. Missing or invalid rivals do not disappear from the vote.
Disagreement prevents the edit. No canonical type is inferred from agreement
on an action. The change must be greater than 1.1 and at most 20 from the latest
screen reading, and each model must encode it within the declared 0.55 tolerance.
The public operation accepts a track and value, never an arbitrary address.

Each plan binds the process creation identity, selected window, investigation,
memory batch, image epoch, prediction and observation. It expires after 60 seconds
or a new ordinary capture. Application requires unchanged complete model bytes,
not just the smaller changed patch. Native writes are restricted to 1–8 bytes in
the original private, writable, non-executable allocation. A temporary write
handle is opened only for the explicit operation. No memory protection changes,
injection, suspension, debugger or bypass are used.

The independent operation journal persists intent before entering the backend.
It records attempted status, transferred byte count, readback and errors.
Completed operation IDs do not write twice. Incomplete or uncertain writes hold
further edits to that process identity, including after reattachment. A restart
expires prepared plans, marks unfinished intents uncertain and never replays
them. A final journal failure preserves the in-memory outcome and holds writes.
There is no automatic reconciliation or force-restore override.

Restoration checks the original investigation and the complete interpretations
with the trial bytes overlaid. Conflicting current bytes are not overwritten.
Successful application/restoration records a separate screen-number check when
possible; additional checks are explicit. Each check has a unique image, hash,
expected value, actual reading and any error or recognizer warning. A failed
optional recognizer does not discard usable Windows OCR words. Trial writes
invalidate passive prediction evidence so self-induced changes cannot become
independent confirmation.

Limits: 128 retained operation records, eight image checks per operation,
10 MiB per image and 128 MiB trial evidence storage. Failed captures retain their
storage reservation because a native worker may finish later. Image paths are
never reused after a timeout or restart. History pruning is not implemented.

## Automated evidence

All 104 workspace tests pass, including nine operation-journal tests and one
live-session test. They cover per-value disagreement, missing rivals, full-byte
restoration guards, stale process/scope/allocation/frame/expiry, pre-write journal
failure, partial writes, uncertain outcomes, restart without replay, completed
operation replay, source-hashed observations and late capture reservations.
Strict workspace Clippy and formatting pass.

The native API smoke creates its own disposable child and uses a known arena.
It verifies write/readback, stale expected-byte refusal, restoration and mapping
refusal. It is deliberately not a discovery test. Evidence:
`test-results/trial-edit-v1/native-write-published.json`.

The separate WinForms discovery fixture allocates a float at a random offset in
a private page and paints its value as Quantity. The engine receives no field
address, page boundary or layout. The test driver controls only the fixture's
displayed values. Twelve changing screen/memory captures are followed by four
fresh hold/change observations and an explicit request from 50 to 53.

The third native run found one surviving float interpretation. MCP applied the
one-byte change, observed 53, repeated the completed operation without changing
its write receipt, restored the original byte and observed 50. These are actual
Windows process writes and target-window OCR, rather than a mocked backend.
Evidence: `test-results/trial-edit-v1/session-v3/trial-result.json`.

A fourth run prepared the same request through MCP. UI Automation invoked the
card's Try edit and Restore bytes buttons. The card showed 53 and Requested
number seen, then 50 and Original bytes restored. Evidence:
`test-results/trial-edit-v1/ui-v4.json` and `session-v4/changes/` below the same
test directory. This was a native control test without gameplay input or manual
screenshot review.

The final fifth run also changed the requested number through the accessible
numeric control. Its declared range was 30–70 around the observed 50. Setting
54 disabled the old 53 plan, Review edit prepared 54, and Try edit produced a
visible 54. Restore bytes returned the display to 50. The earlier spinner had
incorrectly exposed a 0–0 range; the final build supplies the actual bounded
range. Evidence: `test-results/trial-edit-v1/ui-v5.json` and `session-v5/changes/`.

Failures are retained: the first run successfully wrote but its screen check
discarded valid fallback OCR when optional neural model files were absent.
That error was corrected and the original bytes were restored, with receipts in
`session/recovery-after-ocr-warning.json`. The second run found no changing
number after a file-sharing error in the test fixture opened a dialog. The
fixture now retries control-file reads and the driver selects the exact intended
window. Its failed evidence remains in `session-v2/trial-result.json`.

The 28-tool release smoke also checks three randomized practice targets,
authenticated local access, stale restoration refusal and restart invalidation.
These practice checks remain distinct from the native trial fixture.
The published run is `test-results/smoke-1a8877df/report.json`. Preview
`0.1.0-20261021-030209` is running on the user's desktop with its prior eight
history entries and model selection retained. The delivery receipt is
`test-results/trial-edit-v1/desktop-delivery.json`. Owned test windows were closed.

## Limits and next steps

Read/check/write is not an atomic compare-and-swap with running target threads.
Windows documents the separate [read](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-readprocessmemory)
and [write](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-writeprocessmemory)
operations; the target can race these guards. Allocation metadata cannot prove
that an address was not freed and reused between checks. Capture and memory
reads are sequential. OCR agreement can be wrong and a matching displayed number
can still be a copy, a derived output or a misleading coincidence.

Restoring bytes does not undo elapsed time, consumed resources, saved state or
hidden downstream effects. Disconnect, restart, a new batch or changed geometry
can remove the scope needed for restoration. Uncertain operations need future
independent reconciliation; the current UI cannot safely retry them.

The fixture is owned and purpose-built. No Blue Prince or other real-game edit
was attempted, and no unseen-game acceptance credit is earned. The next work is
explicit effect scope and causal checks against independent outcomes, lifecycle
recovery, then unchanged evaluation on independent games. The goal remains active.
