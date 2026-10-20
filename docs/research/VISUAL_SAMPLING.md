# Faster visual sampling — 2026-10-21

## Change and scope

Text recognition previously occupied the sole engine worker between every pair
of image/memory captures. Full mode preserves that behavior. The new visual mode
skips OCR, retains image and memory evidence, and records missing numeric readings
instead of repeating an old value. Recognition is not automatically deferred or
replayed later. External single-image interpretations can still be correlated
against these snapshots; temporal interpretations remain descriptive.

The shared MCP capture and observation-job interfaces expose the mode. Visual
jobs begin immediately and accept a minimum 50 ms post-completion delay. Full
mode requires 250 ms. Each job still has at most 16 captures and every mode uses
the same 20-moment, 512 MiB per-batch limit and investigation budgets. The mode
is persisted in each step; legacy full-mode payloads serialize identically for
durable retry compatibility. Cancellation and takeover use the existing owner.

The normal application-selection flow remains automatic full-mode discovery.
Visual sampling is an additional agent investigation capability, not an extra
required step for the user. The desktop labels visual jobs and distinguishes
skipped recognition in capture details. This does not recognize motion or enable
new game edits by itself.

## Native measurement

An owned 560×260 WinForms text window displayed a fixed quantity. The same
published build and pinned PP-OCRv6 models were used in Anode. Each mode had
one warm-up capture followed by six measured captures; full was measured first.
All images used Windows Graphics Capture and each captured 1 MiB of memory.
The full-mode images contained actual neural OCR words. The host was shared;
this is one small window/run, not a controlled hardware or game benchmark.

| Measurement, median | Full | Visual |
|---|---:|---:|
| Capture operation | 1,793.5 ms | 193.5 ms |
| Text recognition | 1,592 ms | 0 ms (skipped) |
| Actual start-to-start interval | 2,124.665 ms | 310.938 ms |
| Requested post-completion delay | 250 ms | 50 ms |

Capture time was about 9.27× shorter in this run. Start-to-start intervals include
different requested delays and controller/persistence overhead, so that ratio
is not an isolated OCR speedup. The visual result is roughly 3.2 samples/second,
not a promised 20 fps. Starting a fresh graphics session per capture, PNG work,
memory reads and job bookkeeping remain costs. This cannot reliably resolve
fast animation; a retained graphics stream and independent motion evaluation
remain required work.

## Verification

The 75-test workspace suite passed, followed by a newly added integration test
for visual captures, image-bound external matching and prospective validation:
76 passing tests in total. Strict Clippy and formatting pass. Tests verify that
visual mode never invokes recognition, clears supplied text, retains memory,
leaves numeric gaps, resumes full recognition, rejects incompatible options,
preserves immutable scope/mode and cancels after the current operation settles.
Legacy capture plans retain their serialized identity.

The actual MCP native test verified six image hashes for each mode, idempotent
job retries, invalid-mode rejection and rejection of automatic comparison in
visual mode. A second visual job paused at capture 8, resumed, and cancelled at
capture 9. No later capture occurred. A subsequent full capture restored text
reading without filling the previous gaps. No game input or manual screenshot
inspection was used.

Reproduce with `scripts/observation-fixture.ps1` and
`scripts/visual-sampling-smoke.py`, using an isolated engine on the same Windows
desktop and `REVERSEBOLT_BIN_DIR` pointing to a published build with models.
Raw evidence is under `test-results/visual-sampling-v1/engine/`; compact metrics
and build hashes are in `research/visual-sampling/evidence/native-window-v1.json`.

## Desktop delivery

Preview `0.1.0-20261021-000945` is running on the normal desktop. All seven
existing history entries were preserved exactly. Both executable hashes match
the release build, both model hashes match their manifest, and the desktop
launcher resolves to this preview. The published-build smoke passed with 23
MCP tools, practice edits/restoration, shared-session checks and restart history:
`test-results/smoke-d9bb897a/report.json`.

The cadence measurement used `0.1.0-20261021-000558`; the later preview changes
capture-detail wording without changing the measured capture implementation.
Delivery proof is `test-results/visual-sampling-v1/desktop-delivery.json`.
