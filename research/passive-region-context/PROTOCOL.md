# Passive region evidence in automatic activity interpretation — fixed study

Question: can the source-selected regional measurements reach the existing
activity interpreter and durable shared context, with geometry, failure status
and uncertainty intact? Compare interpretations with/without these additional
measurements. The two recordings are known development cases, not unseen tests.
Model prose and flags are observations, not ground-truth accuracy or new controls.

Use exact case manifests and five PNGs per case from
`test-results/stable-region-selection-v1/case-{0,1}`. Retain the prior
`stable_texture-0.json` output for complete proposal equality. Use existing
`test-results/activity-horizon-v1/case-{i}-augmented.json` appearance summaries in
both model arms so the only added interpretation context is the new `regions`
summary. Bind source identities and image bytes, not archive filenames. Keep
every failure. No manual screenshot review, input, game launch or target access.

New summary policy `selected_passive_regions_v1` contains at most four candidates
and 6 KiB. It retains the root self-check, preceding quiet outcome and both exact
action outcomes as independent correspondences. Requested origin regions remain
distinct from normalized/matched rectangles. Before/after rectangles and their
signed differences use original image pixels. Failed checks retain their reason
without displacement or rectangles; later intervals after a stop are not_measured.
No root failure is hidden as zero motion. The full source, frame metadata,
proposal report, origin/quiet frame references and quiet input hash are bound.

The underlying v2 region selection and matching are unchanged. Freeze all
Rust/Cargo sources, this protocol and runner, exact original inputs, source
manifests and model digest before building/replaying. Use the product summary
conversion path through the file-only region example. Require the complete
proposal output (excluding the newly requested summary keys) to equal its prior
archive. Summarize twice and require complete equality. Freeze the four generated
interpretation manifests and summaries before starting any interpretation call.

Run exactly four local `activity-replay` calls, baseline then augmented for each
case. Both arms include identical source, four image bytes, global patch evidence
and prior appearance summary. The augmented arm also supplies the new regions.
Use `qwen3.8:latest`, digest
`22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643`, checked
before every call and at the end. Keep existing options temperature 0, seed 42,
num_ctx 8192 and num_predict 600. There are no retries, object-detection calls,
metadata-hypothesis calls or prompt adjustments after seeing responses.

Retain complete prompts, replies, errors, schema/options, receipts and process
outcomes. Record prompt bytes, summary bytes, interpretation flag, description,
uncertainty, elapsed time and whether the exact optional summary survived
Receipt.context(). A false/uncertain interpretation must not become true merely
because regional matching succeeded. Existing comparison eligibility, names,
cards, write authority and assessment gates are unchanged. Absent regions must
keep legacy serialized shape and the old prompt unchanged.

Before replay, automated checks cover source/geometry/stop integrity, original-
pixel scaling, explicit failed/missing outcomes, serialization and uncertainty,
legacy records, and automatic preparation from a no-target-access session with
no observer labels. The unchanged proposal equality check covers natural image
measurements; interpretation output has no semantic correctness oracle. Any
source correction after freezing requires a new study version with original
results retained. Audit original/frozen/generated files and both executable
hashes after completion. The quarantined desktop is not replaced or launched.
