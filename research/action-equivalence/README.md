# Exact actions with unresolved representations

Research-only Rust experiment. The desktop and MCP engine do not call this code.
Nothing here attaches to a process or changes a user's game. The main product
surface remains named cards; these records are development evidence, not app UI.

## Frozen evaluation v1

Development used seed 41000 on the existing fixtures. The subsequent run uses
20 seeds, 43000 through 43019, for all 12 inherited scenarios (two tracks each)
and all 30 previously seen adversarial cases (one track each): 1,080 searches,
2,160 requested actions. These are new seeds of known synthetic mechanisms,
not an independent unseen-game evaluation. Source and executable hashes are
recorded before the run. No selection of favorable cases or parameter fitting
on the later run is permitted.

1. Preserve the earlier bytewise discovery and adaptive canonical validator.
   It may eliminate interpretations using its disposable trials. Failed prior
   trials, recovery or disposal block further action testing.
2. For each surviving interpretation, encode baseline +3 and baseline -4.
   Require baseline agreement and a desired-value round trip within the existing
   0.6 observation tolerance. Missing/unencodable/stale alternatives stay
   unresolved. Do not choose a type by rank or omit a troublesome interpretation.
3. Normalize the proposed resulting bytes to the first through last changed
   byte. Group identical before/after/offset actions. This is a new write
   footprint, not proof that the original representations or transactions are
   interchangeable. A no-op is never an actionable agreement.
4. If every survivor agrees on one action and the action fits the original
   scope, replay that exact patch twice on fresh exclusively owned targets.
   Each trial has an eight-tick sham, checks the requested value at ticks 1, 2,
   and 8, checks other tracks and unexplained memory changes every tick, then
   restores only the exact patch and compares the full observable state after
   eight recovery ticks. Partial/unknown writes, conflicts and failed recovery
   remain explicit. A failed action stops subsequent requests for that search.
5. Report `ObservedWithinScope` only for fixtures whose original contract states
   a closed effect scope. Otherwise report `ObservedWithUnknownEffects` even
   when every observable check passes. Neither result identifies the source
   type or establishes general gameplay meaning. Two replays of the same action
   are reproducibility checks, not independent semantic evidence.
6. Only after inference has answered, audit successful actions over 32 ticks
   plus 32 recovery ticks on separate copies, including private latent state.
   Do not feed audits back to the algorithm or silently reclassify its answers.
   Preserve counts of private effects remaining from prior and action trials.

Agreement is recalculated for every requested value and snapshot. A float and
an affine integer interpretation can agree within one exponent interval and
diverge outside it. A tested point does not justify an entire slider range.
Exclusive compare/write exists only inside these disposable in-memory fixtures.
This is not an implementation of atomic live-process writes, runtime suspension,
crash recovery, or scope inference. Private effects outside the audit horizon
can still exist. The old validator's assumptions and incomplete model family
remain limitations of this experiment.

## Reproduce from the repository root

```powershell
cargo test --locked --manifest-path research/action-equivalence/Cargo.toml --target-dir target/action-equivalence
cargo clippy --locked --manifest-path research/action-equivalence/Cargo.toml --target-dir target/action-equivalence --all-targets -- -D warnings
cargo run --locked --release --manifest-path research/action-equivalence/Cargo.toml --target-dir target/action-equivalence -- 20 43000 test-results/action-equivalence/frozen-v1.json
```

The core library cannot import evaluator fixtures or private truth. Full run
records stay in `test-results/`; compact source-bound summaries are tracked in
`evidence/`. Eight new tests cover exponent-boundary divergence, shifted
footprints, unavailable rivals, no-ops/stale baselines, fresh exact-action
effects, unknown scope, conflicts, unknown acknowledgements, denied restoration,
sham drift and prior failure gates.

## Provenance

MIT originals came from `experiments/causal-validation/` without modifying that
user-owned research directory. `provenance.json` records exact original hashes.
The vendored library and inherited fixture are verbatim copies. The new runner
adapts the prior evaluator adapter. `src/fixtures/withheld.rs` preserves its
original target mechanisms and initialization; it changes imports, removes the
unused canonical safe-case classifier and replaces the old suite driver with
the action runner and post-answer audit. The original MIT notices are retained
in `vendor/causal-validation/LICENSE`; new code uses the repository MIT license.
