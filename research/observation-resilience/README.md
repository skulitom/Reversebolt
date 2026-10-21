# Preserve useful observations when an optional region or model fails

This change addresses two failures from the unchanged
[independent-game baseline](../independent-games/README.md). It does not add game
profiles, expected values, known HUD regions or new model training. Progress
remains **25%** against the full vision: useful observation improved, but no new
verified gameplay control or independent-game acceptance gate was established.

## Text-region isolation

The PP-OCRv6 reader previously returned an error for the entire image when any
line exceeded its 1024-pixel recognition-width limit. That discarded text already
recognized elsewhere and cleared temporal hints. In 2048 this happened in 31 of
32 captures. The reader now partitions unsupported crops before batching,
retains valid regions and returns explicit skipped-region geometry, source and
reason. The desktop/shared engine retain other words with a partial-reading
warning. Missing regions do not become empty strings, zero values or inferred
digits. Detection/tensor/model errors still reject the neural pass.

Recognition width now rounds up consistently before the limit check and tensor
allocation. Tests cover the exact boundary, a fractional width above it, rotated
tall crops, invalid crops and valid neighboring regions. Previously learned
regions still expire after bounded misses; revisited outputs cannot refresh
their own geometry or lifetime.

## Optional model failures

An unavailable model, transport failure or rejected interpretation contributes
zero findings. After its immutable evidence receipt is saved, the engine records
a warning for the owning job and source images and permits other capture steps
to continue. The separately scheduled final review can still run; the failed
pass is not retried. No invalid output is truncated or accepted to force success.
The proposal schema now advertises its existing text-length limits as well.

`state.live.local_observation_warnings` exposes the same notices through MCP,
bounded to 32 for the current memory batch. The desktop shows one short notice;
**Inspect application → Visual review warnings** holds the reasons and evidence
ids. Batch advancement clears current notices after archiving; the original
model receipts and activity remain subject to their existing retention policy.
Job completion does not mean every optional model review succeeded.

Evidence-save, checkpoint, scope, publication and unexpected worker failures
still stop the pipeline. Cancellation suppresses publication. Tests retain the
exact 306-character summary rejected in the Tetris baseline, confirm its strict
rejection and durable receipt, then continue capture through moment 16. Separate
forced archive/checkpoint failures confirm that persistence errors still stop
the owning job. Restart does not resurrect a live investigation.

## Paired saved-image evaluation

`replay.py` compares the original published executable with the new executable
on 100 existing images: the four 16-frame browser runs and three older 12-frame
sequences. It writes binary/input/scorer hashes before starting either reader,
copies and verifies full images, and runs each sequence with a fresh reader.
Only model files and image paths reach the product. No games are launched.

Browser scores reuse the original timing/geometry exclusions and image-coordinate
oracle boxes. They never choose a track by the expected value. Earlier game
regressions use their existing separate field annotations/scorer. This is an
exposed-corpus regression, not fresh or withheld-game evaluation. Results below
are selected-counter results, not full-scene precision.

| Counter, across two runs | Original C/W/X/A/U | Updated C/W/X/A/U |
|---|---|---|
| 2048 score | 9 / 1 / 1 / 5 / 16 | 13 / 0 / 2 / 1 / 16 |
| 2048 best | 12 / 0 / 1 / 3 / 16 | 15 / 0 / 0 / 1 / 16 |
| Tetris score | 30 / 0 / 0 / 0 / 2 | 30 / 0 / 0 / 0 / 2 |
| Tetris rows, constant zero | 32 / 0 / 0 / 0 / 0 | 32 / 0 / 0 / 0 / 0 |

C = correct, W = wrong, X = conflicting, A = abstention, U = unscorable.
Do not inflate changing-counter accuracy with the constant-zero row counter.
The previously wrong 2048 score-region reading becomes conflicting, not a
verified score. Animation remains ambiguous in half of its opportunities.
Neural words survive in all 32 2048 images, compared with one image previously;
31 partial-reading warnings remain because the unsupported lines are still
missing. Recovering more text adds work: the two 2048 runs' median recognition
times rose from approximately 1.85/1.86 seconds to 2.51/2.24 seconds on this host.
Concurrent development activity and one run per configuration preclude a
general performance claim.

The older selected-field regressions remain unchanged: Blue Prince 12/12,
Half-Life 2 36/36 and the ordinary window 12/12 correct, with no wrong, uncertain
or missing selected readings in either executable. These values do not measure
unannotated false readings elsewhere in an image. All 132 workspace tests,
strict workspace Clippy, formatting and the release build pass.

## Native integration and reproduction

`native.py` runs an owned ordinary-window fixture in a guarded Anode child
session and requests a four-capture job through the real 29-tool MCP server.
Both deliberately unavailable model reviews leave warnings; all four captures
complete and retain the numeric value 72. No visual items, memory matches or
writes are credited. Accessibility inspection confirms the simple Quantity
card, short notice and optional detailed warnings. No screenshot inspection or
game input was used. Owned processes were closed and the lease released.

Run `python research/observation-resilience/replay.py <candidate-exe> <new-output>`
with the pinned source captures, old executable and installed OCR models present.
Every output directory must be new. For the native integration, execute
`native.py <binary-directory> <new-output> <child-session> <parent-session>`
through Anode; it rejects parent-session execution. Its owned windows remain
available for accessibility checks until `finish-ui` exists or five minutes pass.
The optional model name is deliberately absent and no weights are downloaded.

Raw paired predictions and manifests are under
`test-results/observation-resilience-v1/`; native state, receipts, trace and UI
Automation snapshots are under `test-results/observation-resilience-native-v1/`.
The invalid-proposal regression fixture is copied verbatim from baseline receipt
`6070e86b-5fbb-486e-b1d0-a700dec284fc.json`, under
`crates/reversebolt-app/tests/fixtures/overlong-observation.json`. It is test data,
never a product profile or inference input.

`verify.py` audits the raw replay/image hashes, input/scorer manifest, selected
scores, native receipts and accessibility assertions, test log and versioned
desktop binary. Its compact output is [evidence/v1.json](evidence/v1.json).
Published preview **0.1.0-20261021-060956** is running on the user desktop, with
all eight prior history entries and `qwen3.8:latest` retained. Its packaged
binary/model hashes and 29-tool MCP/practice smoke passed. The earlier desktop
was idle and closed normally before the new preview launched.

Next: broaden grounded visual items beyond actors and held objects, then evaluate
fresh independent behavior. Process relationships, faster observation, authority
versus copies, high-level edits and recovery remain incomplete.
