# Stable source texture for pre-edit baseline proposals — fixed study

Bring the tested passive region quality selection into the fresh runtime-test
baseline, without reusing discovery coordinates or choosing from post-edit data.
This changes the proposal step, not locking/edit authority or trial assessment.

Policy `baseline_stable_texture_regions_v2` keeps the existing six-frame baseline
and verified sampled-input prerequisites. Missing/corrupt evidence, mixed monitors
or mismatched planned inputs stop before candidate matching. Its first-action
change-minus-idle tile gate is unchanged (16/32/64 normalized pixels, half-side
strides, mean residual >=8). Then use the shared stable-texture selector with
**frame 0 as the root template and frame 1 as the quiet comparison**. This differs
from passive selection, whose source is the first action's before frame.

For the exact normalized matcher rectangle, require sparse and dense source
standard deviation >=8, center 9x9 minimum-eigenvalue corner score >=16 and quiet
mean absolute difference <=4. Rank by corner score/(1+quiet difference), then
action-minus-idle change descending, grid tile size/y/x ascending. Greedily suppress
overlap above a quarter of the smaller grid tile. No new constants or search
thresholds are introduced relative to the shared quality selector.

Try at most four candidates and 24 immutable-template checks against all six
baseline frames, stopping a candidate at its first unresolved match. No candidate
is replaced or reranked using a later match. A recommendation requires all six
matches, quiet displacement <=1 and all four planned-action displacements >=2
normalized pixels. The recommendation binds baseline frame 0's hash. It is not
locked automatically and never grants semantic, causal or edit authority.

Before freezing, controlled tests must keep the existing automatic/historical
read, corruption/input and no-reacquisition checks. Add a larger flat flashing
distractor and a small idle brightness change. The old method should abstain,
while the new method must recover the known alternating +16/-16-pixel local
motion. Verify that source quality matches the first template, not the changed
idle image. Exercise the actual read recommendation -> pre-edit lock -> retained
changed/recovered comparison -> restart path using generated evidence, with no
native input or field write. Keep the original actor/revision and post-edit-lock
refusal checks. Do not mistake this software fixture for a real intervention.

Freeze source/protocol/runner and exactly the three retained OwnedPathFixture
records used in `trial-appearance-suggestions-v1`, with baseline files only.
Retain prior outputs/manifest for comparison. These records are not games. Run
both `change_only` and `stable_texture` twice per record using the same file-only
executable. Require complete old output equality to the original archive and
complete within-method equality. Record read indices, exclusions, tried regions,
matching status, recommendation and timings. Incomplete baselines must read no
files; all other reads must belong to condition zero. Keep failures/abstentions.
Do not tune after the retained-data outcomes; corrections require a new study
version with this one preserved.

Audit all current/frozen sources, original inputs and executable hashes after
replay. No model call, screenshot, game input, target memory operation, desktop
replacement or relaunch is part of this study. No positive control or independent
generalization credit is expected from these fixture records.
