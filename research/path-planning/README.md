# Keep competing paths available for investigation

The comparison planner previously proposed only primary candidates and deduplicated
them by `(type_id, field_id)`. That grouping cannot distinguish a repeated route
from a different object carrying the same field. It also left retained alternatives
out of the suggested follow-up choices, although explicit test preparation already
supported them.

Current source proposes eligible primary candidates and retained alternatives,
deduplicating only an identical path. A path's own primary hypothesis takes
precedence when another question lists it as an alternative; `question_refs`
preserves both associations. Selected types remain interleaved. The output carries
the exact `selected_candidate` and an explicit primary/alternative `selection_role`.
The question's name and explanation still describe its original hypothesis; they
are not transferred as verified semantics to the alternative.

The runtime traversal already chooses one route per node within an individual
snapshot. Snapshot-local node identifiers cannot establish identity across captures,
and a path can change occupants. The planner therefore leaves relationships between
different paths unresolved. It does not call them distinct actors or independent
causal evidence. Existing comparison statuses attach only to the exact selected
path, replacement value, actions, investigation, catalogue, report and frame/input
metadata hash. Even a positive result does not validate or remove the other paths.

## Saved-case replay

The [protocol](PROTOCOL.md), executable, source and inputs were frozen before the
single run in `test-results/path-planning-v1`. The unchanged `agent-handoff-replay`
example rehashed fifteen Hyperbolica images, recomputed the production measurements
exactly, resolved every choice through the normal validator and completed in
0.875 seconds. The previously archived V3 handoff output provides the baseline.

| Same retained evidence | Earlier planner | Revised planner |
|---|---|---|
| walkingSpeed 2.25 | Proposed zero probe | Same primary proposal |
| jumpSpeed 2.2, retained alternative | Omitted | Separate alternative proposal |
| shakeStrength 0 | Outside probe policy | Still omitted |
| baseFOV 50, retained alternative | Omitted | Still outside probe bounds |

The two original questions, image model's false verdict and uncertainty, original
source attribution and four 198 ms holds are unchanged. Neither proposal was
executed. This run used zero new model calls, target accesses, inputs or writes.
It is a regression on a known saved case, not independent-game acceptance.

All **302 workspace tests** pass, including synthetic checks for same-field paths,
shared alternatives, primary-reference preference, Boolean/probe bounds, stale
evidence and exact prior-result matching. Strict workspace Clippy, formatting and
diff checks pass. Synthetic history records test status handling, not native effects.
The [audit](audit.py) verifies frozen source/input hashes, the archived executable,
unchanged original evidence and complete proposal provenance; [results](results.json)
include the output and test-log hashes.

The main control cards and edit authority are unchanged. No new verified gameplay
meaning or generalization is established. Weighted progress remains **35%**. The
desktop launcher still reports the quarantined preview unavailable; no replacement
build, game run or external submission was made.
