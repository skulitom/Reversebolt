# Local OCR and renderer evaluation

For the later PP-OCRv6 experiment, including fresh Blue Prince footage and
its unresolved integration gaps, see [the experimental report](../../research/ocr-v6/README.md).
Those results do not replace the released-reader evaluations below.

20 October 2026. This is observation evidence, not proof of semantic discovery,
authoritative state, or verified game editing.

## Half-Life 2 development sequence

Local captures:
`test-results/hl2-capture/captures/2a0c4a95-9964-45c3-bc15-6399b3c54d07/`.
Twelve 960×540 frames show health 100, pistol ammunition from 18 down to 7,
and reserve ammunition 150. These labels and expected values belong to this
manual evaluation; they are never inputs to the observer. The installed
City 17 ASCII Workshop addon was active. The console was used to supply weapons
for the test (`sv_cheats 1; impulse 101`); Reversebolt did not edit the game.

Windows OCR recovered 7/12 ammunition readings, 12/12 reserves, and interpreted
health as `IOO`, which our numeric parser rejected. Whole-image grayscale,
inversion, Otsu thresholding and 2× scaling did not fix the missing single digit
or health reading. See `test-results/hl2-ocr-sweep/` and
`test-results/hl2-ocr-ammo7/`.

Ocrs improves health recognition but can truncate digits in a full-image pass.
Forcing a numeric alphabet merged unrelated fields and interpreted a crosshair
as a digit, so no such alphabet restriction or letter-to-digit replacement is
used. Grouping distant words into one line also harmed HUD recognition.

Re-reading regions learned from earlier numeric detections recovers missed
numbers. The first prototype incorrectly used expanded recognition geometry to
expand future regions, eventually producing cropped values, overlaps and
duplicate tracks. The corrected observer learns geometry only from fresh full
image detections, expires regions after three missed detections, and resets on
image-size changes. Revisited output cannot perpetuate or enlarge its own seed.

The current policy uses both OCR sources. Spatially overlapping readings that
disagree remain uncertain; votes are not counted as independent confirmations.
The raw alternatives remain visible. All admitted values are provisional.

| Annotated field | Windows-only correct / 12 | Combined correct / 12 | Combined wrong | Combined uncertain/missing |
|---|---:|---:|---:|---:|
| Health | 0 | 12 | 0 | 0 |
| Pistol ammunition | 7 | 9 | 0 | 3 |
| Reserve ammunition | 12 | 12 | 0 | 0 |
| Total | 19 / 36 | 33 / 36 | 0 | 3 |

The three ammunition conflicts are 17 versus 3, 15 versus 5, and 10 versus 0.
They are excluded from matching. There are only nine admitted changing samples,
below the current ten-value correlation requirement. No memory hypothesis or
verified control is claimed for this sequence.

This is a development clip used to diagnose the observer, not a withheld
evaluation or a general OCR accuracy estimate. The expected values were compared
after predictions. Saved artifacts include `hl2-ocrs-fused.json` and
`hl2-ocrs-score.json` under `test-results/`.

Reproduce with the pinned model files:

```powershell
scripts/get-models.ps1
cargo run -p reversebolt-app --bin Reversebolt -- --ocr-sequence models test-results/hl2-capture/captures/2a0c4a95-9964-45c3-bc15-6399b3c54d07 test-results/hl2-ocrs-replay.json
```

## Blue Prince: independent target, initial findings

User-selected next test. Installed Steam app 1569580, build 25208956, launched in
Anode using its Steam bridge. No game code, offsets or source layouts were read.
The existing profiles were preserved; gameplay testing uses the previously
empty third profile.

PrintWindow produced uniform black images both while the game was obscured and
while foreground. Anode showed the profile selection screen correctly. The
failed captures and full response metadata are retained under
`test-results/blue-prince-ocr/`, including
`print-window-obscured.json` and `print-window-foreground.json`.

Windows Graphics Capture then produced a correct 1280×720 image of that same
window while obscured by Reversebolt. It captured only the selected HWND.
Initial sample: 257 ms image acquisition, 7 ms for 16 MiB memory, 312 ms local
recognition, 576 ms total. The timing includes first-use model loading; it is not
a steady-state benchmark. Eligible writable memory at attachment was about
899 MiB, so most memory remains outside this sample.

The first unchanged OCR pass found menu numbers including 40, 67 and 26. It also
misread a small 3 as 8 with no competing numeric reading. This is a concrete
false provisional observation: disagreement handling cannot detect every
single-reader error. Decorative digits, units attached to numbers and tiny
symbols remain difficult. A usable capture does not establish reliable gameplay
observation or memory discovery.

The new rendering backend addresses a generic capture failure; no Blue Prince
recognition regions or character rules were introduced.

Two initial gameplay captures are also retained. On the first, the footprints
icon beside a visible 50 was interpreted as extra digits, yielding a provisional
1850. On the next, Windows read 50 and the remembered region read 950; the engine
correctly abstained on that disagreement. No changing gameplay-number trajectory
or memory hypothesis has been established. The game was paused in the new third
profile after a short walk, leaving the two existing profiles intact.

## Blue Prince: changing counter and idle-crash investigation

The next session used the same third profile and ordinary gameplay: collect the
blueprint, draft a Closet from the Entrance Hall, and walk across that doorway
in both directions. No expected values, HUD coordinates, game names or memory
layouts were supplied to the observer. Evaluation labels were recorded separately
after inspecting the saved images.

