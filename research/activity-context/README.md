# Activity context for metadata questions

Reversebolt now uses retained play observations to inform its local metadata
review. The selector requires two separated, single-interval episodes of the
same sampled key, bounded by quiet intervals, with image differences exceeding
both 5 grayscale levels and five times the quiet median. It chooses one key by
response contrast and keeps its first two eligible episodes. Missing evidence,
sustained activity and insufficient contrast do not supply an image context.

The local model sees four images as two before/after pairs before receiving
internal metadata. It is not told the key name or game title. Its output describes
visible appearance and arrangement, with explicit uncertainty. Only a consistent
response conditions the existing type-selection and field-question prompts.
Abstention and failed image interpretation fall back to the ordinary metadata
review while retaining the original outcome. No model proposal renames a card,
establishes an effect or grants permission to write.

The archive includes the complete frame references and sampled input windows,
four copied PNGs, image/input hashes, capture clocks, selection measurements,
model identity and exact prompts/responses. Shared status exposes the accepted
context. A prepared response test retains it with the original hypothesis and
alternatives, including after restart; old records without context remain valid.

The [frozen protocol](PROTOCOL.md) uses the already explored Manifold Garden
save in Anode. The evaluator supplies four known short movement inputs; those
inputs are not learned by the product. It never edits game values. Any prepared
comparison is stopped without collecting comparison frames, then checked after
an engine restart. This evaluates selection and evidence retention, not a
verified gameplay control or independent-game generalization.

`freeze.py` captures source/binary hashes and backs up game storage. `run.py`
uses the real shared MCP and automatic attach. `audit.py` checks retained
artifacts without further game access. The audit is authored after execution;
it does not change the frozen input plan, product or selection thresholds.

## Retained result, 2026-10-21

The frozen `test-results/activity-context-v1` run passed eight integration checks.
Automatic attach captured fifteen moments, with four evaluator holds and no
manual review of measurement images. Four menu/setup screenshots were reviewed.
The selector chose the two D-associated intervals (captures 2→3 and 8→9), whose
mean grayscale differences were 5.566 and 5.567 against a quiet median of 0.072.

The installed model described a subtle change in scene perspective, explicitly
allowing camera drift, rendering artifacts or other inputs as explanations.
That exact description conditioned both metadata stages. Twelve cited questions
were accepted, clustered around vertical camera settings. The frozen first-eligible
policy chose `sensitivityY`; its proposed test was prepared, stopped and reopened
with the exact context, hypothesis and references preserved. There were no
comparison captures, gameplay writes or active holds after restart.

This establishes the evidence handoff, **not that the chosen camera setting
explains the sampled movement response**. The meaning and usefulness of the
model's choices remain unverified. No prompt tuning or repeat inputs followed
the result, and there is no new generalization or semantic-control credit.

Both helpers completed and unloaded. They reported 111 pauses, maximum 5.187 ms,
with no helper heap operations while paused. These are helper measurements, not
an independent pause oracle. All owned evaluator processes and the game exited;
Anode was released. User saves 1/2 and their screenshots are unchanged. The test
save, test thumbnail and game logs changed normally; no storage rollback occurred.

Validation: 271 workspace tests, strict Clippy, formatting, release build,
52-tool shared-engine smoke and the post-run [artifact audit](results.json).
Preview `0.1.0-20261021-224455` is running with the four frozen binary hashes.
All 44 desktop history entries, model settings, changes, tests, trials and Freeze
records survived the update.
