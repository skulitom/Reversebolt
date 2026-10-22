# Continue an investigation after an agent submits hypotheses

Submitting an agent review previously set `play_context` to null, dropping valid
activity evidence and preventing the comparison planner from continuing. The
shared submission path now retains that context when the original completed
review, discovery report, investigation, batch and frame/input metadata still
match. It retains the image model's false verdict and uncertainty without
promoting a hypothesis to a control.

The receipt reports `activity_transfer`, including why evidence was discarded.
The context's optional `interpretation_source` preserves the original model and
review ID separately from the agent's new metadata questions. It follows further
handoffs and comparison-test provenance. Legacy contexts keep their format until
attribution is attached; no original evidence archive is rewritten.

## Verification and retained failures

The app integration test uses the real command, persistence and replay code with
a synthetic target that panics on reads or captures. Repeated submissions preserve
the original source. Replaying an older request returns its archived result and
does not replace the current review. Changed requests and stale review cursors
are refused. Changed image metadata discards activity, and archived replay still
works after engine restart without reconnecting. Main-board cards remain unchanged.

The [frozen replay protocol](PROTOCOL.md) uses the existing Hyperbolica report,
two metadata questions and fifteen retained images. V1 stopped before planning
because recomputed evidence did not equal the parsed saved value. V2 retained
the exact mismatch: the default JSON reader changed one numeric value by one
binary64 step. The recomputed value exactly matched the original file literal.
The [roundtrip correction](ROUNDTRIP.md) enables `float_roundtrip` for workspace
JSON parsing and adds a bit-exact persistence regression. The acceptance check
remains exact; no epsilon or evidence rewriting was introduced.

V3 passes. All fifteen image hashes and their recomputed measurements remain
bound to the prior replay. Reused questions validate under a new agent review ID.
The one eligible comparison and four 198 ms holds remain the same; its resolved
test origin retains the original image producer and ambiguous interpretation.
A changed image reference discards the context. No model, editor, runtime loader,
game interaction or field edit ran in this replay. The proposed comparison has
not been executed. [Results and artifact audit](results.json).

All **299 workspace tests** and strict all-target Clippy pass. Coverage includes
altered input samples, report, investigation and batch, incomplete reviews,
invalid source identity, context serialization and comparison-store recovery.
Only the retained case's historical JSON/hash bindings were checked through this
replay; other historical data is not declared exhaustively compatible. Hash
mismatches remain failures and never silently update identities.

V1's example executable was overwritten by its next ordinary debug build, so its
bytes cannot be reverified; its pre-run hash, source and failure remain. V2's
binary was preserved after matching that attempt's hash, and V3 archived its
binary before execution. No quarantined executable was copied or run.

These are shared-engine and persistence fixes, not new agent intelligence,
gameplay control or generalization evidence. The source update is unpublished;
Windows quarantine and the pending review submission remain unresolved. Overall
progress remains **35%** and the full goal stays active.
