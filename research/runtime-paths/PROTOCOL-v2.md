# Runtime paths v2 — response-envelope repair

The v1 owned run completed discovery and performed its first refresh, returning
four correct scalar values. The loader then rejected the result because refresh
placed the process identity only under `scope`, while the loader requires the
common top-level `pid`, `created`, `session` envelope. Its helper unloaded, and
the host exited normally with eight starts and eight stops. That failed run is
retained unchanged under `test-results/runtime-paths-v1`; it is not a passing case.

Repair only the refresh response by including the common identity envelope.
Keep the original protocol, fixture and both runners unchanged. Run a fresh
owned fixture under `runtime-paths-v2`, then the unchanged real-game case only
if all fixture assertions pass. Retain separate exact binary copies and freeze
the repaired source revision before running. All acceptance criteria, limits,
coverage claims and 31% progress remain as in PROTOCOL.md.
