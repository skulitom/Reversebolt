# Automatic reviewed cards: retained discovery replay

Freeze source and inputs before replaying the two saved discovery snapshots:
independent-runtime-v2 (Hyperbolica) and comparison-planner-v1 (Manifold Garden).
Both snapshots and their original runtime reports are unchanged. We already know
the retained reviews contain two and zero accepted primary hypotheses respectively;
these cases were not selected for a successful card-publication outcome.

Run the shared Rust card inclusion on the original card lists. Validate the
review against the complete original inventory; do not rerun interpretation or
rewrite hypotheses, semantic claims, citations, paths, scalar types or values.
Record exact initially present, shown, added, omitted and removed-preview path
IDs, resulting revision and complete resulting runtime view. Repeat inclusion
and require an identical view with no additional insertions/removals. All new
readings must come from discovery and keep its timestamp and false edit authority.
Empty reviews must not invent findings. Reject snapshots containing edit, freeze
or comparison records; separate automated tests cover preserving those controls.

The runner archives Rust/manifests, itself, this protocol, both snapshots and
their two reports before building the file-only example. Record source/input and
binary hashes and audit them after both runs. Preserve failures and outputs;
never overwrite an existing run directory. No game execution, native helper,
capture, input, memory operation or model call is used. This tests routing saved
findings into cards, not correct gameplay meaning, effects or generalization.
