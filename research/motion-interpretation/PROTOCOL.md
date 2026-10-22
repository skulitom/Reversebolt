# Measured correspondence in automatic image interpretation

The motion reader is available to agents and archived during automatic discovery,
but the image interpreter currently sees only four images. Add a bounded numeric
summary of the exact two pairs. Keep metadata type/field discovery independent.
Do not infer controls or change comparison eligibility from motion statistics.

## Implementation contract

Use the existing v2 product matcher on the observer's validated image bytes.
Summaries retain metric/version, source metadata digest, pair image IDs/hashes,
selected and matched counts, normalized dimensions, matched cells, exact matched
anchors/displacements, and rejection counts. No actor name, game name, internal
field, key meaning or physical velocity is supplied. Missing matches remain
missing. Bind the summary to the activity source and retain it with the model
receipt, shared context, handoffs and comparison origin. Existing contexts without
this optional evidence remain readable and retain their original interpretation.

The prompt must identify numeric matches as fallible observations, explain units
and partial coverage, retain uncertainty and require the model to inspect both
image pairs. Raw correspondence cannot establish identity, causality or semantics.
Keep the existing output schema and model sampling options. No automatic positive
override is permitted. Validate bounded summaries before retaining context.

## Fixed paired evaluation

Freeze the current image-only observer and sources before changing its prompt or
dependencies. Run each of four cases once through that binary and once through
the revised binary, with the same installed qwen3.8:latest model digest and four
images. Archive full prompts, options, responses, timings, hashes and failures.
Do not retry merely because interpretation is negative or uncertain; do not tune
the prompt after viewing results. A timeout does not prove the server stopped:
inspect the specific process/request before continuing or restarting.

Two cases use the already-selected activity source from retained Hyperbolica
and genuine Manifold Garden discovery. Their semantic truth is unknown. Report
interpretation changes and coverage, not an accuracy score. Keep all original
records and interpretations unchanged.

Two controlled cases derive from the normalized first retained Hyperbolica image:
both pairs shift right by 18 pixels, or the second pair instead shifts left by 18.
The source/timing/key annotations are explicitly synthetic test metadata; no game
input occurred. Compute image-difference values from those generated pairs.
The narrow expected consistency answers are true and false respectively. Inspect
whether the description remains within image-space evidence; any positive answer
is still not proof of a physical effect. These two controls do not establish a
general model accuracy rate. Retain negative or erroneous outcomes.

This requires eight image model calls and no metadata calls, game launches,
screenshots, manual image review, target access, input or writes. If the server is
unavailable, preserve the prepared cases and report that inference is unverified.
The desktop security detection is not bypassed or resolved by this evaluation.
No independent-game or verified-control credit is planned.
