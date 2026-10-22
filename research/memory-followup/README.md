# Prospective memory follow-up

Implemented 2026-10-22. Full-goal progress remains **35%**. This completes a
shared agent workflow for selecting memory proposals and checking their original
expectations against a fresh recording. It does not identify gameplay meaning or
establish a causal control.

## Workflow

After automatic memory discovery, `prepare_memory_followup` pins the exact source
report checksum, capture count and one to eight selected groups. Every retained
typed member of a chosen group stays in the plan, including rival interpretations
and earlier contradictions. Selection uses only the saved proposals and original
read plan. There are no fresh byte reads, input events or model calls during
selection. Original group selection may itself have been screen-guided, and an
agent can choose which groups to follow; this is not a blind generalization test.

The plan is saved before changing the batch and before any fresh capture. The old
batch remains archived. At most 96 small windows, each up to 64 bytes and inside
the original read chunk, replace the broad memory plan. Mapping metadata is kept
for later read checks; it does not prove allocation lifetime or object identity.

The default continuation is twelve Visual captures in an ordinary cancellable
observation job. It performs no OCR, local model review, managed-runtime discovery,
input or writes. The last capture automatically exports and assesses already
collected bytes. `auto_observe=false` allows explicit collection; at least four
fresh memory samples are required. Play captures do not count toward that minimum.
This creates no report UI or unidentified cards.

Two-level hypotheses retain the source's two allowed values. Increasing/decreasing
hypotheses check adjacent fresh values only: the source/fresh boundary, a failed
read and an invalid numeric representation each break comparison continuity.
Varying hypotheses make no prediction. A violation remains a contradiction; too
few checks remain insufficient. A constant fresh sequence can satisfy a constraint,
so changed adjacent pairs are recorded separately and are not called activity or
control evidence. Current scope, snapshot storage and ordinary investigation read
budgets remain enforced.

## Persistence and bounds

Plans/results live under `captures/memory-followups/<investigation>/batch-NNN`.
The plan and result each have a 256 KiB cap, with a 64-byte result commit checksum;
the investigation has at most 24 batches. Files are create-only. An exact completed
write can be reused; a partial or different file is not replaced. A failed batch
transition can leave a plan reservation, which an identical retry can reuse.
The broad raw evidence export retains its separate 512 MiB investigation budget.

The live state and final job receipt retain the plan checksum. Historical reads
survive detach/restart: `read_memory_followup` first returns a result checksum,
then reads up to eight candidates/outcomes per page with that checksum pinned.
It verifies stored plan/result files, not raw bytes again. The offline
`memory-followup-replay` core example recomputes the result from the pinned plan
and raw archive without a target, image interpreter or production application.

Cancellation is checked during export, verification, assessment and publication.
There is no automatic replay of collection after restart. A completed evidence
file may remain after late cancellation. Optional review errors preserve captured
observations. Later collection cannot replace an already committed follow-up.

## Evidence

Generated target fixtures exercise the actual shared preparation/final-capture
handlers; the job controller uses a mock transport with its real executor/journal.
The representative fixture collects twelve source and twelve fresh captures with
exactly 24 target reads. Preparation adds no reads or images. Three typed members
share one 64-byte follow-up window; the float and its integer interpretation both
survive. Replay and historical reads run with every target method prohibited.

Tests also inject a third state into a two-level sequence, decreasing values across
a missing-read gap, invalid floating-point bytes, unchanged values and changed
mapping metadata. They verify contradictions, no comparison across gaps or the old
interval, insufficient evidence, stale scope rejection, cancellation, checksum
damage, immutable plan retry, page bounds and persistence after engine reload.
Automatic follow-up uses twelve Visual steps despite an available runtime/model;
its final plan/result checksums fit the existing 4 KiB journal budget.

See `results.json` for measured checks and frozen evidence hashes. No game,
production app, native input worker or live model ran. The executable/review packet
stay local; Windows protection and quarantine were not changed. Semantic discovery,
causal validation and independent-game evaluation remain incomplete.
