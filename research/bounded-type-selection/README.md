# Bounded type selection

Automatic naming now gives each complete page an explicit nomination allowance.
The allowance appears in the prompt, response schema and retained receipt and is
checked locally. Once there are at most eight pages, eight slots are shared evenly
between them, with unused capacity from small pages redistributed. Earlier
screening rounds keep every page and allow at most eight or half its rows, with a
minimum of one. The model can choose fewer or none.

The existing four-round, 24-selection-request and 12,000-byte page limits remain.
Complete field labels and bounded, path-scoped object text remain intact. Small
catalogues retain their original one-request selection. Historical receipts without
page allowances remain readable. No game name, field preference, offset or source
layout is used to allocate slots.

This fixes a convergence failure, not global ranking. Page order and grouping can
exclude useful types clustered on one page. More than eight inseparable singleton
rows can still fail explicitly; the implementation does not silently discard
pages or expand its budgets to force success.

## Frozen saved-report replay

The [protocol](PROTOCOL.md) preceded inference. The revised evaluator and 233
source/configuration files were frozen, along with five existing reports and the
installed local model identity. Each case ran once through the ordinary full
metadata-hypothesis evaluator. Existing baseline receipts were reused without
repeating their model calls. No game was launched or controlled, no process memory
was accessed, and no image was reviewed.

The Vampire Survivors inventory exactly matches the failed native V2 run. That
baseline exhausted four rounds after **364 → 30 → 18 → 12 → 10**. The revised
selection finishes in two rounds: **364 → 40 → 8**, using 17 selection requests.
Eight field-review requests follow, producing four accepted provisional questions
and one rejected proposal. This is 25 total calls versus the baseline's 20 calls
that stopped before field review; fewer selection rounds do not mean less total
inference once the next stage can run.

| Saved case | Baseline accepted/rejected | Revised accepted/rejected | Revised selection/total calls |
|---|---:|---:|---:|
| Vampire V2 | Failed before proposals | 4/1 | 17/25 |
| Earlier gameplay (`latest`) | 7/1 | 9/0 | 14/21 |
| Menu | 2/2 | 2/0 | 9/16 |
| Owned context fixture | 0/0 | 1/0 | 1/2 |
| Opaque-label variant | 0/0 | 0/0 | 5/5 |

All five runs finish without a naming error. The menu still produces unwanted
device-setting questions (`maxJoysticksPerPlayer`, `force4WayHats`), despite the
instructions to exclude those preferences. Quotas do not solve relevance. The
fixture's `Speed` question comes from the separately implemented same-object-text
fallback, not this quota change. The opaque case continues to abstain.

Accepted questions concern `maxHp`, `moveSpeed`, `startingSpawns` and `interval`.
The last question uses an exact same-object description about damaging zones;
it is not inferred from the generic field name alone. A proposed expansion from
`destructibleFreq` to “destructible frequency” is rejected by lexical grounding.
These are label-supported investigation questions, not verified controlling values,
player identity, scope, safe numeric ranges or gameplay effects.

The [audit](audit.py) checks source/binary/input hashes, the unchanged Vampire
inventory, complete pages, exact path-bound text, every quota and nomination,
prompt hashes, model identity, input-token headroom and retained outcomes. Run
`python -X utf8 research/bounded-type-selection/audit.py` to audit the saved
evidence without a model. Add `--verify-current` to check current source hashes.
Full artifacts remain local in `test-results/bounded-type-selection-v1`.
The [derived results](results.json) retain all question names and rejections.

The four older baselines precede other changes in question handling and cannot
isolate this quota revision. Counts measure coverage and reference/lexical
validation, not semantic precision. Progress remains **38%**, with no new causal
or independent-game generalization evidence.

## Verification

All **497 ordinary workspace tests**, strict all-target/all-feature Clippy,
formatting and the release build pass. One opt-in archive replay remains excluded
from the ordinary test run. Added regressions cover full-allowance convergence,
per-page response limits, capacity redistribution, inseparable singleton refusal
and historical receipt compatibility.

Packaged preview `0.1.0-20261023-081315` passes headless startup/dispatch and the
65-tool MCP catalogue check. The native helper is byte-identical to the previous
preview, preserving its separate native test evidence. The first headless check
incorrectly sent empty argument objects and failed its expected-error assertion;
the corrected check supplies the documented request fields. Both attempts are
retained in `test-results/bounded-type-selection-preview-v1` and `-v2`. No product
change or native game rerun was needed to correct that test.

The desktop shortcut selects this preview for the next launch. The user's existing
editor remained responsive and was not restarted. Next: improve relevance and
establish a usable gameplay baseline before testing any proposed value's effect.
