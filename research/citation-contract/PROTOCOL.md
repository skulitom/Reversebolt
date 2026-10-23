# Citation generation contract repair

The retained relevance challenge produced six object-text citations with null
indexes. The engine correctly rejected them, but the generation schema allowed
the source/index combination. Preserve that complete failure and all earlier
results. Do not infer or repair an index after generation.

Express the already enforced relation as two disjoint schema branches: field/type
labels require a null index; object text requires a nonnegative integer index.
Keep exact candidate/path/quote validation, lexical grounding, budgets and semantic
prompts unchanged. Reuse the same helper for automatic hypotheses, the MCP
submission schema and research relevance review. Preserve each caller's existing
quote-length bound. This change prevents a structural error; it does not prove
relevance, meaning, ownership, liveness, safe values or effects.

Freeze the revised sources/executables, protocol/scripts, source inputs, exported
schemas and local model before inference. First use an independent JSON Schema
validator on saved citations and a fixed source/index/missing/extra-field matrix.
Confirm that the six formerly allowed malformed citations are now rejected,
without rejecting historically valid citations. The Rust validator must still
reject switched citation modes even when quote text would otherwise match.

Then run four fixed development cases once each: relevance review of the failed
synthetic batch; relevance review of the saved Vampire candidates; full automatic
naming of the saved owned fixture; and full naming of the saved Vampire report.
These are new runs under a changed schema, not replacements for the failed
baseline. Reuse baseline receipts without repeating them. Keep every error,
rejection, abstention and semantic difference. No prompt or schema tuning after
seeing fresh outputs. Compare structure separately from proposed meanings.

Audit hashes, exact source inventories, prompts, model identities, schema-valid
responses, candidate references and token headroom. Recheck research assessments
with the frozen binary, without inference. Check the actual packaged MCP tool
schema against the exported shared schema, plus headless startup/dispatch before
activating an updated preview. Preserve the user's current editor. No game,
input, screenshot or process-memory operation is needed for these replays.

The relevance classifier remains research-only and cannot hide candidates; its
known order-sensitive false exclusion is unaffected by a structural schema fix.
No causal gameplay or independent-generalization credit follows from these tests.
