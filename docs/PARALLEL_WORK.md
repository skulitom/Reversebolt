# Parallel Astra sessions

Send one prompt below to each session. Three sessions are enough for a useful
first split. The main Reversebolt session owns the desktop, Windows process and
window capture, numeric OCR, shared engine, MCP, deployment, and progress record.

Each researcher owns a separate directory. Create a standalone Rust package
there with its own `[workspace]` section, so it builds without changing the root
workspace. Read the rest of Reversebolt freely, but modify only the assigned
directory. Do not stage or commit another session's files. Do not change the
running desktop preview or global MCP settings.

These tasks are parts of the full vision, not alternative definitions of success.
Synthetic benchmarks are useful for development; label them as such.

## Session 1 — visual behavior and appearance

> Work on Reversebolt's general visual observation research. Own only
> `C:\DEV\Reversebolt\experiments\visual-behavior\`.
>
> First read `C:\DEV\Reversebolt\docs\VISION.md`,
> `docs\PROGRESS.md`, and the source under `crates\reversebolt-core`.
> The main session is implementing native window capture and OCR; do not
> duplicate or modify those components.
>
> Build and evaluate a Rust prototype that takes ordinary timestamped image
> sequences and finds useful non-HUD signals: moving entities, position and
> velocity, changes in shape/scale/posture, and repeated animation states.
> It must not identify a game by executable name or use predefined game regions,
> sprites, offsets, or source-derived layouts.
>
> Research primary sources for promising approaches. Choose a tractable method
> and implement it, rather than delivering only a literature survey. Test it on
> varied footage or independently generated scenes, including camera motion,
> occlusion, static UI, lighting changes, and animation without translation.
> Separate object motion from camera motion where supported; report ambiguity
> when those cannot be distinguished. Do not call a tracked blob "the player"
> without evidence.
>
> Deliver runnable code, reproducible evaluation inputs or acquisition
> instructions, measured results and failure cases, plus a small JSONL output
> schema for observations (timestamps, track identities, geometry, uncertainty
> and provenance). Explain how these signals can be associated with memory
> trajectories and actions. Keep production-facing code in Rust; exploratory
> scripts are acceptable.
>
> Use Anode only when actual desktop footage is needed, respecting its exclusive
> lease and closing only your own test windows. The main session is using
> Half-Life 2; choose another offline target and coordinate if the seat is busy.
> Do not modify real-game memory in this task.
>
> End with a handoff in `HANDOFF.md`: what works, measured limitations, exact
> commands, artifacts, dependencies/licenses, and the minimal integration API.
> Do not change Reversebolt's overall progress percentage.

## Session 2 — causal validation and representation ambiguity

> Work on Reversebolt's causal validation research. Own only
> `C:\DEV\Reversebolt\experiments\causal-validation\`.
>
> Read `C:\DEV\Reversebolt\docs\VISION.md`,
> `docs\research\EXPERIMENT_RESULTS.md`, `docs\research\RESEARCH.md`,
> `crates\reversebolt-core\src\discovery.rs`, and `engine.rs`.
> The original MIT experiment is in `C:\DEV\GameDiscoveryLab`.
>
> The key unresolved failure is that multiple overlapping numeric interpretations
> can pass small interventions: floating-point aliases, partial-word edits,
> telemetry copies, coupled fields and unobserved side effects. A successful
> observed scalar change does not prove that we found the correct representation
> or scope.
>
> Design and implement a Rust validation prototype that chooses informative,
> bounded interventions, compares alternative hypotheses, detects overlapping
> representations and side effects, and abstains when authority or type remains
> unidentifiable. Include explicit scope and rollback requirements. Do not assume
> that an entire live-game memory region can be restored safely.
>
> Evaluate against the inherited difficult cases plus new withheld cases.
> Keep authoritative layout/type information confined to the evaluator and use
> it only after the validator has produced its answer. Include negative cases,
> failure injection, and counterexamples where no available experiment can
> distinguish the hypotheses. Report false acceptance, abstention, correct
> identification, intervention count and rollback failures separately.
>
> Deliver runnable code, reproducible results, and a recommended experiment
> policy/API suitable for the shared engine. Focus on a tested improvement over
> the current two fixed deltas; do not just tune the benchmark seeds.
> Experiments must use disposable targets created by your harness. No Anode or
> real-game writes are needed.
>
> End with `HANDOFF.md` containing the exact commands, evidence, unresolved risks,
> dependencies/licenses, and integration boundary. Do not edit the main engine,
> desktop, root Cargo files, or overall progress percentage.

## Session 3 — hidden rules and active investigation

> Work on Reversebolt's general hidden-rule inference and experiment planning.
> Own only `C:\DEV\Reversebolt\experiments\rule-inference\`.
>
> Read `C:\DEV\Reversebolt\docs\VISION.md`,
> `docs\ARCHITECTURE.md`, and `docs\research\RESEARCH.md`.
> The main session owns native capture, OCR, memory access and MCP; another
> researcher owns visual tracking and another owns safe causal validation.
>
> Build a Rust prototype that consumes timestamped observations and actions and
> proposes interpretable relationships: rates, thresholds, cooldowns, simple
> physics, or state-dependent rules. The property need not have an on-screen
> number. Inputs can include position/velocity, animation or appearance events,
> generic numeric tracks, and action timing.
>
> Start with a small explicit hypothesis language and implement an investigation
> planner that selects useful next observations/actions to distinguish competing
> explanations. Keep observation, hypothesis, intervention proposal and verified
> conclusion separate. Report unidentifiable parameters and correlated causes.
> Do not bake game names, gravity constants, health layouts, or expected answers
> into the inference path.
>
> Research primary sources as needed and test your prototype on diverse,
> randomized systems with hidden parameters and withheld mechanisms, noisy or
> missing observations, delayed effects, and confounding. Ground truth belongs
> only in the evaluator. Include cases where the correct response is "insufficient
> evidence." Measure prediction error, false conclusions, abstention, and the
> experiments needed compared with passive observation.
>
> Deliver runnable code, reproducible experiments, results and failure cases,
> plus a JSONL observation/action contract and a structured proposed-experiment
> output. Proposals should state expected outcomes, evidence needed, scope,
> uncertainty, and cancellation/rollback preconditions so the main engine can
> decide how to execute them. This task does not execute experiments on the
> user's real games.
>
> End with `HANDOFF.md`: what works, exact commands, artifacts,
> dependencies/licenses, integration API, and the next experiment that would
> most challenge your conclusions. Do not change the root workspace, desktop,
> shared engine, or overall progress percentage.

## Integration

The main session will inspect each handoff, reproduce its results, and integrate
useful parts into the same engine and MCP workflow. Integration credit requires
end-to-end evidence. An impressive isolated benchmark does not establish
unfamiliar-game understanding.
