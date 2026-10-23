# Sable: unchanged native discovery failed during naming

The same published Reversebolt build used for the Viewfinder live test connected
to Sable, scanned its runtime and selected promising types. Its first proposal
request then exceeded the existing 90-second limit. **No named hypotheses,
comparisons, gameplay edits or freezes resulted.** The outer observation job
completed while the optional naming review explicitly reported `unavailable`.
This is a retained failure, not independent-game acceptance.

Sable is independently developed by Shedworks and its
[official Steam listing](https://store.steampowered.com/app/757310/Sable/) declares
single-player. We used installed build 10132013 with preview
`0.1.0-20261022-235934`, its four unchanged binaries and the exact source snapshot
from the prior Viewfinder evaluation. Current repository HEAD is deliberately
recorded separately from that older product source. No game source, symbols,
decompiled assemblies, layouts or manually selected fields were supplied.

The [protocol](PROTOCOL.md) was frozen before launch. Four setup images and the
ordinary Continue action reached gameplay. The unchanged evaluator collected
fifteen images and delivered four guarded 200 ms D/A/D/A holds; it did not assign
meanings to those keys. No measurement image was manually reviewed.

The bounded runtime catalog retained 338 types, 1,808 fields and 44,901 reachable
paths. Paths can alias and this does not establish complete runtime coverage.
Fourteen completed selection requests covered the catalog through rounds of
11, 2 and 1 pages, taking a summed 43.276 seconds. The first request for actual
questions failed, so the later activity-interpretation stage did not run.
The whole evaluator completed in 211.626 seconds with process exit code zero;
that is a harness completion, not successful control discovery.

The server log records a reload to a 32,768-token context and an 801-token prompt
before the timeout. This evidence does not support blaming prompt overflow.
The failed HTTP body was not archived by the older product. A separately declared
[context diagnostic](CONTEXT-PROTOCOL.md) reconstructs the first request from
frozen code and saved selection, with the game closed. It cannot prove the cause
of the native failure or reproduce concurrent game GPU load.

The runtime helper completed and unloaded, with 144 recorded pauses, maximum
stop-to-resume time 5,641 microseconds and no helper allocation/deallocation
while paused. All owned test processes exited. The user's main desktop editor
remained responsive; antivirus, real-time protection and tamper protection
remained enabled, and the cleanup check found no new Reversebolt detection.
Anode's lease was released. No Windows Security setting or external submission
was made.

The main save file `SaveData/sable_data_t7pna00g`, its image, settings file and
Steam cloud descriptor are byte-identical to their pre-launch copies. The game
changed `SaveData/SaveManager` and its logs; those changes have not been described
as a full save rollback. The six-file pre-launch folder backup remains local.

[Artifact checks](audit.py) and [results](results.json) bind source/binary hashes,
page coverage, captured images, input records, helper reports, cleanup and save
differences. All native artifacts remain in `test-results/independent-sable-v1`.
Run `python research/independent-sable/audit.py` to recheck them without starting
a game or model request. Progress stays **36/100**; the full objective is active.

## Saved-request diagnostic and bounded code change

The reconstructed first request has 3,540 UTF-8 bytes and evaluates to the same
801 tokens recorded by the native server log. With the game closed, all four
declared requests completed and returned the same empty hypothesis list:

| Context capacity | First request | Second request |
| --- | --- | --- |
| 32,768 tokens | 6,141 ms, including model load | 218 ms |
| 16,384 tokens | 6,469 ms, including model load | 218 ms |

Both contexts work in this condition. The diagnostic does not prove context size
caused the native timeout. GPU contention or another transient live condition
remains possible; per-process GPU memory was not recorded during that failure.

Small text requests now use 16,384 tokens when their **complete byte length plus
the unchanged output allowance and 512-token headroom** fits. Larger requests
retain the original context, and evaluated-token accounting still rejects
possible truncation. No labels or candidate rows are removed to fit, and naming
prompts, quotas, citations and edit guards are unchanged. Failed naming stages
now retain exact prompt/schema/options, elapsed time, failure phase, error and
any returned response. A failed stage cannot seed object-text fallback questions.
Legacy receipts remain readable.

A [separately frozen full catalog replay](REPLAY-PROTOCOL.md) completed 21 model
requests in a summed 62.344 seconds and retained three cited hypotheses:
`max ground incline`, `usables check range`, and `Grip Multiplier`. These names
come from supplied runtime labels, not hardcoded game knowledge. The largest
proposal group still required 32,768 tokens: 56,542 prompt bytes and 16,844
evaluated input tokens. Reducing every request to the smaller context would be
incorrect. Its need for a larger context remains relevant to future live tests.

No new image, target access, input or field write occurred in either replay.
The [replay audit](audit-replay.py) verifies exact source/evaluator/report hashes,
every supplied group row, context decisions, citations and responses. Its
[results](replay-results.json) explicitly leave `native_timeout_resolved` false.
The three names are untested questions, and game-closed latency is not a live
performance comparison.

All **459 workspace tests**, strict all-target Clippy and formatting pass.
Three new generated tests cover context boundaries, incomplete-request evidence,
identity-check failure and legacy serialization. Native revalidation and desktop
publication of this change remain outstanding. The user's connected desktop
session is preserved on its existing build.
