# Image/memory timing evidence

Reversebolt now brackets each fresh prediction image with candidate-memory
reads. A changed or unbracketed value is timing-uncertain, rather than an
agreement or contradiction. Later stable captures can still supply evidence.
This addresses an observed limitation in the [previous process-scope study](../process-scope-study/README.md):
the displayed score could change during capture, so a mismatching memory read
was not sufficient to establish that the mapping was wrong.

## Implementation

Only an existing frozen check causes extra reads. Before requesting its image,
the engine reads each unique candidate address/width once, up to 200 candidates
of at most eight bytes. These queries use the selected memory process and the
original region metadata. The existing post-image scan supplies the other
endpoint. Prefix read attempts are reserved and charged against the existing
4 GiB budget; no game-specific logic or additional memory scanning is introduced.

Each sample retains the prefix bytes and receipt. Comparisons require matching
raw bytes, complete reads, the same investigation clock and the order
`prefix read → image request → later read`. Missing later values remain missing;
changed or unbracketed samples become `timing_uncertain`. They cannot eliminate
a rival or contribute to the four agreements, adjacent hold and distinct change
required for a trial. Historical agreements without prefix witnesses do not
count toward a new trial. Late external observations cannot override timing.

The shared API exposes these witnesses and charges. Inspect uses the short label
“Capture timing uncertain.” An explicit image-request wall-clock start prevents
scorers from accidentally including the prefix read interval. The overall
capture start and monotonic read/image bounds remain available separately.

Equal endpoints do not exclude an intervening change and return, delayed rendered
frames, display copies or address reuse with indistinguishable metadata. Values
that change continuously can remain uncertain. Initial fitting still uses the
ordinary one-sided samples; the bracket requirement applies to fresh checks.
This is a minimum comparison rule, not synchronization, statistical confidence
or proof of controlling state.

## Verification

All **144 workspace tests**, strict Clippy, formatting and release build pass.
The controlled target changes bytes during image capture and separately fails
prefix reads. Both cases abstain, then recover when four later stable frames
include a hold and a change. Other regressions cover overlapping intervals,
wrong clocks, missing image bounds, incomplete reads, legacy evidence, immutable
frozen coefficients, scope invalidation and budget exhaustion before any capture
side effect. Fixture state now advances between ordinary captures; the separate
in-flight mutation test supplies the deliberately asynchronous case.

The packaged 30-tool MCP/practice smoke also passes, including edit/restore on
the owned practice fixture. This is separate from real-game performance. Preview
`0.1.0-20261021-073028` is running on the user's desktop with eight history
entries and the selected model preserved. Binary/model hashes were verified.

## Fresh game evaluation

The [protocol](PROTOCOL.md), [configuration](protocol-v1.json), original source
hashes, binaries and changed source snapshots were frozen before capture. Each
unchanged original game received 12 training observations and eight fresh
observations without refitting. Renderer selection, budget and fixed native
input schedule match the earlier study. Optional local image interpretation was
disabled. The separate oracle was never supplied to Reversebolt. There were no
memory writes, retries, adaptive inputs or manually viewed screenshots.

| Game | Initial hypotheses | Fresh result |
| --- | --- | --- |
| 2048 | None | All eight fresh observations retained; no timing-check evidence |
| Javascript Tetris | Five score-related mappings | Each agreed at frames 13–18; all invalidated when mappings became unavailable at frame 19 |

Tetris's initial observed score was
`0, 10, 20, 20, 30, 40, 50, 60, 70, 80, 110, 140`.
It stayed at 140 throughout frames 13–20, with the independent oracle confirming
all eight readings. The game had stopped playing during the run. There was no
fresh change, so even the six agreements per candidate were insufficient to
qualify a trial before the mappings were invalidated. No value became editable.

The native run retained 30 agreeing comparisons and ten missing ones, with
140 extra candidate bytes attempted in total. It produced no changed-endpoint
sample. Replaying the old single-read classification on these same samples gives
the same outcomes. Thus the real-game run establishes native endpoint collection,
byte accounting and invalidation, **not** a measured reduction in timing errors.
The controlled tests establish the changed-endpoint rule. This limitation is
preserved rather than rerunning until a more favorable sequence appears.

Selected-counter OCR scores, each out of 20 captures:

| Game | Score | Other selected counter |
| --- | --- | --- |
| 2048 | 8 correct, 1 abstention, 1 conflicting, 10 unscorable | Best: 10 correct, 10 unscorable |
| Tetris | 20 correct | Rows: 20 correct, always zero |

The scorer retains the original 200 ms timing, geometry and stable-oracle
exclusions, using the explicit image start. Excluded readings are never counted
correct. These exposed development sessions are unpaired with prior runs; no
accuracy improvement or withheld generalization is claimed. The test oracle can
also allocate copies in the renderer. Input audit confirms 221/178 ordered,
trusted key-down/key-up pairs for 2048/Tetris. Both jobs exited successfully,
their owned processes closed, and the Anode lease was released.

## Evidence and next work

The compact [receipt](evidence/v1.json) records trajectories, before/after bytes,
read/image bounds, outcomes, input audit, source identities and artifact hashes.
Raw captures, statuses and oracle samples remain in
`test-results/capture-alignment-v1/`. `source/` preserves the exact changed
product/test source bytes named by the frozen configuration. No raw training
memory was retained, so initial fitting cannot be replayed in full.

Run `python research/capture-alignment/verify.py` to recompute the saved audit;
it checks source/method hashes, scope selection, immutable mappings, ordered
input delivery, capture hashes, timing classification and all attempted-byte
charges. `score.py <run-directory>` recomputes OCR scoring. New live evaluations
must use a new frozen configuration and new output directory, never overwrite
this evidence. Build/test/delivery logs are named `test-results/timing-*`, and
the packaged smoke receipt is `test-results/smoke-eb361c5c/report.json`.

Overall progress remains **26%** against the fixed full vision. This improves
the evidence used by both humans and agents, but automated selection of useful
memory, continuous-value tracking, active ambiguity resolution and verified
gameplay controls remain unfinished. The next investigation should advance
those capabilities rather than count additional passive agreements as authority.
