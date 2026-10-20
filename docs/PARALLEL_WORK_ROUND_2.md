# Parallel Astra sessions — round 2

20 October 2026. Send one complete prompt below to each of three sessions.
These tasks can run concurrently. The main session retains the shared engine,
desktop, capture/OCR, MCP integration, desktop delivery, and progress record.

The first round produced useful prototypes, but also exposed important limits:
visual tracking has only synthetic evaluation, inference can accept incorrect
rules, and observed recovery cannot prove the absence of hidden side effects.
The main session independently reran all 38 tests across those three packages.
The full benchmarks and production integration remain separate work.

## Session 1 — find values beyond the first few memory regions

> Build and evaluate broader memory observation for Reversebolt. Own only
> `C:\DEV\Reversebolt\experiments\memory-coverage\`. Create a standalone Rust
> package there with its own `[workspace]` and lockfile.
>
> Read `C:\DEV\Reversebolt\docs\VISION.md`, `docs\ARCHITECTURE.md`,
> `docs\PROGRESS.md`, `crates\reversebolt-core\src\live.rs`,
> `crates\reversebolt-core\src\discovery.rs`, and
> `crates\reversebolt-app\src\native.rs`. Read other research as useful.
> Do not edit the root workspace, shared crates, other experiment folders,
> desktop build, progress record, or global settings. Do not stage or commit
> another session's files.
>
> Current limitation: native observation retains a fixed sample of memory,
> generally 16 MiB. Our Half-Life 2 session has roughly 363 MiB of eligible
> writable memory. The current ordering favors writable image regions and
> smaller private regions. An owned WinForms fixture with a 4 MiB budget yielded
> zero memory correspondences. We do not know whether coverage, representation,
> or another issue caused that failure.
>
> Implement a read-only strategy that searches broader memory over time while
> keeping useful trajectories under explicit memory, disk, I/O, and time budgets.
> Consider streaming, sparse/delta retention, bounded spooling, adaptive region
> selection, or a justified combination. Compare against the current fixed
> small-region baseline at equal resource budgets. Do not assume rotating
> samples provide simultaneous or complete trajectories.
>
> Use real external-process reads against disposable child targets that your
> harness owns. Randomize region sizes, allocation order, locations, values,
> supported encodings, distracting copies, allocation churn and relocation.
> Give the solver ordinary process inventory and anonymous observed trajectories;
> do not advertise an arena, address range, allocation tag, field name, layout,
> seed, or ground-truth address. Keep truth in a separate evaluator and inspect
> it only after the solver has emitted results. A synthetic child process is a
> memory benchmark, not an independent game.
>
> Preserve process creation identity and allocation/sampling epochs. Treat
> unreadable, changed, absent and partially read bytes as missing evidence.
> Include address reuse, target exit, budget exhaustion, cancelled scans and
> memory-map changes. Never imply that bytes read at different times form an
> atomic snapshot. Quantify timing skew between observations and memory reads.
>
> Report eligible bytes, attempted/read/retained coverage, region exclusion
> reasons, actual field coverage, candidate recovery, false matches, missed
> fields, peak retained memory, disk use, bytes read, latency and throughput.
> Include negative cases and fresh evaluation seeds after freezing parameters.
> Compare recall and false matches, not just scan speed.
>
> Use conservative default target sizes around 256–768 MiB and a configurable
> observer memory ceiling no greater than 512 MiB. Bound subprocess count and
> disk growth; use larger stress cases only if resources permit. No Anode,
> arbitrary-game reads/writes, injection, drivers, or global settings changes
> are needed. Target programs may update their own state normally.
>
> Deliver runnable Rust code, meaningful tests, raw result artifacts, exact
> reproduction commands and `HANDOFF.md`. Recommend a small integration API
> compatible with NativeTarget/LiveSession, including cancellation and coverage
> metadata. Document source and dependency licenses, limitations and tradeoffs.
> Do not change Reversebolt's overall progress percentage or claim general
> discovery from a memory-coverage improvement alone.

## Session 2 — one durable investigation shared by people and agents

> Build Reversebolt's shared investigation job machinery as an isolated Rust
> prototype. Own only `C:\DEV\Reversebolt\experiments\session-jobs\`.
> Create a standalone package with its own `[workspace]` and lockfile.
>
> Read `C:\DEV\Reversebolt\docs\VISION.md`, `docs\ARCHITECTURE.md`,
> `docs\MCP.md`, `crates\reversebolt-app\src\main.rs`, the shared engine
> under `crates\reversebolt-core\src`, and
> `experiments\causal-validation\HANDOFF.md` plus its API document.
> Do not modify root/shared crates, other experiment folders, the desktop,
> global MCP configuration, or progress record. Do not stage or commit another
> session's files.
>
> The desktop and MCP currently submit commands to the same serial worker.
> Implement the reusable core for longer investigations that both interfaces
> can observe, pause, resume, cancel and continue. Keep one authoritative job
> state and event stream; do not implement a separate MCP server or desktop UI.
> Design a narrow adapter boundary for the existing worker.
>
> Define explicit state transitions and ownership for queued, running, pausing,
> paused, cancelling, completed, failed and interrupted work as appropriate.
> Define what each acknowledgement guarantees. Cancellation must stop future
> effects; report or await an already-started operation rather than pretending
> cancellation undid it. Bound queued work, event retention, result size and
> resource consumption. Support monotonic event cursors and request IDs with
> documented idempotency behavior across duplicate submissions and restarts.
>
> Scope every job to session/target identity and generation. Reconnection,
> process exit, target replacement and stale clients must not let an old job
> operate on a new target or revive invalid evidence. Human and agent commands
> need explicit ordering and provenance. A human should be able to interrupt
> agent work and understand what is still in flight.
>
> Implement durable history/checkpoints and crash recovery. Record intent before
> a simulated effect, record known outcomes afterward, and represent an unknown
> outcome explicitly if interrupted between them. Do not promise exactly-once
> effects from a log alone, automatically repeat uncertain effects, automatically
> resume side-effecting experiments, or call a history record an active job.
> Specify which observation-only work can resume and when consent/scope must be
> renewed. Prefer a small understandable storage design over infrastructure.
>
> Use an instrumented fake executor to exercise deterministic races, slow and
> failing work, cancellation at each boundary, queue pressure, duplicate request
> IDs, stale generations, target exit, and crash/restart at persistence and
> effect boundaries. Include torn or invalid records and failed storage writes.
> Check invariants and observable effects, not just enum transitions.
> The fake executor is for control-flow testing, not game-discovery evidence.
>
> No real-game memory writes, GUI automation, Anode lease, production service
> replacement, or extra background daemon are needed. Research primary sources
> where useful, and keep any proposed future executor separate from this task.
>
> Deliver a small runnable library/demo, tests and `HANDOFF.md` with exact
> commands, recovery guarantees and limits, dependencies/licenses, serialized
> command/event examples, and the minimum engine/MCP integration contract.
> Do not change Reversebolt's overall progress percentage.

## Session 3 — challenge visual tracking on actual game footage

> Independently challenge and improve Reversebolt's visual behavior research.
> Own only `C:\DEV\Reversebolt\experiments\visual-challenge\`. Create a
> standalone Rust package with its own `[workspace]` and lockfile.
>
> Read `C:\DEV\Reversebolt\docs\VISION.md` and the handoff, research, API and
> results under `experiments\visual-behavior\`. Preserve that package unchanged.
> You may call it as a read-only baseline dependency or copy a frozen baseline
> into your directory with provenance and license notices. Record its source
> hash so later changes cannot silently alter the comparison.
>
> The current observer performs useful translation/appearance tracking on
> synthetic scenes. Rotation, parallax and moving lights produce many false
> regions; slow motion gives poor whole-object bounds; animation can look like
> translation. Rejecting a camera model does not necessarily reject false object
> motion. Your task is to establish these limits on real offline game footage
> and make a measured generic improvement.
>
> Acquire ordinary timestamped frames from at least one independent offline
> game, preferably two with different visual structure. The main session uses
> Half-Life 2; choose other targets. Use Anode for desktop/game interaction,
> respecting its exclusive lease. Hold the lease only while collecting or
> testing, release it before offline analysis, and close only your own windows.
> If unavailable, work on the baseline, evaluator and synthetic regression cases
> while waiting. Do not change global display/game settings or other sessions'
> running apps. No real-game memory access or writes are needed.
>
> Include camera pan/rotation/zoom, movement through a 3D scene, stationary
> animation, occlusion, moving lights, textureless areas and scene cuts where
> available. Record acquisition settings and action timing. Feed only frames and
> timestamps to the observer: no game identifiers, known HUD/player regions,
> sprites, expected geometry, source-derived labels or manually supplied masks.
>
> Define evaluation units and annotation rules before scoring. Keep manual
> annotations and uncertain cases in the evaluator, never in observer inputs.
> Separate visible activity-region quality from whole-object tracking and
> screen motion from physical/world motion. Save the baseline's predictions
> before viewing ground truth or tuning.
>
> Then implement a tractable improvement in your owned package, such as more
> robust generic registration, unsupported-motion abstention, false-region
> suppression or identity reset after cuts. Choose from evidence rather than
> adding game-specific exceptions. Preserve baseline results. Freeze parameters
> before collecting/scoring fresh withheld sequences and compare at equal
> budgets. New clips from one game are within-game evidence; do not describe
> them as unseen-game generalization.
>
> Report strict and loose overlap thresholds, precision/recall, false proposals,
> identity switches, camera/model abstention, motion/appearance errors where
> observable, latency, memory and processing resolution. Include negatives and
> failure clips. Do not call a region “the player,” animation “movement speed,”
> or a pixel extent “world scale” without independent supporting evidence.
>
> Deliver runnable code, reproduction commands, timestamped manifests and
> checksums, small local evidence clips/frames or acquisition instructions,
> result tables and `HANDOFF.md`. Record source/dependency/model licenses and
> footage provenance; keep game footage local unless redistribution is allowed.
> Recommend the minimal observation API changes for the shared engine.
> Do not edit shared crates, desktop, root Cargo files, other experiment folders,
> or Reversebolt's overall progress percentage. Do not stage or commit another
> session's work.

## Integration checkpoint

The main session will reproduce results and review each boundary before
integration. These assignments do not replace the full VISION.md acceptance
criteria. The current overall milestone estimate remains **13%**.

Meanwhile, the main session is testing OCR against real Half-Life 2 captures.
Neural recognition improves some readings but also introduces conflicting and
cropped numeric results. Those experiments are not yet the desktop's observation
path. The existing rule-inference planner remains advisory: its synthetic active
mode increased acceptance and also increased false acceptance, so it is not a
production verification gate.
