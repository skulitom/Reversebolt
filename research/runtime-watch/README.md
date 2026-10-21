# Bounded internal-value tracking

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

All **213 workspace tests**, strict workspace Clippy, formatting and the release
build passed after the repair. The [native protocol](PROTOCOL.md) and
[runner](native_case.py) were committed before evaluation. Six binaries and 130
source identities were frozen at `841aae1797954612d29a9ff8eef47b30533d7716` under
`test-results/runtime-watch-v1`; the [post-run audit](audit_postrun.py) checks their
identities against retained reports and [results](results.json).

The real desktop/shared engine and 39-tool MCP completed 22 refreshes of two paths
in the unchanged owned runtime-path fixture. The first four readings preserved
normal/null/restored/normal status, and differences did not bridge the unavailable
gap. Sixteen more checks exercised history rollover. A two-check watch paused
during its first helper, published exactly that reading, and resumed successfully.
Exact replay reused its job, a fresh stale request was refused, and cancelling a
paused queued watch dispatched no helper. The final 16 readings retained six
omitted readings; the fixture's changing Identity had 15 adjacent differences,
while its Coefficient stayed constant. These paths do not imply actor continuity.

Refresh durations were 68–80 ms in this fixture, excluding deliberate spacing;
this is not a game performance guarantee. All 23 helper operations completed and
unloaded. The 48 managed pauses were balanced, with zero helper Rust heap
allocations/deallocations while paused. Accessibility inspection found the enabled
Watch changes button and the constant-value trace on the visible card; the changing
trace was also present in the accessibility tree below the viewport. No screenshots,
gameplay input or gameplay writes were used.

The driver and fixture host exited normally. The runner deliberately terminated
its GUI/MCP children after inspection, so their exit codes do not demonstrate
graceful UI shutdown. Owned test processes are gone and the Anode lease was released.
The frozen 39-tool smoke passed three practice seeds with edit/restore, shared
session access, authentication/origin checks, stale-restore rejection and restart
invalidation. The final source/binary/report/UI/cleanup/desktop audit passed.

Preview `0.1.0-20261021-171456` is running on the user's desktop (PID 93476). Its
four product binaries match the evaluated copies. All 29 history entries were
preserved exactly, along with `qwen3.8:latest`. The previous preview closed normally.
Weighted progress remains **31%**: this fixture validates measurement and control
flow, with no independently verified useful hidden gameplay control. The next
acceptance target remains an effective control in actual gameplay.
