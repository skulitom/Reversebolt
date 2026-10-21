# Search by changing observations — 2026-10-21

Reversebolt's **Follow changes** card action can select memory areas whose
numbers follow the display with a different scale or offset. It no longer needs
a literal encoding of the displayed value to nominate those areas. The same
workflow is exposed through MCP, with automatic four-moment selection followed
by sixteen wholly fresh fitting/checking observations.

This is original Rust implementation, using no game names, offsets, layouts,
known field types, or per-game rules. No Squalr code was imported for this change.
It complements **Find this value**, which remains the cheaper literal search.

## Native evidence

The [frozen protocol](PROTOCOL.md) and [manifest](protocol-v1.json) specify two
owned native-window cases. Both allocate a 32 MiB arena, place an F32 field at a
random aligned location in its upper half, and display twice its stored value.
The evaluator selects the window normally, changes the fixture through its own
control file, and uses a separate oracle only to score results. The ordinary
memory budget is one MiB. Neither addresses nor representation hints reach
Reversebolt, and no memory writes are requested.

| Case | Result |
|---|---|
| Changing encoded value | Passed: field outside the initial sample selected, fitted from 12 new moments, then four fresh agreements |
| Constant value | Passed: explicit insufficient-contrast abstention, no selected areas, no fresh batch/job or prediction check |

The positive case planned **103.758 MiB per selection observation**. Of this,
**103.629 MiB** was complete and mapping-consistent across all four moments.
Selection took **1,096 ms**, retained **six 64-byte areas (384 bytes per fresh
snapshot)**, and omitted none because of the window cap. These are measurements
of one local run, not performance guarantees.

The actual field was **24.950 MiB** into the arena. Its saved selection bytes
independently decode to half the observed sequence 20,80,30,70. Fresh discovery
froze one F32 mapping with slope 2, intercept 0, six training and six held-out
inliers. The four later readings 54,54,50,50 all agreed. Old and selection frame
identities are disjoint from those fresh fitting/checking frames. The complete
investigation requested 435,841,040 memory bytes within its 4 GiB allowance.

The constant case collected four readings of 20 and refused selection before
examining numeric trajectories. Its selection `complete_bytes=0` means that
comparison was not run; it does not mean no memory was read. The investigation
requested 432,451,584 bytes and kept the four observations, without borrowing
contrast from the earlier source card's reading of 50.

The product, rather than the evaluator, performed the selection-to-validation
handoff. The evaluator called neither finish_change_search nor any fresh-job
submission. Both cases completed on their first frozen runs. All **26 PNG hashes**
match their saved frame identities; no screenshot was manually viewed.

## Selection and control semantics

Four complete snapshots can nominate naturally aligned little-endian i32/f32/f64
locations through affine relationships. All four numeric observations must be
readable in the original region, with repeated contrast, unchanged image size,
and no geometry epoch change. Missing, partial, or mapping-changed chunks do not
contribute. This selects addresses only: the four points are not a validated
mapping, causal experiment, semantic label, or permission to write.

The search uses at most 128 MiB per selection moment and 512 MiB of raw snapshot
retention. It retains at most 4096 64-byte areas through deterministic address
sampling, saving omissions and raw selection witnesses. Eight searches share
the existing 24-batch, 4 GiB requested-read and archive/image budgets. A constant
or ambiguous series abstains. Persistence failure preserves the current batch.
After selection, the old snapshots are released and all fitting/checking starts
from new observations; selection bytes are never reused as fresh evidence.

Automatic collection respects job pause, cancellation, failure, scope changes,
and human takeover. A human stop/pause while the read-only selection worker is
settling takes control and suppresses the later job. A late Stop on its completed
parent stops the fresh child job. Agent envelopes are not rewritten, and rejected
agent takeover cannot remove a pending continuation. Restart never resumes work.
The selection computation itself settles cooperatively; this adds no preemption
of an in-flight native call. Explicit agent scheduling and completion remain
available through MCP, and a stopped four-moment selection can be completed
under application Inspect.

Four-point affine selection can retain coincidences and miss nonlinear,
unaligned, moving, unavailable, unvisited or omitted storage. Window/mapping
checks do not prove allocation identity or atomic screen/memory correspondence.
The later tests remain necessary. This experiment identifies a synthetic field;
it establishes no independent-game gameplay control, hidden semantics, or
restoration. The earlier Tetris and 2048 results remain unchanged.

## Verification and desktop delivery

All **184 workspace tests**, strict Clippy, formatting, release build, and the
**35-tool packaged smoke** pass. Five new core tests cover encoded-value recovery,
fresh-evidence separation, failed/changed reads, geometry/missing values,
nonlinear and insufficient-contrast negatives, stale scopes/budgets/persistence,
and deterministic bounded sampling. Three controller tests cover the complete
handoff, cancellation during selection, rejected takeover, and a late parent
Stop reaching the child job. The latter exposed and fixed a handoff cancellation
gap before the native protocol was frozen.

The [post-run audit](audit.json) checks hashes, saved raw field witnesses,
selection/fresh frame separation, no evaluator-triggered handoff, budgets,
all hypotheses, and job outcomes. It is labeled post-run and does not select
product findings. [Execution receipts](execution.json) retain job IDs, cleanup,
desktop delivery and the packaged smoke path. Raw local data is under
`test-results/change-search-v1/`. [product.patch](product.patch) records the exact
production diff from the manifest's base commit, including new source files.

Preview **0.1.0-20261021-114033** is running on the user's desktop, PID 62456 in
session 1. The previously selected game had exited, with no active capture,
analysis or edit. Its disconnected evidence was saved before the graceful
restart. All **29 history entries** and the `qwen3.8:latest` setting were preserved.
The desktop launcher points to the new build. All six owned native-test
processes exited and Anode's lease was released. Its viewer was already visible
when this run acquired the seat and was left unchanged.

Progress is **31%**: general discovery moves from 7/20 to 8/20 for bounded
change-based broad selection with a verified native automatic handoff and fresh
encoded-field evidence. The other weighted credits are unchanged, including
independent-game acceptance at 0/10. The full goal remains active.
