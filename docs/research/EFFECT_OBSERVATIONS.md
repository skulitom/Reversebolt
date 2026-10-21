# Broader observations around an explicit edit

2026-10-21. This extends the live trial workflow. It measures observed
differences; it does not establish causation, identify a gameplay mechanism
or certify that all effects have been reversed.

The [compact evidence record](../../research/effect-observations/evidence/native-v1.json)
retains all three outcomes, private audits, source/receipt/image checksums,
published binary hashes and desktop delivery evidence.

## Behavior

Review edit captures a fresh before-image. Its selected OCR number must still
match the latest ordinary observation and proposed models, otherwise the plan
becomes unavailable. Up to 32 active number regions are frozen for the trial,
with the selected region first and an explicit omitted count. New captures
cannot silently rename or remap the watched regions during this comparison.

After applying or restoring a trial, the engine rereads those regions.
Each other value is changed, unchanged within the existing 0.55 OCR tolerance,
or unreadable. Missing baseline/current readings and ambiguous region association
stay unreadable. Untracked numeric regions are counted. The main card adds a
short notice when another observed value changes; other cards also show their
fresh readings. Details remain inside Inspect.

Every observation retains its stage, image hash, capture-operation timing,
warnings and selected/other readings. There are at most eight post-observations
plus one baseline. Existing 10 MiB per-image and 128 MiB trial-storage limits
remain, including reservations for late native capture workers.

The new read-only MCP tool, `reversebolt_read_change_evidence`, returns the
before-image and a chosen post-edit/restoration image as native PNG blocks,
followed by their metadata and comparison. It verifies hashes and confines
paths to the engine's operation-evidence directory. The pair is limited to
16 MiB and remains readable after restart without granting target access.
Old trials without a baseline report that limitation. Images include
nonnumeric appearance, but the automatic comparison currently measures numbers.

## Native automated matrix

The fixture is a separate WinForms process with a randomly placed float.
Discovery receives screen images and ordinary memory snapshots, with no
field address, layout or effect-mode parameter. The driver supplies twelve
varying screen values and four fresh hold/change observations, then requests
50 → 53 and restores 50 through MCP. Each mode has its own process and session.
Private fixture truth is written only at process close for auditing.

| Fixture mechanism | Other value after edit | After restoring selected bytes | Private audit |
|---|---|---|---|
| Coupled visible effect | Resource 25 → 22 | Resource remains changed at 19 | Two writes; selected quantity 50 |
| Independent time drift | Resource 120 → 127 | Resource 138 versus baseline 120 | Counter advances with time, independently of writes |
| Hidden effect | Resource stays 25 | Resource still 25 | Two hidden state changes remain |

All three runs applied/restored and delivered two image blocks with matching
hashes. The coupled case shows an observed residual effect that the previous
single-number check missed. It also shows that restoration itself can have a
further consequence. The drift case intentionally receives the same descriptive
changed status; the engine does not claim that the write caused it. The hidden
case is an explicit observation failure: unchanged visible values do not reveal
the private mutations. No successful recovery or closed-effect claim is made.

The coupled fixture also changes a colored rectangle after an external edit.
That appearance is retained in the paired images; this experiment does not
claim automatic recognition or a scored interpretation of that visual change.

Raw matrix receipts, images and private audit:
`test-results/effect-observations-67b1a8f8/report.json` and its case directories.
The matrix ran in Anode without gameplay control or manual screenshot review.
Run it inside that session with:

```powershell
python scripts/effect-observation-smoke.py
```

The source is a known, purpose-built fixture and all mechanisms were visible
during development. Random addresses are not an unseen-game evaluation.
These results earn no additional generalization or causal-validation credit.

The published build was also exercised with its bundled neural OCR models.
That separate coupled run passed the same edit/restore and image-delivery checks.
Native UI Automation confirmed Quantity 50, Resource 19, the short other-value
notice, and Inspect's 25 → 19 comparison alongside its causal limitation.
The test used no manual screenshot review. Evidence is under
`test-results/effect-observations-67b1a8f8/desktop/`.

Preview `0.1.0-20261021-031846` is running on the user's desktop with its prior
eight history entries and local model selection retained. The published 29-tool
smoke passes at `test-results/smoke-8bcba2a3/report.json`. Owned fixture/GUI
windows were closed and the Anode lease released. Binary hashes and delivery
proof are in the desktop subdirectory's `desktop-delivery.json`.

## Automated checks and remaining work

All 105 workspace tests pass; strict Clippy and formatting pass. Extended
live-session tests cover a stale fresh-baseline reading preventing any write,
changes to another channel, missing observations, geometry changes, capture
chronology, restart-readable evidence, tampered images and path escape refusal.
A comparator test covers duplicate IDs, nonfinite values, missing baseline
values, observed change, tolerance and omitted scope.

The next requirement is independently grounded action/context observations
and controlled contrasting trials. Normal gameplay must be distinguished from
intervention effects; appearance, motion and hidden consequences need their own
measurements. This work provides evidence for that investigation rather than
promoting every successful numeric edit into a verified control.
