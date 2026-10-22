# Automatic review of input experiments

Implemented 2026-10-22 after the initial input-only investigation. Full-goal
progress remains **35%**. This change closes a manual handoff in the shared
engine; generated tests do not establish useful game discovery.

## Behavior

An input experiment now has ten capture steps and one final analysis step under
the same durable job owner. The final step checks all ten successful outcomes,
their job/plan/key identities, exact retained frame metadata and sampled-input
witnesses before measuring those images. It cannot start from a partial sequence
or silently substitute older passive images. It performs no extra capture,
input, OCR, runtime-helper operation or target-memory access.

The existing shared image/region analyzer runs automatically. If it finds an
eligible response and a local observer model was configured at submission, one
optional interpretation examines two measured before/after pairs. The model
name is fixed in the immutable plan; the existing observer verifies its installed
model around inference. Interpretation remains separate from measurement.
Negative replies, unavailable models and inconsistent receipts remain explicit.
No semantic name, editable card, numeric bound or write authority follows.

The result is saved before publishing a compact entry in
`live.input_experiments`. `read_activity_experiment` reads its retained evidence
and verifies result/image hashes without contacting a model or target. The
summary and desktop state checkpoint share the engine worker. The desktop job
shows that it is understanding the response; it does not add a report or unnamed
value cards. Later metadata discovery prefers the latest measured experiment's
exact image set instead of requiring an agent to collect its IDs manually.

A review intent prevents replay after a crash or a lost publication checkpoint.
Stop retains the raw outcome but suppresses publication of its source/context.
An identical completed request returns its saved receipt. Reading a result is
historical, not a current game-value read. Up to four summaries are retained in
the batch; old sessions default to an empty list. New batches clear live entries
while archives remain on disk. Restart still disconnects the old target; it does
not silently restore the live binding or expose these summaries as current.

## First-image timing correction

The original first Play image requested input history from the preceding passive
capture. Including the three-second return-to-game delay could overflow the
256-poll window and stop the experiment before any key press.

The first experiment image now begins its input window at that image's capture
start. Its pixels must still pass the capture-time coverage checks. No continuity
across the wait is claimed: the preceding passive-to-seed interval is unknown.
Every subsequent interval still uses the previous image's end and must pass the
unchanged input witness, focus, quiet-bracket and sampling-generation checks.
Normal capture behavior is unchanged.

## Verification

All **398 workspace tests** pass, plus a strengthened final command/checkpoint
integration test. Strict workspace/all-target Clippy and formatting pass.
[Verification hashes and source snapshot](results.json) retain the local logs,
including initial failures.

The generated LiveSession fixture includes a three-second wait, then produces
four bounded response episodes. It calls the actual internal review command
without a model configured, verifies its published and saved state, and checks
that startup discards the live binding. Exactly ten experiment images enter
analysis and the later metadata activity preparation. No extra capture or memory
read occurs; the fixture panics if target memory is read. Altered completion and
result files fail validation. Repeated review requests do not rerun analysis.

Six mocked interpretation cases cover positive, negative, unavailable model,
Stop during inference, a mismatched source and unchanged images. The unchanged
images skip inference entirely. All cases retain their actual result, leave
control lists empty, and refuse another inference when the publication entry is
lost but the durable intent remains. Controller tests additionally verify the
eleven-step plan, pinned model name, authorization, cancellation and no extra
capture from the review step.

Initial test compilation found fixture initializers missing the new optional
summary field. A later checkpoint test incorrectly expected startup to preserve
the live target; it now verifies the saved summary and the deliberate startup
disconnection separately. These were test corrections, not relaxed production
guards, and failed logs remain retained.

## Remaining work

No real game, native input worker, live model request or desktop preview ran.
Generated motion and mocked replies do not validate discovery quality, inference
quality, real capture timing or generalization. The fixed input policy still
needs an unchanged independent-game trial when native execution is available.

This step does not automatically run metadata discovery after reviewing input.
A later metadata pass currently makes its usual activity interpretation call;
it does not yet reuse the earlier reply. Selecting which input to investigate,
linking responses to controlling internal state and verifying useful named edits
remain unfinished. Historical result access after restart also remains separate
from the current-batch MCP reader.

The read-only desktop check still reports the quarantined executable missing.
No alternate app build was launched, no security setting changed and nothing
was uploaded. All new evidence and security-review materials remain local.
