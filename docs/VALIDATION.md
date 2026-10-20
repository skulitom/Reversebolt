# Preview 0.1.0 validation — 2026-10-20

## Automatic discovery, control cards and frozen predictions

Sixty-seven workspace tests pass, including automatic connection scheduling,
failed connection and explicit opt-out behavior, fixed predictions, late
observations and invalidation. Strict Clippy and formatting pass. Application
and prediction integration tests were rerun after the initial presentation
change; the final card layout passed strict Clippy and native UI checks. The
release practice and real MCP suite passes with 21 tools.

One real MCP connection to an owned WinForms fixture collected 16/16 expected
values and compared 3.9 MiB, without a capture, start-job or correlate command.
It found zero hypotheses. Pause, resume, cancel, completion limits and manual
opt-out passed. The desktop picker independently started its own automatic pass;
UI Automation confirmed automatic observation and secondary manual controls. This is
orchestration evidence, not game discovery. See
[the detailed report](research/FROZEN_PREDICTIONS.md).

Blue Prince's four initial correlations all contradicted later observations
under the frozen mappings. Its selected OCR counter scored 12 correct, 2 missing
and 2 wrong across 16 captures. Agent image interpretations remained separate.
No gameplay control or write was verified. Saved artifacts retain the failures.

Testing preference: prioritize automated checks, recorded-input replay and owned
windows. Live-game interaction and manual image inspection should resolve a
specific unresolved question, not substitute for automated regression coverage.

The final native UI check verified a Quantity card with its observed value,
Learning state and Inspect explanation. UI Automation changed a verified
practice value to 123 through the numeric widget and Apply; the engine confirmed
the change and Undo restored 34. No game writes occurred. Evidence:
`test-results/automatic-report-v1/cards-final/ui-check.json`.

Preview `0.1.0-20261020-233851` is running on the normal desktop. All seven prior
activity entries were preserved. Published executable hashes match the tested
release, model hashes match their manifest, and the desktop shortcut resolves
to this version. The final published-build smoke passed:
`test-results/smoke-3e6cd17d/report.json`. Delivery evidence is
`test-results/automatic-report-v1/desktop-delivery.json`.

## Previous observation jobs preview: 0.1.0-20261020-214537

Fifty-three workspace tests, strict Clippy and formatting pass. Seven app tests
and the release practice/MCP suite were rerun after the final UI wording change.
The final suite passes with 20 tools:
`test-results/smoke-57937da7/report.json`.

Blue Prince in Anode verified real MCP job completion, exact retry, rejection of
conflicting IDs, pause, cancellation during a native capture, events, target
replacement and restart. Status responded in 27 ms and cancellation in 53 ms in
the measured run. The current capture finished and the next did not start.
Desktop Take control blocked agent mutations; human resume/pause/stop/release
worked. A separate human-started Observe over time job paused and stopped after
six captures. Restart left unfinished work Interrupted and preserved all nine
existing capture/evidence files byte-for-byte. These checks are coordination
evidence on a paused game, not automatic value discovery or game editing.

The final version is running on the user's normal desktop with all five prior
activity events retained. Both published executable hashes match the tested
release files; the desktop shortcut targets this version. Anode test engines
were closed and the lease released, leaving Blue Prince paused. See
[observation job evidence](research/OBSERVATION_JOBS.md) for scope, limitations
and raw report paths. Delivery is recorded in
`test-results/jobs-native-v1/desktop-delivery.json`.

## Previous memory coverage preview: 0.1.0-20261020-212137

Twenty-one workspace tests, strict Clippy and formatting pass. Release
practice/MCP regression checks pass with 17 tools:
`test-results/smoke-6804a334/report.json`.

The integrated batch fixture finds a value beyond the initial window and
retains its byte witnesses without joining separate trajectories. Tests cover
archive failure, stale replay, observed mapping changes, bounded resources and
late renderer storage reservations. The independent coverage prototype's 12
tests and 56 saved-artifact checks were also rerun; its full benchmark was not.

Paused Blue Prince completed two 12-capture batches, with 32 MiB of complete
memory histories. A third batch increased unique reads to 48 MiB. Both OCR
sources were active, images were correct WGC captures, and no warnings occurred.
The static scene lacked changing observations, so correlation refused to run;
this is coverage evidence, not discovered gameplay values. Anode verified desktop
advancement and historical summaries; real MCP advancement shared that state.
Restart preserved both batch archives and 31 history events while invalidating
the live connection. See [memory batch validation](research/MEMORY_BATCHES.md)
for exact artifacts, timing, limits and failures.

The new version is running on the user's desktop with all five prior activity
events retained. Both published executables match the tested release binaries;
delivery is recorded in `test-results/memory-batches-v3/desktop-delivery.json`.

## Previous agent observation preview: 0.1.0-20261020-210429

Thirteen workspace tests, strict Clippy and formatting checks pass. The final
release practice/MCP suite passes with 16 tools:
`test-results/smoke-cc3df47f/report.json`.

Blue Prince images returned through actual MCP image content were inspected
and recorded as provisional counter readings 38, 37 and 36. Text, boolean and
uncertain interpretations were also accepted. Anode verified the desktop's
source, explanation and unverified status. This is an agent-assisted integration
test, not a blinded accuracy benchmark or a new automatic OCR result.

Nine invalid submissions left the accepted observations unchanged. Exact replay
was idempotent; stale investigation IDs and altered image checksums were
rejected. Restart retained the archive byte-for-byte and 26 history events,
while invalidating the live connection. Unit tests also cover persistence failure,
geometry changes, target exit, backfill and numeric matching with fixture labels.
See [agent observation evidence](research/AGENT_OBSERVATIONS.md) for artifacts,
scope and remaining limitations. No external-game write was performed.

The published executable and MCP adapter match the tested release binaries.
The new desktop preview is running in the user's normal Windows session with
all five previous history events retained; the desktop launcher points to it.
Anode test engines were closed and Blue Prince was left paused in the test
profile. Delivery verification is recorded in
`test-results/visual-agent-v1/desktop-delivery.json`.

## Previous reliability preview: 0.1.0-20261020-203437

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

## Native PP-OCRv6 integration

The current release passes 60 workspace tests, strict Clippy and formatting.
The release regression suite exercises 20 MCP tools and three randomized
practice seeds, including edits, restoration, stale restoration refusal and
restart invalidation (`test-results/smoke-1eea615a/report.json`).

The frozen native observer tracks 12/12 fresh Blue Prince counter samples and
12/12 fresh ordinary-window values. Development replay scores and remaining
false detections, timing, source/model hashes and real memory comparison are
documented in [native OCR evidence](research/NATIVE_OCR.md). The 18 real-game
hypotheses are unverified; no arbitrary-game editing capability is inferred.

Anode confirms shared activity and source-tagged text in the native desktop.
Owned test windows were closed and the game left paused. Desktop preview
`0.1.0-20261020-224603` was published with verified models and binaries, launched
after the prior instance was confirmed idle, and retained all five prior activity
entries. Local delivery proof: `test-results/ocr-native-v1/desktop-delivery.json`.
