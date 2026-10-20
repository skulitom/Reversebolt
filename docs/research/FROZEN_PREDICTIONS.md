# Frozen predictions and automatic reports — 2026-10-20

Four correlations from a fresh Blue Prince sequence failed when their unchanged
predictions were checked against four later observations. This establishes
useful rejection behavior, not discovery of a gameplay control. No game writes,
profiles, signatures or source-derived layouts were used.

## Prospective protocol

The prediction checker and release binaries were frozen before collecting the
new sequence. Twelve initial captures followed normal movement between two rooms
in the disposable test profile. Inspection of the saved, hash-checked images
gave the visible counter values 14 through 3. Initial automatic OCR missed 14
and 13. It produced no hypotheses in the initial automatic comparison.

The image inspections were submitted as a separate, explicitly declared agent
series. They did not replace automatic OCR. Comparison of that series against
16 MiB of sampled memory yielded four non-exact I32 mappings. The new real MCP
tool froze all four after capture 12. Later captures were 3, 3, 1, 1, with holds
and normal room crossings. The new images were inspected before submitting
their interpretations. All four mappings contradicted all four fresh values.

| Mapping | Predictions at captures 13, 14, 15, 16 | Observations |
|---|---|---|
| A | -5.00, -5.00, -7.37, -7.37 | 3, 3, 1, 1 |
| B | 1.80, 1.80, 0.00, 0.00 | 3, 3, 1, 1 |
| C | -117.40, -121.24, -186.88, -186.88 | 3, 3, 1, 1 |
| D | 13.00, 13.00, 11.00, 11.00 | 3, 3, 1, 1 |

Tolerance was fixed at 0.55, matching the existing numeric discovery tolerance.
The original coefficients were unchanged. The local audit checks every image
hash, prediction reconstructed from saved bytes, submitted value and outcome.
The game's memory may change between image and byte acquisition. The initial
fitting interval's full memory is transient; this audit checks frozen mappings
and fresh witnesses, not a reproduction of fitting from all original bytes.

Automatic OCR on the inspected counter was **12 correct, 2 missing and 2 wrong**
across 16 images. The two wrong readings were 7 where the stylized numeral was
1. Footprint false positives and duplicate Rank tracks also remain. These are
selected-field results, not whole-image accuracy. The earlier successful OCR
sequence does not erase this failure; the reader was not retuned on these frames.

## Checker contract and limits

The shared engine exposes one check per memory batch. Original mappings and
scope are fixed before new captures. Late external observations can join their
own frames; they cannot refit predictions. Missing values remain gaps. Image
geometry, observed memory mapping changes, target exit and persistence failures
invalidate the applicable scope. Four agreements with an adjacent hold and a
distinct change allow only “consistent in observed scope.” Two display copies
can both qualify. There is no causal, representation, allocation-lifetime or
editing-authority claim, and no statistical confidence estimate.

Controlled tests include identical copies plus a misleading activity counter,
late agent submissions, missing reads, resizing, mapping replacement, process
exit, stale scope, duplicate freezing and failed evidence storage. They preserve
coefficients and reject refitting during a check. They are synthetic contract
evidence, separately identified from the real-game sequence.

## Automatic discovery and control cards

After the game evaluation, the user requested automatic reports on connection
and less manual game/screenshot testing. Connection now schedules one durable
16-moment pass through the existing controller. The first capture begins without
a delay; comparisons refresh from capture 12. The cards fill as observations
arrive. Pause and Stop remain available. Manual capture tools are secondary;
Keep looking archives this batch and starts another bounded pass. Agents
can opt out with `auto_observe:false` for precisely timed experiments.

The production automatic flow was checked with the owned WinForms window and a
script changing its displayed quantity. A single real MCP attachment collected
16/16 expected readings and performed comparisons without any separate capture,
start-job or comparison command. Attachment returned in 62 ms; the complete pass
took 41.14 seconds on this machine. It compared 3.9 MiB and found zero hypotheses.
The pass stopped at its limit. Pause at one capture, resume, cancel at two, and
manual opt-out all passed. This verifies orchestration and plumbing, not game
understanding. The desktop picker also started the same human-owned job without
an extra capture action, confirmed through UI Automation and the shared API.

The user subsequently clarified the intended interface: direct control cards,
not a report. The primary view now shows named cards with short descriptions,
current values and an Inspect action. Images, raw investigation and activity
are collapsed. Live values are marked Learning or Uncertain; none is presented
as editable. Verified practice values support dragging or typing a number and
applying it through the existing checked engine command. Functional names and
descriptions for unfamiliar game state still require semantic discovery.

The final native UI check confirmed the Quantity card, Learning state and Inspect
explanation. An automated interaction changed an owned practice value to 123
through its numeric widget and Apply, verified the engine state and restored 34.
Evidence is in `test-results/automatic-report-v1/cards-final/ui-check.json`.
Preview `0.1.0-20261020-233851` was delivered to the normal desktop with all seven
prior history entries retained, matching executable and model hashes, and a
passing published-build smoke (`test-results/smoke-3e6cd17d/report.json`).
Delivery proof is `test-results/automatic-report-v1/desktop-delivery.json`.

The UI changes were tested after the frozen game experiment, so the final
delivery has a different binary hash from that experiment. Discovery and
prediction algorithms were not tuned to the observed failures.

## Reproduction and retained artifacts

- `research/predictions/evidence/blue-prince-v1.json`: compact frozen hashes,
  observations, frame checksums, predictions and audit results.
- `research/predictions/summarize.py`: offline audit of saved evidence; no capture
  or fitting. Pass a run directory, inspected OCR track ID and new output path.
- `scripts/prediction-evidence.py`: retrieve hash-checked images and submit
  separately authored interpretations. It generates no values or labels.
- `scripts/automatic-report-smoke.py`: automatic native report and interruption
  test with `scripts/observation-fixture.ps1` and an isolated engine.
- Local raw game evidence: `test-results/predictions-v1/`, including original
  freeze, actual MCP responses, images and `prediction-check.json`.
- Local automatic report evidence:
  `test-results/automatic-report-v1/engine/automatic-report.json`,
  `automatic-completed.json` and `desktop-auto-start.json`.

Game images and full process snapshots are not committed. Blue Prince was left
paused, and its read-only test engine was detached and closed. Prefer automated
contract checks, recorded-input replay and owned windows for subsequent work.
Use live game interaction or image inspection only to resolve a named remaining
uncertainty; avoid manually browsing large screenshot batches.
