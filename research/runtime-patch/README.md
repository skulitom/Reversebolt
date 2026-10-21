# Explicit runtime numeric patch primitive

The shared core can prepare a bounded numeric patch from a freshly read runtime
path, and the native helper can attempt it after resolving that path again. It
compares the full observed address chain, scalar type/width, field address and
expected bytes in the same managed-world pause as the write and read-back.
Preparation and inverse preparation perform no target writes. Integer parsing
preserves exact 64-bit values; non-finite values, unsupported scalar kinds, no-op
changes, deltas above 20 and plans older than 60 seconds are refused.

Read-only/literal/static terminal fields and invalid instance extents are excluded.
The write also requires committed writable, non-executable storage without guard
protection. It does not alter page protections. A write attempt without complete
transfer and matching read-back is uncertain; it is not reported as a refusal.
Only the exact confirmed receipt can prepare an inverse, which keeps the original
chain/address guards. Address equality still cannot exclude object reuse.

This is an underlying edit mechanism, **not yet an editable runtime card or MCP
action**. Existing automatic discovery and refresh remain read-only with respect
to gameplay fields. The command-line preparer is `reversebolt-runtime-patch`;
the explicit loader option is `--patch-file`. No app flow calls it yet.

## Frozen native evaluation

[Protocol](PROTOCOL.md), [runner](native_case.py), [owned headless host](host.py),
[post-run audit](audit_postrun.py), [results](results.json).

Commit `dc9a8b78f669915c78363432c96fef26cd0d7db0`, seven exact binaries and 132 source
identities were frozen under `test-results/runtime-patch-v1` before evaluation.
The unchanged runtime-path fixture was advanced only on evaluator requests. Its
numeric candidate came from real discovery and the shared converter; fixture
label selection belongs to the evaluator, not the production implementation.

All 21 native cases passed: 19 helper operations and two preflight refusals. Four
numeric writes were confirmed: the initial change, its restoration, a change
after reference replacement, and explicit cleanup through a fresh plan. Ten
helper requests refused to write, covering changed expected bytes, scalar type,
reference chain, field address, actual root replacement, null roots, changed
runtime type and read-only array storage. Expired plans and a different process
identity were rejected before helper loading. The preparer separately rejected
a non-finite requested value without creating a plan.

Nine independent host-side memory readings verified the changed, refused and
restored bytes. In particular, an old restoration refused after its root changed,
leaving the value altered. A new plan then restored the fixture's numeric value;
this is not counted as successful execution of that refused original restoration.
These readings score byte outcomes only and never feed candidate discovery.

All 19 helpers unloaded. All 42 managed pauses were balanced and recorded zero
helper Rust heap allocations/deallocations while paused. Maximum stop-to-resume
time was 63 microseconds in this fixture. The host and all 29 child commands exited;
three nonzero child exits were the expected invalid/expired-request refusals.
The test used no screenshots, graphical desktop, gameplay input or real game.

All 217 workspace tests, strict workspace Clippy, formatting and the release build
passed. The frozen 39-tool MCP smoke passed shared-session access, three practice
edit/restore seeds, authentication/origin checks, stale restoration refusal and
restart invalidation. The source/binary/request/report/oracle/cleanup audit passed.
Native partial-write failures, process-crash recovery and moving-GC behaviour were
not induced or established by this evaluation.

## Next integration boundary

Connect the primitive to the existing shared controller and durable effect history:

- Preparation must retain the exact fresh report, plan, process identity and
  catalogue cursor, and expose a concrete review without writing.
- Apply/restore must save intent before instrumentation, validate the exact receipt
  before reporting success, and preserve uncertainty after interruption or missing
  evidence. Replaying an already-attempted operation must not issue another write.
- A refused restoration must retain the original change and its unresolved gameplay
  consequences. Fresh reads alone cannot prove full rollback or causal authority.
- Cards and MCP must use the same operations, ownership checks and saved history.
  Confirmation of bytes must not promote a candidate to a verified gameplay control.

The desktop remains preview `0.1.0-20261021-171456`, with its tested internal-value
watcher available. Weighted progress remains **31%**. Actual useful hidden controls,
effect verification, persistent actor identity and independent-game transfer remain
unverified; this primitive makes their experiments possible but does not prove them.
