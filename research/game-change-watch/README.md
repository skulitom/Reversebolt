# Watching independent games through updates — 2026-10-21

The fixed two-game evaluation completed without runner errors. Original 2048
completed discovery and a three-moment trial watch; its field evolved during
ordinary moves and guarded restoration refused to overwrite newer bytes.
Javascript Tetris abstained during broad selection and made no edit. These
outcomes identify limitations, not general-purpose gameplay controls. Progress
remains **31%**; production code and the running desktop preview are unchanged.

The [protocol](PROTOCOL.md), [manifest](protocol-v1.json), methods and product
122703 binaries were frozen before either launch. Original upstream games and
MIT provenance remain in the independent-games study. No game-specific layout,
address, representation or value entered discovery. The runner used Anode,
fresh profiles, fixed native inputs and an external DOM counter oracle. No
manual screenshot review or gameplay occurred. Neither case was replaced.

## 2048: the tested field continued evolving

The game completed 16 initial, four selection and 16 fresh observations.
Selection readings were 368, 404, 516, 520. The broad pass planned 22,319,104 bytes
per moment and compared 20,176,896 complete bytes, retaining four 64-byte areas.
Two OCR checks each retained two candidates with all four fresh agreements;
score and best remained competing meanings. The prescribed lowest qualifying
candidate was an I32 interpretation with learned scale 0.5 and zero offset.

A one-byte trial requested 1232 → 1242. Immediate readback confirmed the write;
the display remained 1232. The first paced watch still read 1232. Four ordinary
moves then preceded a reading of 1254; four more preceded 1318. Both score and
best were independently stable at these respective values over the scored watch
intervals. Thus the early nonresponse was retained alongside later changes.

The selected interpretation's complete bytes at both capture endpoints decoded
to 1242, 1242, 1254 and 1318 across the immediate and three watched moments.
The original requested bytes did not persist after movement. Reversebolt
reported `memory_changed` and refused the single restoration attempt with no
write. The refusal's automatic observation again read 1318 and remained in the
written phase. No restoration watch ran. Closing the disposable browser ended
the test; no whole-game rollback is claimed.

The post-run [continuity analysis](continuity.py), with [retained results](continuity.json),
applies the original mapping to those already-recorded bytes without refitting.
The newer endpoint predictions equal the visible 1254 and 1318 readings. This
supports investigating a continuing relationship rather than interpreting every
changed byte as loss of relevance. It is **post-hoc descriptive analysis**, not
a new product outcome, acceptance gate or proof that the edit caused the score.
Natural scoring and random tiles remain confounders; no paired counterfactual ran.

The complete prepare and apply intervals had stable independent readings.
The restore-attempt RPC was unscorable because of delayed oracle coverage;
this does not erase its authoritative refused-write receipt. Sampled product
input context contained no held keys in trial observations: the evaluator paused
the watch during input bursts, and monitoring gaps remain explicit. Separate
trusted browser events verified all 133 native pulses. Neither sampled context
nor trusted delivery proves that every pulse changed gameplay.

## Tetris: insufficient contrast in a short window

Tetris completed 16 initial and four selection captures. Its selection readings
were **60, 70, 70, 80**. The product's repeated-contrast requirement rejected
this short sequence: the second-highest and second-lowest values were both 70.
The selection planned 28,876,800 bytes but compared none, retained no areas and
scheduled no fresh batch. No candidate, trial or watch was forced. All 69 native
pulses matched trusted browser events. A longer or event-sensitive observation
strategy is an open improvement, not a successful result claimed here.

## Observation accuracy and integrity

| Counter | Correct | Conflicting | Abstention | Unscorable |
|---|---:|---:|---:|---:|
| 2048 score, 36 captures | 34 | 1 | 1 | 0 |
| 2048 best, 36 captures | 35 | 0 | 1 | 0 |
| Tetris score, 20 captures | 19 | 0 | 0 | 1 |
| Tetris rows, 20 captures | 20 | 0 | 0 | 0 |

Rows stayed zero; this is not evidence of changing-value recognition. Scoring
requires stable value, geometry and bounded oracle request gaps, and keeps
ambiguous readings. The six synthetic scoring outcomes, watch-interval exclusions
and owned headless DOM oracle test passed before launch. The [post-run audit](audit.py)
verifies frozen files, disjoint discovery evidence, candidate ordering, job
cursors, write/refusal receipts, **62 source PNG hashes** and **202 native pulses**.
Its initial implementation assumed selection windows were always arrays; it
was corrected to accept the product's documented null arrays for abstention.
This audit repair changed neither the frozen method nor either recorded run.

See [results](results.json), [integrity audit](audit.json) and
[execution receipts](execution.json). Raw evidence remains under
`test-results/game-change-watch-v1/`. All 22 recorded owned processes exited;
Anode was released and its pre-existing viewer state was left unchanged.
The user's desktop remains PID 67560, build 122703, with 29 history entries and
the selected local model preserved. The unchanged product's 189-test, strict
Clippy and 36-tool smoke validation was not needlessly rerun.

## Consequence for the architecture

These cases reinforce that a visually seeded scalar correspondence is one
investigation channel. Hidden values need discovery independent of visible
numbers. Future work prioritizes reusable runtime structure, memory behaviour,
objects and events, with visual evidence for context and consequence checks.
Runtime names are hypotheses, and changing memory is not automatically a failed
edit. The complete goal and all milestone credits remain intact. See
[the architecture direction](../../docs/DISCOVERY_DIRECTION.md).
