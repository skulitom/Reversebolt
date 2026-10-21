# Test the requested action, retain unresolved representations

2026-10-21. Automated research on disposable targets; no game input, screenshots,
live process writes, or model calls. This does not change the desktop controls.

## Result

The old requirement of identifying exactly one canonical storage type is too
strict for some useful edits. Over the fixed 1,080 searches, that validator
identified 41 canonical representations. A new action planner found at least
one successfully tested exact edit in 323 searches, including 282 searches where
canonical identification abstained. It did not select a winning type.

| Fixed evaluation | Inherited mechanisms | Previously seen adversarial mechanisms | Total |
|---|---:|---:|---:|
| Searches | 480 | 600 | 1,080 |
| Correct canonical identifications | 39 | 2 | 41 |
| Searches with a scoped tested action | 239 | 84 | 323 |
| Requested actions | 960 | 1,200 | 2,160 |
| Actions observed within declared scope | 471 | 149 | 620 |
| Actions observed with unknown effects | 0 | 120 | 120 |
| Actions not tested | 489 | 931 | 1,420 |
| Failed longer audits of scoped actions | 0 | 0 | 0 |
| Failed longer audits of unknown-effect actions | 0 | 80 | 80 |

Each observed action was replayed twice and restored on fresh owned targets:
1,480 additional trials. None had an observable recovery or disposal failure.
The prior validator still incurred 161 recovery failures and 20 disposal
acknowledgement failures; these remain in the evidence and block action tests.
The two methods answer different questions, so the table is not a claim that
canonical identification accuracy improved.

All 620 scoped actions also passed the post-answer 32-tick effect and 32-tick
recovery audit. This is **zero observed audit failures in these cases**, not a
guarantee of safety, causal uniqueness, or general-game compatibility. The
successful requests are specific baseline +3 and baseline -4 edits, not an
arbitrary slider range. All 41 canonical identifications matched evaluator
truth. Of the 620 scoped actions, 538 came after canonical abstention.

## Why this matters for cards

An editable card ultimately needs an operation that achieves its stated effect.
Knowing whether the original programmer declared a field as an integer or float
is a separate question. Several shifted or overlapping numeric interpretations
can imply exactly the same byte change for one requested value.

For example, the two supplied targets `NarrowFloat` and `NarrowBits` have
different canonical types but expose indistinguishable evidence in their bounded
domain. Every seed now produces identical blind answers and successful requested
actions in both. In a separate test, the same interpretations disagree when the
requested value crosses an exponent boundary. Agreement must be recalculated
for each baseline and requested value; it cannot certify a wider range.

The planner compares the full resulting bytes and normalizes an agreed edit to
the first through last changed byte. That shorter write is a **new operation**.
A regression target accepts the old full-width write but produces an unwanted
effect for the shortened write. The new exact-action replay catches that failure.
It never inherits effect verification just because the resulting bytes match.

Useful coverage is still selective. All 40 inherited Plain and Unaligned
searches obtain tested actions despite unresolved types. None of the Nonlinear,
RotatingXor, Relocation, Constant or Coupled searches do. Only 4/40 DoubleOffset
searches and 0/20 DoubleBoundary searches do. The full per-scenario counts retain
these failures. Of 1,420 untested requests, 536 lack a unanimous nonzero action
and 884 lack sufficient prior evidence or follow a failed prior trial/recovery.

## Hidden effects remain a decisive limit

`HiddenClean`, `HiddenLatent` and `HiddenImmediate` receive identical answers for
every seed. Their fixture contracts explicitly state that effect observation is
incomplete, so all 120 actions remain `ObservedWithUnknownEffects`. A real game
does not acquire a closed effect scope merely because visible checks pass.

The later audit finds hidden irreversible state changes in 80 of those actions:
40 immediate effects and 40 effects appearing beyond the eight-tick validation
horizon. Their scalar observations and bytes restore successfully. Separately,
private-state audits find 204 residual hidden effects from the prior validator's
trials and 80 from the new exact-action trials. Those are counts of disposable
trials, not the 80 audited requested actions, and must not be conflated.

Consequently, these results do not justify enabling general-game editing or
describing an operation as globally reversible. They provide a way to retain
type uncertainty while testing a precise operation within an explicit scope.

## Method and verification

The fixed method and full reproduction commands are in
[the experiment README](../../research/action-equivalence/README.md).
Development used seed 41000. The frozen run uses 20 new seeds, 43000–43019,
across every existing scenario. These are known synthetic mechanisms, including
the formerly withheld fixtures, not new independent-game evidence.

The library receives bytes, numeric observations, surviving models and declared
scope. Fixture code and private truth are linked only into the evaluator binary.
The original adaptive validator and discovery library are vendored verbatim
with their MIT notices. The original user-owned `experiments/` tree is unchanged.
No production crate depends on it or on this new research package.

Eight new Rust tests and strict Clippy pass. Tests cover missing rivals, stale
baselines, no-ops, changed write footprints, unknown acknowledgements, concurrent
conflicts, failed undo, sham drift, prior-failure gates, and incomplete effect
scope. The standalone verifier checks source/executable hashes, complete survivor
accounting, failure gates, recovery, the full action counts and identical blind
answers for both sets of indistinguishable targets.

Run the receipt/source check from the repository root:

```powershell
python scripts/action-equivalence-verify.py
```

The compact tracked record is
[`results-v1.json`](../../research/action-equivalence/evidence/results-v1.json),
bound to the pre-run [`freeze-v1.json`](../../research/action-equivalence/evidence/freeze-v1.json).
The full local record is `test-results/action-equivalence/frozen-v1.json`
(16,012,390 bytes); its hash is in the compact record.

The next integration task is to carry a requested operation, its competing
interpretations, effect scope, identity and recovery evidence through the shared
engine. That needs an actual live-target lifecycle and observations of meaningful
gameplay consequences. This experiment's exclusive compare/write, exact replay,
closed synthetic scope and disposable target are not capabilities the Windows
process reader currently provides.
