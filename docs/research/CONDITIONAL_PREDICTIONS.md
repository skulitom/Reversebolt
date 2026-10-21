# Conditional predictions from sampled episodes

Reversebolt now automatically freezes simple competing explanations when the
same sampled control sometimes accompanies a change and sometimes does not.
It prepares predictions for the next observed starting context, then compares
them with later episodes. This works through the shared engine without a new
report screen, model call, input injection or game profile.

## Research basis and boundary

The read-only `experiments/rule-inference/HANDOFF.md` and `CONTRACT.md` were
reviewed. Their synthetic evaluation includes a false acceptance where fresh
trials never enter the context separating an incorrect threshold from the
truth. That motivates preserving alternatives and unresolved boundary ranges,
then looking for contexts where the predictions disagree.

That research library was **not linked into production**. Its rate/delay models
assume delivered zero-order-held actions, calibrated measurement uncertainty,
stable object coordinates, and independent/randomized trial attestations.
Passive Windows key samples do not satisfy those assumptions. The new original
Rust implementation uses the narrower evidence that the engine actually has;
it grants no causal or editing authority. The existing research remains untouched.

## Candidate language

A bounded episode is defined by `action_patterns`: contiguous eligible
single-key intervals, with readable no-key-detected intervals on both sides.
At least three bounded episodes are required, with both unchanged outcomes and
one consistent nonzero kind of change (increase, decrease, or nonnumeric change).
Changed no-key episodes prevent initial fitting. Mixed/reversing outcomes and
constant response cannot produce a conditional rule.

For each eligible starting feature, fit either:

```text
observed change when starting feature is on the higher side of a boundary
observed change when starting feature is on the lower side of a boundary
```

Features are active OCR numbers, provisional scalar/boolean image declarations,
and the count of earlier bounded episodes for this same sampled key. The history
feature is an alternative explanation, not evidence of a hidden attempt counter.
The target's own starting value can be a feature. No semantic names are inferred.
Missing/uncertain context is not imputed. Vector/text targets can have changed
outcomes, but are not scalar conditioning features.

Positive and unchanged feature values must separate completely. The entire gap
between their nearest boundaries is retained; no midpoint is chosen as the
identified threshold. Predictions within that gap are unknown. Predictions
outside it are tentative extrapolations of the stated candidate, not calibrated
confidence. Two feature sets can remain observationally equivalent. Real rules
can depend on unobserved variables or be absent from this small language.

Up to 32 features are considered, including history first; each check reports
the number omitted. At most eight checks are retained per memory batch. Each
target/source/key can freeze only once in that batch. Fitting stops after moment
17 (earlier for a large memory budget) so at least three of the engine's available
capture slots can remain. Observation
jobs retain their existing maximum of 16 captures; no budget was increased.

## Actual prospective ordering

Each check stores the exact discovery episodes, feature readings, candidate
parameters, image/input scope and freeze boundary. They never refit to future
outcomes. A forecast is attached to a captured baseline image and its checksum,
with a monotonic timestamp set **after prediction computation**. The engine
saves the evidence before publishing it. The timestamp is computation completion,
not a claim of fsync or UI presentation time.

A later bounded episode can challenge the check only if it starts at or after
the frozen capture boundary. Each candidate needs a forecast for that baseline
with known context, prepared strictly before the first recorded key-down sample
in the episode. Forecasts made too late, missing context, and unresolved
threshold gaps are unknown, not agreements. This compares predictions with
sampled press timing; missed earlier input and game presentation timing remain
unresolved.

Current-frame declarations may fill a previously missing candidate forecast once,
with their actual completion time. Old frames never gain backdated predictions.
When such a declaration completes a previous episode, a new forecast for the
current image and revised episode count is appended with a fresh timestamp;
the earlier version is preserved. The challenge selects the matching starting
image/count rather than silently rewriting a prediction.
Discovery observations changed after freezing invalidate the check. Geometry,
input generation, process or memory-batch changes also invalidate it. Checks and
forecasts survive as historical evidence; restart clears the live connection.

Persistence failure invalidates existing checks and stops prediction updates for
that batch, including after the path is repaired. A new forecast that could not
be saved is never published as issued. The last successfully saved archive can
remain historical; `rule_error` and the live invalidation describe the failure.

## Human and agent workflow

Cards say “Checking why the outcome varies.” Inspect shows the competing
explanations, later agreements/unknowns and explanations ruled out by a
contradiction. A surviving model stays **possible**, not “verified.” The shared
`live.rule_checks` contains the complete machine-readable candidates, per-frame
forecasts, first-pressed times and comparisons; `live.rule_error` exposes storage
failure. No additional MCP tool is needed.

