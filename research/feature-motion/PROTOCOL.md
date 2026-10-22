# Distributed feature correspondence — fixed evaluation

Prior grid measurements miss texture and motion beyond ten normalized pixels.
Evaluate an original Rust replacement using source-selected corners and a
two-level bounded block search. Do not change gameplay semantics or control gates.

## Method fixed before retained-image results

Use bounded RGB601 grayscale PNG decoding and the same 256×144 triangle resize.
Build a 128×72 triangle pyramid level. On a two-pixel source grid with a twelve
pixel border, calculate the smaller eigenvalue of the average 9×9 gradient
structure matrix (central differences divided by two). Require eigenvalue >=16.
Sort strongest first with deterministic coordinate ties. Select at most two per
cell of a 6×4 image grid, separated by at least twelve pixels (Euclidean distance)
globally; maximum 48 patches. Selection must use the before image only.

For each selected point search a 9×9 zero-mean squared-difference patch at half
resolution within +/-16 pixels, then refine at full resolution within +/-3 of
twice that offset. Report displacement in full normalized integer pixels.
At both levels require patch standard deviation >=8, residual RMS <=12, and a
distinct-basin runner-up MSE >=1.25 times the best plus 9. Distinct basins differ
by more than one pixel on either axis. Reject best matches at the search boundary
or clipped image-patch boundary. Reverse the complete search independently from
the matched point and require return error <=1 full-resolution pixel per axis.

Expose all selected points, matched/unresolved counts, failure reasons, selection
capacity and occupied cells. Unresolved displacement is null, not zero. Consensus
must have >=10 supporters and >=75% of successful matches within one pixel per
axis, and supporters in >=6 grid cells spanning >=2 rows and >=2 columns. Never
average opposing motions into zero. Corner score and all thresholds are heuristic.
No correspondence identifies camera, actor, physical velocity or input causality.

## Tests and replay

Synthetic tests: signed small and larger translations with additive brightness,
texture spread and deterministic selection, flat/one-dimensional/repeated texture,
unrelated images, occlusion, opposing motion, out-of-range motion, source geometry
and input limits. Check recovered vectors against the known transform, not merely
their existence. A localized patch cluster must not become spatial consensus.

After these pass, freeze source/protocol/input hashes and the built executable.
Use the same 48 retained images and 43 pairs from patch-motion-v3 with corrected
source names: Hyperbolica, the owned striped fixture, and Manifold Garden. Compare
against the frozen grid output, including matched/selected counts and fractions,
spatial support, exclusions and displacement summaries. Preserve every old image
difference exactly. Do not tune from those results. No real-flow ground truth is
available; higher match counts alone do not prove correctness or better semantics.

Also measure known translations of each game's first retained grayscale image:
[0,0], [3,-2], [18,8], [-24,-10], with additive brightness +20 (saturating). Record
every matched vector's error and exclusions; image boundaries and saturation can
reduce matches. This is a controlled image transform, not observed gameplay.
Require zero accepted errors exceeding one pixel on either axis to call these
transform checks passing. Report low coverage even if that error criterion passes.

Archive all failed attempts unchanged. Any code correction before a repeated run
needs a new output directory and an explanation. Use the product measurement path
for the ordinary 43 pairs. No screenshots, games, models, input workers, target
access or gameplay writes run. No native or generalization credit is planned.

## Research basis

[Shi and Tomasi (1994)](https://cecas.clemson.edu/~stb/klt/shi-tomasi-good-features-cvpr1994.pdf)
describe gradient-based feature selection; [Bouguet](https://graphics.stanford.edu/courses/cs448a-00-fall/bouget00.pdf)
describes a pyramid for handling larger displacements; the earlier
[patch-motion protocol](../patch-motion/PROTOCOL.md) cites forward/backward failure
checks. This is original integer block matching, not Lucas–Kanade or Median Flow.
No third-party code, models or weights are copied.
