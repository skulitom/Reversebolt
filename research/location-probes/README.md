# Explicit tests of competing locations

Verified on 2026-10-21 with preview `0.1.0-20261021-080250`.
This adds a reviewed experiment when separate memory locations predict the same
observed number. It does not automatically identify a controlling gameplay value.

Ordinary preparation still requires all surviving interpretations to agree on
one patch. Probe preparation selects a candidate from an exact frozen prediction
check, keeps all transitively overlapping interpretations in the agreement vote,
and retains the other locations as competing conditional predictions. Missing
overlapping evidence blocks the probe. Nonoverlapping missing evidence remains
explicit. Apply, observation, durable receipts, expiry and guarded restoration
use the existing shared engine. Attempting a write invalidates the passive check.

The desktop keeps simple named cards. Selection of a possibility is behind
**Inspect**, while the reviewed **Try test** and **Restore bytes** actions remain
on the card. No main-view report or expanded diagnostic panel was added.

## Fixed native integration

[Protocol](PROTOCOL.md), [configuration and hashes](protocol-v1.json), runner,
fixture and exact changed source snapshots were frozen before the first case.
Protocol SHA-256:
`b1d14863b4115e491540e95649faf234174d1ccbdbe40a7fe1ec0c53eea877c5`.

Two fresh owned WinForms processes exposed a painted Quantity with two matching
float fields at randomized offsets. Both received the same twelve varied
training values, then four fresh hold/change observations. Candidate zero was
selected in advance; no candidate replacement or failed-case rerun occurred.
The evaluator checked patch containment in an owned field before application;
the engine never received the oracle or layout.

| Case | Selected role | Ordinary plan | Explicit probe observation | Restoration |
|---|---|---|---|---|
| Lower field drives display; MCP execution | Source | Unavailable: two different locations | 50 → 53; selected prediction agreed, rival differed | Both fields and display returned to 50 |
| Upper field drives display; UI execution | Unused copy | Unavailable: two different locations | Display stayed 50; selected prediction differed, rival agreed | Both fields and display returned to 50 |

Each check retained two interpretations. Repeating a completed apply kept the
same write receipt; unit tests additionally count backend calls to establish no
second write. All alternative predictions and descriptive comparisons remain in
the journal. Restored observations do not reuse predictions for the changed
patch. Neither outcome automatically removes a rival or grants editing authority.

The second case prepared through MCP and applied/restored through accessible
desktop buttons. Its main card showed Quantity, the description, arrange handle,
editable value and Inspect; the possibility picker was absent from the main
view. After the copy test, it showed **Effect not confirmed**. No screenshots
were manually reviewed and no games were controlled. Both jobs exited zero and
all six owned processes exited; the Anode lease was released.

Raw artifacts: `test-results/location-probes-v1/slot-{0,1}/`, with the UI
accessibility observations in `test-results/location-probes-v1/ui-accessibility.json`.
Run `python research/location-probes/verify.py` for a read-only artifact audit.
The audit was written after execution; it is not the preregistered method.

## Other verification and limits

All **146 workspace tests**, strict Clippy, release build and packaged smoke checks
pass. Tests include transitive overlaps, unresolved overlapping/nonoverlapping
alternatives, stale check identity, observed responses, duplicate application,
restoration and journal reopening. The packaged MCP exposes **31 tools**.
Logs are `test-results/probe-{tests-final,clippy-final,build,smoke}.log` and
`test-results/smoke-e96b1b4c/report.json`.

The updated desktop is running with its eight history entries and local-model
preference preserved; receipt: `test-results/probe-desktop-delivery.json`.

This is controlled fixture integration, not independent-game generalization.
The native cases each had one interpretation per field; overlapping aliases are
covered by controlled Rust tests. UI application/restoration is verified; the
new Inspect selector itself was compiled but not exercised in the native cases.
Reads are sequential, predictions assume other bytes remain unchanged, effects
may propagate, and no response can hide delayed or off-screen consequences.
Expected-byte checks are not atomic with target threads, and restoring bytes
does not restore the whole game. A subsequent experiment needs fresh evidence.
Automatic experiment selection, reusable semantic controls and comprehensive
effect/recovery validation remain unfinished.
