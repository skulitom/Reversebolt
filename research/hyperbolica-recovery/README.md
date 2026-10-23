# Recover the automatically learned control after a full restart

Reversebolt recovered Hyperbolica's previously tested walking-speed control in a
new game process, reopened the durable history in a new engine, and completed a
fresh before/change/restoration comparison. The published product was unchanged.
No model calls, manual field/address selection, recipe backfill or process-scope
substitution were used.

Discovery itself published one fresh candidate with a needs-revalidation hint
bound to the prior test. `recover_runtime_control` independently checked the
same evidence and returned the same candidate and a preparation template, with
`edit_authority: false`. The evaluator added only new test/change IDs and ran
that template through the ordinary baseline, edit, readback and restoration gates.

| Condition | Value | Product response median | Independent full-frame grayscale median |
|---|---:|---:|---:|
| Fresh baseline | 2.25 | 12.0190 | 11.2817 |
| Changed | 0 | 0.5518 | 0.6976 |
| Restored | 2.25 | 11.9507 | 11.0926 |

The product again records `suppression_observed`: measured visual response fell
95.4% while changed and returned to 99.4% of baseline after exact restoration.
The independent image calculation supports the direction. This is an
action-specific rendered effect, not physical speed, persistent actor identity
or proof that unobserved consequences are absent.

## Retained evidence

The [protocol](PROTOCOL.md), [freezer](freeze.py) and [runner](run.py) were frozen
before launch. The test copied 187 settled data files exactly, omitting only the
obsolete endpoint credential. The original native artifacts remain unchanged.
Copied historical tests, trials, runtime reports and images also retain their
original hashes. There is no fabricated learning phase or imported substitute
test: the source is the previous run's genuine product-generated record.

The new process identity, engine boot and candidate ID differ from the originals.
The stored typed recipe resolves to one current route. Automatic discovery and
MCP recovery agree on its source test, compact-JSON test/recipe fingerprints and
current candidate. A new recipe is retained with the fresh comparison. The
underlying test-store file has a separate byte hash because it is pretty-printed.

Recovery takes no discovery screenshots and makes no model calls. Setup used
two viewed images, a window-focus action and one Continue click. The comparison
used 18 automatically captured images and 12 key holds; none of its images was
manually viewed. The evaluator reused the previously recorded action sequence,
but all input delivery receipts, sampled windows, field readbacks and effect
measurements are new.

The [artifact audit](audit.py) checks exact images/actions/input files, key
acceptance and matched durations, chronology, guarded writes/readbacks,
restored bytes, current process scope, canonical history fingerprints and
immutable prior evidence. All seven current helper operations completed and
unloaded. Ninety recorded pauses show zero helper allocations/deallocations;
the largest is 4.058 ms. This is recorded timing, not a hard real-time guarantee.

Both retained trials are restored, no test/job/freeze remains unresolved, all
owned processes exited and Anode was released. The user's main editor remained
responsive. `game.save` and Steam app storage are unchanged; normal Player logs
and Unity analytics changed. Before/after storage copies remain local.

Run `python -X utf8 research/hyperbolica-recovery/audit.py` to check the evidence
without launching a game, running a model or viewing screenshots. Add
`--verify-current` to also compare current sources with the frozen product.
[Results](results.json) summarize identities, timings, measurements and storage
differences; full artifacts remain in `test-results/hyperbolica-recovery-v1`.

Progress is **40%**, with intent/editing increasing from 3/10 to 4/10 for recovery
and fresh revalidation of an automatically learned control in another game.
Validation remains 6/15 and withheld-game generalization remains 0/10. This is
one restart of a known game/build; scene/load changes, updated layouts, persistent
actor identity, broader effects and unfamiliar-game acceptance remain unproven.
Product code is unchanged and preview `0.1.0-20261023-093127` stays selected.
