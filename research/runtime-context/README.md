# Searchable runtime context

The desktop and MCP now search a retained runtime catalogue before applying the
card limit. Candidates carry up to eight direct same-object text hints, each with
field provenance and a truncation flag. This allows a generic field such as
`value` to be found through nearby text. Text is untrusted snapshot evidence; it
does not name the current occupant permanently or establish a gameplay effect.

The **Find more values** panel offers query/pagination and **Show card**. New cards
retain the original observation time until **Refresh value** reads the path
again. Existing cards keep newer readings. Selection requires the current
catalogue revision; search validates the original report digest even after later
refreshes. Both operations reach the same shared engine through two new MCP tools
(39 total). Neither search nor selection loads a helper or writes target state.

## Evaluation

[Protocol](PROTOCOL.md), [native runner](context_case.py),
[post-run audit](audit_postrun.py), [results](results.json).
Committed revision `da32512de2b2ad7dd7be31788c8b91c93326bc3f`, six exact binaries and
128 source identities were frozen before evaluation under
`test-results/runtime-context-v1`. The audit checks frozen identities, receipt and
report hashes, original observation provenance, helper completion/unload, cursor
rejections, accessible UI controls, completed native drivers and preview parity.

The unchanged owned runtime-path fixture produced 48 cards. Searching its
discovered `Speed` text returned three paths. Generic `Coefficient` fields carried
that complete hint plus an explicitly truncated 128-character hint. Four paths
refreshed in 453 ms; replay, stale selection/refresh rejection, null roots and
restored structure passed. Four helper operations used ten balanced pauses, with
zero helper Rust heap allocations/deallocations while paused. This is workflow
evidence from a simulated runtime API, not independent-game generalization.

The same frozen runner then used a fresh Blue Prince process (PID 86048, creation
`01dd6170d2e43e52`, session 2). Automatic discovery displayed 64 candidates from a
catalogue of 27,052 paths. These are paths, not distinct gameplay controls. A
query taken from discovered text, `RewiredSaveData.dat`, matched nine paths.
An unfiltered page beyond the initial display supplied four new cards, all of
which refreshed successfully in 2.735 seconds. They concerned update counters,
font features, shadows and masking. None establishes a useful gameplay control.
Both helper operations unloaded; all 161 managed pauses recorded zero helper
Rust heap operations. Exact replay reused the completed job and stale requests
were rejected. The game remained responsive and was closed from its own desktop
after a parent-session close request could not locate its window.

Both native runs used zero screenshots, gameplay input and gameplay writes.
Accessibility inspection verified Search/Show card without images. The test
drivers exited successfully; their owned GUI/MCP children were deliberately
terminated after inspection, so those child exit codes are not graceful shutdown
claims. The fixture host exited normally. No owned test app/game remains and the
Anode lease was released.

An explicitly exploratory replay queried `RookPos` in the previous Blue Prince
report after that label had been noticed in saved evidence. It found four paths,
including a generic `value` field. None appears in either the old or newly ranked
first 64 cards, although all new initial cards have context. This is evidence
that search reaches omitted observations; it also shows that preferring text
context does not solve automatic usefulness. It is not a held-out discovery test.

## Delivery and next milestone

All 205 workspace tests, strict workspace Clippy, formatting and the release
build passed before freezing. The frozen 39-tool smoke passed three practice
seeds with write/restore, shared session access, authentication/origin checks,
stale restoration rejection and restart invalidation. The audit passed for all
six binary hashes and 128 committed source identities.

Preview `0.1.0-20261021-163051` is running on the user's desktop (PID 80632).
Its four product binaries match the evaluated copies. All 29 history entries
were preserved exactly, along with `qwen3.8:latest`. Updating required a second
normal close request after re-reading the old process's changed main window;
the old app then exited normally. No user state was reset.

Weighted progress remains **31%**. The next acceptance target is a useful hidden
control with an independently verified effect. Further work should connect
candidate context with behaviour during relevant gameplay and compare discovery
methods under matched budgets. Search access and more readable labels earn no
causal or cross-game generalization credit by themselves. See the
[architecture decision and comparison plan](../../docs/DISCOVERY_DIRECTION.md).
