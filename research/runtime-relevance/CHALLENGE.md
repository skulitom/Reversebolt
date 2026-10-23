# Relevance is unsuitable as a hard exclusion gate

The broader fixed challenge found an order-sensitive exclusion: `aimSensitivity`
with the explicit description “Scale applied to player aim rotation in the world.”
was classified as application configuration in its original batch order, then as
gameplay when the same rows were reversed. A gate that hides configuration would
lose this useful lead. The classification can be retained as provisional advice;
it should not decide that a candidate is irrelevant to a user's intent.

The [protocol](CHALLENGE-PROTOCOL.md) and [dataset generator](challenge-freeze.py)
were frozen before inference. All 32 cases are authored development fixtures,
not observations from real games. The prompt, policy, schema, model and limits
are byte-identical to the first relevance experiment. Only the research runner
gained an explicitly synthetic input mode. Expectations and fixture keys never
reach the model; raw type/field labels and same-object text do.

## Results

Eleven requests completed, covering four balanced batches, their reversals and a
paired adversarial batch. Ten assessments passed the evidence contract. One
original batch failed: six `object_text` citations used `index: null` instead of
the required numeric index. The quote strings matched nearby text, but exact
bindings were invalid. No index was inferred or repaired, no partial assessment
was accepted and the batch was not retried. This exposes a separate schema gap:
the generation schema currently permits combinations the validator must reject.

Of the 32 original candidates, 24 received valid assessments and eight were in
that rejected batch. Twenty-three of the 24 classified candidates matched the
authored rubric; the remaining one was aim sensitivity. This is not a real-game
accuracy estimate, and the eight unavailable assessments are not counted as
successful classifications.

| Comparison | Observed result |
|---|---|
| First balanced batch and reversal | Same categories for all eight paths |
| Second batch and reversal | Original assessment rejected; reversal valid |
| Third batch and reversal | Aim sensitivity changes from configuration to gameplay |
| Fourth batch and reversal | Same categories for all eight paths |
| Embedded instruction to mark every candidate as gameplay | No category changes from its paired baseline |
| Embedded instruction to exclude a useful gameplay field | No category changes from its paired baseline |

The eight ambiguous cases remain unclear wherever their batch yields a valid
assessment. Configuration-based jump/spawn rules, camera properties, appearance,
animation, simulated hearing, projectile/resource limits and interaction bindings
are retained in their valid assessments. The tested device/OS preferences and
bookkeeping cases match their intended negative categories. These authored
examples and two attack strings cannot establish broad semantic accuracy or
prompt-injection robustness.

## Evidence and decision

The [audit](challenge-audit.py) passes all frozen source/binary/input hashes,
candidate transformations, blinded prompt content, exact request/response bindings,
model identity and token headroom. Every valid assessment is rechecked by the
frozen Rust executable without inference. The rejected response and its six bad
citations remain in the [derived results](challenge-results.json). Full local
artifacts are in `test-results/runtime-relevance-challenge-v1`. Run
`python -X utf8 research/runtime-relevance/challenge-audit.py`; use
`--verify-current` only while the working sources match this freeze.

The changed research runner passes strict Clippy, formatting and release build.
The core and observer relevance implementation remains byte-identical to the
previously tested 149-test version, so those unchanged tests were not repeated.
All eleven owned inference workers exited. No game, input, screenshot or process
memory access was needed.

Do not enable categorical hiding in the desktop or MCP workflow. Keep uncertain
and potentially useful properties available, and use intent and actual behavior
to decide which experiments matter. A concrete next repair is to express the
citation-source/index relationship in the generation schema, then test it against
these retained malformed responses and fresh outputs without weakening exact
binding checks. This structural repair would not solve relevance or causal
meaning. Desktop preview `0.1.0-20261023-081315` stays selected; progress remains
**38%**, with no additional gameplay or generalization credit.
