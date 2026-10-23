# Citation format matches the engine contract

Automatic naming and the MCP submission tool now share a citation schema with
two disjoint cases: field/type labels require `index: null`; object text requires
a nonnegative numeric index. The research relevance reviewer uses the same
helper with its existing quote-length policy. Exact source text, candidate/path
bindings and lexical checks remain enforced by the engine after generation.

The previous generation format admitted combinations the engine rejects. This
change closes that structural gap. It does not infer a missing index, repair a
wrong citation, verify meaning or grant edit authority.

## Frozen checks and fresh outputs

The [protocol](PROTOCOL.md) preceded the runs. The revised executables and 237
source/configuration files, schemas, fixed inputs and local model identity were
frozen. An independent JSON Schema validator passes 60 source/index/quote/extra
field checks. It preserves **232 valid relevance citations and 32 naming
citations** from the retained experiments and rejects the six formerly allowed
object-text citations with null indexes. The Rust regression also confirms that
switching or inferring a citation's index mode remains invalid.

Four fixed cases were then run once under the revised schema, preserving all
baselines and failures:

| Case | Fresh outcome | Model calls |
|---|---|---:|
| Previously failed synthetic relevance batch | Still rejected for six wrong source bindings | 1 |
| Saved Vampire relevance candidates | Four provisional gameplay classifications | 1 |
| Owned fixture full naming | One existing object-text fallback question, no rejected questions | 2 |
| Saved Vampire full naming | Four accepted questions, one lexical rejection | 25 |

The difficult relevance batch no longer emits the forbidden null object index.
Instead, it labels six nearby object descriptions as `field_name` citations.
Those are schema-valid but do not equal the field names, so the unchanged engine
correctly rejects the assessment. The structural repair therefore **does not
resolve the full attribution failure**. The raw result is retained and was not
retried or corrected. A useful next approach is explicit references to supplied
evidence entries, rather than asking the model to reproduce both a quote and its
source metadata correctly. Such references would still need exact binding checks.

Full Vampire naming keeps the same four names (`maxHp`, `moveSpeed`, `starting
spawns`, `interval`) and the same rejection of “destructible frequency” as the
baseline. The fixture's `Speed` question comes from the already existing fallback;
it is not evidence that this schema adds semantic understanding. All meanings,
ownership, live effect and numeric bounds remain unverified. The relevance
classifier stays research-only and cannot hide production candidates.

## Packaged verification

All **504 ordinary workspace tests**, strict all-target/all-feature Clippy,
formatting and release builds pass. One opt-in archive replay remains excluded
from the ordinary workspace run. Preview `0.1.0-20261023-085244` passes headless
startup/dispatch and the 65-tool MCP catalogue check. The actual packaged
`reversebolt_submit_runtime_hypotheses` schema exactly matches the shared core
schema, including the source/index alternatives.

Rebuilding the shared core changed the native helper's binary hash. The exact
packaged helper therefore ran the existing seven-case owned ABI matrix: 51 calls,
90 recorded pauses with zero helper allocations/deallocations while paused, four
restored fixture writes and four readonly refusals. Typed routes, values and root
fingerprints remain unchanged; the header/layout budget boundaries still behave
as expected. This is mock ABI evidence, not real-game coverage or remote unload
proof. No game, gameplay input or screenshot was involved.

The [audit](audit.py) checks frozen artifacts, historical citation bytes, fresh
schemas/responses, source inventories, candidate bindings, model identities and
token headroom, packaged interfaces and the native matrix. [Results](results.json)
include the unresolved misattributions. Run
`python -X utf8 research/citation-contract/audit.py`; `--verify-current` also
compares current sources with the freeze. Full local artifacts are in
`test-results/citation-contract-v1`, with separate `citation-contract-headless-v1`,
`citation-contract-catalog-v1` and `citation-contract-native-v1` package checks.

The desktop shortcut selects this preview for the next launch. The user's existing
editor remains open. Progress stays **38%**; this improves evidence-format
reliability without establishing new gameplay effects or generalization.
