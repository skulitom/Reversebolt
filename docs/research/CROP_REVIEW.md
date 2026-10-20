# Crop-focused proposal review — 2026-10-21

## Question and implementation

The full-image observer could locate many real actors and held components, but
also labelled scenery as characters. This experiment tests whether a second
look at an enlarged, unlabelled region can reduce those errors. It separately
measures category filtering and the reviewer's proposed box changes.

The implementation is Rust, in `reversebolt-observer::review`, with a
`reversebolt-review` executable. Each review takes an existing image-bound
proposal. It checks source image hashes, sizes and timestamps against the
proposal receipt before connecting to the local model. It supplies the last
one or two supported locations, preserving any missing frame. The model sees
each full source image followed by an enlarged context crop. It receives the
focus geometry and original frame index, but **not the proposed name, category,
description, annotations, game name or memory**.

Crop width/height are three times the proposal extent, with a minimum span of
one eighth of the shorter image dimension. The crop is clipped to the source;
its longer side is rendered at 768 pixels with Lanczos3 resampling. Receipts
retain the exact source rectangle, focus transform, rendered dimensions and
hash, source evidence, prompt/schema/options, raw response and rejection reason.
The normalized crop boxes are mapped back to source-image coordinates using
that exact transform. Bounds are validated without silently clamping them.

The model classifies the focus as actor, held component, other or uncertain.
Supported actor/component judgments need a localization in a supplied crop.
Duplicate, reordered, unsupplied or invalid locations are rejected. Other and
uncertain judgments can omit locations. The transport retains the fixed local
endpoint, installed-model checks, no proxies/redirects, bounded response and
90-second generation timeout. At most four images / 16 MiB enter one request.

**This is a repeated judgment by the same model, not independent confirmation.**
It can repeat a mistaken category, select a neighboring object, or move a box
incorrectly. No result grants persistent identity, physical-motion meaning,
memory correspondence or editing authority.

## Frozen replay design

Use the preceding categorized Qwen run without regenerating or replacing any
proposal. All 53 archived image pairs remain in scoring, including the three
initial structural failures. Review every actor/component proposal having
support in the later image. Labels are consulted only by the Python scorer;
Rust inference accepts source images and candidate geometry only.

Compare three variants on identical annotations and one-to-one assignments:

1. **Original:** the unchanged proposal category and coordinates.
2. **Filtered:** retain original coordinates only when the crop reviewer
   supports actor/component and supplies later-frame evidence.
3. **Refined:** the same retained set, using mapped review coordinates.

The scorer asserts that original TP/FP/FN and category totals reproduce the
preceding results exactly. It counts structurally invalid or failed reviews as
missing evidence, not successful rejections of scenery. Original static-reference
boxes are included, as documented in [the preceding evaluation](LOCAL_OBSERVER.md).
One unmatched ignore-region proposal is handled by the same matching rule.
No new annotations or source-image inspection were needed for this experiment.

Before freezing, two proposals in the first development idle pair were probed:
a character and a flower. The flower was classified as other; the character's
initial response was structurally rejected for an overlong explanation. The
prompt was then shortened to request a single sentence, and schema string limits
were added. Both probes then returned valid judgments. The source and evaluator
were frozen before the complete replay; no thresholds or wording changed during
it. The initial failed response is preserved. This is reuse of already seen
clips and labels, not a withheld or unseen-game test.

Displacement is conditional on both endpoint boxes matching the same manual
entity at loose IoU 0.3. Earlier static-reference labels are not manufactured.
Report moving and stationary subsets separately, with their coverage. Units are
320×180 annotation pixels; the pre-existing single-rater labels have roughly
±2 pixel uncertainty. Different variants can retain different matched subsets.
These numbers do not measure physical movement or long-term identity.

## Results and decision

The frozen run completed all **53 pairs and 90 reviews**. The installed tag was
`qwen3.8:latest`, digest
`22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643`, on Ollama
0.40.6. The tag is recorded as observed, not treated as verified upstream model
identity or licensing. All model, binary, source and evaluator hashes matched
the launch manifest after completion. Scoring the retained receipts again
produced an exactly equal result without model inference.

