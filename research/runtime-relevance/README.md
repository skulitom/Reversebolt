# Blinded raw-label relevance review

The subsequent [32-case challenge](CHALLENGE.md) found an order-sensitive
misclassification of aim sensitivity and a malformed-citation batch. It supports
keeping this review advisory rather than using categories to hide candidates.

The saved menu case continues to propose device preferences even though naming
instructions exclude them. Its names pass the exact-label contract, demonstrating
that lexical grounding is insufficient to establish useful gameplay meaning.

A separate research reviewer now sees each candidate's complete type/field labels
and same-object text, with a local index. It cannot see the generator's proposed
name, explanation or question, the numeric value, opaque path identifier or image
interpretation. Exact source paths, values and report identity still bind the
saved input digest. The reviewer classifies every supplied candidate and cites
its own exact evidence; it cannot borrow text from another object or omit a row.

Categories are plausible gameplay, application configuration, implementation
detail and unclear. Hidden/inactive fields and configuration assets can still be
gameplay. Unclear is retained. This assessment is provisional and does not prove
the field's owner, liveness, meaning, range or effect. It neither removes a
candidate nor authorizes an edit.

## Frozen experiment

The [protocol](PROTOCOL.md) preceded inference. The source (236 files), executable,
model identity and five existing naming inputs/receipts were frozen. Eight cases
ran once each, with seven local model requests; the empty original opaque case
made no inference request. No naming calls were repeated, and no game, input,
memory access or screenshot was needed.

| Saved candidate batch | Gameplay | App configuration | Implementation | Unclear |
|---|---:|---:|---:|---:|
| Vampire original | 4 | 0 | 0 | 0 |
| Earlier gameplay original | 9 | 0 | 0 | 0 |
| Menu original | 0 | 2 | 0 | 0 |
| Owned context fixture | 0 | 0 | 0 | 1 |
| Original opaque case | 0 | 0 | 0 | 0 |
| Earlier gameplay reversed | 9 | 0 | 0 | 0 |
| Menu reversed | 0 | 2 | 0 | 0 |
| Vampire with labels removed | 0 | 0 | 0 | 4 |

All requests complete and pass the evidence audit. Both reordered cases retain
the same category for every exact source path. The menu's `maxJoysticksPerPlayer`
and `force4WayHats` are classified as device configuration. The fixture's generic
`Coefficient` with nearby `Speed` remains unclear. Removing meaningful labels
and object text from the four Vampire candidates produces four unclear results.

These are development diagnostics, not a precision score. The same model produced
the earlier hypotheses and these new classifications. Agreement is not independent
truth, and all thirteen original gameplay classifications remain unverified. For
example, the display-fill fields are called plausible gameplay based on labels;
the review has not established whether they affect the world, UI or something
else. The fixture reason also ends abruptly within its text budget; that wording
is retained unchanged. No case exercises a real implementation-detail exclusion.

The result supports further investigation, not an automatic production filter.
Next evaluate false exclusions using additional evidence for configuration-based
game rules, in-world cameras, animation/appearance and ambiguous plumbing, plus
adversarial text. Do not add a blacklist for the two known device fields or call
their exclusion a general solution.

## Verification and delivery

The [artifact audit](audit.py) checks the frozen binary/source and all input hashes,
exact candidate transformations, blinded prompt contents, every citation, response
and classification, model identity and token headroom. Each accepted receipt is
also rechecked by the frozen Rust executable without inference. [Results](results.json)
retain every reason and citation. Full local artifacts are in
`test-results/runtime-relevance-v1`. Run
`python -X utf8 research/runtime-relevance/audit.py`; `--verify-current` additionally
checks the working sources against the frozen snapshot.

All **149 core/observer tests**, strict all-target/all-feature workspace Clippy,
formatting and the research release build pass. Six added tests cover exact
coverage, cross-object/sub-string citation refusal, hidden-value binding, duplicate
paths, no extra authority, receipt tampering, incomplete output and token budgets.
An oversized prompt fails without truncating evidence.

The implementation is behind the `runtime-relevance-research` feature, verified
absent from default desktop and MCP dependencies. Desktop preview
`0.1.0-20261023-081315` and the user's current editor stay unchanged. No product
candidate is filtered, no inference is added to normal connection, and no new
gameplay control is claimed. Progress remains **38%**.
