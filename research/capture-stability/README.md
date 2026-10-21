# Fixed-window capture sampling — 2026-10-21

The native capture path now retains the last frame completed within a fixed
80 ms window after the first completed frame, capped at eight frames. It keeps
the latest image and small receipts, without choosing images by pixel content,
OCR or an expected value. Genuine blank scenes remain evidence. A dominant-color
diagnostic now recognizes nearly uniform images even when a few outlier pixels
would defeat the old minimum/maximum test.

This change follows the retained final-frame failure in
[learned references](../learned-references/README.md). That experiment is intact.
The two new relocation cases use byte-identical copies of its runner and fixture
against the changed binary. The third case is a persistently blank window. The
[protocol](PROTOCOL.md), sources, scripts and executable hashes were frozen
before execution; [verify.py](verify.py) is a separately identified post-run
auditor. No screenshots were manually inspected and no games were controlled.

| Fixed case | Result | Audited captures | First / selected low-detail frames |
|---|---|---:|---:|
| 0: relocate, edit, observe, restore | Passed; inferred offset 2960, moved field changed 50 → 53 → 50 | 24 | 1 / 0 |
| 1: relocate, prepare, move again | Passed; outdated write refused, old/current fields remained 50 | 21 | 0 / 0 |
| 2: persistent blank | Passed; all four captures retained, no OCR words/tracks or edits | 4 | 4 / 4 |

All 49 selected PNGs match their sampled RGBA receipts; all source timestamps
are present. Bursts contained one to three completed frames; the largest
first-to-selected completion span was 59.728 ms, within the 80 ms limit. In one
intervention capture, the diagnostic detail fraction changed from approximately
0.000106 to 0.023742 before selection. The timing rule selected the later frame
without consulting either value. Only the selected pixels are persisted;
intermediate hashes and diagnostics are receipts, not retained images.

The four blank captures had a detail fraction of approximately 0.000106, a
diagnostic warning and no text or numeric tracks. They were not retried or
replaced. Their selected pixels all share RGBA hash
`3bf28707010e9cd306e6417effdadb31f19f383dfc7d1b386140e32e3166703b`.

The successful native edit closes the previous missing native positive check
for learned-reference reuse. Both relocation runs inferred the pointer root and
offset from anonymous fitting bytes, preserved the original mapping, and
collected four fresh value and reference witnesses. The evaluation-only oracle
did not select candidates or provide addresses to discovery. This remains an
owned fixture result, not evidence of a controlling gameplay field.

## Bounds and interpretation

The sampler adds up to 80 ms after the first completed frame, in addition to
startup, processing and shutdown costs; eight frames can end it sooner. Static
windows may provide only one frame. A late callback does not replace the selected
image. Native capture still uses one bounded worker and the existing five-second
caller timeout. If OS shutdown hangs, that worker remains occupied instead of
spawning replacements. Compatibility capture has no WGC sampling receipt.

Microsoft documents `SystemRelativeTime` as a compositor-render QPC timestamp;
it is recorded raw, not treated as simultaneous game-memory evidence. See
[the Windows API documentation](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.direct3d11captureframe.systemrelativetime?view=winrt-28000).
Memory/image capture remains sequential. These receipts do not exclude ABA
changes, compositor lag, incorrect semantics or downstream effects. Three
fixtures do not establish statistical reliability or eliminate transient blanks
on other applications. No independent-game progress is claimed.

## Validation and delivery

All **170 workspace tests**, strict Clippy, release build and packaged smoke
pass. The smoke covers the shared engine, history, authentication and 32 MCP
tools. Three new tests cover content-independent selection (including real
blanks), static/frame-count/deadline bounds and outlier-resistant diagnostics.

Preview `0.1.0-20261021-102913` is running in desktop session 1, PID 80068, with
eight history entries and `qwen3.8:latest` preserved. Main controls remain named
draggable cards; sampling metadata is in shared evidence rather than a report
on the main surface. All nine owned test processes exited and the Anode lease
was released. Weighted progress remains **29%**, with the full goal active.

- [Frozen inputs and hashes](protocol-v1.json)
- [Audited native outcomes and capture receipts](evidence.json)
- [Execution, checks and desktop delivery](execution.json)
- Raw run artifacts: `test-results/capture-stability-v1/case-{0,1,2}/`
- Packaged smoke: `test-results/smoke-a80ebc5a/report.json`

To reproduce with fresh output directories, run `run.py 0 OUTPUT`, `run.py 1
OUTPUT`, and `blank.py OUTPUT` using Python with Pillow inside an acquired Anode
session. Set `REVERSEBOLT_TEST_SESSION` and `REVERSEBOLT_PARENT_SESSION` to the
verified distinct sessions. Runners require the exact frozen binary and method
hashes. The post-run auditor targets the retained v1 paths and does not alter
the frozen methods or runner outcomes.
