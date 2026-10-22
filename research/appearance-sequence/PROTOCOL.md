# Immutable appearance sequences

Freeze implementation, executable, protocol, manifest and original input hashes
before replay. Use saved images only: no game, capture, model call, native target,
input injection or edit. Do not tune the matcher or sequence policy on the results.

Use the existing exact-source region selection, capped at four regions. Each
sequence compares the same original rectangle and original image independently
with later captures. It does not update or round-trip a returned rectangle into
a new template. Stop at the first ambiguous/unsupported/failed match, source read
or hash error, missing intermediate capture, or changed geometry/epoch. Never
reacquire a later appearance after a failure. At most six following captures per
source and twelve attempted pairs across sources, allocated in chronological
rounds over the existing deterministic source order. These are additional to the
existing first-pair measurements.

For accepted matches retain displacement from the original and difference from
the previous accepted displacement. Retain capture-duration bounds and the shared
sampled-key exposure classification. Missing/invalid timing or input stays unknown;
it does not make a geometric match zero or establish absence of input. Report
stopping reasons and omitted work. Hash-bind the selection, algorithm versions,
limits, requested frame metadata and each accepted/failed prefix. No identity,
speed, key meaning, numeric memory correspondence or edit authority is established.

## Frozen cases

Reuse image bytes from `region-appearance-v1` controls to construct five declared
sequences. Only frame IDs/times and provisional-source bindings are new. Keep
scoring expectations separate from matcher inputs.

1. Original → translated → original → original appearance. Expect three accepted
   steps, signed step displacements (+160,+64), (-160,-64), (0,0), then end of
   requested captures.
2. Original → translated → unrelated texture → original appearance. Expect one
   accepted step, a failed step, and no reacquisition of the fourth image.
3. Original → translated → original, with capture number 3 omitted. Expect one
   accepted step and an omitted-capture stop before number 4, without reading it.
4. Four distinct overlapping source rectangles in ten identical images. Expect
   three accepted steps per source, twelve attempts total and shared-budget stops.
5. One source in ten identical images. Expect six accepted steps and a per-source
   step-limit stop, with no implied result for the remaining captures.

Append the unchanged three saved-Tetris manifests from `observed-regions-v1`.
Keep every series and stopping reason. Primary gameplay accuracy remains unknown;
this exposed corpus is not withheld validation. Assert that existing selection,
fixed-region and two-image appearance outputs remain exactly equal to the previous
`region-appearance-v1` replay. Do not send the game oracle to the product. Record
all input/source/result hashes and one local elapsed time, not a benchmark.

Unit tests separately verify that gradual appearance change can match adjacent
images while correctly failing the immutable original template; source corruption,
unavailable files, geometry changes and gaps stop the appropriate series; later
independent observations remain usable; unknown timing/input is retained; and
changes to frame/input metadata invalidate the corresponding evidence hashes.
Shared-reader/archive tests must show automatic publication without target I/O.
