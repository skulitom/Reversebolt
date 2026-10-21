# Batched runtime traversal v1 — before native evaluation

The previous live-object study filled its 1,024 nodes with roots, so it never
discovered a new child in Blue Prince. This revision moves the reusable reader
into `crates/reversebolt-runtime` and evaluates deeper traversal, learned runtime
types, reference/scalar arrays, short strings and explicit continuation. No
game-specific class names, field names, layouts or addresses enter the adapter.
It is not yet called by the desktop application or MCP dispatcher.

Freeze source/shared-host/binary hashes and commit source before native runs.
Record that source revision with the execution results so subsequent code changes
do not require retaining another implementation copy. Retain original failures.

Owned ABI case: use 130 distinct root slots pointing to a head actor. Its child
box, text objects, arrays and two additional actors have no direct static roots.
Only three class definitions are initially enumerable; four encountered types
must be described between pauses. The callback fixture aborts if a metadata API
is called while stopped. Require three final batches (0–64, 64–128, 128–130),
depth-three actor values, inherited fields, correct cyclic edges, three primitive
array values, a 64-of-70 reference-array prefix and a 128-of-200 text prefix.
Require fresh snapshots after type preparation, zero helper Rust heap operations
in every paused section, successful unload and balanced stop/resume counts.
Then request a continuation at root 64 with the saved schema fingerprint and
require coverage 64–130. Reject malformed root input before loading, and reject a
wrong schema fingerprint before any managed pause. Close the owned fixture.

If that passes, launch Blue Prince through Anode, select no save, send no gameplay
input, and run one unchanged full discovery request. Require at least one new
non-root object, one successful deeper main-game scalar read, some described
encountered types, some successful array/text reads and zero helper Rust heap
operations during pauses. Record actual snapshot, warmup and pause counts, all
truncations and budgets, complete root coverage or the continuation cursor.
If root coverage is incomplete, allow exactly one continuation using the returned
cursor and matching fingerprint. This is coverage continuation, not a rerun of
the first case. No retry after an uncertain or failed helper thread. Verify helper
unload, target responsiveness, close the owned game and release the Anode lease.

Limits: 64 roots/batch, 1,024 objects/snapshot, 16,384 reads, depth 4, 64 elements
per supported array, 128 UTF-16 units per string, three type preparation rounds
per batch. Check a 5 ms pause budget between operations. Metadata preparation has
a 20 s deadline; start no new batch after 18 s. Snapshot JSON has a 24 MiB budget
(schema/receipts are additional). Runtime calls and OS reads cannot be forcibly
cancelled to satisfy a hard real-time deadline. Array value structs/enum decoding,
native-only roots and complete container traversal remain unsupported/partial.

Only metadata pointers are retained between pauses. Object addresses and IDs
remain snapshot-local, and raw reads are never promoted to editing authority.
The collector is resumed before metadata work, allocation or serialization.
Root fingerprints describe the ordered runtime schema, excluding object values
and addresses. Continuation must match this fingerprint; root-scan completion
does not mean graph completeness or useful semantics. Progress stays 31% until
useful validated capabilities warrant credit under the existing rubric.