An agent can compare candidate predictions for the current starting context,
or seek a reachable state where they differ. Context is a precondition to
observe, never an instruction to write guessed bytes. The engine does not
execute the proposed interaction. State changes, controls not sampled, ordinary
drift, observation error, selection bias and causal confounding remain possible.
No claim about full rollback, damage magnitude, cooldown, hidden-state identity,
mechanism uniqueness or independent-game compatibility follows from agreement.

`rule-evidence.json` stores the checks alongside input/image frames, numeric and
declared image observations and the descriptive episode evidence. It is an
archive, not a transaction with `session.json` or a capability to write memory.

## Tests

Core tests distinguish a starting-value threshold from an action-history rule,
keep frozen candidates byte-identical while future outcomes arrive, preserve
threshold-gap abstention, reject forecasts issued after a sampled press, invalidate
changed scope/discovery, avoid inventing conditional rules for constant response,
and timestamp only newly prepared predictions.

`scripts/rule-check-smoke.py` uses an owned Anode fixture. Two implementations
look identical initially: both decrease twice and then stop. One uses the visible
starting value; the other uses private action history. After fitting, the harness
restores the visible number without resetting history. Five real delivered
presses and production OCR test whether frozen alternatives then disagree and
the later observations eliminate the wrong one. Boolean annotations are declared
fixture data, not model inference. The refill mechanism exists only in the owned
test fixture; this is not a game reset or recovery capability.

## Verified native results — 2026-10-21

The final native matrix is `test-results/rule-checks-v1/native-03/`.
Both modes used the same engine, thresholds and 14-capture schedule. Three
discovery episodes ended at moment 8. The visible value was then restored from
498 to 500 without resetting action history. Moment 9 forecasts disagreed;
two later episodes challenged the frozen alternatives:

| Fixture | History candidate | Starting-value candidate |
|---|---|---|
| State-dependent decrease | Contradicted twice | Agreed twice |
| History-dependent decrease | Agreed twice | Contradicted twice |

The numeric target used production OCR. A separate declared boolean target
followed the same shared contract; it does not establish appearance inference.
Each run verified five delivered key events independently of OS polling, unchanged
frozen candidates, positive forecast-to-first-press timing, session/archive parity,
and restart invalidation with an unchanged archive. The smallest accepted timing
margins were 62.225 ms (state) and 39.204 ms (history). These are sampled-input
margins, not game-delivery latency measurements.

Both runs then replaced only their owned archive path with an empty directory
to force a save failure. New predictions were withheld, old checks invalidated,
and subsequent captures after restoring the path did not silently resume checks.
The prior successful archive remained byte-identical. `storage-failure.json`
and each run's `report.json` preserve those outcomes.

Earlier failures remain recorded:

- `native-01`: the harness requested 18 captures, exceeding the existing job
  limit of 16. It was rejected before observation or input. The harness now uses
  shorter sampled presses within 14 captures; the product budget was not raised.
- `native-02`: a late boolean declaration completed the preceding episode, but
  the current forecast still had the earlier episode count. A later challenge
  correctly stayed unknown instead of counting as agreement. The implementation
  now retains the earlier forecast and appends a new timestamped current-context
  version. A regression test reproduces this case.

`development-failures.json`, the failed traces, and the exact pre-fix executables
remain under `test-results/rule-checks-v1/`. No threshold was adjusted to make the
two fixtures pass. All **128 workspace tests**, strict Clippy, formatting and the
release build pass after the corrections.

The published history-mode run passed independently at
`test-results/rule-checks-v1/published-01/history/report.json`. Its UI Automation
receipt (`ui-evidence.json` beside it) confirms the short card description,
one remaining explanation, one ruled out, and two later episodes checked.
No screenshots or game control were used. The published 29-tool smoke is
`test-results/smoke-22ab43ef/report.json`.

Preview `0.1.0-20261021-045650` is responding on the user's desktop after a normal
close/relaunch, with eight history entries and the existing local model choice
preserved. The delivery receipt is
`test-results/rule-checks-v1/desktop/desktop-delivery.json`. Owned Anode windows
were closed and the lease released. The compact tracked record is
`research/conditional-predictions/evidence/native-v1.json`.

Reproduce inside Anode with the actual child/parent session guards and its
agent/lease environment:

```text
python scripts/rule-check-smoke.py --out <new-directory> --bin <preview-directory> --check-storage
```

`--modes history --visible --keep-ui` leaves the owned app open until
`<out>/history/close-ui` exists (at most five minutes). Existing output directories
are not reused. The harness releases injected keys and closes only its owned
processes. Its controlled refills are test operations, not a product game-control API.
