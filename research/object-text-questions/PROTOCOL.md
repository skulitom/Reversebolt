# Questions from an object's own label

Implement a deterministic question fallback after a completed metadata review
that genuinely abstained: no accepted or rejected proposals, no error or
cancellation, and an exact selection rebuilt from the same inventory/report.
Do not override a nonempty review, a validation rejection or a failed model call.

An eligible row has one distinct complete object-text label within the existing
name/citation limits. It must pass the same lexical/reference validator as an
agent/model proposal. Multiple conflicting labels remain ambiguous. The source
path must have no truncated or over-budget text hint; an omitted second label
cannot establish uniqueness. Construct a
question about the relationship between this value and that literal label; do
not infer an effect, actor, units, limits, controlling state or gameplay relevance.
Do not use a game name, field dictionary, source layout or preferred label.

Group paths only when declared type, field and exact label agree. Keep every
selected path as a separate alternative, without claiming object identity or
merging values. Defer a group that exceeds the four-alternative budget whole.
Share up to twelve questions across types before extra groups from one type.
Record exclusions, omissions, exact paths and the deterministic source. Preserve
the model's original empty result in the receipt. This fallback creates internal
questions only, never card names or permission to write.

Freeze the implementation and observer-only runner before replaying all five
`object-label-selection-v1/revised` reports/receipts. Reuse those model responses;
make no model calls, game launches, input, writes or screenshot reviews. Latest,
earlier and menu nonempty reviews must remain unchanged. The owned labelled
fixture should produce one cited question with its competing paths retained.
The opaque case must remain empty. Record all failures without tuning to them.

Regression checks must cover ambiguous labels, generic placeholders, oversized
groups, fairness/order stability, altered evidence and cancellation/error/rejected
review refusal. Verify that source provenance survives into a test origin and
does not change the existing edit guards. These development cases do not prove
that a label is a gameplay property or that the proposed relationship is true.
No causal/generalization credit follows from constructing a question.
