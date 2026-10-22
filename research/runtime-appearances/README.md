# Appearance observations during runtime discovery

Runtime automatic discovery previously captured fifteen play images and reviewed
metadata, but scheduled no object observation pass. Consequently its automatic
region reader had no local observation sources even though both components existed.

With a configured local model, the same shared job now starts one background pass
after capture 2. Later captures continue. Before runtime inspection/naming, the
engine settles that job's worker so the evidence archive can include the original
regions and their later appearance comparisons. There is no additional final
image pass and no new edit authority. Without a configured model, the plan remains
fifteen play captures followed by one runtime discovery.

Preparation failures receive immutable unavailable receipts before a warning is
published. A resize invalidates an old appearance without cancelling independent
metadata work. User cancellation, replaced investigation, ownership and evidence
storage failures retain their existing protections. The appearance template and
thresholds are unchanged. The current activity interpreter does not yet consume
these regional sequences as semantic or causal evidence.

## Automated evidence

All 82 app tests pass, including the real runtime-command join point, late
capture-2 publication, subsequent automatic archive, missing timing, resize,
cancellation, replacement and persistence failures. Strict app Clippy across all
targets and the standalone replay build pass. The native runtime precondition
is deliberately unavailable in the integration fixture: no helper is loaded.

The [protocol](PROTOCOL.md), source, executable and inputs were frozen before
two inference runs. The file-only replay used the actual production observation
worker, publication and play-evidence reader on previously exposed saved images.
Game names and manifest metadata were not supplied to the model. No new captures,
input, memory reads or field writes occurred. No retry or threshold adjustment
was made after seeing results.

| Saved case | Published provisional appearances | Completed model calls | Selected regions | Accepted sequence steps |
|---|---:|---:|---:|---|
| Hyperbolica | 4 | 5 | 4 | 6 for one region; 0 for the other three |
| Manifold Garden | 5 | 6 | 4 | 6 for one region; 0 for the other three |

Each case used nine sequence attempts. One series reached the six-step limit;
the others stopped immediately on poor/ambiguous appearance or unsupported size.
Five eligible Garden regions competed for four slots; its omitted-observation
counts are empty because the fifth was valid but excluded by the selection budget.
The same reader with no observations selected zero regions and attempted zero
sequence pairs. Base play evidence remained exactly equal before/after publication.

Inspection of the retained results shows:

- The model-labelled Hyperbolica “Tree” region had source-pixel x deltas
  `[-58.0078125, 0, 53.7109375, 0, 0, 0]`, all y deltas zero.
- The model-labelled Garden “Ceiling panel” region had x deltas
  `[-40, 0, 40, 0, 0, 0]`, all y deltas zero.
- Both had sampled exposure `[D, no keys detected, A, no keys detected,
  no keys detected, no keys detected]` for those intervals. This does not prove
  input delivery, camera motion, object identity or a controllable gameplay value.

Localization/category accuracy is unknown. For example, the model classified a
robot-like figure as an object, and included architectural surfaces. No semantic
truth was supplied and no successes are credited for these labels. The runs took
33.69 and 27.91 seconds locally; these are single-run observations, not benchmarks.

The [compact audit](results.json) records all stops and counts. Full receipts,
unchanged PNG copies, base/augmented reader outputs and source snapshots are in
`test-results/runtime-appearances-v1`. All 149 frozen source files and 36 inputs
were unchanged at audit time.

- Source manifest SHA-256: `3905054fcb6c08f6e64317c5ac4d5fff918ff167b8a058e95380fc8f72397252`
- Input manifest SHA-256: `47f823ca5095ef75d7ca9b530eeeca1d961b975e90f880eaef18c3c55d9b2dd4`
- Model digest: `22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643`

This closes a missing automatic observation path. It is not a live native test,
withheld-game generalization, verified control or completed goal. The desktop
preview remains unavailable after the previously recorded quarantine.
