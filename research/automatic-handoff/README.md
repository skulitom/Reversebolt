# Automatic association of response evidence and metadata questions

Implemented 2026-10-22. Full-goal progress remains **35%**. The shared input
experiment now carries its result into existing metadata questions without an
additional model pass or a manual hypothesis resubmission.

## Behavior

If completed, nonempty metadata questions exist when an input job is submitted,
the immutable plan pins their catalogue ID, revision, report hash and complete
review hash. The final response-review step checks these bindings and the original
report, then associates the new retained image context with the existing questions.
Names, alternatives, citations, questions and original question author remain
unchanged. No per-game rules, source layouts or key meanings are introduced.

The association creates a new review ID. `question_origin` preserves the original
question review and author; `play_context.interpretation_source` preserves the
independent image producer. Both attributions follow a comparison's immutable
test origin. A later agent can replace questions normally without inheriting a
claim that the old author produced them.

Positive and negative image interpretations are retained as given. If image
interpretation failed or is unavailable, existing questions remain valid but the
new review has no activity context. Changed metadata or cancellation leaves the
current metadata review unchanged. Missing eligible questions skip association.
The handoff does not generate new questions, refresh a value, call a model, inspect
a runtime, send input or write memory. It does not establish that a candidate
controls the observed response. Selected/refreshed/edited control preservation
uses the existing shared publication path.

An immutable handoff intent prevents a second derived review after interrupted
publication. The result is saved before publishing the staged review; the shared
checkpoint retains the input summary and metadata review together. Completed
request replay returns the archived result and does not republish. A hash in the
input summary binds the handoff archive, which cites the original input-result
hash. A corrupted handoff is reported separately and does not invalidate a
separately valid image interpretation. Restart still drops the old live binding.

## Verification

All **403 workspace tests**, strict workspace/all-target Clippy and formatting
pass. The initial targeted integration also passed before the checkpoint and
source-mutation assertions were added to the full-suite run.

The generated integration test runs the same ten-capture experiment and final
review code with existing synthetic metadata. It covers positive and negative
interpretations, unavailable image inference, cancellation and changed metadata.
The image callback runs once; association makes zero additional model calls.

The successful cases reach the real `plan_runtime_comparisons` command without
agent hypothesis resubmission, obtaining four observed actions and a next proposed
comparison. The original question batch and author are unchanged, a selected
display value remains `999` rather than being refreshed from the old report's
value `8`, and the candidate has no edit authority. There are eleven retained
images including the initial source and zero memory samples. The fixture panics
on target-memory reads. Negative image interpretation remains negative.

The tests verify source-question changes without a revision change, changed report
binding, preserved question origins through serialization, exact replay, refusal
to repeat the handoff intent, and the production wrapper's durable checkpoint.
A damaged association archive produces a handoff error while valid original image
evidence remains usable. Legacy question origins default to absent.

[Verification counts, logs and source hashes](results.json) record the local
checks. These are generated images, synthetic metadata and mock interpretations;
no game, native input worker, live model or production app was run.

## Limits

This closes an orchestration gap, not a causal-discovery gap. The system still
needs useful experiments on real games, automatic choice between investigation
stages, and evidence that a proposed internal value actually controls the observed
behavior. No semantic or generalization credit is added. All artifacts stay local;
the quarantined desktop preview is not replaced or launched.
