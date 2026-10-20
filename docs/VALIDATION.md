# Preview 0.1.0 validation — 2026-10-20

## Current reliability preview: 0.1.0-20261020-203437

Blue Prince exposed an access violation after an idle interval: Windows unloaded
the OCR DLL while windows-rs retained its factory pointer. A process-lifetime MTA
hold now keeps that runtime available. Ten workspace tests, strict Clippy,
formatting and release practice/MCP smoke checks pass, with report
`test-results/smoke-88bb4d9e/report.json`. The added short OCR/thread-lifetime test
also passed before the fix; it is not claimed as a crash reproducer.

The fixed release completed twelve gameplay captures, then another after a
199.899-second idle gap in the same process, without warnings or a crash.
The changing counter failed recognition: zero correct, three incorrect, eight
uncertain and one missing sample. Memory correlation refused insufficient
observations. See [observation evaluation](research/OBSERVATION_EVALUATION.md)
and `test-results/blue-prince-changing-v2/score.json` / `idle-report.json`.
No game-specific reading rules were added.

## Previous observation preview: 0.1.0-20261020-201128

Nine workspace tests, strict Clippy and formatting checks pass. Tests cover
conflicting and overlapping readings, invalid numeric input, region feedback
and expiry, resize invalidation, memory-before-recognition ordering, incomplete
pages and process exit, alongside the existing discovery cases.

The published binaries passed `scripts/smoke.py`:
`test-results/smoke-4e4aab25/report.json` records three practice seeds, verified
practice edit/restore, 14 MCP tools, shared state, authentication/Origin
rejections, stale restoration rejection and retained history after restart.

Native original and fresh 12-value sequences both passed exact track comparison.
All fresh captures used Windows Graphics Capture at 560×260, with one numeric
track and zero memory hypotheses. Reports are
`test-results/observation-v3/native-report-original.json` and
`native-report-fresh.json`. The initially failed combined-reader test is
preserved as `failed-native-status.json`; the regression and correction are
explained in [observation evaluation](research/OBSERVATION_EVALUATION.md).

Anode and saved images verify actual Blue Prince window rendering where the old
PrintWindow path returned black, including an obscured window. Saved gameplay
also exposes false OCR digits from icons. Half-Life 2 development replay yields
33/36 correct numeric samples and three abstentions. These are observation
results; neither game has passed a memory-discovery or editing acceptance gate.

The versioned desktop build is running in the user's normal session with the
five prior history events retained. Isolated test data was used throughout.

## Initial automated evidence

`cargo test --workspace`: two discovery regression tests pass, covering robust
inverted scaling with observation noise and the inability of passive matching
to distinguish identical control/display trajectories.

`cargo clippy --workspace --all-targets -- -D warnings`: passes.

`scripts/smoke.py` passes against debug and release executables. It uses real
Windows process memory and a real MCP stdio lifecycle, not mocked tool results.

| Random layout seed | Health matches | Ammunition matches | Unique surviving controls |
|---|---:|---:|---:|
| 8675309 | 4 | 4 | 2 |
| 1001 | 3 | 4 | 2 |
| 20261020 | 3 | 4 | 2 |

Each seed passes edits and verified restoration on both tracks. Attempts to edit
before testing are rejected. Tests reject unauthenticated HTTP and browser-origin
requests; a subsequent state change makes restoration refuse to overwrite newer
bytes. Engine restart retains history and invalidates controls from the old
process. A genuine MCP client negotiates the protocol, lists nine tools, changes
the already-running engine's value, and observes that change through the shared
API. The history identifies the source as an agent.

Local raw release report:
`test-results/smoke-1542715d/report.json` (generated evidence, ignored by Git).

## Desktop evidence

Anode hidden Windows session, 1280×720 at 100% scaling:

- Open practice, Observe changes, Find matches, and Test effects were invoked
  through Windows UI Automation.
- A practice Health value changed from 38 to 120 and restored to 38 in the
  initial build, with visible verification messages.
- Review found low-contrast sidebar headings, an oversized initial window, and
  a restore action below the fold. All three were corrected.
- Release window opens at 1136×659 including borders, entirely within the screen.
  Both value controls and the toolbar's restore action are visible.
- Agent-issued API actions populate the visible desktop values and activity log.
- Release Health changed from 61 to 120 and restored to 61 through UIA.
- The text input initially ignored UIA SetValue. Explicit accessibility action
  handling was added; setting the release input to 120 through UIA then passed.
- Owned test windows and practice processes were closed; the Anode lease released.

## What this does not prove

The practice supplies observation labels, clean numbers, a bounded arena, and a
paused simulation. Its data model uses two scalar channels. This does not prove
screen recognition, appearance or behavior discovery, full-process performance,
real-game compatibility, safe rollback of a running game, arbitrary numeric
representation identification, or control of unobserved side effects.

The independent GameDiscoveryLab research includes difficult failures, notably
overlapping interpretations of floating-point values. Those findings remain
applicable; the successful practice seeds do not erase them.

## Application connection and capture — 2026-10-20

Four workspace tests and strict Clippy pass. The added tests preserve missing
OCR detections as gaps, keep adjacent number tracks separate, exclude pages
missing from a trajectory, and revoke connected status and hypotheses on exit.
The final release also passes the existing process/edit/restore/auth/restart
suite and MCP lifecycle with 14 tools:
`test-results/smoke-8b266a25/report.json`.

In Anode, UI Automation opened the application picker, searched for an owned
WinForms window, selected it, connected it, and invoked capture. The workspace
displayed the correct name, title and PID. A nonexistent search showed an empty
result and disabled Connect; Cancel returned to the workspace. The final dark
layout was reviewed at 1280×720, including the prominent sidebar connection
button and a picker that does not resize as filtering changes.

`scripts/native-smoke.py` against the owned `observation-fixture.ps1` window
verified:

- A stale creation identity and wrong window are rejected; failed replacement
  preserves the existing connection.
- Twelve varied on-screen numbers are recognized and associated into one track,
  with memory read and a saved target-window image for each capture.
- The 560×260 captures took 11–40 ms (mean 15.2 ms) on this machine. This is
  fixture timing, not game-performance evidence.
- Disconnect leaves the target running. Closing the target invalidates the
  connection automatically and prevents captures or stale reattachment.
- Correlation ran over the 4 MiB sampled memory and found **zero hypotheses**.
  This proves the observation path, not successful value discovery.

Raw native report and images:
`test-results/application-picker-v2/native-report.json`.

The initial run failed on an OCR result merging `Quantity` and `56` into one
word after needless upscaling. Capture now preserves native resolution unless
it exceeds the OCR limit; the unchanged 12-value fixture then passed. Merged
words, unusual fonts, localization, moving HUD elements, renderer compatibility,
non-atomic screen/memory timing, partial coverage and inaccessible targets remain
limitations. External application editing is not enabled.
