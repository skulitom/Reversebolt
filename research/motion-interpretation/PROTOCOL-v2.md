# Correct the controlled-case label leak

V1 generated image IDs containing `consistent` and `opposed`. The revised prompt
included those IDs with its hash bindings. The old image-only prompt did not.
This contaminates the paired controls; their answers cannot demonstrate that
numeric evidence improved interpretation. Preserve all V1 inputs and outputs.
The two real-game cases use non-revealing IDs and remain descriptive results.

Use the same first Hyperbolica image and identical generated pixel content. Change
only synthetic source/clock/image IDs so neither expected answer is exposed. The
generator's case names remain outside model input. Rebuild/archive only the
control generator; use the exact already-frozen baseline and revised observer
executables and unchanged prompts, options and model digest. Check equality of
all generated image hashes with V1 before inference.

In a new `motion-interpretation-v2` directory, run the two controlled cases once
through each frozen observer (four additional image calls). Do not repeat the
real-game cases. Audit the prompt for the removed labels, count these calls
separately, and preserve all results regardless of the answer. This is an input
provenance correction, not a prompt/algorithm adjustment or a fresh held-out test.
