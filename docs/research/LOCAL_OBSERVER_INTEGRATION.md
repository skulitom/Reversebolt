# Local visual observations in the shared session — 2026-10-21

## Implemented behavior

The desktop and MCP now share a persisted **Local vision** choice. Selecting an
installed local Ollama vision model checks availability and capabilities without
downloading weights or running inference. Setting it to `null` disables model
work in future automatic jobs. Existing plans retain their original choice.

Automatic application connection still starts a bounded 16-moment capture job.
If configured, it interprets the latest two source images after captures 2 and
16. Explicit MCP jobs can supply `local_model` with at least two captures; absent
or null retains the previous capture-only behavior. A two-capture job runs one
model pass, not two. No game name, process title, memory, annotations or fixture
state enters the model request.

The existing image-only proposal pass is followed by the crop review measured
in [the preceding experiment](CROP_REVIEW.md). Actor/held-component proposals
need supported later-frame crop evidence. Original coordinates are retained;
the less accurate refined boxes are not adopted. Other, uncertain or malformed
reviews do not publish observations. Source images, capture timing, original
proposal, every completed review, raw responses and failure/cancellation state
are saved under `captures/model-observations/<id>.json` before publication.

Accepted items become ordinary shared-session visual tracks from engine source
`local-model`. They have generic model-proposed names, provisional text values
and source-image coordinates. Each pass assigns new keys: matching appearance
does not establish persistent identity. These text tracks cannot enter numeric
memory matching or enable edits. The cards show short descriptions and Learning;
later captures without renewed evidence show Earlier. Inspect retains the full
explanation, source moment and evidence ID.

## Ownership and failure behavior

Model work runs inside the same engine-owned capture operation. The controller
continues answering status, pause, cancel and takeover while inference runs.
A shared flag is checked between model stages. The current HTTP request can
finish, but the remaining pass and publication are skipped when cancellation is
observed. Already completed evidence is not undone. Ownership is retained until
the worker returns. Cancelling an unrelated queued job does not cancel the
current operation.

Model steps use a 15-second ledger poll cadence instead of the one-second native
capture cadence; completion and cancellation signals are checked every controller
iteration. A 90-second pass budget is checked between stages, so an active
request can extend elapsed time by up to its 90-second generation timeout and
bounded metadata checks. The image count, pixel/byte budgets and maximum six
proposals remain enforced. There is no per-frame model invocation, request
replacement, automatic retry or resumption after restart. An HTTP timeout does
not prove that the independently running Ollama server stopped computation.

A failed model pass stops that job. Its preceding capture may already have
completed; failure is not a rollback. The job outcome does not label that
partial operation an uncaptured moment. Source image validation happens before
inference; process, image hash and geometry checks run again before observation
publication. The entire observation group is validated and saved before any
track is exposed. A later invalid item or failed evidence save cannot publish
the earlier items. The raw pass file records proposed observations, not a claim
that they were published. The job journal, visual evidence and session file are
separate durable writes, not one atomic transaction.

## Verification

All **85 workspace tests** pass. Strict workspace Clippy and formatting pass.
The final UI-only changes passed Clippy and native UI Automation. An additional
assertion in the existing cancellation test was rerun after the main suite to
cover cancelling an unrelated queued job. Added checks cover pinned model plans,
legacy capture serialization, automatic scheduling, cancellation ownership,
unverified text publication, original coordinate preservation and rejection of
partial failed/cancelled passes. Existing image-binding tests now exercise group
validation and failed group persistence.

An automated MCP check used a WinForms window in Anode displaying one existing
development image. This is replay plumbing, **not a live game or fresh
generalization test**. Inference received real native window captures; the
model did not receive the source filename or any labels. The selected installed
tag was `qwen3.8:latest`, digest
`22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643`.
Its upstream identity/license remains unverified; no weights are redistributed.

The first automatic pass produced one card, “Pink-haired pixel character”,
after moment 2, with no manual capture/start-model command. Proposal time was
10.847 s and its one eligible crop review took 3.575 s. Pause succeeded. During
a second pass, cancellation was acknowledged in **35.97 ms**; the active proposal
finished, zero crop reviews started, and no new track was published. Maximum
observed status-call latency was **113.15 ms**. These are one-run timings, not a
performance guarantee. Invalid model selection preserved the existing setting.

Restart preserved the configured model and exact history, revoked the live
connection and resumed no old job. The final published UI build then completed
a new two-image pass and showed “Pink-headed pixel character”, Character,
Learning and a 110-character description. UI Automation verified that Inspect
kept the full explanation and source moment. No manual screenshots were viewed.

A final three-step job deliberately requested an absent model. It failed after
its second capture, started no third capture, preserved the previous findings,
saved its failure receipt and made no game writes. The first checker incorrectly
expected the failed step not to advance the cursor; the ledger counts every
settled outcome. The corrected check inspected that same terminal job without
replaying work.

The 24-tool release smoke suite passed against the published executables,
including practice edits/restoration, authentication, shared history and
restart invalidation. Native test windows were closed and the Anode lease
released. User desktop preview **0.1.0-20261021-011044** is running with local
vision enabled, all seven preceding history entries retained, and one new
configuration event. Executable and OCR-model hashes match the published
manifest; the desktop launcher points to this version.

Raw evidence: `test-results/local-observer-integration-v1/`; full test/Clippy
logs use the `test-results/local-observer-integration-*` prefix. Published smoke:
`test-results/smoke-8707ecf0/report.json`. The
[compact record](../../research/local-observer-integration/evidence/native-v1.json)
retains their hashes and results without shipping the image.

## Remaining limits and next work

This adds an automatic provisional appearance source, not causal understanding.
It still confuses objects, misses targets, uses correlated same-model review,
adds seconds of latency and makes no object continuity claim between passes.
Motion quantities, hidden behavior, internal-state correspondence, scoped edits,
recovery and withheld-game evaluation remain incomplete. Observation credit
increases from 3/15 to 4/15 for the integrated image-derived source; the full
weighted goal is **21/100**, with no new causal or generalization credit.

The original blocking-pass check is retained in `scripts/local-observer-smoke.py`
for its historical build. For current builds, run `scripts/image-fixture.ps1`
and an isolated desktop engine in Anode, then use the updated concurrency check:

```powershell
python scripts/background-observer-smoke.py DATA_DIRECTORY FIXTURE_PID INSTALLED_MODEL
```

The script leaves the owned engine/window available for UI Automation; close
them afterward. It does not install a model, launch or control a game, or supply
synthetic object labels to the observer.
