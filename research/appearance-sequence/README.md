# Appearance sequences tied to an unchanged source

Reversebolt now measures sequences of image correspondences from automatically
selected observations. Each later image is compared independently with the
original image and rectangle; a matched rectangle never becomes the next template.
The shared evidence reader, MCP and automatic discovery archive expose the same
results. No new capture, model call, target access or editable card is required.

This turns isolated pair measurements into signed sequences with capture timing
and sampled-input context. It is still appearance evidence, not a persistent
object identity, world velocity, input meaning, memory correspondence or verified
gameplay control. Automatic activity interpretation and trial/edit gates do not
yet consume these sequences.

## Scope and stopping policy

The existing deterministic policy selects at most four exact-source regions.
Each sequence attempts at most six following captures. A shared twelve-attempt
budget is allocated in rounds across selected sources; these attempts are
additional to the existing first-pair measurements. Each attempt reads the original
and current image under the existing 10 MiB/image and four-million-pixel limits.
Pixels are rehashed before each match, and actual PNG
dimensions are checked against metadata.

A sequence ends at the first unresolved appearance, source read/hash failure,
omitted capture or geometry/epoch change. It never skips that failure to reacquire
a later similar appearance. Independent source observations can still be analyzed.
No returned rectangle updates the original template. Limits and the first
unaccepted frame are explicit; exhausting the requested frames is distinct from
exhausting a budget. No interpolated point or invented zero fills a gap.

Accepted points retain displacement from the original and the difference from
the previous accepted displacement. Capture-operation duration bounds and sampled
key exposure describe that adjacent interval. Missing/invalid timing and mixed or
unavailable input remain unknown. The source plan, full requested frame metadata,
policy versions, limits and each prefix are hash-bound. None of these associations
establish physical speed, key delivery or input causality.

## Frozen replay

[The protocol](PROTOCOL.md), executable, 148 source files, manifest and inputs were
frozen before execution in `test-results/appearance-sequence-v1`. Five analytic
sequences reuse unmodified image bytes from the earlier appearance controls. Three
unchanged Tetris cases follow them. Scoring truth and game oracle are never supplied
to matching; no game names, offsets or predefined meanings enter the matcher.

| Declared control sequence | Observed result |
|---|---|
| Move out, return, stay | Three steps: (+160,+64), (-160,-64), (0,0) |
| Move, disappear, later identical appearance | One accepted step; failure ends the series before reacquisition |
| Missing intermediate capture | One accepted step; omitted-capture stop |
| Four sources, ten images | Three accepted steps each; twelve total attempts |
| One source, ten images | Six accepted steps; per-source limit |

All five controls pass. On the exposed saved Tetris session, the orange-preview
source accepts captures 3 and 4 with zero displacement and fails at capture 5.
The green-piece source accepts capture 3 at approximately (+19.40625,+120.375)
source pixels and fails at capture 4. The board source exceeds the existing
normalized-template size limit. There are six attempts in total. No-observation
and omitted-capture ablations perform zero attempts. All sampled intervals have
several controls and are explicitly unknown, not attributed to a particular key.

All three prior cases' region selection, fixed-region motion and two-image
appearance outputs are exactly unchanged. All 24 input files remain unchanged.
The eight-case local replay took 2.672 seconds and exited successfully; this is
one run, not a performance benchmark. [Compact results](results.json) retain the
hashes and outcomes; full reports and process receipts remain in the frozen folder.

## Separate stop diagnostic

After replay, `inspect_stops.py` checked the existing oracle at captures 2–5 using
the original capture brackets, 200 ms oracle-timing bounds, stable recorded geometry
and no-pending-redraw condition. All four brackets were stable. The recorded green
piece changes orientation at capture 4; the preview changes from orange to cyan
at capture 5. These are plausible appearance-change explanations for the two
stops, not failures reclassified as matches or proof of identity.

Both accepted stationary preview crops also compare exactly at the original RGB
pixel level, with maximum channel difference zero. The moving capture-2-to-3
pair's separate geometry check remains in the earlier
[appearance evaluation](../region-appearance/README.md). No new accuracy rate or
withheld-game acceptance is claimed. Diagnostic inputs, source hashes and outputs
are retained under `test-results/appearance-sequence-v1/stop-diagnostic`; the oracle
never enters the product. The algorithm is unchanged after examining these results.

## Verification and remaining limitations

All **347 workspace test executions** and strict workspace Clippy pass. Tests
cover signed deltas, input/timing provenance, per-source and shared budgets,
source corruption, omitted captures, scene changes, independent later sources,
and no reacquisition. A gradual-change control matches adjacent images but fails
against the original template; the sequence correctly stops instead of adapting
and drifting. Shared-reader and archive tests exercise the real product path with
a target that panics on captures or memory reads. A development compile error
requiring an explicit closure result type was fixed before the successful suite;
its original log is retained.

Rotation, deformation, size changes, occlusion and identical-looking replacements
remain major limits. Stopping is not proof an object disappeared. Keeping the
original template reduces one form of drift but cannot prove identity. Image/input
associations still need investigation against internal state and actual effects.

Weighted goal progress remains **35%**. The desktop preview remains unavailable
after its earlier Windows quarantine. No replacement was published, no protection
setting changed and no executable was externally submitted.
