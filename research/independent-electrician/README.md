# Electrician Simulator: unchanged pipeline, negative first comparison

Preview **0.1.0-20261023-093127** completed discovery, one automatically chosen
native comparison and exact restoration in Electrician Simulator build 13101022.
It did **not** establish a useful gameplay effect. Progress stays **40/100**;
generalization stays **0/10**.

This game was selected before inspecting its runtime metadata, using installed
availability, IL2CPP support and a first-person setting for the unchanged short
movement probes. No previous evaluation was found in recorded research/docs.
The [official Steam listing](https://store.steampowered.com/app/1080020/Electrician_Simulator/)
identifies it as single-player. This is one withheld-from-development target,
selected for backend compatibility, not a representative sample of games.

## Frozen method

The [protocol](PROTOCOL.md) freezes 237 source files, four published binaries,
local model identity, game/runtime hashes, app manifest and both known storage
locations before launch. The existing `research/independent-runtime/run.py` ran
once, unchanged. No source decompilation, game-specific field selection, prompt
tuning, alternate candidate or retry was used.

Anode launched the game using the user's existing Steam client. Setup used six
images and six actions: focus, three title/notice acknowledgements, Continue and
its confirmation. The image budget ended at that confirmation; the evaluator
started after a loading wait. There was no additional manual image to verify the
loaded scene. Automatic model evidence describes a furnished interior, with
uncertainty retained. No measurement screenshot was manually viewed.

## Findings

- Fifteen automatic images and four 200 ms D/A/D/A probes were retained.
- Native discovery completed in **4.733 s**. Its partial catalogue supplied 504
  types, 2,344 fields and 61,667 reachable paths, which can include aliases.
  Full root/object-graph coverage was not achieved.
- Naming used **25 metadata model calls** (242.586 s summed call time), plus one
  activity interpretation. It accepted seven provisional questions with no
  lexical/citation rejection: wide cast enable, raycast distance, walk speed,
  jump speed, min force, camera lerp speed and player money. Acceptance checks
  grounding, not actual gameplay meaning.
- The separate visual-object pass exhausted its budget after six calls. The
  activity interpretation reported inconsistent changes; independent patch
  matching also lacked enough support. Those failures did not block naming.
- The ordinary planner chose **wide cast enable**, changed true to false, ran
  its recorded action sequence, restored true and repeated the sequence.
  The full evaluator took about **414 s**. No preferred candidate was substituted.

| Measurement | Original | Flag false | Restored |
|---|---:|---:|---:|
| Product rendered-response median | 23.2672 | 23.1066 | 23.1028 |
| Independent full-frame grayscale median | 26.2009 | 26.1449 | 26.1464 |

The outcome is **suppression_not_observed**. This says only that this edit did not
suppress the measured response under these actions. It does not establish the
flag's semantics, disprove its usefulness, or validate the other six questions.
In particular, no walk-speed or money effect was tested.

The planner currently takes the first untried choice after its declared-type
round robin. It selected an interaction-related question for a rendered-response
suppression test; walk speed was the second choice. This exposes a gap between
question selection and an experiment that could answer that question. Future
work should improve that general relationship without adding a field-name
whitelist or counting a tuned rerun as another withheld success. Naming latency
and partial visual review are separate remaining limits.

## Evidence and cleanup

[Audit](audit.py) verifies frozen identities, all 33 image hashes, model schemas
and evidence references, input records, planner choice, exact native readbacks,
comparison chronology and restoration. Seven helper operations completed and
unloaded. All **139 recorded pauses** had zero helper allocations/deallocations;
the maximum stop-to-resume time was **5.571 ms**. The written byte was restored
exactly: `[1] → [0] → [1]`.

The audit preserves the visual budget error rather than requiring every optional
stage to succeed. It checks retained evidence and does not turn its own passing
status into a successful gameplay/generalization result. No unresolved job,
test, trial or freeze remained. Owned game, engine and evaluator/MCP processes
exited; Anode was released. The user's main editor remained responsive.

Existing save files and Steam app storage are unchanged. Only `Player.log` and
`Player-prev.log` changed. Before/after copies are retained locally. Nothing was
uploaded and no Windows Security setting was changed.

Run `python research/independent-electrician/audit.py --verify-current` to audit
the local `test-results/independent-electrician-v1` artifacts. The compact
[results](results.json) are committed; raw captures, models, RPC receipts,
backups and process/job records remain local. Product sources and the published
preview are unchanged, so the existing 510-test release verification applies;
no duplicate build or unrelated test run was performed.
