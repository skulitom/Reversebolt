# Baseline-only appearance suggestions

Freeze source and inputs before running the same three retained planned-input-v1
OwnedPathFixture tests. These are striped research fixtures, not independent games.
Preserve every outcome, including empty/incomplete baselines and failure to propose
a useful region. Do not inspect screenshots or tune regions after seeing results.

Policy baseline_action_region_suggestions_v1 uses RGB601/Triangle grayscale
512×288 images. Rank square tiles with sides 16, 32, 64 and half-side stride by
mean max(0, abs(frame2-frame1)-abs(frame1-frame0)). Keep scores >=8. Ties use side,
y, x ascending. Convert to original pixels outward and reject invalid regions;
do not pad them. Suppress intersections exceeding one quarter of the smaller
normalized tile. Check at most four candidates using the existing NCC matcher,
unchanged thresholds and immutable frame-0 templates; stop at first unresolved
match. At most 24 frame checks. Recommend the first ranked candidate with all six
matches, idle Chebyshev displacement <=1 and all four action displacements >=2
normalized pixels. Hash-verified input witnesses must match and share one monitor.
No later-condition image/input/result informs ranking or candidate eligibility.

The runner archives the three records and only their baseline PNG/input files.
Later files are deliberately absent in the replay folder. The example asserts
condition zero in every reader and records actual accesses. Incomplete baselines
must do no image/input work. Run twice to separate output files and require exact
JSON equality. Archive source/manifests, protocol and runner; record and audit
binary and all source/input hashes before/after. Keep errors and raw outputs.

Automated generated-image tests separately cover a small moving appearance on
static background, same-direction repeated responses, opposite actions, source
corruption, missing observations, mixed monitors, stationary scenes, disappearance
without reacquisition, automatic API inclusion, opt-out and historical reads.
Existing matcher tests compare complete prepared-image/PNG measurement results.
No candidate grants identity, meaningful control, causality or write authority.
This is a baseline-selection prototype; no real-game intervention is validated.
No model, native helper, game, new capture, gameplay input or memory operation runs.
