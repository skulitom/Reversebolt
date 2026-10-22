# Fast initial observation without losing memory discovery

The [browser replay](../browser-activity-replay/README.md) found that 54 of 60
saved intervals mixed controls and none supplied a quiet-bracketed action
episode. Source inspection then found that only supported runtimes received
an automatic image/input-only phase. Other targets ran full OCR/memory captures.

Initial automatic attachment for other targets now starts fifteen Play captures
with 100 ms delays, then retains all sixteen original full captures with 1,000 ms
delays. The first capture of each phase starts immediately. These are delays
after completion, not frame rates. The initial phase is not repeated for later
memory-search batches, change selection or prediction rechecks. Supported-runtime
discovery retains its existing fifteen-capture schedule. Agents can request the
mixed sequence through `start_observation_job` with `observe_play: true`.

This required more than changing the scheduler. The old complete-trajectory
rule required memory in every capture, so inserting Play captures would exclude
every memory chunk. The shared session now retains `memory_sample_numbers`:
the exact one-based capture numbers where memory reads were attempted. Numeric
fitting, its chronological fit/holdout split, complete-coverage accounting and
reference learning use that selection. Missing full/visual reads stay in the
selection and prevent completeness; only deliberate Play captures are excluded.
Image/input evidence and archived witnesses retain their original numbering and
gaps. Frozen prediction basis images cite the actual memory capture numbers.

The independent limits are 36 total captures, 20 memory-sampling attempts and
512 MiB retained memory. Existing investigation-wide image/read budgets remain.
The mixed default uses 31 capture slots and 16 memory samples. It can fit after
twelve memory samples (capture 27), then check four fresh samples (28–31).
No image-only observation supplies a memory reading or editable-control authority.
Legacy records without memory-sample provenance count all captures conservatively
for budgeting.

Both phases belong to one durable job, with normal pause, stop, scope and retry
rules. Queued jobs reserve both capture and memory capacity. Job capacity grows
from 16 to 32 steps. Opening older history permits this one-way capacity expansion
while preserving every old plan; pending work is still interrupted and its
connection revoked. Shrinking capacity or changing other limits is rejected.
With a configured local model, the early background pass uses play capture 2;
the second pass follows the final full capture. This retains two passes rather
than adding another pair for the extra phase.

The shared image-evidence reader, activity preparation, agent schemas and offline
replay entry points accept the extended image timeline. Region-candidate and
template-check budgets remain unchanged. No unidentified cards or new editing
permissions are introduced.

## Verification and limits

Controlled native-target tests exercise fifteen Play captures followed by twelve
full samples and four fresh checks. They verify zero target reads during Play,
original-frame provenance, learned references, both matching and contradicted
future observations, and separate image/memory ceilings. A missing memory read
remains part of the trajectory and prevents a fitted candidate. Scheduler tests
execute all 31 steps through the shared worker, verify both phases and budgets,
check retry identity, and preserve the existing search/recheck paths. The journal
test checks old-plan preservation, restart interruption and capacity compatibility.

An extended generated-image test reads all 36 retained images through shared
activity preparation, retains regional support under the existing 80-check
budget and rejects an oversized request. Its first run passed those functional
assertions but failed a cleanup-path assertion: the fixture directory was
canonicalized while the comparison path was not. The cleanup guard now compares
canonical paths and verifies the fixture directory prefix before deletion.
The original failed log remains in `test-results/mixed-observation-images.log`.

All **385 workspace test executions**, strict workspace/all-target Clippy and
format checks pass. Final logs are in `test-results/mixed-observation-v1`; the
[compact verification record](verification.json) includes their hashes. These are software integration tests,
not measured Windows capture throughput or independent gameplay validation.
No game was launched, no new gameplay input was sent, and no model was called
for this work. A 100 ms delay cannot guarantee useful input separation; capture
cost, action duration, focus and natural pauses still determine eligibility.

Overall progress remains **35%** until stronger end-to-end evidence exists. The
desktop check still reports the quarantined preview unavailable. No replacement
was published, and the executable/security-review packet remain local.
