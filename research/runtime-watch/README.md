# Internal-value tracking implementation checkpoint

The shared runtime job now supports 1–16 checks of selected internal-value paths.
Cards retain the latest 16 readings, explicit unavailable gaps, timing bounds,
report provenance and an omitted-reading count. **Watch changes** schedules six
checks through the same operation available to MCP. Discovery still runs once.
These histories describe path readings; they do not establish persistent actor
identity, gameplay meaning or an effective edit.

Controller testing exposed a pause bug: pausing revoked an in-flight read while
the job could advance, leaving its later pinned revisions inconsistent. A pause
now allows that already-started runtime read to finish and publish. Cancellation,
invalidated bindings and unresolved-operation fences still revoke work. Regression
tests cover in-flight pause/resume and cancellation during a pending pause, along
with bounded plans, replay and stale revisions. Existing visual/model pause tests
continue to pass.

All **213 workspace tests**, strict workspace Clippy and formatting passed after
the repair. The native runner also passed Python syntax checking. These checks
are not evidence that the real helper or desktop workflow has passed evaluation.
The [native protocol](PROTOCOL.md) and [runner](native_case.py) now explicitly
include an in-flight pause followed by successful resumed reads.

Native evaluation, artifact auditing, packaged smoke and desktop publication are
still pending at this checkpoint. The running desktop remains the preceding
tested preview. Weighted progress remains **31%**, with no independently verified
useful hidden gameplay control. The next acceptance target remains an effective
control in actual gameplay, rather than additional field counts or screenshots.
