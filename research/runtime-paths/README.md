# Refresh runtime-derived value paths

Reversebolt's shared Rust core now builds typed scalar candidates from observed
runtime graphs. Each candidate carries an exact process identity, a static root,
runtime-qualified types and layout digests, and a sequence of fields/array indices.
The path contains no memory addresses. The runtime reader can follow it afresh,
checking each object type, declaring field, array length and scalar encoding.

This names the **current occupant of a path**, not a persistent object. A collection
slot may contain a different actor after reordering. Returning that actor's current
value is a path read; it is not proof that the original actor survived. Raw addresses
appear only in snapshot evidence. Neither candidates nor refreshes grant editing
authority or establish a gameplay meaning. Progress remains **31%**.

The candidate builder is in `reversebolt-core::runtime`, shared by the converter
and native reader. Human/agent dispatch and desktop cards are not connected yet.
The current desktop preview remains `0.1.0-20261021-122703`, with 29 history entries
and its model setting preserved. No new runtime controls are advertised there.

## Evidence

- [Original protocol](PROTOCOL.md), [response repair](PROTOCOL-v2.md),
  [v1 freeze](frozen-v1.json), [v2 freeze](frozen-v2.json),
  [results/artifact hashes](results.json), [post-run audit](audit_postrun.py).
- Four shared-core tests establish that addresses, values and catalogue numbering
  do not become path identity; generic type arguments do matter; another process
  identity is rejected; contradictory values/dangling edges are rejected; and
  unknown types/unread edges abstain. All **194 workspace tests** passed. The
  runtime/fixture builds, formatting and strict Clippy passed. After the small
  response repair, runtime tests and Clippy passed again before native evaluation.
- The first native run found a real integration defect. Refresh put process
  identity only inside `scope`; the loader required the common top-level identity
  envelope and rejected the response after unloading the helper. The raw values
  were present, but that case is retained as a failure. Its fixture exited 0,
  with eight starts and eight stops. The repair added the expected envelope.
- The repaired **13-case owned run** passed, with 31 balanced pauses and zero helper
  Rust heap operations while paused. It verified dynamic types, child objects
  through a collection, inherited fields, primitive array values, changing
  occupants at reused addresses, and collection reordering. Null roots, changed
  generic arguments, changed layouts, shortened arrays, an unavailable root and
  an unavailable field returned explicit refusals. Restoring structure allowed
  fresh reads again. A wrong process identity was rejected before loading.
- Three successive reads observed fixture identities 72, 92 and 112 at the same
  field address. After reordering, the selected slot yielded identity 121 from
  the other child role. The hidden scale agreed with each current identity; the
  inherited and primitive-array values also matched. The fixture exposes no
  offsets to the reader and aborts on metadata APIs inside a managed pause.
  It is an owned ABI simulation, not real-runtime or gameplay acceptance.
- In a fresh Blue Prince process, the unchanged reader completed all **4,267
  supported root slots** in 67 final batches and 93 preparatory snapshots.
  The shared core retained its requested cap of 4,096 scalar candidates.
  The fixed selection chose four two-step paths, and **all four refreshed**.
  Both helpers unloaded; the game remained responsive before owned cleanup.

## Limits of the real-game result

The four selected values were three array elements and `_size`, all declared in
`mscorlib.dll`. They did not change between discovery and refresh. This proves
the path mechanism against a real runtime; it does **not** prove a useful hidden
gameplay control or an effective semantic ranking strategy. The first available
values are not necessarily the values a person wants to edit. The next work must
prioritize useful application state and test effects, rather than count candidates.

The full discovery load/inspect/write/unload took **18,679 ms**. Targeted refresh
took **1,819 ms**, dominated by **1,764 ms** of fresh metadata preparation. Its
three stop-to-resume sections took 162, 208 and 146 microseconds (516 total), with
zero helper Rust heap operations. No hard real-time guarantee is claimed. The
reader rebuilds metadata on every invocation, so this is not yet an efficient
high-frequency watch service. Discovery graph coverage remains bounded/partial.

No screenshots, save selection, gameplay input or gameplay-field writes occurred.
The owned game, fixture hosts and workers exited; Steam stayed in the parent
session and the Anode lease was released. Paths are bound to the current process:
cross-restart recovery and logical object identity remain unfinished. A stable path
ID is not evidence of causality, actor identity or edit authority.

## Reproduction and provenance

The original source was committed as `b4936dbff070d67dadcc1af7a3a68c373c46e05f`;
the repaired source as `a8618c466391dd57c8bd296891b6088ca77662de`, both before
their respective native cases. Every frozen source file is checked against the
recorded Git revision. All five binaries for each version remain in separately
frozen `test-results/runtime-paths-v{1,2}/bin` directories and pass their hashes.
This avoids the shared-build-directory retention problem in the preceding study.

At that revision, build `reversebolt-runtime` and the standalone mock crate,
copy their binaries and the unchanged `runtime-objects` host to the experiment's
`bin` directory, then freeze/record the new run before testing. `mock_case.py`
takes a new output directory whose parent contains `bin`; `game_case.py` also
takes the exact PID, creation identity and session. The candidate converter
calls the same core API used by the native reader's path contract. The loader's
`--paths-file` accepts a bounded JSON array of those locators. No uncertain
remote operation may be replayed or unloaded blindly.

Primary API binding basis is the installed Unity 2022.3.18f1 IL2CPP SDK:
`il2cpp-api.cpp` defines the allocated assembly-qualified type name and matching
`il2cpp_free`; all such metadata work runs outside pauses. The installed game
exports that API, and this run tests its Unity 6000.0.58f2 build. Export presence
and these results do not imply compatibility with every Unity version.
