# Automatic naming: metadata is a clue, not identification

Six exploratory runs used retained reports and the installed local
`qwen3.8:latest` model, digest
`22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643`,
through Ollama 0.40.7. No game was launched, controlled, captured or written during
this work. This is iterative research on already-inspected data, not a held-out
generalization test. The Python scripts preserve the tested versions, including
their inadequate acceptance checks; they are not the product naming policy.

Each run first selected up to eight types from all observed non-core types, then
requested names/explanations for actual observed fields. The only assembly
exclusion was the runtime report's core-library identity. Full requests, options,
raw responses, parse results, rejections, timings and hashes are retained under
`test-results/runtime-naming-v{1,2,3}`. Nearby text was not included in these model
experiments. JSON/schema compliance did not establish truthful explanations.

| Version and input | Structural acceptance | What failed or remained unresolved |
| --- | --- | --- |
| v1, Manifold gameplay | 11 accepted, 1 rejected | Walking-speed explanation referenced `moveSpeed` while citing `walkSpeed`; exact-reference validation rejected it. |
| v1, Blue Prince menu | 4 accepted | Included panel fade speed despite the requested exclusion of menu styling. |
| v2, Manifold gameplay | 12 accepted | Field-name references removed the mismatch, but the model selected `moveSpeed` and omitted the independently tested `walkSpeed`. |
| v3, Manifold gameplay | 2 accepted | Retained move/walk alternatives, but grouped a running speed, current state and input preference together; one explanation ended mid-sentence. |
| v3, Blue Prince menu | 8 accepted | Included UI/internal execution state, mixed different properties in groups and emitted unfinished text. |
| v3, opaque Manifold names | 8 accepted | Invented positions, scales and rates from numeric patterns despite an explicit instruction to abstain on opaque names. |

The v3 opaque run replaced type and field names with deterministic opaque tokens;
counts, values, types and assembly labels remained. It tested whether the method
would decline unsupported meanings. It did not. Descriptions starting with “May”
or “Likely” and exact existing field references were insufficient safeguards.
The schema's character ceiling also allowed unfinished text even though the
server reported normal completion. “Accepted” in these research files means only
that the script's structural checks passed, not that Reversebolt identified the
value. None of these proposals is published to the desktop board.

## Implemented shared evidence layer

`reversebolt_core::runtime_inventory` now builds an address-free inventory using
the existing runtime graph, type, byte/value and process-scope checks. A streaming
visitor reaches candidates before the display/page limit. Fields are grouped by
exact declaring type/layout and field index, never by equal names or values.
Each field retains up to three observed values, four path samples, total distinct
path count, and text tied to each sample's observed object. Truncation flags stay
with text. Multiple paths can alias; counts do not establish distinct actors.

The inventory is bound to the canonical report digest. Exact field resolution
checks this binding and returns bounded existing paths with their total count,
without constructing a textual search or granting write authority. Core-library
fields, unnamed array elements, characters, noncanonical booleans and nonfinite
numbers are excluded with counts. An 8,192-field and 8 MiB evidence budget fails
the whole build instead of returning silent first-page coverage. This inventory
does not interpret names, merge competing fields, establish numeric bounds, or
claim coverage of unobserved memory. Values are historical candidate readings.

The offline `reversebolt-runtime-inventory` executable consumes a saved report
and writes a new inventory. This is a reusable core API and offline evaluator;
automatic model scheduling, desktop presentation and a dedicated MCP inventory
action are not yet connected.

| Retained report | Types | Fields | Reachable paths before exclusions |
| --- | ---: | ---: | ---: |
| Manifold gameplay | 345 | 1,893 | 46,461 |
| Blue Prince menu | 261 | 1,243 | 27,052 |
| Owned context fixture | 3 | 5 | 48 |

The checked inventory retains both movement fields and the exact previously
tested walking path. The fixture retains `Coefficient` with the same-object
`Speed` text and a separate truncated string. These are evidence-preservation
checks, not new demonstrations of semantics or gameplay effects.

Workspace tests passed. Final focused checks cover report rebinding, separate
object text, competing fields beyond the card limit, failed/unreachable reads,
nonfinite values, core exclusions and evidence-budget refusal. Strict Clippy and
formatting checks passed. `audit.py` reproduces inventory outputs and checks the
six retained model runs without another model call. Evidence is under
`test-results/runtime-naming-v3/final` and `validation`.

Next, evaluate a bounded hypothesis contract with exact evidence citations and
competing candidates, then require independent observations or effect tests
before promoting a guess to an identified gameplay control. A second model's
agreement alone would not establish the meaning. Progress remains **32%**; the
full goal and the requirement for unchanged independent-game evaluation remain.

Follow-up (2026-10-21): a bounded, evidence-linked proposal contract and automatic
engine/MCP review are now implemented. The retained replays still contain irrelevant
settings and missed context, so proposals do not rename controls. See the
[integration, failures and current limits](../runtime-hypotheses/README.md).
