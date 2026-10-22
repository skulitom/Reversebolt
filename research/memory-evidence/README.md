# Local evidence for memory-first research

Implemented 2026-10-22. Full-goal progress remains **35%**. This work makes future
memory-only candidate experiments reproducible; it does not yet implement a new
candidate producer or establish meaningful controls in another game.

## Why this is needed

The four original exposed 2048/Tetris recordings contain 64 capture records and
49,360 memory-read receipts. The receipts count 888,086,528 retained bytes, but
do not contain those bytes. The known capture folders have no broad memory
payloads. [The metadata audit](corpus-audit.json) hashes the exact capture JSON and
records the counts. `audit-corpus.py` only reads saved JSON; it never runs the
original game harness, reads browser profiles or consults game oracles/source.

The older `LiveSession::archive_batch` format saves at most 256 candidate witnesses
selected from OCR or external numeric matches. It explicitly discards noncandidate
memory. Running a purported memory-first selector on those witnesses would inherit
the screen-based selection and could not measure discovery independent of it.

## Implemented path

The shared engine/MCP `export_memory_evidence` command freezes already-collected
batch memory once, after the caller supplies the exact investigation, batch and
capture count. It saves all retained planned chunks, including bytes with no OCR
match, alongside the full view, sampling/search provenance, failed read receipts,
per-read timing, invalidated chunks and image-only sample membership. It does not
read the target, capture, send input, call a model, change a control or upload data.
No unidentified cards or report UI are introduced.

Payload segments and the complete manifest are hashed. The reader validates the
layout, size and timing scope, rejects mismatched bytes and supplies historical
samples with missing chunks absent. It never reconnects or interprets addresses
as current bindings. The offline `memory-evidence-audit` example verifies a frozen
archive without an application or process connection.

A separate 512 MiB per-investigation disk budget covers payloads and manifests;
the bounded reservation markers add at most 96 KiB. The full size is reserved
before payload writing. Partial failures keep the reservation and cannot be
overwritten. A batch can be exported only once; exact replay verifies the frozen
archive, while a later sample count is refused. Exceeding capacity fails the whole
export rather than silently selecting witnesses or truncating history.

## Verification

All **409 workspace tests** pass. Strict workspace/all-target Clippy and formatting
pass. The first Clippy run rejected a complex test-only callback type; replacing
boxed callbacks with function pointers resolved it. The changed test is rerun
separately after that cleanup. The offline audit example verifies the exported
fixture's exact manifest hash and payload segments.

The generated mixed-sample fixture retains five capture moments, four memory
samples and **4,063,232 bytes**. A mapping changes and later returns: both later
reads remain absent, while the earlier bytes survive. It includes an image-only
moment, broad zero-valued ranges and an unrecognized visual-mode numeric change.
The original full view is preserved. Memory intervals remain separate from image
capture intervals. A sealed target panics on contact during export and exact replay.
The archive remains usable after advancing the batch and dropping the live session.

Other tests alter payload bytes and lengths after opening, change the manifest,
remove segments, duplicate reads/ranges, change clock bindings and sample membership,
exhaust storage with a partial reservation, and alter the reservation charge.
They verify refusal without overwriting existing evidence. The shared service
handler is exercised with an image-unavailable fixture, exact replay, malformed
scope and an attempted custom path. All data are generated fixtures, not game
recordings. [Verification results](results.json) record checks and frozen files.

## Remaining work

Collect new broad-memory evidence only when native operation is available, using
unchanged sampling rules on independent games. Develop blind candidate producers
that operate on the retained bytes without selecting from labels or visible
numbers, then compare their proposals to later held-out behavior and interventions.
Prior exposed 2048/Tetris data are development evidence, not a withheld evaluation.

An export still samples only planned ranges; its plan may already be influenced by
screen-based searches. Retention does not establish allocation identity, atomic
snapshots, continuous input coverage during reads, gameplay meaning or edit
authority. No bytes can be recovered from old receipt counts. The native desktop
remains unavailable under the existing Windows security block. All work stays local.
