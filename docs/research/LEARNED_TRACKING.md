# Learned point tracking — 2026-10-21

**Decision: retain as research, without desktop integration or additional goal
credit.** Learned point correspondences improve the earlier patch tracker's
small-motion retention, but the gated method loses all three larger-motion
opportunities. The full goal remains **22/100**.

## Implementation and provenance

The official [causal BootsTAPIR streaming implementation](https://github.com/google-deepmind/tapnet/blob/730cda1c730877cfedbe01bf87fb1cadb78a565d/tapnet/pytorch_live_demo.py)
supplies point predictions. Its three required Torch source files are retained
verbatim under `research/learned-tracking/vendor/`, together with the upstream
license. Code revision is `730cda1c730877cfedbe01bf87fb1cadb78a565d`.
The [official model card](https://huggingface.co/google/tapnet/blob/5d3fb48e76c5422841e38501514121e251beabb7/README.md)
licenses the sources and checkpoints under Apache-2.0. The 218,887,028-byte
checkpoint is pinned by revision and SHA-256 in `upstream.json` and kept outside
Git. No game-specific profiles, labels, internal layouts or process access enter
inference. The images can of course contain visual game-specific features;
this does not establish that the pretrained model has never seen related data.

The isolated Python research runner uses Torch 2.4.0+cu124 on an RTX 4090. It
resizes each image to 256x256, seeds a row-major 3x3 grid at the inner 25%, 50%
and 75% positions of every existing model box, and initializes query features
from the first image. Each subsequent call receives exactly one frame and the
prior causal state. No future frames, manual reseeding or reacquisition are
used. As in the official streaming demo, it uses the final refinement output
and visibility `(1-sigmoid(occlusion))*(1-sigmoid(expected_dist)) > 0.5`.
Coordinates are converted back to source-image pixels.

Original Rust code in `reversebolt_observer::point_tracks` aggregates those
correspondences. It estimates an isotropic scale from the median pairwise
distance ratio and translations from the median per-axis offsets. The gated
variant needs five visible in-frame points, scale 0.5–2 relative to the seed,
and five residuals within max(2 source pixels, 12% of the scaled shorter seed
side). Loss is permanent. The ungated comparator uses all nine predictions and
omits visibility, scale-range and coherence gates. Neither variant models
rotation, nonrigid deformation, camera motion, foreground segmentation or
world coordinates. Consensus is not proof of object identity.

Inputs have bounded point/frame counts, source-image hashes, fixed point order,
finite geometry, stable dimensions and increasing timestamps with gaps at most
two seconds. Seeds must span at least one source pixel on each side. Receipts
retain model and aggregation inputs, hashes, per-point visibility, rejection
reasons and timings. The desktop and MCP do not invoke this research runner.

## Evaluation

The procedural check translates a textured square 30 pixels horizontally and
10 vertically over ten updates. Median endpoint point error is **0.451 source
pixels**, and the gated box survives. This tests coordinate conversion and the
streaming path; it is not game-generalization evidence.

The game replay reuses exactly the earlier [patch-tracking](SEEDED_TRACKING.md)
48 one-second windows from five previously examined Quake/Celeste clips, with
the same 57 initial model-box seeds and all four empty-seed windows. Each window
processes ten actual 100ms updates. There are 46 initially matched seeds, ten
false seeds and one ignored seed. The endpoint truth has 52 annotated boxes.
The original greedy one-to-one scorer is imported unchanged, using endpoint
IoU 0.3/0.5 and ignore overlap 0.3. Annotations remain in the scorer, outside
the inference and Rust aggregation processes. No new game input, capture or
manual image review was required.

| Endpoint method | IoU 0.3 matched / false / missed | IoU 0.5 matched / false / missed |
|---|---:|---:|
| Hold original boxes | 35 / 22 / 17 | 31 / 26 / 21 |
| Prior ungated patch search | 31 / 25 / 21 | 23 / 33 / 29 |
| Prior gated patch search | 17 / 2 / 35 | 16 / 3 / 36 |
| Learned points, ungated | 36 / 21 / 16 | 30 / 27 / 22 |
| Learned points, gated | 29 / 3 / 23 | 23 / 9 / 29 |
| Saved fresh model + category review | 47 / 8 / 5 | 41 / 14 / 11 |

The independently refreshed model has one ignored endpoint proposal. Its boxes
have no persistent track IDs. Localization scoring does not additionally demand
the correct semantic category. A false seed can coincide with a later true box;
that does not retroactively make its initial interpretation correct.

| Method | Surviving / 57 | Same-ID stationary / 20 | Same-ID moving / 15 | Mean moving displacement error |
|---|---:|---:|---:|---:|
| Hold | 57 | 20 | 13 | 2.30 px |
| Prior gated patch search | 19 | 16 | 1 | 3.50 px |
| Learned points, ungated | 57 | 20 | 12 | 3.55 px |
| Learned points, gated | 32 | 17 | 11 | 3.39 px |

Errors use annotation pixels (320x180, roughly ±2px uncertainty), conditional
on retained correct matches. Different accepted subsets prevent direct accuracy
claims. Most nonzero displacements are small enough that holding boxes still
is a strong comparator. For displacement at least four pixels, the gated
learned method retains **0/3**, and the ungated method retains **1/3**.
The gated losses occur after two updates (incoherent points), three updates
(insufficient visibility), and five updates (incoherent points), respectively.

Across all seeds the gated method first rejects 12 for insufficient points,
11 for incoherence and two for scale. Four initially false seeds survive.
There are zero observed gated endpoint ID switches and three ungated switches;
this limited endpoint test cannot verify identity in the intervening frames.

The final replay's 440 seeded updates take **78.18ms median / 139.43ms maximum**
on this machine. This includes decoding, resizing, inference and GPU
synchronization, but excludes acquisition, model-box discovery and Rust
aggregation. It does not prove a stable live 10Hz observation pipeline or
performance while sharing the GPU with a game and the local vision model.

## Freeze, checks and next decision

Code, model, inputs, labels and scorer hashes were fixed before the first game
inference. After the first replay, code review added a one-pixel minimum seed
side to prevent degenerate floating-point point grids. The smallest seed in
this cohort is 25.6 source pixels. No inference, tracking gate or scoring
threshold was tuned. A separately frozen full rerun has identical detection,
retention, displacement and rejection results. Both attempts and the original
aggregator source/binary are retained locally; timing varies between runs.

Five Rust tests cover translation/scale with an outlier, permanent loss,
incoherent/collapsed support, subpixel geometry and atomic refusal of malformed
updates. All 110 workspace tests, strict Clippy and formatting pass. Logs are
retained in `test-results/point-tracking-v1/`. The
[compact evidence record](../../research/learned-tracking/evidence/replay-v1.json)
retains hashes, results and all larger-motion failures without game imagery.
`scripts/point-tracking-evidence.py` verifies the local receipt chain.

Next work must distinguish points on the intended foreground object from
background points and allow supported deformation, then evaluate without
retuning against these outcomes. Fresh detection remains the stronger endpoint
baseline here. More polished controls do not compensate for unreliable
appearance/motion understanding; this experiment earns no new capability or
independent-game credit.
