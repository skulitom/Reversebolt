# Literal questions from complete object text

When the metadata model returns a valid empty review, Reversebolt can now form
an internal question from a complete literal clue on the same object. It asks
whether the value affects what the text refers to. It does not rename a control,
establish a gameplay effect, infer a numeric range, or grant permission to edit.
The ordinary direct-value UI remains separate.

The rule uses no game profile, source layout or preferred field-name dictionary.
A selected row needs exactly one distinct retained complete string that passes
the existing citation/name validator. Conflicting complete labels remain
ambiguous. Failed, cancelled, rejected and nonempty model reviews are preserved.
The original empty batch and raw model responses remain in the receipt.

Questions group only paths with the same declared type, field and exact clue;
every selected rival remains an alternative. Groups with more than five paths
are deferred whole. The twelve-question budget is shared across selected types.
`question_source: object_text` and `question_policy: complete-object-clue-v2`
follow a question into a comparison's origin, where evidence and policy are
rechecked. Old declared-question serialization remains unchanged.

## Retained failure and revision

[V1 protocol](PROTOCOL.md) excluded any row with incomplete surrounding text.
The fixture had a complete `Speed` clue plus an unrelated truncated string on
each of three paths. V1 therefore produced no question and **failed its positive
expectation**. Its frozen source, runner and receipts remain unchanged.

[V2 protocol](PROTOCOL-v2.md) allows a complete clue to seed a question while
explicitly counting incomplete context. It never claims that clue is a unique
property identity. V2 produced one literal question with both competing paths
and disclosed all three incomplete-context rows.

| Saved case | Original accepted/rejected | V1 | V2 |
| --- | --- | --- | --- |
| Latest gameplay report | 7 / 1 | unchanged | unchanged |
| Earlier gameplay report | 8 / 0 | unchanged | unchanged |
| Menu report | 2 / 2 | unchanged | unchanged |
| Labelled owned fixture | 0 / 0 | 0 / 0, failed expectation | 1 / 0, two alternatives |
| Opaque-label control | 0 / 0 | unchanged | unchanged |

Each version replayed the five retained responses once, with **zero new model
calls**, target access, game input, edits or manual image review. Process duration
is recorded separately from the original model timings retained in receipts.
This is a known development fixture, not evidence of semantic correctness or
held-out generalization. Existing unwanted menu suggestions remain a limitation.

All 292 workspace tests and strict all-target Clippy pass. Regression coverage
includes evidence mutation, cancellation/errors/rejections, conflicting or
generic clues, whole-group deferral, type fairness, policy provenance and legacy
serialization. [Results](results.json) distinguish artifact integrity from the
failed V1 expectation. The audit can be repeated without model or target access:

```powershell
python research/object-text-questions/audit.py
```

Raw receipts, binaries and source snapshots are local under
`test-results/object-text-questions-v1` and `test-results/object-text-questions-v2`.
This replay does not establish desktop deployment or live causal verification.
