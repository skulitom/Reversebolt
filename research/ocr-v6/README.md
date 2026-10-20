# PP-OCRv6 observation experiment

20 October 2026. This records the original research stage, when the desktop used
Windows OCR + Ocrs and progress remained **18%**. The subsequent native Rust
implementation, fresh complete-pipeline results and delivered preview are
documented in [native integration evidence](../../docs/research/NATIVE_OCR.md).
Nothing in this earlier experiment identifies controlling game state or
authorizes a game edit.

## Finding

An off-the-shelf text detector/recognizer plus regions learned from preceding
images recovers Blue Prince's stylized counter without a game profile, known
HUD rectangle, expected-value sequence, digit substitutions or forced alphabet.
It remains an imperfect observer: one footprint was separately read as `1`,
and numbers embedded in text lines are not isolated by this frozen harness.

| Reader | Sequence | Correct selected values | Wrong | Missing |
|---|---|---:|---:|---:|
| Tiny detector + small recognizer | Blue Prince development | 4 / 12 | 2 | 6 |
| Medium detector + medium recognizer | Blue Prince development | 8 / 12 | 0 | 4 |
| Medium + learned region re-reading | Blue Prince development | 12 / 12 | 0 | 0 |
| Same frozen medium + regions | **Fresh Blue Prince** | **12 / 12** | **0** | **0** |
| Tiny + small | Half-Life 2 development | 31 / 36 | 0 | 5 |
| Medium + regions | Half-Life 2 development | 36 / 36 | 0 | 0 |
| Medium + regions | Ordinary window, original and fresh | 0 / 24 numeric fields | 0 | 24 |

The ordinary-window model output contains all **24 / 24 exact text lines**, such
as `Quantity 40`. The harness treats whole text regions as candidate values;
`Quantity 40` is not a number. This is an extraction/geometry integration gap,
not a failure to recognize those characters. Do not count full-line recognition
as a successful numeric track, or replace the current reader on this evidence.
Current Windows recognition already handles these windows; combining readers
still needs an explicit, tested disagreement policy.

The selected-field scores are not a full-scene precision estimate. The fresh
Blue Prince frame 8 also contains a false numeric `1` at the right footprint
(box approximately x35–49, y19–48), separate from its correct `29` reading.
The model score is 0.518; this is not a calibrated probability. All unscored
numeric predictions are retained in the evidence. Thresholds were not adjusted
after seeing this result.

## Protocol and limits

`evaluate.py` reads full PNGs and temporal order only. It uses the official
model-export normalization and postprocessing settings, a recognition threshold
of 0.5, and four CPU threads. The detector uses BGR channels, scale 1/255,
mean `[0.485,0.456,0.406]`, std `[0.229,0.224,0.225]`, minimum side 736 rounded
to multiples of 32, mask threshold 0.2, box threshold 0.45 and expansion 1.4.
These differ from RapidOCR's generic default normalization.

Only numeric full-image detections seed regions. A missing current detection
triggers recognition of the most recently detected rectangle, with no padding.
At most 64 regions persist; they expire after more than three full-image misses
and reset on resolution changes. Revisited outputs never refresh their own
geometry or lifetime. Regions contain no expected values. The detector finds
each sequence's initial regions independently.

The medium configuration and evaluator SHA-256 were frozen at
`2026-10-20T21:00:26Z`, before fresh capture. The fresh clip contains twelve
1280×720 images, visually checked counter values 36 through 25, from normal
movement between the Entrance Hall and Closet in the existing third test
profile. This is new footage of the same scene and game, not a withheld-game
acceptance test. Blue Prince was paused afterwards. The two original profiles
were not used. No memory writes, game source or offsets were used.

The capture engine also collected bounded memory using the existing released
path. This experiment only evaluated image recognition; its offline predictions
were not submitted as memory-correlated observations. No memory hypothesis,
semantic control, causal validation or successful game edit is claimed.

Fresh Blue Prince recognition took 657–813 ms after its first frame (median
702 ms), with 1,353 ms for the first frame. The first frame includes lazy model
initialization. All timings exclude native capture, memory acquisition and UI work. The two
medium weights total 138,587,816 bytes, compared with about 12 MB in the current
preview. No GPU performance has been measured.

## Rust feasibility

`rten-probe` loads both unmodified official ONNX exports using RTen 0.26.0,
already used by Reversebolt. Initial zero-input smoke tests pass. More usefully,
`runtime_parity.py` records the identical real-image tensors supplied to ONNX
Runtime from the first development frame of Blue Prince, Half-Life 2 and the
ordinary window. RTen processes those tensors without ONNX Runtime or Python.

