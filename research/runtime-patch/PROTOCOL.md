# Guarded runtime numeric trial v1

The watcher measures internal paths but cannot edit them. Add one explicit numeric
patch primitive that resolves the path again, compares exact type/width, the full
observed address chain, field address and expected bytes, then attempts one write
and read-back in that same bounded managed-world pause. Accept numeric scalars
only, finite changes of at most 20, and plans at most 60 seconds old. Reject
read-only/literal/static terminal fields, invalid object extents, guarded pages,
non-writable pages and executable pages. Do not change page protections.

Preparation performs no target write. An inverse requires this exact confirmed
receipt and preserves the original chain/address while swapping expected and
replacement bytes. Refusals must report no write attempt; any attempted write
without complete transfer and exact read-back is uncertain. Never automatically
retry an uncertain result. Equality of addresses does not exclude reuse and
does not prove persistent actor identity. Neither byte restoration nor a helper
receipt establishes complete gameplay rollback or a useful gameplay effect.

Keep this primitive below app/MCP dispatch until durable effect intents, receipt
recovery and user review are connected. Existing automatic discovery/refresh must
continue to pass with the optional patch absent. Do not publish new editable cards
based only on this low-level test. Preserve the running tested desktop preview.

Before native evaluation run core boundary tests, workspace tests, strict Clippy
and a release build. Commit the implementation, protocol and runner, then freeze
exact binaries and source hashes. Preserve failures; do not alter an evaluated
method and overwrite its results.

Use the unchanged runtime-path fixture DLL in an owned headless host. The host
advances its existing MockTick only on evaluator requests, making reference
replacement deterministic. This is a controlled ABI fixture, not an independent
game or real moving-GC test. No UI or game input is needed. Obtain the path from
the actual discovery and candidate converter, then prepare plans through the core
CLI. The evaluator may select fixture labels; production must have no such rules.

Test write and guarded restoration, duplicate expected-byte refusal, numeric-type
mismatch, changed chain and field addresses, actual reference replacement, null
roots, changed runtime type, read-only array storage, expired plans and mismatched
process scope. Verify values independently inside the host using ReadProcessMemory;
those readings score outcomes only and never enter candidate generation. Inverse
restoration after reference replacement must refuse. Explicitly clean up its
remaining fixture change with a fresh reviewed path, rather than claiming that
this is the refused original restoration. Invalid or expired requests must fail
before helper loading. Do not induce partial native writes or target crashes;
their uncertain classification is covered by receipt refusal tests, not a native
failure-injection claim.

Retain request/report/loader output and process identities. Check all completed
helpers unloaded, every managed pause resumed, zero helper Rust heap operations
while paused, and the owned host/loader processes exited. Run the existing frozen
MCP smoke to ensure discovery-facing compatibility. Weighted progress remains 31%
unless independently verified useful effects earn credit under the stable rubric.
