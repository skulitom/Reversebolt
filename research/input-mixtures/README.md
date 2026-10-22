# Sampled input mixtures do not predict these image changes reliably

2026-10-22. Progress remains **35%**. The fixed additive mixture model fails the
declared comparison in all **16 / 16** train/test/timing combinations. It is not
integrated into discovery. Production source remains at `b58e939`; this change
adds an offline Rust research example, analysis and retained evidence only.

## Question and result

The [capture/input study](../input-cadence/README.md) found no repeated isolated
action opportunities in four original browser recordings. Rather than assigning
an effect to one of several keys, this experiment tests whether aggregate sampled
key fractions can predict aggregate image change across recordings. The target
is the existing product image-difference metric, not score, motion, semantics,
physical speed or any hidden gameplay property.

The frozen method trains on one run and evaluates the other run of the same game,
then reverses direction. It compares fixed ridge regression using elapsed time
plus sampled key fractions against the same regression using elapsed time alone.
All fitting and standardization use the training run only. The primary input
window lies strictly between the two capture operations.

| Training → evaluation | Time-only MSE | Mixture MSE | Relative error reduction | Misaligned controls at least as good |
|---|---:|---:|---:|---:|
| 2048 run 1 → run 2 | 2.0959 | 2.1434 | −2.3% | 7 / 14 |
| 2048 run 2 → run 1 | 1.3544 | 1.3785 | −1.8% | 7 / 14 |
| Tetris run 1 → run 2 | 0.2749 | 0.3682 | −34.0% | 13 / 14 |
| Tetris run 2 → run 1 | 0.2159 | 0.2956 | −37.0% | 12 / 14 |

Negative reductions mean the mixture model is worse. The alternative capture
boundary choices sometimes improve 2048 error, by up to 11.2%, but at least one
misaligned-input control equals or beats every result. Tetris is worse than its
time-only baseline under all four timing choices in both directions.

All 60 original intervals produce valid profiles under all four choices:
**240 profiles**, **16 evaluations** and **224 alignment controls**. No interval,
variant, direction or error was dropped. Circularly rotating test input fractions
is a sensitivity check, not an independent permutation test or a p-value.
No simultaneous-key samples occurred in these profiles: the mixtures contain
successive key presses between images. Their order is lost in a fraction vector.

## Method and evidence

[Protocol](PROTOCOL.md), [runner](run.py), [compact results and verification hashes](results.json).
The Rust example is `crates/reversebolt-core/examples/input-mixtures.rs`.
Its extraction uses production frame/input types and validates original scope,
chronology, retention, gap receipts and sample ordering before deriving features.
It preserves up to 512 polls from the union of two source windows; it does not
truncate the union to one window's limit. Invalid intervals remain errors.

The research representation permits a five-second span and mixed sampled keys.
It does not alter the production isolated-action classifier's four-second limit,
capture ambiguity checks, activity selection, planning or editing rules. Its
fractions describe sampled states; they are not continuous hold measurements or
proof of delivered inputs. A valid numeric feature vector is not causal evidence.

All **64 PNGs** match their original hashes. Each cached measurement is matched
to its exact before/after frame IDs and image hashes; cached result and status
hashes match the previous retained evidence. All **245 frozen files** passed the
post-run audit. Complete features, models, predictions, scores, process receipts
and manifests remain in `test-results/input-mixtures-v1/`.

Feature SHA-256: `06c089c7670ae3e0f6b58210d06793a01ca80c2b2e390d9f2ccd1f7dc7b4f365`.
Evaluation SHA-256: `7d003bbba6c5bde21317aa3ef2b9708d4c370b16cd470b1d22c4e56d9fd1a8d8`.

Three extractor tests pass: preservation of mixed keys without isolated-action
promotion; complete outer-window union beyond 256 polls; and invalid scope,
ordering, missing samples and foreground rejection. Targeted strict Clippy and
format checks pass. Fixed synthetic analysis checks recover a predictive linear
response and retain rank deficiency for perfectly coupled keys. Those synthetic
results validate arithmetic, not gameplay learning. Production files are
unchanged, so the preceding 389-test result remains applicable and was not rerun.

No game, new capture, input delivery, target access or model invocation ran.
The preview availability check still finds the quarantined executable missing.
Security-review materials remain local; nothing was uploaded.

## What this changes next

This result rejects using this particular additive sample-fraction model with
this aggregate image metric on these exposed development recordings. It does
not show that all mixed-input models fail. Action order, state dependence,
object-specific responses and delayed consequences are absent from this model;
the small recordings provide weak evidence for fitting those richer alternatives.
Capture-edge feature choices are sensitivity variants, not true image timestamps.

The next concrete route is a bounded input-only observation experiment that can
establish separated response evidence before any gameplay field is named. The
current comparison planner requires an existing isolated activity source, which
these recordings cannot supply. An observation experiment should preserve the
existing ownership, stop, foreground and sampled-input checks, capture quiet
intervals around repeated observed inputs, and retain unsuccessful outcomes.
It must not gain memory-edit authority from a key sample or successful delivery.
This route needs implementation and fresh validation; it has not run here.

No generalization or capability credit is added for a failed model comparison.