All six model calls pass: three detection maps have no threshold-mask flips,
and all twelve text crops have identical greedy CTC decoded paths. Maximum
absolute output difference is 0.000286. This checks Rust operator compatibility,
not a complete Rust image-to-observation pipeline. Preprocessing, word geometry,
temporal regions and reader fusion still need native implementation and tests.

On this host, Python initially crashed while importing ONNX Runtime with its
Anaconda-bundled VC 14.27 runtime. Loading the already-installed System32 VC
runtime into the evaluation process resolves that failure. Both 1.31.0 and
1.22.1 failed without that process-local change; the final experiment pins
1.22.1. No global Python, system runtime or application runtime was modified.
The native Rust probe does not need this workaround.

## Evidence and reproduction

Committed `evidence/scores.json` includes image hashes, expected values,
field locations, every scored raw reading and unscored numeric predictions.
`evidence/protocol-freeze.json` predates the fresh footage.
`evidence/runtime-parity-report.json` preserves the native comparison.
`labels.json` and `score.py` are **evaluation-only**, separate from recognition.
The scorer verifies input image hashes and the frozen evaluator hash.

Raw frames, full model predictions, capture metadata and failed attempts remain
locally under `test-results/`. Game images and weights are not committed.
This means another checkout needs its own footage, or access to those local
evaluation images, to reproduce image scores; the JSON reports are not a
replacement for the images.

```powershell
# Use an isolated Python 3.11 environment; do not alter the application runtime.
python -m venv test-results/ocr-v6/env
test-results/ocr-v6/env/Scripts/python.exe -m pip install -r research/ocr-v6/requirements.txt
test-results/ocr-v6/env/Scripts/python.exe research/ocr-v6/get_models.py test-results/ocr-v6/models

# Substitute a directory of full frame PNGs and a NEW output file.
test-results/ocr-v6/env/Scripts/python.exe research/ocr-v6/evaluate.py test-results/ocr-v6/models <frames> <new-output.json> PP-OCRv6_medium_det_onnx PP-OCRv6_medium_rec_onnx --temporal

# With the local result filenames in labels.json and their original images:
test-results/ocr-v6/env/Scripts/python.exe research/ocr-v6/score.py <new-scores.json>

# Native operator comparison; export accepts one or more prediction JSON files.
test-results/ocr-v6/env/Scripts/python.exe research/ocr-v6/runtime_parity.py export <new-case-dir> <prediction.json>
$env:CARGO_TARGET_DIR = 'C:\DEV\Reversebolt\target'
cargo run --release --manifest-path research/ocr-v6/rten-probe/Cargo.toml -- --cases <new-case-dir>/cases.json
test-results/ocr-v6/env/Scripts/python.exe research/ocr-v6/runtime_parity.py check <new-case-dir> <new-parity-report.json>
```

`capture.py <isolated-data-dir> <pid>` binds an already running test engine to
an inventoried window. Without the PID it captures one moment and saves the
response. It reads authentication locally without printing credentials. It
does not play the game or supply evaluation labels.

The download helper verifies hashes and sizes, refuses existing mismatches and
retains failed partial downloads. Verification of all twelve existing model
files and fresh downloads of eight configuration/model-card files passed.
The Rust probe builds in release mode and passes strict Clippy and formatting.

## Sources and licenses

Our harness and runtime probe are original MIT code under the repository license.
The unmodified PaddlePaddle weights and configuration are separately Apache-2.0;
`models.json` records exact upstream commit URLs, sizes, hashes and model cards.
`LICENSE-PaddleOCR.txt` is the upstream license retrieved 20 October 2026,
SHA-256 `3840c5c0c61c294264d2dd77b8777be6ddd90121ef4e0e64abcd22edea581d6e`.
The Python harness calls RapidOCR's installed implementation; no upstream OCR
implementation is copied into our Rust probe. Preserve upstream notices if
later porting implementation code or distributing model files.

- [Official detection model](https://huggingface.co/PaddlePaddle/PP-OCRv6_medium_det_onnx)
- [Official recognition model](https://huggingface.co/PaddlePaddle/PP-OCRv6_medium_rec_onnx)
- [PaddleOCR source and license](https://github.com/PaddlePaddle/PaddleOCR)
- [RapidOCR source](https://github.com/RapidAI/RapidOCR)
- [RTen source](https://github.com/robertknight/rten)

Next: implement bounded native preprocessing and decoding with word geometry,
test disagreement behavior alongside Windows OCR, then evaluate the resulting
complete observer on fresh footage before publishing a desktop update. Scene
changes, tiny UI, nonnumeric appearance, motion, hidden variables and causal
editing remain separate unresolved capabilities.
