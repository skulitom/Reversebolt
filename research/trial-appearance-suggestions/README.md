# Automatic baseline appearance suggestions

The shared test reader and final baseline observation now derive possible image
regions automatically when a test reaches `ready_to_apply` without a saved choice.
The agent can pass `recommended` to the existing appearance-lock command with
the current revision. Nothing is locked or edited automatically. No report or
unidentified control is added to the desktop board.

`read_runtime_test.suggest_appearance` can opt out or request the same baseline-only
analysis from history. Errors preserve readable test metadata. Suggestions retain
the complete baseline digest, image/input hashes, input verification, scores,
per-frame matching diagnostics and signed displacements. Later condition files
are never opened. The algorithm retains six normalized images and uses the existing
streaming verifier, rather than caching all original PNGs or rerunning a model.

The [fixed policy](PROTOCOL.md) ranks tiles using first-action image change minus
idle change. Four candidates at most are checked against the immutable first
baseline image, with at most 24 frame checks and no reacquisition after failure.
A recommendation requires matching sampled input from one monitor, six accepted
appearances, little idle displacement and movement in each planned action. The
matcher and its thresholds are unchanged; its prepared-image path now borrows
the normalized images. Existing tests compare complete PNG/prepared results.

All 360 workspace test executions and strict workspace/all-target Clippy pass.
Generated-image tests recover a small translated appearance against textured
static background and its alternating +16/-16 normalized-pixel responses. They
check that later corrupt conditions cannot affect the result, historical reads
produce identical suggestions after restart, and no plan/edit is created. They
also reject missing/corrupt input or images, mixed monitors, stationary scenes,
incomplete baselines and disappearance without reacquisition. These are synthetic
image/dispatcher tests, not game understanding or physical response validation.

Frozen replay of the three existing **OwnedPathFixture** records:

| Baseline frames | Outcome | Ranked regions | Frame checks | Recommendation |
| ---: | --- | ---: | ---: | --- |
| 0 | Incomplete | 0 | 0 | None |
| 6 | No supported region | 647 | 4 | None |
| 2 | Incomplete | 0 | 0 | None |

All four candidates in the complete striped baseline fail their first match as
ambiguous. No attempt is made to replace them after seeing that result. Each case
was run twice with exact JSON equality. Only baseline evidence was copied into
the replay folder; incomplete cases accessed no PNG/input files. All 157 frozen
source files, 19 inputs and the binary passed audit. Manifest SHA-256:
`79cb1bfd59cb00036bbe78f3deb3e570bb38dd7f7045de6948b760739cbc65d2`.
Archive: `test-results/trial-appearance-suggestions-v1`.
See [retained summary](results.json).

The heuristic can prefer background or camera motion. It does not identify a
character, establish meaningful control, prove causality or remove baseline
selection bias. Real-game intervention evidence is still absent. No game, new
capture, input, target access or model call ran in the replay. Desktop publication
remains unavailable after the prior Windows quarantine.
