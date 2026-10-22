# Mixed-input prediction v1

2026-10-22. This is an exposed-development-data experiment, not withheld game
acceptance. Earlier inspection already found frequent mixed keys and no usable
isolated activity source in all four browser recordings. Before this protocol,
we inspected key sample counts, but fitted no mixture model.

Question: does a simple additive description of sampled key mixtures improve
cross-recording prediction of aggregate image change beyond elapsed time?
This is not causal attribution, gameplay naming or evidence of an editable value.

Use all 16 saved frames and 15 adjacent intervals in each original 2048 and
Javascript Tetris run. Read only retained frame/input metadata and the existing,
hash-verified product image-difference measurements. Verify all 64 original PNG
hashes and the four cached result hashes. No game source, oracle, delivered-input
log, memory address, DOM data or new model interpretation enters the experiment.

A Rust offline example uses production frame/input types and buffer validation.
Reject an interval for scope, geometry, chronology, missing/truncated input,
foreground or sampling-gap errors. Allow up to 5 seconds for this research
representation; the product's 4-second isolated-action limit is unchanged.
The union retains up to 512 polls from two individually valid 256-poll source
receipts; it is not silently truncated to a single receipt's limit.
Keep invalid intervals and their reasons. Four prespecified timing choices use
each combination of the preceding capture's begin/end and following capture's
begin/end. The primary choice is preceding end through following begin. No
intermediate image or actual image timestamp is inferred.

For each choice retain its duration, all key sample counts, total poll count,
largest sampling gap and simultaneous-key sample count. A key feature is its
pressed sample fraction, not a measured hold duration. Admit only complete
foreground polls inside the chosen interval, at least three samples and no gap
or individual poll longer than 100 ms. This does not imply continuous holds or
input delivery.

Fit fixed ridge linear regression with an unpenalized intercept and penalty 1
on each standardized feature in the mean-squared-error objective. Training-only
means/scales are used. Constant training columns become zero. Baseline features:
elapsed seconds alone. Mixture features: elapsed seconds plus one fraction per
observed key (training keys only). The target is the existing mean absolute image
difference. Predictions are not clipped. No fitting choices change after results.

Train on run 1 and test on run 2 of the same game, then reverse; four directions
total, each with 15 possible evaluation intervals. Fit all four timing choices
separately and retain every result; do not pick the best timing choice. Require
all 15 intervals in both runs for scoring a direction; otherwise retain its
failure. Report raw targets/predictions, coefficients, standardization, design
rank, baseline and mixture MSE, and relative MSE reduction.

Negative alignment controls rotate the test key-fraction rows by each nonzero
offset 1–14, keeping duration, outcomes and training model fixed. Preserve every
control score. These are sensitivity controls, not independent permutation tests
or p-values. A promising result must improve over elapsed time and beat every
misaligned control in all four directions and all four timing choices. Otherwise
do not integrate this representation into discovery on the strength of this
experiment. Passing still does not establish individual effects or causal names.

Before the real-data analysis, run a fixed synthetic check of an independently
varying two-key linear response, plus a collinear-key control. Record prediction
improvement and rank; the collinear design must retain its rank deficiency. These
are checks of the analysis, not game results.

Freeze sources, method, executable, original statuses, images and cached results
before running. Use a new output directory and preserve errors, all directions,
all variants and all controls. Audit hashes afterward. No new game, capture,
input delivery, target access, model call, security change or upload is permitted
by this runner. Production inference remains unchanged.
