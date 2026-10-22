# Follow an appearance beyond its original rectangle

Fixed-region patch motion can lose a moving appearance as it leaves the original
box. Automatically selected regions now also receive a separate bounded template
search across the next image. The shared evidence reader, MCP and automatic
discovery archive all expose the result. No additional capture or model is used.

The original Rust implementation uses zero-mean normalized correlation, whose
definition is documented by [OpenCV](https://docs.opencv.org/4.13.0/df/dfb/group__imgproc__object.html).
No OpenCV code or dependency was added. An evenly spaced grid supplies a bounded
search; every pixel of the winning normalized rectangle is then checked. A
separate reverse search must unambiguously return to the original location.
This is a heuristic appearance correspondence, not a persistent object tracker.

The [protocol](PROTOCOL.md) fixes 512×288 normalization, at most 256 samples per
position, minimum contrast eight, correlation 0.92, uniqueness margin 0.06 outside
a two-pixel basin, and reverse tolerance one pixel. Normalized template sides must
be at least eight pixels; area must not exceed 16,384. All fitting positions are
searched, without a nearby-location prior. Border-touching winners abstain. The
best candidate is densely checked; no second candidate is substituted after
failure. At most four source-bound regions are considered by the existing policy.

Only a successful forward/dense/reverse result returns a location and source-pixel
displacement. Unknown results retain their reason and diagnostic scores, with null
matched locations. Scores are not probabilities. Source hashes, dimensions and
epoch are checked before publication. Existing fixed-region measurements,
selection, image interpretation, trial decisions and edit eligibility are unchanged.

## Frozen automated evaluation

The protocol, 146 source files and two offline executables were frozen before
generating and replaying five analytic cases plus the three unchanged prior
saved-Tetris cases. The matching executable received no scoring truth, game-state
oracle, annotations, game source or predefined gameplay meaning. All 31 input
files remained unchanged. Source/result hashes and raw records are retained under
`test-results/region-appearance-v1`; [compact results](results.json) retain the
measurements and process receipts.

| Analytic input | Expected | Result |
|---|---|---|
| Textured region moves (+160,+64), brightness +20 | Exact translation | Matched (+160,+64) |
| Stationary textured region | Zero displacement | Matched (0,0) |
| Two identical target copies | Unresolved | Ambiguous |
| Flat images | Unresolved | Low texture |
| Replaced texture | Unresolved | Poor match |

All five controlled cases pass. Unit tests additionally reject reverse ambiguity,
a false match that agrees only at sparse sample points, image boundaries, invalid
geometry and unsupported sizes. These controls do not establish real-game accuracy.

On the exposed Tetris pair, the earlier observer's orange-piece region matches at
zero displacement. Its green-piece region matches at approximately (+19.40625,
+120.375) original pixels, where the fixed-region matcher had zero accepted patches.
The board region is too large under the frozen size policy and remains unresolved.
The no-observation and omitted-capture ablations still produce no measurements.
All three prior cases' original selections and fixed-region reports compare equal.

The replay took 0.656 seconds locally and exited successfully. This is a single
eight-case run, not a performance benchmark. No threshold was changed after the
game results were examined.

## Separate exploratory geometry check

After the primary replay, a separate scorer examined the already retained game
oracle. This was **post-hoc** and is not the predeclared real-game acceptance test.
The product implementation and frozen results remained unchanged. The oracle and
its derived geometry never entered the matching executable.

The scorer reused the original evaluation's capture brackets, maximum 200 ms
oracle timing bounds, stable scene/canvas/object checks, no-pending-redraw rule,
and four cell-center pixel/color checks. Both captures 2 and 3 were scorable.
Source-region IoU associates the two small regions with the oracle's `next` and
`current` slots; labels do not choose the association. Their colors, relative cell
arrangements, canvas and viewport agree across the pair.

The preview's expected displacement is (0,0), matching exactly. The falling-piece
geometry changes by (20,120); absolute estimation errors are (0.59375,0.375) source
pixels. Destination overlaps are approximately 0.732 and 0.757 IoU. The board has
no corresponding target in this narrow scoring list and is unscored. This supports
the two correspondences on this one exposed pair, not identity, a general accuracy
rate, camera speed, a learned rule or a verified control. Results and frozen scoring
inputs are under `test-results/region-appearance-v1/saved-geometry-check`.

## Verification and limits

All **105 test executions** across vision, app and MCP pass, along with strict
Clippy, formatting and diff checks. Shared-reader/archive tests verify the new
result is available without new target I/O; hash, metadata and epoch mismatches
produce explicit errors. One initial integration test expected low texture for
an oversized normalized fixture; it correctly returned unsupported size. The
assertion was corrected, the original failure log retained and production limits
unchanged. The first formatting pass also caught and corrected a Rust cast parse
error before the successful development tests.

Matching assumes stable appearance and size. Rotation, deformation, occlusion,
scale changes, background dominance and identical-looking object replacement can
break that assumption. Dense validation reduces sparse-sampling aliases, but
does not prove identity. Resizing limits localization accuracy. No result is
propagated to a third frame or used to enable an edit. Regional evidence still
does not enter automatic activity interpretation or causal trial assessment.

Weighted goal progress remains **35%**. Native preview publication is still
unavailable after the existing Windows quarantine; no replacement was launched,
no protection setting changed and no sample was externally submitted.
