# Seeded patch tracking — 2026-10-21

**Decision: research only; do not publish trajectories in the desktop.** The
gated tracker retained only **1/15** nonstationary endpoint opportunities from
initially correct seeds. Ungated tracking was worse than holding boxes still.
The full goal remains **21/100**. A low false-positive count does not compensate
for losing nearly all moving targets.

## Method and provenance

The new original Rust implementation in `reversebolt-observer::tracking` follows
image patches seeded by the existing image-only model and category review. It
receives pixels, timestamps, source-bound boxes and numeric seed identifiers.
It receives no names, game rules, memory, annotations or game-specific profiles.
It does not introduce a model download or an OpenCV dependency.

The patch score is mean-centered normalized color correlation, with separate
channel means, following the mathematical form described in the
[OpenCV template-matching reference](https://docs.opencv.org/4.13.0/df/dfb/group__imgproc__object.html).
The additional reverse search measures whether the new patch returns to its
prior position. Forward/backward disagreement is a known tracking-failure cue;
the [Kalal, Mikolajczyk and Matas paper](https://cmp.felk.cvut.cz/ftp/articles/matas/kalal-2010-fb_track-icpr.pdf)
motivates that check. This custom patch search is **not** an implementation or
reproduction of MedianFlow. Agreement is a heuristic, not proof of identity.

Images are reduced to a maximum side of 640 pixels. A patch contains 12×12 RGB
samples, bilinearly interpolated and normalized. The search uses scales
0.9/1.0/1.1, a four-pixel coarse stride and a radius of 1.5 times the largest
box side, clamped to 16–48 processing pixels. Four distinct coarse peaks are
refined to one-pixel steps so that a duplicate is less likely to be missed by
coarse sampling. Distinctness uses box IoU <0.5.

Fixed gates require correlation ≥0.72, agreement with the original patch ≥0.45,
best-to-distinct-second margin ≥0.06, and reverse center error ≤2.5 processing
pixels. Reverse correlation and distinctness must pass the same gates.
Uniform support, changed image dimensions or gaps longer than two seconds
revoke the track. Loss is permanent within a run; nothing is silently
reacquired. Accepted deltas are source-image pixel displacements, not world
movement or camera-compensated movement. The ungated comparator uses the same
search but omits the appearance/ambiguity/reverse gates.

All constants were fixed after procedural tests and before game replay. No
threshold was changed after these results. The executable, source, evaluator,
input requests, source imagery and label hashes are recorded locally in
`test-results/seeded-tracking-v1/frozen-replay/`. The
[compact record](../../research/seeded-tracking/evidence/replay-v1.json) retains
the results and provenance without redistributing game images.

## Frozen replay

The experiment uses all **48** adjacent annotated one-second windows from the
five previously examined Quake/Celeste clips, including **four zero-seed
windows**. These are development replays, regardless of legacy `test_` folder
names. They are not withheld-game evidence. Each window starts with filtered
original model boxes at frame N, then processes the actual ten 100 ms frames
through N+10. A fresh, independently saved model proposal at N+10 supplies a
fourth endpoint comparator; its inference cost is not included in tracker
timings. No new inference, screen capture, screenshot inspection or game input
was used.

The 57 seeds comprise 46 initial matches, 10 unmatched proposals and one
ignored proposal at IoU 0.3. The endpoint set contains 52 annotated targets.
Scoring combines dynamic actors, held components and annotated static
references. It uses the previous evaluator's greedy one-to-one box matching;
an unmatched proposal overlapping an ignore region at IoU ≥0.3 is ignored.
Labels are 320×180 with roughly ±2-pixel uncertainty. Semantic category
correctness is not an additional requirement in this localization score.

| Endpoint method | IoU 0.3 matched / false / missed | IoU 0.5 matched / false / missed |
|---|---:|---:|
| Hold original boxes still | 35 / 22 / 17 | 31 / 26 / 21 |
| Ungated patch search | 31 / 25 / 21 | 23 / 33 / 29 |
| Gated patch search | 17 / 2 / 35 | 16 / 3 / 36 |
| Fresh model + category review | 47 / 8 / 5 | 41 / 14 / 11 |

Fresh proposals have one ignored endpoint proposal at each threshold; the other
methods have none. At strict IoU 0.5, gated precision is 84.2% but recall is only
30.8%, versus 74.5% precision and 78.8% recall for the fresh model proposals.
These denominators differ from the preceding 53-pair crop-review experiment.

Only 35 initial correct seeds have the same annotation identifier at both
endpoints: 20 stationary and 15 nonstationary. Endpoint retention below requires
the correct match at both ends at IoU 0.3. It does not verify identity during
the intervening frames, through occlusion or across cuts.

| Method | Surviving seeds / 57 | Retained stationary / 20 | Retained nonstationary / 15 | Conditional nonstationary error |
|---|---:|---:|---:|---:|
| Hold still | 57 | 20 | 13 | 2.30 px |
| Ungated | 56 | 16 | 12 | 5.01 px |
| Gated | 19 | 16 | 1 | 3.50 px |

The error is mean displacement-vector error in annotation pixels, measured
only on retained matches. The different subsets prevent claiming an accuracy
improvement from the gated mean. Many nonzero annotated movements are small;
a post-hoc diagnostic finds only three opportunities with displacement ≥4
pixels, of which the gated method retains none. That diagnostic did not alter
the primary grouping or any tracking threshold.

The gated method records 29 weak-correlation, eight reverse-check and one
ambiguity rejection. Two initially false seeds still survive. Zero observed
endpoint identifier switches among its few survivors is insufficient evidence
of identity reliability. Hold and ungated each have one endpoint switch.

For the 440 updates in seeded windows, median gated update time was **13.53 ms**
and maximum **33.08 ms**; ungated median was **13.03 ms**, maximum **24.25 ms**.
These local timings include PNG decoding/resizing within each tracker update,
include updates after tracks were lost, and exclude acquisition, model seed
latency and process startup. They are not a live frame-rate guarantee.

## Checks and integration implications

All **91 workspace tests**, strict workspace Clippy and formatting pass. The
six added tests cover translation and source-pixel deltas, duplicate ambiguity,
disappearance/no reacquisition, uniform support, chronology/geometry/gaps, seed
validation and the bounded manifest contract. CLI checks reject a seed bound to
a different image and preserve an existing receipt. An independent reconstruction
from receipts reproduced every endpoint count: 48 windows × four variants × two
thresholds. Frozen input/source/executable hashes were checked after the run.

The offline runner accepts 2–120 frames, at most six seeds, a 64 KiB manifest and
the existing bounded PNG format. It streams frames and preserves source hashes,
timestamps, parameters and rejection reasons. It exposes no editing authority.

The present desktop capture worker also waits for model inference between
captures. Full OCR and inference delays exceed this tracker's two-second gap
limit. Successful replay at 10 Hz would therefore not establish compatibility
with the default live capture cadence. This experiment fails even before that
integration obstacle.

Keep the existing provisional appearance cards. Further work needs acquisition
independent of slow interpretation, explicit observation freshness and a method
that retains moving/animated objects without following scenery. Repeated model
proposals can refresh observations, but associating them into a persistent
identity needs separate evidence. No position, speed or edit control should be
derived from this tracker in the product yet.

Desktop preview `0.1.0-20261021-011044` was confirmed running in the user's
desktop session. This research-only change does not require a new preview.

With the existing local source data, reproduce into a new directory:

```powershell
cargo build -p reversebolt-observer --bin reversebolt-track --release
python scripts/seeded-tracking-evaluate.py --out test-results/seeded-tracking-new
```

The evaluator refuses an existing output directory and freezes all inputs before
starting the first tracking run. It reads `experiments/` without modifying it.
