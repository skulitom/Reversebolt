# Browser activity eligibility replay

Evaluate production commit `24f5e89` unchanged on all four retained runs from
`independent-games-v1`: two 2048 runs and two Javascript Tetris runs. These games
were independently authored, but the recordings are already exposed development
data. This is not withheld acceptance or a fresh gameplay intervention.

Use every retained frame in each final status, in its original order, with its
complete original input/timing metadata. Resolve each PNG by its retained number
and require a unique file with the retained SHA-256. The replay manifest contains
only these references, paths, investigation and batch. Cached observations come
only from the product's retained visual tracks. Do not use game source, oracle,
DOM geometry, evaluator-delivered events, field addresses or new annotations.

Run the existing `runtime-appearance-replay` executable with `--observations`.
That mode does not call a model. Its backend rejects native reads, writes and
capture. Freeze executable, production source/lockfiles, protocol/runner, final
statuses, all PNGs and generated manifests/observation documents before running.
Create new output under `test-results/browser-activity-replay-v1`; never rerun
into an existing directory or tune the product on intermediate outcomes.

Retain the entire reader output and every process outcome. Report input exposure
classifications/reasons, image interval durations and differences, quiet and
bounded-key episodes, selected activity source, regional attempts/outcomes, and
any loss of input/image evidence. Verify base evidence is identical before and
after replaying product observations; no annotation may repair input coverage.
If a source is absent, report the concrete earlier gate rather than calling it a
failed image match or inventing a gameplay conclusion. Do not count a selected
source as a named, causal or editable control. Audit frozen files afterward.