The first run crashed on its second capture after about 154 seconds between
captures. The image was saved, but recognition did not complete. Windows recorded
an access violation in the cached `OcrEngine` activation-factory call. The crash
dump's invalid address lies inside an **unloaded `Windows.Media.Ocr.dll`**.
Evidence is in `test-results/blue-prince-changing-v1/crash-summary.json`, with the
original binary hash and symbol; the original preview remains in its versioned
distribution directory. The dump remains local in the Windows CrashDumps folder.

The OCR path now retains one process-lifetime MTA usage cookie, matching the
lifetime of windows-rs's static activation-factory cache. Individual OCR calls
still balance their thread initialization. This prevents the runtime disappearing
between observations. Microsoft's documentation describes
[runtime teardown](https://learn.microsoft.com/en-us/windows/win32/api/roapi/nf-roapi-rouninitialize)
and [keeping the MTA available between workers](https://learn.microsoft.com/en-us/windows/win32/api/combaseapi/nf-combaseapi-coincrementmtausage).
The added four-thread OCR/library-unload check passes, but also passed before the
fix: it does **not** reproduce the idle failure on its own.

The fixed release also completed capture 13 in the same process after a
**199.899-second gap** from capture 12, with 23 recognized words and no capture
or recognition warning. This is recorded in `blue-prince-changing-v2/idle-report.json`.
One idle regression is evidence for this fix, not a general stability guarantee.

With that lifetime fix, twelve consecutive captures completed. The unchanged
numeric reader failed the changing-counter evaluation:

| Evidence | Result |
|---|---|
| Visually checked counter values | 49, 48, 47, 46, 45, 44, 43, 42, 41, 40, 39, 38 |
| Correct admitted readings | **0 / 12** |
| Incorrect admitted readings | **3 / 12**: 1849, 1847, 14 |
| Explicit conflicting readings | **8 / 12** |
| Missing reading | **1 / 12** |
| Capture path | Windows Graphics Capture, 1280×720, no warnings |
| End-to-end capture time | 347–469 ms, mean 381 ms |
| Memory sampled per capture | 24 MiB of 4,353,998,848 eligible bytes at attachment |
| Memory correlation | Refused: no track met the changing-observation requirement |

The footprint symbol repeatedly becomes leading digits. The stylized 41 was
also read as 14. Agreement and temporal location do not establish numeric
correctness. The raw alternatives, all twelve PNGs, metadata, manual annotations,
hashes and scoring output are under `test-results/blue-prince-changing-v2/`.
The region is identified only in those evaluation annotations, never in runtime.
Other tracks include the room's Rank 1 and spurious scenery readings.

This is failure evidence, not a successful memory search with zero results:
matching never ran, and the memory budget covered only about 0.58% of eligible
bytes anyway. Next observation work needs generic icon/text separation and
better handling of stylized characters, tested on new sequences and prior
regressions. Trimming a known prefix, subtracting 1800, or supplying the visible
counter as an oracle would not satisfy the generalization goal.

Preview `0.1.0-20261020-203437` contains the lifetime fix. Ten workspace tests,
strict Clippy, formatting and the release practice/MCP smoke suite pass; the
smoke report is `test-results/smoke-88bb4d9e/report.json`. The game remains
paused in the third profile, Day 1, with 38 footsteps. The test engine was
disconnected and closed; the original profiles were not used.

## Ordinary-window regression

The initial combined reader regressed on the existing WinForms test: region
re-reading returned 7 instead of 27, .92 instead of 92, and 881 instead of 81,
despite both full-image readers agreeing on the correct values. The failure is
retained in `test-results/observation-v3/failed-native-status.json`. The policy
now requests region recovery only where the two full-image readers do not agree.
This avoids an unnecessary constrained recognition pass; it does not vote away
contradictory outputs after observing them.

Graphics capture initially included title-bar controls, one of which was read
as zero. Capture now crops to the checked client area. The native regression
requires a track to equal the entire supplied sequence, rather than merely
requiring twelve nonmissing observations. This prevents an unrelated constant
track from satisfying the test.

The final release passed the original sequence and a fresh sequence
`17,62,48,93,26,71,35,89,54,22,78,46`: both 12/12 exact track samples, one
numeric track each, and zero memory hypotheses. All twelve fresh captures used
Windows Graphics Capture at the expected 560×260 client size. Fresh total
capture time was 334–386 ms (mean 360 ms). Reports are
`test-results/observation-v3/native-report-original.json` and
`native-report-fresh.json`. The unchanged Half-Life 2 replay remained 33/36
correct, three abstentions (`test-results/hl2-ocrs-final.json`).

The final release also captured Blue Prince's paused gameplay through the same
client-area graphics path, with no capture warning:
`test-results/observation-v3/blue-prince-final.json`. This is a rendering check,
not another numeric accuracy result. The earlier gameplay OCR failures remain.

## Dependencies and sources

- [Ocrs](https://github.com/robertknight/ocrs), Rust crate 0.13.1.
- [RTen](https://github.com/robertknight/rten), Rust crates 0.26.0.
- [Ocrs model card](https://huggingface.co/robertknight/ocrs), CC-BY-SA-4.0;
  exact file URLs and hashes are in `models/manifest.json`.
- [windows-capture](https://github.com/NiiightmareXD/windows-capture), MIT,
  Rust crate 2.0.1.
- [Microsoft screen capture documentation](https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture).

Images stay local. Model checksums are verified before loading; missing or
mismatched models produce a visible Windows-only fallback. No image uploads,
game-specific profiles, signatures or expected values are used by production.
