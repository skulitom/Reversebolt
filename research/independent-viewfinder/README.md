# Viewfinder: native failure, correction and live development comparisons

## Live effect evidence after paging (23 October 2026 local time)

The published paged build `0.1.0-20261022-235934` now completes naming in the
running game: six label-grounded hypotheses, none rejected. Viewfinder is a
known development target following the earlier failure and replay. This does
not replace the original held-out failure or pass multi-game acceptance.

One predeclared comparison selected capsule height and found no suppression.
After that run finished, a separately frozen continuation requested at most four
more choices from the unchanged product planner. No candidate, path, name, value
or input sequence was manually selected. All five trials completed through MCP:

| Planner choice | Original → test → restored | Measured response: baseline / changed / recovered | Result |
|---|---|---|---|
| Capsule height, primary path | 1.88 → 0 → 1.88 | 14.347 / 14.210 / 14.288 | Suppression not observed |
| Step distance | 2 → 0 → 2 | 14.324 / 14.329 / 14.342 | Suppression not observed |
| Max stable move speed | 3.5 → 0 → 3.5 | 14.518 / 0.140 / 14.034 | Suppression and return observed |
| Capsule height, alternative path | 1.88 → 0 → 1.88 | 14.432 / 14.429 / 14.429 | Suppression not observed |
| Fall wind min speed | 5 → 0 → 5 | 14.479 / 14.529 / 14.528 | Suppression not observed |

These numbers are the product's image-difference metric, not physical speed.
The positive comparison reduced the response by 99.0% and recovered to 96.7%
of baseline. A separate full-frame grayscale calculation from the hashed PNGs,
without the product's resize/crop/blur, also finds a strong reduction and return
(14.548 / 0.570 / 13.851). All twelve repeated input holds and the changed and
restored bytes are recorded for each comparison. This supports a controlling
effect in this context; exact semantics, units, actor identity, load/restart
recovery and exclusive causality remain unproven. Negative results do not mean
those fields are useless: the movement sequence does not test every property.

The audit checks 105 automatic images, 64 input holds, source/binary snapshots,
all type-page coverage, proposal citations, command choices, input receipts,
readback chronology, five restorations and 31 helper completions/unloads.
There were three setup screenshots and zero manually reviewed measurement
images. The save and backup remained byte-identical. Normal logs/analytics can
change. Owned applications and evaluators exited; the main editor remains
responsive. Checked Windows protections stayed enabled, with no new Reversebolt
detection in the interval. Nothing was uploaded.

Discovery still took about 320 seconds before comparison preparation. Metadata
model stages alone totalled 215.380 seconds, versus 64.365 seconds in the offline
replay. The three proposal calls took 70.868, 46.522 and 60.893 seconds; type-page
selection was much faster. This exposes a live responsiveness problem, not a
reason to claim automatic discovery is finished.

The outer observation job also ended **Failed** despite publishing the usable
hypotheses: its last checkpoint exceeded the cumulative 4 KiB result budget.
The frozen failed job is retained. The correction stores the naming evidence ID
and publication counts in job summaries; full path lists remain archived and
available through the shared state/API. A regression runs the complete automatic
16-step job with twelve hypotheses and 64 removed preview paths; its successful
checkpoints total 1,571 bytes. Native comparisons above used the pre-fix binary.
All **450 workspace tests**, strict all-target Clippy and formatting pass after
the correction, as does the offline native artifact audit. Logs are retained in
`test-results/viewfinder-live-workspace.log`, `viewfinder-live-clippy.log` and
`runtime-naming-job-budget-focused.log`.

Artifacts: `test-results/viewfinder-live-paging-v1` and the separate
`test-results/viewfinder-followup-v1`. See [live protocol](PROTOCOL-LIVE-PAGING.md),
[follow-up protocol](PROTOCOL-FOLLOWUP.md), [continuation evaluator](continue.py),
[offline audit](audit-live.py) and [audited results](live-results.json).

Validation credit increases from 4/15 to 5/15 for automatically selected,
input-matched live edit/effect/recovery evidence in an independently authored
game. Other credits stay fixed: **36/100** overall, including 0/10 generalization.
Next: test selection matched to the proposed effect, faster live naming, and a
new unchanged independent-game evaluation. The broader goal remains active.

## Original native failure

The unchanged published build `0.1.0-20261022-233909` reached gameplay in
Viewfinder (installed build 20950646) and collected fifteen images, four brief
input probes and a runtime inventory containing 349 types, 1,716 fields and
31,065 reachable paths. One helper completed and unloaded. No field was edited.

Automatic naming failed on its first model request. The local server recorded
a 36,168-token prompt and cancellation at the existing 90-second deadline while
still processing input. The saved receipt reports an unavailable review, with
zero named questions and no comparison. The outer observation job completed;
that does not mean discovery succeeded. [Audited native evidence](results.json).

Setup used three manually viewed images and one Continue click. No measurement
images were manually viewed. The game's save and backup are byte-identical;
normal logs and analytics changed. Both owned applications and the evaluator
exited. The user's desktop build stayed responsive, all checked Windows
protections stayed enabled, and no new Reversebolt detection was found in the
checked interval. Everything remains local.

The native failure is retained in `test-results/independent-viewfinder-v1`.
The [frozen protocol](PROTOCOL.md), source/binary hashes, before/after storage,
image hashes, model/server receipt and unload events are independently checked
by `audit.py`. The test selects a supported runtime; it is not representative
coverage of arbitrary games and does not pass the generalization requirement.

## Correcting large-catalog requests

The observer now partitions large type indexes into complete rows with a
12,000-byte prompt cap. Each page nominates up to eight types; subsequent rounds
compare the nominees with their complete original rows. It checks page-local
IDs, rejects duplicates, preserves path-scoped object text and records every
page/nomination/stage. Four rounds and 24 selection calls bound the work.
Oversized individual types, lack of narrowing, malformed responses and exhausted
budgets fail explicitly instead of silently dropping labels. Existing small
catalogs keep their one-request flow. Timeout messages now identify the deadline.

The development replay uses the same saved native inventory and local model,
without running the game, collecting more images or accessing a target process.
Eleven first-round pages covered all 349 types and every field label, followed
by one nominee-selection page and three proposal calls. Fifteen model calls
completed in a summed 64,365 ms, with no failed call. Six cited hypotheses were
accepted and none rejected. These are label-grounded questions, not tested
controls, and this is a development regression rather than a fresh held-out run.
[Replay audit](paging-results.json).

All **449 workspace tests**, strict all-target Clippy and formatting pass.
Four new tests cover lossless encoded-size partitioning, oversized rows, invalid
page nominations and non-progressing reductions. The full-goal estimate remains
**35%**. Actual effect validation and independent-game acceptance are still needed.
