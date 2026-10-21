# Process scope and repeated evidence

Four automated sessions on the original 2048 and Javascript Tetris sources
tested child-renderer memory with 12 initial observations and four fresh checks.
The first pair exposed a matcher flaw: one isolated FPS reading admitted 200
hypotheses despite a constant later interval. The updated generic matcher now
requires repeated contrast in both halves and in the observations each mapping
actually explains. No game names, layouts or gameplay labels enter the matcher.

## Frozen method

[V1 protocol](PROTOCOL.md) and [configuration](protocol-v1.json) precede the
first captures; [V2 protocol](PROTOCOL-V2.md) and
[configuration](protocol-v2.json) precede the follow-up. V1 uses product commit
`350b9b4`. V2 adds only the repeated-contrast product change, whose exact source
and packaged binaries are hashed in its configuration. The two runners differ
only in the protocol filename. Original source hashes and fixed native input
schedules are inherited from the [independent-game baseline](../independent-games/README.md).
The [archived matcher source](evidence/discovery-v2.rs) preserves those exact
hashed bytes so later product changes do not invalidate the historical audit.

Each session used a fresh browser profile, optional local image interpretation
disabled, and the ordinary MCP API with a 16 MiB memory budget. A three-second
CPU measurement selected the most active accessible renderer, with a PID tie
break. This browser-specific evaluation rule is not automatic product selection
or evidence of semantic ownership. The separate oracle was used only for
scoring after capture; it was never sent to Reversebolt. Instrumentation can
still allocate copies inside the selected process.

The lowest-ID active OCR series with matches was frozen after frame 12. Frames
13–16 were collected without comparison or refitting. There were no writes,
retries, manual gameplay or manual screenshot inspections. Every native pulse
has a matching ordered, trusted browser key-down/key-up pair: V1 176/144 pulses
and V2 182/141 pulses for 2048/Tetris respectively. All four owned jobs exited
successfully and closed their processes.

## Results

| Run | Initial hypotheses | Frozen fresh-check outcome |
| --- | --- | --- |
| V1 2048 | 0 | No hypothesis; four fresh observations still collected |
| V1 Tetris | 6 score, 200 FPS | All six frozen score mappings agreed at all four frames |
| V2 2048 | 7 for a tile-position number; none for score/best | All seven contradicted; four also invalidated by changed/unavailable mappings |
| V2 Tetris | 7 score, 0 FPS | Three agreed at all four frames; four contradicted |

The V1 score's fresh observations were 120, 130, 140, 140, all independently
scorable and correct. V2's were 90, 100, 120, 120. The last V2 image overlaps a
score change and is excluded by the independent scorer; its apparent hold is
therefore not independently verified. Two V2 mappings predicted 130 at that
boundary while OCR read 120. The product treats this as a contradiction; the
evaluation cannot establish which value was right throughout that capture.
The three remaining mappings encode the same displayed trajectory at separate
addresses. No result establishes which memory, if any, controls gameplay.

The V2 FPS observations were constant, so that fresh run cannot isolate the
new rule's effect. The retained V1 trajectory (eleven 66s and one 63) fails the
new observation-eligibility requirement. A Rust regression uses synthetic bytes
with that exact trajectory. Raw initial memory was discarded, so this is not a
full replay of V1 memory fitting. The fresh sessions are unpaired, exposed
development runs, not withheld acceptance or an improvement-rate estimate.

Selected-counter OCR scores out of 16 captures each:

| Run | Score | Best / rows |
| --- | --- | --- |
| V1 2048 | 7 correct, 5 abstentions, 4 unscorable | Best: 12 correct, 4 unscorable |
| V1 Tetris | 14 correct, 2 unscorable | Rows: 16 correct |
| V2 2048 | 5 correct, 1 conflicting, 10 unscorable | Best: 6 correct, 10 unscorable |
| V2 Tetris | 14 correct, 2 unscorable | Rows: 16 correct |

Unscorable includes changing values/geometry and the frozen timing exclusions;
incorrect-looking excluded readings remain retained, never counted correct.
Rows stayed zero, and fresh 2048 score/best values coincide. These are not
independent changing-control successes. The 2048 tile series was outside the
predeclared score/best oracle, so its fresh disagreements are with OCR only.

Complete 12-frame memory trajectories covered approximately 11.7/13.1 MiB in
V1 and 11.8/13.2 MiB in V2 (2048/Tetris), out of roughly 36–38 MiB eligible.
The current plan includes 4.7 MiB of image memory and favors small regions;
the largest selected private regions were 156–268 KiB. This exposes a coverage
limitation but does not establish where the missing 2048 score state lives.
Later captures clear live match lists; the retained frozen check, not the final
live match count of zero, is the source of prospective outcomes.

## Change and verification

The existing tolerance remains 0.55 and inlier ratio remains at least 80%.
Each half must now have at least two low and two high explained readings, with
the second-highest minus second-lowest greater than 2.2. Rejected outliers
cannot supply that contrast. This can delay discovery after a rare single event.
It does not solve repeated coincidental correlations, OCR errors, asynchronous
image/memory capture, display copies, or choosing the controlling address.

All 140 workspace tests, strict Clippy, formatting and release build pass.
Regressions cover the saved FPS pattern, insufficient contrast in either half,
contrast supplied only by rejected observations, repeated levels, changing
scores, inverse scaling, outliers and ambiguous copies. The packaged 30-tool
MCP/practice smoke passes, including guarded edit/restore on the owned practice
fixture. That fixture result is separate from the game evaluations.

Preview `0.1.0-20261021-071232` is running on the user's desktop with eight
history entries and the selected model preserved. Binary and model hashes match
the published manifests. Overall progress stays **26%** against the fixed full
vision. Next work should address time alignment and useful memory coverage,
then distinguish controlling state from copies through bounded investigations.

## Reproduce and audit

Raw evidence is under `test-results/process-scope-study-v1/` and `-v2/`.
Compact, token-free [V1](evidence/v1.json) and [V2](evidence/v2.json) receipts
include source/process identities, trajectories, fresh byte witnesses, mapping
coefficients, coverage and artifact hashes. They preserve failures as well as
agreements. No training-byte witnesses remained in the archived batches.

Run `python research/process-scope-study/verify.py v1` (or `v2`) to audit saved
artifacts. It checks original source/method hashes, CPU selection, ordered input
delivery, capture hashes, immutable frozen mappings, fresh numeric predictions
and the existing product assessment rule. It does not rerun the missing initial
memory fit. `score.py <run-directory>` recomputes the fixed independent OCR
scoring. Raw oracle data are required for that computation.

To repeat, first freeze a new protocol and binary configuration, use a new
output directory, then run the corresponding Node runner through a leased
Anode seat with the runtime/session variables documented in the runner. Never
overwrite these runs or edit the frozen V1/V2 method files. Build, test and
delivery logs are named `test-results/contrast-*`; the packaged smoke receipt
is `test-results/smoke-6fcd1a86/report.json`.
