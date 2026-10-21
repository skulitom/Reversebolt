# Reference graph experiment v1

Question: can public runtime metadata and briefly stopped managed threads supply
coherent object/reference/scalar snapshots, with no game-specific names/layouts?
This does not test edit authority, useful semantics or persistent object handles.

The attempted real Unity 2022 fixture build stopped with no active Editor license.
Retain its original log; do not activate licenses or rerun it. Use the owned Rust
ABI stress fixture for correctness, then the installed Blue Prince runtime for
feasibility. The latter uses Unity 6000.0.58f2. Source contracts were checked in
the installed official Unity 2022.3.18f1 SDK; cross-version support is an empirical
test here, not a universal ABI guarantee.

Before native runs, freeze source/shared-source/binary hashes. First run three
loader calls on the owned fixture, six total snapshots. Its roots replace objects
every 100 ms, reuse a four-object allocation pool, and contain a two-object cycle
and an inherited field. Require consistent identity/hidden-scalar/inheritance
values and cyclic edges within every snapshot; identities must change across
captures. Require paired stop/start counts, no Rust helper heap operations while
paused, successful unload and host exit. Run the already-tested loader's wrong
identity check once and require rejection before instrumentation.

If these pass, launch Blue Prince through Anode into its normal startup/menu;
select no save and send no gameplay input. Run one unchanged helper call producing
two snapshots 250 ms apart. Require successful unload, a responsive game window,
at least one live object scalar in the main game image, and no helper Rust heap
operations while paused. Record all omissions, failures, coverage limits, and
actual pause durations. If a call times out, do not rerun/free/unload while its
thread is uncertain. Do not retry a failed game case under the same identifier.
Close only the owned test game and fixture, and release the Anode lease.

All class/field names, parent relationships, instance sizes, static storage and
field offsets come from runtime APIs. No pointer search, game profile, property
getter or arbitrary managed method is used. Warm metadata and allocate/touch
buffers before `il2cpp_stop_gc_world`; inside the critical section use only
preallocated structures, clock checks, bounded memory reads and the trivial
object-class accessor. Always call `il2cpp_start_gc_world` before serialization,
logging, allocation or freeing helper buffers. No GC handles are retained.

Budgets: 50,000 catalogued classes, 200,000 fields, 10,000 static reference roots,
20 seconds catalog preparation; per snapshot 1,024 nodes, 16,384 field reads,
depth 4 and 10 ms checked between reads. Stop/resume latency and an individual
runtime call are not forcibly cancellable, so this is not a hard real-time bound.
No claim that native threads/engine systems are atomically frozen. Arrays,
strings, embedded structs, unknown instantiated generic types, and objects not
reachable through supported static roots are coverage limits. Snapshot IDs and
addresses do not prove identity across snapshots, especially after address reuse.
The helper instruments and briefly pauses the process; it is not automatic app
selection behaviour. Full progress remains 31% pending useful validated controls.