There were 39 actor judgments, 23 held-component judgments, 27 other judgments
and one structural failure: a review named an unsupplied frame. The 62 retained
proposals include one ignored-region proposal. The three original invalid
proposal pairs remain in the denominator. Median review time was **3.817 s per
proposal**, with 344.432 s summed across reviews, in addition to the original
proposal pass. This is too expensive to run for every captured frame.

Scoring is class-agnostic localization of the annotated actors/components and
seven static references. It does not evaluate every useful object or establish
that an actor name or role is correct. False positives include misplaced boxes
as well as mistaken object categories.

| Variant | IoU | Matched | Unmatched proposals | Missed | Precision | Recall |
|---|---:|---:|---:|---:|---:|---:|
| Original | 0.5 | 45 | 44 | 13 | 50.6% | 77.6% |
| Category filtered | 0.5 | 45 | 16 | 13 | 73.8% | 77.6% |
| Reviewed coordinates | 0.5 | 44 | 17 | 14 | 72.1% | 75.9% |
| Original | 0.3 | 51 | 38 | 7 | 57.3% | 87.9% |
| Category filtered | 0.3 | 51 | 10 | 7 | 83.6% | 87.9% |
| Reviewed coordinates | 0.3 | 50 | 11 | 8 | 82.0% | 86.2% |

Filtering removes 28 unmatched proposals without losing any matched box on this
archive. **Only 27 removals are semantic judgments; the other is a failed review.**
Do not count that failure as a correct scenery classification. At strict IoU,
unmatched proposals by clip fell from 4 to 2 (development Quake), 8 to 0 (idle
Celeste), 15 to 5 (moving Celeste), 7 to 6 (test Quake), and 10 to 3 (transition
Celeste). The improvement is concentrated in the Celeste clips. The existing
`test_` names do not make these previously inspected clips fresh held-out data.

Filtering retains 43 paired matches from 44 current dynamic matches: 11 moving
pairs with mean displacement error 4.144 annotation pixels, and 32 stationary
pairs with mean predicted displacement 0.328 pixels. Refinement retains only
42/43, including 10 moving pairs with mean error 3.770 pixels. Its lower error
is conditional on a different subset, not evidence that it improved tracking.
It loses a real box in development Quake at frame 40.

**Keep the original coordinates and retain crop review as experimental category
filtering.** It is useful evidence for reducing clutter but still leaves false
proposals, misses actors and adds substantial latency. It is not enabled as an
automatic desktop source, and cannot publish editable controls. Next work needs
independent visual grounding, appropriate model-job ownership and an explicit
provisional observation path into the shared session. The full goal remains at
20%; this experiment earns no unseen-game or causal-validation credit.

The [compact evidence](../../research/local-observer-review/evidence/replay-v1.json)
includes per-clip metrics, conditional motion coverage, every review-receipt
hash, frozen runtime hashes and negative provenance checks. Raw source images
and model responses remain local under the evidence root below.

## Verification and reproduction

The full 81-test workspace suite and strict Clippy pass. New tests cover crop
round trips at all image edges and tiny regions, invalid geometry, unsupplied
frames, uncertainty without boxes and attempts to add edit authority. Native
CLI negative checks rejected altered source timestamps and image hashes before
model connection and wrote no review receipts.

```powershell
cargo test --workspace --locked
cargo build -p reversebolt-observer --release --locked
python scripts/local-observer-review.py --source test-results/local-observer-v1/qwen-categorized-all --out test-results/NEW_REVIEW --model LOCAL_INSTALLED_TAG
```

`--rescore` reads retained reviews and never starts inference. Inputs and receipts
are immutable; source results, proposal files, model digest, executable, Rust
sources and evaluator hashes are retained. The raw evidence root is
`test-results/local-observer-review-v1/`. The executable also accepts an input
manifest containing `model`, original `frames`, `proposal_receipt` and `candidate`
(the zero-based entity index), followed by a new output filename.

The implementation and evaluator are MIT. They use the existing local model
installation; no weights or commercial images are redistributed. This experiment
does not control games, capture new screenshots or change the desktop interface.
