# Automatic choice of an input experiment

Implemented 2026-10-22. Full-goal progress remains **35%**. This connects observed
input to a one-click investigation and shared MCP planning. It does not establish
gameplay semantics or useful causal controls.

## Selection and shared workflow

The desktop's **Explore next** button replaces the key-selection menu. The shared
`start_activity_experiment` command now selects a key when `key` is omitted.
Agents can inspect the identical policy through read-only
`plan_activity_experiment`, or still supply an explicit observed key.

The versioned policy `untried_observed_key_v1` uses each eligible key's latest
complete input window. It sorts by polls that sampled that key alone, then all
polls sampling that key, source recency and key code as a deterministic tie break.
It uses no application name, key meaning, game profile or image-model call.
Overlapping windows are not accumulated. These counts describe sampling; they
are not verified continuous holds or expected information gain.

Previously proposed keys in the current investigation/batch are excluded from
automatic selection. Every new proposal, including an explicit-key proposal,
creates an immutable marker before admission. This survives job-history eviction
and restart; cancelled, uncertain or failed admissions are not implicitly retried.
A damaged marker or missing/mismatched referenced plan fails explicitly. An
explicit key remains available for a deliberate new proposal under the existing
checks. Historical jobs predating this feature are covered while retained; the
feature cannot reconstruct legacy history already evicted.

The saved plan pins the complete recommendation and selected source metadata.
Retrying the same request returns the same plan despite later changes in ranking;
changing between automatic and explicit selection is a different request. The
completed experiment archive retains `input_selection`. Agent jobs still wait
for the existing desktop Start authorization. Selecting a process alone does not
send input, and the experiment still checks focus, sampling, scope and capacity.

## Saved-recording replay

The unchanged policy was run read-only against the four original final-status
documents from the previously exposed browser-game evaluation:

| Recording | First proposed key | Pressed polls in its source | Eligible keys |
| --- | --- | ---: | ---: |
| 2048 run 1 | Up | 15 | 4 |
| 2048 run 2 | Left | 15 | 4 |
| Tetris run 1 | Down | 35 | 5 |
| Tetris run 2 | Down | 40 | 5 |

Each first choice had the same number of single-key and pressed polls in its
latest qualifying window. Replay also records the hypothetical ordering as keys
are excluded. It does not execute those experiments, update game state or spend
capture capacity, so that ordering is not a feasible full execution schedule.
No key is labelled with an inferred gameplay action. The recordings are exposed
development evidence, not a withheld evaluation or a causal success result.

The read-only Rust example accepts one saved status JSON path:

```powershell
cargo run -p reversebolt-core --example input-selection-replay -- test-results/independent-games-v1/2048-1-attempt-01/final-status.json
```

## Automated verification

All **402 workspace tests**, strict workspace/all-target Clippy and formatting
pass. The logs and source snapshot are retained locally.

Tests cover key-renaming equivariance, overlapping source windows, single-key
versus chord ordering, exclusions, insufficient capture/storage capacity,
incomplete coverage, changed geometry and a changed pinned source digest.
The real controller command test verifies that planning creates no job, agent
proposals remain queued without input, replay preserves the chosen key, changing
selection mode is rejected, cancelled proposals stay excluded, and a new batch
has a separate history. It evicts the original jobs by admitting and cancelling
26 later observation jobs, then checks exclusions again and after reopening the
journal. A corrupted reservation is rejected.

The generated capture integration now uses an automatically selected, pinned
plan through all ten captures and the real final review command. It checks four
bounded response episodes, zero target-memory samples and preservation of the
selection in the hash-checked review archive. No native key worker, game or live
model is used. [Results, logs and hashes](results.json) record verification.

Initial targeted checks caught a missing method visibility annotation and a test
that attempted to reopen a still-owned journal. The test now drops the owner
before reopening it; journal ownership protection is unchanged. Failed logs are
retained.

## Remaining work

Sampling strength is only an initial scheduling heuristic. Whether its first
choice produces discriminating response evidence needs native-game validation.
The broader scheduler still needs to choose between further observation,
metadata questions and causal comparisons and link responses to controlling
state. No generalization credit is added. All work and security artifacts remain
local; the quarantined desktop preview is not replaced or launched.
