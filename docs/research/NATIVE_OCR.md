# Native OCR integration, 2026-10-20

The image-only PP-OCRv6 experiment is now implemented in Rust and integrated
with the desktop and shared MCP engine. The same general models, extraction,
word geometry and temporal regions handle all evaluated applications. No game
names, expected values, hand-marked regions, memory offsets or source layouts
are supplied to the observer. Manual annotations are evaluation-only.

## Evidence and separation

| Evaluation | Correct selected fields | Wrong / uncertain / missing |
|---|---:|---:|
| Blue Prince development replay | 12 / 12 | 0 / 0 / 0 |
| Half-Life 2 development replay | 36 / 36 | 0 / 0 / 0 |
| Ordinary-window development replay | 12 / 12 | 0 / 0 / 0 |
| Fresh live Blue Prince, counter 25 through 14 | 12 / 12 | 0 / 0 / 0 |
| Fresh live ordinary window, nonmonotonic values | 12 / 12 | 0 / 0 / 0 |

These are selected-field scores, **not whole-image precision**. The fresh Blue
Prince run produced two spurious numeric tracks around the footprint, with
eight and seven accepted readings respectively, plus two duplicate Rank
readings. They remain provisional. Native recovery improved recall but also
propagated icon false readings until its bounded expiry. Full semantic filtering,
calibrated confidence and temporal behavior understanding remain unresolved.

The native implementation and release executable were frozen at
`2026-10-20T21:33:34.4651151Z`, before the fresh Blue Prince captures. Source and
binary hashes still match that freeze. The development data informed extraction
and association fixes; it is not withheld evaluation. The fresh run tests the
frozen pipeline on more gameplay in an already studied game, not an unseen-game
generalization gate. No game memory writes were made.

The initial fused development replay read the digits correctly but lost the
counter track on four frames because an adjacent icon competed through proximity.
Association now reserves direct overlaps before proximity fallback and abstains
when readings compete, independent of iteration order. Tests cover both cases.
The ordinary-window extraction now isolates the numeric word instead of keeping
only a combined `Quantity 56` line. Geometry is approximate, not ground truth.

## Real memory comparison

The live engine retained twelve 16 MiB snapshots and the corresponding captured
window images. Through a real stdio MCP client, the unchanged matcher compared
the changing counter with 16 MiB of complete trajectories, out of approximately
4.37 GB eligible at attachment. It returned **18 non-exact hypotheses**, in about
250 ms. No causal test, controlling address or editable gameplay control was
established. A monotonic counter is particularly vulnerable to unrelated time
and activity correlations. Sample coverage is about 0.38% of eligible memory.

The ordinary-window run found zero memory hypotheses. It verified stale process
and window rejection, preservation of an existing connection after failed
replacement, detach without termination and automatic invalidation on exit.
Both Windows OCR and PP-OCRv6 participated in the correct numeric track. An
earlier fixture check omitted models and used Windows OCR only; it is retained
separately and excluded from the native-reader result.

## Cost and limits

Fresh 1280×720 Blue Prince capture time was 1,790–2,111 ms, median 1,886.5 ms.
Recognition took 1,572–1,811 ms, median 1,638.5 ms; image capture median was
221 ms and memory read median 10 ms. The ordinary window took 1,813–2,043 ms.
This CPU implementation improves readability at a substantial latency cost;
one-second observation jobs cannot sustain one capture per second here.
There is no GPU acceleration or real-time claim.

The two pinned Apache-2.0 models total 138,587,816 bytes. Four inference threads,
input and tensor size limits, line/contour limits, strict dictionary dimensions,
model hashes and finite-value checks bound the inference path. Model absence,
mismatch and inference errors retain Windows OCR with warnings. Single-reader
values remain provisional; disagreements remain explicit gaps. Legacy Ocrs
installations are still readable, but neural versions do not vote together.

Native interpolation, rectangle expansion and word geometry differ slightly
from the Python/OpenCV reference. Earlier operator-parity results do not imply
identical whole-pipeline predictions. The native scores above are separate.

## Reproduction and artifacts

Use `scripts/get-models.ps1`, then build the release workspace. The application
replay helper waits for the GUI-subsystem executable and rejects reused outputs:

```powershell
python research/ocr-v6/replay_native.py target/release/Reversebolt.exe models <frames> <new-replay.json>
python research/ocr-v6/score_native.py <labels-key> <new-replay.json> <new-score.json>
```

`reversebolt-vision/examples/sequence.rs` evaluates the standalone reader.
`live_compare.py` compares an already captured isolated session through real MCP.
`summarize_native.py` checks the frozen hashes and summarizes saved live evidence;
it never calls the observer. Original Python evaluation remains frozen unchanged.

Committed compact evidence: `research/ocr-v6/evidence/native-freeze.json`,
`native-scores.json`, and `native-blue-live-summary.json`. Local raw predictions,
captured images and original failed replays are in `test-results/ocr-native-v1/`.
The game images and model binaries are not committed. Live fixture report:
`ordinary-live-v2/native-report.json`; release MCP/practice regression report:
`test-results/smoke-1eea615a/report.json`.

All 60 workspace tests, strict Clippy and formatting checks pass. Anode confirms
the native window displays source-tagged PP-OCRv6 text from a paused Blue Prince
capture. Preview `0.1.0-20261020-224603` includes the identical tested executables,
the two verified model files, their manifest and license notices.
