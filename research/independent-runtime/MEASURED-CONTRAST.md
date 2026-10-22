# Investigating an ambiguous rendered response

The frozen Hyperbolica run completed 15 observations and six metadata model calls.
It proposed walking speed and shake strength with alternative paths, but its
separate image interpretation set `consistent_visible_change: false`. The old
planner refused without considering the four bounded input episodes. No test or
field edit occurred. Preserve this native result unchanged.

For the next policy, distinguish a positive model interpretation from a measured
contrast that the model found ambiguous. The latter uses an explicit versioned
`basis: measured_contrast_v1`, retains the original false flag, description and
uncertainty, and must pass every existing hash, quiet-period, frame geometry,
focus/input and timing check. Four repeated, separated episodes are still needed
for a comparison plan. Errors, missing model responses, weak contrasts, missing
input, changed evidence and invalid legacy contexts remain ineligible.

This is a basis for asking a question, not for asserting a gameplay effect. The
metadata questions, model prompts, selection budgets, edit bounds, fresh target
checks and explicit input/write/restore operations remain unchanged. The context
and its basis must survive into test provenance and appear in the agent's plan.
Legacy positive interpretations keep their serialized shape. An unversioned
negative interpretation cannot silently acquire the new policy.

Freeze changed source and the offline replay executable before applying it once
to the saved Hyperbolica result. Rehash all fifteen original PNGs, recompute image
contrasts with the production metric, and rebuild the selected activity source
before planning. Reuse the exact model receipt and metadata questions; make no
new model calls or game actions. Expect the original ambiguity to remain false
and any resulting plan to retain the exact measured input timing and candidate
references. Retain failures; do not tune after this replay.

This already inspected case is a development regression, not new independent
acceptance. A proposed comparison is not a tested control. The original native
run and revised offline policy outcome must be reported separately.
