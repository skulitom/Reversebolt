# From an agent hypothesis to a response comparison

Agents can now read the engine-checked runtime inventory, submit evidence-linked
questions through the same validator used by the local model, and prepare a
comparison directly from one retained hypothesis or alternative. The comparison
keeps its source question, quotations, exact discovery hash and competing paths.
The rendered-response assessment remains distinct from semantic confirmation.

`read_runtime_inventory` returns a type/field index, or a selection of up to 256
rows from eight types. `submit_runtime_hypotheses` pins the catalogue, runtime
revision, report hash and previous review ID. It accepts up to twelve proposals
and retains both accepted and rejected entries. A new review UUID owns its saved
request and receipt. Replaying an identical submission returns the archive and
never republishes an older review. Changed requests, stale evidence and incomplete
archives fail. Neither operation accesses the target, captures images or edits
gameplay values. The receipt is saved before the shared review changes.

`prepare_runtime_test.hypothesis_ref` chooses the review ID, primary hypothesis
path and one selected path. The selected path must be the primary candidate or
one of its retained alternatives. The engine rebuilds and checks the inventory,
then resolves the exact path without converting its name into a search query.
It can reach a path outside the 64-card display limit. After plan validation,
an explicitly selected off-board candidate joins the cards with its existing
metadata label. The normal fresh read, baseline, guarded write, recovery and
interruption rules still apply. The caller still supplies the replacement value
and four-action input plan; no automatic input planner or semantic promotion has
been added.

The original proposal and alternatives are immutable test provenance. They stay
available after catalogue replacement and restart. The summary includes the
hypothesis reference, so agents can associate completed comparisons with the
questions and alternatives they investigated. A historical prepare replay never
resumes a stopped test or re-reads a target. Manual tests without a hypothesis
reference remain supported, and old saved tests deserialize with no origin.

## Evaluation design and limitations

The native evaluator uses the existing owned ABI/visual fixture with one explicit
metadata label changed from `Coefficient` to `WalkSpeed`. `build_fixture.py`
generates this copy under the test output directory; the original source is
unchanged. The evaluator selects that observed label and supplies D/A holds to
the existing visual adapter. This deliberately coupled fixture exercises API
integration, provenance and restoration, not independent gameplay understanding.
Product code contains no fixture or game-specific selection rule.

The first attempt used the installed local model. It selected the relevant type
but returned no hypotheses even with the explicit label. The run stopped before
any input or edit; its receipt and child exits remain under
`test-results/hypothesis-tests-v1/case`. That abstention is retained as a model
limitation, not converted into a successful automatic naming result. The next
attempt uses the new external-agent submission API with observed field IDs and
exact quotations. It also proposes the unsupported name `Health` to check rejection.

Core regression coverage resolves a distinct competing field beyond the 64-card
limit in an 80-field report; rejects stale/incomplete reviews, invented alternatives
and changed provenance; and verifies durable test-origin recovery. Native checks
exercise submission replay, non-republication, candidate resolution, invalid
references, a complete baseline/change/recovery sequence and an independent byte
oracle. No captured images are manually reviewed. Exact build identities, counts,
outcomes and limitations are recorded in [results.json](results.json).

The final frozen run (`test-results/hypothesis-tests-v2`) passed 47 native checks,
265 workspace tests, strict Clippy, formatting, release build and the 51-tool smoke.
One agent proposal passed the reference checks; the invented Health label was
rejected. The complete comparison retained image-response medians of 131.806,
0.000 and 131.806 before/change/recovery, with original bytes independently verified.
A second path was prepared as a separate comparison and stopped without editing.
The native selected path was already on the board; beyond-card resolution is
covered by the synthetic report regression, not claimed as native coverage.

Eight helpers unloaded; 19 pauses balanced with zero helper heap operations while
paused. The maximum fixture pause was 78 microseconds. Eighteen images and twelve
input holds were evaluated automatically; two writes include the restoration.
All owned processes exited. Engine/MCP exit code 1 reflects deliberate termination
after assertions; the fixture exited normally. The first audit incorrectly required
the initial anchor frame to represent a completed input interval; it now follows
the protocol's existing rule that input matching starts at the second frame. The
native workflow was not rerun for that audit correction. The first strict Clippy
run also caught schema placement after a test module; moving the unchanged schema
before the tests resolved it.

This completes a shared handoff from a question to an explicit comparison. It does
not select informative inputs automatically, establish a gameplay meaning, compare
the effects of all alternatives, or satisfy independent-game generalization.
