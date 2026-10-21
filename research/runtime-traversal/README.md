# Runtime traversal beyond static roots

The reusable Rust reader now reaches children, arrays and strings in Blue Prince
without screenshots, gameplay input, game-specific names, offsets or layouts.
This is a discovery component under `crates/reversebolt-runtime`; the desktop
and MCP dispatchers do not call it yet. It does not grant editing authority.
Overall progress remains **31%**.

## Measured result

One frozen request inspected Blue Prince (Unity 6000.0.58f2), PID 14300, creation
`01dd61641a335484`, Anode session 2. It read all **4,267 supported static root
slots** in 67 final batches. Ninety-three additional snapshots prepared newly
encountered types between pauses, adding 944 class descriptions. No continuation
was needed for this game run; continuation was exercised by the owned fixture.

Across final batches there were **7,747 non-root node occurrences**, 27,023 scalar
read occurrences, **26 deeper scalar reads declared in the main game image**,
1,841 successful string reads and 895 arrays with elements read. Maximum depth
was four. These are occurrences: different batches may revisit the same objects
and fields. They are not counts of distinct gameplay controls or durable objects.

All 160 recorded managed stop/read/resume sections had zero helper Rust heap
operations. Total stop-to-resume time was 119.893 ms, median 0.4175 ms, maximum
5.216 ms. Three exceeded 5 ms including stop/resume overhead. The budget is checked
between operations; runtime and OS calls do not have a hard real-time guarantee.
Initial metadata preparation took 926 ms. Inspection before final JSON writing
took 4,075 ms; the complete load/inspect/write/unload cycle took 15,252 ms.
This is bounded research instrumentation, not an established continuous sampler.

The helper unloaded and the game's window remained responsive. The owned game
was then closed, all owned test processes exited, and the Anode lease was
released. Steam stayed in session 1. No screenshots, save selection, gameplay
inputs or gameplay-field writes were used. The existing desktop preview
`0.1.0-20261021-122703` remains running with its 29 history entries and model setting.

## What changed and what remains partial

The previous experiment filled its node capacity with root objects and discovered
no new children. This revision limits each root batch to 64 and resolves newly
encountered types outside the managed pause, then takes a fresh snapshot. Public
runtime APIs provide array shapes and string access. The adapter uses no private
game object layout. Reference arrays and primitive scalar arrays are supported;
other arrays are explicitly partial. Only runtime class metadata pointers survive
between pauses; object addresses and IDs remain local to one snapshot.

Root-scan completion does **not** mean graph completion. Three final snapshots
reached the node cap and one reached the time/read cap. Fifty node occurrences
still had uncatalogued types. There were 342 truncated array occurrences,
including 223 unsupported array occurrences, and six truncated strings. The
catalogue retained 802 open-generic omissions and 1,090 uncatalogued-parent
notices; these notices can repeat during rebuilding and are not unique classes.
All attempted root/field reads succeeded, but skipped/unprocessed data is not a
successful read. Native-only roots, value-struct/enum arrays, objects deeper than
four links and full collection traversal remain outside this result.

These data do not identify a player, prove a gameplay role, distinguish a display
copy from controlling state, or establish a refreshable object identity. In
particular, names and current values alone must not become editable cards.

## Validation and provenance

- [Protocol fixed before native runs](PROTOCOL.md), [frozen file manifest](frozen-v1.json),
  [aggregate results and artifact hashes](results.json), [post-run audit](audit_postrun.py).
- Source committed before execution as `13254ee4ed0251f2d5b0f5c2ada0c106ec9f2682`.
  Fourteen source files match the freeze and that revision; root Cargo files
  permit only Git's recorded line-ending conversion. Three of four original
  binaries remain hash-verifiable. Subsequent full-workspace tests rebuilt the
  shared-target helper DLL; its original bytes are no longer retained there.
  Every native loader validation receipt and the game harness record the original
  frozen helper hash, `c4ab56693b0491255e23c1133ea386eaea28c9092b403b83f7efdf0e57adeaeb`.
  The audit reports this retention gap explicitly. Future experiments must copy
  frozen binaries outside Cargo's shared output directory before running tests.
- Owned ABI fixture: 130 root slots, hidden child objects reachable through a
  reference array, four types absent from initial enumeration, inheritance,
  cycles, primitive arrays and truncated text/collections. It verifies coherent
  values, continuation at root 64, malformed-input rejection and mismatched
  catalogue rejection. All nine pauses were balanced. Metadata API calls while
  stopped abort the fixture; no such failure occurred. This simulation is not
  independent IL2CPP/runtime acceptance.
- New crate and fixture builds, strict Clippy and formatting passed before the
  source freeze. Full-workspace tests passed afterward, including the new
  early-error resume-guard test. The result-consistency audit passed after the
  native runs; its binary-retention limitation is separate from that result.
- Raw local evidence is in `test-results/runtime-traversal-v1`. Game metadata
  and string contents remain local; this repository retains aggregates/hashes.

For reproduction, use the recorded source revision and build the runtime crate
and the standalone fixture. The existing `runtime-objects` host supplies the
fixture process. `mock_case.py` takes a new output directory; `game_case.py`
takes an exact PID, creation identity, session and new output directory. Native
cases run inside Anode against owned processes. Do not retry an uncertain helper
thread. These scripts never accept an old output directory as a new run.

## Next useful milestone

Expose typed runtime candidates through the shared engine, establish fresh
reference paths, and test a hidden control's predicted effect. Compare that path
against memory-change discovery using unchanged adapters on independent games.
Measure validated controls, incorrect labels, time to a useful control and runtime
overhead. More readable fields alone should not earn generalization credit.
