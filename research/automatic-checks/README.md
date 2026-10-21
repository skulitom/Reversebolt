# Automatic prospective checks after connection

Verified on 2026-10-21 with preview `0.1.0-20261021-081604`.
The default observation job can now choose one numeric finding, freeze its
memory predictions and challenge them with later observations. No manual
capture, comparison or prediction-check command is required, and no trial
preparation or memory write happens automatically.

The shared engine chooses only a currently readable OCR series with matches
when at least four captures remain in both the job and batch budgets. It prefers
fewer separate overlapping location groups, more readable moments, fewer
interpretations, then track ID. This is an investigation-cost policy, not an
estimate of user importance or causal authority. Names never affect selection.
The ranked alternatives and rationale are persisted with the prediction check
before later captures, and are visible through MCP and Inspect. Main cards
briefly show Checking; trial editors require fresh eligible evidence.

Existing explicit checks remain unchanged. Manual attachment and comparison
remain available. The same pause/stop controls and budgets govern automatic
checks. One series per batch is supported; other cards are not simultaneously
validated, and manually selecting another series currently requires a fresh
batch/investigation. Jobs serialized before this feature do not gain an
automatic check retroactively.

## Frozen native evaluation

[Method](PROTOCOL.md), [configuration](protocol-v1.json), exact changed source
snapshots, binaries and runners were frozen before testing. Configuration hash:
`c6d15ff9a08f85bae0350de9841ce777850b8f53397ca0524a0e51813406e428`.

| Case | Automatic outcome | Fresh evidence | Verified gameplay control |
|---|---|---|---|
| Owned two-location fixture | Selected Quantity at moment 12 | Two candidates agreed at all four later hold/change observations | None; controlled fixture only |
| Original 2048 | No qualifying memory matches | No prospective check started | None |
| Original Javascript Tetris | Selected the score reading at moment 12; nine candidates frozen | Four contradicted; five need further contrast | None |

The fixture had sixteen exact expected readings and eight fresh agreements.
Both indistinguishable copies remained supported; the engine did not invent a
unique control. Its command log contains attachment and status polling, with no
manual capture/check/edit calls, and its change journal remained empty. The job
stopped at sixteen moments.

Tetris's fresh score was **90, 100, 110, 130**. Five mappings agreed throughout,
but the sequence had no hold, so none qualified for a trial. Across all nine
frozen mappings there were 24 agreements and 12 contradictions. All four fresh
score images were independently scorable and correct; the earlier unscorable
score image was moment 10. The selection only had one eligible numeric series,
so this native run does not establish which of several gameplay findings is
most useful. The controlled ranking test covers that algorithmic choice.

Independent OCR scoring, out of sixteen captures per counter:

| Game/counter | Correct | Unscorable |
|---|---:|---:|
| 2048 score | 8 | 8 |
| 2048 best | 8 | 8 |
| Tetris score | 15 | 1 |
| Tetris rows | 16 | 0 |

Unscorable moments retain their timing/geometry explanations and are not
converted into successes. Tetris rows stayed zero. The input audit matched all
194 injected 2048 key presses and 142 Tetris presses to trusted browser events
in order. Original game sources and all stored image hashes were checked.
Renderer selection used the frozen evaluation CPU heuristic; product selection
of the correct child process remains unsolved. Source/oracle layouts never
entered Reversebolt. These games are already exposed development cases, not a
withheld generalization evaluation.

All three jobs exited zero. Owned fixture/browser processes exited and the
Anode lease was released. There was no manual screenshot review or game input.

## Evidence and limits

[Compact audit receipt](evidence/v1.json) preserves selected hypotheses, ranking,
fresh byte/image/read witnesses, replayed assessments, input counts and hashes.
Raw outputs remain under `test-results/automatic-checks-v1/`. Run the unchanged
`score.py` on a game directory to reproduce scoring, then run `verify.py` to
audit all three cases. The audit was written after execution; it is not the
preregistered method. Initial training memory is not retained, so the original
fit cannot be replayed in full.

All **149 workspace tests**, strict Clippy, release build and packaged checks
passed. Tests cover automatic freezing before future observations, immutable
predictions, absence of a sufficient future budget, preservation of explicit
checks, overlapping ranges, missing/inactive findings, deterministic ranking
independent of labels/input order, and old-record deserialization. Logs are
`test-results/automatic-check-{tests,clippy,build,smoke,audit}.log`; packaged smoke
receipt: `test-results/smoke-4984041b/report.json`. The current desktop was updated
with all eight history entries and its local-model preference preserved; see
`test-results/automatic-check-desktop-delivery.json`.

Overall progress remains **27%** against the fixed full vision. This connects
existing discovery and validation steps automatically; it does not add verified
gameplay semantics or independent-game acceptance. Useful-object prioritization,
checking multiple findings, deliberate hold/change experiments, controlling-state
identification, hidden effects and reusable recovered controls remain unfinished.
