# Input-only investigation before named fields

The later [automatic review](../activity-review/README.md) adds an eleventh
analysis step and corrects the initial input window. This record describes the
original ten-step implementation and its frozen verification snapshot.

Implemented 2026-10-22. Full-goal progress remains **35%**. This is a software
integration result using generated fixtures, not a successful game experiment.

## Problem and implementation

The retained independent-game recordings contain several successive keys between
images. Their isolated-action evidence is unavailable, and the frozen additive
mixed-input predictor failed its declared checks. The existing comparison path
also requires a candidate internal field and an isolated response. It cannot
collect the initial evidence when neither exists.

The new shared `start_activity_experiment` job needs only a previously sampled
key and the connected application. A source key must have complete foreground
sampling in the current batch and image geometry. Its meaning remains unknown.
The fixed ten-image sequence is a seed, a quiet interval, then four repetitions
of a 250 ms hold followed by a quiet interval. It uses Play captures without
target-memory reads, OCR or a model call. No game profile or engine-specific
layout is involved.

The desktop offers **Explore an input**. An agent can propose the same job; its
input steps wait for the desktop Start action under the existing effect-consent
rule. There are three seconds to return to the game, with no automatic focus
change. Stop and takeover retain ownership until the active step settles.
Starting this experiment can change gameplay; it cannot restore the game world.

Ten captures must fit before starting. The image cap increases from 36 to 48,
allowing the existing 31-image automatic pass plus this experiment. Memory
attempts remain capped at 20 and retained-memory/image budgets remain 512 MiB
each. Queued work shares the budgets. Other work cannot queue into an active
input experiment.

Plans are immutable and hash addressed. Steps and isolated workers claim their
plans durably before input, and retain outcomes without replay. Worker receipts
bind the exact parsed plan, worker PID and accepted press/release counts.
Successful receipt validation means Windows accepted input, not that the game
responded. Missing receipts remain unknown. Changed scope, sampler restart,
foreign keys, focus loss, image failures and cancellation stop later steps.
Once input investigation begins, a paused/interrupted sequence cannot resume
its input steps. Repeated request IDs return original receipts.

The response images are available to the existing `read_play_evidence` and
activity interpretation workflow. This experiment does not automatically run
model naming, discover an internal field or publish a control. Existing capture
ambiguity, quiet-bracket and planned-hold checks remain in force.

## Automated evidence

All **397 workspace tests** pass. Final input-worker checks pass after binding
the receipt hash to the exact parsed bytes. Strict workspace/all-target Clippy
passes, followed by targeted strict checks for the final UI and tool schema
changes; formatting passes. [Compact verification](results.json) retains log
hashes and the local frozen source manifest.

The main integration fixture uses the real LiveSession, capture persistence,
plan/step bridge and shared evidence reader, with generated textured images and
sampled key states. Its memory-read method panics if invoked. Ten captures yield
four quiet-bracketed response episodes and an activity source, with zero memory
samples. It verifies plan plus ten intent/outcome files and rejects repeated
execution. It injects a simulated executor rather than calling Windows input.

Additional tests cover missing source keys, capacity, target identity, changed
plans, duplicate intent, sampler restart, queued agent authorization, conflicting
work, request replay, unknown outcomes through journal restart, cancellation
while a step is in flight, receipt integrity and worker claim reuse.

Initial failed runs remain in the evidence: two older tests assumed the 36-image
limit; their expectations now use the actual cap while retaining the independent
memory-limit checks. A new cancellation test originally waited before driving
the job controller far enough to dispatch; it now drives dispatch before waiting.
These failures were not discarded or counted as passes.

## Limits and next verification

No game, native input worker, new native screenshot or model ran. The generated
fixture supplies deterministic image responses; it does not establish that
250 ms holds are appropriate for an unfamiliar game, that screenshots arrive
quickly enough, or that repeated input causes a stable semantic effect. The
fixed timing is a first bounded policy, not an optimized universal one. Slow
captures or sample gaps can correctly end the sequence without usable evidence.
Gamepads, mouse movement and audio remain unsupported by this path.

The desktop availability check still reports the quarantined executable missing.
No substitute app was launched and no protection setting was changed. Security
review artifacts and all new evidence stayed local; nothing was submitted.

Next: once native execution is available, run this unchanged policy against an
independently implemented offline game, retaining failures and delivery receipts.
Measure completed intervals, accepted activity sources and downstream candidate
quality before changing timings or claiming discovery/generalization progress.
Automatic follow-up interpretation and selection of which observed key to test
remain unfinished.
