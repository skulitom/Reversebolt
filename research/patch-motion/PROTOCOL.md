# Sparse patch-displacement measurement: frozen evaluation

Evaluate the original Rust implementation without target access, games, models,
input workers or new screenshots. This is a descriptive image measurement; it
must not replace the existing response metric or promote gameplay semantics.

## Fixed measurement

Decode bounded PNGs using RGB601 grayscale; reject unequal source geometry.
Resize to 256x144 with triangle filtering. Use 21 12x12 patches at x=32..224 in
steps of 32, y=32,64,96. Search integer offsets within +/-10 normalized pixels.
Use zero-mean squared differences to separate uniform additive brightness from
patch displacement. Before matching require grayscale standard deviation >=8.
Require residual RMS <=12, a distinct-basin runner-up cost >=1.25 times the best
MSE plus 9, and a best match strictly inside the search boundary. A distinct
basin differs by more than one pixel on either axis. Repeat the check backward;
require return error <=1 pixel per axis. Failed checks remain unresolved, with
no displacement value. Thresholds are heuristic and uncalibrated.

A consensus needs at least ten patches and 75% of successful matches within one
pixel per axis of a reported match. Report exact coverage, local matches and
unresolved patches. Opposing local displacements must not be averaged into zero.
Zero integer displacement is not evidence that a scene or actor is stationary.
Neither correspondences nor consensus identify camera, player or physical speed.

## Cases and acceptance

Synthetic tests cover signed translations with additive brightness, uniform and
repeated images, unrelated images, motion at/outside the search limit, occlusion,
opposing movements, invalid images, changed dimensions and changed scene epoch.
Do not adjust thresholds after inspecting the following real-case results.

Use all fifteen saved Hyperbolica discovery frames from independent-runtime-v2,
and all eighteen saved frames (three six-frame conditions) from completed
Manifold Garden comparison 080b5643-5718-494f-9ed6-751bf8059402 in planned-input-v1.
These are previously explored cases, not withheld acceptance. Hash every image,
freeze metadata/source/executable, archive the executable, and record every
interval's output and time. The product pair-measurement path must be used.

Check the old mean-absolute-difference values against their saved baseline with
exact floating-point comparisons; record any discrepancy. Summarize matched and
unresolved coverage by interval and condition without inferring true motion from
key samples. The earlier intervention can provide context, but does not supply a
ground-truth optical-flow map. Do not manually review these images or relabel them.

The result is useful structured evidence for shared human/agent investigation.
It does not change the planner's activity/semantic gates or comparison assessment.
No native-integration, independent-game or security-clearance credit is planned.

## Research basis

Local correspondence and image motion have ambiguity from texture, illumination,
occlusion and projection. See the authors' [optical-flow treatment](https://visionbook.mit.edu/optical_flow.html)
and [Haussecker and Fleet's brightness study](https://www.cs.utoronto.ca/~fleet/research/Papers/pami2001.html).
[Kalal, Mikolajczyk and Matas](https://dspace.cvut.cz/entities/publication/4f34fd8a-dffe-4857-96c2-42ed3d5a8449)
describe forward/backward discrepancy as a tracking-failure measure. This code
uses original bounded block matching; it is not their Median Flow implementation.
No third-party source code or trained weights are copied.
