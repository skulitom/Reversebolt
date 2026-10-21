# Original-game location trials, 2026-10-21

Four fixed native Tetris cases produced two abstentions, one application refusal,
and one confirmed byte write with **no observed score response**. That write's
restoration was refused because the bytes had changed again. No gameplay control
or complete recovery was established. Progress remains **27%** against the same
full-vision denominator.

## Method and provenance

The frozen [V1 method](PROTOCOL.md) and [V2 method](PROTOCOL-v2.md) use the unchanged
Javascript Tetris revision `e5c0c42f7dac0f3514a55eff656c6e22e95d68ed` already retained
in the independent-game evaluation. Each case starts a fresh browser profile in
Anode, blocks external requests and uses the same product binaries from preview
`0.1.0-20261021-081604`. Source, method and binary hashes are in the two protocol
JSON files and checked by the post-run audit. These are exposed development games,
not withheld generalization acceptance.

The evaluator chooses an accessible renderer by the fixed largest-positive-CPU-
delta rule. The product receives its validated identity, a 16 MiB memory budget
and no local vision model. Connection starts sixteen observations; the product
chooses a numeric track and freezes predictions at capture twelve. The evaluator
never passes source layouts, oracle values or addresses into discovery. The
independent oracle can itself allocate additional copies in the renderer.

Case one always selects candidate zero; case two selects the final candidate.
There is no replacement after refusal. V1 requests the latest observed number
plus three; V2 uses plus ten. Eligible proposals are applied once, checked after
750 ms and offered one guarded restoration. No force restore or thread suspension
is used. No screenshots were manually reviewed and no game was manually played.

V1's continuous input left both games ended before the fresh checking period.
Both failures remain in the evidence. V2 was frozen separately before either new
case: fixed bursts between captures, no input after captures twelve and thirteen,
then a fixed burst after fourteen. Game gravity continues throughout. The first
startup failed before any game/browser/engine/input/oracle started because the
output parent directory was absent. [The record](startup-failure.json) preserves
that failure; creating the parent allowed the unchanged first method to start.
No observed case was rerun.

## Outcomes

| Case | Candidates | Fresh selected readings | Result |
|---|---:|---|---|
| V1 / 1 | 3 | 110, 110, 110, 110 | Insufficient contrast; preparation refused |
| V1 / 2 | 5 | 100, 100, 100, 100 | Insufficient contrast; preparation refused |
| V2 / 1 | 6 | 90, 90, 120, 130 | Prepared 130 to 140; application refused before native entry |
| V2 / 2 | 5 | 100, 100, 150, 150 | Prepared 150 to 160; one-byte write confirmed; restoration refused |

V2 / 1 retained twenty agreeing comparisons, two contradictions and two uncertain
timing samples across its six candidates. Its selected I32 interpretation used
slope 10 and intercept -30. The current bytes changed after preparation, so the
executor did not enter the write backend.

V2 / 2 selected an I32 interpretation with slope 0.5 and intercept 0. The native
receipt confirms one byte changed from 44 to 64 and read back as 64. Both the
automatic visual check and the later explicit check still read 150, not 160.
During the sampled operation windows the game's actual score, display score and
DOM score all remained 150; `playing` was false. Restoration refused before
native entry because the interpreted bytes had changed again. The owned game
was then closed. This is evidence of no response in the observed score channels
and time window, not proof of no delayed or hidden effect, a permanently unused
copy, an active-gameplay trial, or recovery of the full game state.

The original runner concatenated phase names into `written; restoration written`.
That label is preserved verbatim in the receipts, but **does not mean restoration
succeeded**. `written` described the original successful write; the reason states
that restoration was refused. The old binary omitted a separate refusal receipt
for this preflight path. The audit classifies the outcome from those facts and
does not revise the frozen runner or raw records.

Score OCR was correct on 14/16, 16/16, 14/16 and 16/16 captures respectively;
the remaining four captures were unscorable by the frozen timing/geometry rules,
not silently scored as correct. Rows were correct on all sixteen captures in each
case. Delivered trusted keydowns matched all 127, 128, 113 and 113 scheduled pulses
in order. V2 retained all thirteen burst completion acknowledgements and fifteen
accepted pause/resume pairs. All four game jobs ended with exit code zero; the
forty recorded owned process IDs were absent at the final cleanup check.

## Reporting fix and delivery

The product now durably records `attempted: false` refusal receipts for both
whole-value and final-patch preflight conflicts. A refused restore preserves the
original successful write receipt. Its card says **Restore wasn't applied**;
Inspect holds the explanation. No new main-view report or automatic write is
introduced. The phase and guard semantics remain unchanged.

Tests cover full-value apply/restore conflicts, a late conflict between full-value
and patch reads, persistence after reopening, and the absence of extra backend
calls. All **151 workspace tests**, strict Clippy, release build and packaged
31-tool smoke checks pass. Package evidence is locally retained at
`test-results/smoke-4e5dc079/report.json`; logs use `test-results/game-probe-*.log`.

Preview `0.1.0-20261021-085002` is running on the user's desktop, with all eight
history entries and `qwen3.8:latest` preserved. Its executable SHA-256 is
`1faf0c80f38fc89dbdcd316d56c160b29957172f081c4edca42bf9f3d3e0fce6`.
The desktop delivery receipt is `test-results/game-probe-desktop-delivery.json`.
The game evidence above used the earlier binary; it does **not** exercise this
new refusal-receipt field or establish native UI validation of the new wording.

## Reproduction

`python research/game-location-probes/verify.py` audits retained raw runs without
starting any applications. This audit was written after the runs, and is labeled
accordingly. It verifies method/game/product hashes, automatic selection and
frozen mappings, timing witnesses for counted comparisons, fixed candidate/value
selection, input order, operation counts, receipt outcomes and oracle summaries.
It does not reconstruct unavailable raw training memory or infer causality from
correlation. Compact receipts are [V1](evidence/v1.json) and [V2](evidence/v2.json).
Full images, states and oracle traces remain in the ignored
`test-results/game-location-probes-v1/` and `test-results/game-location-probes-v2/`.

Anode job IDs: V1 `j_26675e9054984b43872554d7af2079ef` and
`j_b683e1d2262a4621864b96a5be28db4f`; V2 `j_706cc1606cc942d5be4f84eb712e6ddc`
and `j_515aa6c8b67d4158ae9f1004bf71804d`. The lease was released after completion.
