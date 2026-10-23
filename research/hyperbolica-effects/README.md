# Automatic discovery, intervention and restoration in Hyperbolica

The unchanged published preview `0.1.0-20261023-093127` automatically discovered
`walking speed` and `jump speed` in the installed Hyperbolica build 8675290.
The shared planner chose walking speed, proposed zero, measured its effect and
restored the original value. No field, route, address or replacement value was
manually chosen. No game profile, game source, decompilation or source-derived
layout was used. Hyperbolica is a known development target, not a withheld game.

The earlier native evaluation found a walking-speed hypothesis but never edited
it. This is a fresh native comparison through the current ordinary product/MCP
workflow, not a replay of that old proposed plan.

| Condition | Stored f32 value | Product response median | Independent full-frame grayscale median |
|---|---:|---:|---:|
| Baseline | 2.25 | 13.8450 | 13.1779 |
| Changed | 0 | 0.9460 | 1.1057 |
| Restored | 2.25 | 13.0963 | 12.1718 |

The product classified the comparison as `suppression_observed`: the rendered
response fell by 93.2% during the edit and returned to 94.6% of baseline after
restoration. These values measure image change, not speed in physical units.
The independent metric uses complete decoded images without the product's crop,
resize or blur and shows the same direction. This supports an action-associated,
reversible effect; it does not establish exact gameplay semantics, persistent
actor identity, complete world rollback or the absence of downstream effects.

## What was run and checked

The [protocol](PROTOCOL.md) and [freezer](freeze.py) retained 237 source files,
the four published binaries, game executables/runtime, installed manifest,
local model identity and before-storage copies. Setup used one Continue click
and three viewed images. No measurement image was manually viewed. The user's
main editor and Steam client stayed in session 1; testing used hidden Anode
session 3 and a fresh engine data directory.

The unchanged generic evaluator collected 15 discovery images and four 200 ms
D/A/D/A probes. Fresh native discovery retained 184 types, 603 fields and 22,146
reachable paths; paths can alias and this is not complete world coverage.
Seven metadata model requests produced two accepted, zero rejected questions.
One separate activity interpretation described consistent foreground movement
but explicitly left its cause ambiguous. Six additional image-object proposal/
review requests completed. All model calls used the frozen local model.

The planner selected the candidate and inferred four holds of 213/199/196/198 ms
from sampled inputs. The test then captured six images in each of three
conditions, executing the same four actions in each. The artifact audit checks:

- All 33 discovery/comparison image hashes, their sampled inputs and foreground
  ownership, plus 18 action plans/outcomes and 12 comparison key holds.
- Exact source/model/binary/game identity, response schemas, evidence reference
  ownership, complete citation text and retained ambiguity.
- The selected plan, guarded write and post-condition value readbacks. Original
  bytes `[0, 0, 16, 64]` became `[0, 0, 0, 0]`, then returned exactly.
- Write/restoration ordering relative to images and readbacks, matched action
  durations, original product medians and a separate grayscale calculation.
- Seven helper operations with confirmed unload; 90 recorded pauses show zero
  helper allocations/deallocations. The largest recorded pause is 3.924 ms,
  which is telemetry rather than a hard real-time bound.
- No unresolved jobs, trials, tests or freezes. Owned game/engine/MCP processes
  exited, Anode was released, and the user's editor remained responsive.

The existing `game.save` and Steam app storage bytes are unchanged. Normal
Player logs and Unity analytics changed; before/after copies are retained. No
complete world or storage rollback is claimed.

Run `python -X utf8 research/hyperbolica-effects/audit.py` to recheck the retained
evidence without a game, model or screenshot viewing. Add `--verify-current` to
also compare working product sources to their frozen copies. The
[results](results.json) record the successful audit and the precise limits.
Full artifacts remain local in `test-results/hyperbolica-effects-v1`.

## Progress

Progress is **39%**, with validation increasing from 5/15 to 6/15 for a fresh
automatically selected, measured and restored effect in another independently
implemented game. Generalization remains 0/10: this was a known development
target and is not the withheld-game acceptance gate. Product code is unchanged;
the prior 510-test release remains selected by the desktop shortcut.

Next: exercise automatic recovery and fresh effect revalidation across a game
restart using this run's retained route recipe, then broaden unchanged discovery
evaluation. Jump speed and other plausible fields remain untested.
