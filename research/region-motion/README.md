# Fixed image regions preserve smaller displacement

The existing shared `read_play_evidence` action now accepts one optional image
rectangle. It measures the same rectangle in each saved pair, independently of
whole-screen normalization, and returns `region_motion`. There are no new
captures, model calls, target accesses or UI cards. An agent can examine an area
suggested by an earlier visual observation without requesting another image.
Automatic region selection and tracking an object as it moves remain unfinished.

Each result preserves image IDs/hashes, the requested rectangle, source dimensions,
patch coverage and unresolved matches. A source digest binds the region together
with complete frame timing/input metadata and the algorithm/policy. Displacements
are in a 256×144 normalization of the region; an explicit scale converts them to
approximate original-image pixels. Different crop sizes must not be compared as
though they had the same units. Malformed/outside rectangles, geometry and epoch
changes, bad hashes and incorrect PNG dimensions cannot produce a valid region
measurement. Existing whole-image evidence and decision gates remain unchanged.

## Frozen evaluation

The [protocol](PROTOCOL.md) and all source/binary/input identities were frozen
before the eight-case replay. Six are controlled image transforms, and two are
previously retained game sequences with a geometrically selected central region.
The latter are not identified objects and have no regional motion ground truth.

| Controlled case | Whole-image consensus | Region consensus | Region matched patches |
| --- | --- | --- | ---: |
| +8 source pixels, 256×144 region | (0, 0) | (8, 0) | 44 |
| −8 source pixels, 256×144 region | (0, 0) | (−8, 0) | 47 |
| +4 source pixels, 128×72 region | (0, 0) | (8, 0), source scale (0.5, 0.5) | 45 |
| Plain region | — | Unresolved | 0 |
| +48 pixels, beyond search range | — | Unresolved | 0 |
| Rectangle outside the image | — | Explicit error | — |

This demonstrates why stationary full-image consensus cannot rule out local
movement. It does not identify a character, infer animation or establish physical
speed. A fixed rectangle may contain background, multiple objects or an object
entering/leaving it; successful patch matches do not establish object identity.

Each genuine game sequence retains ten region consensuses at (0, 0), with its
four action intervals still unresolved. Region-match accuracy is unknown. All
28 earlier full-image patch reports and activity-evidence results remain exactly
unchanged. No claim of successful gameplay-motion validation or generalization
is added.

The replay exited normally in 4.610 seconds, a single local run rather than a
benchmark. The audit verifies 139 frozen source files and 47 unchanged input files,
including generated fixtures. [Results](results.json) and local
`test-results/region-motion-v1` retain the manifest, expected transforms, images,
source snapshots, binaries, process receipts and outputs. No manual image review
or gameplay control was used.

After the replay, the MCP help text was clarified: initial saved-frame failures
reject the whole request before regional analysis; later region-specific errors
are retained per pair. The audit checks this exact help-only change against the
frozen source. No API schema, measurement or executable control path changed.

## Tests and limits

All 95 test executions across app, vision and MCP targets pass, along with strict
Clippy and formatting. The app helper checks region/hash/geometry/epoch bindings;
vision checks source scaling, weak texture, invalid rectangles and equality with
the original matcher when the region is the full image. This does not substitute
for a new native desktop or game test. Logs are
`test-results/region-motion-tests-final.log` and `region-motion-clippy.log`.

During fixture development, the first full-image-consensus assertion failed on
a per-pixel random background. The fixture was changed to preserve background
texture after downsampling; production thresholds were not changed. An initial
MCP compile also failed on an untyped large integer schema limit; that literal
was given its correct unsigned type. The failed build log remains at
`test-results/region-motion-tests.log`. These are development regressions, not
held-out evaluation results.

The original 10 MiB/four-million-pixel image bounds remain. Regions are at least
32×32 pixels; both images must have the same dimensions. The existing feature
matcher still has limited displacement range and can miss appearance changes,
repetition, occlusion or untextured motion. Whole-screen and regional evidence
remain descriptive. The desktop is still unavailable after quarantine, and this
source increment is unpublished. Full-goal progress remains **35%**.
