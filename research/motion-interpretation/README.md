# Motion evidence reaches the automatic image interpreter

Automatic activity reviews previously archived patch motion without giving it to
the image interpreter. They now supply a compact summary of the exact two image
pairs, and retain it with the interpretation. **This improves two controlled
consistency answers but does not make model explanations reliable.** Metadata
discovery remains independent of visible activity, so hidden or inactive fields
remain eligible for investigation.

The summary uses the existing v2 matcher on the same validated PNG bytes sent to
the local model. It includes the metric, source metadata digest, pair image IDs
and hashes, normalized dimensions, selected/matched-cell counts, unresolved
reasons, exact accepted coordinates/displacements and optional spatial consensus.
The prompt explains partial coverage and missing evidence and does not force a
positive interpretation. Existing output schema, temperature, seed and inference
budgets remain unchanged.

`play_context.motion` is optional for compatibility with old records. Shared core
validation binds it to its source, limits it to two pairs/10 KiB, and checks count,
coordinate, spacing and consensus consistency. This establishes provenance and
structure, not correspondence accuracy. The app's real agent submission/replay
test retains nonempty numeric evidence through successive handoffs and restart.
The comparison-store restart test retains it in the original question provenance.
Changed image/source evidence still invalidates the activity context. No card
meaning, value range, freeze ability or write authority is added.

## Fixed paired runs and an input-label correction

The [protocol](PROTOCOL.md) froze an image-only observer before changing the
implementation. Baseline and revised observers each interpreted four cases once,
using installed `qwen3.8:latest`, digest
`22130167c4c20e20c7b71454612966ca8e8171e9b3cc8ab6ce8aa6cbfec79643`,
on Ollama 0.40.7. Metadata inference was not run. All prompts, options, raw replies,
sources, image hashes, executables and timings are retained.

**V1's synthetic controls are contaminated.** Their generated image IDs contained
the expected-answer labels `consistent` and `opposed`. Only the revised prompt
received those IDs. Those outputs cannot establish a benefit from numeric
evidence. They are preserved rather than replaced.

[V2](PROTOCOL-v2.md) removes the revealing IDs and repeats only the two controlled
cases through both versions. It uses the exact same frozen observer executables,
unchanged prompts/options/model and identical image bytes. Only synthetic source,
clock and image IDs change. V1's two genuine game cases use opaque IDs and were
not repeated. This makes **twelve image calls** total: eight original calls and
four corrected control calls. No prompt was tuned after viewing results.

| Case | Image-only answer | With motion evidence | What can be concluded |
| --- | --- | --- | --- |
| Saved Hyperbolica pairs | Uncertain | Uncertain | Actual change remains unverified; 3 and 7 matches. |
| Saved Manifold Garden pairs | Consistent | Uncertain | Interpretation changed; no ground truth establishes which is correct. There are 4 and 3 matches. |
| Blinded controls: both pairs shift right 18 px | Uncertain | Consistent | Revised consistency flag matches the known transform; 22 matches per pair. |
| Blinded controls: right 18 px, then left 18 px | Uncertain | Uncertain | Both flags reject consistency; only the revised description identifies opposite directions. There are 22 and 20 matches. |

The controlled source/timing/key annotations are synthetic; no input was sampled
or sent. Their pixels derive from a normalized retained image. The baseline
descriptions incorrectly call the scene static or describe opacity/particle
changes. The revised positive description correctly says an 18-pixel rightward
shift, then contradicts itself by saying the same objects move leftward. The
negative explanation also speculates about whether a single input could cause
the change. Several game descriptions suggest camera causes the evidence does
not establish. These prose failures remain in the original receipts and results.

Two correct revised consistency flags are a narrow controlled result, not an
accuracy estimate or evidence of learned gameplay meaning. In particular, neither
positive model interpretation nor matching patches establish an actor, camera,
physical speed or causal relationship to a field. No questions, names or controls
were promoted from this evaluation. Original game reviews are unchanged.

## Verification and limits

All **316 workspace test executions** pass (315 distinct cases; one epoch guard
also runs in an example target). Tests cover mismatched actual bytes/source hashes,
clock changes, malformed measurements, missing errors, false consensus, legacy
receipt compatibility, and preserved uncertainty. After strengthening nonempty
handoff/store fixtures, their focused suites also pass. Strict all-target Clippy,
formatting and diff checks pass.

The [artifact audit](audit.py) checks both source freezes, all image/input and
executable hashes, prompt hashes, exact embedded summaries, unchanged metadata
discovery code, unchanged control pixels, the removed labels and every run's
outcome. [Results](results.json) include all twelve interpretations, including
the contaminated controls and contradictory prose. Raw artifacts are under
`test-results/motion-interpretation-v1` and `-v2`. All runners exited normally;
no inference was retried to obtain a preferred answer.

There were no game launches, new screenshots, manual image reviews, target
accesses or gameplay writes in this evaluation. No external service or new model
weights were used. This increment is source-only; the quarantined desktop preview
was not restored or republished. Progress remains **35%**. Reliable structured
consequence interpretation and validation against actual gameplay remain required.
