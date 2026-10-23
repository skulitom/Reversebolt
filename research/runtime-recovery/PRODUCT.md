# Remembered controls in the shared engine

New input-response comparisons save a compact declared-route recipe from their
own validated discovery. It contains typed route metadata and source checksums,
not an old address, value, permission or game-specific profile. The recipe is
immutable within the test history. Older test records remain readable and have
no recipe; the product does not silently migrate them.

After runtime discovery, the desktop/agent engine checks up to eight latest
remembered routes for the same runtime module location. Completed comparisons
must reproduce their positive whole-screen or regional result from retained
images, sampled inputs, exact edit/restoration receipts and value readbacks.
A newer stopped, negative or unavailable comparison prevents automatic fallback
to an older positive result for that route.

One graph traversal resolves the bounded batch. Qualified owner/declaring types,
shapes, unique field names, representation, flags, inline types and array positions
must agree. Layout hashes and field ordinals may differ; the strict mismatch stays
in the result. Only an unambiguous candidate produced by the current validated
graph is eligible. The current process, catalogue and report checksum bind card
publication. Existing newer readings and the user's search are preserved.

The card says “Needs a fresh check” and uses the existing inline editor/Freeze
controls. Unidentified labels remain hidden. Recovery does not establish a
gameplay name, persistent actor identity, physical quantity, current effect,
complete world rollback or edit authority. It performs no target access, capture,
input or model call. Fresh experiments still use the ordinary checked edit and
restoration workflow.

Agents can call `reversebolt_recover_runtime_control` with `test_id`, `catalog_id`
and `after_revision`. It uses the same checks, shows the fresh candidate and
returns a `prepare_runtime_test` argument template. The caller supplies new
test/change UUIDs and explicitly conducts the new comparison. This operation
changes card selection and revision; it is not advertised as read-only. Retained
test summaries expose the recipe checksum without claiming current validity.

## Evidence

- All **494** ordinary workspace tests pass; one archive replay is opt-in.
  Strict workspace Clippy and formatting pass.
- Integration fixtures reopen the real durable stores and exercise both whole
  and regional evidence. Their target implementation panics on reads/captures.
  Changed images, metrics, inputs, geometry, readbacks, recipe bindings and process
  scope reject publication. Cancellation and newer stopped tests preserve state.
- Core regressions cover compact serialization without the old report, changed
  layouts/ordinals, missing and ambiguous routes, batch limits and malformed late
  graph entries. UI checks retain the short explanation and reject stale or
  unidentified presentation.
- The opt-in retrospective replay runs the actual shared bridge against archived
  Viewfinder evidence. Both explicit recovery and automatic discovery publication
  produce candidate `0cccec42269c8556a3627e8619cadf3535b2c6be5ac35563875f00c8aeff69f0`.
  The strict status remains `catalog_changed`. All 18 images and 18 input files
  are checked, alongside the original restored edit. The two recovery calls take
  4.168 seconds in this run. No game process is contacted and no model is called.

That historical comparison predates recipe retention. The replay deliberately
adds a recipe from its own archived discovery to an **isolated research copy**,
then reopens the stores. It does not modify the original evidence or demonstrate
automatic migration of old production history. Synthetic fixtures cover recipe
persistence. The subsequent prospective native run below verifies that retention
and automatic recovery in the product itself.

Run `replay-product.py NEW_OUTPUT_DIRECTORY` to reproduce with the local archived
evidence. The manifest pins source files and input reports; `result.json` records
the checked file hashes and output bindings. Original artifacts are under
`test-results/runtime-recovery-product-v1`; the compact result is
[product-replay-results.json](product-replay-results.json).

## Prospective product test across a full restart

Published preview `0.1.0-20261023-070135` completed two new native comparisons in
separate Viewfinder processes, with the headless engine closed and reopened
between them. The first comparison saved its own route recipe during ordinary
MCP preparation. Its historical seed was used only by the generic proposal CLI;
no old test was imported into product storage. In the second process, automatic
discovery rechecked the new test's retained evidence and displayed a unique fresh
candidate. `recover_runtime_control` independently returned the same candidate
and the new preparation template used for the second comparison.

| Fresh process | Product response: baseline / changed / restored | Independent full-frame response |
|---|---|---|
| Learn | 14.146 / 0.139 / 14.067 | 14.218 / 0.562 / 14.074 |
| Recover | 13.939 / 0.141 / 14.143 | 13.987 / 0.572 / 14.219 |

These are image-difference measurements, not physical speed. Both runs restored
the original value and observed the response return. The audit checks the two
process identities and engine boots, unchanged first-test history, source recipe
and fresh report bindings, both MCP paths, all 36 images, 24 short input holds,
input execution receipts, value readbacks and ordered restoration. All 14 helpers
completed and unloaded. Across 516 recorded managed pauses, helper allocations
and deallocations were zero.

Only four setup images were viewed, with two Continue clicks in total. No
measurement image was manually viewed and no model was called. Save and backup
bytes remained identical; normal logs/analytics changed. All owned test processes
exited, Anode was released, and main-desktop editor PID 98332 remained running.
The tested app/MCP/loader/helper are the exact packaged binaries already selected
by the desktop shortcut. No production code changed for this evaluation.

Intent/editing credit rises from **2/10 to 3/10** for durable knowledge reused and
revalidated after a complete product/game restart: **38/100** overall. This is
still a known development game. Exact semantic meanings, persistent actor identity,
composed changes, full world rollback and independent-game transfer remain unproven.

- [Frozen protocol](NATIVE-PRODUCT-PROTOCOL.md), [freezer](freeze-native-product.py),
  [native runner](run-native-product.py), [audit](audit-native-product.py).
- [Audited result](native-product-results.json).
- Complete local artifacts: `test-results/runtime-recovery-native-product-v1`.
