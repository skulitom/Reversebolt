# Durable runtime trials through desktop and MCP

Runtime trials now use a shared durable history. Preparation freshly reads one
displayed path and saves a bounded plan; application records intent before one
guarded write/read-back; restoration checks the original receipt and reference.
Repeated attempted IDs return saved outcomes. Restart expires unapplied reviews,
retains receipts and holds interrupted writes as uncertain. Storage failures and
unknown runtime/OCR writes fence further edits to that process.

Two frozen evaluations used the unchanged owned runtime-path fixture. Each passed
20 workflow checks, 19 helper operations, 44 balanced pauses, six confirmed
numeric writes, two helper refusals, eight saved trials and nine independent host
byte readings. UI restoration was actually invoked through native accessibility
and confirmed through the shared MCP history. The maximum pause was 66 µs in v1
and 95 µs in v2; both had zero helper heap operations while paused and all helpers
unloaded. No screenshots or gameplay input were used. The fixture host exited
normally; the runner deliberately terminated its owned app/MCP instances.

228 workspace tests, strict Clippy, release build, frozen 42-tool smoke and
post-run audits passed for those revisions. The native restart test followed an
unapplied review; interruption during a write and disk-failure recovery were core
tests, not native crash injection. The v2 preview preserved all 29 desktop history
entries and the selected observer model. Later direct-control work simplifies
the UI while retaining these underlying checks.

See [protocol](PROTOCOL.md), [retained results](results.json) and local evidence
under `test-results/runtime-trials-v1` and `test-results/runtime-trials-v2`.
These are byte-level fixture results. They grant no credit for real-game semantic
understanding, useful hidden gameplay effects, persistent actor identity,
complete rollback or independent-game generalization. Full-goal progress remains
31%. Manifold Garden is the next selected game; Blue Prince is no prerequisite.
