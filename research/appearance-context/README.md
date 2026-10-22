# Regional measurements reach the activity interpreter

The automatic evidence reader now projects its existing appearance traces onto
the exact two action intervals selected for interpretation. The compact summary
contains original source rectangles, matching before/after rectangles and signed
displacements in original image pixels. Failed matches keep their status;
intervals outside a trace remain `not_measured` with no rectangle or displacement.
Measurements are not copied between action episodes.

The summary has a 4 KiB limit, at most four sources and two outcomes per source.
It binds the complete activity source, frame metadata and original sequence
report. Digests use the corresponding Rust/Serde serialization, not raw archive
file bytes. No observer names/descriptions are sent in this numeric context.
The image interpreter receives the summary with explicit limitations. Its receipt
and shared `play_context::Context` retain it without overriding model uncertainty.
Existing source/interpretation transfer and history paths carry the optional
field; older records omit it and retain their previous shape. Metadata hypothesis
generation, comparison eligibility and write authority are unchanged.

## Verification and frozen replay

All 354 workspace test executions and strict workspace Clippy pass. New cases
cover selected-pair displacement versus origin displacement, correct displayed
rectangles, missing intervals, failed matches, input/image/source rebinding,
malformed geometry, and uncertainty/serialization through the interpretation
record. An initial test used a rectangle extending three pixels beyond its
45-pixel fixture image; correcting the fixture retained the production check.

The [protocol](PROTOCOL.md) was fixed before inference. Two cached-observation
replays retained the original four/five model observations without a new object
detection call. Every original sequence report and base play-evidence result
remained exactly equal to runtime-appearances-v1. The frozen experiment then made
four activity interpretations: baseline and augmented for each case, on identical
images with the same model/options. There were no retries.

| Exposed saved case | Selected captures | Regional evidence supplied | Baseline → augmented flag |
|---|---|---|---|
| Hyperbolica | 2→3 and 8→9 | One first-pair match, x = −58.0078125; all second-pair results unmeasured | false → false |
| Manifold Garden | 4→5 and 10→11 | One first-pair match, x = +40; all second-pair results unmeasured | false → false |

Both y displacements are zero. Other first-pair outcomes are explicitly failed
or unmeasured. Global patch measurements are identical across each ablation.
The augmented Garden description discusses the upper-right colored area, but
claims consistent direction across both intervals while retaining a false
consistency flag. Hyperbolica's augmented prose calls the central tree largely
stable despite the retained shift in the rectangle previously labelled as a
tree. Those labels and matches are themselves unverified; neither disagreement
is ground truth. These results demonstrate delivery of partial evidence, not
improved interpretation accuracy. The engine retains the disagreement instead
of promoting the prose to a verified control.

All 151 frozen source files, 37 original input files, four generated interpretation
manifests and both executable hashes pass the audit. The model digest was checked
before each interpretation. [Compact results](results.json) include every model
description, uncertainty and numeric outcome. Full archives are in
`test-results/appearance-context-v1`.

- Source manifest SHA-256: `0a9553faada9c3e442eb42b7597515aa80845095f08f83cb95fc9e7b8ebafe36`
- Model digest: `22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643`

No game, new screenshot, input or field write ran. Generalization, causal
association to internal fields and useful editable controls remain unverified.
The desktop preview is still unavailable after its recorded quarantine.
