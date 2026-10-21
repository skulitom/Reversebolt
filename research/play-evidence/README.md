# Passive play evidence

Reversebolt now collects a short image/input sequence before automatic runtime
discovery. Its new `play` capture mode skips memory sampling and OCR. A shared
MCP command, `read_play_evidence`, summarizes retained images and sampled input
as ordered intervals and contiguous key episodes. It returns cited image IDs,
image hashes, a digest of the complete frame/input metadata, rendered differences,
quiet comparisons and explicit unknown intervals. It does not add unidentified
cards, a report panel, another capture button, semantic labels or automatic input.

This supplies evidence for an investigator to choose useful observations. It
does **not** yet connect a runtime hypothesis to an executable test plan. The
local metadata model does not consume these summaries. Overall progress remains
**34%**, with no independent-game or semantic-discovery credit.

## Behavior

Automatic attach to a supported runtime now queues fifteen play captures with
500 ms delays after each completed capture, followed by one discovery step. The
existing job journal, stop/pause, scope and human-control rules own the sequence.
It uses the existing twenty-moment batch and image-storage limits. Play captures
consume image slots but no retained-memory allowance. Full and visual captures
still read memory; visual skips OCR. Missing numeric observations remain missing.
An agent can request play mode through `start_observation_job` for other targets.

`read_play_evidence` operates on 2–20 ordered, retained frame IDs in an exact
investigation and batch. It checks each saved image's hash, computes the existing
versioned grayscale response metric and checks input chronology, focus and
coverage. Chords, lost focus, sampler changes and missing frames remain unknown;
missing measurements never become zero. Consecutive same-key intervals form one
episode. A bounded episode needs readable no-key intervals on both sides. The
response includes descriptive per-episode and quiet medians, with no causal
threshold, key meaning, field bounds or write permission. An agent can inspect
the cited images selectively when the numeric summary cannot explain a change.

The summary is computed from retained evidence; reading it does not capture the
screen, read target memory, run a model, launch a helper or send gameplay input.
It does not create a separate durable interpretation record. Original frames,
timing and sampled input are retained by the existing capture evidence pipeline;
the live API currently requires that batch to remain current.

## Verification

The native evaluator used the existing owned ABI/visual fixture through the
actual stdio MCP server in Anode. Product binaries, source files and the evaluator
were frozen before each attempt. The fixture renderer was configured explicitly
by the evaluator; this is not discovery on an independent game.

The final attempt passed **18 checks**. Automatic attach captured fifteen play
moments before its single discovery operation. A fresh connection then captured
another fifteen moments with six evaluator-supplied 200 ms inputs. Four bounded
D episodes were retained; adjacent D intervals counted as one episode. The
median rendered difference was **138.892**, versus **0** for quiet intervals.
The D+A chord remained unknown. No screenshot was manually viewed.

Wrong investigation, wrong batch, reversed and duplicate IDs, and a deliberately
altered image were rejected. Restoring the fixture image restored the exact
summary. Every play capture recorded zero memory bytes, memory read receipts,
prediction reads and OCR time. Three separate runtime discovery/refresh helpers
completed and unloaded; their twelve pauses balanced against the fixture oracle,
with maximum stop-to-resume 62 microseconds and no helper heap activity while
paused. There were no memory writes. The fixture exited normally; the evaluator
terminated its owned headless engine/MCP after retaining results. Anode was released.

The first native attempt stopped after its automatic fifteen captures because
the evaluator looked for metadata under `data/live` instead of the engine's
actual capture folder. It sent no gameplay input and performed no writes. The
original script and failed outcome remain under `play-evidence-v1`; the corrected
evaluator was frozen under `play-evidence-v2` before the next run. No thresholds
or input plans were changed. The corrected attempt is not presented as first-pass
success.

All **268 workspace tests**, strict Clippy, formatting, the release build and
the **52-tool** smoke pass. A repeated suite run failed a journal fault test.
Inspection found that its test-directory helper could reopen old evidence after
Windows reused a process ID; it now creates directories exclusively and skips
existing names. The exact cause of that particular failed run was not established.
The failure log and subsequent successful full-suite log are retained.

Preview `0.1.0-20261021-222348` is running on the user's desktop, with all 44
history entries, the local model setting, changes, trials, tests and Freeze
records preserved. Its four binary hashes match the tested release. The
post-run audit and compact outcome are in `audit.py` and `results.json`.

## Limits and reproduction

This is a bounded initial sequence, not continuous play learning or a promised
frame rate. Mouse motion, gamepads and sound are not sampled. A rendered change
can reflect cameras, menus, animation, scene changes or other inputs. Keys sampled
in Windows are not proof of delivery. Repeated episodes are observational intervals,
not independent trials. No automatic plan or new gameplay control was established.

Build the workspace in release mode and use `freeze.py NEW_OUTPUT` to retain the
product and existing owned fixture. The fixture binary comes from the earlier
`research/hypothesis-tests/build_fixture.py` procedure. Run `run.py NEW_OUTPUT/case`
inside Anode, verify the reported window is foreground, and create its
`fixture-focused` marker. The runner refuses an existing case directory. Retain
desktop publication verification before running `audit.py NEW_OUTPUT`.
The audit was written after execution and verifies retained evidence rather
than imposing a new effect criterion. Raw fixture evidence stays under the
ignored test-results folder. Product code contains no fixture-specific label,
address, key meaning or game profile.
