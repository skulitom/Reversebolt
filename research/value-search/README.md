# Value-guided broad memory search — 2026-10-21

Reversebolt's **Find this value** card action and `reversebolt_search_number`
search beyond the current memory sample using the card's latest OCR number.
The product reads up to 512 MiB, retains small possible areas, then automatically
collects sixteen wholly new observations. A seed selects where to look; it does
not count as a fit, fresh check, semantic interpretation or permission to edit.

This addresses the cost of repeatedly storing large complete snapshots just to
find a relatively small set of possible locations. It complements **Keep looking**,
which retains broader representation discovery without literal-value seeding.
The new operation is original Rust code; no Squalr code was imported.
[Squalr's primary documentation](https://github.com/Squalr/Squalr) describes
separating snapshot collection from scanning and narrowing later scans through
result filters. That architecture informed the separation here between broad
selection and later targeted temporal evidence; it does not supply semantic or
causal identification.

## Method and results

The [frozen method](PROTOCOL.md) commits a 128 MiB arena in an owned native
fixture, placing a value at a random aligned location in its upper half.
The ordinary sampling budget is one MiB. Product inputs contain no known
addresses, offsets, layouts or type hints. The evaluator changes the fixture's
own number and reads a separate oracle only for scoring. Both automatic passes
use fixed value sequences; no manual screenshot review or game control occurs.

| Case | Outcome | Search | Fresh evidence |
|---|---|---|---|
| Float32 direct field | Failed before search: initial frame 12 was blank | Not reached | Not reached |
| Float64 direct field | Passed; distant field retained outside the old plan | 215.707 MiB in 443 ms; 2,321 windows | Exact F64 fit on 12 new moments, then 54,54,50,50 all agreed |
| Float32 half of visible value | Expected limitation reproduced | 216.188 MiB in 475 ms; 2,019 windows | Actual encoded field omitted; no final numeric matches/checks |

The successful search retained only 148,544 bytes per new snapshot (about
145 KiB), and the negative case 129,216 bytes. Neither search exceeded the
window cap or omitted matching windows by sampling. Each excluded 94,208 bytes
after unsuccessful/changed reads. These timings describe two local scans, not
a general performance guarantee or recall estimate.

The float32 failure is retained, not replaced by a new run. Its capture receipt
records a detailed first frame followed by a nearly uniform selected frame;
the fixed-window sampler kept that later image and emitted its warning. One
missing initial reading violated this protocol's strict all-sixteen condition,
so the wider-search operation was never called. Direct float32 seeding passes
the Rust contract test, but this native float32 case does not verify it.

The negative result is not successful encoded-value recovery. Literal copies
elsewhere were retained, while the actual field storing half the displayed
number was absent. None became a frozen finding after the fresh sequence.
This illustrates why the literal search cannot replace general temporal
discovery or be treated as a controlling-state detector.

## Semantics and bounds

Only naturally aligned little-endian i32/f32/f64 literal encodings nominate
64-byte windows. All retained windows are reread in the new batch; the selection
bytes are discarded. Fitting still uses separate training and held-out halves,
then immutable predictions are challenged by later captures. The original seed
image and old sample IDs cannot become those fresh samples. Previous findings
and sparse evidence remain archived, and stale batch/frame requests are refused.

Each scan has a 512 MiB read cap and a ten-second deadline checked between native
calls. The latter cannot preempt a blocked OS call. At most 4096 windows are
retained using a stable address-hash ordering when the value is common; omitted
window counts remain explicit. The investigation allows eight such searches,
shares its existing 4 GiB requested-read limit, and charges selection files to
the archive budget. A process exit, failed persistence or absence of locations
does not silently replace the current findings. Completed reads remain charged
even when saving fails. Applied/unresolved trials block the transition.

The scan can miss a changed value, nonliteral encoding, unaligned storage,
unvisited range or omitted window. Address and mapping checks do not establish
allocation lifetime, atomic capture or gameplay authority. No writes occurred
in these cases, and no independent-game acceptance credit is claimed.

## Verification and delivery

All **176 workspace tests**, strict Clippy, release build and packaged smoke
pass. Six new contracts cover late-memory discovery, invalid/partial/dead reads,
bounded common-value sampling, direct types and unsupported encodings, new
trajectories, stale scopes/budgets, and persistence-failure accounting. The
controller check covers automatic scheduling for both human and agent callers.

The packaged smoke exposes 33 tools and passes shared-session, authentication,
history and existing practice edit/restore checks. Preview
`0.1.0-20261021-105104` is running on desktop session 1, PID 86012, with eight
history entries and `qwen3.8:latest` preserved. All nine owned native-test
processes exited and the Anode lease was released. Native action routing was
tested through MCP; the new card button was not separately clicked in a native
UI test. Existing card layout stays concise; search details are in Inspect.

Discovery credit moves from 6/20 to 7/20 for implemented broad candidate
selection followed by verified fresh native evidence. Weighted progress is
**30%**; all other credits and the full objective remain unchanged. The next
evaluation should apply the unchanged search to independent games and retain
representation/capture failures rather than supplying game-specific answers.

- [Frozen method, source and binary hashes](protocol-v1.json)
- [Post-run audit and retained outcomes](evidence.json)
- [Execution and delivery receipts](execution.json)
- Raw artifacts: `test-results/value-search-v1/case-{0,1,2}/`
- Packaged smoke: `test-results/smoke-caf4a168/report.json`

Reproduction uses Python inside an acquired Anode session: `run.py CASE OUTPUT`
with a new output directory and CASE 0, 1 or 2. Set the verified distinct
`REVERSEBOLT_TEST_SESSION` and `REVERSEBOLT_PARENT_SESSION` environment values.
The runner requires the frozen method and executable hashes. `verify.py` is a
post-run auditor over the retained paths; it never rewrites the frozen criteria
or reclassifies a runner failure as a pass.
