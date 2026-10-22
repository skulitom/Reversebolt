# Stable source texture for passive region selection — fixed protocol

The previous automatic selector spent most of its four-candidate budget on weak
source texture or unresolved appearances. Test a replacement ranking on exactly
the same two saved passive recordings, preserving the previous method as a
paired comparison. Both cases are now known development evidence, not an unseen
generalization test. Do not adjust selection after viewing the new outcomes.

Use the exact replay manifests and five-image folders in
`test-results/activity-region-suggestions-v1/case-{0,1}`. Case 0 is Hyperbolica;
case 1 is Manifold Garden. Preserve all frame references, source selections,
sampled input metadata and PNGs. Retain each prior `result-0.json` for complete
JSON equality against the old policy. No visual observer boxes, names or model
output enter ranking. No screenshot review, fresh capture, model call, gameplay
input, target access or memory operation runs.

New policy: `passive_activity_stable_texture_regions_v2`. Reuse the previous
normalized 512x288 image path and 16/32/64-pixel tiles at half-size strides.
First-action change minus quiet change, clamped per pixel, must still have mean
>=8. Outward mapping to original pixels and minimum valid rectangle size are
unchanged. Only the ranking after that gate changes:

1. Map each candidate's original rectangle back to the exact normalized matcher
   rectangle. Reject if either its existing sparse or dense template standard
   deviation is below the existing matcher threshold of 8.
2. At that rectangle's center, reuse `patch_motion::corner_score`: the smaller
   eigenvalue of the mean gradient covariance matrix in a 9x9 window, with
   central differences divided by two. Reject below the existing threshold 16.
3. Measure mean absolute quiet-to-root difference over the entire actual matcher
   rectangle. Reject above 4. These rejection counts are exclusive in this order.
4. Rank by `corner_score / (1 + quiet_mean_absolute_difference)` descending, then
   action-minus-quiet change descending, then original grid tile side/y/x
   ascending. Apply the same greedy quarter-smaller-tile overlap suppression.

The minimum-eigenvalue texture concept is described in the primary
[OpenCV feature documentation](https://docs.opencv.org/4.12.0/dd/d1a/group__imgproc__feature.html).
This implementation reuses Reversebolt's original Rust scorer and template
statistics; no new dependency or upstream source is incorporated. Quality scores
and thresholds are heuristics, not probabilities or semantic labels.

The four-candidate/twenty-check budget, five PNG reads, immutable template,
matching thresholds, stop-at-first-unresolved behavior and recommendation rule
stay unchanged. No failed candidate is replaced. Matching the later activity
interval cannot rerank or alter selection. Keep the old serialized v1 report
exact when `change_only` is explicitly selected; default shared API/MCP behavior
uses v2 with additional selection counts and per-candidate quality diagnostics.

Software checks before freezing include known local translations with a larger
untextured flashing distractor, opposite same-key motion, later-pair image changes
that cannot rerank candidates, quiet flicker, flat/one-direction texture, exact
rounded geometry, existing image/source/input errors, stop behavior, deterministic
repeats and the application default/opt-out/no-target-access checks.

Freeze all Rust/Cargo sources, protocol and runner plus the exact archived case,
PNG and baseline-result inputs before building or replay. Run both methods twice
per case with the same file-only executable. Require complete v1 output equality
to the previous archived result, complete within-method repeat equality, exactly
five image IDs read, at most four candidates/twenty checks and an unchanged audit.
Report every rejection category, ranked/candidate count, match status, interval
delta, recommendation and elapsed time. Retain failures and abstentions; do not
substitute an image/model method after results. Source corrections require a new
study version with the original run retained.

Any positive result is a heuristic appearance correspondence in independently
checked intervals. It does not establish continuous identity between intervals,
game input delivery, camera/actor separation, world speed, gameplay meaning,
causal field influence, edit authority or a trial baseline. Higher match counts
are not accuracy without ground truth. Do not publish a desktop replacement for
the quarantined application or award generalization credit from this replay.
