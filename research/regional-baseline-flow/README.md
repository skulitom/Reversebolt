# Small local responses can enter the live comparison flow

The old final-baseline gate required whole-screen mean image change of at least
5 gray levels and five times idle change. A small moving subject could therefore
stop the experiment before the regional selector was ever called. Earlier region
tests manually constructed a ready baseline and did not cover this entry gate.

After six captures with valid sampled input, a weak whole-screen baseline now
enters `baseline_review`. The same final observation runs the existing bounded
v2 selector on its saved baseline images and input windows. A supported result
automatically saves its comparison region and reaches `ready_to_apply`. The
choice is immutable before any edit; the engine records the complete baseline
and then its exact choice transition separately. A late stop can preserve that
choice while stopping the test. No suitable region stops the comparison.

Missing input never reaches regional acceptance. The review rejects an existing
linked edit, an unresolved runtime write, or a test that has already stopped.
Abort is checked before and after analysis. A persisted review becomes stopped
on restart and cannot resume. A caller has no command to assert that a region
passed; the native capture path supplies the engine-owned files to the selector.
Read-time suggestions remain read-only, and sufficiently strong whole-screen
baselines retain their previous optional selection flow.

The tests use generated PNGs and sampled-input files with no native target:

| Case | Required outcome |
| --- | --- |
| Small translated subject on static textured background | Actual whole-screen metric is below 5; regional review saves a choice and permits the next explicit edit step |
| Saved changed and recovered images | The chosen region measures the known change-and-return pattern; the same history is readable after restart |
| Static scene or corrupt image | No appearance choice; test stops |
| Missing input | Baseline stops before regional acceptance |
| Aborted, restarted, or already-prepared edit | No new choice; no edit becomes available |
| Stale cursor, changed source hash, or attempted replacement | Rejected without changing the retained choice |

The local-response test feeds every frame through `finish_capture`, calculating
the actual production whole-screen metric from saved bytes, then calls the same
review helper as native capture. It exercises the durable store and shared
history dispatcher. This closes the earlier test gap without pretending to test
native capture or real gameplay. Core protocol checks cover atomic acceptance,
premature edits and restart. Existing image/input verification tests still cover
mixed monitors, unavailable evidence and unresolved correspondence.

The whole-screen scalar assessment remains unchanged: a small regional baseline
can still produce an inconclusive scalar result. Its separate appearance
comparison remains descriptive, and does not establish object identity, physical
speed, causal gameplay meaning or edit authority. Connecting verified field
interventions to that regional outcome remains necessary work. No new desktop
card or report is introduced.

All **374 workspace test executions**, strict workspace/all-target Clippy and
format checks pass. Validation logs and a source/log hash receipt are retained
under `test-results/regional-baseline-flow-v1`.
No game, model call, fresh capture, input or native memory operation is part of
these tests. Overall progress remains 35%; the desktop preview is still
unavailable after Windows quarantine.
