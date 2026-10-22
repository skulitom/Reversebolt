# Viewfinder: native failure, then a saved-catalog correction

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
