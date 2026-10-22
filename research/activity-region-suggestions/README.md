# Automatic region proposals on retained gameplay

The baseline tile-ranking method is now available for passive activity evidence,
without observer labels. The shared application reader and MCP return numeric
proposals automatically when the existing activity selector finds two eligible
intervals. This does not create UI cards, assign names, choose a trial baseline,
grant edit authority or alter an assessment. Unidentified values remain hidden.

The first fixed replay **recommended no region in either game**. The result
exposes a selection weakness: large action-minus-quiet changes can rank a uniform
area, repeated texture, or an appearance that cannot be matched after the action.
These cases were retained without adjusting regions or thresholds after results.

| Saved passive recording | Ranked tiles | Tried candidates | Frame checks | Recommendation |
| --- | ---: | ---: | ---: | --- |
| Hyperbolica | 835 | 4 | 6 | None |
| Manifold Garden | 367 | 4 | 6 | None |

In Hyperbolica, three candidates failed the root self-check for low texture. One
matched the root and quiet image with zero displacement, then failed the first
action image (`best_ncc` approximately 0.855, below the unchanged 0.92 threshold).
In Garden, two candidates failed root texture, one failed self-match ambiguity,
and one similarly reached a poor first-action match. No candidate reached the
second action pair. Neither outcome establishes absence of movement.

Each case was run twice with identical complete JSON output. Each run read only
the five selected PNGs; unselected PNGs were absent from its replay directory.
Elapsed times were 0.281–0.344 seconds on this local debug-profile executable,
excluding build time; this is not a general performance benchmark. There were
no model calls, new screenshots, gameplay inputs, target memory reads or writes.

The method ranks change tiles in normalized images and tests the first action's
before-image template against five images. Root self-match and quiet comparison
precede the two independent activity intervals; the first unresolved appearance
stops that candidate. Limits are four candidates and twenty checks. Read the
[frozen protocol](PROTOCOL.md) for exact ranking, geometry, input and stop rules.
The shared baseline candidate generator and existing matcher are reused without
new search thresholds. The recommendation uses quiet <=1 and both action deltas
>=2 normalized pixels; matching direction or magnitude is not presumed.

The two activity pairs are independent correspondences, **not continuous object
tracking** between them. Intervening pixels are not inspected; camera/background
motion, identical objects and unobserved replacement remain possible. Even a
future positive result would not by itself establish a useful gameplay name,
physical quantity, controlling field or causal edit. This passive replay does
not advance the generalization milestone or the overall **35%** estimate.

Automated software checks cover opposite directions under the same sampled key,
exact image-read bounds, deterministic output, stationary first/second intervals,
unavailable/corrupt PNGs, actual-dimension disagreement, stale metadata, quiet
input, selected monitor changes, known capture/epoch/geometry discontinuity,
reordered episode blocks and stopping without reacquisition. A no-target-access
application test obtains proposals with zero observer labels, verifies the
default/opt-out behavior and preserves base evidence and the complete session.
All 364 workspace test executions pass; the affected vision checks were rerun
after the final source-order guard. Strict workspace/all-target Clippy and
format/diff checks pass on the final source.

Frozen artifacts are in `test-results/activity-region-suggestions-v1`; compact
results are retained in [results.json](results.json). The audit covers 160 source
files, 14 original inputs and 12 generated replay inputs, with no changes.

- Manifest SHA-256: `41a7bc8bcaeebae000ecb3988ded25ec203019a2aa3bf6a49b4a1c4655950d6f`
- Executable SHA-256: `28da9a5f9ffaa5d029ba514f026513d155d282e40878a617aadaeb919c4a39f8`

The runner refuses to overwrite the frozen study. A follow-up should separately
test selection that accounts for source texture and quiet stability before
spending the matching budget, retaining this failure as the comparison. Because
these two outcomes are now known, improvement on them would be development
evidence; new recordings remain necessary for an independent evaluation.

The desktop preview remains unavailable after the earlier Windows quarantine.
This study used a file-only replay executable and did not replace or relaunch the
flagged application.
