# Path-scoped object labels during type selection

Type selection can now see complete text attached to sampled runtime paths,
alongside the complete field-label index. An opaque or generic field no longer
has to be selected before its nearby label becomes visible to the model.
Examples retain their exact path and field name. A label on one object is never
copied onto another object or treated as a name for every instance of its type.
The subsequent candidate citations and edit guards are unchanged.

The core index visits fields before extra paths and shares examples across
types. It includes at most four examples per type, within 16 KiB of added context
and the existing 110,000-byte total prompt limit. Only complete, nonempty text
of at most 256 bytes is included. Available and omitted sample counts remain
explicit. Field labels are never silently removed to make space: if the base
index does not fit, the optional review fails with an explanation.

The selection context increases to 49,152 tokens; per-group questions retain
32,768. The installed model advertises a 262,144-token maximum. Receipts retain
actual input-token accounting, and missing accounting or input near the limit
prevents hypothesis publication after the raw response is saved. This is a
headroom check, not a universal guarantee about server-side tokenization.

## Frozen saved-report replay

The [protocol](PROTOCOL.md) preceded inference. The baseline reuses the five
existing `metadata-independence-v1/revised` receipts and their frozen runner;
those model calls were not repeated. The revised observer-only runner and source
were frozen before one run per case. No prompt was tuned after viewing revised
outputs. No game, target inspection, memory edit, gameplay input or manually
reviewed image was needed. The blocked desktop executable was not rebuilt,
restored or launched.

| Retained report | Baseline accepted/rejected | Revised accepted/rejected |
|---|---:|---:|
| Latest gameplay | 10/1 | 7/1 |
| Earlier gameplay | 10/1 | 8/0 |
| Menu | 0/2 | 2/2 |
| Owned context fixture | 0/0 | 0/0 |
| Opaque-label research variant | 0/0 | 0/0 |

The fixture now selects type 2, containing three observed paths with the label
`Speed`, and sends those exact rows to field review. Previously it stopped at
type selection. The field-review model still returns no questions. This fixes
the evidence-routing gap but does not produce a named or verified control.

Gameplay question counts fall, and the menu produces `maxJoysticksPerPlayer`
and `force4WayHats` questions despite the instruction to exclude device
preferences. The added context is therefore not a consistent semantic-quality
improvement. These remain internal hypotheses, not promoted card names or edit
authority. Counts measure coverage and reference validation, not correctness.
The prompt, object context and context-window size changed together, so this
comparison cannot isolate their individual effects.

The two gameplay indexes include 74 and 64 text examples, omitting 592 and 684.
The menu includes 95 and omits 1,221. All three fixture examples fit; the opaque
variant has no eligible text. Largest selection input is 36,266 reported tokens;
all revised requests have recorded headroom. Every original field label remains
in the index. The [artifact audit](results.json) checks these budgets, source
paths and text, frozen hashes, model identities, receipts and five runner exits.

The 179 core/observer tests, strict workspace Clippy, formatting and the offline
release build pass. Regression tests cover tight-budget fairness, exact path
binding, refusal to borrow another object's label and missing/near-limit token
accounting. Progress stays **35%**: these are already explored reports and an
owned labelled fixture, with no new causal or independent-game evidence.

Desktop preview `0.1.0-20261021-225832` remains unchanged while its Windows
security detection is unresolved. Next: improve useful field-question selection
and connect ambiguous candidates to observable test conditions, retaining the
current abstentions and unwanted device-setting suggestions as regression cases.

## Agent API alignment (2026-10-22)

The shared `read_runtime_inventory` command now exposes the same kind of bounded
object-local clues before an MCP agent selects a type. Existing full type names
and field descriptors are preserved. Optional `object_samples` retain their exact
field name/path/text binding; `context_coverage` records available and omitted
samples. If the bounded context index is too large, full field descriptors still
return with an explicit context error and unknown counts. This prevents a context
budget failure from erasing the agent's metadata inventory.

Two added regression checks cover path binding and descriptor preservation, plus
the oversized-index fallback. All 68 app tests and strict app/MCP Clippy pass.
`scripts/mcp-catalog-smoke.py` verifies the actual stdio handshake and 53-tool
catalogue, including the inventory tool's description and read-only annotation,
without connecting an engine or target. Its process exits successfully. Evidence:
`test-results/agent-context-tests.log`, `agent-context-clippy.log` and
`agent-context-mcp-v1/result.json`. This is source/API validation; the current
desktop engine has not been replaced while Windows detection review is pending.
