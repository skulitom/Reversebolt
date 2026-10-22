# Compare retained displacement across a trial

The shared history reader now accepts `include_motion: true`. It returns a
separate, versioned `trial_motion_comparison` containing numeric movement evidence
from baseline, changed and recovered images, without returning images or asking
a model to inspect them. The default read remains inexpensive. Each action index
is compared separately, so alternating left/right displacements are not averaged
into a misleading zero. No report was added to the desktop card view.

The production analyzer verifies retained image and input hashes, PNG dimensions,
chronology and monitor generation, and recomputes input witnesses. Its compact
output preserves selected/matched patch counts, spatial support, signed vectors,
per-pair errors, action indices and a test-record digest. Partial records, sparse
matches, missing evidence, input mismatch, idle displacement and threshold gaps
remain explicit. Unaffected pairs survive a missing/corrupted file.

All four baseline actions must have supported displacement, their changed
counterparts must differ, and recovery must agree with baseline for the
`displacement_changed_and_returned` description. This describes selected image
patches. It does not establish a gameplay effect, a particular actor, physical
speed, input delivery, complete recovery or causality. The heuristic thresholds
and full exclusions are in [the protocol](PROTOCOL.md). Existing scalar trial
assessments and discovery-origin comparisons are unchanged.

## Evidence

The real command/store/restart test uses eighteen generated texture images with
known alternating +8/-8-pixel shifts, zero shifts in the changed condition, and
returning shifts in recovery. It verifies the exact vectors, repeatability after
restart, unchanged null trial assessment, no implicit PNG payload, explicit input
and image corruption failures, retained unaffected pairs, optional-read behavior
and boolean argument validation. Policy tests reject a failed return, sparse
support, idle motion, wrong input and the two-pixel threshold gap. These are
synthetic observations, not a native/gameplay experiment.

A frozen file replay then reads all three older `planned-input-v1` records. Their
provenance is **OwnedPathFixture**, including the raw field named WalkSpeed.
The complete comparison's fifteen intervals contain no selected two-dimensional
corners, so motion correctly remains `insufficient_support`. Its original
`suppression_observed` scalar assessment is retained exactly. The two interrupted
records remain `incomplete`. There are no measurement or input-verification
errors. This is not evidence of game generalization or motion-estimator accuracy
on genuine gameplay.

The three replay processes exited normally in 0.343 seconds total (one local run,
not a benchmark). The audit verifies 137 frozen source files and all 43 unchanged
original evidence files. The 91 test executions across app, vision and MCP targets
pass, as do strict Clippy and formatting. Logs are
`test-results/trial-motion-tests.log` and `trial-motion-clippy.log`.

No new screenshot, game, target access, input, field write or model call occurred.
The desktop executable remains quarantined and this increment is unpublished.
Full-goal progress stays **35%**. See [results](results.json); frozen source,
binary, process receipts and detailed outputs are in `test-results/trial-motion-v1`.

To reproduce in a new output directory, build the `trial-motion-replay` example
from reversebolt-vision, then pass a saved test JSON and a new output JSON. The
example uses file readers, never the mutable runtime-test store. `run.py` freezes
and executes the declared three-record evaluation; `audit.py` only checks files.
