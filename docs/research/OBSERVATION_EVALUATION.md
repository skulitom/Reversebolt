# Local OCR and renderer evaluation

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
