# Shared question budget: not promoted

One frozen development replay tested a global twelve-question budget against the
existing per-type allocation. The candidate table shares complete type names and
keeps exact field/object-text bindings. General wording also clarifies object
labels and excludes peripheral configuration. No game-specific dictionary,
offset, profile or source layout was introduced.

The experiment passed its mechanical audit but failed its predeclared semantic
regression criteria. **The production observer policy was restored.** The
[prototype patch](prototype.patch), [frozen protocol](PROTOCOL.md), [receipts and
checks](results.json) and [promotion decision](promotion.json) retain the failure.
There were no prompt changes or repeated calls after inspecting the results.

| Development report | Baseline accepted/rejected | Shared budget accepted/rejected |
|---|---:|---:|
| Latest gameplay | 7/1 | 12/0 |
| Earlier gameplay | 8/0 | 12/0 |
| Menu | 2/2 | 0/0 |
| Owned labelled fixture | 0/0 | 0/0 |
| Opaque-label variant | 0/0 | 0/0 |

The latest gameplay result spends eleven of twelve questions on carrying-object
fields and one on a line-display field. It omits the movement question retained
by the baseline. The earlier gameplay result retains running/gravity/step
questions. The fixture still abstains despite receiving the exact `Speed` text
on its three observed paths. Removing the menu's device-configuration questions
is useful, but increased acceptance counts do not establish better coverage.

All five type selections, inventories and first-stage prompts/options are
unchanged. The same local model digest and server version are retained. Requests
fell from thirty to nine across the five cases; total runner time fell from about
276 to 241 seconds, while the latest case itself became slower. These are single
development runs, not a latency benchmark. Question allocation, table structure
and wording changed together, so their individual effects are not isolated.

The artifact audit checks exact frozen sources/executable/reports, full selected
row retention, complete type labels, exact path/text bindings, prompt hashes,
input-token headroom, model identity and all five runner exit codes. The 179
core/observer tests and strict observer Clippy passed. Artifacts remain in
`test-results/shared-question-budget-v1`. These are known reports; no new game,
target inspection, input, memory edit or image review was used. No control name,
edit authority, gameplay effect or generalization claim follows from acceptance.

The next selection experiment should measure sensitivity to field order and
candidate diversity explicitly. The current result is consistent with ordering
bias, but it does not prove ordering caused the failure. Repeatedly tuning a
prompt on these cases would not establish generalization.
