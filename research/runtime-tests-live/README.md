# Shared live response tests

The earlier [protocol foundation](../runtime-test-protocol/README.md) is now
connected to the engine, durable evidence storage, MCP and desktop interruption.
An agent can prepare one explicit hypothesis, gather a baseline, apply its linked
edit, gather the changed condition, restore through the existing guarded Undo,
and gather the recovered condition. The engine owns the images and sampled input
records. It does not choose the candidate, supply input or infer a confirmed name.

The desktop keeps the direct-control cards. An active comparison adds only its
hypothesis, **Stop test** and **Take control**. Full retained evidence is available
through MCP; no report is added to the main card view. Taking control holds future
agent observations while evidence remains readable. Stopping does not itself
restore a changed value; the existing Undo path remains available independently.

## Contract

`prepare_runtime_test` pins exact process/investigation/catalog identities,
candidate type/path/reference chain, freshly read original bytes, the proposed
value, a caller-supplied hypothesis, four key/chord holds and distinct test/edit
UUIDs. The current runtime revision is required. Repeating the identical plan
returns its saved state; a stopped plan never resumes. The linked write is refused
until the baseline is ready, and re-reads the path before applying the edit.

Each `observe_runtime_test` uses the exact test revision. It durably saves capture
intent before reserving a new slot. The three conditions each have an idle pair
and four after-action images. Missing input, wrong foreground, timing/geometry
changes, weak baseline and incomplete evidence prevent a successful comparison.
Fresh reads after the changed/recovered conditions check bytes and references
against the actual durable edit. The result describes rendered response only.

One active test is allowed, with a 15-minute lifetime, at most eight retained
tests per data directory, 512 KiB per record and 64 MiB of evidence per test.
The storage budget reserves room for the next maximum image/input pair. There
is no automatic retention pruning yet. Image/input read-back verifies hashes.
Restart stops active comparisons; capture intents and occupied slots cannot be
replayed. Storage faults stop sampling and hold further tests. An unrelated edit,
target/catalog change, observation job or human takeover interrupts the test.
An active native helper is allowed to settle before stopping subsequent work.

## Evaluation

`host.py` adapts an unchanged owned runtime ABI fixture to a 640×360 striped
window. Its rendered response is deliberately driven by the discovered
`Coefficient` field. The evaluator selects that field and configures the visual
adapter from the product's discovered address; this coupling is evaluator-only.
It is neither independent software nor evidence of generalized discovery.

`run.py` uses frozen release binaries through the real stdio MCP and shared
engine. Anode explicitly focuses and verifies the fixture before acknowledging
the `fixture-focused` file. Twelve bounded D/A holds produce 18 comparison
images. Six additional images exercise missing input. No game is launched and
no image is manually reviewed. The native desktop's actual **Take control**
button is invoked through accessibility, then the evaluator verifies stopped
state, the human hold and rejected agent observation with readable evidence.

The workflow has 52 checks covering the positive comparison, missing input,
stale cursor and plan replay, premature/stopped edits, another edit taking over,
disconnect, induced record-write failure, restart and desktop takeover.
The byte oracle independently checks restoration. `audit.py` verifies frozen
source/binary hashes, all image/input digests, reference/byte read-back, helper
unloading, balanced pauses and owned child exit receipts. See
[results.json](results.json) for final measurements and source identities.

The final run passed 257 workspace tests, strict Clippy, formatting and the
49-tool smoke. Its response medians were 131.806 before, 0 while changed and
127.297 after restoration. All 18 helpers unloaded; 45 pauses were balanced,
with a maximum of 66 microseconds and no helper heap operation while paused.
JSON parse/serialization shifted a few image metrics by one floating-point ULP;
the audit permits that only for image differences/medians, keeping identities,
raw artifact hashes, values, times and decisions exact. Preview
`0.1.0-20261021-210458` matches the tested binaries and preserves all 44 desktop
history entries, model selection, trials, changes and Freeze records.

Earlier failed attempts remain under `test-results/runtime-tests-live-v1`:
case 1 completed the comparison but its evaluator oracle parser rejected a
numeric address; case 2 stopped before input because the fixture was not
foreground. Case 3 passed after evaluator-only fixes. The next frozen run also
passed. Final review then caught cancellation being cleared after preparation's
helper finished; admission now clears only a previous stop, and a later takeover
survives preparation. A focused controller regression covers that race and a
fresh final workflow checks the resulting binaries.

Native takeover is exercised between observations. The in-flight preparation
race has controller regression coverage; it is not a native delayed-helper
experiment. The 15-minute expiry is implemented but was not timed in this run.
The sampled-key and exploratory image-metric limits of the earlier protocol
still apply. No semantic naming, automatic planner, broader recovery or
independent-game acceptance credit is claimed.

## Reproduction

Build the locked workspace in release mode. Freeze its four release binaries
and the existing `reversebolt_path_fixture.dll` in a new run directory's `bin/`,
and record source/binary hashes in `frozen.json`. Run `run.py <run>/case` in
Anode. At `fixture_focus_ready`, focus its exact discovered window and verify
foreground state, then write `case/fixture-focused`. At `human_takeover_ready`,
observe that engine window and invoke **Take control** once. Retain the complete
observation/action receipt as `case/ui-takeover.json`. Wait for terminal job
status and all owned children to exit, then release the Anode lease. Run
`audit.py <run>` and the ordinary smoke test against the frozen binary directory.

Use a new output directory for each attempt; never reuse an uncertain input or
capture. The evaluator intentionally terminates its owned engine/MCP processes
after the test and retains their exit codes. It does not close unrelated apps.
