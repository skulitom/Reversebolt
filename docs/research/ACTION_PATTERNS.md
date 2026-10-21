# Input/value contrasts

Reversebolt automatically compares sampled input context with OCR numbers and
provisional per-image values. It can suggest a short description on a card,
show counterexamples under Inspect, and give an agent a next observation to
try. There are no game names, offsets, key meanings or game layouts in this
algorithm. This is a step toward investigating controls, not verified gameplay
understanding or permission to change memory.

## Method

Adjacent captures must have consecutive numbers, unchanged geometry, a common
monotonic clock, and the current input-monitor generation. Their capture-operation
bounds must not overlap and must span at most four seconds. Resizing, resuming
input collection and starting a new memory batch retire the old comparison scope.

The input window must cover the entire interval without retention truncation.
Only complete polls between the first image operation's end and the next one's
start are considered. At least three foreground polls are required, with no
gap or uncovered edge longer than 100 ms. A key needs at least two pressed
samples. Several different keys anywhere in the interval are ambiguous, even
if they were sequential. Focus loss, unavailable data, missing/uncertain values,
and incompatible value types exclude that interval. Empty sampled key sets
mean **no keys detected**, never proven inactivity.

Numbers use exact observed increase/decrease/equality. Booleans, strings and
equal-length finite vectors use changed/unchanged, without inventing a physical
direction or meaning. Recognition noise has no hidden tolerance. Simultaneous
changes are not attributed to one object or gameplay cause.

Contiguous eligible intervals with the same exposure form one episode. Several
frames from a sustained hold are not independent trials. Opposite numeric
changes within an episode are mixed, even if the final value returns to its
start. Key episodes need readable no-key-detected intervals on both sides;
boundary holds and episodes interrupted by unknown data do not establish a pattern.

The first two bounded episodes for each key must agree on a nonzero kind of
change. Later bounded episodes are compared with that direction without choosing
a more favorable pair. A tentative card description requires at least one later
agreement, no later disagreement, at least two unchanged no-key episodes, and
no changed no-key episode. A later mismatch produces a counterexample. Changes
without detected keys remove the contrast. First-two disagreement or no response
produces no pattern; absence of evidence is never advertised as a learned rule.

These are chronological, descriptive comparisons, **not prospective tests**.
Late provisional image annotations can alter earlier episodes. Episodes are not
randomized, duration-matched or statistically independent; the counts are not
confidence scores. Capture bounds are not presentation timestamps. Short taps,
delayed consequences, scene/state changes, unmeasured inputs, drift, edit
interventions and recognition errors remain alternative explanations. There is
no automatic input injection or write, and no pattern changes editing eligibility.

## Shared evidence and interface

`live.action_patterns` is recomputed on capture and image-observation submission.
Each active OCR/external series carries interval frame references, endpoint
values, timing ranges, exclusions, episodes, first/later comparison indices,
no-key comparisons and a suggested next observation. The exact frames and image
hashes remain in `live.frames`. Per-sequence temporal interpretations are not
treated as point samples. Source identities stay distinct.

Cards may say “Often decreases with D sampled · unverified” or flag inconsistent
effects. Inspect contains counts and the suggested follow-up. There is no new
report page. MCP status exposes the same evidence; no new tool is required.
`evidence.json` retains the comparisons and their numeric/image observations.
Restart drops live authority but preserves that historical file.

## Verification

The core suite covers separated versus sustained episodes, reverse changes,
zero response, counterexamples, drift, focus loss, sampling gaps, mixed keys,
truncation, changed generations, missing frames/values, keys during the ending
capture operation, boolean/vector changes, and invalid types/numbers.

`scripts/action-pattern-smoke.py` runs only in Anode, using the separately owned
`scripts/action-pattern-fixture.ps1`. It delivers three real D key presses with
sampled quiet intervals, verifies delivered fixture events independently, and
uses production OCR for the visible number. Four fixture modes exercise coupled
decrease, no response, a third press with no effect, and independent time drift.
The same engine algorithm and thresholds are used for every mode. Boolean
annotations are explicitly fixture-supplied; they verify the generic contract,
not visual inference. No game inputs, manual screenshots or model calls are needed.

## Verified results — 2026-10-21

All four native modes passed on their first run, with 12 captures each and three
independently confirmed delivered key presses. The coupled run read
500 → 499 → 498 → 497, separated by stable intervals; two initial episodes and
one later agreement produced a tentative decrease. The no-response run stayed
at 500 and produced no pattern. The conditional run stopped at 498 on the third
press and recorded one later disagreement. Drift changed during all four
no-key episodes and produced no contrast. No intervals in these runs were
excluded. Maximum saved input sampling gaps ranged from 16.884 to 18.041 ms;
paired capture-operation spans ranged from 1.063 to 1.226 seconds.

Raw receipts: `test-results/action-patterns-v1/native-01/{coupled,none,conditional,drift}/report.json`.
They retain endpoint readings, all interval/episode evidence, declared boolean
annotations, source fixture event counts and exact binaries. Every run verified
session/archive parity and restart invalidation with an unchanged evidence file.
These fixtures do not test independent games, natural player activity, learned
semantic labels, input confounding or causal attribution.

A final published-build coupled run passed independently:
`test-results/action-patterns-v1/published-01/coupled/report.json`.
Between the matrix and publication, only presentation changed: the counterexample
wording and a bounded scroll area in Inspect. Detection logic and thresholds
were unchanged. The final **120 workspace tests**, strict Clippy, formatting and
release build pass. The published 29-tool smoke is
`test-results/smoke-2f0618e8/report.json`.

UI Automation verified the numeric and boolean card descriptions and Inspect's
two initial episodes, one later agreement and four unchanged no-key episodes.
`ui-evidence.json` beside the published receipt preserves that check. No
screenshots or game control were used. All owned Anode windows were closed and
the lease released. The compact tracked record is
`research/action-patterns/evidence/native-v1.json`.

Preview `0.1.0-20261021-042809` is responding on the desktop. Normal close/relaunch
preserved eight history entries and the existing local model selection. See
`test-results/action-patterns-v1/desktop/desktop-delivery.json`.

Reproduce inside Anode with its agent ID/lease token and explicit session guards:

```text
python scripts/action-pattern-smoke.py --out <new-output-directory> --bin <preview-directory>
```

Set `REVERSEBOLT_TEST_SESSION` to the actual child session and
`REVERSEBOLT_PARENT_SESSION` to its parent. `--modes coupled --visible --keep-ui`
holds the owned app for inspection until `<out>/coupled/close-ui` exists (bounded
to five minutes). Existing output directories are never reused. The harness
releases its injected keys and closes its owned processes on success or failure.
