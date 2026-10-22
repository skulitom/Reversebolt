# Distributed image features and larger displacement

The v2 patch-motion measurement replaces a fixed grid with source-selected corners
and a two-level search. It produces more accepted correspondences on retained game
images, and passes controlled image-shift checks. **Actual gameplay displacement
accuracy remains unmeasured.** No new gameplay controls or semantic claims follow.

Selection uses texture in both directions, distributes at most 48 patches over a
6×4 grid, and keeps twelve normalized pixels between points. A coarse search
finds larger offsets; full-resolution refinement, residual/ambiguity checks and
an independent reverse search can reject them. The metric identifier changes to
`rgb601-triangle256x144-corners48-pyramid2-patch9-zmssd-r16x3-fb1-v2`.
The algorithm and thresholds were [fixed before replay](PROTOCOL.md); no adjustments
were made after the retained-image results.

The existing shared `read_play_evidence` and historical `read_runtime_test` readers
use this implementation. Automatic activity evidence archives retain it. Reports
expose selected/maximum counts, eligible source points, occupied/matched cells,
per-patch corner scores, displacement or null, and rejection reasons. A consensus
also needs support in at least six cells across two rows and two columns. This
prevents a localized group from claiming broad agreement. It still covers only
selected image features; texture scores are not confidence probabilities.

## Frozen replay

All 48 images, their identities and metadata are unchanged from the corrected
[grid evaluation](../patch-motion/README.md). There are 43 adjacent pairs:

| Source | Grid v1: matched / sampled | Features v2: matched / selected | Nonzero v2 matches |
| --- | ---: | ---: | ---: |
| Hyperbolica, 14 pairs | 51 / 294 | 236 / 348 | 43 |
| Owned striped fixture, 15 pairs | 0 / 315 | 0 / 0 | 0 |
| Manifold Garden, 14 pairs | 36 / 294 | 139 / 178 | 13 |

Denominators differ: v1 samples fixed locations; v2 selects usable texture. Higher
match counts do not by themselves establish accuracy. The one-dimensional striped
fixture supplies no eligible corners; that is missing evidence, not zero motion.

Each real-game sequence has four intervals containing sampled keys. Hyperbolica
has 3–7 accepted matches in those intervals and Manifold Garden 3–5. None supports
consensus. Each game's other ten intervals has near-zero consensus among the
selected patches. Those intervals contain no detected keys, which does not prove
that the game accepted no input, that the scene is stationary, or that no actor
moved. Local correspondence does not identify camera, actor or physical speed.
Actual game-pair motion has no ground-truth map and may include false matches.

All **43 existing image-difference values and their complete play-evidence
summaries are preserved exactly**. The replay completed in 3.062 seconds, compared
with the earlier grid run's 2.750 seconds. These individual runs are not a latency
benchmark. There were no new captures, image-model calls, target accesses, inputs
or gameplay writes, and no manual screenshot review.

## Known transforms and other checks

The first retained image from each game was normalized to grayscale, shifted by
[0,0], [3,-2], [18,8] or [-24,-10], and brightened by +20 with saturation. The
translation is known by construction; this checks correspondence on natural
image texture, not actual gameplay motion or camera projection.

| Source | Selected across four checks | Accepted | Exact known displacement | Errors over one pixel |
| --- | ---: | ---: | ---: | ---: |
| Hyperbolica | 104 | 82 | 82 | 0 |
| Manifold Garden | 52 | 46 | 46 | 0 |

The eight checks took 0.282 seconds. The acceptance criterion was zero accepted
errors over one pixel per axis; all 128 accepted matches were exact. Twenty-eight
selected patches remained unresolved. This limited result does not establish a
general false-match rate or robustness to deformation, rotation or occlusion.

Five synthetic tests cover signed small/large translation and brightness, spatial
selection and determinism, flat/edge/repeated patterns, unrelated images, excessive
motion, opposite movement, occlusion, localized consensus and invalid inputs.
All **314 workspace test executions**, strict Clippy, formatting and diff checks
pass (313 distinct cases; one epoch guard also runs in the example target).

Raw outputs, source copies, input hashes and both archived executables are in
`test-results/feature-motion-v1`. [The audit](audit.py) verifies those artifacts,
selection spacing/capacity, image bindings, spatial consensus, legacy evidence
identity and the known-transform errors. [Results](results.json) retain their
hashes, patch statuses, key-interval coverage and test-log identity. The runner
refuses to replace an existing output directory. Earlier grid artifacts remain
unchanged.

This increment improves descriptive observations. Movement validation, actor
identity and automatic semantic discovery remain unfinished; weighted progress
stays **35%**. The source is saved but unpublished, and the desktop preview remains
unavailable after quarantine. The protocol cites the primary feature-selection,
pyramid and reverse-check research; the Rust block matcher copies no third-party
code or weights and is not Lucas–Kanade or Median Flow.
