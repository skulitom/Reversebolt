# Provisional object cards beyond characters

The local observer previously rejected discrete non-character objects from the
automatic card path. It now proposes and crop-reviews `object` alongside `actor`
and `held_component`, preserving the reviewed category in the shared engine.
Names and brief descriptions appear in the existing movable cards; full evidence
stays under Inspect. No game names, profiles, offsets, source layouts or oracle
values are supplied to inference. The prompt names broad object classes only.

An object still needs a supported localization in the source frame. Other and
uncertain judgments do not publish cards. Published observations retain original
proposal geometry, source image hashes and frame slots. Every pass uses new
keys; repeated-looking items are not persistent identities. Text observations
remain provisional and cannot enter numeric matching or enable edits.

## Frozen native evaluation

Before capture, [the protocol](PROTOCOL.md), driver, scorer, candidate executables,
product source hashes and installed local-model digest were frozen in
[protocol-v1.json](protocol-v1.json). The original MIT Javascript Tetris source
from the earlier [independent-game baseline](../independent-games/README.md) was
hash-checked unchanged. A fresh owned browser in Anode received fixed native
80 ms pulses at 1500 ms intervals. Ordinary MCP attachment initiated all 16
captures and both model passes. There was no manual screenshot inspection or
adaptive game control. All 48 pulses reached the page in order as trusted
keydown/keyup pairs; that does not prove a gameplay consequence for every pulse.

A separate read-only game-state reader supplied scoring truth, never product
input. Only the falling piece and upcoming piece were predeclared targets.
The scorer requires stable timing-bracketed cells, geometry and color, no pending
redraw, and agreement with four saved-image cell-center pixels. Uncertain targets
are unscorable, not successes. Each model pass contributes its original source
frame, including empty or failed passes. This run had no failed pass or excluded
target.

| Source capture | Published object cards | Matched / scorable targets at IoU 0.3 and 0.5 | Miss |
|---|---:|---:|---|
| 2 | 3 | 2 / 2 | None |
| 16 | 4 | 1 / 2 | Upcoming piece |
| Total | 7 | 3 / 4 | One |

The missed upcoming-piece proposal covered the preview panel too broadly.
Other unmatched findings include the board and block piles: these were outside
the selected target list and are **unscored**, not established false positives.
No whole-scene precision is claimed. No Character or Held object was published.
The first three findings were present by capture 11 and remained associated with
capture 2; asynchronous publication did not relabel them as current.

## Saved character regression

The first annotated pair from each of `dev_q_demo`, `dev_c_idle` and `dev_c_move`
was processed with the same candidate and model. The user-owned visual-challenge
corpus was read only. At both IoU thresholds, proposals scored 4 true positives,
1 false positive and 0 misses. Crop review filtered the false positive while
retaining all four annotated entities using **original proposal boxes**, which
matches the product's geometry policy. Substituting refined boxes would instead
score 3 true positives, 1 false positive and 1 miss. That inferior variant is
retained in the evidence. Newly proposed Object items are unscored by these
narrow actor/held-component annotations. The stationary reference is a positive
character target, not a negative scene.

This is three exposed pairs and one exposed game, not broad recognition or
withheld-game acceptance. Same-model crop review is not independent validation.

## Limits and verification

Some raw explanations exceed the requested 25 words, and some infer game use or
interactivity despite the prompt. The main card shortens these descriptions;
Inspect retains the original claim. Structural validation and box overlap do not
validate semantics, identity, behavior, physical rules or control authority.
The browser window's process is sampled within the normal 16 MiB budget; this
evaluation neither finds nor verifies controlling memory. It performs no writes.

All 133 workspace tests, formatting, strict all-target Clippy and the release
build passed. Unit coverage requires supported Object localization and retains
its provisional category. The packaged preview also passes the existing MCP
smoke suite. Progress stays **25%** against the unchanged full-goal rubric.

`verify.py` audits the retained raw evidence without inference or game input and
compares it with [the compact receipt](evidence/v1.json). Local raw artifacts are
under `test-results/visual-objects-v1/`; they include all 16 images, oracle data,
input receipts, original model responses, scores, and frozen executables. The
local desktop-delivery receipt is `test-results/visual-objects-desktop-delivery.json`.

To rescore the retained native run:

```powershell
python research/visual-objects/score.py test-results/visual-objects-v1/tetris-1-attempt-01
python research/visual-objects/verify.py
```

`run.mjs` requires the pinned local runtimes, model, original sources and a
leased Anode child session. Every fresh execution must use a new output directory.
`protocol-v1.json` and its method files retain their pre-capture bytes. Do not
retune this frozen evaluation or replace failures with fresh attempts.
