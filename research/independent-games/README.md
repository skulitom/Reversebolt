# Independent browser-game baseline

The unchanged desktop engine completed 64 captures across four automatic runs
of two independently authored games. It did **not** discover a verified editable
control. This is a failure baseline for generalization work, not a release gate
passed. Overall progress remains **25%** against the existing full-goal rubric.

## Method and provenance

The product, model digest, original sources, input schedules and scoring rules
were frozen before the first capture in [protocol-v1.json](protocol-v1.json) and
[PROTOCOL.md](PROTOCOL.md). The product was commit `d5623d9`, desktop preview
`0.1.0-20261021-053218`, with the already installed `qwen3.8:latest` local model.
Each run selected its own headed Chromium window through the real MCP API;
attachment started the ordinary 16-capture observation job. No game profile,
address, source-derived layout, DOM value or manually interpreted image was
provided to Reversebolt. No memory writes were attempted.

Two fresh sessions each of [2048](https://github.com/gabrielecirulli/2048) and
[Javascript Tetris](https://github.com/jakesgordon/javascript-tetris) ran in
Anode, alternating games. Both upstream projects are MIT licensed. The pinned
commits, archive hashes and every original file hash are in
[upstream.json](upstream.json). Downloaded source files remained unchanged.
Fresh profiles and fixed native input pulses made execution automatic; the
driver guarded its owned foreground window and the child session. No manual
screenshot review or adaptive gameplay was used.

An evaluation-only reader sampled the games separately. It recorded visible
counter values and bounds, delivered browser key events, and nonnumeric Tetris
piece state. Those readings scored saved product evidence afterward; the
product never received them. Event listeners and CDP reads add overhead and may
allocate value copies, so a memory match would not establish authority.

## Results

Each row below contains 32 counter/frame opportunities over two runs. A result
is unscorable when oracle timing, value stability or geometry fails the frozen
rule. Different readable values within one counter region are conflicting.
Missing readable values are abstentions. The original per-run results and
artifact hashes are in [the compact evidence](evidence/browser-v1.json).

| Game and counter | Correct | Wrong | Conflicting | Abstention | Unscorable |
|---|---:|---:|---:|---:|---:|
| 2048 score | 9 | 1 | 1 | 5 | 16 |
| 2048 best | 12 | 0 | 1 | 3 | 16 |
| Tetris score | 30 | 0 | 0 | 0 | 2 |
| Tetris rows, constant zero | 32 | 0 | 0 | 0 | 0 |

Do not combine the constant-zero row counter with changing-counter accuracy.
The fresh 2048 sessions also displayed equal score and best values; these are
not independent evidence of two controlling fields. Discrete oracle samples
cannot exclude a transient change between polls. The scoring implementation was
written after capture to apply the predeclared rules, not frozen as executable
code beforehand.

All 664 injected pulses had matching trusted browser keydown and keyup events
in the same sequence. Browser receipt does not prove game-accepted meaning.
The engine retained all 64 native captures. Three jobs completed. Tetris run 1
failed after its 16th capture because an optional model response contained a
306-character summary, exceeding the 300-character validator limit. This
product failure was retained without retrying the run or discarding its images.

The Tetris score produced two memory matches at captures 12 and 13 in run 1,
and four at capture 12 in run 2. All disappeared in later captures. Every run
ended with zero numeric matches, zero visual/temporal tracks and zero conditional
rule checks. Tetris retained only learning-stage input patterns. No controlling
memory or downstream effect was verified.

The separate Tetris oracle recorded 218 and 171 distinct current-piece states
(position, direction, color and shape), plus 51 and 31 samples where actual and
displayed scores differed. These show nonnumeric variation and display lag in
the evaluation target; they are **not** product observations or inferred rules.
The evidence field `unique_tiles` for 2048 counts distinct DOM tile observations,
including animations, not distinct semantic board states.

## Failures that determine the next work

1. **One wide text region discards the neural text pass.** In 31 of 32 2048
   captures, the neural reader returned its 1024-pixel recognition-budget error.
   Source inspection confirms `recognize` returns an error for an oversized
   crop, losing any earlier recognized lines from that pass. Windows OCR remains
   available. Isolate unsupported regions, retain explicit partial-coverage
   warnings, and compare saved-image replay before collecting new game runs.
2. **A counter region can contain several meanings.** The wrong 2048 score
   reading was 24 while the stable oracle score was 1192. Its region included
   an animated points increment. This is a failed useful counter interpretation,
   not necessarily a misrecognized glyph. Do not repair it with known score
   bounds, expected values or game-specific digit trimming.
3. **Useful objects are excluded by the current model taxonomy.** All eight
   model responses classified their entities as `other`; none became product
   observations. The prompt explicitly limits useful entities to actors and
   held components. A general tool needs grounded objects and their changes,
   with uncertainty, beyond those two categories. This baseline does not measure
   whether the proposed object boxes or descriptions were correct.
4. **Optional interpretation failure ends an otherwise captured job.** The
   overlong model summary above must remain rejected, but its failure should
   not make unrelated numeric observation unavailable. Separate this failure
   from capture failure while retaining the original response and visible status.
5. **The selected window process is not necessarily the gameplay process.**
   Chromium's own inventory reports separate browser and renderer processes.
   Reversebolt reads only the selected browser process, with a 16 MiB budget
   out of roughly 57–63 MiB eligible there. This is evidence of a process-scope
   limitation, not proof that any particular renderer address controls a score.
6. **Ordinary input cadence exceeds the current relationship method.** Fixed
   350 ms pulses are much denser than full capture cadence. Sparse, bounded
   single-key episodes cannot be assumed. No prospective conditional check
   emerged here. Future experiments must preserve mixed/missing evidence rather
   than retroactively crediting predictions.

These findings call for general engine improvements and fresh independent
evaluation. Replaying this now-exposed corpus is a regression/development check,
not withheld acceptance. No capability credit is added for collecting failures.

## Reproduction and artifact audit

`run.mjs` and `input.py` are the frozen evaluation harness, not product adapters.
The harness requires Windows, the original pinned archives extracted under
`test-results/independent-games-v1/upstream/{2048,tetris}`, the manifest there,
the exact published binaries, local model and Chromium path in the protocol,
and Node Playwright. It must execute inside a leased Anode child session. Its
session guard rejects parent-desktop execution. The environment supplies
`REVERSEBOLT_PLAYWRIGHT`, `REVERSEBOLT_PYTHON`, `REVERSEBOLT_TEST_SESSION`,
`REVERSEBOLT_PARENT_SESSION`, and the current Anode agent/lease credentials.

Invoke `node research/independent-games/run.mjs <game> <replicate> <new-output>`
inside that session. The output directory must not already exist. Preserve all
attempts; a nonzero exit can be a product failure with useful captures. Sources,
profiles, full captures, model responses and per-frame states remain local under
ignored `test-results/`. The compact tracked receipt alone cannot reproduce
pixel scoring without those raw files or a new run.

For each retained run, score with
`python research/independent-games/score.py <output-directory>`. Then run
`python research/independent-games/verify.py` from the repository. The latter
checks pinned source and binary hashes, frozen method hashes, all 64 image
hashes, chronological capture states, model digests and complete input-event
sequences; it regenerates the compact evidence. Scoring never calls the engine.
The verifier is post-run auditing code and has no product role.

The original four runs are `2048-1-attempt-01`, `tetris-1-attempt-01`,
`2048-2-attempt-01`, and `tetris-2-attempt-01`, all beneath
`test-results/independent-games-v1/`. Owned test browsers and engines were closed,
and the Anode lease was released. The user's published desktop remains running.
