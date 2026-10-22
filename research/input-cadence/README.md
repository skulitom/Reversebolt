# Capture timing can manufacture apparent quiet intervals

The faster observation phase exposed a correctness problem in passive input
classification. The classifier considered key samples between the preceding
capture's end and the following capture's start, ignoring samples during either
image operation. It could therefore report `no_keys_detected` even when a key
was sampled while an endpoint image was being acquired. Image operations have
duration; their returned pixels are not known to represent either endpoint.

The regression deliberately changed an existing test that expected this false
quiet classification. It failed on the old implementation, then passed after
the fix. Both endpoint image operations now require available, well-formed
sampled-input coverage. Pressed keys overlapping either operation, lost focus,
omitted/restarted coverage, malformed polls and capture gaps exceeding 100 ms
make the interval unknown. The original inter-image classifier and its minimum
sample rules then run. Unknown intervals cannot supply quiet neighbors for
activity selection. Raw input and image metadata are unchanged.

These checks still concern sampled states. Short taps between polls may be
missed; an accepted interval does not prove input delivery, physical stillness,
gameplay meaning or causation. Active comparison witnesses already have separate
release/bracketing/settling requirements, which were not relaxed.

## Frozen timing study

The [protocol](PROTOCOL.md), [runner](run.py) and Rust timing-only example were
fixed before the first run. All four original browser recordings supplied their
retained input metadata. The input protocol specifies **80 ms presses every
350 ms**, correcting the earlier replay report's wording. No pixels, game source,
oracle or delivered-event data entered this study.

The original capture operations had upper medians of 199,102 / 211,074 / 200,796 /
207,167 microseconds across the four runs. The sensitivity grid covers hypothetical
100/200/400 ms capture costs, 50/100/250/500 ms post-capture delays, fourteen initial
offsets and fifteen captures, applied to each original poll stream: 672 scenarios,
9,408 classified intervals per version. These are hypothetical image-operation
bounds over existing inputs, not new captures or measured throughput.

| Measure | Before | After |
| --- | ---: | ---: |
| Original intervals accepted despite a sampled key during capture | 4 / 60 | 0 / 60 |
| Original intervals classified unknown | 56 / 60 | 60 / 60 |
| Hypothetical intervals accepted despite a sampled key during capture | 7,074 / 9,408 | 0 / 9,408 |
| Hypothetical scenarios with repeated same-key timing opportunities | 198 / 672 | 0 / 672 |

The timing opportunities count isolated single-key classifications with quiet
neighbors, repeated for one key. They contain no measured image response and
were never submitted as activity evidence or gameplay findings. The original
four accepted intervals were the two single-key intervals in each Tetris run.
They did not previously produce bounded episodes or a selected activity source.
First rejection reasons can change when an earlier gate rejects; the reasons
are not mutually exclusive causes.

All 171 frozen files in each version pass the post-run audit. The four source
statuses and the study method are identical between runs. Original unique poll
counts are 3,122 / 2,765 / 3,107 / 2,744, within the production buffer's 4,096-poll
limit. No retained source window reported omitted polls. The shared buffer
constructs hypothetical windows without filling missing observations. Complete
outputs, binaries and manifests remain in `test-results/input-cadence-v1`;
[compact results](results.json) retain the grid and artifact hashes.

## Consequences and verification

This study does not validate the new fast phase. Across the declared grid, a
shorter delay alone does not provide the current selector's repeated isolated
input opportunities on these dense traces. Better acquisition timestamps for
the actual selected image, or temporal inference that represents mixed inputs
and timing uncertainty, needs investigation. Do not choose a new cadence from
this hypothetical grid, weaken the ambiguity checks or invent intermediate
images. The mixed capture phase remains available for new measured evaluation;
the full numeric/memory sequence is preserved.

All **386 workspace test executions**, strict workspace/all-target Clippy and
format checks pass. The new regression covers both image operations, pressed keys, focus loss,
coverage loss, retained-history bounds and malformed samples. Existing pattern,
regional selection, planning and comparison tests are included in the workspace
check. Verification logs are retained alongside the study; these checks establish
software behavior, not independent generalization. No game, model, native target
access, screenshot or input delivery ran for this study.

Overall progress remains **35%**. The desktop preview remains unavailable after
quarantine, and all executable/security-review material stays local.
