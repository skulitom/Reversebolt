# Saved-evidence comparison prioritization

Use the initial Viewfinder live plan saved before any trial. This is a known
development case, not held-out evidence. Freeze current Rust sources, evaluator,
original chosen-plan document and this protocol before inference. Do not change
the prompt, model or validation after observing results. Use qwen3.8:latest and
the existing local model service; no new images, game launch, target reads,
input or field writes. Preserve all responses and failures, without retry.

Run four predeclared cases in order: original; reversed candidate order (with
new indices); opaque field/type labels and removed object text; a synthetic
brightness-only activity description with motion/appearance evidence removed.
The last two are synthetic robustness checks and must not be described as actual
game observations. Every model order must cite supplied candidate indices; it
cannot change a probe value, input sequence or restoration policy. Empty order
is a valid abstention and leaves the original ordering intact.

Report whether the original and reversed inputs prioritize the candidate whose
previously recorded trial had a positive response, and whether the synthetic
cases abstain. Do not interpret this small development experiment as measured
generalization, causal validation, proof of safe ranges or a new gameplay test.
Full prompt/response/model identity and source hashes remain local. Integrate
only with exact input binding, explicit optional failure and preserved fallback
choices; existing edit guards remain authoritative.

The first frozen run abstained in all four cases. For a separately frozen v2,
include measured action/quiet image differences in the prompt, retain concise
motion uncertainty, and clarify that an experiment investigates an unproven
mechanism. Repeat the same four cases and preserve v1 unchanged. This is a
development correction informed by the failed replay, not an independent test.
