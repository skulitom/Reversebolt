# Follow changes on independent games — 2026-10-21

The unchanged published build completed automatic discovery, fresh checks and
one guarded location trial on each original game. Both byte edits and byte
restorations succeeded. Neither trial produced the requested displayed value.
This establishes an investigation workflow on independent games, not a working
gameplay control. Progress remains **31%** and generalization acceptance 0/10.

## Method and retained cases

The [protocol](PROTOCOL.md), [hash manifest](protocol-v1.json), runner, input
driver, oracle and scorer were frozen before either game launch. Product commit
`67cdd3a` is published preview `0.1.0-20261021-114033`. Unchanged original sources
and MIT licenses are retained from the independent-games study: 2048 commit
`478b6ec346e3787f589e4af751378d06ded4cbbc` and Javascript Tetris commit
`e5c0c42f7dac0f3514a55eff656c6e22e95d68ed`.

Each case used a fresh owned browser profile in Anode, fixed native inputs,
16 MiB ordinary sampling and no optional image model. The declared CPU-delta
rule selected an accessible renderer. The lowest-ID eligible changing OCR
track triggered Follow changes, without an address, value, representation,
scale, game variable or source-derived layout supplied to the product. Four
broad observations selected small areas. The product itself scheduled sixteen
wholly fresh moments: twelve for fitting and four for future checks.

Both cases completed all **36 discovery captures** with no runner errors,
replacement runs or manual screenshot review. These are new feature evaluations;
the earlier failed 2048 setup remains in the separate value-search study.

| Measurement | Original 2048 | Javascript Tetris |
|---|---:|---:|
| Broad selection plan per moment | 25.54 MiB | 24.70 MiB |
| Complete bytes used for comparison | 23.36 MiB | 18.90 MiB |
| Selection readings | 464, 504, 512, 552 | 70, 70, 80, 90 |
| Selection comparison time | 228 ms | 144 ms |
| Retained 64-byte areas | 2 | 83 |
| Bytes per fresh snapshot | 128 | 5,312 |
| Areas omitted by the cap | 0 | 0 |
| Fresh frozen candidates | 2 | 51 |
| Candidates agreeing on all four later moments | 2 | 38 |

Times cover selection comparison, not the whole observation workflow. Complete
bytes exclude changed or unavailable chunks; neither plan covers all memory.
These single-run measurements are not recall or performance guarantees.

## Candidate ambiguity and trial results

2048's two I32 candidates learned a scale of 0.5 without a representation hint.
Both predicted the fresh readings **1136, 1136, 1264, 1264**. The source and fresh
series were spatially associated with the best counter. Score and best were
often equal, so passive agreement cannot distinguish their roles. The declared
first-candidate trial requested 1274 from the observed 1264 and changed one byte.
Readback confirmed that byte, and guarded restoration confirmed the original.
Product observations and the independent DOM oracle still read 1264. Both score
and best were stable at 1264 across the scored baseline, post-edit and
post-restoration RPC intervals.

Tetris froze 51 candidates. Thirteen agreed twice and then contradicted twice;
38 agreed on **180, 180, 190, 190**, with numerous different scale/offset mappings.
That large surviving set illustrates unresolved correlation, not 38 identified
score fields. The first eligible candidate was index 1, an I32 mapping with
scale 0.625. Its trial requested 200 from 190. One byte was written, read back,
and restored successfully. Product observations remained 190. Independently
scored post-edit and post-restoration intervals read score 190 and rows 0. The
baseline RPC interval is **unscorable because of delayed oracle coverage**;
it is not counted as an independently verified baseline.

No additional candidate was tried. During each trial the input driver was idle;
natural game activity continued. Failure to show the requested value in these
intervals does not prove that a location is a display copy or has no later
effect. A game may consume a value or redraw only after a relevant event. Four
future agreements here also contain only two distinct values and do not resolve
causal authority, hidden state, effect scope or restart stability. Restoring a
byte does not restore the whole game state.

## Independent observation scoring

The oracle reads displayed counter text and boxes through CDP DOM methods and
parses numbers outside the renderer. It reads no board, piece or numeric game
variables. Generic viewport/scroll geometry and trusted key-event listeners
remain instrumentation, with their associated objects and overhead. This does
not establish the absence of observer artifacts.

| Counter across 36 captures | Correct | Conflicting | Abstention | Unscorable |
|---|---:|---:|---:|---:|
| 2048 score | 21 | 2 | 13 | 0 |
| 2048 best | 36 | 0 | 0 | 0 |
| Tetris score | 35 | 0 | 0 | 1 |
| Tetris rows | 36 | 0 | 0 | 0 |

Rows remained zero. The unscorable Tetris image interval contained a changing
value. Missing score readings and conflicting OCR in 2048 remain failures to
provide one dependable reading; successful best readings do not erase them.
The fixed timing/geometry exclusions apply throughout. DOM text/box reads are
sequential and equal samples cannot rule out intervening changes. Each case's
125 native key presses and releases matched trusted browser events in order;
delivery does not prove every input changed gameplay.

## Reproduction, audit and next work

The committed runner verifies frozen method, source and binary hashes before
launch. Use Anode session 2 with its acquired identity and lease passed in the
environment, the installed Chromium/Node/Python paths from the manifest, and a
new output directory. Raw local evidence is in
`test-results/game-change-search-v1/`. [Compact results](results.json),
[execution receipts](execution.json) and the [post-launch audit](audit.json)
retain both cases. The audit is an integrity check, not a predeclared acceptance
test. It verifies disjoint evidence stages, fresh check provenance, bounded
selection/windows, input receipt, exact trial selection, write/restore readbacks,
**72 discovery PNG hashes and 10 trial PNG hashes**.

The owned headless oracle fixture, six-outcome scorer and trial-timing exclusion
checks passed before launch. All 22 recorded owned browser/helper/runner
processes exited. Anode's lease was released; its already-visible viewer was
left unchanged. The existing 184-test, strict-Clippy, release and 35-tool smoke
validation remains the unchanged product baseline, rather than a newly rerun
test suite. A fresh status/hash check confirms desktop PID 62456, the same
published build, 29 history entries and the local model setting preserved.

The next investigation needs controlled observations across relevant game
updates, with competing predictions and guarded restoration, to separate
co-moving fields from effective controls. It must retain nonresponses and
uncertainty, without adding game-specific discovery rules. Hidden semantics,
reliable high-level edits and independent acceptance remain unfinished. The
application stays a card interface; this research document is repository-only.
