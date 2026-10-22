# Browser recordings stop before regional activity selection

The unchanged product at `24f5e89` was replayed on all four retained browser-game
runs: two 2048 runs and two Javascript Tetris runs. All 64 original images were
read successfully, but no run supplied a bounded action episode. The current
regional fallback therefore had no eligible source to measure.

| Recording | Mixed-control intervals | Timing failures | Single-key intervals | Quiet intervals | Bounded action episodes |
| --- | ---: | ---: | ---: | ---: | ---: |
| 2048, run 1 | 14 | 1 | 0 | 0 | 0 |
| Tetris, run 1 | 13 | 0 | 2 | 0 | 0 |
| 2048, run 2 | 14 | 1 | 0 | 0 | 0 |
| Tetris, run 2 | 13 | 0 | 2 | 0 | 0 |
| Total | 54 | 2 | 4 | 0 | 0 |

These are the classifier's first rejection reasons, not necessarily mutually
exclusive underlying problems. The two timing failures exceeded its four-second
bound. Each interval spans from the preceding image operation's start through
the following image operation's end. Across these recordings, that span ranges
from 2.828482 to 4.383975 seconds. Multiple 80 ms presses, repeated every 350 ms, could fall between
images. The four single-key intervals lack neighboring quiet intervals; they do
not establish the isolated, repeated response required by source selection.

Every image difference was available. There were zero regional attempts, zero
selected sources and zero retained visual tracks. This is an input/timing
eligibility failure, not a failed template match, evidence of an inactive game,
or a result about the correctness of gameplay names. Current regional matching
cannot repair missing temporal separation in the original recording.

## Method and integrity

The [protocol](PROTOCOL.md) was frozen before execution. The [create-only
runner](run.py) retained every original frame, hash, capture bound and sampled
input window, using only the original product observations. No game source,
oracle, DOM geometry, delivered-input event stream or address entered the replay.
These recordings are already exposed development data, not withheld acceptance.

The existing saved-image replay executable was reused unchanged. Its backend
rejects native reads, writes and captures. Cached-observation mode made zero
model calls. All four processes exited successfully; base evidence was identical
before and after observation replay. All 242 frozen source, input, protocol and
executable files passed the post-run hash audit. No new game, screenshot, input
or gameplay edit was involved.

[Compact results](results.json) retain counts, timing ranges, image differences
and result hashes. Full output, manifests and process records are in
`test-results/browser-activity-replay-v1`. The frozen-file manifest SHA-256 is
`089953497a00012eab063abc58e9d0b25cb7c5ae3b98d7e3ad1cc9cd7c03033d`;
the compact results SHA-256 is
`f305528e9d9041bb6ed30074c1bb6e3b84e39e5042df0b3862c86f126a8266cf`.
No production source changed, so the preceding 379-test/Clippy/format verification
remains the applicable build check; this replay did not rerun that suite.

## What this determines next

Source inspection confirms that automatic supported-runtime discovery already
starts fifteen `Play` captures with a 500 ms delay between steps. These omit OCR
and bulk memory reads. Automatic observation for other targets still schedules
sixteen full captures with a 1,000 ms delay. Both delays are added to capture and
processing time, not guaranteed frame periods. The old browser recordings do
not measure today's `Play` cadence or prove that a shorter interval suffices.

The next acquisition experiment should compare a bounded fast observation phase
for other targets while retaining numeric/memory evidence, measuring actual
image/input coverage rather than assuming useful episodes. Input-aware capture
and temporal models that retain mixed actions are alternatives if fixed cadence
still misses separation. None may invent intermediate images or assign an
individual effect to mixed inputs without further evidence. No scheduling change
or new live experiment is claimed here.

Overall completion remains **35%**. No additional capability or generalization
credit follows from this failure analysis. The desktop check still reports its
executable unavailable after quarantine. The executable and security-review
packet remain local as Artem requested.
