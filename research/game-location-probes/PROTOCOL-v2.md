# Paced input variant, frozen after V1 failures

V1 collected two fresh sessions but neither qualified for a write: both games
had ended before the fresh change challenge. Preserve both abstentions and the
separate startup-directory failure. This is a new development experiment with a
revised input schedule, not a rerun or replacement of those cases.

Retain V1's unchanged original game, exact product binaries, scope-selection
rule, two fresh profiles, oracle, timing checks, read budget, automatic track
selection, first/last candidate rules, one-shot execution, checked restoration
and cleanup limits. Freeze these revised files/configuration before either run.
Do not retry an observed case or choose a different candidate after refusal.

The input driver initially sends only the start key, then stops sending keys.
After each automatic capture, pause the observation job through its existing MCP
control and verify the capture count did not advance. After captures 1–11 send
eight pulses from the original repeating movement/down schedule, then wait for
the driver to confirm release of the final key. Resume the observation job.
Each pulse holds for 80 ms with a 100 ms period inside a burst; long idle gaps
between bursts are governed by completed observations, not a continuous timer.
After captures 12 and 13, resume without any input. After capture 14 send a fixed
burst of 24 pulses, confirm completion, then resume. After capture 15 resume
without input; after capture 16 wait for job completion. The game continues
running during all waits. Pause/resume never suspends target threads or freezes
game physics. A missed pause or capture is a failed case, not permission to retry.

The trial requests the latest observed number plus ten, still within the engine's
twenty-unit bound. This uses an integer perturbation compatible with the coarse
integer mappings seen in prior exposed development evidence; it is not a product
rule or a source-derived address. Keep candidate zero for case one and the last
candidate for case two, with no replacement if either representation refuses.
All other apply/observe/restore actions and 750 ms settle intervals remain V1's.
Do not use oracle state to choose inputs, candidates, addresses or desired values.

Report outcomes separately from V1. These are development trials of whether a
generic discovered location influences observed and game-internal score in this
scope, not proof of general game understanding, complete recovery or a withheld
acceptance pass. If evidence is insufficient, retain the refusal unchanged.
