# Shared observation jobs — 2026-10-20

## Delivered boundary

The desktop and MCP now share durable, timed **capture-only** jobs. The human
can pause, resume, stop or take over an agent's work while a capture is running.
The controller returns receipts and progress without blocking on the native
engine worker. This is an integration of the MIT session-jobs experiment;
the original license and provenance are retained in `crates/reversebolt-jobs`.

Plans do not contain gameplay input, semantic-model calls, memory batch
advancement, experiments or live-game writes. The generic library's fake effect
executor is test machinery, not a production capability. Pausing and cancelling
stop later captures; they cannot retract a capture already dispatched.

## Reproduced evidence

The prototype's 29 tests were rerun before integration. The integrated workspace
passes 53 tests: 7 app tests (3 new controller-adapter tests), 12 core unit tests,
5 coverage integration tests, 23 job-state tests and 6 storage tests. Strict
Clippy and formatting pass. The release practice suite exercises actual process
edits/restoration, authentication, origin rejection, MCP and restart behavior.
There are 20 MCP tools, three of them new job operations.

The three adapter tests use a controlled worker to hold an actual command reply
pending. They verify responsive cancellation with ownership retained, exact
retry after rebind, and human takeover rejecting old agent control. They are
adapter-contract tests, not Blue Prince evidence. The library tests cover
intent/outcome failures, truncated final records, complete corruption, exclusive
ownership, quotas, duplicates, request-ID eviction, stale generations, ordered
events and deterministic command schedules. Simulated effects do not establish
real-game rollback or exactly-once writes.

## Independent game integration

Blue Prince, Steam app 1569580, build 25208956, in Anode's separate Windows
session 2 at 1280×720. The game stayed paused in the existing third test profile,
Day 1, facing the Closet doorway. This evaluation sent no game input or memory
writes. The two original profiles were not used.

`scripts/jobs-smoke.py` connects through the real stdio MCP adapter to an isolated
GUI engine. Its `exercise`, `takeover-check`, `prepare-restart` and `restart-check`
phases verified:

- A three-capture job completed with three durable successful checkpoints.
- An exact submit retry returned the original receipt without another capture;
  changed contents under the same request ID were rejected.
- Pausing before the first capture retained zero completed steps after waiting
  longer than its interval. Resume started the capture normally.
- During an owned native capture, status returned in **27.26 ms**. Cancellation,
  including fetching its next request envelope, returned in **53.29 ms** with
  phase `Cancelling`. The current capture finished, final phase was `Cancelled`,
  and no second capture started. These are one-run measurements, not latency
  guarantees or a load benchmark.
- A supplied HTTP `source: "desktop"` did not grant human authority.
- Detaching a paused job made it historical; reconnecting and retrying its old
  submit returned only the historical receipt, with zero new captures.
- Ordered event pagination and rejection of a future cursor worked through MCP.
- Desktop UI Automation invoked **Take control**. Job 4 became human-controlled
  and paused at zero captures. Agent resume, capture and new-job attempts failed.
  Human **Resume**, **Pause**, **Stop job** and **Release control** worked.
- The isolated engine was terminated while job 5 was running between captures.
  Reopening its data directory incremented boot, left the connection unbound and
  the job `Interrupted` at zero captures. Exact old submit retry remained a
  receipt; a new request carrying the old boot was rejected. All nine existing
  capture/evidence files were byte-identical. This real-process restart check
  was between captures; pending-intent crash semantics are covered by the
  separate state-machine/storage tests, not claimed from this run.
- After reconnecting, the desktop **Observe over time** button created human
  job 6. **Pause** left six recorded captures, and **Stop job** cancelled it.
  Both the UI and shared API reported the same creator, count and phase.

The first four job captures each read 16 MiB with zero unreadable or observed
mapping-change bytes. All four produced 1280×720 Windows Graphics Capture
images without a screen error. Capture durations were 440, 357, 359 and 346 ms;
memory reads took 10, 7, 8 and 9 ms respectively. The static scene supplied no
changing gameplay trajectory. No correlation or semantic discovery is claimed,
and this does not repair the earlier Blue Prince OCR failure.

The final code change after the native run only translates several internal
error messages into readable desktop messages. App tests, strict Clippy,
formatting and release smoke checks were rerun for that delivered binary.

## Local raw evidence

- `test-results/jobs-native-v1/jobs-report.json`: MCP receipts, phases, timings,
  takeover state, event counts and restart state.
- `test-results/jobs-native-v1/ui-human-paused.json` and `ui-human-stopped.json`:
  shared state following the human-started desktop flow.
- `test-results/jobs-native-v1/capture-hashes-before.json`: nine pre-restart
  evidence hashes; checked against the same files after reopening.
- `test-results/jobs-native-v1/captures/`: native images, read receipts and
  archived frame identities. No evaluation labels were supplied to discovery.
- `target/jobs-validation.log`, `target/jobs-app-final.log`,
  `target/jobs-clippy.log`: test/check outputs.
- `test-results/jobs-native-v1/desktop-delivery.json`: final published binary
  hashes and desktop replacement checks.

Generated evidence is local and ignored by Git. Version and the final release
smoke-report path are recorded in `docs/VALIDATION.md`.

## Remaining limits

There is one shared MCP actor, requiring coordinated sequential request IDs.
Multi-agent identity, streaming progress and resource fairness are unfinished.
The controller preserves a single engine owner and bounds command admission,
job retention, events, payloads and journal size. It is not a hard real-time
system: filesystem synchronization and operating-system calls can stall. A
late native renderer retains ownership until it actually settles.

The 64 MiB journal uses full checkpoints with no rotation/migration yet. A
storage fault fences job dispatch. Recovery tooling and archive compaction are
future work; do not delete history to evade quota or replay protection. Capture
files and the job ledger are separate persistence boundaries. The journal's
`evidence_valid` field indicates freshness in its binding, not truth, semantic
correctness or editing authority. Restart never grants an active process handle
or resumes unfinished work.

The next investigation loop still needs an observation interpreter, meaningful
action selection, scene/identity tracking and independent causal verification.
Those capabilities must be evaluated on changing gameplay without game-specific
offsets, labels or source-derived layouts.
