# Broader memory coverage — 2026-10-20

## Delivered behavior

Preview `0.1.0-20261020-212137` adds **Search more memory** and the shared MCP
`reversebolt_advance_memory` tool. Each batch selects a bounded part of the
ordinary eligible memory map. Advancing preserves the current observations and
hypotheses, retains sparse byte evidence, releases raw snapshots, and selects
previously unvisited addresses. It requires new observations for the new batch.
No game names, offsets, signatures or source-derived layouts guide selection.

This removes the old restriction of repeatedly capturing the same initial
window after reconnecting. It is an explicit batch workflow, not an automatic
whole-process sweep or a durable background investigation job.

## Research reviewed

The independent `experiments/memory-coverage` handoff reported improved recall
with bounded cohorts, alongside representation aliases, churn starvation and
same-address lifetime errors. Its 12 tests were rerun successfully and its
artifact checker was rerun against all 56 frozen results. The full benchmark
was not rerun in this integration turn.

The production implementation uses the existing shared engine and matcher,
with a new planner rather than importing the experiment's fixture or evaluator.
It preserves unvisited region suffixes and skips previously selected addresses
even when their observed metadata changes. This avoids that form of repeated
selection starvation, but also means changed allocations need a future revisit
policy. Prototype recall numbers are not claimed for the integrated application.

## Correctness and resource boundaries

Snapshots from different batches never form one trajectory. Each chunk is
checked before and after reading for allocation base, protection, type and
containing range. An observed incompatibility excludes it for the rest of the
batch, even if later metadata returns to its earlier value. Failed or partial
reads never become zero-filled valid samples.

Windows describes a range beginning at the queried page; an interior query can
return a shorter suffix. The comparison accepts a suffix that covers the whole
requested chunk with matching attributes. Neither this metadata nor equal
before/after observations proves an allocation was never released and reused.
See [VirtualQueryEx](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualqueryex)
and [MEMORY_BASIC_INFORMATION](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-memory_basic_information).

Coverage separates unique addresses selected and read, complete local memory
histories, bytes requested from the process, retained snapshots and bytes
actually compared. Twelve complete memory samples can exist without any usable
visual observation. Historical coverage may exceed current eligible memory
after relocation; it is not a percentage of known objects or a simultaneous
snapshot of the process.

Each batch retains at most 20 captures and 512 MiB of raw snapshots. A connection
has a 24-batch and 4 GiB requested-read ceiling. The image allowance is 512 MiB;
late or failed renderers reserve their full 10 MiB cap, and native PNG encoding
enforces that cap before writing. Batch archives have a separate 32 MiB budget.
Per-frame metadata and visual manifests are separate, bounded by capture/track
limits. These limits are not a whole-application RSS cap or global disk quota.

Advancing writes a synchronized archive before dropping snapshots. An archive
failure preserves the current batch. Archives include its observations, plan,
hypotheses and up to 256 candidate byte witnesses with read timing; excess
hypotheses remain listed without full witnesses. Raw noncandidate bytes are
discarded. A stale batch/investigation request is rejected, and old frame IDs
cannot be submitted into the new batch. Archives never grant current edit
authority and are not automatically resumed after restart.

## Verification

All **21 workspace tests**, strict Clippy and formatting checks pass. New
integration tests exercise a value outside the initial window, isolated batch
matching, retained byte witnesses, stale requests, archive failure, observed
mapping changes and read/storage limits. A deferred renderer test checks that
its reserved storage survives a timeout. A native PNG test checks overflow and
pixel round-trip. Fixture labels prove these mechanisms, not game understanding.

The final release passes the real-process practice/MCP suite with **17 tools**:
`test-results/smoke-6804a334/report.json`. Existing edit/restore, authentication,
shared-state and restart checks remain passing.

Blue Prince ran paused in Anode using the existing test profile. Two disjoint
16 MiB batches each completed 12 captures. Every planned read succeeded, with
no observed mapping changes. The second batch began with zero retained raw
bytes; its payload grew to 192 MiB, rather than accumulating the first batch.
A third batch's first capture brought unique read coverage to **48 MiB** while
complete memory histories remained **32 MiB**. Eligible memory was about 4.36 GB,
so this is still partial coverage. Both correlation attempts refused to run
because the static scene supplied no changing numeric track. No control was
discovered and no game memory was written.

The final 24-capture run took 340–499 ms per capture (mean 406.8 ms); mean memory
read time including mapping checks was 13.8 ms. This is one machine's static
scene timing, not a general performance bound. Both OCR sources were present,
WGC captured 1280×720 and no capture warnings occurred.

Anode UI checks on the preceding coverage build verified the visible batch and
coverage counts, button activation, cleared current observations after advancing,
and historical summaries. A real MCP client also advanced the same session and
read a third-batch image with a matching checksum. The final build repeated
native coverage and MCP advancement after the image-budget change. Restart
retained both archives byte-for-byte and all 31 activity events while invalidating
the live connection. Blue Prince remained paused; owned test engines were closed.

## Local evidence

- `test-results/memory-batches-v1`: initial native/UI run using Windows OCR only;
  missing optional model warning was recorded.
- `test-results/memory-batches-v2`: both models present, UI/MCP checks,
  `ui-final-state.json`, `archive-hashes.json`, `restart-report.json`, and the
  third-batch MCP image.
- `test-results/memory-batches-v3`: final release, `coverage-report.json`,
  individual capture states, `mcp-advance`, `final-state.json`,
  `metrics-summary.json`, `archive-hashes.json` and `restart-report.json`.
- `scripts/coverage-smoke.py <isolated-data-dir> <pid>` reproduces the two-batch
  native check on an explicitly selected application; it supplies no gameplay
  labels or addresses. Use a separate test session and data directory.

Final application SHA-256:
`33cbd5407b3f70682a09f3c266a13c321cbffd1dec5e4297fe3e52f5ad49fd6e`.

Automatic scheduling, cancellation, revisiting changed allocations, canonical
representation discovery, semantic observation and causal gameplay validation
remain unfinished. This work earns native-access credit only.
