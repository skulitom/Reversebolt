# Background image interpretation — 2026-10-21

**Subsequent change:** archived inference/validation failures now leave a scoped
warning and contribute no findings while other captures continue. Persistence,
scope and worker failures still stop the pipeline. The historical missing-model
stop result below describes the earlier build, not current behavior. See
[the failure-isolation regression](../../research/observation-resilience/README.md).

The first local visual review now runs alongside capture. In the automated
native fixture, the sequence reached **moment 12 while the review of moments
1–2 was still outstanding**. It completed all 16 captures and both reviews.
Cards show the age of their source image and keep earlier values explicitly
associated with that earlier moment. This improves observation plumbing;
it does not improve model accuracy or establish continuous object identity.

## Ownership and publication

New model-enabled observation jobs retain their sixteen ordinary capture steps.
The second step can start a background review; the final step waits for the
first review, then reviews the latest pair before completing. A two-capture job
has one blocking review. Existing serialized plans without `background_model`
keep their original blocking semantics.

The worker receives immutable copies of PNG bytes verified against the original
sequence. It has no process handle, memory access or publication capability.
Only the engine worker publishes results. It rechecks investigation, memory
batch, process identity, image checksum and geometry, then validates and saves
the entire group before exposing findings. A late finding fills its original
frame slot; later slots remain missing. It is never relabeled as current.

There is at most one model worker. Its job, evidence id, model, source frame ids,
observation time, start time and stopping status appear in `state.analysis`.
The cancellation signal belongs to the whole job, including intervals between
capture steps. Stopping a queued job does not cancel another job's worker.

Capture and interpretation have distinct settlement states. Pause or cancel can
stop the capture sequence while an already-issued model request is still
draining. That request stays visible; resume and target-changing mutations are
rejected until it settles, and another queued job cannot take over. A resumed
job receives a new cancellation signal without clearing the old worker's signal.
The final capture's ledger operation remains pending until both reviews settle.
Model steps allow thirty seconds per ledger poll, while cancellation/readiness
checks occur at approximately 25 ms intervals. This covers two sequential model
passes without classifying normal inference latency as a capture timeout.

Failure is reconciled before the next capture command begins. A capture already
in progress can finish before the failure is handled. Failed model work stops
later steps and is never automatically retried. Cancellation is checked between
model calls and before publication; an active HTTP request is allowed to settle.
A client timeout does not prove the separate Ollama server stopped computing.
Restart clears transient worker state, revokes live scope and starts no inference.

## Automated evidence

The fixture was an owned WinForms window in Anode displaying the already seen
Celeste idle image. It supplied neither labels nor memory layouts to inference.
The installed local model was `qwen3.8:latest`, with digest
`22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643`.
No model was downloaded. There was no gameplay input, new game recording or
manual screenshot inspection.

The normal automatic connection completed 16 captures and produced two
provisional appearance cards. Ten additional captures completed while the
first review remained outstanding. Capture-start spacing was **1,353.35 ms
median**, **1,579.27 ms maximum**; the gap between moments two and three was
**1,360.65 ms**. This removes the previous model-inference pause from that
sequence, but remains much slower than 10 Hz. These are one-run fixture timings,
not a renderer or game performance guarantee.

A separate four-capture visual job was paused while its first review was active.
Pause acknowledged in **47.00 ms**. Capture stopped at two moments, the cancelled
pass made no crop-review calls and published no findings, and premature resume
and detach were rejected. After the worker settled, resume collected the remaining
two moments and produced a fresh card. Maximum measured status latency was
**32.00 ms**. No game writes or numeric memory matches were introduced.

A deliberately absent model failed after the second capture of a four-step job.
No third capture started, the worker drained, and the previous findings remained
unchanged in their earlier slots. Restart preserved the model selection and exact
history, increased the job boot, cleared live scope and resumed no analysis.

UI Automation verified a short named card, its visible age (“Seen 22s ago”, later
“Seen 38s ago”), and an Inspect view retaining the full explanation and original
“Observed in moment 4” source. These checks used accessibility text, not screenshots.

All **94 workspace tests**, strict workspace Clippy and formatting pass. Three
new tests cover concurrent capture with a deliberately blocked reviewer, original
frame retention, cancellation/scope/geometry rejection, and pausing between
capture steps without losing ownership. The final capture-boundary reconciliation
change also passed the complete app tests and strict workspace Clippy.

Preview **0.1.0-20261021-015824** is running on the user desktop. All eight prior
history entries and the selected local model were preserved. Published executable
and OCR-model hashes match their manifests. The final published binary completed
a new native model pass and passed the missing-model stop check; its 24-tool
release smoke also passed (`test-results/smoke-569a837b/report.json`). The owned
test windows were closed and the Anode lease released.

Raw evidence is in `test-results/background-observer-v1/`, including the native
trace, model receipts, UI inspection and failure/restart checks. The
[compact record](../../research/background-observer/evidence/native-v1.json)
retains hashes and results without redistributing images. Reproduce with an owned
engine and `scripts/image-fixture.ps1` window in Anode:

```powershell
python scripts/background-observer-smoke.py DATA_DIRECTORY FIXTURE_PID INSTALLED_MODEL
```

An optional fourth argument selects the executable directory. The script leaves
the owned engine/window available for accessibility checks; close them afterward.

## Remaining work

The full weighted goal remains **21/100**, with unchanged category credits.
This is refinement of the integrated observation path, not general gameplay
understanding. Model proposals remain uncertain and independent per pass;
old and new similar-looking cards do not prove the same object. Continuous
movement, animation, hidden rules, memory correspondence, validated edits and
recovery across game changes remain incomplete. The failed patch tracker stays
experimental. Faster acquisition and a better moving-object observer still need
separate measurement and acceptance gates.
