# Current-state selection in a native game

The fresh Sable development run found two scalar paths that passed the existing
four-repeat input-association rule. This establishes repeated associations in
one running process, not semantic controls, successful edits or generalization.
No gameplay field was written and nothing was promoted to the desktop UI.

The [protocol](PROTOCOL.md) was frozen before launch, together with exact sources,
binaries and model identity. Artifacts are retained locally under
`test-results/runtime-state-activity-v1`. The unchanged `current_state` selector
made 22 requests, and the unchanged enclosing-role sampler chose 128 paths across
12 roles, including 23 embedded paths. Another 269 eligible fields were omitted.
One discovery page covered 3,712 of 4,173 roots; 461 roots remain unscanned.

The first eight of ten planned batches measured 106 paths, including 21 embedded
paths and 89 distinct initial addresses. All 1,484 scalar readings succeeded.
Eight paths changed, 98 remained constant, and two passed repeated association:

| Runtime label | Quiet value | First D / A | Second D / A |
| --- | --- | --- | --- |
| `_slopeAnimAmount` | 0 | -1.6317046 / 1.562001 | -1.6315935 / 1.6316304 |
| `_oldFootCycle` | 1 | -0.060619786 / 0.48373395 | 0.2343135 / 0.6002136 |

Both paths had stable locations and all four quiet pairs were exactly stable.
Their runtime names suggest animation-related state, but those names are clues:
no actor identity, field meaning, direction, physical movement or useful edit has
been verified. The two addresses are distinct and process-local. The process has
exited; its paths must not be reused against another launch.

## Independent rendered evidence

The published shared engine captured 48 images automatically, six per batch.
Only three setup screenshots were viewed manually and two menu keys were sent.
No measurement image was manually viewed. Captures followed native samples and
key release; they were not atomic with the memory samples. There are independent
image pairs for the first D/A holds only, not the second repetitions.

Every recorded action interval had more rendered difference than its adjoining
quiet intervals. For the two positive batches, the existing grayscale difference
metric (0–255) produced:

| Batch | Before input | D | After D | A | After A |
| --- | --- | --- | --- | --- | --- |
| 4, slope-labelled path | 0.255 | 8.273 | 1.067 | 7.164 | 2.704 |
| 5, foot-cycle-labelled path | 0.045 | 6.468 | 0.033 | 6.529 | 2.646 |

These are descriptive comparisons, without duration normalization. Action image
intervals were approximately 5.05–5.68 seconds, whereas the before/after-A quiet
intervals were approximately 3.00–3.68 seconds and after-D intervals 5.91–6.59
seconds in these two batches. The audit retains the exact interval bounds.
Some change continued after release. None of the sixteen action image pairs had
distributed patch-motion consensus, so the rendered evidence cannot identify
actor displacement or establish that either field caused the visual change.

Direct capture has no product-owned input monitor. Product input exposure remains
`unknown`, and activity_source remains null. The diagnostic uses its separately
recorded external sequence and foreground/key coverage; it does not replace those
product classifications. Inter-batch image pairs are retained but excluded from
the within-batch comparisons above.

## Verification and limits

All eight native analyses replay exactly from saved observations. The image hashes,
compositor clock checks and all 47 adjacent rendered differences pass the offline
audit. All 113 native helpers completed and unloaded. Across 485 observed pauses,
the maximum stop-to-resume interval was 5,653 microseconds, with zero helper Rust
heap allocations or deallocations while paused. All 32 SendInput holds recorded
accepted down/up transitions and passed the existing input-coverage checks.
Their QPC brackets lasted 5,000.401–5,001.045 milliseconds: the five-second
performance-counter deadline still has a small scheduling/API overhead. The
protocol's nominal bound is not an exact upper bound on those measured brackets.
The largest model request was 11,944 bytes, within the frozen 12,000-byte limit.

This is not a controlled comparison of selection prompts: the catalog, run,
input format and independent image evidence also differ from the earlier test.
The omitted roots, fields and two unmeasured batches remain unknown. Constant
parameters may still be useful controls; inactivity does not disqualify them.

The next investigation should use repeated state activity as evidence for locating
relevant objects, then test parameter hypotheses with bounded interventions and
measured outcomes. It must reacquire fresh paths, retain alternative explanations
and distinguish an effect on animation from an effect on gameplay. These results
alone do not justify adding a named editable control.

The game, runner, MCP child and headless editor exited; Anode was released. The
main desktop editor is still responsive. The pre-launch storage backup is intact.
The main save, its image, settings and cloud metadata are unchanged; logs and
SaveManager changed. No upload, security-setting change or desktop replacement
was performed. Full-goal progress remains **36/100**.

This checkpoint adds research only. The existing 475 passing all-feature workspace
tests, strict Clippy and formatting results belong to unchanged product code.
The new Python sources pass syntax parsing. Replay without game or model access:

```powershell
python research/runtime-state-activity/audit.py C:/DEV/Reversebolt/test-results/runtime-state-activity-v1
```

The audit uses the existing `target/release/examples/response_audit.exe` and records
its hash. `audit.py` was authored after measurement; the frozen protocol, plan,
sources and original artifacts remain unchanged.
