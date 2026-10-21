# Value-guided search on independent games — 2026-10-21

The unchanged desktop build completed the Tetris search workflow, but its newly
found candidate failed later predictions. Original 2048 stopped during harness
setup, before discovery. Neither case demonstrates gameplay control. Progress
remains **30%**, with independent-game acceptance still 0/10.

## Frozen method and retained outcomes

The [protocol](PROTOCOL.md) and [hash manifest](protocol-v1.json) were fixed before
the first game launch. The product is commit `e9e097c`, published preview
`0.1.0-20261021-105104`. Original upstream game files, hashes, and licenses remain
unchanged from the earlier independent-games evaluation. The cases run in order
with fresh owned browser profiles, fixed native inputs, and no manual screenshots
or gameplay. No memory writes, oracle-fed observations, game layouts, offsets,
representation hints, or source-derived discovery rules enter the product.

| Case | Retained outcome |
|---|---|
| Original 2048 | Harness setup failed: the Anode CLI helper lacked lease credentials. Zero observations and no product search. No replacement run. |
| Javascript Tetris | Initial 16 automatic moments, one wider search, then 16 fresh moments completed. No engine/runner errors; newly frozen candidate contradicted twice. |

The Tetris launch explicitly supplied the already acquired Anode agent/lease
credentials to its CLI helper. That corrects the launch configuration, not the
frozen runner, inputs, product, or scoring rules. The failed 2048 case remains in
the denominator and all receipts. It provides no discovery evidence.

## What Tetris established

Reversebolt selected a changing OCR series using the declared lowest-ID rule;
the evaluator did not tell it which series was the score. At moment 16 its
latest reading was 60. The wider search read **28,815,360 bytes (27.48 MiB)** beyond
the original selected memory in **24 ms**, retaining **127 64-byte windows**.
That is **8,128 bytes per fresh snapshot**, with no window-cap omissions and no
failed/changed reads during selection. This is one local measurement, not a
performance or recall guarantee.

The first pass had frozen ten numeric candidates. Nine contradicted at least one
later reading. The other had one timing-uncertain result and three agreements;
it did not establish gameplay authority. The new batch started with empty
observations/checks and fitted twelve wholly new moments. It froze one exact
I32 mapping, with six training and five held-out inliers. Its four later outcomes
were **timing uncertain, agrees, contradicts, contradicts**: predicted values
143, 150, 152, 152 against observed values 140, 150, 160, 160. The fresh checks
therefore exposed a candidate that did not keep following the displayed value.
No location is identified here as a controlling field or a particular kind of
copy. Different memory selection and game trajectories prevent a direct
before/after recall comparison.

Automated spatial/timing scoring counted **28 correct score readings**, with
four score intervals excluded because the value changed during the image
interval; rows were correct in all 32 captures, but remained zero. Each stage
had fourteen correct and two unscorable score readings. There were no scored
wrong, missing, or conflicting counter readings in this run. The **113 native
key presses and releases** exactly matched trusted browser events in order.
Delivery confirms input receipt, not that every key had a gameplay effect.

## Oracle and limits

The evaluator reads displayed text and element boxes using
[Chrome DevTools Protocol DOM methods](https://chromedevtools.github.io/devtools-protocol/tot/DOM/).
It parses counter numbers in the separate Node evaluator, avoiding the previous
harness's repeated numeric game-variable reads inside the renderer.
`Runtime.evaluate` reads only viewport/scroll geometry; event listeners record
key delivery. This still adds instrumentation and objects and does not prove
the absence of observer artifacts. Actual internal score, board, and piece
state were not measured. Therefore no claim about display versus authoritative
state, hidden semantics, or nonnumeric discovery follows.

Oracle samples bracket image requests; delayed sampling, changed values/geometry,
scrolling, and unsupported browser/client geometry exclude a score. Text/box
reads are sequential, not atomic, and equal samples cannot rule out ABA changes.
This is an exposed development game, not withheld acceptance. Literal selection
can still miss encoded values, changing values, relocated state, and omitted or
unvisited areas. The concrete remaining problem is retaining dependable evidence
as a game evolves, not merely producing more matching addresses.

## Reproduction and audit

The runner, input driver, oracle and scorer are committed beside the frozen
manifest. Run inside an acquired Anode session, with `ANODE_AGENT_ID` and
`ANODE_LEASE_TOKEN` passed through the job environment; do not put lease tokens
in arguments or saved evidence. The raw local output is under
`test-results/game-value-search-v1/`. [Compact results](results.json),
[post-run integrity audit](audit.json), and [execution receipts](execution.json)
retain both cases. The audit was written after launch and checks integrity;
it is not a predeclared acceptance test or a discovery mechanism.

The oracle's owned headless fixture and the six-outcome synthetic scorer check
pass. A malformed timestamp in the scorer's initial synthetic fixture was
corrected before the final protocol freeze and before any game launch. The
audit verifies frozen hashes, unchanged game sources/binaries, native input
receipt, separate old/new frame identities, frozen-check basis/future frames,
selection provenance, and bounded read/window accounting. All 17 recorded game
and helper processes exited; Anode's lease was released and its viewer stayed
hidden. No screenshots were manually inspected.

Production source did not change, so the existing 176-test/strict-Clippy/package
validation from the [value-search implementation](../value-search/README.md)
remains the build baseline; those checks were not rerun for these research files.
A fresh desktop status check confirmed PID 86012 in session 1 running the same
published binary, with eight history entries and the local observer setting
preserved. The main application remains named draggable cards with short
descriptions/actions; this research report is a repository artifact.
