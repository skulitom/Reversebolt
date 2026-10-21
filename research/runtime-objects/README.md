# Live object snapshot feasibility

The unchanged Rust reader reached live instance fields in Blue Prince on
2026-10-21, without game-specific names/offsets, screenshots, save selection or
gameplay input. Two snapshots each read **1,871 instance scalar values**, including
**45 in the main game code image**, from 1,024 reached objects. Both completed
with zero memory-read failures or helper Rust heap operations during suspension.
The helper unloaded and the game remained responsive before owned-process cleanup.

This advances the previous static-value probe to existing objects, parent fields,
reference edges and explicit snapshot lifetimes. It does not establish player
identity, gameplay meaning, edit authority or durable object identity. Progress
remains **31%** and the running production preview is unchanged.

## Evidence

- [Frozen protocol](PROTOCOL.md), [21 source/binary hashes](frozen-v1.json),
  [results and raw artifact hashes](results.json), and [post-run audit](audit_postrun.py).
- Owned Rust ABI fixture: three successful loader cycles, six snapshots, six
  paired stop/resume operations. The fixture repeatedly replaces cyclic objects,
  reuses addresses and exposes an inherited scalar. Values and links were
  coherent in every snapshot. Four consecutive snapshot pairs showed the same
  root address with a different fixture identity. That directly demonstrates why
  an address cannot be treated as durable object identity.
- A wrong process creation identity was rejected before instrumentation. Shared
  loader hash/session/existing-output guards were already tested by the prior
  [static-value study](../runtime-probe/README.md); their source is reused unchanged.
- Blue Prince PID 78756, creation `01dd615844717331`, Anode session 2, Unity
  6000.0.58f2: prepared 16,153 class descriptions, 85,015 fields and 3,288 static
  reference roots in 952 ms. Open generic definitions are omitted explicitly.
- Paused read sections measured **7,302 and 6,057 microseconds**; including
  stop/resume calls, **7,521 and 6,225 microseconds**. Full load, preparation,
  two snapshots, serialization and unload took 3,852 ms. No hard real-time
  deadline is claimed.
- Both snapshots hit the 1,024-node capacity. **492 nodes had uncatalogued runtime
  types**, so their instance fields were not read. This is partial coverage, not
  complete game discovery. Budget refusals and reference edges remain in raw data.
  Root enumeration filled the capacity before deeper traversal: every reached
  game object was at depth zero, 102 root references and 473 child references per
  snapshot were refused by the node budget. The owned cycle fixture's two objects
  were also both static roots. Its cycle checks establish edge correctness, not
  discovery of additional child objects; that needs a separate deeper fixture.
- Rust build, strict Clippy, formatting and an early-error resume-guard unit test
  pass. The owned stress test is an ABI simulation, not an independent runtime.

An attempted Unity 2022.3.18f1 fixture build terminated before compilation because
the installed Editor has no active license. The original log is retained. No
license activation or retry occurred. The mock is not a replacement claim of
real-Unity fixture correctness; the installed game supplies the real-runtime
feasibility evidence. The fixture source remains available for a future licensed
build and is currently unverified.

## Lifetime and suspension contract

Before stopping managed threads, the helper reads runtime-provided class/field
descriptions, sizes, inheritance and static storage. It reserves and touches all
snapshot storage and warms the kernel read and timing paths. While stopped, it
performs bounded memory reads, looks up already-owned schema, records edges in
preallocated buffers, and calls the trivial object-class accessor on references
read from supported reference-typed fields. It does not enumerate metadata,
create GC handles, format text, log or allocate/free Rust heap memory in that
section. An instrumented allocator records helper heap operations; it does not
measure every native runtime allocation.

The resume guard runs before any formatting, serialization or owned-buffer
destruction, including on a Rust early-error/unwind path. Native access violations
or a stuck runtime call are not recoverable by Rust unwinding. The loader's
uncertain-thread rule still applies: do not retry or unload under live execution.

Object IDs are scoped to one snapshot. The reader retains copied bytes and
reference paths, never object pointers as future editing authority. Snapshot
addresses may be reused. Stopping managed threads does not claim atomicity for
all native engine subsystems. Metadata preparation, runtime attachment and brief
suspension can perturb timing and lifetime. This is explicit research
instrumentation, not an automatic production connection feature.

The contract was checked against official sources in the installed Unity SDK:
`il2cpp-api-functions.h`, `il2cpp-api.cpp`, `vm/Object.cpp`, `gc/BoehmGC.cpp`, and
`external/bdwgc/misc.c`. Their hashes are in the results; no implementation was
copied. The SDK's stop/resume functions acquire/release the collector lock around
managed-world suspension. This is why metadata work and heap operations are kept
outside the paused section. Unity's published
[runtime API declarations](https://raw.githubusercontent.com/Unity-Technologies/SimpleUIDemo/master/Tiny3D/Library/DotsRuntimeBuild/artifacts/Stevedore/il2cpp/libil2cpp/il2cpp-api-functions.h)
and [collector implementation](https://raw.githubusercontent.com/Unity-Technologies/SimpleUIDemo/master/Tiny3D/Library/DotsRuntimeBuild/artifacts/Stevedore/il2cpp/libil2cpp/gc/BoehmGC.cpp)
also establish these mechanisms; neither guarantees every later ABI.

## Remaining coverage and product work

This version follows supported static roots and plain reference fields. It does
not read array elements, full string contents, embedded structs or instance
fields of uncatalogued constructed types. It can miss objects held only by
native engine roots or collections. A full node budget can prevent deeper
traversal, and partial snapshots must remain visibly partial to agents.

Next prepare schema for encountered constructed types outside suspension, spread
root coverage across bounded batches, and evaluate native runtime roots and
collections with a verified lifetime contract. Then expose selected candidates
through the shared engine/MCP session with refreshable reference paths. Names
and values must still pass behavioural tests before becoming useful editing
cards. Keep raw metadata out of the user's main interface.

Build with `cargo build --manifest-path research/runtime-objects/Cargo.toml
--target-dir target/runtime-objects --offline`. Run the frozen Python harnesses
inside leased Anode with new output directories. Raw data stays under
`test-results/runtime-objects-v1`; full game metadata is not committed.
