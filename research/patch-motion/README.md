# Sparse image-patch motion: experimental evidence

The shared evidence reader now reports local image-patch displacements alongside
the existing image-difference measurement. This adds a way to describe some image
changes without a model call. **Coverage on retained game captures is too low to
validate movement.** It creates no controls, gameplay names or edit authority.

The original Rust matcher compares 21 patches in a normalized 256×144 image.
It separates uniform additive brightness changes, rejects weak or repeated
texture and checks each match in the reverse direction. Displacements use integer
image pixels, with right/down positive. Failed patches have null displacement;
zero displacement does not establish no movement. A consensus requires at least
ten supporting patches and agreement among 75% of successful matches. These are
uncalibrated heuristic thresholds, not probabilities or physical speeds.

`read_play_evidence` returns these measurements with image IDs and hashes, and
automatic activity reviews retain them in their evidence archive. Historical
`read_runtime_test` frame reads also return the preceding pair within the same
condition. A missing or corrupted previous image leaves the requested current
image readable with an explicit motion error. A corrupted requested image still
fails verification. Planner activity selection, comparison assessments and
semantic promotion are unchanged. See [the API details](../../docs/MCP.md).

## Retained-image results

The fixed [initial protocol](PROTOCOL.md) and [corrected source protocol](PROTOCOL-v2.md)
were evaluated through the product pair-measurement code. No thresholds were
adjusted after reviewing results. The final replay covers 48 images and 43 pairs:

| Source | Pairs | Matched patches / total | Nonzero matches | Consensus pairs |
| --- | ---: | ---: | ---: | ---: |
| Hyperbolica discovery | 14 | 51 / 294 | 15 | 0 |
| Owned striped fixture, three conditions | 15 | 0 / 315 | 0 | 0 |
| Manifold Garden discovery | 14 | 36 / 294 | 4 | 0 |

Hyperbolica has 0–6 matched patches per pair and Manifold Garden 1–4. Most game
patches have insufficient texture. The striped fixture produces boundary or
texture exclusions; its repeated pattern remains unresolved even for identical
images. None of these exclusions means the scene is stationary. Partial matches
have no ground-truth flow map and may be incorrect. They do not identify the
player, camera or an input's effect. These are previously explored captures,
not a withheld generalization evaluation.

All **43 prior image-difference values are preserved exactly**. There were no new
screenshots, manual image reviews, model calls, target accesses, inputs or gameplay
writes. Weighted progress remains **35%**. This source increment is unpublished;
the desktop preview remains unavailable after quarantine.

## Source correction and frozen runs

The three fixture cases in V1 were incorrectly labeled Manifold Garden. They
actually came from the owned ABI fixture and striped renderer in planned-input-v1.
[The correction](../investigation-history/PROVENANCE-CORRECTION.md) identifies the
authoritative record and corrects the same mistake in the earlier history replay.
No independent-game credit was awarded for those fixture results.

V1's files and labels are preserved. V2 corrects the labels and adds fifteen
separately identified genuine Manifold Garden discovery images. V3 freezes the
final source after adding reader tests and API descriptions. All three runs use
the same archived executable; V2 and V3 outputs are identical. Their elapsed
times were 1.516, 2.797 and 2.750 seconds respectively. Raw artifacts are retained
under `test-results/patch-motion-v1`, `-v2` and `-v3`.

[The audit](audit.py) checks frozen source copies, executable and input hashes,
provenance, unchanged earlier measurements, null exclusions and output identity.
[Recorded results](results.json) include artifact hashes and patch-status counts.
The runner refuses an existing output directory; it does not overwrite prior runs.

Workspace checks pass: **313 test executions** (312 distinct cases; the epoch
guard also runs in the example target), strict Clippy, formatting and diff checks.
New coverage includes signed translation with brightness changes, missing texture,
repetition, excessive motion, occlusion, opposing motion, invalid images, changed
geometry/epoch and historical-reader corruption handling.

The protocol links the primary research on optical-flow ambiguity, illumination
and forward/backward checks. This implementation copies no third-party code or
weights and is not an implementation of Median Flow. Feature selection or
multiscale correspondence needs a separately frozen evaluation before claiming
better coverage; gameplay validation remains a further requirement.
