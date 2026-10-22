# Independent native discovery and an ambiguous response

The unchanged published product completed automatic discovery in Hyperbolica
(Steam 1256230, installed build 8675290). It found two cited hypotheses:
`walking speed` from `walkingSpeed = 2.25`, and `shake strength` from
`shakeStrength = 0`. Each retained an alternative field. These are hypotheses,
not validated controls. The original planner refused a comparison because the
image model marked the visible response ambiguous. **No gameplay field was edited.**

## Frozen native evaluation

[V1](PROTOCOL.md) first selected installed Freight Hopper. Its ordinary Play
action automatically created a Steam P2P lobby and the level screen displayed
a leaderboard. The offline boundary was not established, so setup stopped before
any runtime helper, model call, gameplay input or edit. Four setup images were
viewed. Normal menu/Steam storage changed; its pre-launch backup remains local.

[V2](PROTOCOL-v2.md) selected Hyperbolica after checking its official
[single-player feature listing](https://store.steampowered.com/app/1256230/Hyperbolica/).
The same four product binaries and every Rust source hash were retained.
No discovery prompt, budget or field-selection policy changed. Continue loaded
the existing save; three setup images were viewed. No measurement image was
manually reviewed. The evaluator supplied four 200 ms D/A/D/A holds during the
automatic fifteen-capture sequence, using observed foreground identity and
released-key guards. Those keys were not assigned gameplay meanings.

One runtime helper completed and unloaded. The bounded inventory retained 77
types, 327 fields and 8,729 reachable paths. Paths may alias and the graph was
incomplete despite root-scan completion. Six metadata model calls produced two
accepted, zero rejected questions. A seventh, separate image call described
subtle changes but set `consistent_visible_change: false`. The original review
therefore had no activity context and the planner refused. There were zero
comparisons, writes or freezes. The original `game.save` hash is unchanged;
normal logs and analytics files were added. All owned test processes exited
normally and Anode's lease was released.

The [native artifact audit](results.json) retains this refusal. It verifies
frozen sources, retained model responses/citations, image hashes, input records,
helper unload, pause accounting and saved-game identity. It does not prove
semantic correctness or independent-game acceptance.

## Revised offline planning

The refusal exposed a distinction: deciding what an image change means and
deciding whether there is a repeatable contrast worth testing are different
decisions. The [revised policy](MEASURED-CONTRAST.md) retains the model's original
ambiguity and adds an explicit `basis: measured_contrast_v1` when the existing
quantitative source checks pass. A positive interpretation keeps its legacy
serialization. Unknown policies, changed evidence, missing input, weak contrast,
failed reviews and malformed contexts remain rejected.

A separately frozen, offline Rust replay rehashed all fifteen original images,
recomputed the production image metric and rebuilt the activity source. With the
same metadata questions and unchanged false image verdict, it produced one
proposed walking-speed comparison and four 198 ms holds derived from sampled
timing. No model call, target access, input or write occurred. The model's
description and uncertainty remain verbatim in the plan and test provenance.
[Replay results](replay-results.json).

This is a known development regression, not a new native trial or evidence that
walking speed was controlled. The proposed comparison has **not** been executed.
All 293 workspace tests and strict all-target Clippy pass. Tests retain the
legacy rejection of an unversioned false interpretation, verify explicit basis
serialization, and exercise missing input, changed evidence and quiet drift.

## Later Windows detection

The initial cleanup check found no new detection. Later, Windows quarantined
`dist/0.1.0-20261022-010125/Reversebolt.exe` as
`Behavior:Win32/DefenseEvasion.A!ml` and closed the user's idle desktop preview.
The initial observation and later detection are both retained. The exact trigger
is not established. The executable's pre-run hash is frozen; its published copy
cannot now be rehashed, and Windows also refuses reading the build-output copy.
No restoration, exclusion, protection change, desktop republish or external
submission followed. See the [review follow-up](../security-review/FOLLOWUP-2026-10-22.md).

The planning source improvement is unpublished. Progress remains **35%** and
the full goal is active. Native validation needs the Windows detection resolved;
neither the saved-report replay nor the accepted names substitute for that.

Local evidence: `test-results/independent-runtime-v1`,
`test-results/independent-runtime-v2`, and `test-results/measured-contrast-v1`.
The V2 test explicitly reused V1's previously empty editor data directory; the
frozen manifest records that path. Re-run `audit.py` and `audit-replay.py` here
to check saved evidence without invoking a model or game.
