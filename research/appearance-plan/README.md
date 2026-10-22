# Save the appearance choice before the linked edit

`lock_runtime_test_appearance` now stores a one-time choice after the complete
six-frame baseline and before any linked runtime edit record exists. It requires
the current test revision and its owner (or desktop controller), checks the
baseline PNG hash/dimensions, and saves the plan as a separate durable transition.
An already prepared edit is enough to reject a new choice; no write attempt is
necessary. The operation does not access the game or run a model.

The optional plan binds the selected rectangle and baseline-image hash, complete
baseline condition, immutable comparison metadata, selection revision and time.
Immutable metadata includes target, requested value, action sequence, provenance
and scope. Digests use typed Rust/Serde serialization. Dynamic capture/result
state is excluded from the comparison-plan digest, so valid later observations
can be appended while changing the baseline or planned experiment is rejected.
The store refuses replacement/removal and requires adding a plan to be its own
transition from the saved baseline state.

An identical selection and original revision can be replayed without rewriting
the record, including after stop/restart. It returns the retained plan rather
than resuming the test. Old records omit the new field and keep their prior
serialization. The regional matcher and comparison thresholds did not change.

Historical reads automatically use the locked selection and retain it in the
comparison output. A conflicting region is rejected. If its source image later
becomes unavailable, metadata remains readable and a separate error explains
why the comparison is unavailable. Tests without a plan retain the optional
retrospective selection API. No desktop report, saved scalar assessment or
write authority is changed.

This records a choice before the linked Reversebolt edit; it does not exclude
external edits, eliminate baseline-selection bias, establish object identity or
prove causality. Region selection is still caller-supplied. Automatic meaningful
subject selection and a real-game regional intervention remain outstanding.

## Verification

All 360 workspace test executions, strict workspace Clippy and formatting pass.
The core test carries a locked choice through generated edit/recovery protocol
receipts and serialization, rejecting changed baseline/action metadata. The real
dispatcher/store test uses generated image files without a target. It checks
actor/cursor enforcement, refusal after a prepared edit record, unchanged retry
bytes, immutable selection, automatic comparison, restart and an unavailable
baseline image that leaves the original test metadata readable. The existing
human-takeover/observation-ownership test also includes the new command.

These are synthetic protocol checks, not native gameplay evidence. The prepared
edit in the dispatcher test is an in-memory fixture; no runtime edit file or
memory operation is created. Later generated observations are retained explicitly
as synthetic data, not claimed as outcomes of an executed edit.

The [frozen compatibility protocol](PROTOCOL.md) replays the same three
**OwnedPathFixture** records and identical centered regions from
trial-appearance-v1. No prospective plan is attached to a completed/interrupted
record. Each complete JSON result equals its previous result exactly, including
the original assessments, global comparisons, regional diagnostics and errors.
The full striped fixture remains unresolved, the partial fixture incomplete,
and the empty fixture reports a missing baseline.

The audit verifies 152 frozen sources, the replay executable and 53 bound input
files unchanged. No game, new capture, input, model or memory operation ran.
[Compact results](results.json) retain the outcomes; the full archive is
`test-results/appearance-plan-v1`. Logs are
`test-results/trial-appearance-plan-tests.log` and
`test-results/trial-appearance-plan-clippy.log`.

Frozen manifest SHA-256:
`ba886dbf4770936b557f488a4cd8d0c57788ebeda2fadaa1b452a9fb43949410`.

Overall progress stays 35%. The desktop preview remains unavailable after its
recorded Windows Security quarantine.
