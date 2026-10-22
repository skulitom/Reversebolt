# Shared memory proposal workflow

Implemented 2026-10-22. Full-goal progress remains **35%**. The existing memory-only
trajectory producer is now connected to observation jobs and the shared agent API.
It still does not identify gameplay meaning or verify controllable memory.

## Behavior

New Full/Visual observation jobs pin a memory-review flag on their final capture.
At least twelve memory samples are required. The review exports already-retained
bytes locally, analyzes the immutable archive and stores a compact summary in the
shared live state. It performs no additional target read, image capture, input,
model inference or memory write. Short jobs skip the optional pass. Play-only,
runtime-inspection and post-edit watch jobs do not request it.

`analyze_memory_evidence` exposes the same operation for an exact current scope.
`read_memory_proposals` retrieves a committed summary using investigation/batch,
then serves one to eight groups per page with a pinned report hash. Historical
reads survive detach/restart without a live process. Groups include numeric
interpretations, later assessments, missing data and omitted-candidate counts.
There are no new desktop reports or unidentified value cards.

One review is frozen per batch. Replaying the same explicit scope returns it;
later explicit counts cannot replace it. Subsequent automatic runs reuse its
historical capture count. A batch transition clears the live summary. This is
historical evidence, not a refreshed control or verified address binding.

The existing export budget remains 512 MiB per investigation. Reports add at most
4 MiB plus a 4 KiB summary per batch, at most 24 batches. Files are create-only;
identical interrupted publication may finish, but different or partial bytes are
never silently overwritten. The report binds its total capture count, source
manifest hash, selection/assessment split and policy. Summary fields are checked
against that report. Reading a report verifies its bytes, not the source payload
again. Raw payload verification remains available separately.

Owning-job cancellation is checked between bounded export/read/analysis work and
before publishing an automatic live summary. Explicit cached analysis uses the
normal serialized command lane. A partial export retains its reservation. A completed
file may remain after late cancellation, without a published live summary. Review
failures are recorded separately and preserve captured observations.

## Job-history fix

The shared journal permits 1 KiB per step and 4 KiB per job. Storing a complete
proposal summary in the final outcome would overflow it; repeated scope and large
optional-model results also threaten long observation jobs. Capture receipts now
keep compact frame/model/report references. The immutable job binding already
contains the scope, and full results remain available through their evidence APIs.
No journal limits were enlarged. The 31-capture generated job completes with
**2,729 result bytes**, retaining the final report hash.

## Verification

All **424 workspace tests**, strict workspace/all-target Clippy and formatting
pass. The initial compile, four targeted integration cases and the full-job
controller test also pass. Two core tests additionally cover bounded cancellation
and immutable proposal-store recovery/validation.

The integration uses the actual final `capture` service handler with generated
memory and file-binding image fixtures. Eleven prior captures plus the final one
produce a saved review with exactly twelve target reads and twelve images. A
sealed target rejects contact during explicit replay. The test reads paged results,
persists/reloads the engine (which drops its live target), looks up the summary
and reads the same page again. Wrong hashes, invalid paths, extra arguments and
oversized pages are rejected. Altered report bytes are detected.

Other cases cover short batches, cancellation before export, cancellation after
the raw storage reservation, failed report storage, later captures retaining the
old review, exact publication recovery, changed summary counts, oversized files
and cancellation between analysis/verification chunks. The job-controller test
checks Full/Visual/Play planning and executes all 31 steps through its real
executor and journal using a mock engine transport, preserving the final reference
within the existing limit. It does not launch the desktop or run a game.

[Results and frozen checks](results.json) record the final test counts, source
hashes and archived generated evidence. All work stays local. Existing independent
game recordings still lack broad bytes, and native validation remains unavailable
under the Windows security block. The desktop preview is not relaunched.

## Remaining work

Use these proposals to choose new observations and prospective interventions,
preserving rivals and omitted candidates. Associate independent visual/motion and
input evidence without treating coincident order patterns as semantics. Verify
controlling state before publishing named editable values. Recovery across scene
or process changes and independent-game evaluation remain unproven.
